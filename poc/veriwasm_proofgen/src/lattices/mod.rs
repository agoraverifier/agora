pub mod calllattice;
pub mod davlattice;
pub mod heaplattice;
pub mod localslattice;
pub mod reachingdefslattice;
pub mod regslattice;
pub mod stackgrowthlattice;
pub mod stacklattice;
pub mod switchlattice;
use crate::{ir, lattices};
use iced_asm::Register;
use ir::types::{Binopcode, MemArg, MemArgs, ValSize, Value, X86Regs};
use ir::utils::{get_imm_offset, is_rsp};
use lattices::reachingdefslattice::LocIdx;
use lattices::regslattice::X86RegsLattice;
use lattices::stacklattice::StackLattice;
use log::info;
use ptir::*;
use std::cmp::Ordering;
use std::fmt::Debug;

use X86Regs::*;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct VarSlot<T> {
    pub size: u32,
    pub value: T,
}

impl<T: PartialOrd> PartialOrd for VarSlot<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.size != other.size {
            None
        } else {
            self.value.partial_cmp(&other.value)
        }
    }
}

pub trait Lattice: PartialOrd + Eq + Default + Debug {
    fn meet(&self, other: &Self, loc: &LocIdx) -> Self;
}

pub trait VarState {
    type Var;
    fn get(&self, index: &Value) -> Option<Self::Var>;
    fn set(&mut self, index: &Value, v: Self::Var) -> ();
    fn set_to_bot(&mut self, index: &Value) -> ();
    fn on_call(&mut self) -> ();
    fn adjust_stack_offset(
        &mut self,
        opcode: &Binopcode,
        dst: &Value,
        src1: &Value,
        src2: &Value,
    ) -> Option<Proof>;
}

#[derive(Eq, Clone, Copy, Debug)]
pub struct BooleanLattice {
    v: bool,
}

impl PartialOrd for BooleanLattice {
    fn partial_cmp(&self, other: &BooleanLattice) -> Option<Ordering> {
        Some(self.v.cmp(&other.v))
    }
}

impl PartialEq for BooleanLattice {
    fn eq(&self, other: &BooleanLattice) -> bool {
        self.v == other.v
    }
}

impl Lattice for BooleanLattice {
    fn meet(&self, other: &Self, _loc_idx: &LocIdx) -> Self {
        BooleanLattice {
            v: self.v && other.v,
        }
    }
}

impl Default for BooleanLattice {
    fn default() -> Self {
        BooleanLattice { v: false }
    }
}

pub type Constu32Lattice = ConstLattice<u32>;

#[derive(Eq, Clone, Debug)]
pub struct ConstLattice<T: Eq + Clone + Debug> {
    pub v: Option<T>,
}

impl<T: Eq + Clone + Debug> PartialOrd for ConstLattice<T> {
    fn partial_cmp(&self, other: &ConstLattice<T>) -> Option<Ordering> {
        match (self.v.as_ref(), other.v.as_ref()) {
            (None, None) => Some(Ordering::Equal),
            (None, _) => Some(Ordering::Less),
            (_, None) => Some(Ordering::Greater),
            (Some(x), Some(y)) => {
                if x == y {
                    Some(Ordering::Equal)
                } else {
                    None
                }
            }
        }
    }
}

impl<T: Eq + Clone + Debug> PartialEq for ConstLattice<T> {
    fn eq(&self, other: &ConstLattice<T>) -> bool {
        self.v == other.v
    }
}

impl<T: Eq + Clone + Debug> Lattice for ConstLattice<T> {
    fn meet(&self, other: &Self, _loc_idx: &LocIdx) -> Self {
        if self.v == other.v {
            ConstLattice { v: self.v.clone() }
        } else {
            ConstLattice { v: None }
        }
    }
}

impl<T: Eq + Clone + Debug> Default for ConstLattice<T> {
    fn default() -> Self {
        ConstLattice { v: None }
    }
}

impl<T: Eq + Clone + Debug> ConstLattice<T> {
    pub fn new(v: T) -> Self {
        ConstLattice { v: Some(v) }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Default, Clone, Debug)]
pub struct VariableState<T> {
    pub regs: X86RegsLattice<T>,
    pub stack: StackLattice<T>,
}

impl<T: std::fmt::Debug + Clone> std::fmt::Display for VariableState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{\n\t{:?}\n\n\t{}\n}}", self.regs, self.stack)
    }
}

// offset from current stack pointer
// returns None if points to heap
pub fn mem_to_stack_offset(memargs: &MemArgs) -> Option<i64> {
    match memargs {
        MemArgs::Mem1Arg(arg) => {
            if let MemArg::Reg(regnum, _) = arg {
                if *regnum == Rsp {
                    return Some(0);
                }
            }
        }
        MemArgs::Mem2Args(arg1, arg2) => {
            if let MemArg::Reg(regnum, _) = arg1 {
                if *regnum == Rsp {
                    if let MemArg::Imm(_, _, offset) = arg2 {
                        return Some(*offset);
                    }
                }
            }
        }
        _ => (),
    }
    return None;
}

impl<T: Lattice + Clone> Lattice for VariableState<T> {
    fn meet(&self, other: &Self, loc_idx: &LocIdx) -> Self {
        VariableState {
            regs: self.regs.meet(&other.regs, loc_idx),
            stack: self.stack.meet(&other.stack, loc_idx),
        }
    }
}

impl<T: Lattice + Clone> VarState for VariableState<T> {
    type Var = T;
    fn set(&mut self, index: &Value, value: T) -> () {
        match index {
            Value::Mem(memsize, memargs) => {
                if let Some(offset) = mem_to_stack_offset(memargs) {
                    self.stack.update(offset, value, memsize.into_bytes())
                }
            }
            Value::Reg(regnum, s2) => self.regs.set_reg(*regnum, *s2, value),
            Value::Imm(_, _, _) => panic!("Trying to write to an immediate value"),
            Value::RIPConst => panic!("Trying to write to a RIP constant"),
        }
    }

    fn get(&self, index: &Value) -> Option<T> {
        match index {
            Value::Mem(memsize, memargs) => mem_to_stack_offset(memargs)
                .map(|offset| self.stack.get(offset, memsize.into_bytes())),
            Value::Reg(regnum, s2) => Some(self.regs.get_reg(*regnum, *s2)),
            Value::Imm(_, _, _) => None,
            Value::RIPConst => None,
        }
    }

    fn set_to_bot(&mut self, index: &Value) {
        self.set(index, Default::default())
    }

    fn on_call(&mut self) {
        self.regs.clear_caller_save_regs();
    }

    fn adjust_stack_offset(
        &mut self,
        opcode: &Binopcode,
        dst: &Value,
        src1: &Value,
        src2: &Value,
    ) -> Option<Proof> {
        if is_rsp(dst) {
            if is_rsp(src1) {
                let adjustment = get_imm_offset(src2);
                match opcode {
                    Binopcode::Add => self.stack.update_stack_offset(adjustment),
                    Binopcode::Sub => self.stack.update_stack_offset(-adjustment),
                    _ => panic!("Illegal RSP write"),
                }
                let sign = match opcode {
                    Binopcode::Add => "+",
                    Binopcode::Sub => "-",
                    _ => panic!("Illegal RSP write"),
                };
                log::info!("Adjusting stack offset [RSP = RSP {} {}]", sign, adjustment);

                let left = LocationBuilder::new().register(Register::RSP).build();

                let src1 = LocationBuilder::new().register(Register::RSP).build();

                let imm = Expr::Imm(Imm::from(adjustment.unsigned_abs() as u64));

                let right: Expr = match opcode {
                    Binopcode::Add => Expr::Binary(
                        ptir::BinaryOp::BV(BVBinaryOp::Arith(BVBinaryArith::Add)),
                        Box::new(Expr::Var(src1)),
                        Box::new(imm),
                    ),
                    Binopcode::Sub => Expr::Binary(
                        ptir::BinaryOp::BV(BVBinaryOp::Arith(BVBinaryArith::Sub)),
                        Box::new(Expr::Var(src1)),
                        Box::new(imm),
                    ),
                    _ => panic!("Illegal RSP write"),
                };

                let assign = Assignment::new(left, right);

                Some(Proof::Asgn(assign))
            } else {
                panic!("Illegal RSP write")
            }
        } else {
            None
        }
    }
}

#[test]
fn boolean_lattice_test() {
    let x = BooleanLattice { v: false };
    let y = BooleanLattice { v: true };
    assert_eq!(x < y, true);
    assert_eq!(x > y, false);
    assert_eq!(x.lt(&y), true);
}

#[test]
fn u32_lattice_test() {
    let x1 = ConstLattice::<u32> { v: Some(1) };
    let x2 = ConstLattice::<u32> { v: Some(1) };
    let y1 = ConstLattice::<u32> { v: Some(2) };
    let y2 = ConstLattice::<u32> { v: Some(2) };

    let z1 = Constu32Lattice { v: Some(3) };
    let z2 = Constu32Lattice { v: Some(3) };

    assert_eq!(x1 < y1, false);
    assert_eq!(y1 < x1, false);
    assert_eq!(x1 == x2, true);
    assert_eq!(x1 != x2, false);
    assert_eq!(y2 != x1, true);
    assert_eq!(x1 >= y1, false);
    assert_eq!(x1 > x2, false);
    assert_eq!(x1 >= x2, true);
    assert_eq!(z1 == z2, true);
    assert_eq!(z1 == x1, false);
    assert_eq!(x1.lt(&y1), false);
}
