use std::io::{Cursor, Seek, SeekFrom};

use anyhow::{anyhow, ensure, Result};
use byteorder::{BigEndian, ReadBytesExt};
use indexmap::IndexMap;

use crate::{elf::{Relocation, Symbol}, util::{pointer::Pointer, read_string}};

pub mod elf;
pub mod formats;
pub mod util;

pub struct ReadContext<'a> {
    pub reader: Cursor<&'a [u8]>,
    
    rodata_section: &'a [u8],
    relocations: &'a IndexMap<Pointer, Relocation>,
    symbols: &'a IndexMap<String, Symbol>,
}

impl<'a> ReadContext<'a> {
    pub fn new(
        reader: Cursor<&'a [u8]>,
        rodata_section: &'a [u8],
        relocations: &'a IndexMap<Pointer, Relocation>,
        symbols: &'a IndexMap<String, Symbol>,
    ) -> Self {
        Self {
            reader,
            rodata_section,
            relocations,
            symbols,
        }
    }
    
    pub fn find_symbol(&self, name: &str) -> Result<Symbol> {
        let result = self.symbols.get(name)
            .ok_or_else(|| anyhow!("Could not find symbol {name:?}"))?;
        
        Ok(result.clone())
    }
    
    pub fn read_string(&mut self) -> Result<String> {
        let pointer = self.read_pointer()?;
        let result = read_string(self.rodata_section, pointer.0)?;
        Ok(result.to_string())
    }
    
    pub fn read_pointer(&mut self) -> Result<Pointer> {
        let offset = Pointer::current(&mut self.reader)?;
        
        let real_value = self.reader.read_u32::<BigEndian>()?;
        ensure!(real_value == 0, "Expected pointer, got 0x{real_value:x} (at offset 0x{:x})", offset.0);
        
        let relocation = self.relocations.get(&offset)
            .ok_or_else(|| anyhow!("Expected pointer, got nothing (at offset 0x{:x}", offset.0))?;
        
        let symbol = self.symbols.get_index((relocation.info >> 8) as usize)
            .ok_or_else(|| anyhow!("Could not find symbol at index {}", relocation.info >> 8))?
            .1;
        
        return Ok(symbol.offset().into());
    }
}

// scoped context pos
pub struct CtxGuard<'a, 'b> {
    pub ctx: &'a mut ReadContext<'b>,
    start_pos: u64,
}

impl<'a, 'b> CtxGuard<'a, 'b> {
    pub fn new(ctx: &'a mut ReadContext<'b>) -> Self {
        let start_pos = ctx.reader.stream_position().unwrap();
        
        Self {
            ctx,
            start_pos,
        }
    }
}

impl<'a, 'b> Drop for CtxGuard<'a, 'b> {
    fn drop(&mut self) {
        self.ctx.reader.seek(SeekFrom::Start(self.start_pos)).unwrap();
    }
}

#[macro_export]
macro_rules! scoped_ctx_pos {
    ($ctx:ident) => {
        let guard = $crate::CtxGuard::new($ctx);
        let $ctx = &mut *guard.ctx;
    };
}

