use std::ops::Range;

use crate::bytecode::Int;

struct Permissions {
    read: bool,
    write: bool,
    exec: bool
}

enum Seal {
    Sealed(u8),
    Unsealed
}

struct Inner {
    ptr: Int,
    meta: Int,
}

struct Capability {
    inner: Inner,
    valid: bool,
}