use alloc::{borrow::ToOwned, boxed::Box, str, string::String, vec, vec::Vec};
use core::mem::size_of;

use bytemuck::{Pod, Zeroable};

use wipi_types::wipic::WIPICWord;

use wie_backend::Database;
use wie_util::{Result, read_generic, read_null_terminated_string_bytes, write_generic};

use crate::context::WIPICContext;

/// Per-handle state for KTF's stream-style database API.
///
/// KTF's `stream_read` / `stream_write` slots behave like a record-scoped
/// `fread` / `fwrite` pair rather than the standard WIPI record-by-id API
/// — the same record id 1 is walked sequentially with implicit cursors.
/// The original interface field names (`read_record_single`,
/// `write_record_single`) were a pre-disassembly guess; the impl-side names
/// `stream_read` / `stream_write` reflect the verified semantics.
///
/// The handle, including its read/write cursors and the in-memory mirror
/// of record 1, lives entirely in emulated memory: the `DatabaseHandle`
/// struct sits at the pointer returned from `open_database`, and the
/// mirror itself is a separate guest-heap allocation referenced by
/// `buffer_ptr`. Every op reads the struct, mutates it, writes it back —
/// no host-side global state.
///
/// `select_record` with a non-zero recid is treated as a seek: KTF apps
/// use slot 4 to position the cursor at known byte offsets within the
/// single backing record, e.g. for multi-slot save files.
#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C)]
struct DatabaseHandle {
    magic: u32,
    next: u32,
    name: [u8; 32], // TODO hardcoded max size
    read_cursor: u32,
    write_cursor: u32,
    buffer_ptr: u32,
    buffer_len: u32,
    buffer_capacity: u32,
}

const MIN_BUFFER_CAPACITY: u32 = 64;
// Logical per-application save volume, not the ARM heap or a claim about
// host free disk. The old 1 MiB profile rejected legitimate multi-MiB installs
// before the filesystem-backed repository could store their data.
const KTF_DATABASE_STORAGE_LIMIT: u64 = 32 * 1024 * 1024;
// "MCDB" — sentinel at the start of the handle struct so we can distinguish
// a real DB handle pointer from an unrelated guest pointer (e.g. a C-string
// name pointer that KTF's slot 6 passes through the same SVC argument slot).
const DATABASE_HANDLE_MAGIC: u32 = 0x4D434442;
const MAX_NAME_LEN: usize = 31; // leave a byte for null terminator inside the 32-byte field

pub async fn open_database(context: &mut dyn WIPICContext, ptr_name: WIPICWord, mode: i32, r#type: i32) -> Result<i32> {
    tracing::debug!("MC_dbOpenDataBase({ptr_name:#x}, {mode}, {type})");

    // Guest-provided C string — invalid UTF-8 must not bring down the
    // emulator. Treat it as a bad parameter and return -22, matching the
    // fail-soft behaviour of the other name-keyed entry points in this
    // file (`stat_by_name_ktf`, `exists_database_ktf`).
    let Ok(name) = String::from_utf8(read_null_terminated_string_bytes(context, ptr_name)?) else {
        tracing::warn!("MC_dbOpenDataBase: invalid utf8 name @ {ptr_name:#x}");
        return Ok(-22);
    };

    // Validate before any repository side effects. Mode 4 deletes record 1
    // up front, so a too-long name reaching that path would wipe data we
    // can't open a handle for anyway.
    if name.len() > MAX_NAME_LEN {
        tracing::warn!("MC_dbOpenDataBase: name {name:?} too long ({} > {MAX_NAME_LEN})", name.len());
        return Ok(-22); // M_E_BADRECID — closest WIPI parameter-error idiom in this file
    }

    let packaged = read_packaged_database(context, &name).await?;

    let system = context.system();
    let pid = system.pid().to_owned();
    let exists = system.platform().database_repository().exists(&name, &pid).await;

    if !exists && packaged.is_none() && mode == 1 {
        return Ok(-12); // M_E_NOENT
    }

    // Mode 4 (`MC_DB_CREATE`) wipes any prior contents up front unless the
    // DB is backed by a packaged resource. Other modes seed the per-handle
    // buffer with the existing record or packaged data so seek+overlay writes
    // preserve unrelated bytes (multi-slot saves at fixed byte offsets).
    let initial: Vec<u8> = if exists {
        let mut db = system.platform().database_repository().open(&name, &pid).await;
        if mode == 4 && packaged.is_none() {
            db.set(1, &[]).await;
            Vec::new()
        } else if let Some(data) = db.get(1).await {
            data
        } else if let Some(data) = packaged {
            db.set(1, &data).await;
            data
        } else {
            Vec::new()
        }
    } else if let Some(data) = packaged {
        let mut db = system.platform().database_repository().open(&name, &pid).await;
        db.set(1, &data).await;
        data
    } else if matches!(mode, 4 | 8) {
        let mut db = system.platform().database_repository().open(&name, &pid).await;
        // LGT open-or-create initializes the stream record. Its callers query
        // list_record_info immediately and use the size to select first-run setup.
        db.set(1, &[]).await;
        Vec::new()
    } else {
        Vec::new()
    };

    let name_bytes = name.as_bytes();

    let mut handle = DatabaseHandle {
        magic: DATABASE_HANDLE_MAGIC,
        next: context.system().wipi_stream_head(),
        name: [0; 32],
        read_cursor: 0,
        write_cursor: 0,
        buffer_ptr: 0,
        buffer_len: 0,
        buffer_capacity: 0,
    };
    handle.name[..name_bytes.len()].copy_from_slice(name_bytes);

    if !initial.is_empty() {
        let cap = (initial.len() as u32).max(MIN_BUFFER_CAPACITY);
        let buf_ptr = context.alloc_raw(cap)?;
        context.write_bytes(buf_ptr, &initial)?;
        handle.buffer_ptr = buf_ptr;
        handle.buffer_len = initial.len() as u32;
        handle.buffer_capacity = cap;
    }

    let ptr_handle = context.alloc_raw(size_of::<DatabaseHandle>() as _)?;
    write_generic(context, ptr_handle, handle)?;
    context.system().set_wipi_stream_head(ptr_handle);

    tracing::debug!("Created database handle {ptr_handle:#x} for {name}");

    Ok(ptr_handle as _)
}

pub async fn close_database(context: &mut dyn WIPICContext, db_id: i32) -> Result<i32> {
    tracing::debug!("MC_dbCloseDataBase({db_id:#x})");

    let Some(handle) = load_handle(context, db_id)? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };

    let handles = active_handles(context)?;
    let Some(index) = handles.iter().position(|(address, _)| *address == db_id as u32) else {
        return Ok(-25);
    };
    if index == 0 {
        context.system().set_wipi_stream_head(handle.next);
    } else {
        let (address, mut previous) = handles[index - 1];
        previous.next = handle.next;
        write_generic(context, address, previous)?;
    }
    // Invalidate the closed handle before returning its memory to the allocator.
    write_generic(context, db_id as u32, 0u32)?;
    // Writes are already persisted; close releases guest allocations.
    if handle.buffer_ptr != 0 && handle.buffer_capacity > 0 {
        context.free_raw(handle.buffer_ptr, handle.buffer_capacity)?;
    }
    context.free_raw(db_id as _, size_of::<DatabaseHandle>() as _)?;

    Ok(0) // success
}

pub async fn list_record(context: &mut dyn WIPICContext, db_id: i32, buf_ptr: WIPICWord, buf_len: WIPICWord) -> Result<i32> {
    tracing::debug!("MC_dbListRecords({db_id:#x}, {buf_ptr:#x}, {buf_len})");

    let Some(db) = get_database_from_db_id(context, db_id).await? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };
    let ids = db.get_record_ids().await;

    let mut cursor = 0;
    for &id in &ids {
        write_generic(context, buf_ptr + cursor, id)?;
        cursor += size_of::<WIPICWord>() as u32;
    }

    Ok(ids.len() as _)
}

/// Returns the database storage available to a KTF application.
///
/// Although the standard interface names function ID 12 `MC_dbListDataBase`,
/// KTF titles use its no-argument return value as an available-storage byte count.
/// Known callers reject values below 0x100 and 0x1200 respectively.
pub async fn list_databases(context: &mut dyn WIPICContext) -> Result<i32> {
    available_database_storage(context, KTF_DATABASE_STORAGE_LIMIT).await
}

/// Preserve the LGT virtual storage profile independently of KTF's install volume.
pub async fn available_storage_lgt(context: &mut dyn WIPICContext) -> Result<i32> {
    available_database_storage(context, 1024 * 1024).await
}

async fn available_database_storage(context: &mut dyn WIPICContext, capacity: u64) -> Result<i32> {
    let system = context.system();
    let pid = system.pid().to_owned();
    let usage = system.platform().database_repository().usage(&pid).await;
    let available = capacity.saturating_sub(usage).min(i32::MAX as u64) as i32;

    tracing::debug!("MC_dbListDataBase() = {available} (used={usage}, limit={capacity})");
    Ok(available)
}

/// KTF's filesystem table slot 11 is MC_fsTotalSpace (no arguments).
/// It reports capacity, unlike slot 12's remaining-space query.
pub async fn total_space_ktf(_: &mut dyn WIPICContext) -> Result<i32> {
    Ok(KTF_DATABASE_STORAGE_LIMIT as i32)
}

pub async fn seek_record_single(context: &mut dyn WIPICContext, db_id: i32, offset: i32, origin: i32) -> Result<i32> {
    tracing::debug!("MC_dbSeekRecordSingle({db_id:#x}, {offset}, {origin})");

    let Some(mut handle) = load_handle(context, db_id)? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };

    let base = match origin {
        0 => 0,
        1 => handle.read_cursor as i64,
        2 => handle.buffer_len as i64,
        _ => return Ok(-1),
    };
    let position = (base + offset as i64).clamp(0, handle.buffer_len as i64) as u32;
    handle.read_cursor = position;
    handle.write_cursor = position;
    write_generic(context, db_id as _, handle)?;

    Ok(position as i32)
}

pub async fn list_record_info(context: &mut dyn WIPICContext, ptr_name: WIPICWord, buf_ptr: WIPICWord, capacity: WIPICWord) -> Result<i32> {
    tracing::debug!("MC_dbListRecordInfo({ptr_name:#x}, {buf_ptr:#x}, {capacity})");

    let Ok(name) = String::from_utf8(read_null_terminated_string_bytes(context, ptr_name)?) else {
        return Ok(-22);
    };
    let system = context.system();
    let pid = system.pid().to_owned();

    if !system.platform().database_repository().exists(&name, &pid).await {
        if let Some(data) = read_packaged_database(context, &name).await? {
            if capacity > 0 {
                write_generic(context, buf_ptr, 1u32)?;
                write_generic(context, buf_ptr + 4, 0u32)?;
                write_generic(context, buf_ptr + 8, data.len() as u32)?;
            }
            return Ok(0);
        }
        return Ok(-12); // M_E_NOENT
    }

    let db = system.platform().database_repository().open(&name, &pid).await;
    let ids = db.get_record_ids().await;

    let mut written = 0;
    for id in ids {
        if written >= capacity {
            break;
        }

        let Some(data) = db.get(id).await else {
            continue;
        };

        let entry_ptr = buf_ptr + written * 12;
        write_generic(context, entry_ptr, id)?;
        write_generic(context, entry_ptr + 4, 0u32)?;
        write_generic(context, entry_ptr + 8, data.len() as u32)?;
        written += 1;
    }

    Ok(0)
}

pub async fn exists_database(context: &mut dyn WIPICContext, ptr_name: WIPICWord, r#type: i32) -> Result<i32> {
    tracing::debug!("MC_dbExistsDataBase({ptr_name:#x}, {type})");

    let Ok(name) = String::from_utf8(read_null_terminated_string_bytes(context, ptr_name)?) else {
        return Ok(-22);
    };
    if read_packaged_database(context, &name).await?.is_some() {
        return Ok(0);
    }

    let system = context.system();
    let pid = system.pid().to_owned();
    if system.platform().database_repository().exists(&name, &pid).await {
        Ok(0)
    } else {
        Ok(-12) // M_E_NOENT
    }
}

pub async fn stream_write(context: &mut dyn WIPICContext, db_id: i32, buf_ptr: WIPICWord, buf_len: WIPICWord) -> Result<i32> {
    tracing::debug!("db.stream_write({db_id:#x}, {buf_ptr:#x}, {buf_len})");

    let Some(mut handle) = load_handle(context, db_id)? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };

    // Cursor + len is guest-controlled, so guard the arithmetic. An
    // overflowed `new_end` would silently bypass the capacity check below
    // and let a write spill into unrelated guest memory.
    let Some(new_end) = handle.write_cursor.checked_add(buf_len) else {
        return Ok(-22); // M_E_BADRECID — closest "bad parameter" code
    };

    let old_len = handle.buffer_len;

    // Grow the guest-heap buffer if the next write would land past its
    // end. Doubling-on-demand starting from MIN_BUFFER_CAPACITY keeps the
    // realloc count amortized; alloc/free is a guest-side `WIPICContext`
    // primitive so we copy old bytes via host-side scratch.
    if new_end > handle.buffer_capacity {
        let Some(rounded) = new_end.checked_next_power_of_two() else {
            return Ok(-22);
        };
        let new_cap = rounded.max(MIN_BUFFER_CAPACITY);
        let new_ptr = context.alloc_raw(new_cap)?;
        if handle.buffer_len > 0 && handle.buffer_ptr != 0 {
            let mut old_data = vec![0u8; handle.buffer_len as usize];
            context.read_bytes(handle.buffer_ptr, &mut old_data)?;
            context.write_bytes(new_ptr, &old_data)?;
        }
        if handle.buffer_ptr != 0 && handle.buffer_capacity > 0 {
            context.free_raw(handle.buffer_ptr, handle.buffer_capacity)?;
        }
        handle.buffer_ptr = new_ptr;
        handle.buffer_capacity = new_cap;
    }

    // If the write_cursor was seeked past the prior end (e.g. via a slot 4
    // multi-slot save), the bytes between the old end and the cursor were
    // never initialised. `alloc_raw` doesn't guarantee zeroed memory and
    // the snapshot below is flushed straight to disk, so explicitly zero
    // the gap to avoid leaking heap residue into the save file. This must
    // run for `buf_len == 0` too: `new_end == write_cursor` still extends
    // `buffer_len`, so the gap would otherwise be snapshotted uninitialised.
    if handle.write_cursor > old_len {
        let gap_size = (handle.write_cursor - old_len) as usize;
        let zeros = vec![0u8; gap_size];
        context.write_bytes(handle.buffer_ptr + old_len, &zeros)?;
    }

    if buf_len > 0 {
        let mut buf = vec![0u8; buf_len as usize];
        context.read_bytes(buf_ptr, &mut buf)?;
        context.write_bytes(handle.buffer_ptr + handle.write_cursor, &buf)?;
    }

    handle.write_cursor = new_end;
    if new_end > handle.buffer_len {
        handle.buffer_len = new_end;
    }
    write_generic(context, db_id as _, handle)?;

    // Write-through to disk on every stream_write. Some titles tear down
    // the game without making a final `close_database` call after their
    // save sequence — relying on close as the only flush point loses all
    // the writes that landed since the session opened. Flushing eagerly
    // costs an extra small file write per call but keeps the on-disk state
    // consistent if the process exits or the title forgets to close.
    let mut snapshot = vec![0u8; handle.buffer_len as usize];
    if handle.buffer_ptr != 0 && handle.buffer_len > 0 {
        context.read_bytes(handle.buffer_ptr, &mut snapshot)?;
    }
    if let Some(mut db) = open_db_for_handle(context, &handle).await {
        db.set(1, &snapshot).await;
    }

    Ok(buf_len as _)
}

/// Standard WIPI `MC_dbDeleteRecord(handle, rec_id)` — delete a single
/// record by id from an open DB handle.
pub async fn delete_record(context: &mut dyn WIPICContext, db_id: i32, rec_id: i32) -> Result<i32> {
    tracing::debug!("MC_dbDeleteRecord({db_id:#x}, {rec_id})");

    let Some(handle) = load_handle(context, db_id)? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };
    let Some(mut db) = open_db_for_handle(context, &handle).await else {
        return Ok(-25);
    };
    let ok = db.delete(rec_id as u32).await;
    Ok(if ok { 0 } else { -22 })
}

/// KTF's stream slot 6 is MC_fsRemove(name, accessMode). Preserve the existing
/// handle/record form for callers using the record API, without treating an
/// ordinary filename as an open handle.
pub async fn delete_record_ktf(context: &mut dyn WIPICContext, a0: i32, a1: i32) -> Result<i32> {
    let handles = active_handles(context)?;
    if handles.iter().any(|(address, _)| *address == a0 as u32) {
        return delete_record(context, a0, a1).await;
    }
    if a1 != 1 {
        return Ok(-24); // M_E_ACCESS: no access to shared/system directories
    }
    let bytes = read_null_terminated_string_bytes(context, a0 as u32)?;
    let Ok(name) = String::from_utf8(bytes) else {
        return Ok(-3);
    };
    if name.len() > MAX_NAME_LEN {
        return Ok(-11);
    } // M_E_LONGNAME
    if name.starts_with('/') || !context.system().filesystem().is_valid_path(&name) {
        return Ok(-3); // M_E_BADFILENAME
    }
    for (_, handle) in handles {
        let length = handle.name.iter().position(|b| *b == 0).unwrap_or(handle.name.len());
        if &handle.name[..length] == name.as_bytes() {
            return Ok(-1); // MC_fsRemove cannot remove an open file
        }
    }
    let system = context.system();
    let deleted = system.platform().database_repository().delete(&name, system.pid()).await;
    tracing::debug!("MC_fsRemove({name:?}, {a1}) = {deleted}");
    Ok(if deleted { 0 } else { -12 })
}

fn active_handles(context: &mut dyn WIPICContext) -> Result<Vec<(u32, DatabaseHandle)>> {
    let mut result: Vec<(u32, DatabaseHandle)> = Vec::new();
    let mut address = context.system().wipi_stream_head();
    while address != 0 {
        if result.iter().any(|(previous, _)| *previous == address) {
            return Err(wie_util::WieError::FatalError("Cyclic database stream handles".into()));
        }
        let Some(handle) = load_handle(context, address as i32)? else {
            return Err(wie_util::WieError::FatalError("Invalid database stream handle link".into()));
        };
        result.push((address, handle));
        address = handle.next;
    }
    Ok(result)
}

pub async fn delete_database(context: &mut dyn WIPICContext, ptr_name: WIPICWord, flags: i32) -> Result<i32> {
    tracing::debug!("MC_dbDeleteDataBase({ptr_name:#x}, {flags})");

    let Ok(name) = String::from_utf8(read_null_terminated_string_bytes(context, ptr_name)?) else {
        return Ok(-22);
    };
    let system = context.system();
    let pid = system.pid().to_owned();

    let deleted = system.platform().database_repository().delete(&name, &pid).await;
    if deleted || !system.platform().database_repository().exists(&name, &pid).await {
        Ok(0)
    } else {
        Ok(-12) // M_E_NOENT
    }
}

pub async fn update_record(context: &mut dyn WIPICContext, db_id: i32, rec_id: i32, buf_ptr: WIPICWord, buf_len: WIPICWord) -> Result<i32> {
    tracing::debug!("MC_dbUpdateRecord({db_id:#x}, {rec_id}, {buf_ptr:#x}, {buf_len})");

    let Some(handle) = load_handle(context, db_id)? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };
    let Some(mut db) = open_db_for_handle(context, &handle).await else {
        return Ok(-25);
    };
    if rec_id < 0 {
        return Ok(-22);
    }
    let rec_id = rec_id as u32;
    if db.get(rec_id).await.is_none() {
        return Ok(-22);
    }

    let mut buf = vec![0; buf_len as usize];
    context.read_bytes(buf_ptr, &mut buf)?;

    if db.set(rec_id, &buf).await { Ok(0) } else { Ok(-22) }
}

pub async fn select_record(context: &mut dyn WIPICContext, db_id: i32, rec_id: i32, buf_ptr: WIPICWord, buf_len: WIPICWord) -> Result<i32> {
    tracing::debug!("MC_dbSelectRecord({db_id:#x}, {rec_id}, {buf_ptr:#x}, {buf_len})");

    let Some(handle) = load_handle(context, db_id)? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };
    let Some(db) = open_db_for_handle(context, &handle).await else {
        return Ok(-25);
    };
    if rec_id < 0 {
        return Ok(-22);
    }

    if let Some(data) = db.get(rec_id as u32).await {
        if buf_len < data.len() as u32 {
            return Ok(-18); // M_E_SHORTBUF
        }
        context.write_bytes(buf_ptr, &data)?;
        Ok(0)
    } else {
        Ok(-22)
    }
}

pub async fn stream_read(context: &mut dyn WIPICContext, db_id: i32, buf_ptr: WIPICWord, buf_len: WIPICWord) -> Result<i32> {
    tracing::debug!("db.stream_read({db_id:#x}, {buf_ptr:#x}, {buf_len})");

    let Some(mut handle) = load_handle(context, db_id)? else {
        return Ok(-25); // M_E_INVALIDHANDLE
    };

    if handle.read_cursor >= handle.buffer_len {
        // Don't touch buf — caller may have passed a sentinel (NULL) that
        // we shouldn't write to. Some titles do this past EOF.
        return Ok(-23); // M_E_EOF
    }

    let take = core::cmp::min(buf_len, handle.buffer_len - handle.read_cursor);
    if take == 0 {
        return Ok(0);
    }

    // Copy from the guest-heap buffer into the caller's destination via
    // host-side scratch; `WIPICContext` doesn't expose an in-guest memmove.
    let mut data = vec![0u8; take as usize];
    context.read_bytes(handle.buffer_ptr + handle.read_cursor, &mut data)?;
    context.write_bytes(buf_ptr, &data)?;

    handle.read_cursor += take;
    write_generic(context, db_id as _, handle)?;

    Ok(take as _)
}

pub async fn stream_read_ktf(context: &mut dyn WIPICContext, handle: i32, buffer: u32, count: u32) -> Result<i32> {
    let result = stream_read(context, handle, buffer, count).await?;
    if result >= 0
        && let Some(mut state) = load_handle(context, handle)?
    {
        state.write_cursor = state.read_cursor;
        write_generic(context, handle as u32, state)?;
    }
    Ok(result)
}

pub async fn stream_write_ktf(context: &mut dyn WIPICContext, handle: i32, buffer: u32, count: u32) -> Result<i32> {
    let result = stream_write(context, handle, buffer, count).await?;
    if result >= 0
        && let Some(mut state) = load_handle(context, handle)?
    {
        state.read_cursor = state.write_cursor;
        write_generic(context, handle as u32, state)?;
    }
    Ok(result)
}

/// KTF's stream table follows MC_fs*: slots 8 and 10 operate on directories.
pub async fn mkdir_ktf(context: &mut dyn WIPICContext, name_ptr: WIPICWord, _access: i32) -> Result<i32> {
    let Ok(name) = String::from_utf8(read_null_terminated_string_bytes(context, name_ptr)?) else {
        return Ok(-1);
    };
    let system = context.system();
    let pid = system.pid().to_owned();
    Ok(if system.platform().database_repository().create_directory(&name, &pid).await {
        0
    } else {
        -1
    })
}

pub async fn list_directory_ktf(context: &mut dyn WIPICContext, name_ptr: WIPICWord, buf: WIPICWord, capacity: u32, _access: i32) -> Result<i32> {
    let Ok(name) = String::from_utf8(read_null_terminated_string_bytes(context, name_ptr)?) else {
        return Ok(-1);
    };
    let system = context.system();
    let pid = system.pid().to_owned();
    let Some(names) = system.platform().database_repository().list_directory(&name, &pid).await else {
        return Ok(-1);
    };
    let required = names.iter().map(|s| s.len() + 1).sum::<usize>() + if names.is_empty() { 2 } else { 1 };
    if required > capacity as usize {
        return Ok(-1);
    }
    let mut bytes = Vec::with_capacity(required);
    for name in names {
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
    }
    bytes.resize(required, 0);
    context.write_bytes(buf, &bytes)?;
    Ok(0)
}

/// KTF filesystem slot 4: MC_fsSeek(handle, signed offset, origin).
/// The cursor and file bytes remain in guest-backed handle storage.
/// KTF stream-table slot 15: the single-handle position query paired with
/// slot 4 seek. It must not move either cursor or touch persistent contents.
pub async fn tell_ktf(context: &mut dyn WIPICContext, db_id: i32) -> Result<i32> {
    let Some(handle) = load_handle(context, db_id)? else {
        return Ok(-25);
    };
    Ok(handle.read_cursor as i32)
}

pub async fn select_record_ktf(context: &mut dyn WIPICContext, db_id: i32, offset: i32, origin: WIPICWord) -> Result<i32> {
    let Some(mut handle) = load_handle(context, db_id)? else {
        return Ok(-25);
    };
    let base = match origin {
        0 => 0,
        1 => handle.read_cursor as i64,
        2 => handle.buffer_len as i64,
        _ => return Ok(-1),
    };
    let position = base + offset as i64;
    if position < 0 || position > handle.buffer_len as i64 {
        return Ok(-1);
    }
    handle.read_cursor = position as u32;
    handle.write_cursor = position as u32;
    write_generic(context, db_id as u32, handle)?;
    Ok(position as i32)
}

/// Slot 5 — KTF custom `db_stat_by_name`. From observed call shape:
///
/// ```text
/// int32 v2[3];
/// ret = slot5(name_ptr, &v2, mode, fn_self_ptr);
/// if (ret == 0 && v2[2] > 0xC7) "valid save";
/// ```
///
/// Takes a name plus a 12-byte (3-int) output struct, and returns 0 when
/// the DB exists with a non-trivial payload. The third int is treated as a
/// size threshold (must exceed 199 bytes). We fill the struct with
/// `{0, 0, record_size}` and return 0 on hit, -22 on miss.
pub async fn stat_by_name_ktf(context: &mut dyn WIPICContext, name_ptr: WIPICWord, out_buf: WIPICWord, mode: i32, _arg3: i32) -> Result<i32> {
    let name = match read_null_terminated_string_bytes(context, name_ptr) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => return Ok(-22),
        },
        Err(_) => return Ok(-22),
    };

    let system = context.system();
    let pid = system.pid().to_owned();
    let exists = system.platform().database_repository().exists(&name, &pid).await;
    let record_size = if exists {
        system
            .platform()
            .database_repository()
            .open(&name, &pid)
            .await
            .get(1)
            .await
            .map(|v| v.len() as u32)
    } else {
        None
    };
    let record_size = match record_size {
        Some(size) => size,
        None => match context.get_resource_size(&name).await? {
            Some(size) => size as u32,
            None => return Ok(-22),
        },
    };

    if out_buf != 0 {
        write_generic(context, out_buf, 0u32)?;
        write_generic(context, out_buf + 4, 0u32)?;
        write_generic(context, out_buf + 8, record_size)?;
    }

    tracing::debug!("db.stat_by_name({name:?}, mode={mode}) -> 0 (size={record_size})");
    Ok(0)
}

/// MC_fsIsExist returns zero on success and a negative error on absence.
/// It is not a boolean; zero for a missing file takes callers into load paths.
pub async fn exists_database_ktf(context: &mut dyn WIPICContext, name_ptr: WIPICWord, access: i32) -> Result<i32> {
    exists_database(context, name_ptr, access).await
}

/// Read a `DatabaseHandle` from guest memory if `db_id` looks like one.
///
/// Returns `Ok(None)` for any pointer that's obviously not a handle —
/// out-of-range, missing the magic sentinel — so callers can return
/// `M_E_INVALIDHANDLE` instead of panicking on garbage input.
fn load_handle(context: &mut dyn WIPICContext, db_id: i32) -> Result<Option<DatabaseHandle>> {
    if db_id < 0x10000 {
        return Ok(None);
    }
    let handle: DatabaseHandle = read_generic(context, db_id as _)?;
    if handle.magic != DATABASE_HANDLE_MAGIC {
        return Ok(None);
    }
    Ok(Some(handle))
}

async fn open_db_for_handle(context: &mut dyn WIPICContext, handle: &DatabaseHandle) -> Option<Box<dyn Database>> {
    let name_length = handle.name.iter().position(|&c| c == 0).unwrap_or(handle.name.len());
    let db_name = str::from_utf8(&handle.name[..name_length]).ok()?;

    let system = context.system();
    let pid = system.pid().to_owned();

    Some(system.platform().database_repository().open(db_name, &pid).await)
}

async fn get_database_from_db_id(context: &mut dyn WIPICContext, db_id: i32) -> Result<Option<Box<dyn Database>>> {
    let Some(handle) = load_handle(context, db_id)? else {
        return Ok(None);
    };
    Ok(open_db_for_handle(context, &handle).await)
}

async fn read_packaged_database(context: &mut dyn WIPICContext, name: &str) -> Result<Option<Vec<u8>>> {
    if context.get_resource_size(name).await?.is_none() {
        return Ok(None);
    }

    Ok(Some(context.read_resource(name).await?))
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;

    use test_utils::TestPlatform;
    use wie_backend::{DefaultTaskRunner, System};
    use wie_util::{ByteRead, ByteWrite};

    use crate::context::test::TestContext;

    use super::{
        KTF_DATABASE_STORAGE_LIMIT, delete_database, exists_database, list_databases, list_record_info, open_database, select_record, stream_read,
        stream_write, total_space_ktf, update_record,
    };

    #[futures_test::test]
    async fn ktf_tell_tracks_seek_read_write_without_changing_state() {
        let mut context = database_test_context();
        let db = open_test_database(&mut context).await;
        assert_eq!(super::tell_ktf(&mut context, db).await.unwrap(), 0);
        context.write_bytes(0x2000, &[10, 20, 30, 40]).unwrap();
        super::stream_write_ktf(&mut context, db, 0x2000, 4).await.unwrap();
        assert_eq!(super::tell_ktf(&mut context, db).await.unwrap(), 4);
        super::select_record_ktf(&mut context, db, -2, 2).await.unwrap();
        assert_eq!(super::tell_ktf(&mut context, db).await.unwrap(), 2);
        super::stream_read_ktf(&mut context, db, 0x2100, 1).await.unwrap();
        let mut before = [0; core::mem::size_of::<super::DatabaseHandle>()];
        context.read_bytes(db as u32, &mut before).unwrap();
        assert_eq!(super::tell_ktf(&mut context, db).await.unwrap(), 3);
        assert_eq!(super::tell_ktf(&mut context, db).await.unwrap(), 3);
        let mut after = before;
        context.read_bytes(db as u32, &mut after).unwrap();
        assert_eq!(before, after);
        super::select_record_ktf(&mut context, db, 0, 0).await.unwrap();
        super::stream_read_ktf(&mut context, db, 0x2100, 4).await.unwrap();
        let mut bytes = [0; 4];
        context.read_bytes(0x2100, &mut bytes).unwrap();
        assert_eq!(bytes, [10, 20, 30, 40]);
        super::close_database(&mut context, db).await.unwrap();
        assert_eq!(super::tell_ktf(&mut context, db).await.unwrap(), -25);
        assert_eq!(super::tell_ktf(&mut context, -12).await.unwrap(), -25);
    }

    #[futures_test::test]
    async fn ktf_remove_deletes_closed_files_but_preserves_open_or_inaccessible_files() {
        use crate::WIPICContext;
        let mut context = database_test_context();
        context.write_bytes(0x1000, b"cert.data\0").unwrap();
        context.write_bytes(0x1100, b"other\0").unwrap();
        let first = open_database(&mut context, 0x1000, 4, 1).await.unwrap();
        let middle = open_database(&mut context, 0x1100, 4, 1).await.unwrap();
        let last = open_database(&mut context, 0x1000, 1, 1).await.unwrap();
        context.write_bytes(0x2000, &[1, 2, 3]).unwrap();
        stream_write(&mut context, first, 0x2000, 3).await.unwrap();
        assert_eq!(super::delete_record_ktf(&mut context, 0x1000, 2).await.unwrap(), -24);
        assert_eq!(super::delete_record_ktf(&mut context, 0x1000, 1).await.unwrap(), -1);
        super::close_database(&mut context, middle).await.unwrap();
        super::close_database(&mut context, last).await.unwrap();
        assert_eq!(super::delete_record_ktf(&mut context, 0x1000, 1).await.unwrap(), -1);
        super::close_database(&mut context, first).await.unwrap();
        assert_eq!(context.system().wipi_stream_head(), 0);
        let system = context.system();
        assert_eq!(
            system.platform().database_repository().open("cert.data", system.pid()).await.get(1).await,
            Some(alloc::vec![1, 2, 3])
        );
        assert_eq!(super::delete_record_ktf(&mut context, 0x1000, 1).await.unwrap(), 0);
        assert_eq!(super::delete_record_ktf(&mut context, 0x1000, 1).await.unwrap(), -12);
        assert_eq!(super::exists_database_ktf(&mut context, 0x1100, 1).await.unwrap(), 0);
        assert_eq!(super::delete_record_ktf(&mut context, 0x1100, 1).await.unwrap(), 0);
        context.write_bytes(0x1200, b"../escape\0").unwrap();
        assert_eq!(super::delete_record_ktf(&mut context, 0x1200, 1).await.unwrap(), -3);
        context.write_bytes(0x1200, b"abcdefghijklmnopqrstuvwxyz012345\0").unwrap();
        assert_eq!(super::delete_record_ktf(&mut context, 0x1200, 1).await.unwrap(), -11);
        // Existing callers using the open-handle/record form retain that contract.
        let record = open_database(&mut context, 0x1100, 4, 1).await.unwrap();
        assert_eq!(super::delete_record_ktf(&mut context, record, 1).await.unwrap(), 0);
        super::close_database(&mut context, record).await.unwrap();
    }

    #[futures_test::test]
    async fn ktf_exists_and_shared_file_cursor_obey_filesystem_contract() {
        let mut context = database_test_context();
        context.write_bytes(0x1000, b"records\0").unwrap();
        assert_eq!(super::exists_database_ktf(&mut context, 0x1000, 1).await.unwrap(), -12);
        let db = open_database(&mut context, 0x1000, 4, 1).await.unwrap();
        assert_eq!(super::exists_database_ktf(&mut context, 0x1000, 1).await.unwrap(), 0);
        context.write_bytes(0x2000, &[10, 20, 30, 40]).unwrap();
        super::stream_write_ktf(&mut context, db, 0x2000, 4).await.unwrap();
        assert_eq!(super::select_record_ktf(&mut context, db, 0, 1).await.unwrap(), 4);
        super::select_record_ktf(&mut context, db, 0, 0).await.unwrap();
        super::stream_read_ktf(&mut context, db, 0x2100, 1).await.unwrap();
        context.write_bytes(0x2000, &[99]).unwrap();
        super::stream_write_ktf(&mut context, db, 0x2000, 1).await.unwrap();
        assert_eq!(super::select_record_ktf(&mut context, db, 0, 1).await.unwrap(), 2);
        super::select_record_ktf(&mut context, db, 0, 0).await.unwrap();
        super::stream_read_ktf(&mut context, db, 0x2100, 4).await.unwrap();
        let mut bytes = [0; 4];
        context.read_bytes(0x2100, &mut bytes).unwrap();
        assert_eq!(bytes, [10, 99, 30, 40]);
        super::stat_by_name_ktf(&mut context, 0x1000, 0x2200, 1, 0).await.unwrap();
        let size: u32 = wie_util::read_generic(&context, 0x2208).unwrap();
        assert_eq!(size, 4);
    }

    #[futures_test::test]
    async fn ktf_directories_list_names_and_never_overrun_short_buffers() {
        let mut context = database_test_context();
        context.write_bytes(0x1000, b"save\0").unwrap();
        assert_eq!(super::mkdir_ktf(&mut context, 0x1000, 1).await.unwrap(), 0);
        assert!(super::mkdir_ktf(&mut context, 0x1000, 1).await.unwrap() < 0);
        context.write_bytes(0x2000, &[0xaa; 32]).unwrap();
        assert_eq!(super::list_directory_ktf(&mut context, 0x1000, 0x2000, 32, 1).await.unwrap(), 0);
        let mut bytes = [0; 32];
        context.read_bytes(0x2000, &mut bytes).unwrap();
        assert_eq!(&bytes[..3], &[0, 0, 0xaa]);
        for name in [b"save/a\0".as_slice(), b"save/b\0".as_slice()] {
            context.write_bytes(0x1100, name).unwrap();
            let handle = open_database(&mut context, 0x1100, 4, 1).await.unwrap();
            super::close_database(&mut context, handle).await.unwrap();
        }
        context.write_bytes(0x2000, &[0xaa; 32]).unwrap();
        assert!(super::list_directory_ktf(&mut context, 0x1000, 0x2000, 4, 1).await.unwrap() < 0);
        context.read_bytes(0x2000, &mut bytes).unwrap();
        assert_eq!(bytes, [0xaa; 32]);
        assert_eq!(super::list_directory_ktf(&mut context, 0x1000, 0x2000, 5, 1).await.unwrap(), 0);
        context.read_bytes(0x2000, &mut bytes).unwrap();
        assert_eq!(&bytes[..6], b"a\0b\0\0\xaa");
        assert!(super::list_directory_ktf(&mut context, 0x1100, 0x2000, 32, 1).await.unwrap() < 0);
    }

    #[futures_test::test]
    async fn ktf_seek_obeys_origin_returns_position_and_preserves_failed_cursor() {
        let mut context = database_test_context();
        let db = open_test_database(&mut context).await;
        context.write_bytes(0x2000, &[10, 20, 30, 40]).unwrap();
        stream_write(&mut context, db, 0x2000, 4).await.unwrap();
        assert_eq!(super::select_record_ktf(&mut context, db, 0, 2).await.unwrap(), 4);
        assert_eq!(super::select_record_ktf(&mut context, db, -2, 1).await.unwrap(), 2);
        assert_eq!(super::select_record_ktf(&mut context, db, -1, 2).await.unwrap(), 3);
        for (offset, origin) in [(0, 3), (-5, 0), (1, 2), (i32::MAX, 1)] {
            assert!(super::select_record_ktf(&mut context, db, offset, origin).await.unwrap() < 0);
            assert_eq!(super::select_record_ktf(&mut context, db, 0, 1).await.unwrap(), 3);
        }
        assert_eq!(stream_read(&mut context, db, 0x2100, 1).await.unwrap(), 1);
        let mut byte = [0];
        context.read_bytes(0x2100, &mut byte).unwrap();
        assert_eq!(byte, [40]);
        assert_eq!(super::select_record_ktf(&mut context, db, 1, 0).await.unwrap(), 1);
        context.write_bytes(0x2000, &[99]).unwrap();
        stream_write(&mut context, db, 0x2000, 1).await.unwrap();
        super::close_database(&mut context, db).await.unwrap();
        let db = open_database(&mut context, 0x1000, 8, 1).await.unwrap();
        assert_eq!(stream_read(&mut context, db, 0x2100, 4).await.unwrap(), 4);
        let mut bytes = [0; 4];
        context.read_bytes(0x2100, &mut bytes).unwrap();
        assert_eq!(bytes, [10, 99, 30, 40]);
    }

    #[futures_test::test]
    async fn ktf_available_database_storage_tracks_app_usage() {
        let mut context = database_test_context();
        assert_eq!(list_databases(&mut context).await.unwrap(), KTF_DATABASE_STORAGE_LIMIT as i32);
        assert_eq!(total_space_ktf(&mut context).await.unwrap(), KTF_DATABASE_STORAGE_LIMIT as i32);

        let db_id = open_test_database(&mut context).await;
        context.write_bytes(0x2000, &[1, 2, 3, 4]).unwrap();
        assert_eq!(stream_write(&mut context, db_id, 0x2000, 4).await.unwrap(), 4);

        assert_eq!(list_databases(&mut context).await.unwrap(), KTF_DATABASE_STORAGE_LIMIT as i32 - 4);
        assert_eq!(total_space_ktf(&mut context).await.unwrap(), KTF_DATABASE_STORAGE_LIMIT as i32);
    }

    #[futures_test::test]
    async fn ktf_storage_allows_multi_megabyte_installs_and_reclaims_deleted_data() {
        use crate::context::WIPICContext;
        use alloc::borrow::ToOwned;
        let mut context = database_test_context();
        let capacity = total_space_ktf(&mut context).await.unwrap();
        let payload = alloc::vec![0x5a; 3 * 1024 * 1024];
        assert!(capacity > payload.len() as i32);
        {
            let system = context.system();
            let pid = system.pid().to_owned();
            let repository = system.platform().database_repository();
            let mut database = repository.open("installed-data", &pid).await;
            let id = database.add(&payload).await;
            assert_eq!(database.get(id).await.unwrap(), payload);
            // A different application's stores do not consume this budget.
            let mut other = repository.open("installed-data", "another-app").await;
            other.add(&payload).await;
        }
        assert_eq!(list_databases(&mut context).await.unwrap(), capacity - payload.len() as i32);
        assert_eq!(total_space_ktf(&mut context).await.unwrap(), capacity);
        {
            let system = context.system();
            let pid = system.pid().to_owned();
            assert!(system.platform().database_repository().delete("installed-data", &pid).await);
        }
        assert_eq!(list_databases(&mut context).await.unwrap(), capacity);
    }

    #[futures_test::test]
    async fn lgt_exists_database_reports_missing_and_existing_database() {
        let mut context = database_test_context();
        context.write_bytes(0x1000, b"records\0").unwrap();

        assert_eq!(exists_database(&mut context, 0x1000, 1).await.unwrap(), -12);
        let db_id = open_database(&mut context, 0x1000, 0, 0).await.unwrap();
        context.write_bytes(0x2000, &[1]).unwrap();
        assert_eq!(stream_write(&mut context, db_id, 0x2000, 1).await.unwrap(), 1);
        assert_eq!(exists_database(&mut context, 0x1000, 1).await.unwrap(), 0);
    }

    #[futures_test::test]
    async fn lgt_open_or_create_reports_empty_size_and_preserves_written_save() {
        let mut context = database_test_context();
        context.write_bytes(0x1000, b"records\0").unwrap();
        let db_id = open_database(&mut context, 0x1000, 8, 1).await.unwrap();
        assert!(db_id > 0);
        context.write_bytes(0x2100, &[0xaa; 16]).unwrap();
        assert_eq!(list_record_info(&mut context, 0x1000, 0x2100, 1).await.unwrap(), 0);
        let mut info = [0; 16];
        context.read_bytes(0x2100, &mut info).unwrap();
        assert_eq!(&info[..12], &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&info[12..], &[0xaa; 4]);
        context.write_bytes(0x2000, &[4, 5, 6]).unwrap();
        assert_eq!(stream_write(&mut context, db_id, 0x2000, 3).await.unwrap(), 3);
        assert_eq!(super::close_database(&mut context, db_id).await.unwrap(), 0);
        let reopened = open_database(&mut context, 0x1000, 8, 1).await.unwrap();
        assert_eq!(list_record_info(&mut context, 0x1000, 0x2100, 1).await.unwrap(), 0);
        context.read_bytes(0x2100, &mut info).unwrap();
        assert_eq!(&info[8..12], &3u32.to_le_bytes());
        assert_eq!(stream_read(&mut context, reopened, 0x2200, 3).await.unwrap(), 3);
        let mut saved = [0; 3];
        context.read_bytes(0x2200, &mut saved).unwrap();
        assert_eq!(saved, [4, 5, 6]);
    }

    #[futures_test::test]
    async fn lgt_create_mode_materializes_empty_database() {
        let mut context = database_test_context();
        context.write_bytes(0x1000, b"records\0").unwrap();

        assert_eq!(exists_database(&mut context, 0x1000, 1).await.unwrap(), -12);
        let db_id = open_database(&mut context, 0x1000, 4, 0).await.unwrap();
        assert!(db_id > 0);
        assert_eq!(exists_database(&mut context, 0x1000, 1).await.unwrap(), 0);
    }

    #[futures_test::test]
    async fn lgt_update_and_select_record_use_standard_record_ids() {
        let mut context = database_test_context();
        let db_id = open_test_database(&mut context).await;
        context.write_bytes(0x2000, &[1, 2, 3]).unwrap();
        assert_eq!(stream_write(&mut context, db_id, 0x2000, 3).await.unwrap(), 3);
        context.write_bytes(0x2010, &[4, 5]).unwrap();

        assert_eq!(update_record(&mut context, db_id, 1, 0x2010, 2).await.unwrap(), 0);
        assert_eq!(select_record(&mut context, db_id, 1, 0x2100, 2).await.unwrap(), 0);

        let mut data = [0; 2];
        context.read_bytes(0x2100, &mut data).unwrap();
        assert_eq!(data, [4, 5]);
    }

    #[futures_test::test]
    async fn lgt_list_record_info_and_delete_database_use_database_name() {
        let mut context = database_test_context();
        let db_id = open_test_database(&mut context).await;
        context.write_bytes(0x2000, &[1, 2, 3, 4]).unwrap();
        assert_eq!(stream_write(&mut context, db_id, 0x2000, 4).await.unwrap(), 4);

        assert_eq!(list_record_info(&mut context, 0x1000, 0x2100, 1).await.unwrap(), 0);
        let mut entry = [0; 12];
        context.read_bytes(0x2100, &mut entry).unwrap();
        assert_eq!(u32::from_le_bytes(entry[0..4].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(entry[8..12].try_into().unwrap()), 4);

        assert_eq!(delete_database(&mut context, 0x1000, 1).await.unwrap(), 0);
        assert_eq!(exists_database(&mut context, 0x1000, 1).await.unwrap(), -12);
    }

    #[futures_test::test]
    async fn lgt_open_database_materializes_packaged_database() {
        let mut context = database_test_context().with_resource("kickass", b"seed-data");
        context.write_bytes(0x1000, b"kickass\0").unwrap();

        assert_eq!(exists_database(&mut context, 0x1000, 1).await.unwrap(), 0);
        let db_id = open_database(&mut context, 0x1000, 1, 0).await.unwrap();
        assert!(db_id > 0);
        assert_eq!(stream_read(&mut context, db_id, 0x2000, 9).await.unwrap(), 9);

        let mut data = [0; 9];
        context.read_bytes(0x2000, &mut data).unwrap();
        assert_eq!(&data, b"seed-data");
    }

    fn database_test_context() -> TestContext {
        let system = System::new(Box::new(TestPlatform::new()), "test-pid", "test-aid", DefaultTaskRunner);
        TestContext::with_system(system)
    }

    async fn open_test_database(context: &mut TestContext) -> i32 {
        context.write_bytes(0x1000, b"records\0").unwrap();
        open_database(context, 0x1000, 0, 0).await.unwrap()
    }
}
