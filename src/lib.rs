use std::{any::TypeId, cell::{Cell, RefCell}, collections::HashMap, mem::{self, transmute, ManuallyDrop}, ptr};

use anyhow::{anyhow, ensure, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use vivibin::{CanRead, CanWrite, EndianSpecific, Endianness, HeapToken, ReadDomain, Reader, WriteCtx, WriteDomain, Writer};

use crate::{elf::{Relocation, Symbol}, util::{pointer::Pointer, read_string}};

pub mod elf;
pub mod formats;
pub mod util;

#[derive(Clone, Copy)]
pub struct ElfReadDomain<'a> {
    rodata_section: &'a [u8],
    relocations: &'a IndexMap<Pointer, Relocation>,
    symbols: &'a IndexMap<String, Symbol>,
}

impl<'a> ElfReadDomain<'a> {
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

impl EndianSpecific for ElfReadDomain<'_> {
    fn endianness(self) -> Endianness {
        Endianness::Big
    }
}

// this should be a macro :/
impl ReadDomain for ElfReadDomain<'_> {
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

impl CanRead<Pointer> for ElfReadDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<Pointer> {
        self.read_pointer(reader)
    }
}

impl CanRead<String> for ElfReadDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<String> {
        self.read_string(reader)
    }
}

#[derive(Clone, Copy)]
pub struct ElfWriteDomain<'a> {
    string_map: &'a RefCell<HashMap<String, HeapToken>>,
    prev_string_len: &'a Cell<usize>,
}

impl EndianSpecific for ElfWriteDomain<'_> {
    fn endianness(self) -> Endianness {
        Endianness::Big
    }
}

impl<'a> ElfWriteDomain<'a> {
    pub fn new(string_map: &'a RefCell<HashMap<String, HeapToken>>, prev_string_len: &'a Cell<usize>) -> Self {
        Self {
            string_map,
            prev_string_len,
        }
    }
    
    pub fn write_string(&self, ctx: &mut impl WriteCtx, value: &str) -> Result<()> {
        // Search for if this string has already been written before
        // TODO: account for substrings (use crate memchr?)
        let existing_token = if ctx.position()? < 0xc32c { 
            self.string_map.borrow().get(value).copied()
        } else {
            None
        };
        
        let token = if let Some(token) = existing_token {
            token
        } else {
            let alignment = if self.prev_string_len.get() <= 2 && value.len() <= 1
                { 0 } else { 4 };
            self.prev_string_len.set(value.len());
            let new_token = ctx.allocate_next_block_aligned(alignment, move |ctx| {
                ctx.write_c_str(value)?;
                Ok(())
            })?;
            self.string_map.borrow_mut().insert(value.to_string(), new_token);
            new_token
        };
        
        ctx.write_token::<4>(token)?;
        Ok(())
    }
    
    pub fn write_pointer_debug(&self, writer: &mut impl Writer, value: Pointer) -> Result<()> {
        writer.write_u32::<BigEndian>(value.0 | 0x70000000)?;
        Ok(())
    }
}

impl WriteDomain for ElfWriteDomain<'_> {
    type Pointer = Pointer;
    type Cat = ();

    fn write_unk<T: 'static>(self, ctx: &mut impl WriteCtx, value: &T) -> Result<Option<()>> {
        let type_id = TypeId::of::<T>();
        
        if type_id == TypeId::of::<String>() {
            let value = unsafe { transmute::<&T, &String>(value) };
            self.write_string(ctx, value)?;
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }

    fn apply_reference(self, writer: &mut impl Writer, heap_offset: usize) -> Result<()> {
        self.write_pointer_debug(writer, Pointer(heap_offset as u32))
    }
}

impl CanWrite<String> for ElfWriteDomain<'_> {
    fn write(self, ctx: &mut impl WriteCtx, value: &String) -> Result<()> {
        self.write_string(ctx, value)
    }
}
