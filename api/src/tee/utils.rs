
#[inline]
pub const fn bit32(nr: u32) -> u32 {
    1u32 << nr
}

#[inline]
pub const fn bit64(nr: u32) -> u64 {
    1u64 << nr
}

#[inline]
pub const fn bit(nr: u32) -> u32 {
    bit32(nr)
}