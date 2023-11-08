use std::{ops::Range, num::NonZeroU8, fmt::{Display, Debug}};

use crate::bytecode::Int;

pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub exec: bool
}

impl Permissions {
    const NULL: Permissions = Self::rwx(false, false, false);

    pub const fn rwx(read: bool, write: bool, exec: bool) -> Self {
        Self { read, write, exec }
    }
}

impl From<Permissions> for u8 {
    fn from(value: Permissions) -> Self {
        value.read as u8
        | (value.write as u8) << 1
        | (value.exec as u8) << 2
    }
}

impl From<u8> for Permissions {
    fn from(value: u8) -> Self {
        Self {
            read: value & 1 != 0,
            write: value & 2 != 0,
            exec: value & 4 != 0
        }
    }
}

impl Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = if self.read { 'r' } else { '-' };
        let w = if self.write { 'w' } else { '-' };
        let x = if self.exec { 'x' } else { '-' };
        write!(f, "{r}{w}{x}")
    }
}

#[derive(Debug)]
pub enum Seal {
    Sealed(NonZeroU8),
    Unsealed
}

impl From<Seal> for u8 {
    fn from(value: Seal) -> Self {
        match value {
            Seal::Sealed(n) => n.into(),
            Seal::Unsealed => 0,
        }
    }
}

impl From<u8> for Seal {
    fn from(value: u8) -> Self {
        match NonZeroU8::new(value) {
            Some(n) => Self::Sealed(n),
            None => Self::Unsealed,
        }
    }
}

pub const CAP_SIZE: usize = 16;

#[derive(Clone, Copy)]
#[repr(align(4))]
pub struct Inner {
    ptr: Int,
    start: Int,
    end: Int,
    meta: Int,
}

impl Inner {
    pub fn new(ptr: Int, bounds: Range<Int>, perms: Permissions, seal: Seal) -> Self {
        let p = u8::from(perms) as u16;
        let s = u8::from(seal) as u16;
        let meta = p | (s << 8);
        Self {
            ptr,
            start: bounds.start,
            end: bounds.end,
            meta: meta as Int
        }
    }

    pub fn perms(&self) -> Permissions {
        Permissions::from(self.meta as u8)
    }

    pub fn seal(&self) -> Seal {
        Seal::from((self.meta >> 8) as u8)
    }

    pub fn bounds(&self) -> Range<Int> {
        self.start..self.end
    }

    pub fn ptr(&self) -> Int {
        self.ptr
    }

    pub fn in_range(&self) -> bool {
        self.bounds().contains(&self.ptr())
    }
}

impl Default for Inner {
    fn default() -> Self {
        Self::new(
            0,
            0..0,
            Permissions::NULL,
            Seal::Unsealed
        )
    }
}

#[derive(Default, Clone, Copy)]
pub struct Capability {
    pub inner: Inner,
    pub valid: bool,
}

impl Debug for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = if self.valid { '*' } else { ' ' };
        write!(f, "[{c}] {:04x} in {:04x}-{:04x} {} {:04x?}", self.inner.ptr, self.inner.start, self.inner.end, self.inner.perms(), self.inner.seal())
    }
}