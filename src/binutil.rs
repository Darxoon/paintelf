use std::{borrow::Cow, fmt::Debug, io::SeekFrom};

use anyhow::{Result, anyhow, bail, ensure};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use vivibin::{
    CanRead, CanReadVec, CanReadVecWithArgs, CanWrite, CanWriteBox, CanWriteSlice, CanWriteSliceNested,
    CanWriteSliceWithArgs, CanWriteSliceWithArgsNested, CanWriteWithArgs, EndianSpecific, Endianness,
    HeapCategory, HeapID, HeapToken, ReadDomain, Readable, Reader, Writable, WriteCtx, WriteDomain, Writer,
    util::HashMap,
};

use crate::{
    RelDeclaration, SymbolDeclaration, SymbolName,
    elf::{Relocation, Symbol},
    scoped_reader_pos,
    util::{pointer::Pointer, read_string},
};

// deserializing
#[derive(Clone, Debug, Default)]
pub struct ReadNullTerminatedVecArgs;

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
    
    pub fn read_null_terminated_vec<T: Default + PartialEq + 'static, R: Reader>(self, reader: &mut R, read_content: impl Fn(&mut R) -> Result<T>) -> Result<Vec<T>> {
        let ptr: Option<Pointer> = self.read_pointer_optional(reader)?;
        
        let Some(ptr) = ptr else {
            return Ok(Vec::new());
        };
        
        scoped_reader_pos!(reader);
        reader.seek(SeekFrom::Start(ptr.into()))?;
        
        let mut values = Vec::new();
        loop {
            let value = read_content(reader)?;
            
            if value == T::default() {
                break;
            }
            
            values.push(value);
        }
        
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

impl<T: Default + PartialEq + 'static> CanReadVecWithArgs<T, ReadNullTerminatedVecArgs> for ElfReadDomain<'_> {
    fn read_std_vec_args_of<R: Reader>(self, reader: &mut R, _: ReadNullTerminatedVecArgs, read_content: impl Fn(&mut R) -> Result<T>) -> Result<Vec<T>> {
        self.read_null_terminated_vec(reader, read_content)
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataCategory {
    Data,
    Rodata,
    Strings,
}

impl DataCategory {
    pub const SIZE: usize = 3;
}

impl HeapCategory for DataCategory {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfCategoryType {
    Unit,
    Data,
}

#[derive(Debug, Clone, Copy)]
pub struct InlineString;

#[derive(Debug, Clone, Default)]
pub struct WriteSliceArgs {
    pub symbol_name: Option<SymbolName>,
}

#[derive(Debug, Clone, Default)]
pub struct WriteNullTerminatedSliceArgs {
    pub symbol_name: Option<SymbolName>,
    pub write_length: bool,
}

#[derive(Clone)]
pub struct ElfWriteDomain {
    pub string_map: HashMap<(HeapID, Cow<'static, str>), HeapToken>,
    pub symbol_declarations: Vec<SymbolDeclaration>,
    pub relocations: Vec<RelDeclaration>,
    pub apply_debug_relocations: bool,
    
    // does it even perform deduplication for non-Strings categories?
    prev_string_lengths: [usize; DataCategory::SIZE],
    string_counts: [usize; DataCategory::SIZE],
}

impl EndianSpecific for ElfWriteDomain {
    fn endianness(&self) -> Endianness {
        Endianness::Big
    }
}

impl ElfWriteDomain {
    pub fn new(apply_debug_relocations: bool) -> Self {
        Self {
            string_map: HashMap::new(),
            symbol_declarations: Vec::new(),
            relocations: Vec::new(),
            apply_debug_relocations,
            prev_string_lengths: [0; _],
            string_counts: [0; _],
        }
    }
    
    pub fn write_string_optional(&mut self, ctx: &mut WriteCtx<DataCategory>, value: Option<&str>) -> Result<()> {
        if let Some(value) = value {
            self.write_string(ctx, value)
        } else {
            0u32.to_writer(ctx, self)
        }
    }
    
    pub fn write_string(&mut self, ctx: &mut WriteCtx<DataCategory>, value: &str) -> Result<()> {
        let current_token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, self)?;
        
        let category = DataCategory::Strings;
        let heap_id = ctx.heap_id_of(&category);
        
        // this 1000 string limit is really funny to me
        let existing_token = if self.string_counts[category as usize] <= 1000 { 
            self.string_map.get(&(heap_id, Cow::Borrowed(value))).copied()
        } else {
            None
        };
        
        if let Some(token) = existing_token {
            ctx.add_relocation(current_token, token)?;
            return Ok(());
        }
        
        self.string_counts[category as usize] += 1;
        let alignment = if self.prev_string_lengths[category as usize] > 2 || value.len() > 1 { 4 } else { 0 };
        self.prev_string_lengths[category as usize] = value.len();
        
        ctx.allocate_next_block_aligned(Some(category), alignment, false, |ctx| {
            let start_pos = ctx.position() as usize;
            let new_token = ctx.heap_token_at_current_pos()?;
            ctx.add_relocation(current_token, new_token)?;
            
            ctx.write_c_str(value)?;
            if value.len() > 2 {
                ctx.align_to(4)?;
            }
            let name_size = ctx.position() as usize - start_pos;
            
            self.put_symbol(SymbolDeclaration {
                name: SymbolName::Internal('.'),
                offset: new_token,
                size: name_size as u32,
            });
            
            self.string_map.insert((heap_id, Cow::from(value.to_string())), new_token);
            Ok(())
        })?;
        
        Ok(())
    }
    
    pub fn write_string_inline(&mut self, ctx: &mut WriteCtx<DataCategory>) -> Result<HeapToken> {
        let current_token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, self)?;
        Ok(current_token)
    }
    
    pub fn write_string_inline_post(&mut self, ctx: &mut WriteCtx<DataCategory>, value: &str, base: HeapToken) -> Result<()> {
        let category = *ctx.default_category();
        let heap_id = ctx.heap_id_of(&category);
        
        // this 1000 string limit is really funny to me
        let existing_token = if self.string_counts[category as usize] <= 1000 { 
            self.string_map.get(&(heap_id, Cow::Borrowed(value))).copied()
        } else {
            None
        };
        
        if let Some(token) = existing_token {
            println!("writing existing string inline {value:?} ({base:x?} -> {token:x?})");
            ctx.add_relocation(base, token)?;
            return Ok(());
        }
        
        self.string_counts[category as usize] += 1;
        
        // relocation
        let start_pos = ctx.position() as usize;
        let new_token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(base, new_token)?;
        
        println!("writing new string inline {value:?} ({base:x?} -> {new_token:x?})");
        
        // write string
        ctx.write_c_str(value)?;
        if value.len() > 2 {
            ctx.align_to(4)?;
        }
        let name_size = ctx.position() as usize - start_pos;
        
        self.put_symbol(SymbolDeclaration {
            name: SymbolName::Internal('.'),
            offset: new_token,
            size: name_size as u32,
        });
        
        self.string_map.insert((heap_id, Cow::from(value.to_string())), new_token);
        Ok(())
    }
    
    pub fn write_box(&mut self, ctx: &mut WriteCtx<DataCategory>) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, self)?;
        Ok(token)
    }
    
    pub fn write_box_post<P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        args: Option<SymbolName>,
        base: HeapToken,
        write_content: impl FnOnce(&mut Self, &mut WriteCtx<DataCategory>) -> Result<P>,
        write_content_post: impl FnOnce(&mut Self, &mut WriteCtx<DataCategory>, P) -> Result<()>,
    ) -> Result<()> {
        let token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(base, token)?;
        
        let start_pos = ctx.position() as usize;
        let state = write_content(self, ctx)?;
        let links_size = ctx.position() as usize - start_pos;
        
        write_content_post(self, ctx, state)?;
        
        if let Some(name) = args {
            self.put_symbol(SymbolDeclaration {
                name,
                offset: token,
                size: links_size as u32,
            });
        }
        Ok(())
    }
    
    pub fn write_slice<T: 'static>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
    ) -> Result<HeapToken> {
        let current_token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, self)?;
        
        (values.len() as u32).to_writer(ctx, self)?;
        Ok(current_token)
    }
    
    pub fn write_slice_post<T: 'static, P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        base: HeapToken,
        args: WriteSliceArgs,
        write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
    ) -> Result<Vec<P>> {
        ctx.align_to(4)?;
        
        // write main values
        // TODO: there is repeated code here
        let start_pos = ctx.position() as usize;
        let new_token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(base, new_token)?;
        
        let mut states = Vec::with_capacity(values.len());
        
        for value in values {
            states.push(write_content(self, ctx, value)?);
        }
        
        if let Some(name) = args.symbol_name {
            let links_size = ctx.position() as usize - start_pos;
            
            self.put_symbol(SymbolDeclaration {
                name,
                offset: new_token,
                size: links_size as u32,
            });
        }
        
        Ok(states)
    }
    
    pub fn write_slice_extra_post<T: 'static, P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        states: Vec<P>,
        write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<()> {
        for (value, state) in values.iter().zip(states) {
            write_content_post(self, ctx, value, state)?;
        }
        
        Ok(())
    }
    
    pub fn write_null_terminated_slice<T: Default + 'static>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        args: WriteNullTerminatedSliceArgs,
    ) -> Result<HeapToken> {
        let current_token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, self)?;
        
        if args.write_length {
            (values.len() as u32).to_writer(ctx, self)?;
        }
        
        Ok(current_token)
    }
    
    pub fn write_null_terminated_slice_post<T: Default + 'static, P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        args: WriteNullTerminatedSliceArgs,
        base: HeapToken,
        write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
    ) -> Result<Vec<P>> {
        ctx.align_to(4)?;
        
        // write main values
        let start_pos = ctx.position() as usize;
        let new_token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(base, new_token)?;
        
        let mut states = Vec::with_capacity(values.len() + 1);
        
        for value in values {
            states.push(write_content(self, ctx, value)?);
        }
        states.push(write_content(self, ctx, &T::default())?);
        
        if let Some(name) = args.symbol_name {
            let links_size = ctx.position() as usize - start_pos;
            
            self.put_symbol(SymbolDeclaration {
                name,
                offset: new_token,
                size: links_size as u32,
            });
        }
        
        Ok(states)
    }
    
    pub fn write_null_terminated_slice_extra_post<T: Default + 'static, P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        states: Vec<P>,
        write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<()> {
        for (value, state) in values.iter().zip(states) {
            write_content_post(self, ctx, value, state)?;
        }
        
        Ok(())
    }
    
    pub fn write_symbol(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        symbol_name: impl Into<String>,
        content_callback: impl FnOnce(&mut Self, &mut WriteCtx<DataCategory>) -> Result<()>
    ) -> Result<()> {
        let token = ctx.heap_token_at_current_pos()?;
        let start_offset = ctx.position();
        
        content_callback(self, ctx)?;
        
        let size = ctx.position() - start_offset;
        
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

impl WriteDomain for ElfWriteDomain {
    type Pointer = Pointer;
    type Cat = DataCategory;

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

impl CanWriteBox<DataCategory> for ElfWriteDomain {
    type PostState = HeapToken;

    fn write_box_of<P>(
        &mut self,
        ctx: &mut WriteCtx<'_, DataCategory>,
        _write_content: impl FnOnce(&mut Self, &mut WriteCtx<'_, DataCategory>) -> Result<P>,
        _write_content_post: impl FnOnce(&mut Self, &mut WriteCtx<'_, DataCategory>, P) -> Result<()>,
    ) -> Result<Self::PostState> {
        self.write_box(ctx)
    }

    fn write_box_of_post<P>(
        &mut self,
        ctx: &mut WriteCtx<'_, DataCategory>,
        state: Self::PostState,
        write_content: impl FnOnce(&mut Self, &mut WriteCtx<'_, DataCategory>) -> Result<P>,
        write_content_post: impl FnOnce(&mut Self, &mut WriteCtx<'_, DataCategory>, P) -> Result<()>,
    ) -> Result<()> {
        // hardcoding 'l' to make lct work is quite a hack
        self.write_box_post(ctx, Some(SymbolName::Internal('l')), state, write_content, write_content_post)
    }
}

impl CanWriteSlice<DataCategory> for ElfWriteDomain {
    type PostState = HeapToken;
    
    fn write_slice_of<T: 'static, P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        _write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
    ) -> Result<HeapToken> {
        self.write_slice(ctx, values)
    }
    
    fn write_slice_of_post<T: 'static, P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        state: Self::PostState,
        write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
        write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<()> {
        let states = self.write_slice_post(ctx, values, state, WriteSliceArgs::default(), write_content)?;
        self.write_slice_extra_post(ctx, values, states, write_content_post)
    }
}

impl<P> CanWriteSliceNested<DataCategory, P> for ElfWriteDomain {
    type ExtraState = Vec<P>;
    
    fn write_slice_nested_of_post<T: 'static>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        state: Self::PostState,
        write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
        _write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<Vec<P>> {
        let states = self.write_slice_post(ctx, values, state, WriteSliceArgs::default(), write_content)?;
        Ok(states)
    }
    
    fn write_slice_of_extra_post<T: 'static>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        state: Vec<P>,
        write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<()>
    {
        self.write_slice_extra_post(ctx, values, state, write_content_post)
    }
}

impl<T: 'static> CanWriteSliceWithArgs<DataCategory, T, WriteSliceArgs> for ElfWriteDomain {
    type PostState = HeapToken;
    
    fn write_slice_args_of<P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        _args: WriteSliceArgs,
        _write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
    ) -> Result<HeapToken> {
        self.write_slice(ctx, values)
    }
    
    fn write_slice_args_of_post<P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        state: Self::PostState,
        args: WriteSliceArgs,
        write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
        write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<()> {
        let states = self.write_slice_post(ctx, values, state, args, write_content)?;
        self.write_slice_extra_post(ctx, values, states, write_content_post)
    }
}

impl<T: Default + 'static> CanWriteSliceWithArgs<DataCategory, T, WriteNullTerminatedSliceArgs> for ElfWriteDomain {
    type PostState = HeapToken;
    
    fn write_slice_args_of<P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        args: WriteNullTerminatedSliceArgs,
        _write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
    ) -> Result<HeapToken> {
        self.write_null_terminated_slice(ctx, values, args)
    }
    
    fn write_slice_args_of_post<P>(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        state: HeapToken,
        args: WriteNullTerminatedSliceArgs,
        write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
        write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<()> {
        let states = self.write_null_terminated_slice_post(
            ctx,
            values,
            args,
            state,
            write_content,
        )?;
        self.write_null_terminated_slice_extra_post(ctx, values, states, write_content_post)
    }
}

impl<T: Default + 'static, P> CanWriteSliceWithArgsNested<DataCategory, T, WriteNullTerminatedSliceArgs, P> for ElfWriteDomain {
    type ExtraState = Vec<P>;

    fn write_slice_args_nested_of_post(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        state: Self::PostState,
        args: WriteNullTerminatedSliceArgs,
        write_content: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T) -> Result<P>,
        _write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<Self::ExtraState> {
        let states = self.write_null_terminated_slice_post(ctx, values, args, state, write_content)?;
        Ok(states)
    }
    
    fn write_slice_args_of_extra_post(
        &mut self,
        ctx: &mut WriteCtx<DataCategory>,
        values: &[T],
        state: Vec<P>,
        _args: WriteNullTerminatedSliceArgs,
        write_content_post: impl Fn(&mut Self, &mut WriteCtx<DataCategory>, &T, P) -> Result<()>,
    ) -> Result<()> {
        self.write_null_terminated_slice_extra_post(ctx, values, state, write_content_post)
    }
}

impl CanWrite<DataCategory, String> for ElfWriteDomain {
    type PostState = ();
    
    fn write(&mut self, ctx: &mut WriteCtx<DataCategory>, value: &String) -> Result<()> {
        self.write_string(ctx, value)
    }
}

impl CanWrite<DataCategory, Option<String>> for ElfWriteDomain {
    type PostState = ();
    
    fn write(&mut self, ctx: &mut WriteCtx<DataCategory>, value: &Option<String>) -> Result<()> {
        self.write_string_optional(ctx, value.as_deref())
    }
}

impl CanWriteWithArgs<DataCategory, String, InlineString> for ElfWriteDomain {
    type PostState = HeapToken;
    
    fn write_args(&mut self, ctx: &mut WriteCtx<DataCategory>, _value: &String, _: InlineString) -> Result<HeapToken> {
        self.write_string_inline(ctx)
    }
    
    fn write_args_post(&mut self, ctx: &mut WriteCtx<DataCategory>, value: &String, state: HeapToken, _: InlineString) -> Result<()> {
        self.write_string_inline_post(ctx, value, state)
    }
}
