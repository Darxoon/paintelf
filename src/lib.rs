use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Display,
    mem::{self, ManuallyDrop, transmute},
    ptr,
};

use anyhow::{Result, anyhow, ensure};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use vivibin::{
    CanRead, CanWrite, CanWriteWithArgs, EndianSpecific, Endianness, HeapToken, ReadDomain, Reader, WriteCtx, WriteDomain, Writer
};

use crate::{
    elf::{Relocation, Symbol},
    util::{pointer::Pointer, read_string},
};

pub mod elf;
pub mod elf_container;
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

#[derive(Clone, Debug)]
pub enum SymbolName {
    None,
    Internal(char),
    InternalNamed(String),
    InternalUnmangled(String),
    Unmangled(String),
}

impl SymbolName {
    pub fn is_internal(&self) -> bool {
        match self {
            SymbolName::Internal(_) => true,
            SymbolName::InternalNamed(_) => true,
            SymbolName::InternalUnmangled(_) => true,
            _ => false,
        }
    }
    
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SymbolName::None => None,
            SymbolName::Internal(_) => None,
            SymbolName::InternalNamed(name)
            | SymbolName::InternalUnmangled(name)
            | SymbolName::Unmangled(name) => Some(name),
        }
    }
}

impl Display for SymbolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolName::None => write!(f, "<none>"),
            SymbolName::Internal(initial_char) => write!(f, "{initial_char}<???>"),
            SymbolName::InternalNamed(name)
            | SymbolName::InternalUnmangled(name)
            | SymbolName::Unmangled(name) => write!(f, "{name}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SymbolDeclaration {
    pub name: SymbolName,
    pub offset: HeapToken,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct RelDeclaration {
    pub base_location: HeapToken,
    pub target_location: HeapToken,
}

#[derive(Debug, Clone)]
pub struct WriteStringArgs {
    pub deduplicate: bool,
}

impl Default for WriteStringArgs {
    fn default() -> Self {
        Self { deduplicate: true }
    }
}

#[derive(Clone, Copy)]
pub struct ElfWriteDomain<'a> {
    string_map: &'a RefCell<HashMap<String, HeapToken>>,
    symbol_declarations: &'a RefCell<Vec<SymbolDeclaration>>,
    relocations: &'a RefCell<Vec<RelDeclaration>>,
    prev_string_len: &'a Cell<usize>,
}

impl EndianSpecific for ElfWriteDomain<'_> {
    fn endianness(self) -> Endianness {
        Endianness::Big
    }
}

impl<'a> ElfWriteDomain<'a> {
    pub fn new(
        string_map: &'a RefCell<HashMap<String, HeapToken>>,
        symbol_declarations: &'a RefCell<Vec<SymbolDeclaration>>,
        relocations: &'a RefCell<Vec<RelDeclaration>>,
        prev_string_len: &'a Cell<usize>,
    ) -> Self {
        Self {
            string_map,
            symbol_declarations,
            relocations,
            prev_string_len,
        }
    }
    
    pub fn write_string(self, ctx: &mut impl WriteCtx, value: &str, args: WriteStringArgs) -> Result<()> {
        // Search for if this string has already been written before
        // TODO: account for substrings (use crate memchr?)
        let existing_token = if args.deduplicate && ctx.position()? < 0xc32c { 
            self.string_map.borrow().get(value).copied()
        } else {
            None
        };
        
        let token = if let Some(token) = existing_token {
            token
        } else {
            let alignment = if self.prev_string_len.get() <= 2 && value.len() <= 1 {
                0
            } else {
                4
            };
            
            // TODO: this is a hack
            if args.deduplicate {
                self.prev_string_len.set(value.len());
            }
            
            let mut name_size: usize = 0;
            let new_token = ctx.allocate_next_block_aligned(alignment, |ctx| {
                let start_pos = ctx.position()? as usize;
                ctx.write_c_str(value)?;
                if value.len() > 2 {
                    ctx.align_to(4)?;
                }
                name_size = ctx.position()? as usize - start_pos;
                Ok(())
            })?;
            
            self.put_symbol(SymbolDeclaration {
                name: SymbolName::Internal('.'),
                offset: new_token,
                size: name_size as u32,
            });
            
            if args.deduplicate {
                self.string_map
                    .borrow_mut()
                    .insert(value.to_string(), new_token);
            }
            
            new_token
        };
        
        // TODO: put Relocation into apply_reference instead
        let current_pos = ctx.heap_token_at_current_pos()?;
        ctx.write_token::<4>(token)?;
        self.put_relocation(RelDeclaration {
            base_location: current_pos,
            target_location: token,
        });
        Ok(())
    }
    
    pub fn put_symbol(self, symbol: SymbolDeclaration) {
        self.symbol_declarations.borrow_mut().push(symbol);
    }
    
    pub fn put_relocation(self, relocation: RelDeclaration) {
        self.relocations.borrow_mut().push(relocation);
    }
    
    pub fn write_pointer_debug(self, writer: &mut impl Writer, value: Pointer) -> Result<()> {
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
            self.write_string(ctx, value, WriteStringArgs::default())?;
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }

    fn apply_reference(self, writer: &mut impl Writer, heap_offset: usize) -> Result<()> {
        // TODO: reenable this for debug purposes
        self.write_pointer_debug(writer, Pointer(heap_offset as u32))?;
        Ok(())
    }
}

impl CanWrite<String> for ElfWriteDomain<'_> {
    fn write(self, ctx: &mut impl WriteCtx, value: &String) -> Result<()> {
        self.write_string(ctx, value, WriteStringArgs::default())
    }
}

impl CanWriteWithArgs<String, WriteStringArgs> for ElfWriteDomain<'_> {
    fn write_args(self, ctx: &mut impl WriteCtx, value: &String, args: WriteStringArgs) -> Result<()> {
        self.write_string(ctx, value, args)
    }
}

// TODO: this sucks
impl CanWrite<SymbolDeclaration> for ElfWriteDomain<'_> {
    fn write(self, _: &mut impl WriteCtx, value: &SymbolDeclaration) -> Result<()> {
        self.put_symbol(value.clone());
        Ok(())
    }
}

impl CanWrite<RelDeclaration> for ElfWriteDomain<'_> {
    fn write(self, _: &mut impl WriteCtx, value: &RelDeclaration) -> Result<()> {
        self.put_relocation(value.clone());
        Ok(())
    }
}
