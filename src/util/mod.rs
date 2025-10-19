use core::ffi::CStr;
use std::{io::{Read, Seek, SeekFrom}};

use anyhow::Result;

pub mod pointer;

// scoped reader pos
pub struct ReaderGuard<'a, R: Read + Seek> {
    pub reader: &'a mut R,
    start_pos: u64,
}

impl<'a, R: Read + Seek> ReaderGuard<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        let start_pos = reader.stream_position().unwrap();
        
        Self {
            reader,
            start_pos,
        }
    }
}

impl<'a, R: Read + Seek> Drop for ReaderGuard<'a, R> {
    fn drop(&mut self) {
        self.reader.seek(SeekFrom::Start(self.start_pos)).unwrap();
    }
}

#[macro_export]
macro_rules! scoped_reader_pos {
    ($reader:ident) => {
        let guard = $crate::util::ReaderGuard::new($reader);
        let $reader = &mut *guard.reader;
    };
}

// string utils
pub fn read_string(buffer: &[u8], index: u32) -> Result<&str> {
    let bytes = &buffer[index as usize..];
    let result = CStr::from_bytes_until_nul(bytes)?.to_str()?;
    Ok(result)
}
