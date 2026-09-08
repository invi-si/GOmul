use log::*;

use crate::util::arm::*;
use crate::util::bit::BitUtilExt;

use crate::exception::Exception;
use crate::reg::{self, cpsr, Reg};
use crate::{Cpu, Memory};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Instruction {
    Shifted,
    AddSub,
    ImmOp,
    AluOp,
    HiRegBx, // Mix of high register access and BX
    PcLoad,
    SingleXferR,
    HwSgnXfer,
    SingleXferI,
    HwXferI,
    SpXfer,
    LoadAddr,
    SpAdd,
    PushPop,
    BlockXfer,
    CondBranch,
    SoftwareInt,
    Branch,
    LongBranch,
    Nop,
    Undefined,
}

const INST_MATCH_ORDER: [Instruction; 21] = [
    Instruction::Branch,
    Instruction::AddSub,
    Instruction::AluOp,
    Instruction::Shifted,
    Instruction::ImmOp,
    Instruction::HiRegBx,
    Instruction::PcLoad,
    Instruction::SingleXferR,
    Instruction::HwSgnXfer,
    Instruction::SingleXferI,
    Instruction::HwXferI,
    Instruction::SpXfer,
    Instruction::LoadAddr,
    Instruction::SpAdd,
    Instruction::PushPop,
    Instruction::BlockXfer,
    Instruction::SoftwareInt,
    Instruction::CondBranch,
    Instruction::LongBranch,
    Instruction::Nop,
    Instruction::Undefined,
];

// Every existing Thumb class mask depends only on the high byte, except the
// exact NOP at 0xbf00. Derive this immutable table from the ordered reference
// patterns, preserving precedence; decode_table checks the NOP's low byte.
// This is keyed by fetched bits, never guest addresses, so modified code does
// not need cache invalidation. The feature-off decoder below is unchanged.
#[cfg(any(feature = "experimental-thumb-table", test))]
const THUMB_CLASSES: [Instruction; 256] = {
    let mut classes = [Instruction::Undefined; 256];
    let mut high_byte = 0;
    while high_byte < classes.len() {
        let inst = (high_byte as u16) << 8;
        let mut index = 0;
        while index < INST_MATCH_ORDER.len() {
            let class = INST_MATCH_ORDER[index];
            let (mask, test) = class.pattern();
            if ((inst ^ test) & mask) == 0 {
                classes[high_byte] = class;
                break;
            }
            index += 1;
        }
        high_byte += 1;
    }
    classes
};

impl Instruction {
    #[inline]
    #[rustfmt::skip]
    const fn pattern(self) -> (u16, u16) {
        use self::Instruction::*;
        match self {
            Shifted     => (0xe000, 0x0000),
            AddSub      => (0xf800, 0x1800),
            ImmOp       => (0xe000, 0x2000),
            AluOp       => (0xfc00, 0x4000),
            HiRegBx     => (0xfc00, 0x4400),
            PcLoad      => (0xf800, 0x4800),
            SingleXferR => (0xf200, 0x5000),
            HwSgnXfer   => (0xf200, 0x5200),
            SingleXferI => (0xe000, 0x6000),
            HwXferI     => (0xf000, 0x8000),
            SpXfer      => (0xf000, 0x9000),
            LoadAddr    => (0xf000, 0xa000),
            SpAdd       => (0xff00, 0xb000),
            PushPop     => (0xf600, 0xb400),
            BlockXfer   => (0xf000, 0xc000),
            CondBranch  => (0xf000, 0xd000),
            SoftwareInt => (0xff00, 0xdf00),
            Branch      => (0xf800, 0xe000),
            LongBranch  => (0xf000, 0xf000),
            Nop         => (0xffff, 0xbf00),
            Undefined   => (0x0000, 0x0000),
        }
    }

    #[cfg(not(feature = "experimental-thumb-table"))]
    fn decode(inst: u16) -> Instruction {
        for typ in INST_MATCH_ORDER.iter() {
            let (mask, test) = typ.pattern();
            if (inst as u32).mask_match(mask as u32, test as u32) {
                return *typ;
            }
        }
        Instruction::Undefined
    }

    #[cfg(feature = "experimental-thumb-table")]
    #[inline]
    fn decode(inst: u16) -> Instruction {
        Self::decode_table(inst)
    }

    #[cfg(any(feature = "experimental-thumb-table", test))]
    #[inline]
    fn decode_table(inst: u16) -> Instruction {
        let class = THUMB_CLASSES[(inst >> 8) as usize];
        if class == Instruction::Nop && inst != 0xbf00 {
            Instruction::Undefined
        } else {
            class
        }
    }
}

impl Cpu {
    /// Executes one instruction and returns whether the CPU should continue
    /// executing.
    #[cfg_attr(feature = "experimental-thumb-inline", inline(always))]
    pub(crate) fn execute_thumb(&mut self, mmu: &mut impl Memory) -> bool {
        #[cfg(feature = "profiling")]
        self.profiling.instruction_set(true);
        let pc = self.reg[reg::PC];
        #[cfg(feature = "exact-counts")]
        crate::exact_counts::fetch(true);
        let inst = profile_span!(self, Fetch, mmu.r16(pc)) as u32;
        let inst_type = profile_span!(self, Decode, self::Instruction::decode(inst as u16));
        #[cfg(feature = "exact-counts")]
        { crate::exact_counts::fetch(false); crate::exact_counts::instruction(pc, inst, true, inst_type as u32, inst_type); }
        #[cfg(feature = "profiling")]
        self.profiling.decoded(inst_type != Instruction::Undefined);
        let cpsr = profile_span!(self, FlagsCpsr, self.reg[reg::CPSR]);
        let c = cpsr.get_bit(cpsr::C);
        let v = cpsr.get_bit(cpsr::V);

        #[cfg(not(feature = "advanced_disasm"))]
        {
            trace!("THM: pc: {:#010x}, inst: {:#06x}", pc, inst);
            trace!("Instruction: {:?}", inst_type);
        }
        #[cfg(feature = "advanced_disasm")]
        {
            if log_enabled!(log::Level::Trace) {
                let cs = self.cs.as_mut().unwrap();
                cs.set_mode(capstone::Mode::Thumb).unwrap();
                if let Ok(inst) = cs.disasm_count(&inst.to_le_bytes(), pc as u64, 1) {
                    let s = format!("{}", inst);
                    trace!("{}", s.trim());
                } else {
                    trace!("failed to disasm instruction");
                }
            }
        }

        profile_reg_write!(
            self,
            reg::PC,
            self.reg[reg::PC] = (if cfg!(feature = "experimental-pc-local") {
                pc
            } else {
                self.reg[reg::PC]
            })
            .wrapping_add(2)
        );

        macro_rules! set_flags {
            ($res: expr , $new_v: expr , $new_c: expr) => {
                let new_z = ($res == 0) as u32;
                let new_n = u32::is_neg($res) as u32;
                let new_flags =
                    profile_span!(self, FlagsCpsr, build_flags($new_v, $new_c, new_z, new_n));
                profile_reg_write!(
                    self,
                    reg::CPSR,
                    self.reg[reg::CPSR] = profile_span!(self, FlagsCpsr, self.reg[reg::CPSR])
                        .set_bit(28, 4, new_flags)
                );
            };
        }

        #[cfg(feature = "profiling")]
        self.profiling.begin_execute();
        use self::Instruction::*;
        match inst_type {
            Shifted => {
                let op = inst.extract(11, 2);
                let shift = inst.extract(6, 5);
                let rs = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                let val = self.reg[rs];

                let (res, new_c) = if shift == 0 {
                    arg_shift0(val, op, c)
                } else {
                    arg_shift(val, shift, op)
                };

                profile_reg_write!(self, rd, self.reg[rd] = res);

                set_flags!(res, v, new_c);
            }
            AddSub => {
                let i = inst.get_bit(10);
                let op = inst.get_bit(9);

                let rs = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                let rn = inst.extract(6, 3) as Reg;
                let val2 = if i == 0 { self.reg[rn] } else { rn as u32 };

                let (res, new_v, new_c) = match op {
                    0 => self.reg[rs].add_flags(val2, 0),
                    1 => self.reg[rs].sub_flags(val2, 0),
                    _ => unreachable!(),
                };

                profile_reg_write!(self, rd, self.reg[rd] = res);
                set_flags!(res, new_v, new_c);
            }
            ImmOp => {
                let op = inst.extract(11, 2);
                let rd = inst.extract(8, 3) as Reg;
                let imm = inst.extract(0, 8);

                let (res, new_v, new_c) = match op {
                    0 /* MOV */ => (imm, v, c),
                    1 /* CMP */ |
                    3 /* SUB */ => self.reg[rd].sub_flags(imm, 0),
                    2 /* ADD */ => self.reg[rd].add_flags(imm, 0),
                    _ => unreachable!(),
                };

                if op != 1 {
                    profile_reg_write!(self, rd, self.reg[rd] = res);
                }

                set_flags!(res, new_v, new_c);
            }
            AluOp => {
                let op = inst.extract(6, 4);
                let rs = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                let vals = self.reg[rs];
                let vald = self.reg[rd];

                let (res, new_v, new_c) = match op {
                    0x0 /* AND */ |
                    0x8 /* TST */ => (vald & vals, v, c),
                    0x1 /* EOR */ => (vald ^ vals, v, c),
                    0x2 /* LSL */ |
                    0x3 /* LSR */ |
                    0x4 /* ASR */ |
                    0x7 /* ROR */ => {
                        let shift = vals & 0xff;
                        let (res, new_c) = if shift == 0 {
                            (vald, c)
                        } else {
                            let shift_type = ((op >> 1) & 2) | (op & 1);
                            arg_shift(vald, shift, shift_type)
                        };
                        (res, v, new_c)
                    },
                    0x5 /* ADC */ => vald.add_flags(vals, c),
                    0x6 /* SBC */ => vald.sub_flags(vals, 1-c),
                    0x9 /* NEG */ => 0.sub_flags(vals, 0),
                    0xA /* CMP */ => vald.sub_flags(vals, 0),
                    0xB /* CMN */ => vald.add_flags(vals, 0),
                    0xC /* ORR */ => (vald | vals, v, c),
                    0xD /* MUL */ => (vald.wrapping_mul(vals), v, 0),
                    0xE /* BIC */ => (vald & !vals, v, c),
                    0xF /* MVN */ => (!vals, v, c),
                    _ => unreachable!(),
                };

                match op {
                    0x8 | 0xA | 0xB => (),
                    _ => profile_reg_write!(self, rd, self.reg[rd] = res),
                };

                set_flags!(res, new_v, new_c);
            }
            HiRegBx => {
                let op = inst.extract(8, 2);
                let hd = inst.get_bit(7);
                let hs = inst.get_bit(6);
                let rs = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                let crs = ((hs * 8) as Reg) + rs;
                let crd = ((hd * 8) as Reg) + rd;

                let vals = self.reg[crs].wrapping_add(((crs == reg::PC) as u32) * 2);
                let vald = self.reg[crd].wrapping_add(((crd == reg::PC) as u32) * 2);

                match op {
                    0 /* ADD */ => profile_reg_write!(self, crd, self.reg[crd] = vald.wrapping_add(vals)),
                    1 /* CMP */ => {
                        let (res, new_v, new_c) = vald.sub_flags(vals, 0);
                        set_flags!(res, new_v, new_c);
                    },
                    2 /* MOV */ => profile_reg_write!(self, crd, self.reg[crd] = vals),
                    3 /* BX or BLX */ => {
                        let new_t = vals.get_bit( 0);
                        let mask: u32 = if new_t == 0 { !3 } else { !1 };

                        #[cfg(feature = "exact-counts")]
                        crate::exact_counts::branch(false, true);
                        profile_reg_write!(self, reg::PC, self.reg[reg::PC] = vals & mask);

                        let cpsr_mask = 1 << cpsr::T;
                        profile_reg_write!(self, reg::CPSR, self.reg[reg::CPSR] = (cpsr & !cpsr_mask) | (new_t << cpsr::T));
                        if hd == 1 {
                            profile_reg_write!(self, reg::LR, self.reg[reg::LR] = pc.wrapping_add(2) | 1);
                        }
                    },
                    _ => unreachable!(),
                };
                if crd == reg::PC {
                    profile_reg_write!(self, reg::PC, self.reg[reg::PC] &= !1);
                }
            }
            PcLoad => {
                let rd = inst.extract(8, 3) as Reg;
                let offset = inst.extract(0, 8);

                let addr = self.reg[reg::PC].wrapping_add(2).wrapping_add(offset * 4) & !3;

                profile_reg_write!(
                    self,
                    rd,
                    self.reg[rd] = profile_span!(self, DataRead, mmu.r32(addr))
                );
            }
            SingleXferR => {
                let l = inst.get_bit(11);
                let b = inst.get_bit(10);

                let ro = inst.extract(6, 3) as Reg;
                let rb = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                let offset = self.reg[ro];
                let addr = self.reg[rb].wrapping_add(offset);
                match (l, b) {
                    (0, 0) => {
                        let v = self.reg[rd];
                        profile_span!(self, DataWrite, mmu.w32(addr, v))
                    }
                    (0, 1) => {
                        let v = self.reg[rd];
                        profile_span!(self, DataWrite, mmu.w8(addr, v as u8))
                    }
                    (1, 0) => profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = profile_span!(self, DataRead, mmu.r32(addr))
                    ),
                    (1, 1) => profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = profile_span!(self, DataRead, mmu.r8(addr)) as u32
                    ),
                    _ => unreachable!(),
                };
            }
            HwSgnXfer => {
                let h = inst.get_bit(11);
                let s = inst.get_bit(10);

                let ro = inst.extract(6, 3) as Reg;
                let rb = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                let offset = self.reg[ro];
                let addr = self.reg[rb].wrapping_add(offset);

                match (s, h) {
                    (0, 0) => {
                        profile_span!(self, DataWrite, mmu.w16(addr & !1, self.reg[rd] as u16))
                    }
                    (0, 1) => profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = profile_span!(self, DataRead, mmu.r16(addr & !1)) as u32
                    ),
                    (1, 0) => profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = profile_span!(self, DataRead, mmu.r8(addr)) as i8 as u32
                    ),
                    (1, 1) => profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] =
                            profile_span!(self, DataRead, mmu.r16(addr & !1)) as i16 as u32
                    ),
                    _ => unreachable!(),
                }
            }
            SingleXferI => {
                let l = inst.get_bit(11);
                let b = inst.get_bit(12);

                let offset = inst.extract(6, 5);
                let rb = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                if b == 0 {
                    let addr = self.reg[rb].wrapping_add(offset * 4);
                    if l == 0 {
                        let val = self.reg[rd];
                        profile_span!(self, DataWrite, mmu.w32(addr, val));
                    } else {
                        profile_reg_write!(
                            self,
                            rd,
                            self.reg[rd] = profile_span!(self, DataRead, mmu.r32(addr))
                        );
                    }
                } else {
                    let addr = self.reg[rb].wrapping_add(offset);
                    if l == 0 {
                        profile_span!(self, DataWrite, mmu.w8(addr, self.reg[rd] as u8));
                    } else {
                        profile_reg_write!(
                            self,
                            rd,
                            self.reg[rd] = profile_span!(self, DataRead, mmu.r8(addr)) as u32
                        );
                    }
                }
            }
            HwXferI => {
                let l = inst.get_bit(11);

                let offset = inst.extract(6, 5);
                let rb = inst.extract(3, 3) as Reg;
                let rd = inst.extract(0, 3) as Reg;

                let addr = self.reg[rb].wrapping_add(offset * 2) & !1;
                if l == 0 {
                    profile_span!(self, DataWrite, mmu.w16(addr, self.reg[rd] as u16));
                } else {
                    profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = profile_span!(self, DataRead, mmu.r16(addr)) as u32
                    );
                }
            }
            SpXfer => {
                let l = inst.get_bit(11);

                let rd = inst.extract(8, 3) as Reg;
                let offset = inst.extract(0, 8) * 4;

                let addr = self.reg[reg::SP].wrapping_add(offset);

                if l == 0 {
                    let val = self.reg[rd];
                    profile_span!(self, DataWrite, mmu.w32(addr, val));
                } else {
                    profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = profile_span!(self, DataRead, mmu.r32(addr))
                    );
                }
            }
            LoadAddr => {
                let s = inst.get_bit(11);
                let rd = inst.extract(8, 3) as Reg;
                let imm = inst.extract(0, 8);

                let base = if s == 0 {
                    self.reg[reg::PC].wrapping_add(2) & !2
                } else {
                    self.reg[reg::SP]
                };

                profile_reg_write!(self, rd, self.reg[rd] = base.wrapping_add(imm * 4));
            }
            SpAdd => {
                let s = inst.get_bit(7);
                let imm = inst.extract(0, 7) * 4;

                let sp = self.reg[reg::SP];

                profile_reg_write!(
                    self,
                    reg::SP,
                    self.reg[reg::SP] = if s == 0 {
                        sp.wrapping_add(imm)
                    } else {
                        sp.wrapping_sub(imm)
                    }
                );
            }
            PushPop => {
                let l = inst.get_bit(11);
                let r = inst.get_bit(8);

                let rlist = inst.extract(0, 8);

                let total = rlist.count_ones() + r;

                let base = self.reg[reg::SP];
                let post_addr = if l == 0 {
                    base.wrapping_sub(total * 4)
                } else {
                    base.wrapping_add(total * 4)
                };

                let addr = if l == 0 { post_addr } else { base };

                let mut rem = rlist
                    | if r == 1 {
                        1 << (if l == 0 { reg::LR } else { reg::PC })
                    } else {
                        0
                    };

                for i in 0..total {
                    let reg = rem.trailing_zeros() as Reg;
                    let idx_addr = addr.wrapping_add(i * 4);
                    if l == 0 {
                        let val = self.reg[reg];
                        profile_span!(self, DataWrite, mmu.w32(idx_addr, val));
                    } else {
                        let val = profile_span!(self, DataRead, mmu.r32(idx_addr));
                        profile_reg_write!(self, reg, self.reg[reg] = val);

                        if reg == reg::PC {
                            // change mode
                            let new_t = val.get_bit(0);
                            let mask = 1 << cpsr::T;
                            profile_reg_write!(
                                self,
                                reg::CPSR,
                                self.reg[reg::CPSR] = (cpsr & !mask) | (new_t << cpsr::T)
                            );
                            profile_reg_write!(self, reg::PC, self.reg[reg::PC] = val & !1);
                        }
                    }

                    rem -= 1 << reg;
                }

                profile_reg_write!(self, reg::SP, self.reg[reg::SP] = post_addr);
            }
            BlockXfer => {
                let l = inst.get_bit(11);
                let rb = inst.extract(8, 3) as Reg;

                let rlist = inst.extract(0, 8);

                let total = rlist.count_ones();

                let base = self.reg[rb];
                // FIXME: if rlist is empty weird stuff happens
                profile_reg_write!(self, rb, self.reg[rb] = base.wrapping_add(total * 4));

                let mut rem = rlist;
                for i in 0..total {
                    let reg = rem.trailing_zeros() as Reg;
                    let idx_addr = base.wrapping_add(i * 4);

                    if l == 0 {
                        let val = if i == 0 && reg == rb {
                            base
                        } else {
                            self.reg[reg]
                        };
                        profile_span!(self, DataWrite, mmu.w32(idx_addr, val));
                    } else {
                        profile_reg_write!(
                            self,
                            reg,
                            self.reg[reg] = profile_span!(self, DataRead, mmu.r32(idx_addr))
                        );
                    }

                    rem -= 1 << reg;
                }
            }
            CondBranch => {
                let cond = inst.extract(8, 4);
                let offset = inst.extract(0, 8) as i8 as u32;

                let taken = profile_span!(self, Condition, cond_met(cond, cpsr));
                #[cfg(feature = "exact-counts")]
                crate::exact_counts::branch(true, taken);
                if taken {
                    profile_reg_write!(
                        self,
                        reg::PC,
                        self.reg[reg::PC] = pc.wrapping_add(4).wrapping_add(offset << 1)
                    );
                }
            }
            SoftwareInt => {
                profile_span!(self, Exception, self.exception(Exception::Software));
            }
            Branch => {
                #[cfg(feature = "exact-counts")]
                crate::exact_counts::branch(false, true);
                let offset = (inst.extract(0, 11) << 1).sign_extend(12);

                profile_reg_write!(
                    self,
                    reg::PC,
                    self.reg[reg::PC] = pc.wrapping_add(4).wrapping_add(offset)
                );
            }
            LongBranch => {
                #[cfg(feature = "exact-counts")]
                crate::exact_counts::fetch(true);
                let inst2 = profile_span!(self, Fetch, mmu.r16(pc.wrapping_add(2))) as u32;
                #[cfg(feature = "exact-counts")]
                crate::exact_counts::fetch(false);
                let offset_hi = inst.extract(0, 11);
                let offset_lo = inst2.extract(0, 11);
                let lr = pc
                    .wrapping_add(4)
                    .wrapping_add((offset_hi << 12).sign_extend(23));

                #[cfg(feature = "exact-counts")]
                crate::exact_counts::branch(false, inst2.mask_match(0xf800, 0xf800) || inst2.mask_match(0xf801, 0xe800));
                if inst2.mask_match(0xf800, 0xf800) {
                    profile_reg_write!(
                        self,
                        reg::PC,
                        self.reg[reg::PC] = lr.wrapping_add(offset_lo << 1)
                    );
                    profile_reg_write!(self, reg::LR, self.reg[reg::LR] = pc.wrapping_add(4) | 1);
                } else if inst2.mask_match(0xf801, 0xe800) {
                    profile_reg_write!(
                        self,
                        reg::PC,
                        self.reg[reg::PC] = lr.wrapping_add(offset_lo << 1) & !3
                    );
                    profile_reg_write!(self, reg::LR, self.reg[reg::LR] = pc.wrapping_add(4) | 1);

                    let mask = 1 << cpsr::T;
                    profile_reg_write!(self, reg::CPSR, self.reg[reg::CPSR] &= !mask);
                } else {
                    return false;
                }
            }
            Nop => {}
            Undefined => return false,
        };

        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ExampleMem, Mode};

    #[test]
    fn table_matches_ordered_decoder_for_every_thumb_encoding() {
        // Keep the original ordered matcher as the oracle, including mask
        // precedence and undefined/NOP encodings. Test all fetched bit patterns.
        for inst in 0..=u16::MAX {
            let reference = INST_MATCH_ORDER
                .iter()
                .copied()
                .find(|class| {
                    let (mask, test) = class.pattern();
                    (inst as u32).mask_match(mask as u32, test as u32)
                })
                .unwrap_or(Instruction::Undefined);
            assert_eq!(Instruction::decode_table(inst), reference, "{inst:#06x}");
            assert_eq!(Instruction::decode(inst), reference, "{inst:#06x}");
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_decode() {
        use super::Instruction::*;

        macro_rules! check (
            ($inst: expr, $val: expr) => {
                assert_eq!($inst, Instruction::decode($val));
            }
        );
        check!(Shifted,     0x0fb4);
        check!(AddSub,      0x1c0a);
        check!(ImmOp,       0x200a);
        check!(AluOp,       0x4042);
        check!(HiRegBx,     0x466c);
        check!(PcLoad,      0x4d00);
        check!(SingleXferR, 0x5045);
        check!(HwSgnXfer,   0x5fb9);
        check!(SingleXferI, 0x7078);
        check!(HwXferI,     0x80b9);
        check!(SpXfer,      0x9102);
        check!(LoadAddr,    0xa001);
        check!(SpAdd,       0xb082);
        check!(PushPop,     0xb407);
        check!(BlockXfer,   0xc103);
        check!(CondBranch,  0xd1fb);
        check!(Branch,      0xe002);
        check!(LongBranch,  0xf801);
        check!(Nop,          0xbf00);
        check!(Undefined,   0xe800);
    }

    #[test]
    fn nop_hint() {
        let mut mmu = ExampleMem::new_with_data(&[0x00, 0xbf]);
        let mut cpu = Cpu::new();

        cpu.reg_set(Mode::User, reg::PC, 0x00);
        cpu.reg_set(
            Mode::User,
            reg::CPSR,
            (0xd3 & !(1 << cpsr::T)) | (1 << cpsr::T),
        );

        assert!(cpu.step(&mut mmu));
        assert_eq!(cpu.reg_get(Mode::User, reg::PC), 0x02);
    }

    macro_rules! emutest {
        ($name:ident, $mem_checks: expr) => {
            #[test]
            fn $name() {
                let prog = include_bytes!(concat!("../tests/data/", stringify!($name), ".bin"));
                let mut mmu = ExampleMem::new_with_data(prog);
                let mut cpu = Cpu::new();

                // Start at 0, with a stack pointer, and in thumb mode
                cpu.reg_set(Mode::User, reg::PC, 0x00);
                cpu.reg_set(Mode::User, reg::SP, 0x200);
                cpu.reg_set(Mode::User, reg::CPSR, 0x10);

                // set thumb mode
                let mask = 1u32 << cpsr::T;
                let cpsr = cpu.reg[reg::CPSR];
                cpu.reg[reg::CPSR] = (cpsr & !mask) | mask;

                while cpu.step(&mut mmu) {}

                for &(addr, val) in ($mem_checks).iter() {
                    assert_eq!(val, mmu.r32(addr), "addr: {:#010x}", addr);
                }
            }
        };
    }

    emutest!(
        emutest_thm0,
        [
            (0x1ec, 10),
            (0x1f0, 15),
            (0x1f4, 5),
            (0x1f8, 60),
            (0x1fc, 0x200),
        ]
    );
    emutest!(emutest_thm1, [(0x200, 0xdead_beef)]);
    emutest!(
        emutest_thm2,
        [(0x200, 0xff00), (0x204, 0xff80), (0x208, 0x7fff_ff80)]
    );
    emutest!(emutest_thm3, [(0x1f8, 8), (0x1fc, 0x200), (0x200, 64)]);
    emutest!(emutest_thm4, [(0x200, 4), (0x204, 5)]);
    emutest!(emutest_thm5, [(0x200, 10), (0x204, 83)]);
    emutest!(emutest_thm6, [(0x1fc, 0)]);
    emutest!(emutest_thm7, [(0x1fc, 0xff)]);
    emutest!(emutest_thm8, [(0x1fc, 0x0123_4567)]);
    emutest!(emutest_thm9, [(0x200, 11), (0x204, 22)]);
    emutest!(emutest_thm10, [(0x1fc, 0x55)]);
    emutest!(emutest_thm11, [(0x200, 11), (0x204, 22)]);
}
