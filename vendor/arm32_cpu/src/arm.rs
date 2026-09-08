use log::*;

use crate::util::arm::*;
use crate::util::bit::{combine64, split64, BitUtilExt};

use crate::exception::Exception;
use crate::mode::Mode;
use crate::reg::{self, cpsr, Reg};
use crate::{Cpu, Memory};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Instruction {
    BranchEx, // Branch and exchange (i.e. switch to THUMB)
    Branch,
    Clz,
    DataProc0,
    DataProc1,
    DataProc2,
    PsrImm,
    PsrReg,
    Multiply,
    MulLong,
    SingleXferI, // Single data transfer, immediate offset
    SingleXferR, // Single data transfer, register offset
    HwSgnXferR,  // Halfword and signed, register offset
    HwSgnXferI,  // Halfword and signed, immediate offset
    BlockXfer,
    Swap,
    SoftwareInt,
    CoprocReg,
    Undefined,
}

const INST_MATCH_ORDER: [Instruction; 19] = [
    Instruction::Branch,
    Instruction::BranchEx,
    Instruction::Clz,
    Instruction::Swap,
    Instruction::PsrImm,
    Instruction::PsrReg,
    Instruction::DataProc0,
    Instruction::DataProc1,
    Instruction::DataProc2,
    Instruction::Multiply,
    Instruction::MulLong,
    Instruction::SingleXferI,
    Instruction::SingleXferR,
    Instruction::HwSgnXferR,
    Instruction::HwSgnXferI,
    Instruction::BlockXfer,
    Instruction::SoftwareInt,
    Instruction::CoprocReg,
    Instruction::Undefined,
];

impl Instruction {
    #[inline]
    #[rustfmt::skip]
    fn pattern(self) -> (u32, u32) {
        use self::Instruction::*;
        match self {
            BranchEx    => (0x0fff_fff0, 0x012f_ff10),
            Branch      => (0x0e00_0000, 0x0a00_0000),
            Clz         => (0x0fff_0ff0, 0x016f_0f10),
            DataProc0   => (0x0e00_0010, 0x0000_0000),
            DataProc1   => (0x0e00_0090, 0x0000_0010),
            DataProc2   => (0x0e00_0000, 0x0200_0000),
            PsrImm      => (0x0fb0_0000, 0x0320_0000),
            PsrReg      => (0x0f90_0ff0, 0x0100_0000),
            Multiply    => (0x0fc0_00f0, 0x0000_0090),
            MulLong     => (0x0f80_00f0, 0x0080_0090),
            SingleXferI => (0x0e00_0000, 0x0400_0000),
            SingleXferR => (0x0e00_0010, 0x0600_0000),
            HwSgnXferR  => (0x0e40_0f90, 0x0000_0090),
            HwSgnXferI  => (0x0e40_0090, 0x0040_0090),
            BlockXfer   => (0x0e00_0000, 0x0800_0000),
            Swap        => (0x0fb0_0ff0, 0x0100_0090),
            SoftwareInt => (0x0f00_0000, 0x0f00_0000),
            CoprocReg   => (0x0f00_0010, 0x0e00_0010),
            Undefined   => (0x0000_0000, 0x0000_0000),
        }
    }

    fn decode(inst: u32) -> Instruction {
        for typ in INST_MATCH_ORDER.iter() {
            let (mask, test) = typ.pattern();
            if inst.mask_match(mask, test) {
                return *typ;
            }
        }
        Instruction::Undefined
    }
}

impl Cpu {
    /// Executes one instruction and returns whether the CPU should continue
    /// executing.
    pub(crate) fn execute_arm(&mut self, mmu: &mut impl Memory) -> bool {
        #[cfg(feature = "profiling")]
        self.profiling.instruction_set(false);
        let pc = self.reg[reg::PC];
        #[cfg(feature = "exact-counts")]
        crate::exact_counts::fetch(true);
        let inst = profile_span!(self, Fetch, mmu.r32(pc));
        let inst_type = profile_span!(self, Decode, self::Instruction::decode(inst));
        #[cfg(feature = "exact-counts")]
        { crate::exact_counts::fetch(false); crate::exact_counts::instruction(pc, inst, false, inst_type as u32, inst_type); }

        let cond = inst.extract(28, 4);
        #[cfg(feature = "profiling")]
        self.profiling.decoded(inst_type != Instruction::Undefined);
        let cpsr = profile_span!(self, FlagsCpsr, self.reg[reg::CPSR]);

        #[cfg(not(feature = "advanced_disasm"))]
        {
            let cflags = cpsr.extract(28, 4);
            trace!(
                "ARM: pc: {:#010x}, inst: {:#010x}, cond: {:#03x}, cflags: {:04b}",
                pc,
                inst,
                cond,
                cflags
            );
            let inst_type = Instruction::decode(inst);
            trace!("Instruction: {:?}", inst_type);
        }
        #[cfg(feature = "advanced_disasm")]
        {
            if log_enabled!(log::Level::Trace) {
                let cs = self.cs.as_mut().unwrap();
                cs.set_mode(capstone::Mode::Arm).unwrap();
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
            .wrapping_add(4)
        );

        let taken = profile_span!(self, Condition, cond_met(cond, cpsr));
        #[cfg(feature = "exact-counts")]
        if matches!(inst_type, Instruction::Branch | Instruction::BranchEx) { crate::exact_counts::branch(cond != 14, taken); }
        if !taken {
            #[cfg(feature = "profiling")]
            self.profiling.condition_failed();
            trace!("cond not met");
            return true;
        }

        #[cfg(feature = "profiling")]
        self.profiling.begin_execute();
        use self::Instruction::*;
        match inst_type {
            BranchEx => {
                let rn = inst.extract(0, 4) as Reg;
                let new_pc = self.reg[rn];
                profile_reg_write!(self, reg::PC, self.reg[reg::PC] = self.reg[rn] & !1u32);
                // maybe switch to thumb mode
                profile_reg_write!(
                    self,
                    reg::CPSR,
                    self.reg[reg::CPSR] |= new_pc.get_bit(0) << cpsr::T
                );
            }
            Branch => {
                let offset = inst.extract(0, 24);
                // Shift up to sign extend
                // shift right by 6 (instead of 8) to multiply by 4
                let s_offset = ((offset << 8) as i32 >> 6) as u32;

                if cond == 0b1111 {
                    // blx
                    let h = inst.get_bit(24);

                    let s_offset = s_offset | (h << 1);

                    profile_reg_write!(
                        self,
                        reg::PC,
                        self.reg[reg::PC] = pc.wrapping_add(s_offset).wrapping_add(8)
                    );
                    profile_reg_write!(self, reg::LR, self.reg[reg::LR] = pc.wrapping_add(4));
                    profile_reg_write!(self, reg::CPSR, self.reg[reg::CPSR] |= 1 << cpsr::T);
                } else {
                    let l = inst.get_bit(24);

                    profile_reg_write!(
                        self,
                        reg::PC,
                        self.reg[reg::PC] = pc.wrapping_add(s_offset).wrapping_add(8)
                    );
                    if l != 0 {
                        profile_reg_write!(self, reg::LR, self.reg[reg::LR] = pc.wrapping_add(4));
                    }
                }
            }
            Clz => {
                let rm = inst.extract(0, 4) as Reg;
                let rd = inst.extract(12, 4) as Reg;

                if rm == reg::PC || rd == reg::PC {
                    return false;
                }

                profile_reg_write!(self, rd, self.reg[rd] = self.reg[rm].leading_zeros());
            }
            DataProc0 | DataProc1 | DataProc2 => {
                let i = inst.get_bit(25);
                let s = inst.get_bit(20);
                let r = inst.get_bit(4);

                let c = cpsr.get_bit(cpsr::C);
                let v = cpsr.get_bit(cpsr::V);

                let opcode = inst.extract(21, 4);

                let rn = inst.extract(16, 4) as Reg;
                let rd = inst.extract(12, 4) as Reg;

                // Three modes:
                let (valm, shift_carry) = if inst_type == DataProc0 || inst_type == DataProc1 {
                    debug_assert!((r == 1) == (inst_type == DataProc1));
                    let rm = inst.extract(0, 4) as Reg;
                    let shift_type = inst.extract(5, 2);

                    let shift = if inst_type == DataProc0 {
                        inst.extract(7, 5)
                    } else {
                        let rs = inst.extract(8, 4) as Reg;
                        debug_assert!(rs <= 14u8);

                        self.reg[rs] & 0xffu32
                    };

                    let valm = self.reg[rm].wrapping_add(((rm == reg::PC) as u32) * (4 + 4 * r));

                    if r == 0 && shift == 0 {
                        arg_shift0(valm, shift_type, c)
                    } else if shift != 0 {
                        arg_shift(valm, shift, shift_type)
                    } else {
                        // [Rs] == 0, so we do nothing
                        (valm, c)
                    }
                } else {
                    let shift = inst.extract(8, 4) * 2;
                    let imm = inst.extract(0, 8);
                    imm.shift_ror(shift)
                };

                let valn = self.reg[rn].wrapping_add(if rn == reg::PC {
                    4 + 4 * (i == 0 && r == 1) as u32
                } else {
                    0
                });

                // execute the instruction now
                let (res, new_v, new_c) = match opcode {
                    0x0 /* AND */ |
                    0x8 /* TST */ => (valn & valm, v, shift_carry),
                    0x1 /* EOR */ |
                    0x9 /* TEQ */ => (valn ^ valm, v, shift_carry),
                    0x2 /* SUB */ |
                    0xA /* CMP */ => valn.sub_flags(valm, 0),
                    0x3 /* RSB */ => valm.sub_flags(valn, 0),
                    0x4 /* ADD */ |
                    0xB /* CMN */ => valn.add_flags(valm, 0),
                    0x5 /* ADC */ => valn.add_flags(valm, c),
                    0x6 /* SBC */ => valn.sub_flags(valm, 1-c),
                    0x7 /* RSC */ => valm.sub_flags(valn, 1-c),
                    0xC /* ORR */ => (valn | valm, v, shift_carry),
                    0xD /* MOV */ => (valm, v, shift_carry),
                    0xE /* BIC */ => (valn & !valm, v, shift_carry),
                    0xF /* MVN */ => (!valm, v, shift_carry),
                    _ => unreachable!(),
                };

                if s == 1 {
                    if rd != reg::PC {
                        let new_z = (res == 0) as u32;
                        let new_n = res.is_neg() as u32;
                        let new_flags =
                            profile_span!(self, FlagsCpsr, build_flags(new_v, new_c, new_z, new_n));
                        profile_reg_write!(
                            self,
                            reg::CPSR,
                            self.reg[reg::CPSR] =
                                profile_span!(self, FlagsCpsr, self.reg[reg::CPSR])
                                    .set_bit(28, 4, new_flags)
                        );
                    } else {
                        profile_reg_write!(
                            self,
                            reg::CPSR,
                            self.reg[reg::CPSR] =
                                profile_span!(self, FlagsCpsr, self.reg[reg::SPSR])
                        );
                        self.reg.update_bank();
                    }
                }

                match opcode {
                    0x8 | 0x9 | 0xA | 0xB => (), // no writeback
                    _ => profile_reg_write!(self, rd, self.reg[rd] = res),
                }
            }
            PsrImm | PsrReg => {
                let p = inst.get_bit(22);
                let rs = if p == 0 { reg::CPSR } else { reg::SPSR };

                let op = inst.get_bit(21);

                if op == 0 {
                    // Move psr to rd
                    let rd = inst.extract(12, 4) as Reg;
                    profile_reg_write!(self, rd, self.reg[rd] = self.reg[rs]);
                } else {
                    let i = inst.get_bit(25);
                    let f = inst.get_bit(19);
                    let c = inst.get_bit(16);

                    // user mode can't change the control bits
                    let ctrl = ((self.reg.mode() != Mode::User) as u32) * c;

                    let mask = 0xf000_0000 * f + 0x0000_00ff * ctrl;

                    let val = if i == 0 {
                        let rm = inst.extract(0, 4) as Reg;
                        self.reg[rm]
                    } else {
                        let rot = inst.extract(8, 4) * 2;
                        let imm = inst.extract(0, 8);
                        imm.rotate_right(rot)
                    };

                    let cur = self.reg[rs];
                    profile_reg_write!(self, rs, self.reg[rs] = (cur & !mask) | val & mask);
                    if rs == reg::CPSR {
                        self.reg.update_bank();
                    }
                };
            }
            Multiply => {
                let a = inst.get_bit(21);
                let s = inst.get_bit(20);

                let rd = inst.extract(16, 4) as Reg;
                let rn = inst.extract(12, 4) as Reg;
                let rs = inst.extract(8, 4) as Reg;
                let rm = inst.extract(0, 4) as Reg;

                let res = self.reg[rm]
                    .wrapping_mul(self.reg[rs])
                    .wrapping_add(if a == 0 { 0 } else { self.reg[rn] });

                profile_reg_write!(self, rd, self.reg[rd] = res);

                if s == 1 {
                    let v = cpsr.get_bit(cpsr::V);
                    let new_z = (res == 0) as u32;
                    let new_n = res.get_bit(31);
                    let new_flags = profile_span!(self, FlagsCpsr, build_flags(v, 0, new_z, new_n));

                    profile_reg_write!(
                        self,
                        reg::CPSR,
                        self.reg[reg::CPSR] = profile_span!(self, FlagsCpsr, self.reg[reg::CPSR])
                            .set_bit(28, 4, new_flags)
                    );
                }
            }
            MulLong => {
                let u = inst.get_bit(22);
                let a = inst.get_bit(21);
                let s = inst.get_bit(20);

                let rdhi = inst.extract(16, 4) as Reg;
                let rdlo = inst.extract(12, 4) as Reg;
                let rs = inst.extract(8, 4) as Reg;
                let rm = inst.extract(0, 4) as Reg;

                let res: u64 = if u == 0 {
                    let vs = self.reg[rs];
                    let vm = self.reg[rm];

                    let prod = (vs as u64) * (vm as u64);
                    prod.wrapping_add(if a == 0 {
                        0u64
                    } else {
                        combine64(self.reg[rdhi], self.reg[rdlo])
                    })
                } else {
                    let vs = self.reg[rs] as i32;
                    let vm = self.reg[rm] as i32;

                    let prod = (vs as i64) * (vm as i64);
                    prod.wrapping_add(if a == 0 {
                        0i64
                    } else {
                        combine64(self.reg[rdhi], self.reg[rdlo]) as i64
                    }) as u64
                };

                let (reshi, reslo) = split64(res);
                profile_reg_write!(self, rdhi, self.reg[rdhi] = reshi);
                profile_reg_write!(self, rdlo, self.reg[rdlo] = reslo);

                if s != 0 {
                    let new_z = (res == 0) as u32;
                    let new_n = reshi.get_bit(31);
                    let new_flags = profile_span!(self, FlagsCpsr, build_flags(0, 0, new_z, new_n));

                    profile_reg_write!(
                        self,
                        reg::CPSR,
                        self.reg[reg::CPSR] = profile_span!(self, FlagsCpsr, self.reg[reg::CPSR])
                            .set_bit(28, 4, new_flags)
                    );
                }
            }
            SingleXferI | SingleXferR => {
                let p = inst.get_bit(24);
                let u = inst.get_bit(23);
                let b = inst.get_bit(22);
                let w = inst.get_bit(21);
                let l = inst.get_bit(20);

                let rn = inst.extract(16, 4) as Reg;
                let rd = inst.extract(12, 4) as Reg;

                let offset = if inst_type == SingleXferI {
                    inst.extract(0, 12)
                } else {
                    // We use the same logic here as for DataProc0
                    let shift = inst.extract(7, 5);
                    let shift_type = inst.extract(5, 2);

                    let rm = inst.extract(0, 4) as Reg;

                    let valm = self.reg[rm];

                    let (shifted, _) = if shift == 0 {
                        let c = cpsr.get_bit(cpsr::C);
                        arg_shift0(valm, shift_type, c)
                    } else {
                        arg_shift(valm, shift, shift_type)
                    };
                    shifted
                };

                let base = self.reg[rn].wrapping_add(((rn == reg::PC) as u32) * 4);
                let post_addr = if u == 0 {
                    base.wrapping_sub(offset)
                } else {
                    base.wrapping_add(offset)
                };

                let addr = if p == 0 { base } else { post_addr };

                if l == 0 {
                    // store
                    let val = self.reg[rd].wrapping_add(((rd == reg::PC) as u32) * 8);
                    if b == 0 {
                        // force alignment of the store
                        profile_span!(self, DataWrite, mmu.w32(addr, val));
                    } else {
                        profile_span!(self, DataWrite, mmu.w8(addr, val as u8));
                    };
                } else {
                    profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = if b == 0 {
                            profile_span!(self, DataRead, mmu.r32(addr))
                        } else {
                            profile_span!(self, DataRead, mmu.r8(addr)) as u32
                        }
                    );
                };

                // post-indexing implies writeback
                // make sure we don't overwrite rd if it was a load
                if (p == 0 || w == 1) && (rd != rn || l == 0) {
                    profile_reg_write!(self, rn, self.reg[rn] = post_addr);
                }
            }
            HwSgnXferR | HwSgnXferI => {
                let p = inst.get_bit(24);
                let u = inst.get_bit(23);
                let w = inst.get_bit(21);
                let l = inst.get_bit(20);

                let s = inst.get_bit(6);
                let h = inst.get_bit(5);

                let rn = inst.extract(16, 4) as Reg;
                let rd = inst.extract(12, 4) as Reg;

                let offset = if inst_type == HwSgnXferR {
                    let rn = inst.extract(0, 4) as Reg;
                    self.reg[rn]
                } else {
                    (inst.extract(8, 4) << 4) | inst.extract(0, 4)
                };

                let base = self.reg[rn].wrapping_add(((rn == reg::PC) as u32) * 4);
                let post_addr = if u == 0 {
                    base.wrapping_sub(offset)
                } else {
                    base.wrapping_add(offset)
                };

                let addr = if p == 0 { base } else { post_addr };

                if l == 0 {
                    // store
                    debug_assert!(s == 0 && h == 1);
                    let val = self.reg[rd].wrapping_add(((rd == reg::PC) as u32) * 8);
                    profile_span!(self, DataWrite, mmu.w16(addr & !1, val as u16));
                } else {
                    profile_reg_write!(
                        self,
                        rd,
                        self.reg[rd] = match (s, h) {
                        (0, 0) /* SWP */ => unreachable!(),
                        (0, 1) /* halfword load */  => profile_span!(self, DataRead, mmu.r16(addr & !1)) as u32,
                        (1, 0) /* signed byte */    => profile_span!(self, DataRead, mmu.r8(addr)) as i8 as u32,
                        (1, 1) /* signed half */    => profile_span!(self, DataRead, mmu.r16(addr & !1)) as i16 as u32,
                        _ => unreachable!()
                    }
                    );
                };

                // post-indexing implies writeback
                // make sure we don't overwrite rd if it was a load
                if (p == 0 || w == 1) && (rd != rn || l == 0) {
                    profile_reg_write!(self, rn, self.reg[rn] = post_addr);
                }
            }
            BlockXfer => {
                let p = inst.get_bit(24);
                let u = inst.get_bit(23);
                let s = inst.get_bit(22);
                let w = inst.get_bit(21);
                let l = inst.get_bit(20);

                let rn = inst.extract(16, 4) as Reg;

                let reglist = inst.extract(0, 16);

                // FIXME: implement S bit correctly
                // FIXME: if reglist is empty apparently theres weird behaviour
                //        ignore this for now
                let total = reglist.count_ones();

                let orig_base = self.reg[rn];
                let base = orig_base;

                let post_addr = if u == 0 {
                    base.wrapping_sub(total * 4)
                } else {
                    base.wrapping_add(total * 4)
                };

                let addr = if u == 0 { post_addr } else { base };

                // If we are going up, and pre-incrementing,
                // or going down, and post-decrementing,
                // then we will be using the range [addr+4, addr+total*4+4]
                let pre_incr = (p == u) as u32;

                let mut rem = reglist;
                if s == 0 || (rem & (1 << reg::PC)) != 0 {
                    if w == 1 {
                        profile_reg_write!(self, rn, self.reg[rn] = post_addr);
                    }

                    for i in 0..16 {
                        if rem == 0 {
                            break;
                        }
                        let r = rem.trailing_zeros() as Reg;
                        let idx_addr = addr.wrapping_add((i + pre_incr) * 4);
                        if l == 0 {
                            // store
                            let val = if r == reg::PC {
                                pc.wrapping_add(12)
                            } else if r == rn && w == 1 && i == 0 {
                                orig_base
                            } else {
                                self.reg[r]
                            };
                            profile_span!(self, DataWrite, mmu.w32(idx_addr, val));
                        } else {
                            // load
                            profile_reg_write!(
                                self,
                                r,
                                self.reg[r] = profile_span!(self, DataRead, mmu.r32(idx_addr))
                            );
                            if r == reg::PC && s == 1 {
                                profile_reg_write!(
                                    self,
                                    reg::CPSR,
                                    self.reg[reg::CPSR] =
                                        profile_span!(self, FlagsCpsr, self.reg[reg::SPSR])
                                );
                                self.reg.update_bank();
                            }
                        };
                        rem -= 1u32 << r;
                    }
                } else {
                    for i in 0..16 {
                        if rem == 0 {
                            break;
                        }
                        let r = rem.trailing_zeros() as Reg;
                        let idx_addr = addr.wrapping_add((i + pre_incr) * 4);
                        if l == 0 {
                            // store
                            let val = self.reg.get(0, r);
                            profile_span!(self, DataWrite, mmu.w32(idx_addr, val));
                        } else {
                            // load
                            let val = profile_span!(self, DataRead, mmu.r32(idx_addr));
                            self.reg.set(0, r, val);
                        };
                        rem -= 1u32 << r;
                    }
                }
            }
            Swap => {
                let b = inst.get_bit(22);

                let rn = inst.extract(16, 4) as Reg;
                let rd = inst.extract(12, 4) as Reg;
                let rm = inst.extract(0, 4) as Reg;

                // If it is not a byte operation then force word align
                let addr = self.reg[rn] & !((1 - b) * 3);

                let val = match b {
                    0 => profile_span!(self, DataRead, mmu.r32(addr)),
                    1 => profile_span!(self, DataRead, mmu.r8(addr)) as u32,
                    _ => unreachable!(),
                };
                let oval = self.reg[rm];
                match b {
                    0 => profile_span!(self, DataWrite, mmu.w32(addr, oval)),
                    1 => profile_span!(self, DataWrite, mmu.w8(addr, oval as u8)),
                    _ => unreachable!(),
                };

                profile_reg_write!(self, rd, self.reg[rd] = val);
            }
            SoftwareInt => {
                profile_span!(self, Exception, self.exception(Exception::Software));
            }
            CoprocReg => {
                let cpopc = inst.extract(21, 3);
                let d = inst.get_bit(20);
                let cn = inst.extract(16, 4);
                let rd = inst.extract(12, 4) as Reg;
                let pn = inst.extract(8, 4);
                let cpinf = inst.extract(5, 3);
                let cm = inst.extract(0, 4);

                if d == 0 {
                    info!(
                        "Writing {:010x?} to P{},C{},C{},{}, cpinfo {}, mode: {:?}",
                        self.reg[rd],
                        pn,
                        cn,
                        cm,
                        cpopc,
                        cpinf,
                        self.mode()
                    );
                } else {
                    info!(
                        "Reading to R{} from P{},C{},C{},{}, cpinfo {}, mode: {:?}",
                        rd,
                        pn,
                        cn,
                        cm,
                        cpopc,
                        cpinf,
                        self.mode()
                    );
                    profile_reg_write!(self, rd, self.reg[rd] = 0);
                }
            }
            Undefined => return false,
        };

        true
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    #[rustfmt::skip]
    fn test_decode() {
        use super::Instruction::*;

        macro_rules! check (
            ($inst: expr, $val: expr) => {
                assert_eq!($inst, Instruction::decode($val));
            }
        );
        check!(BranchEx,    0xE12F_FF1C);
        check!(Branch,      0xEB00_00F8);
        check!(Clz,         0xE16F_6F12);
        check!(DataProc0,   0xE1A0_816C);
        check!(DataProc1,   0xE092_3011);
        check!(DataProc1,   0xC092_3011);
        check!(DataProc2,   0xE2A2_3AFF);
        check!(PsrImm,      0x1329_F000);
        check!(PsrReg,      0xE10F_A000);
        check!(Multiply,    0x8004_0393);
        check!(MulLong,     0xE083_4192);
        check!(SingleXferI, 0x85C6_7011);
        check!(SingleXferR, 0xB796_5100);
        check!(HwSgnXferR,  0xE10B_00B1);
        check!(HwSgnXferI,  0xE1EB_10B4);
        check!(BlockXfer,   0xE8BF_8006);
        check!(Swap,        0xE10D_1090);
        check!(SoftwareInt, 0xEF00_0000);
        check!(Undefined,   0xE7DE_AD10);
    }

    macro_rules! emutest {
        ($name:ident, $mem_checks: expr) => {
            #[test]
            fn $name() {
                use crate::ExampleMem;

                let prog = include_bytes!(concat!("../tests/data/", stringify!($name), ".bin"));
                let mut mmu = ExampleMem::new_with_data(prog);
                let mut cpu = Cpu::new();
                cpu.reg_set(Mode::User, reg::PC, 0x00);
                cpu.reg_set(Mode::User, reg::CPSR, 0x10);

                while cpu.step(&mut mmu) {}

                for &(addr, val) in ($mem_checks).iter() {
                    assert_eq!(val, mmu.r32(addr), "addr: {:#010x}", addr);
                }
            }
        };
    }

    emutest!(emutest_arm0, [(0x100, 5), (0x104, 0)]);
    emutest!(emutest_arm1, [(0x100, 5), (0x104, 5), (0x108, 5)]);
    emutest!(
        emutest_arm2,
        [(0x100, 6), (0x104, 0x2000_00e1), (0x108, 0xe100_001c)]
    );
    emutest!(emutest_arm3, [(0x100, 64)]);
    emutest!(
        emutest_arm4,
        [
            (0x100, 6),
            (0x104, 0x2000_00e1),
            (0x108, 0xe100_001c),
            (0x10c, 6),
            (0x110, 6 * 0x100),
        ]
    );
    emutest!(
        emutest_arm5,
        [(0x100, 0xf000), (0x104, 0xfff0), (0x108, 0x104)]
    );
    emutest!(
        emutest_arm6,
        [
            (0x1f4, 0xa),
            (0x1f8, 0xc),
            (0x1fc, 0x10),
            (0x200, 6),
            (0x204, 0x200),
        ]
    );
    emutest!(emutest_arm7, [(0x1fc, 1), (0x200, 1), (0x204, 0x200)]);
    emutest!(emutest_arm8, [(0x200, 10), (0x204, 83)]);
    emutest!(emutest_arm9, [(0x200, 55), (0x204, 66), (0x208, 77)]);
    emutest!(emutest_arm10, [(0x100, 24)]);
}
