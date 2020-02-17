pub mod error;
pub mod read;

use read::ReadArray;
use read::U32Be;

#[macro_use]
extern crate rental;

pub struct TestTable<'a> {
    bitmap_sizes: ReadArray<'a, U32Be>,
}

rental! {
    mod tables {
        use super::*;

        #[rental(covariant)]
        pub struct CBLC {
            data: Box<[u8]>,
            table: TestTable<'data>
        }
    }
}
