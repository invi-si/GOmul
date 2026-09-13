//! WIPI fixed-size record stores, distinct from the KTF filesystem stream table.
use crate::context::WIPICContext;
use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
use bytemuck::{Pod, Zeroable};
use core::mem::size_of;
use wie_backend::Database;
use wie_util::{Result, read_generic, read_null_terminated_string_bytes, write_generic};

const MAGIC: u32 = 0x57444231;
// Guest record IDs are positive signed integers, so this cannot be a guest ID.
const SIZE_RECORD: u32 = u32::MAX;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Handle {
    magic: u32,
    next: u32,
    record_size: u32,
    mode: i32,
    name: [u8; 256],
}

fn normalize_name(bytes: Vec<u8>) -> Option<String> {
    let raw = String::from_utf8(bytes).ok()?;
    if raw.contains('\\') || raw.split('/').any(|part| part == "..") {
        return None;
    }
    let name = raw
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/");
    if name.is_empty() || name.len() >= 256 { None } else { Some(name) }
}

fn name(handle: &Handle) -> &str {
    core::str::from_utf8(&handle.name[..handle.name.iter().position(|b| *b == 0).unwrap_or(256)]).unwrap()
}
async fn repository(context: &mut dyn WIPICContext, handle: &Handle) -> Box<dyn Database> {
    let system = context.system();
    let pid: String = system.pid().into();
    system
        .platform()
        .database_repository()
        .open(&format!("wipi-records/{}", name(handle)), &pid)
        .await
}
fn handle(context: &mut dyn WIPICContext, id: i32) -> Result<Option<Handle>> {
    if id < 0x10000 {
        return Ok(None);
    }
    let value: Handle = read_generic(context, id as u32)?;
    if value.magic != MAGIC || value.record_size == 0 || value.record_size > context.total_memory() {
        return Ok(None);
    }
    let end = value.name.iter().position(|b| *b == 0).unwrap_or(256);
    if core::str::from_utf8(&value.name[..end]).is_err() {
        return Ok(None);
    }
    Ok(Some(value))
}

pub async fn open(context: &mut dyn WIPICContext, ptr: u32, size: i32, create: i32, mode: i32) -> Result<i32> {
    if ptr == 0 {
        return Ok(-1);
    }
    let Some(name) = normalize_name(read_null_terminated_string_bytes(context, ptr)?) else {
        return Ok(-1);
    };
    if name.is_empty() || name.len() >= 256 {
        return Ok(-1);
    }
    let max_size = context.total_memory();
    let system = context.system();
    let pid: String = system.pid().into();
    let key = format!("wipi-records/{name}");
    let exists = system.platform().database_repository().exists(&key, &pid).await;
    if !exists && create == 0 {
        return Ok(-12);
    }
    if !exists && (size <= 0 || size as u32 > max_size) {
        return Ok(-1);
    }
    let mut db = system.platform().database_repository().open(&key, &pid).await;
    let size = if exists {
        let Some(bytes) = db.get(SIZE_RECORD).await else {
            return Ok(-1);
        };
        let Ok(bytes) = <[u8; 4]>::try_from(bytes.as_slice()) else {
            return Ok(-1);
        };
        u32::from_le_bytes(bytes)
    } else {
        if !db.set(SIZE_RECORD, &(size as u32).to_le_bytes()).await {
            return Ok(-1);
        }
        size as u32
    };
    if size == 0 || size > context.total_memory() {
        return Ok(-1);
    }
    let mut value = Handle {
        magic: MAGIC,
        next: context.system().wipi_record_head(),
        record_size: size,
        mode,
        name: [0; 256],
    };
    value.name[..name.len()].copy_from_slice(name.as_bytes());
    let ptr = context.alloc_raw(size_of::<Handle>() as u32)?;
    write_generic(context, ptr, value)?;
    context.system().set_wipi_record_head(ptr);
    Ok(ptr as i32)
}

pub async fn close(context: &mut dyn WIPICContext, id: i32) -> Result<i32> {
    if handle(context, id)?.is_none() {
        return Ok(-25);
    }
    let mut current = context.system().wipi_record_head();
    let mut previous = None;
    while current != 0 {
        let Some(h) = handle(context, current as i32)? else {
            return Ok(-25);
        };
        if current == id as u32 {
            if let Some(mut parent) = previous {
                let (parent_ptr, ref mut state): (u32, Handle) = parent;
                state.next = h.next;
                write_generic(context, parent_ptr, *state)?;
            } else {
                context.system().set_wipi_record_head(h.next);
            }
            break;
        }
        previous = Some((current, h));
        current = h.next;
    }
    write_generic(context, id as u32, 0u32)?;
    context.free_raw(id as u32, size_of::<Handle>() as u32)?;
    Ok(0)
}
pub async fn delete(context: &mut dyn WIPICContext, ptr: u32, _mode: i32) -> Result<i32> {
    if ptr == 0 {
        return Ok(-1);
    }
    let Some(requested) = normalize_name(read_null_terminated_string_bytes(context, ptr)?) else {
        return Ok(-1);
    };
    let mut current = context.system().wipi_record_head();
    while current != 0 {
        let Some(h) = handle(context, current as i32)? else {
            return Ok(-25);
        };
        if name(&h).trim_start_matches('/') == requested.trim_start_matches('/') {
            return Ok(-1);
        }
        current = h.next;
    }
    let system = context.system();
    let pid: String = system.pid().into();
    let key = format!("wipi-records/{requested}");
    if !system.platform().database_repository().exists(&key, &pid).await {
        return Ok(-12);
    }
    Ok(if system.platform().database_repository().delete(&key, &pid).await {
        0
    } else {
        -1
    })
}

async fn data(context: &mut dyn WIPICContext, handle: &Handle, ptr: u32, len: i32) -> Result<Option<Vec<u8>>> {
    if ptr == 0 || len <= 0 || len as u32 > handle.record_size {
        return Ok(None);
    }
    let mut bytes = vec![0; handle.record_size as usize];
    context.read_bytes(ptr, &mut bytes[..len as usize])?;
    Ok(Some(bytes))
}
pub async fn insert(context: &mut dyn WIPICContext, id: i32, ptr: u32, len: i32) -> Result<i32> {
    let Some(h) = handle(context, id)? else {
        return Ok(-25);
    };
    let Some(bytes) = data(context, &h, ptr, len).await? else {
        return Ok(-1);
    };
    let mut db = repository(context, &h).await;
    let id = db.next_id().await;
    if id > i32::MAX as u32 || !db.set(id, &bytes).await {
        return Ok(-1);
    }
    Ok(id as i32)
}
pub async fn select(context: &mut dyn WIPICContext, id: i32, record: i32, ptr: u32, len: i32) -> Result<i32> {
    let Some(h) = handle(context, id)? else {
        return Ok(-25);
    };
    if record <= 0 || ptr == 0 || len < h.record_size as i32 {
        return Ok(-1);
    }
    let db = repository(context, &h).await;
    let Some(bytes) = db.get(record as u32).await else {
        return Ok(-22);
    };
    if bytes.len() != h.record_size as usize {
        return Ok(-1);
    }
    context.write_bytes(ptr, &bytes)?;
    Ok(0)
}
pub async fn update(context: &mut dyn WIPICContext, id: i32, record: i32, ptr: u32, len: i32) -> Result<i32> {
    let Some(h) = handle(context, id)? else {
        return Ok(-25);
    };
    if record <= 0 {
        return Ok(-22);
    }
    let Some(bytes) = data(context, &h, ptr, len).await? else {
        return Ok(-1);
    };
    let mut db = repository(context, &h).await;
    if db.get(record as u32).await.is_none() {
        return Ok(-22);
    }
    Ok(if db.set(record as u32, &bytes).await { 0 } else { -1 })
}
pub async fn delete_record(context: &mut dyn WIPICContext, id: i32, record: i32) -> Result<i32> {
    let Some(h) = handle(context, id)? else {
        return Ok(-25);
    };
    if record <= 0 {
        return Ok(-22);
    }
    Ok(if repository(context, &h).await.delete(record as u32).await {
        0
    } else {
        -22
    })
}
pub async fn count(context: &mut dyn WIPICContext, id: i32) -> Result<i32> {
    let Some(h) = handle(context, id)? else {
        return Ok(-25);
    };
    Ok(repository(context, &h)
        .await
        .get_record_ids()
        .await
        .iter()
        .filter(|&&id| id != SIZE_RECORD)
        .count() as i32)
}
pub async fn record_size(context: &mut dyn WIPICContext, id: i32) -> Result<i32> {
    Ok(handle(context, id)?.map_or(-25, |h| h.record_size as i32))
}
pub async fn access_mode(context: &mut dyn WIPICContext, id: i32) -> Result<i32> {
    Ok(handle(context, id)?.map_or(-25, |h| h.mode))
}
pub async fn list(context: &mut dyn WIPICContext, id: i32, ptr: u32, capacity: i32) -> Result<i32> {
    let Some(h) = handle(context, id)? else {
        return Ok(-2); // M_E_BADFD
    };
    let mut ids = repository(context, &h).await.get_record_ids().await;
    ids.retain(|id| *id != SIZE_RECORD);
    ids.sort();
    // The capacity counts M_Int32 entries, not bytes. Native callers pass N
    // for an N-element record-ID array; each returned entry occupies four bytes.
    if ptr == 0 || capacity <= 0 {
        return Ok(-9); // M_E_INVALID
    }
    if (capacity as usize) < ids.len() {
        return Ok(-18); // M_E_SHORTBUF
    }
    for (n, id) in ids.iter().enumerate() {
        write_generic(context, ptr + n as u32 * 4, *id)?;
    }
    Ok(ids.len() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::test::TestContext;
    use wie_util::{ByteRead, ByteWrite};
    #[futures_test::test]
    async fn list_capacity_counts_record_ids_and_preserves_guard_bytes() {
        let mut c = TestContext::with_system(wie_backend::System::new(
            Box::new(test_utils::TestPlatform::new()),
            "record-list-test",
            "record-list-test",
            wie_backend::DefaultTaskRunner,
        ));
        c.write_bytes(0x1000, b"multiple\0").unwrap();
        let db = open(&mut c, 0x1000, 4, 1, 1).await.unwrap();
        let mut expected = Vec::new();
        for n in 0..12u32 {
            write_generic(&mut c, 0x2000, n).unwrap();
            let id = insert(&mut c, db, 0x2000, 4).await.unwrap();
            expected.extend_from_slice(&(id as u32).to_le_bytes());
        }
        c.write_bytes(0x2100, &[0xa5; 56]).unwrap();
        assert_eq!(list(&mut c, db, 0x2104, 11).await.unwrap(), -18);
        let mut bytes = [0; 56];
        c.read_bytes(0x2100, &mut bytes).unwrap();
        assert_eq!(bytes, [0xa5; 56]);
        assert_eq!(list(&mut c, db, 0x2104, 12).await.unwrap(), 12);
        c.read_bytes(0x2100, &mut bytes).unwrap();
        assert_eq!(&bytes[..4], &[0xa5; 4]);
        assert_eq!(&bytes[4..52], expected.as_slice());
        assert_eq!(&bytes[52..], &[0xa5; 4]);
        assert_eq!(list(&mut c, db, 0, 12).await.unwrap(), -9);
        assert_eq!(list(&mut c, db, 0x2104, 0).await.unwrap(), -9);
        assert_eq!(list(&mut c, db, 0x2104, -1).await.unwrap(), -9);
        assert_eq!(list(&mut c, 0, 0x2104, 12).await.unwrap(), -2);
        close(&mut c, db).await.unwrap();
        let reopened = open(&mut c, 0x1000, 4, 0, 1).await.unwrap();
        assert_eq!(list(&mut c, reopened, 0x2104, 12).await.unwrap(), 12);
        c.read_bytes(0x2104, &mut bytes[..48]).unwrap();
        assert_eq!(&bytes[..48], expected.as_slice());
        close(&mut c, reopened).await.unwrap();
    }
    #[futures_test::test]
    async fn records_persist_size_reuse_ids_and_protect_short_buffers() {
        let mut c = TestContext::with_system(wie_backend::System::new(
            Box::new(test_utils::TestPlatform::new()),
            "records-test",
            "records-test",
            wie_backend::DefaultTaskRunner,
        ));
        c.write_bytes(0x1000, b"sample\0").unwrap();
        assert_eq!(open(&mut c, 0x1000, 4, 0, 1).await.unwrap(), -12);
        let db = open(&mut c, 0x1000, 4, 1, 1).await.unwrap();
        assert!(db > 0);
        c.write_bytes(0x2000, &[1, 2, 3, 4, 5]).unwrap();
        assert!(insert(&mut c, db, 0x2000, 5).await.unwrap() < 0);
        let id = insert(&mut c, db, 0x2000, 3).await.unwrap();
        assert!(id > 0);
        assert_eq!(count(&mut c, db).await.unwrap(), 1);
        c.write_bytes(0x2100, &[0xaa; 8]).unwrap();
        assert!(select(&mut c, db, id, 0x2100, 3).await.unwrap() < 0);
        let mut bytes = [0; 8];
        c.read_bytes(0x2100, &mut bytes).unwrap();
        assert_eq!(bytes, [0xaa; 8]);
        assert_eq!(select(&mut c, db, id, 0x2100, 8).await.unwrap(), 0);
        c.read_bytes(0x2100, &mut bytes).unwrap();
        assert_eq!(bytes, [1, 2, 3, 0, 0xaa, 0xaa, 0xaa, 0xaa]);
        close(&mut c, db).await.unwrap();
        let db = open(&mut c, 0x1000, 99, 0, 1).await.unwrap();
        assert_eq!(record_size(&mut c, db).await.unwrap(), 4);
        assert_eq!(list(&mut c, db, 0x2200, 1).await.unwrap(), 1);
        assert_eq!(delete_record(&mut c, db, id).await.unwrap(), 0);
        assert_eq!(count(&mut c, db).await.unwrap(), 0);
        assert_eq!(insert(&mut c, db, 0x2000, 4).await.unwrap(), id);
        assert!(delete(&mut c, 0x1000, 1).await.unwrap() < 0);
        let second = open(&mut c, 0x1000, 4, 0, 1).await.unwrap();
        close(&mut c, db).await.unwrap();
        assert!(delete(&mut c, 0x1000, 1).await.unwrap() < 0);
        close(&mut c, second).await.unwrap();
        assert_eq!(delete(&mut c, 0x1000, 1).await.unwrap(), 0);
        assert_eq!(open(&mut c, 0x1000, 4, 0, 1).await.unwrap(), -12);
        assert_eq!(c.system().wipi_record_head(), 0);
    }
}
