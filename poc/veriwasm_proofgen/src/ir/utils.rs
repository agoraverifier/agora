use crate::ir::types::*;

use ValSize::*;
use X86Regs::*;

pub fn is_rsp(v: &Value) -> bool {
    match v {
        Value::Reg(Rsp, Size64) => return true,
        Value::Reg(Rsp, Size32) | Value::Reg(Rsp, Size16) | Value::Reg(Rsp, Size8) => {
            panic!("Illegal RSP access")
        }
        _ => return false,
    }
}

pub fn is_rbp(v: &Value) -> bool {
    match v {
        Value::Reg(Rbp, Size64) => return true,
        Value::Reg(Rbp, Size32) | Value::Reg(Rbp, Size16) | Value::Reg(Rbp, Size8) => {
            panic!("Illegal RBP access")
        }
        _ => return false,
    }
}

pub fn is_zf(v: &Value) -> bool {
    match v {
        Value::Reg(Zf, _) => return true,
        _ => return false,
    }
}

pub fn memarg_is_stack(memarg: &MemArg) -> bool {
    if let MemArg::Reg(Rsp, regsize) = memarg {
        if let Size64 = regsize {
            return true;
        } else {
            panic!("Non 64 bit version of rsp being used")
        };
    }
    return false;
}

pub fn is_stack_access(v: &Value) -> bool {
    if let Value::Mem(_size, memargs) = v {
        match memargs {
            MemArgs::Mem1Arg(memarg) => return memarg_is_stack(memarg),
            MemArgs::Mem2Args(memarg1, memarg2) => {
                return memarg_is_stack(memarg1) || memarg_is_stack(memarg2)
            }
            MemArgs::Mem3Args(memarg1, memarg2, memarg3) => {
                return memarg_is_stack(memarg1)
                    || memarg_is_stack(memarg2)
                    || memarg_is_stack(memarg3)
            }
            MemArgs::MemScale(memarg1, memarg2, memarg3) => {
                return memarg_is_stack(memarg1)
                    || memarg_is_stack(memarg2)
                    || memarg_is_stack(memarg3)
            }
        }
    }
    false
}

pub fn memarg_is_frame(memarg: &MemArg) -> bool {
    if let MemArg::Reg(Rbp, size) = memarg {
        assert_eq!(*size, Size64);
        true
    } else {
        false
    }
}

pub fn is_frame_access(v: &Value) -> bool {
    if let Value::Mem(_, memargs) = v {
        // Accept only operands of the form `[rbp + OFFSET]` where `OFFSET` is
        // an integer. In Cranelift-generated code from Wasm, there are never
        // arrays or variable-length data in the function frame, so there should
        // never be a computed address (e.g., `[rbp + 4*eax + OFFSET]`).
        match memargs {
            MemArgs::Mem1Arg(memarg) => memarg_is_frame(memarg),
            MemArgs::Mem2Args(memarg1, memarg2) => {
                memarg_is_frame(memarg1) && matches!(memarg2, MemArg::Imm(..))
            }
            _ => false,
        }
    } else {
        false
    }
}

pub fn memarg_is_bp(memarg: &MemArg) -> bool {
    if let MemArg::Reg(Rbp, regsize) = memarg {
        if let Size64 = regsize {
            return true;
        } else {
            panic!("Non 64 bit version of rbp being used")
        };
    }
    return false;
}

pub fn is_bp_access(v: &Value) -> bool {
    if let Value::Mem(_size, memargs) = v {
        match memargs {
            MemArgs::Mem1Arg(memarg) => return memarg_is_bp(memarg),
            MemArgs::Mem2Args(memarg1, memarg2) => {
                return memarg_is_bp(memarg1) || memarg_is_bp(memarg2)
            }
            MemArgs::Mem3Args(memarg1, memarg2, memarg3) => {
                return memarg_is_bp(memarg1) || memarg_is_bp(memarg2) || memarg_is_bp(memarg3)
            }
            MemArgs::MemScale(memarg1, memarg2, memarg3) => {
                return memarg_is_bp(memarg1) || memarg_is_bp(memarg2) || memarg_is_bp(memarg3)
            }
        }
    }
    false
}

pub fn extract_stack_offset(memargs: &MemArgs) -> i64 {
    match memargs {
        MemArgs::Mem1Arg(_memarg) => 0,
        MemArgs::Mem2Args(_memarg1, memarg2) => get_imm_mem_offset(memarg2),
        MemArgs::Mem3Args(_memarg1, _memarg2, _memarg3)
        | MemArgs::MemScale(_memarg1, _memarg2, _memarg3) => panic!("extract_stack_offset failed"),
    }
}

pub fn is_mem_access(v: &Value) -> bool {
    if let Value::Mem(_, _) = v {
        true
    } else {
        false
    }
}

pub fn get_imm_offset(v: &Value) -> i64 {
    if let Value::Imm(_, _, v) = v {
        *v
    } else {
        panic!("get_imm_offset called on something that is not an imm offset")
    }
}

pub fn get_imm_mem_offset(v: &MemArg) -> i64 {
    if let MemArg::Imm(_, _, v) = v {
        *v
    } else {
        panic!("get_imm_offset called on something that is not an imm offset")
    }
}

pub fn has_indirect_calls(irmap: &IRMap) -> bool {
    for (_block_addr, ir_block) in irmap {
        for (_addr, ir_stmts) in ir_block {
            for (_idx, ir_stmt) in ir_stmts.iter().enumerate() {
                match ir_stmt {
                    Stmt::Call(Value::Reg(_, _)) | Stmt::Call(Value::Mem(_, _)) => return true,
                    _ => (),
                }
            }
        }
    }
    false
}

pub fn has_indirect_jumps(irmap: &IRMap) -> bool {
    for (_block_addr, ir_block) in irmap {
        for (_addr, ir_stmts) in ir_block {
            for (_idx, ir_stmt) in ir_stmts.iter().enumerate() {
                match ir_stmt {
                    Stmt::Branch(_, Value::Reg(_, _)) | Stmt::Branch(_, Value::Mem(_, _)) => {
                        return true
                    }
                    _ => (),
                }
            }
        }
    }
    false
}

pub fn get_rsp_offset(memargs: &MemArgs) -> Option<i64> {
    match memargs {
        MemArgs::Mem1Arg(arg) => {
            if let MemArg::Reg(regnum, _) = arg {
                if *regnum == Rsp {
                    return Some(0);
                }
            }
            None
        }
        MemArgs::Mem2Args(arg1, arg2) => {
            if let MemArg::Reg(regnum, _) = arg1 {
                if *regnum == Rsp {
                    if let MemArg::Imm(_, _, offset) = arg2 {
                        return Some(*offset);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

pub fn valsize(num: u32) -> ValSize {
    ValSize::try_from_bits(num).unwrap()
}

pub fn mk_value_i64(num: i64) -> Value {
    Value::Imm(ImmType::Signed, Size64, num)
}
