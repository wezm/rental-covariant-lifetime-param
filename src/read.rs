#![allow(missing_docs)]

use crate::error::ParseError;
use std::marker::PhantomData;

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
pub struct ReadArray<'a, T> {
    scope: ReadScope<'a>,
    length: usize,
    // args: T::Args,
    _item: PhantomData<T>,
}

pub struct ReadArrayIter<'a, T: ReadUnchecked<'a>> {
    ctxt: ReadCtxt<'a>,
    length: usize,
    phantom: PhantomData<T>,
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

    pub fn read_array<T: ReadUnchecked<'a>>(
        &mut self,
        length: usize,
    ) -> Result<ReadArray<'a, T>, ParseError> {
        let scope = self.read_scope(length * T::SIZE)?;
        let args = ();
        Ok(ReadArray {
            scope,
            length,
            _item: PhantomData,
        })
    }

    pub fn read_scope(&mut self, length: usize) -> Result<ReadScope<'a>, ReadEof> {
        if let Ok(scope) = self.scope.offset_length(self.offset, length) {
            self.offset += length;
            Ok(scope)
        } else {
            Err(ReadEof {})
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

impl<'a, T: ReadFixedSizeDep<'a>> ReadArray<'a, T> {
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn iter(&self) -> ReadArrayIter<'a, T>
    where
        T: ReadUnchecked<'a>,
    {
        ReadArrayIter {
            ctxt: self.scope.ctxt(),
            length: self.length,
            phantom: PhantomData,
        }
    }
}

impl<'a, T: ReadUnchecked<'a>> Iterator for ReadArrayIter<'a, T> {
    type Item = T::HostType;

    fn next(&mut self) -> Option<T::HostType> {
        if self.length > 0 {
            self.length -= 1;
            Some(unsafe { T::read_unchecked(&mut self.ctxt) })
        // Safe because we have (at least) `SIZE` bytes available.
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }
}

impl<'a> ReadScope<'a> {
    pub fn new(data: &'a [u8]) -> ReadScope<'a> {
        let base = 0;
        ReadScope { base, data }
    }

    pub fn offset_length(&self, offset: usize, length: usize) -> Result<ReadScope<'a>, ParseError> {
        if offset < self.data.len() || length == 0 {
            let data = &self.data[offset..];
            if length <= data.len() {
                let base = self.base + offset;
                let data = &data[0..length];
                Ok(ReadScope { base, data })
            } else {
                Err(ParseError::BadEof)
            }
        } else {
            Err(ParseError::BadOffset)
        }
    }

    pub fn ctxt(&self) -> ReadCtxt<'a> {
        ReadCtxt::new(self.clone())
    }
}

impl<'a> ReadCtxt<'a> {
    /// ReadCtxt is constructed by calling `ReadScope::ctxt`.
    fn new(scope: ReadScope<'a>) -> ReadCtxt<'a> {
        ReadCtxt { scope, offset: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_array_iter() {
        let mut data = Vec::new();
        for i in 0..100 {
            data.push(0u8);
            data.push(0u8);
            data.push(0u8);
            data.push(i);
        }

        let mut ctxt = ReadScope::new(&data).ctxt();
        let array = ctxt.read_array::<U32Be>(10).unwrap();
        for x in array.iter() {
            println!("{}", x);
        }
    }
}
