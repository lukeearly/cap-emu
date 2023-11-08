use std::{ops::{IndexMut, Index}, mem::{align_of, size_of}, fmt::Display};

use crate::{bytecode::{Int, GpRegister, Instruction, Value, GP_REGISTERS, CRegister}, capability::{Capability, Inner, CAP_SIZE, Permissions, Seal}};

pub struct Machine {
    pub memory: Memory,
    pub reg: RegisterFile
}

#[derive(Default)]
pub struct RegisterFile {
    gp: [Int; GP_REGISTERS],
    cap: [Capability; GP_REGISTERS],
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

impl Index<CRegister> for RegisterFile {
    type Output = Capability;

    fn index(&self, index: CRegister) -> &Self::Output {
        &self.cap[index as usize]
    }
}

impl IndexMut<CRegister> for RegisterFile {
    fn index_mut(&mut self, index: CRegister) -> &mut Self::Output {
        &mut self.cap[index as usize]
    }
}

impl Display for RegisterFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..GP_REGISTERS as u8 {
            // safe because i in [0, GP_REGISTERS)
            let gp: GpRegister = unsafe { std::mem::transmute(i) };

            write!(f, "{:?} {:04x}", gp, self[gp])?;

            let cap: CRegister = unsafe { std::mem::transmute(i) };
            writeln!(f, "\t\t{:?} {:?}", cap, self[cap])?;
        }
        writeln!(f, "PC {:04x} ({})", self.pc, self.pc as usize / size_of::<Instruction>())?;
        Ok(())
    }
}

const MEMORY_SIZE: usize = 4096;

#[repr(align(4))]
pub struct Memory {
    mem: [u8; MEMORY_SIZE],
    cap_tags: [u8; MEMORY_SIZE / CAP_SIZE / 8]
}

impl Memory {
    unsafe fn get_mut_ptr<T>(&self, addr: Int, count: usize) -> Result<*mut T, RuntimeError> {
        let ptr = unsafe { self.mem.as_ptr().add(addr as usize) as *mut T };

        let align = align_of::<T>();
        if (ptr as usize) % align != 0 {
            return Err(RuntimeError::UnalignedAccess { addr, align: align as Int })
        }

        Ok(ptr)
    }

    unsafe fn checked_ptr<T>(&self, cap: Capability, offset: Int, count: usize) -> Result<*mut T, RuntimeError> {
        if !cap.valid {
            return Err(RuntimeError::InvalidCapability(cap))
        }

        let bounds = cap.inner.bounds();
        let bounds_usize = bounds.start as usize .. bounds.end as usize;
        let start = cap.inner.ptr() + offset;
        let end = start as usize + size_of::<T>() * count;

        if !bounds.contains(&start) || !bounds_usize.contains(&(end - 1)) || end > MEMORY_SIZE {
            return Err(RuntimeError::OutOfBoundsAccess(cap))
        }

        let ptr = self.get_mut_ptr(cap.inner.ptr() + offset, count)?;

        Ok(ptr)
    }

    pub fn load<T: Copy>(&self, cap: Capability, offset: Int) -> Result<T, RuntimeError> {
        unsafe {
            if !cap.inner.perms().read {
                return Err(RuntimeError::InsufficientPermissions(cap))
            }

            let ptr = self.checked_ptr(cap, offset, 1)?;
            let data = *ptr;

            Ok(data)
        }
    }

    pub fn fetch(&self, cap: Capability, offset: Int) -> Result<Instruction, RuntimeError> {
        unsafe {
            if !cap.inner.perms().exec {
                return Err(RuntimeError::InsufficientPermissions(cap))
            }

            let ptr = self.checked_ptr(cap, offset, 1)?;
            let data = *ptr;

            Ok(data)
        }
    }

    pub fn store<T>(&mut self, cap: Capability, offset: Int, data: T) -> Result<(), RuntimeError> {
        unsafe {
            if !cap.inner.perms().write {
                return Err(RuntimeError::InsufficientPermissions(cap))
            }

            let ptr = self.checked_ptr(cap, offset, 1)?;

            self.invalidate_range(cap.inner.ptr() + offset, size_of::<T>());

            *ptr = data;

            Ok(())
        }
    }

    pub fn store_slice<T>(&mut self, cap: Capability, offset: Int, data: &[T]) -> Result<(), RuntimeError> {
        unsafe {
            if !cap.inner.perms().write {
                return Err(RuntimeError::InsufficientPermissions(cap))
            }

            let ptr = self.checked_ptr(cap, offset, data.len())?;

            let size = data.len() * size_of::<T>();

            self.invalidate_range(cap.inner.ptr() + offset, size);

            let dest_slice = std::slice::from_raw_parts_mut(ptr as *mut u8, size);

            let src_slice = std::slice::from_raw_parts(data.as_ptr() as *const u8, size);

            dest_slice.copy_from_slice(src_slice);

            Ok(())
        }
    }

    pub fn load_cap(&self, cap: Capability, offset: Int) -> Result<Capability, RuntimeError> {
        unsafe {
            if !cap.inner.perms().read {
                return Err(RuntimeError::InsufficientPermissions(cap))
            }

            let ptr = self.checked_ptr(cap, offset, 1)?;
            let inner = *ptr;
            let valid = self.get_cap_tag(((cap.inner.ptr() + offset) / CAP_SIZE as i16).try_into().unwrap());

            Ok(Capability { inner, valid })
        }
    }

    pub fn store_cap(&mut self, cap: Capability, offset: Int, data: Capability) -> Result<(), RuntimeError> {
        unsafe {
            if !cap.inner.perms().write {
                return Err(RuntimeError::InsufficientPermissions(cap))
            }

            let ptr = self.checked_ptr(cap, offset, 1)?;

            let addr = cap.inner.ptr() + offset;

            self.invalidate_range(addr, size_of::<Inner>());

            self.set_cap_tag((addr / CAP_SIZE as i16).try_into().unwrap(), data.valid);

            *ptr = data.inner;

            Ok(())
        }
    }


    unsafe fn set_cap_tag(&mut self, idx: usize, valid: bool) {
        if valid {
            self.cap_tags[idx / 8] |= 1 << idx % 8;
        } else {
            self.cap_tags[idx / 8] &= !(1 << idx % 8);
        }
    }

    unsafe fn get_cap_tag(&self, idx: usize) -> bool {
        self.cap_tags[idx / 8] & 1 << idx % 8 != 0
    }

    unsafe fn invalidate_range(&mut self, addr: Int, size: usize) {
        let start = addr as usize / CAP_SIZE;
        let end = (addr as usize + size - 1) / CAP_SIZE;
        // if size is 0, then end = start - 1 and i..=(i-1) is an empty range
        for i in start..=end {
            self.set_cap_tag(i, false);
        }
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            mem: [0; MEMORY_SIZE],
            cap_tags: [0; MEMORY_SIZE / CAP_SIZE / 8],
        }
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    UnalignedAccess { addr: Int, align: Int },
    OutOfBoundsAccess(Capability),
    InvalidCapability(Capability),
    InsufficientPermissions(Capability),
}

impl Machine {
    pub fn new() -> Self {
        let mut mach = Self { memory: Default::default(), reg: Default::default() };
        let cap = Capability {
            inner: Inner::new(0, 0..MEMORY_SIZE as Int, Permissions::rwx(true, true, true), Seal::Unsealed),
            valid: true,
        };
        mach.reg[CRegister::CC] = cap; 
        mach.reg[CRegister::DD] = cap; 
        mach
    }

    pub fn tick(&mut self) -> Result<(), RuntimeError> {
        let instr = self.memory.fetch(self.reg[CRegister::CC], self.reg.pc)?;

        if !self.execute_instruction(instr)? {
            self.reg.pc += size_of::<Instruction>() as Int;
        }

        Ok(())
    }

    fn execute_instruction(&mut self, instr: Instruction) -> Result<bool, RuntimeError> {
        // println!("{:?}", instr);
        use Instruction::*;
        use CRegister::{CC, DD};
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

            Load(a, b) => self.reg[a] = self.memory.load(self.reg[DD], self.eval(b))?,
            Store(a, b) => self.memory.store(self.reg[DD], self.eval(b), self.eval(a))?,

            Jmp(a) => {
                self.reg.pc = self.eval(a);
                return Ok(true)
            },

            Push(a) => {
                let addr = self.reg[GpRegister::SP].saturating_sub(size_of::<Int>() as Int);
                self.memory.store(self.reg[DD], addr, self.eval(a))?;
                self.reg[GpRegister::SP] = addr;
            },

            Pop(a) => {
                self.reg[a] = self.memory.load(self.reg[DD], self.reg[GpRegister::SP])?;
                self.reg[GpRegister::SP] = self.reg[GpRegister::SP].saturating_add(size_of::<Int>() as Int);
            },

            Cond(a, c, b) => {
                if !c.test(self.reg[a], self.eval(b)) {
                    self.reg.pc += 2 * size_of::<Instruction>() as Int;
                }
            }

            Emit(a) => {
                let n = self.eval(a) % 256;
                print!("{}", n as u8 as char)
            },
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