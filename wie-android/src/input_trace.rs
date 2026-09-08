// GOmul provenance: gomul-component:afb25abf-cc6c-4ab9-b0b9-3a5a3435e6bb (input-diagnostics); see PROVENANCE.json.
// GOmul contributions: Copyright (c) 2026 invi-si. SPDX-License-Identifier: MIT
//! Bounded opt-in diagnostic buffer; no formatting or file writes while recording.
use std::{
    path::Path,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use wie_util::input_trace::Hooks;
const CAPACITY: usize = 1_000_000;
#[derive(Clone, Copy)]
struct Record {
    time: u64,
    tid: i32,
    kind: u16,
    phase: u8,
    id: u64,
    value: u64,
}
static EPOCH: AtomicU64 = AtomicU64::new(0);
static CENSUS: AtomicBool = AtomicBool::new(false);
pub fn census(enabled: bool) {
    CENSUS.store(enabled, Ordering::Relaxed);
}
static ACTIVE: AtomicBool = AtomicBool::new(false);
static LOST: AtomicU64 = AtomicU64::new(0);
static BUFFER: OnceLock<Mutex<Vec<Record>>> = OnceLock::new();
thread_local! {
 static TID: i32 = unsafe { libc::syscall(libc::SYS_gettid) as i32 };
 static AGGREGATE: std::cell::RefCell<(u64,crate::trace_aggregate::Aggregate)> = std::cell::RefCell::new((0,crate::trace_aggregate::Aggregate::default()));
}
pub fn now() -> u64 {
    clock(libc::CLOCK_MONOTONIC)
}
fn clock(id: libc::clockid_t) -> u64 {
    let mut t = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe {
        libc::clock_gettime(id, &mut t);
    }
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn enabled() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}
fn event(kind: u16, phase: u8, id: u64, value: u64) {
    if CENSUS.load(Ordering::Relaxed) && !matches!(kind,10..=14|17|26|60..=65|75|80|87|90|94|99|110..=126) {
        return;
    }
    if matches!(kind, 3 | 24) {
        return;
    }
    let time = now();
    AGGREGATE.with(|state| {
        let mut state = state.borrow_mut();
        let epoch = EPOCH.load(Ordering::Relaxed);
        if state.0 != epoch {
            *state = (epoch, crate::trace_aggregate::Aggregate::default());
        }
        let valid = state.1.event(
            crate::trace_aggregate::Record {
                time,
                kind,
                phase,
                id,
                value,
            },
            |r| {
                append(Record {
                    time: r.time,
                    tid: TID.with(|v| *v),
                    kind: r.kind,
                    phase: r.phase,
                    id: r.id,
                    value: r.value,
                })
            },
        );
        if !valid {
            LOST.fetch_add(1, Ordering::Relaxed);
            state.1 = crate::trace_aggregate::Aggregate::default();
        }
    });
}
fn append(r: Record) {
    if let Some(buffer) = BUFFER.get() {
        let mut buffer = buffer.lock().unwrap();
        if ACTIVE.load(Ordering::Relaxed) {
            if buffer.len() < CAPACITY {
                buffer.push(r);
            } else {
                LOST.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
static HOOKS: Hooks = Hooks { enabled, event };
pub fn start() {
    ACTIVE.store(false, Ordering::Relaxed);
    let buffer = BUFFER.get_or_init(|| Mutex::new(Vec::with_capacity(CAPACITY)));
    buffer.lock().unwrap().clear();
    LOST.store(0, Ordering::Relaxed);
    EPOCH.fetch_add(1, Ordering::Relaxed);
    wie_util::input_trace::install(&HOOKS);
    ACTIVE.store(true, Ordering::Relaxed);
    event(99, b'I', clock(libc::CLOCK_BOOTTIME), now());
}
pub fn stop(path: &Path) -> anyhow::Result<()> {
    event(99, b'I', clock(libc::CLOCK_BOOTTIME), now());
    ACTIVE.store(false, Ordering::Relaxed);
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    use std::io::Write;
    writeln!(out, "# monotonic_ns,tid,kind,phase,id,value; lost={}", LOST.load(Ordering::Relaxed))?;
    if let Some(buffer) = BUFFER.get() {
        for r in buffer.lock().unwrap().iter() {
            writeln!(out, "{},{},{},{},{},{}", r.time, r.tid, r.kind, r.phase as char, r.id, r.value)?;
        }
    }
    out.flush()?;
    Ok(())
}
