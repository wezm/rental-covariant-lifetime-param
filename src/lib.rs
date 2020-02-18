pub mod error;
pub mod read;

use read::{ReadArray, ReadScope, U32Be};

#[macro_use]
extern crate rental;

pub struct TestTable<'a> {
    bitmap_sizes: ReadArray<'a, U32Be>,
}

pub struct WorkingTable<'a> {
    data: ReadScope<'a>,
}

rental! {
    mod tables {
        use super::*;

        // This one causes an error
        #[rental(covariant)]
        pub struct CBLC {
            data: Box<[u8]>,
            table: TestTable<'data>
        }

        // This one works
        #[rental(covariant)]
        pub struct Working {
            data: Box<[u8]>,
            table: WorkingTable<'data>
        }
    }
}
