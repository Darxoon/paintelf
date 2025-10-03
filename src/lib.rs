use std::{any::TypeId, mem::{self, ManuallyDrop}, ptr};

use anyhow::{anyhow, ensure, Result};
use byteorder::{BigEndian, ReadBytesExt};
use indexmap::IndexMap;
use vivibin::{CanRead, EndianSpecific, Endianness, ReadDomain, Reader};

use crate::{elf::{Relocation, Symbol}, util::{pointer::Pointer, read_string}};

pub mod elf;
pub mod formats;
pub mod util;

#[derive(Clone, Copy)]
pub struct ElfDomain<'a> {
    rodata_section: &'a [u8],
    relocations: &'a IndexMap<Pointer, Relocation>,
    symbols: &'a IndexMap<String, Symbol>,
}

impl<'a> ElfDomain<'a> {
    pub fn new(
        rodata_section: &'a [u8],
        relocations: &'a IndexMap<Pointer, Relocation>,
        symbols: &'a IndexMap<String, Symbol>,
    ) -> Self {
        Self {
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
    
    pub fn read_string(&self, reader: &mut impl Reader) -> Result<String> {
        let pointer = self.read_pointer(reader)?;
        let result = read_string(self.rodata_section, pointer.0)?;
        Ok(result.to_string())
    }
    
    pub fn read_pointer(&self, reader: &mut impl Reader) -> Result<Pointer> {
        let offset = Pointer::current(reader)?;
        
        let real_value = reader.read_u32::<BigEndian>()?;
        ensure!(real_value == 0, "Expected pointer, got 0x{real_value:x} (at offset 0x{:x})", offset.0);
        
        let relocation = self.relocations.get(&offset)
            .ok_or_else(|| anyhow!("Expected pointer, got nothing (at offset 0x{:x}", offset.0))?;
        
        let symbol = self.symbols.get_index((relocation.info >> 8) as usize)
            .ok_or_else(|| anyhow!("Could not find symbol at index {}", relocation.info >> 8))?
            .1;
        
        return Ok(symbol.offset().into());
    }
}

impl EndianSpecific for ElfDomain<'_> {
    fn endianness(self) -> Endianness {
        Endianness::Big
    }
}

// this should be a macro :/
impl ReadDomain for ElfDomain<'_> {
    type Pointer = Pointer;

    fn read_unk<T: 'static>(self, reader: &mut impl vivibin::Reader) -> Result<Option<T>> {
        let type_id = TypeId::of::<T>();
        
        let result: Option<T> = if type_id == TypeId::of::<Pointer>() {
            let value = ManuallyDrop::new(self.read_pointer(reader)?);
            
            Some(unsafe { ptr::read(mem::transmute::<&Pointer, &T>(&value)) })
        } else if type_id == TypeId::of::<String>() {
            let value = ManuallyDrop::new(self.read_string(reader)?);
            
            Some(unsafe { ptr::read(mem::transmute::<&String, &T>(&value)) })
        } else {
            None
        };
        
        Ok(result)
    }

    // at some point vivibin will properly support these :P
    fn read_unk_std_vec<T, R: vivibin::Reader>(self, _reader: &mut R, _read_content: impl Fn(&mut R) -> Result<T>) -> Result<Option<Vec<T>>> {
        Ok(None)
    }

    fn read_unk_std_box<T, R: vivibin::Reader>(self, _reader: &mut R, _read_content: impl Fn(&mut R) -> Result<T>) -> Result<Option<Box<T>>> {
        Ok(None)
    }

    fn read_box<T, R: vivibin::Reader>(self, _reader: &mut R, _parser: impl FnOnce(&mut R, Self) -> Result<T>) -> Result<Option<T>> {
        Ok(None)
    }
}

impl CanRead<Pointer> for ElfDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<Pointer> {
        self.read_pointer(reader)
    }
}

impl CanRead<String> for ElfDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<String> {
        self.read_string(reader)
    }
}

