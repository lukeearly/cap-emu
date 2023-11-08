use std::{collections::HashMap, mem::size_of};

use crate::{bytecode::{Instruction, Int}, ir::{InterRep, Env}};

#[derive(Debug)]
pub enum CompileError {
    UndefinedLabel(String)
}

pub fn compile(inter_rep: Vec<InterRep>) -> Result<Vec<Instruction>, CompileError> {
    let mut labels: HashMap<String, Int> = HashMap::new();

    let mut instrs = Vec::new();

    for ir in inter_rep {
        match ir {
            InterRep::Instruction(i) => {
                instrs.push(i)
            },
            InterRep::Label(name) => {
                labels.insert(name, (instrs.len() * size_of::<Instruction>()).try_into().unwrap());
            },
        }
    }

    let res: Result<Vec<Instruction>, String> = instrs.into_iter().enumerate().map(|(n, instr)| {
        instr(Env { map: &labels, position: (n * size_of::<Instruction>()) as Int }).clone()
    }).collect();

    res.map_err(|name| CompileError::UndefinedLabel(format!("Undefined label: {name}")))
}