use std::{collections::HashMap, fmt::Debug};

use crate::bytecode::{Instruction, Value, Int, GpRegister, Condition, CRegister};

pub struct Env<'a> {
    pub map: &'a HashMap<String, Int>,
    pub position: Int
}
pub type IrInstruction = Box<dyn FnOnce(Env<'_>) -> Result<Instruction, String>>;

pub enum InterRep {
    Instruction(IrInstruction),
    Label(String),
}

#[derive(Debug, Clone)]
pub enum InterRepValue {
    ByteCodeValue(Value),
    LabelRef(String),
    Here
}

pub trait Convert<T> {
    fn convert(&self, labels: &Env) -> Result<T, String>;
}

impl Convert<Value> for InterRepValue {
    fn convert(&self, labels: &Env) -> Result<Value, String> {
        match self {
            InterRepValue::ByteCodeValue(bcv) => Ok(*bcv),
            InterRepValue::LabelRef(lab) => 
                labels.map.get(lab).map(|n| Value::Imm(*n)).ok_or_else(||lab.clone()),
            InterRepValue::Here => Ok(Value::Imm(labels.position))
        }
    }
}

impl Convert<GpRegister> for GpRegister {
    fn convert(&self, _: &Env) -> Result<GpRegister, String> {
        Ok(*self)
    }
}

impl Convert<CRegister> for CRegister {
    fn convert(&self, _: &Env) -> Result<CRegister, String> {
        Ok(*self)
    }
}

impl Convert<Condition> for Condition {
    fn convert(&self, _: &Env) -> Result<Condition, String> {
        Ok(*self)
    }
}