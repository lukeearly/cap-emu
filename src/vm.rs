use std::{ops::{IndexMut, Index}, mem::{align_of, size_of}, fmt::Display};

use crate::bytecode::{Int, GpRegister, Instruction, Value, GP_REGISTERS};

pub struct Machine {
    pub memory: Memory,
    pub reg: RegisterFile
}

#[derive(Default)]
pub struct RegisterFile {
    gp: [Int; GP_REGISTERS],
    pc: Int
}

impl Index<GpRegister> for RegisterFile {
    type Output = Int;

    fn index(&self, index: GpRegister) -> &Self::Output {
        &self.gp[index as usize]
    }
}

impl IndexMut<GpRegister> for RegisterFile {
    fn index_mut(&mut self, index: GpRegister) -> &mut Self::Output {
        &mut self.gp[index as usize]
    }
}

impl Display for RegisterFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..GP_REGISTERS as u8 {
            // safe because i in [0, GP_REGISTERS)
            let gp: GpRegister = unsafe { std::mem::transmute(i) };

            writeln!(f, "{:?} {:04x}", gp, self[gp])?;
        }
        writeln!(f, "PC {:04x} ({})", self.pc, self.pc as usize / size_of::<Instruction>())?;
        Ok(())
    }
}

const MEMORY_SIZE: usize = 4096;

#[repr(align(4))]
pub struct Memory {
    mem: [u8; MEMORY_SIZE]
}

impl Memory {
    unsafe fn get_mut_ptr<T>(&self, addr: Int, count: usize) -> Result<*mut T, RuntimeError> {
        if addr as usize + count * size_of::<T>() > MEMORY_SIZE {
            return Err(RuntimeError::OutOfBoundsAccess(addr));
        }

        let ptr = unsafe { self.mem.as_ptr().add(addr as usize) as *mut T };

        let align = align_of::<T>();
        if (ptr as usize) % align != 0 {
            return Err(RuntimeError::UnalignedAccess { addr, align: align as Int })
        }

        Ok(ptr)
    }

    pub fn load<T: Copy>(&self, addr: Int) -> Result<T, RuntimeError> {
        unsafe {
            let ptr = self.get_mut_ptr(addr, 1)?;
            let data = *ptr;

            Ok(data)
        }
    }

    pub fn store<T>(&mut self, addr: Int, data: T) -> Result<(), RuntimeError> {
        unsafe {
            let ptr = self.get_mut_ptr(addr, 1)?;
            *ptr = data;

            Ok(())
        }
    }

    pub fn store_slice<T>(&mut self, addr: Int, data: &[T]) -> Result<(), RuntimeError> {
        unsafe {
            let ptr: *mut T = self.get_mut_ptr(addr, data.len())?;

            let size = data.len() * size_of::<T>();

            let dest_slice = std::slice::from_raw_parts_mut(ptr as *mut u8, size);

            let src_slice = std::slice::from_raw_parts(data.as_ptr() as *const u8, size);

            dest_slice.copy_from_slice(src_slice);

            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    UnalignedAccess { addr: Int, align: Int },
    OutOfBoundsAccess(Int),
}

impl Machine {
    pub fn new() -> Self {
        Self { memory: Memory { mem: [0; MEMORY_SIZE] }, reg: Default::default() }
    }

    pub fn tick(&mut self) -> Result<(), RuntimeError> {
        let pc = self.reg.pc;
        let instr = self.memory.load(pc)?;

        if !self.execute_instruction(instr)? {
            self.reg.pc += size_of::<Instruction>() as Int;
        }

        Ok(())
    }

    fn execute_instruction(&mut self, instr: Instruction) -> Result<bool, RuntimeError> {
        // println!("{:?}", instr);
        use Instruction::*;
        match instr {
            Mov(a, b) => self.reg[a] = self.eval(b),

            Add(a, b) => self.reg[a] = self.reg[a].saturating_add(self.eval(b)),
            Sub(a, b) => self.reg[a] = self.reg[a].saturating_sub(self.eval(b)),
            Mul(a, b) => self.reg[a] = self.reg[a].saturating_mul(self.eval(b)),
            Div(a, b) => self.reg[a] = self.reg[a].saturating_div(self.eval(b)),

            And(a, b) => self.reg[a] &= self.eval(b),
            Or(a, b) => self.reg[a] |= self.eval(b),
            Xor(a, b) => self.reg[a] ^= self.eval(b),
            Not(a) => self.reg[a] = !self.reg[a],

            Load(a, b) => self.reg[a] = self.memory.load(self.eval(b))?,
            Store(a, b) => self.memory.store(self.eval(b), self.eval(a))?,

            Jmp(a) => {
                self.reg.pc = self.eval(a);
                return Ok(true)
            },

            Push(a) => {
                let addr = self.reg[GpRegister::SP].saturating_sub(size_of::<Int>() as Int);
                self.memory.store(addr, self.eval(a))?;
                self.reg[GpRegister::SP] = addr;
            },

            Pop(a) => {
                self.reg[a] = self.memory.load(self.reg[GpRegister::SP])?;
                self.reg[GpRegister::SP] = self.reg[GpRegister::SP].saturating_add(size_of::<Int>() as Int);
            },

            Cond(a, c, b) => {
                if !c.test(self.reg[a], self.eval(b)) {
                    self.reg.pc += 2 * size_of::<Instruction>() as Int;
                }
            }
        }

        Ok(false)
    }

    fn eval(&self, value: Value) -> Int {
        match value {
            Value::Reg(gp) => self.reg[gp],
            Value::Imm(imm) => imm,
        }
    }
}