use std::{fmt::Debug, io::SeekFrom, marker::PhantomData};

use anyhow::{Result, anyhow, bail, ensure};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use vivibin::{
    CanRead, CanReadVec, CanWrite, CanWriteBox, CanWriteSlice, CanWriteSliceWithArgs,
    CanWriteWithArgs, EndianSpecific, Endianness, HeapCategory, HeapToken, ReadDomain, Readable,
    Reader, Writable, WriteCtx, WriteDomain, Writer, util::HashMap,
};

use crate::{
    RelDeclaration, SymbolDeclaration, SymbolName,
    elf::{Relocation, Symbol},
    scoped_reader_pos,
    util::{pointer::Pointer, read_string},
};

// deserializing
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
    
    // TODO: find a way to do this with less repetition
    pub fn read_string(&self, reader: &mut impl Reader) -> Result<String> {
        let offset = Pointer::current(reader)?;
        let pointer = self.read_pointer_optional(reader)?;
        let Some(pointer) = pointer else {
            // TODO: improve debug info
            bail!("Expected non-nullable string, got null (at offset 0x{:x})", offset.0);
        };
        
        let result = read_string(self.rodata_section, pointer.0)?;
        Ok(result.to_string())
    }
    
    pub fn read_string_optional(&self, reader: &mut impl Reader) -> Result<Option<String>> {
        let pointer = self.read_pointer_optional(reader)?;
        
        if let Some(pointer) = pointer {
            let result = read_string(self.rodata_section, pointer.0)?;
            Ok(Some(result.to_string()))
        } else {
            Ok(None)
        }
    }
    
    pub fn read_vec<T: 'static, R: Reader>(self, reader: &mut R, read_content: impl Fn(&mut R) -> Result<T>) -> Result<Vec<T>> {
        let ptr: Option<Pointer> = self.read_pointer_optional(reader)?;
        let count: u32 = u32::from_reader(reader, self)?;
        
        let Some(ptr) = ptr else {
            return Ok(Vec::new());
        };
        
        if count == 0 {
            return Ok(Vec::new());
        }
        
        scoped_reader_pos!(reader);
        reader.seek(SeekFrom::Start(ptr.into()))?;
        
        let values: Vec<T> = (0..count)
            .map(|_| read_content(reader))
            .collect::<Result<_>>()?;
        
        Ok(values)
    }
    
    pub fn read_pointer(&self, reader: &mut impl Reader) -> Result<Pointer> {
        let offset = Pointer::current(reader)?;
        let optional_pointer = self.read_pointer_optional(reader)?;
        
        let Some(pointer) = optional_pointer else {
            bail!("Expected pointer, got nothing (at offset 0x{:x})", offset.0);
        };
        
        Ok(pointer)
    }
    
    pub fn read_pointer_optional(&self, reader: &mut impl Reader) -> Result<Option<Pointer>> {
        let offset = Pointer::current(reader)?;
        
        let real_value = reader.read_u32::<BigEndian>()?;
        ensure!(real_value == 0, "Expected pointer, got 0x{real_value:x} (at offset 0x{:x})", offset.0);
        
        if let Some(relocation) = self.relocations.get(&offset) {
            let symbol = self.symbols.get_index((relocation.info >> 8) as usize)
                .ok_or_else(|| anyhow!("Could not find symbol at index {}", relocation.info >> 8))?
                .1;
            
            Ok(Some(symbol.offset().into()))
        } else {
            Ok(None)
        }
    }
}

impl EndianSpecific for ElfReadDomain<'_> {
    fn endianness(&self) -> Endianness {
        Endianness::Big
    }
}

impl ReadDomain for ElfReadDomain<'_> {
    type Pointer = Pointer;

    fn read_box_nullable<T, R: Reader>(self, reader: &mut R, read_content: impl FnOnce(&mut R) -> Result<T>) -> Result<Option<T>> {
        let Some(ptr) = self.read_pointer_optional(reader)? else {
            return Ok(None);
        };
        
        scoped_reader_pos!(reader);
        reader.set_position(ptr)?;
        
        read_content(reader).map(Some)
    }
}

impl CanReadVec for ElfReadDomain<'_> {
    fn read_std_vec_of<T: 'static, R: Reader>(self, reader: &mut R, read_content: impl Fn(&mut R) -> Result<T>) -> Result<Vec<T>> {
        self.read_vec(reader, read_content)
    }
}

impl CanRead<Pointer> for ElfReadDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<Pointer> {
        self.read_pointer(reader)
    }
}

impl CanRead<Option<Pointer>> for ElfReadDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<Option<Pointer>> {
        self.read_pointer_optional(reader)
    }
}

impl CanRead<String> for ElfReadDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<String> {
        self.read_string(reader)
    }
}

impl CanRead<Option<String>> for ElfReadDomain<'_> {
    fn read(self, reader: &mut impl Reader) -> Result<Option<String>> {
        self.read_string_optional(reader)
    }
}

// serializing
// TODO: this has the potential to cause a lot of code bloat :/
pub trait ElfCategory: HeapCategory + Copy {
    fn string() -> Self;
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitCategory;

impl HeapCategory for UnitCategory {}

impl ElfCategory for UnitCategory {
    fn string() -> Self {
        Self
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataCategory {
    #[default]
    Data,
    Rodata,
}

impl HeapCategory for DataCategory {}

impl ElfCategory for DataCategory {
    fn string() -> Self {
        Self::Rodata
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfCategoryType {
    Unit,
    Data,
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

#[derive(Debug, Clone, Default)]
pub struct WriteNullTermiantedSliceArgs {
    pub symbol_name: Option<SymbolName>,
    pub write_length: bool,
}

#[derive(Clone)]
pub struct ElfWriteDomain<C: ElfCategory> {
    pub string_map: HashMap<String, HeapToken>,
    pub symbol_declarations: Vec<SymbolDeclaration>,
    pub relocations: Vec<RelDeclaration>,
    pub string_dedup_size: u64,
    pub apply_debug_relocations: bool,
    
    prev_string_len: usize,
    
    _marker: PhantomData<C>,
}

impl<C: ElfCategory> EndianSpecific for ElfWriteDomain<C> {
    fn endianness(&self) -> Endianness {
        Endianness::Big
    }
}

impl<C: ElfCategory> ElfWriteDomain<C> {
    pub fn new(string_dedup_size: u64, apply_debug_relocations: bool) -> Self {
        Self {
            string_map: HashMap::new(),
            symbol_declarations: Vec::new(),
            relocations: Vec::new(),
            string_dedup_size,
            apply_debug_relocations,
            prev_string_len: 0,
            _marker: PhantomData,
        }
    }
    
    pub fn write_string_optional(&mut self, ctx: &mut impl WriteCtx<C>, value: Option<&str>, args: WriteStringArgs) -> Result<()> {
        let Some(value) = value else {
            0u32.to_writer(ctx, self)?;
            return Ok(());
        };
        
        self.write_string(ctx, value, args)
    }
    
    pub fn write_string(&mut self, ctx: &mut impl WriteCtx<C>, value: &str, args: WriteStringArgs) -> Result<()> {
        // Search for if this string has already been written before
        // TODO: account for substrings (use crate memchr?)
        let existing_token = if args.deduplicate && ctx.position()? < self.string_dedup_size { 
            self.string_map.get(value).copied()
        } else {
            None
        };
        
        if let Some(token) = existing_token {
            ctx.write_token::<4>(token)?;
            return Ok(());
        }
        
        let alignment = (self.prev_string_len > 2 || value.len() > 1).then_some(4).unwrap_or_default();
        
        if args.deduplicate {
            self.prev_string_len = value.len();
        }
        
        let mut name_size: usize = 0;
        let new_token = ctx.allocate_next_block_aligned(Some(C::string()), alignment, |ctx| {
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
            self.string_map.insert(value.to_string(), new_token);
        }
        
        ctx.write_token::<4>(new_token)?;
        Ok(())
    }
    
    pub fn write_box<W: WriteCtx<C>>(
        &mut self, ctx: &mut W, args: Option<SymbolName>,
        write_content: impl FnOnce(&mut Self, &mut W::InnerCtx<'_>) -> Result<()>,
    ) -> Result<()> {
        let mut links_size: usize = 0;
        let token = ctx.allocate_next_block_aligned(None, 4, |ctx| {
            let start_pos = ctx.position()? as usize;
            write_content(self, ctx)?;
            links_size = ctx.position()? as usize - start_pos;
            Ok(())
        })?;
        
        ctx.write_token::<4>(token)?;
        
        if let Some(name) = args {
            self.put_symbol(SymbolDeclaration {
                name,
                offset: token,
                size: links_size as u32,
            });
        }
        Ok(())
    }
    
    pub fn write_slice<T: 'static, W: WriteCtx<C>>(
        &mut self, ctx: &mut W, values: &[T], args: Option<SymbolName>,
        write_content: impl Fn(&mut Self, &mut W::InnerCtx<'_>, &T) -> Result<()>,
    ) -> Result<()> {
        let mut links_size: usize = 0;
        let token = ctx.allocate_next_block_aligned(None, 4, |ctx| {
            let start_pos = ctx.position()? as usize;
            for value in values {
                write_content(self, ctx, value)?;
            }
            links_size = ctx.position()? as usize - start_pos;
            Ok(())
        })?;
        
        ctx.write_token::<4>(token)?;
        (values.len() as u32).to_writer(ctx, self)?;
        
        if let Some(name) = args {
            self.put_symbol(SymbolDeclaration {
                name,
                offset: token,
                size: links_size as u32,
            });
        }
        Ok(())
    }
    
    pub fn write_null_terminated_slice<T: Default + 'static, W: WriteCtx<C>>(
        &mut self, ctx: &mut W, values: &[T], args: WriteNullTermiantedSliceArgs,
        write_content: impl Fn(&mut Self, &mut W::InnerCtx<'_>, &T) -> Result<()>,
    ) -> Result<()> {
        let mut links_size: usize = 0;
        let token = ctx.allocate_next_block_aligned(None, 4, |ctx| {
            let start_pos = ctx.position()? as usize;
            for value in values {
                write_content(self, ctx, value)?;
            }
            write_content(self, ctx, &T::default())?;
            links_size = ctx.position()? as usize - start_pos;
            Ok(())
        })?;
        
        ctx.write_token::<4>(token)?;
        if args.write_length {
            (values.len() as u32).to_writer(ctx, self)?;
        }
        
        if let Some(name) = args.symbol_name {
            self.put_symbol(SymbolDeclaration {
                name,
                offset: token,
                size: links_size as u32,
            });
        }
        Ok(())
    }
    
    pub fn write_symbol<W: WriteCtx<C>>(
        &mut self,
        ctx: &mut W,
        symbol_name: impl Into<String>,
        content_callback: impl FnOnce(&mut Self, &mut W) -> Result<()>
    ) -> Result<()> {
        let token = ctx.heap_token_at_current_pos()?;
        let start_offset = ctx.position()?;
        
        content_callback(self, ctx)?;
        
        let size = ctx.position()? - start_offset;
        
        self.put_symbol(SymbolDeclaration {
            name: SymbolName::Unmangled(symbol_name.into()),
            offset: token,
            size: size as u32,
        });
        Ok(())
    }
    
    pub fn put_symbol(&mut self, symbol: SymbolDeclaration) {
        self.symbol_declarations.push(symbol);
    }
    
    pub fn put_relocation(&mut self, relocation: RelDeclaration) {
        self.relocations.push(relocation);
    }
    
    pub fn write_pointer_debug(&mut self, writer: &mut impl Writer, value: Pointer) -> Result<()> {
        writer.write_u32::<BigEndian>(value.0 | 0x70000000)?;
        Ok(())
    }
}

impl<C: ElfCategory> WriteDomain for ElfWriteDomain<C> {
    type Pointer = Pointer;
    type Cat = C;

    fn apply_reference(&mut self, writer: &mut impl Writer, heap_offset: usize) -> Result<()> {
        self.put_relocation(RelDeclaration {
            base_location: writer.position()? as usize,
            target_location: heap_offset,
        });
        
        if self.apply_debug_relocations {
            self.write_pointer_debug(writer, Pointer(heap_offset as u32))?;
        }
        Ok(())
    }
}

impl<C: ElfCategory> CanWriteBox<C> for ElfWriteDomain<C> {
    fn write_box_of<W: WriteCtx<C>>(
        &mut self,
        ctx: &mut W,
        write_content: impl FnOnce(&mut Self, &mut W::InnerCtx<'_>) -> Result<()>
    ) -> Result<()> {
        // hardcoding 'l' to make lct work is quite a hack
        self.write_box(ctx, Some(SymbolName::Internal('l')), write_content)
    }
}

// TODO: box with args

impl<C: ElfCategory> CanWriteSlice<C> for ElfWriteDomain<C> {
    fn write_slice_of<T: 'static, W: WriteCtx<C>>(
        &mut self,
        ctx: &mut W,
        values: &[T],
        write_content: impl Fn(&mut Self, &mut W::InnerCtx<'_>, &T) -> Result<()>,
    ) -> Result<()> {
        self.write_slice(ctx, values, None, write_content)?;
        Ok(())
    }
}

impl<C: ElfCategory, T: 'static> CanWriteSliceWithArgs<C, T, Option<SymbolName>> for ElfWriteDomain<C> {
    type PostState = ();
    
    fn write_slice_args_of<W: WriteCtx<C>, P>(
        &mut self,
        ctx: &mut W,
        values: &[T],
        args: Option<SymbolName>,
        write_content: impl Fn(&mut Self, &mut W::InnerCtx<'_>, &T) -> Result<P>,
        _write_content_post: impl Fn(&mut Self, &mut W::InnerCtx<'_>, &T, P) -> Result<()>,
    ) -> Result<()> {
        self.write_slice(ctx, values, args, |domain, ctx, value| {
            write_content(domain, ctx, value)?;
            Ok(())
        })
    }
}

impl<C: ElfCategory, T: Default + 'static> CanWriteSliceWithArgs<C, T, WriteNullTermiantedSliceArgs> for ElfWriteDomain<C> {
    type PostState = ();
    
    fn write_slice_args_of<W: WriteCtx<C>, P>(
        &mut self,
        ctx: &mut W,
        values: &[T],
        args: WriteNullTermiantedSliceArgs,
        write_content: impl Fn(&mut Self, &mut W::InnerCtx<'_>, &T) -> Result<P>,
        _write_content_post: impl Fn(&mut Self, &mut W::InnerCtx<'_>, &T, P) -> Result<()>,
    ) -> Result<()> {
        self.write_null_terminated_slice(ctx, values, args, |domain, ctx, value| {
            write_content(domain, ctx, value)?;
            Ok(())
        })
    }
}

impl<C: ElfCategory> CanWrite<C, String> for ElfWriteDomain<C> {
    fn write(&mut self, ctx: &mut impl WriteCtx<C>, value: &String) -> Result<()> {
        self.write_string(ctx, value, WriteStringArgs::default())
    }
}

impl<C: ElfCategory> CanWrite<C, Option<String>> for ElfWriteDomain<C> {
    fn write(&mut self, ctx: &mut impl WriteCtx<C>, value: &Option<String>) -> Result<()> {
        self.write_string_optional(ctx, value.as_deref(), WriteStringArgs::default())
    }
}

impl<C: ElfCategory> CanWriteWithArgs<C, String, WriteStringArgs> for ElfWriteDomain<C> {
    type PostState = ();
    
    fn write_args(&mut self, ctx: &mut impl WriteCtx<C>, value: &String, args: WriteStringArgs) -> Result<()> {
        self.write_string(ctx, value, args)
    }
}
