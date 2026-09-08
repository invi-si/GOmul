//! Default-off bounded Thumb superblocks. No guest addresses or game IDs are special-cased.
//! Each cached block is guarded by its complete current byte sequence. All stores
//! terminate a block, so even a write to the next instruction is observed before
//! another cached operation executes. Guest memory is single-threaded in run().
use super::{Arm32CpuMemory, EmulatedMemory, PAGE_MASK, PAGE_SIZE};
use alloc::{boxed::Box, vec::Vec};
use arm32_cpu::{Cpu, Memory, Mode, reg};

const SLOTS: usize = 1024;
const MAX_HALFWORDS: usize = 24;

#[derive(Clone, Copy)]
struct Load {
    dst: usize,
    base: usize,
    offset: u32,
    index: Option<usize>,
    width: u8,
}
#[derive(Clone, Copy)]
enum Op {
    Load(Load),
    LoadPair(Load, Load),
    Mov(usize, usize),
    Imm { kind: u16, dst: usize, value: u32 },
    Lsl { dst: usize, src: usize, shift: u32 },
    Cmp(usize, usize),
    CmpBranch { left: usize, right: usize, equal: bool, delta: i32 },
    Branch { condition: Option<bool>, delta: i32 },
    StoreHalf { src: usize, base: usize, offset: u32 },
}
impl Op {
    fn steps(self) -> u32 {
        if matches!(self, Self::LoadPair(..) | Self::CmpBranch { .. }) {
            2
        } else {
            1
        }
    }
    fn ends(self) -> bool {
        matches!(self, Self::Branch { .. } | Self::CmpBranch { .. } | Self::StoreHalf { .. })
    }
}
struct Block {
    pc: u32,
    bytes: Vec<u8>,
    ops: Vec<Op>,
}
pub(super) struct BlockCache {
    entries: Box<[Option<Block>]>,
    // Runtime switch is used by differential tests; ordinary builds omit this path entirely.
    #[cfg(test)]
    pub enabled: bool,
}
impl BlockCache {
    pub fn new() -> Self {
        Self {
            entries: (0..SLOTS).map(|_| None).collect(),
            #[cfg(test)]
            enabled: true,
        }
    }
    pub fn run(&mut self, cpu: &mut Cpu, mem: &mut EmulatedMemory, end: u32, count: u32) -> Option<(u32, Option<u32>)> {
        #[cfg(test)]
        if !self.enabled {
            return None;
        }
        let cpsr = cpu.reg_get(Mode::User, reg::CPSR);
        // Only the user/system bank for the first prototype. No bank/mode changes
        // are supported; exceptions, SVC and high-PC operations use the oracle.
        if cpsr & 0x20 == 0 || !matches!(cpsr & 31, 0x10 | 0x1f) {
            return None;
        }
        let pc = cpu.reg_get(Mode::User, reg::PC);
        if pc & 1 != 0 {
            return None;
        }
        let offset = (pc & PAGE_MASK) as usize;
        let page = mem.pages[pc as usize / PAGE_SIZE].as_ref()?;
        let slot = ((pc >> 1) as usize) & (SLOTS - 1);
        let valid = self.entries[slot]
            .as_ref()
            .is_some_and(|b| b.pc == pc && page.get(offset..offset + b.bytes.len()) == Some(b.bytes.as_slice()));
        if !valid {
            self.entries[slot] = Some(compile(pc, &page[offset..]));
        }
        let block = self.entries[slot].as_ref()?;
        if block.ops.is_empty() {
            return None;
        }
        let mut state = [0u32; 17];
        for (i, value) in state.iter_mut().enumerate() {
            *value = cpu.reg_get(Mode::User, i as u8);
        }
        let mut memory = mem.as_arm32cpu_memory();
        let mut steps = 0;
        for &op in &block.ops {
            let n = op.steps();
            let here = state[15];
            if count - steps < n || here == end || (n == 2 && here.wrapping_add(2) == end) {
                break;
            }
            // PC advances before data access in the interpreter, including on faults.
            state[15] = here.wrapping_add(2);
            steps += 1;
            match op {
                Op::Load(load) => load.run(&mut state, &mut memory),
                Op::LoadPair(first, second) => {
                    first.run(&mut state, &mut memory);
                    if memory.memory_error().is_none() {
                        state[15] = here.wrapping_add(4);
                        steps += 1;
                        second.run(&mut state, &mut memory);
                    }
                }
                Op::Mov(dst, src) => state[dst] = state[src],
                Op::Imm { kind, dst, value } => {
                    let result = match kind {
                        0 => {
                            nz(&mut state, value);
                            value
                        }
                        2 => {
                            let a = state[dst];
                            arithmetic(&mut state, a, value, false)
                        }
                        _ => {
                            let a = state[dst];
                            arithmetic(&mut state, a, value, true)
                        }
                    };
                    if kind != 1 {
                        state[dst] = result;
                    }
                }
                Op::Lsl { dst, src, shift } => {
                    let value = state[src];
                    let result = value << shift;
                    if shift != 0 {
                        state[16] = (state[16] & !(1 << 29)) | (((value >> (32 - shift)) & 1) << 29);
                    }
                    nz(&mut state, result);
                    state[dst] = result;
                }
                Op::Cmp(a, b) => {
                    let (a, b) = (state[a], state[b]);
                    arithmetic(&mut state, a, b, true);
                }
                Op::CmpBranch { left, right, equal, delta } => {
                    let (a, b) = (state[left], state[right]);
                    arithmetic(&mut state, a, b, true);
                    steps += 1;
                    state[15] = here.wrapping_add(4);
                    if (a == b) == equal {
                        state[15] = here.wrapping_add(6).wrapping_add_signed(delta);
                    }
                }
                Op::Branch { condition, delta } => {
                    if condition.is_none_or(|equal| (state[16] & (1 << 30) != 0) == equal) {
                        state[15] = here.wrapping_add(4).wrapping_add_signed(delta);
                    }
                }
                Op::StoreHalf { src, base, offset } => memory.w16(state[base].wrapping_add(offset) & !1, state[src] as u16),
            }
            if memory.memory_error().is_some() {
                break;
            }
        }
        if steps == 0 {
            return None;
        }
        for (i, value) in state.iter().enumerate() {
            cpu.reg_set(Mode::User, i as u8, *value);
        }
        Some((steps, memory.memory_error()))
    }
}
impl Load {
    fn run(self, s: &mut [u32; 17], m: &mut Arm32CpuMemory<'_>) {
        let address = s[self.base].wrapping_add(self.offset).wrapping_add(self.index.map_or(0, |i| s[i]));
        s[self.dst] = match self.width {
            1 => m.r8(address) as u32,
            2 => m.r16(address & !1) as u32,
            _ => m.r32(address & !3).rotate_right((address & 3) * 8),
        };
    }
}
fn nz(s: &mut [u32; 17], result: u32) {
    s[16] = (s[16] & 0x3fff_ffff) | (result & 0x8000_0000) | (u32::from(result == 0) << 30);
}
fn arithmetic(s: &mut [u32; 17], a: u32, b: u32, subtract: bool) -> u32 {
    let (result, carry, overflow) = if subtract {
        let result = a.wrapping_sub(b);
        (result, a >= b, ((a ^ b) & (a ^ result)) >> 31 != 0)
    } else {
        let (result, carry) = a.overflowing_add(b);
        (result, carry, (!(a ^ b) & (a ^ result)) >> 31 != 0)
    };
    s[16] = (s[16] & 0x0fff_ffff) | (u32::from(carry) << 29) | (u32::from(overflow) << 28);
    nz(s, result);
    result
}
fn decode(i: u16) -> Option<Op> {
    let dst = (i & 7) as usize;
    let base = ((i >> 3) & 7) as usize;
    let load = |dst, base, offset, index, width| {
        Some(Op::Load(Load {
            dst,
            base,
            offset,
            index,
            width,
        }))
    };
    if i & 0xf800 == 0x9800 {
        return load(((i >> 8) & 7) as usize, 13, ((i & 255) * 4) as u32, None, 4);
    }
    if i & 0xf800 == 0x6800 {
        return load(dst, base, (((i >> 6) & 31) * 4) as u32, None, 4);
    }
    if i & 0xfe00 == 0x5c00 {
        return load(dst, base, 0, Some(((i >> 6) & 7) as usize), 1);
    }
    if i & 0xfe00 == 0x5a00 {
        return load(dst, base, 0, Some(((i >> 6) & 7) as usize), 2);
    }
    if i & 0xff00 == 0x4600 {
        let d = dst + ((i >> 4) & 8) as usize;
        let r = ((i >> 3) & 15) as usize;
        if d < 15 && r < 15 {
            return Some(Op::Mov(d, r));
        }
    }
    if i & 0xe000 == 0x2000 {
        return Some(Op::Imm {
            kind: (i >> 11) & 3,
            dst: ((i >> 8) & 7) as usize,
            value: (i & 255) as u32,
        });
    }
    if i & 0xf800 == 0 {
        return Some(Op::Lsl {
            dst,
            src: base,
            shift: ((i >> 6) & 31) as u32,
        });
    }
    if i & 0xffc0 == 0x4280 {
        return Some(Op::Cmp(dst, base));
    }
    if i & 0xfe00 == 0xd000 {
        return Some(Op::Branch {
            condition: Some(i & 0x100 == 0),
            delta: (i as u8 as i8 as i32) * 2,
        });
    }
    if i & 0xf800 == 0xe000 {
        return Some(Op::Branch {
            condition: None,
            delta: (((i & 0x7ff) as i32) << 21) >> 20,
        });
    }
    if i & 0xf800 == 0x8000 {
        return Some(Op::StoreHalf {
            src: dst,
            base,
            offset: (((i >> 6) & 31) * 2) as u32,
        });
    }
    None
}
fn compile(pc: u32, bytes: &[u8]) -> Block {
    let mut ops = Vec::new();
    let mut used = 0;
    while used + 2 <= bytes.len() && used < MAX_HALFWORDS * 2 {
        let Some(mut op) = decode(u16::from_le_bytes([bytes[used], bytes[used + 1]])) else {
            break;
        };
        used += 2;
        if used + 2 <= bytes.len() && used < MAX_HALFWORDS * 2 {
            let next = decode(u16::from_le_bytes([bytes[used], bytes[used + 1]]));
            match (op, next) {
                (Op::Load(a), Some(Op::Load(b))) => {
                    op = Op::LoadPair(a, b);
                    used += 2;
                }
                (
                    Op::Cmp(left, right),
                    Some(Op::Branch {
                        condition: Some(equal),
                        delta,
                    }),
                ) => {
                    op = Op::CmpBranch { left, right, equal, delta };
                    used += 2;
                }
                _ => {}
            }
        }
        ops.push(op);
        if op.ends() {
            break;
        }
    }
    Block {
        pc,
        bytes: bytes[..used.max(2).min(bytes.len())].to_vec(),
        ops,
    }
}

#[cfg(test)]
#[path = "thumb_blocks_tests.rs"]
mod tests;
