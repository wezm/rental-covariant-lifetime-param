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
pub struct ReadArray<'a, T: ReadFixedSizeDep> {
    scope: ReadScope<'a>,
    length: usize,
    args: T::Args,
}

pub trait ReadBinary {
    type HostType: Sized; // default = Self

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType, ParseError>;
}

pub trait ReadBinaryDep {
    type Args: Clone;
    type HostType: Sized; // default = Self

    fn read_dep<'a>(ctxt: &mut ReadCtxt<'a>, args: Self::Args) -> Result<Self::HostType, ParseError>;
}

pub trait ReadFixedSizeDep: ReadBinaryDep {
    /// The number of bytes consumed by `ReadBinaryDep::read`.
    fn size(args: Self::Args) -> usize;
}

/// Read will always succeed if sufficient bytes are available.
pub trait ReadUnchecked {
    type HostType: Sized; // default = Self

    /// The number of bytes consumed by `read_unchecked`.
    const SIZE: usize;

    /// Must read exactly `SIZE` bytes.
    /// Unsafe as it avoids prohibitively expensive per-byte bounds checking.
    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> Self::HostType;
}

pub trait ReadFrom {
    type ReadType: ReadUnchecked;
    fn from(value: <Self::ReadType as ReadUnchecked>::HostType) -> Self;
}

impl<T> ReadUnchecked for T
where
    T: ReadFrom,
{
    type HostType = T;

    const SIZE: usize = T::ReadType::SIZE;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> Self::HostType {
        let t = T::ReadType::read_unchecked(ctxt);
        T::from(t)
    }
}

impl<T> ReadBinary for T
where
    T: ReadUnchecked,
{
    type HostType = T::HostType;

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType, ParseError> {
        ctxt.check_avail(T::SIZE)?;
        Ok(unsafe { T::read_unchecked(ctxt) })
        // Safe because we have `SIZE` bytes available.
    }
}

impl<T> ReadBinaryDep for T
where
    T: ReadBinary,
{
    type Args = ();
    type HostType = T::HostType;

    fn read_dep<'a>(ctxt: &mut ReadCtxt<'a>, (): Self::Args) -> Result<Self::HostType, ParseError> {
        T::read(ctxt)
    }
}

impl<T> ReadFixedSizeDep for T
where
    T: ReadUnchecked,
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

impl ReadUnchecked for U32Be {
    type HostType = u32;

    const SIZE: usize = size::U32;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> u32 {
        ctxt.read_unchecked_u32be()
    }
}
