#[macro_use]
extern crate rental;

use std::error::Error;
use std::marker::PhantomData;

type ParseError = Box<dyn Error>;

#[derive(Clone, Debug, PartialEq)]
pub struct ReadScope<'a> {
    base: usize,
    data: &'a [u8],
}

pub struct ReadScopeOwned {
    base: usize,
    data: Box<[u8]>,
}

impl ReadScopeOwned {
    pub fn new<'a>(scope: ReadScope<'a>) -> ReadScopeOwned {
        ReadScopeOwned {
            base: scope.base,
            data: Box::from(scope.data),
        }
    }

    pub fn new_with_data<'a>(data: Box::<[u8]>) -> ReadScopeOwned {
        ReadScopeOwned {
            base: 0,
            data,
        }
    }

    pub fn scope<'a>(&'a self) -> ReadScope<'a> {
        ReadScope {
            base: self.base,
            data: &self.data,
        }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Clone)]
pub struct ReadCtxt<'a> {
    scope: ReadScope<'a>,
    offset: usize,
}

pub trait ReadBinary<'a> {
    type HostType: Sized; // default = Self

    fn read(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType, ParseError>;
}

pub trait ReadFixedSize<'a>: ReadBinary<'a> {
    /// The number of bytes consumed by `ReadBinaryDep::read`.
    fn size() -> usize;
}

#[derive(Clone)]
pub struct ReadArray<'a, T: ReadFixedSize<'a>> {
    scope: ReadScope<'a>,
    length: usize,
    _item: PhantomData<T>
}

impl<'a> ReadScope<'a> {
    pub fn offset_length(&self, offset: usize, length: usize) -> Result<ReadScope<'a>, ParseError> {
        if offset < self.data.len() || length == 0 {
            let data = &self.data[offset..];
            if length <= data.len() {
                let base = self.base + offset;
                let data = &data[0..length];
                Ok(ReadScope { base, data })
            } else {
                Err("ParseError::BadEof".into())
            }
        } else {
            Err("ParseError::BadOffset".into())
        }
    }
}

impl<'a> ReadCtxt<'a> {
    fn read_u32be(&mut self) -> Result<u32, ParseError> {
        let b0 = u32::from(self.scope.data[self.offset]);
        let b1 = u32::from(self.scope.data[self.offset + 1]);
        let b2 = u32::from(self.scope.data[self.offset + 2]);
        let b3 = u32::from(self.scope.data[self.offset + 3]);
        self.offset += 4;
        Ok((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
    }

    pub fn read_array<T: ReadFixedSize<'a>>(
        &mut self,
        length: usize,
    ) -> Result<ReadArray<'a, T>, ParseError> {
        let scope = self.read_scope(length * T::size())?;
        Ok(ReadArray {
            scope,
            length,
            _item: PhantomData
        })
    }

    pub fn read_scope(&mut self, length: usize) -> Result<ReadScope<'a>, ParseError> {
        if let Ok(scope) = self.scope.offset_length(self.offset, length) {
            self.offset += length;
            Ok(scope)
        } else {
            Err("EOF".into())
        }
    }
}

pub enum U32Be {}

pub struct Parsed<'a> {
    pub items: ReadArray<'a, U32Be>
}

impl<'a> ReadBinary<'a> for U32Be {
    type HostType = u32;

    fn read(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType, ParseError> {
        ctxt.read_u32be()
    }
}

impl<'a> ReadFixedSize<'a> for U32Be {
    fn size() -> usize {
        4
    }
}

impl<'a> ReadBinary<'a> for Parsed<'a> {
    type HostType = Self;

    fn read(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType, ParseError> {
        let items = ctxt.read_array(4)?;
        Ok(Parsed {
            items
        })
    }
}


rental! {
    mod demo {
        use super::*;

        // #[rental]
        #[rental(covariant)]
        pub struct ParsedWithData {
            data: Box<[u8]>,
            parsed: Parsed<'data>
        }
    }
}
