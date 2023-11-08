pub type Int = i16;

#[repr(align(4))]
#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Mov(GpRegister, Value),
    
    Add(GpRegister, Value),
    Sub(GpRegister, Value),
    Mul(GpRegister, Value),
    Div(GpRegister, Value),

    And(GpRegister, Value),
    Or(GpRegister, Value),
    Xor(GpRegister, Value),
    Not(GpRegister),

    Load(GpRegister, CRegister, Value),
    Store(CRegister, Value, Value),
    Jmp(CRegister, Value),

    Push(Value),
    Pop(GpRegister),

    Cond(GpRegister, Condition, Value),

    Emit(Value),

    // CLoad(GpRegister, CRegister),
    // CStore(Value, CRegister),

    // CLoadCap(CRegister, CRegister),
    // CStoreCap(CRegister, CRegister),
    // CJmp(CRegister),

    // CPushCap(CRegister),
    // CPopCap(CRegister),

    // CInvoke(CRegister, CRegister),

    // CRestrict(CRegister, Value),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Condition {
    L,
    LE,
    E,
    GE,
    G
}

impl Condition {
    pub fn test(&self, left: Int, right: Int) -> bool {
        match self {
            Condition::L => left < right,
            Condition::LE => left <= right,
            Condition::E => left == right,
            Condition::GE => left >= right,
            Condition::G => left > right,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Reg(GpRegister),
    Imm(Int),
}

pub const GP_REGISTERS: usize = 8;
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum GpRegister {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    SP,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum CRegister {
    C0,
    C1,
    C2,
    C3,
    C4,
    C5,
    CC,
    DD
}