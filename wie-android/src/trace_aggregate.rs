// GOmul provenance: gomul-component:afb25abf-cc6c-4ab9-b0b9-3a5a3435e6bb (input-diagnostics); see PROVENANCE.json.
// GOmul contributions: Copyright (c) 2026 invi-si. SPDX-License-Identifier: MIT
//! Allocation-free aggregation of active CPU/host polls within executor task polls.
//! Short tasks still contribute to tick totals. Detailed task intervals >=250 us survive.
#[derive(Clone, Copy, Default)]
pub struct Record {
    pub time: u64,
    pub kind: u16,
    pub phase: u8,
    pub id: u64,
    pub value: u64,
}
#[derive(Clone, Copy, Default)]
struct Stage {
    id: u64,
    kind: u16,
    cpu: [u64; 7],
    run_lengths: [u64; 8],
    count: u64,
    total: u64,
    max: u64,
}
pub struct Aggregate {
    stages: [Stage; 64],
    polls: [Record; 64],
    poll_depth: usize,
    callback: Option<usize>,
    callback_last: u64,
    callback_parents: [Option<usize>; 64],
    task: Option<Record>,
    stats: [u64; 8],
    last: u64,
    stack: [(u16, u64, u64); 64],
    depth: usize,
    tick: Option<u64>,
    totals: [u64; 10],
}
impl Default for Aggregate {
    fn default() -> Self {
        Self {
            stages: [Stage::default(); 64],
            polls: [Record::default(); 64],
            poll_depth: 0,
            callback: None,
            callback_last: 0,
            callback_parents: [None; 64],
            task: None,
            stats: [0; 8],
            last: 0,
            stack: [(0, 0, 0); 64],
            depth: 0,
            tick: None,
            totals: [0; 10],
        }
    }
}
impl Aggregate {
    fn charge_callback(&mut self, now: u64) {
        if let Some(owner) = self.callback {
            if self.depth > 0 {
                let category = match self.stack[self.depth - 1].0 {
                    5 => Some(0),
                    6 => Some(1),
                    _ => None,
                };
                if let Some(category) = category {
                    self.stages[owner].cpu[category] += now.saturating_sub(self.callback_last);
                }
            }
        }
        self.callback_last = now;
    }
    /// False indicates an invalid nesting/capacity boundary; discard the recording.
    pub fn event(&mut self, r: Record, mut emit: impl FnMut(Record)) -> bool {
        if matches!(r.kind, 4 | 5 | 6 | 84) {
            self.charge_callback(r.time);
        }
        if matches!(r.kind, 3 | 24) {
            return true;
        }
        // Ledger active polls can also be frequent. Keep total/count/max for every
        // operation, retaining individual intervals only when >=250 us.
        if matches!(r.kind, 72 | 74 | 75 | 76 | 77 | 88 | 89) {
            if r.phase == b'B' {
                let Some(slot) = self.stages.iter_mut().find(|s| s.id == 0) else {
                    return false;
                };
                *slot = Stage {
                    id: r.id,
                    kind: r.kind,
                    ..Stage::default()
                };
            } else if r.phase == b'E' {
                if let Some(slot) = self.stages.iter_mut().find(|s| s.id == r.id) {
                    for (kind, value) in [(91, slot.count), (92, slot.total), (93, slot.max)] {
                        emit(Record {
                            kind,
                            phase: b'I',
                            value,
                            ..r
                        });
                    }
                    if slot.kind == 75 {
                        for (n, value) in slot.cpu.iter().enumerate() {
                            emit(Record {
                                kind: 130 + n as u16,
                                phase: b'I',
                                value: *value,
                                ..r
                            });
                        }
                    }
                    if slot.kind == 75 {
                        for (n, value) in slot.run_lengths.iter().enumerate() {
                            emit(Record {
                                kind: 140 + n as u16,
                                phase: b'I',
                                value: *value,
                                ..r
                            });
                        }
                    }
                    *slot = Stage::default();
                }
            }
        }
        if r.kind == 84 {
            if r.phase == b'B' {
                if self.poll_depth == 64 {
                    return false;
                }
                self.callback_parents[self.poll_depth] = self.callback;
                if let Some(owner) = self.stages.iter().position(|s| s.id == r.value && s.kind == 75) {
                    self.callback = Some(owner);
                }
                self.polls[self.poll_depth] = r;
                self.poll_depth += 1;
            } else if r.phase == b'E' && self.poll_depth > 0 {
                let begin = self.polls[self.poll_depth - 1];
                if begin.id != r.id {
                    return false;
                }
                self.poll_depth -= 1;
                self.callback = self.callback_parents[self.poll_depth];
                let duration = r.time.saturating_sub(begin.time);
                if let Some(slot) = self.stages.iter_mut().find(|s| s.id == begin.value && s.id != 0) {
                    slot.count += 1;
                    slot.total += duration;
                    slot.max = slot.max.max(duration);
                }
                if duration >= 250_000 {
                    emit(begin);
                    emit(r);
                }
            }
            return true;
        }
        if r.kind == 1 && r.phase == b'B' {
            self.tick = Some(r.id);
            self.totals = [0; 10];
        }
        if matches!(r.kind, 4 | 5 | 6) && self.task.is_some() {
            let category = if self.depth == 0 {
                2
            } else {
                match self.stack[self.depth - 1].0 {
                    5 => 0,
                    6 => 1,
                    _ => 2,
                }
            };
            self.stats[category] += r.time.saturating_sub(self.last);
            self.last = r.time;
        }
        match (r.kind, r.phase) {
            (4, b'B') => {
                if self.task.is_some() {
                    return false;
                }
                self.task = Some(r);
                self.stats = [0; 8];
                self.depth = 0;
                self.last = r.time;
                return true;
            }
            (4, b'E') => {
                if let Some(begin) = self.task.take() {
                    if begin.id != r.id || self.depth != 0 {
                        return false;
                    }
                    let duration = r.time.saturating_sub(begin.time);
                    for (total, part) in self.totals.iter_mut().zip(self.stats) {
                        *total += part;
                    }
                    self.totals[8] += 1;
                    self.totals[9] = self.totals[9].max(duration);
                    if duration >= 250_000 {
                        emit(begin);
                        emit(r);
                        for (n, value) in self.stats.iter().enumerate() {
                            emit(Record {
                                kind: 40 + n as u16,
                                phase: b'I',
                                value: *value,
                                ..r
                            });
                        }
                    }
                }
                return true;
            }
            (5 | 6, b'B') => {
                if self.task.is_some() {
                    if self.depth == 64 {
                        return false;
                    }
                    self.stack[self.depth] = (r.kind, r.id, r.time);
                    self.depth += 1;
                    if r.kind == 5 {
                        self.stats[3] += 1;
                        if let Some(owner) = self.callback {
                            self.stages[owner].cpu[2] += 1;
                        }
                    }
                }
                return true;
            }
            (5 | 6, b'E') => {
                if self.task.is_some() {
                    if self.depth == 0 || (self.stack[self.depth - 1].0, self.stack[self.depth - 1].1) != (r.kind, r.id) {
                        return false;
                    }
                    self.depth -= 1;
                    if r.kind == 5 {
                        self.stats[4] += r.value;
                        if let Some(owner) = self.callback {
                            self.stages[owner].cpu[3] += r.value;
                            let bucket = match r.value {
                                0 => 0,
                                1..=16 => 1,
                                17..=64 => 2,
                                65..=256 => 3,
                                257..=1024 => 4,
                                1025..=4096 => 5,
                                4097..=9999 => 6,
                                _ => 7,
                            };
                            self.stages[owner].run_lengths[bucket] += 1;
                        }
                    } else {
                        self.stats[if r.value == 1 { 6 } else { 7 }] += 1;
                        if let Some(owner) = self.callback {
                            self.stages[owner].cpu[if r.value == 1 { 5 } else { 6 }] += 1;
                        }
                    }
                }
                return true;
            }
            (18, _) => {
                if self.task.is_some() && r.value >= 0x100000000 {
                    self.stats[5] += 1;
                    if let Some(owner) = self.callback {
                        self.stages[owner].cpu[4] += 1;
                    }
                }
                return true;
            }
            (1, b'E') => {
                if self.tick.take() == Some(r.id) {
                    for (n, value) in self.totals.iter().enumerate() {
                        emit(Record {
                            kind: 50 + n as u16,
                            phase: b'I',
                            value: *value,
                            ..r
                        });
                    }
                }
            }
            _ => {}
        }
        emit(r);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(events: &[(u64, u16, u8, u64, u64)]) -> Vec<Record> {
        let mut agg = Aggregate::default();
        let mut out = Vec::new();
        for &(time, kind, phase, id, value) in events {
            assert!(agg.event(
                Record {
                    time,
                    kind,
                    phase,
                    id,
                    value
                },
                |r| out.push(r)
            ));
        }
        out
    }
    #[test]
    fn ledger_short_active_polls_are_aggregated_and_long_polls_retained() {
        let r = run(&[
            (0, 75, b'B', 7, 8),
            (1, 84, b'B', 8, 7),
            (10, 84, b'E', 8, 0),
            (20, 84, b'B', 9, 7),
            (300_020, 84, b'E', 9, 1),
            (400_000, 75, b'E', 7, 0),
        ]);
        assert_eq!(r.iter().filter(|r| r.kind == 84).count(), 2);
        assert_eq!(r.iter().find(|r| r.kind == 91).unwrap().value, 2);
        assert_eq!(r.iter().find(|r| r.kind == 92).unwrap().value, 300_009);
        assert_eq!(r.iter().find(|r| r.kind == 93).unwrap().value, 300_000);
    }
    #[test]
    fn nested_active_polls_are_exclusive() {
        let r = run(&[
            (0, 1, b'B', 1, 0),
            (0, 4, b'B', 2, 9),
            (100_000, 6, b'B', 3, 1),
            (200_000, 5, b'B', 4, 10000),
            (400_000, 5, b'E', 4, 7),
            (500_000, 6, b'E', 3, 1),
            (600_000, 4, b'E', 2, 0),
            (600_000, 1, b'E', 1, 0),
        ]);
        for (kind, value) in [
            (40, 200_000),
            (41, 200_000),
            (42, 200_000),
            (43, 1),
            (44, 7),
            (46, 1),
            (58, 1),
            (59, 600_000),
        ] {
            assert_eq!(r.iter().find(|r| r.kind == kind).unwrap().value, value);
        }
    }
    #[test]
    fn short_polls_count_without_detailed_records() {
        let r = run(&[
            (0, 1, b'B', 1, 0),
            (1, 4, b'B', 2, 9),
            (2, 5, b'B', 3, 100),
            (3, 5, b'E', 3, 1),
            (4, 4, b'E', 2, 1),
            (5, 1, b'E', 1, 0),
        ]);
        assert!(!r.iter().any(|r| matches!(r.kind, 4 | 5 | 6 | 40..=47)));
        assert_eq!(r.iter().find(|r| r.kind == 54).unwrap().value, 1);
    }
    #[test]
    fn trace_edge_does_not_make_fake_tick_totals() {
        let r = run(&[(1, 5, b'E', 3, 1), (2, 4, b'E', 2, 0), (3, 1, b'E', 1, 0)]);
        assert!(!r.iter().any(|r| matches!(r.kind, 50..=59)));
    }
    #[test]
    fn callback_cost_is_exclusive_and_does_not_leak_across_suspension() {
        let r = run(&[
            (0, 4, b'B', 1, 0),
            (1, 75, b'B', 2, 99),
            (2, 84, b'B', 3, 2),
            (3, 6, b'B', 4, 0),
            (5, 5, b'B', 5, 0),
            (12, 5, b'E', 5, 100),
            (13, 18, b'I', 5, 0x100000000),
            (15, 6, b'E', 4, 0),
            (16, 84, b'E', 3, 0),
            (17, 4, b'E', 1, 0),
            // An unrelated task during suspension must not be attributed to callback 2.
            (20, 4, b'B', 6, 0),
            (21, 5, b'B', 7, 0),
            (99, 5, b'E', 7, 999),
            (100, 4, b'E', 6, 0),
            (110, 4, b'B', 8, 0),
            (111, 84, b'B', 9, 2),
            (112, 6, b'B', 10, 0),
            (116, 6, b'E', 10, 1),
            (117, 84, b'E', 9, 1),
            (118, 75, b'E', 2, 0),
            (119, 4, b'E', 8, 0),
        ]);
        assert_eq!(r.iter().find(|r| r.kind == 143 && r.id == 2).unwrap().value, 1);
        for (kind, value) in [(130, 7), (131, 9), (132, 1), (133, 100), (134, 1), (135, 1), (136, 1)] {
            assert_eq!(r.iter().find(|r| r.kind == kind && r.id == 2).unwrap().value, value);
        }
    }

    #[test]
    fn mismatched_scopes_are_rejected() {
        let mut a = Aggregate::default();
        assert!(a.event(
            Record {
                kind: 4,
                phase: b'B',
                id: 1,
                ..Record::default()
            },
            |_| {}
        ));
        assert!(!a.event(
            Record {
                kind: 4,
                phase: b'E',
                id: 2,
                ..Record::default()
            },
            |_| {}
        ));
    }
}
