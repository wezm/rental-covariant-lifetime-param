#![allow(missing_docs)]

use crate::error::ParseError;

#[derive(Copy, Clone)]
pub enum U32Be {}

mod size {
    use std::mem;

    pub const U32: usize = mem::size_of::<u32>();
}

#[derive(Debug, Copy, Clone)]
pub struct ReadEof {}

#[derive(Clone, Debug, PartialEq)]
pub struct ReadScope<'a> {
    base: usize,
    data: &'a [u8],
}

#[derive(Clone)]
pub struct ReadCtxt<'a> {
    scope: ReadScope<'a>,
    offset: usize,
}

#[derive(Clone)]
pub struct ReadArray<'a, T: ReadFixedSizeDep<'a>> {
    // scope: ReadScope<'a>,
    length: usize,
    args: T::Args,
}

pub trait ReadBinary<'a> {
    type HostType: Sized; // default = Self

    fn read(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType, ParseError>;
}

pub trait ReadBinaryDep<'a> {
    type Args: Clone;
    type HostType: Sized; // default = Self

    fn read_dep(ctxt: &mut ReadCtxt<'a>, args: Self::Args) -> Result<Self::HostType, ParseError>;
}

pub trait ReadFixedSizeDep<'a>: ReadBinaryDep<'a> {
    /// The number of bytes consumed by `ReadBinaryDep::read`.
    fn size(args: Self::Args) -> usize;
}

/// Read will always succeed if sufficient bytes are available.
pub trait ReadUnchecked<'a> {
    type HostType: Sized; // default = Self

    /// The number of bytes consumed by `read_unchecked`.
    const SIZE: usize;

    /// Must read exactly `SIZE` bytes.
    /// Unsafe as it avoids prohibitively expensive per-byte bounds checking.
    unsafe fn read_unchecked(ctxt: &mut ReadCtxt<'a>) -> Self::HostType;
}

pub trait ReadFrom<'a> {
    type ReadType: ReadUnchecked<'a>;
    fn from(value: <Self::ReadType as ReadUnchecked<'a>>::HostType) -> Self;
}

impl<'a, T> ReadUnchecked<'a> for T
where
    T: ReadFrom<'a>,
{
    type HostType = T;

    const SIZE: usize = T::ReadType::SIZE;

    unsafe fn read_unchecked(ctxt: &mut ReadCtxt<'a>) -> Self::HostType {
        let t = T::ReadType::read_unchecked(ctxt);
        T::from(t)
    }
}

impl<'a, T> ReadBinary<'a> for T
where
    T: ReadUnchecked<'a>,
{
    type HostType = T::HostType;

    fn read(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType, ParseError> {
        ctxt.check_avail(T::SIZE)?;
        Ok(unsafe { T::read_unchecked(ctxt) })
        // Safe because we have `SIZE` bytes available.
    }
}

impl<'a, T> ReadBinaryDep<'a> for T
where
    T: ReadBinary<'a>,
{
    type Args = ();
    type HostType = T::HostType;

    fn read_dep(ctxt: &mut ReadCtxt<'a>, (): Self::Args) -> Result<Self::HostType, ParseError> {
        T::read(ctxt)
    }
}

impl<'a, T> ReadFixedSizeDep<'a> for T
where
    T: ReadUnchecked<'a>,
{
    fn size((): ()) -> usize {
        T::SIZE
    }
}

impl<'a> ReadCtxt<'a> {
    fn check_avail(&self, length: usize) -> Result<(), ReadEof> {
        match self.offset.checked_add(length) {
            Some(endpos) if endpos <= self.scope.data.len() => Ok(()),
            _ => Err(ReadEof {}),
        }
    }

    unsafe fn read_unchecked_u32be(&mut self) -> u32 {
        let b0 = u32::from(*self.scope.data.get_unchecked(self.offset));
        let b1 = u32::from(*self.scope.data.get_unchecked(self.offset + 1));
        let b2 = u32::from(*self.scope.data.get_unchecked(self.offset + 2));
        let b3 = u32::from(*self.scope.data.get_unchecked(self.offset + 3));
        self.offset += 4;
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    }

}

impl<'a> ReadUnchecked<'a> for U32Be {
    type HostType = u32;

    const SIZE: usize = size::U32;

    unsafe fn read_unchecked(ctxt: &mut ReadCtxt<'a>) -> u32 {
        ctxt.read_unchecked_u32be()
    }
}
