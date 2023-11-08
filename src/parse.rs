use pom::parser::*;

use crate::bytecode::Instruction;
use crate::bytecode::Value;
use crate::bytecode::Condition;
use crate::bytecode::GpRegister;
use crate::bytecode::Int;
use crate::ir::InterRep;
use crate::ir::InterRepValue;
use crate::ir::IrInstruction;
use crate::ir::Convert;
use crate::ir::Env;

fn gp_reg<'a>() -> Parser<'a, u8, GpRegister> {
    seq(b"r0").map(|_|GpRegister::R0)
    | seq(b"r1").map(|_|GpRegister::R1)
    | seq(b"r2").map(|_|GpRegister::R2)
    | seq(b"r3").map(|_|GpRegister::R3)
    | seq(b"r4").map(|_|GpRegister::R4)
    | seq(b"r5").map(|_|GpRegister::R5)
    | seq(b"r6").map(|_|GpRegister::R6)
    | seq(b"sp").map(|_|GpRegister::SP)
}

fn number<'a>() -> Parser<'a, u8, Int> {
    let integer = one_of(b"0123456789").repeat(1..);
	let number = sym(b'-').opt() + integer;
	number.collect().convert(std::str::from_utf8).convert(|s| s.parse())
}

fn label<'a>() -> Parser<'a, u8, String> {
    let string = is_a(|n: u8| n.is_ascii() && (n as char).is_alphabetic()).repeat(1..);

    string.convert(String::from_utf8)
}

fn value<'a>() -> Parser<'a, u8, InterRepValue> {
    gp_reg().map(|r| InterRepValue::ByteCodeValue(Value::Reg(r)))
    | number().map(|n| InterRepValue::ByteCodeValue(Value::Imm(n)))
    | (sym(b'#') * label()).map(|s| InterRepValue::LabelRef(s))
    | sym(b'.').map(|_| InterRepValue::Here)
}

fn cond<'a>() -> Parser<'a, u8, Condition> {
    seq(b"<").map(|_|Condition::L)
    | seq(b"<=").map(|_|Condition::LE)
    | seq(b"==").map(|_|Condition::E)
    | seq(b">=").map(|_|Condition::GE)
    | seq(b">").map(|_|Condition::G)
}

fn space<'a>() -> Parser<'a, u8, ()> {
	one_of(b" \t\r\n").repeat(0..).discard()
}

macro_rules! instr {
    ($gen:expr, $name:ident, $a:expr) => {
        seq(stringify!($name).as_bytes()) * space() * $a.map(|l| {
            Box::new(move |labels: Env| Ok($gen(l.convert(&labels)?))) as IrInstruction
        })
    };

    ($gen:expr, $name:ident, $a:expr, $b:expr) => {
        seq(stringify!($name).as_bytes()) * space() * (($a - space()) + $b).map(|(l, r)| {
            Box::new(move |labels: Env| Ok($gen(l.convert(&labels)?, r.convert(&labels)?))) as IrInstruction
        })
    };

    ($gen:expr, $name:ident, $a:expr, $b:expr, $c:expr) => {
        seq(stringify!($name).as_bytes()) * space() * (($a - space()) + ($b - space()) + $c).map(|((l, c), r)| {
            Box::new(move |labels: Env| Ok($gen(l.convert(&labels)?, c.convert(&labels)?, r.convert(&labels)?))) as IrInstruction
        })
    };
}

fn instruction<'a>() -> Parser<'a, u8, IrInstruction> {
    use Instruction::*;

    instr!(Mov, mov, gp_reg(), value())

    | instr!(Add, add, gp_reg(), value())
    | instr!(Sub, sub, gp_reg(), value())
    | instr!(Mul, mul, gp_reg(), value())
    | instr!(Div, div, gp_reg(), value())

    | instr!(And, and, gp_reg(), value())
    | instr!(Or, or, gp_reg(), value())
    | instr!(Xor, xor, gp_reg(), value())
    | instr!(Not, not, gp_reg())

    | instr!(Load, load, gp_reg(), value())
    | instr!(Store, store, value(), value())
    | instr!(Jmp, jmp, value())

    | instr!(Push, push, value())
    | instr!(Pop, pop, gp_reg())

    | instr!(Cond, cond, gp_reg(), cond(), value())

    | instr!(Emit, emit, value())
}

fn statement<'a>() -> Parser<'a, u8, InterRep> {
    instruction().map(|i| InterRep::Instruction(i))
    | (label() - sym(b':')).map(InterRep::Label)
}


fn program<'a>() -> Parser<'a, u8, Vec<InterRep>> {
    // list(instruction(), space())
    // instruction().map(|i| vec![i])
    space() * (statement() - space()).repeat(0..) - end()
}

pub fn parse(input: &str) -> Result<Vec<InterRep>, pom::Error> {
    let bytes = input.as_bytes();
    program().parse(bytes)
}
