use std::{fmt::Debug, io::{Read, Seek, SeekFrom}};

use anyhow::Result;
use binrw::{BinRead, BinWrite};
use indexmap::IndexMap;

use crate::util::pointer::Pointer;

pub mod container;

/// Section flag which indicates that the section occupies memory at execution.
pub const SHF_ALLOC: u32 = 0x2;
/// Section flag which indicates that `sh_info` contains the index of another section header.
pub const SHF_INFO_LINK: u32 = 0x40;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, BinRead, BinWrite)]
#[brw(repr = u32)]
pub enum SectionType {
    #[default]
    None,
    Progbits,
    SymTable,
    StringTable,
    Rela,
}

#[derive(Debug, Clone, Default, BinRead, BinWrite)]
#[brw(big)]
pub struct SectionHeader {
	pub sh_name: u32,
	pub sh_type: SectionType,
	pub sh_flags: u32,
	pub sh_addr: u32,
	pub sh_offset: u32,
	pub sh_size: u32,
	pub sh_link: u32,
	pub sh_info: u32,
	pub sh_addralign: u32,
	pub sh_entsize: u32,
}

#[derive(Clone, Default)]
pub struct Section {
    pub header: SectionHeader,
    pub name: String,
    pub relocations: Option<IndexMap<Pointer, Relocation>>,
    pub content: Vec<u8>,
}

impl Section {
    pub fn from_reader<R: Read + Seek>(header: SectionHeader, name: String, reader: &mut R) -> Result<Self> {
        reader.seek(SeekFrom::Start(header.sh_offset as u64))?;
        
        let mut content: Vec<u8> = vec![0; header.sh_size as usize];
        reader.read_exact(&mut content)?;
        
        Ok(Self {
            header,
            name,
            relocations: None,
            content,
        })
    }
}

impl Debug for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Section")
            .field("header", &self.header)
            .field("name", &self.name)
            .field("content", &format_args!("<{} bytes>", self.content.len()))
            .finish()
    }
}

#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(big)]
pub struct Relocation {
    pub offset: u32,
    pub info: u32,
    pub addend: u32,
}

impl Relocation {
    pub fn new(offset: u32, info: u32, addend: u32) -> Self {
        Self {
            offset,
            info,
            addend,
        }
    }
}

#[derive(Debug, Clone, Default, BinRead, BinWrite)]
#[brw(big)]
pub struct SymbolHeader {
    pub st_name: u32,
    pub st_value: u32,
    pub st_size: u32,
    pub st_info: u8,
    pub st_other: u8,
    pub st_shndx: u16,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub header: SymbolHeader,
    pub name: String,
}

impl Symbol {
    pub fn new(header: SymbolHeader, name: String) -> Self {
        Self { header, name }
    }
    
    pub fn offset(&self) -> u32 {
        self.header.st_value
    }
    
    pub fn size(&self) -> u32 {
        self.header.st_size
    }
}

pub const AUTO_SYMBOL_NAME_CHAR_COUNT: usize = 93;
pub const AUTO_SYMBOL_NAME_CHARS: &[u8; AUTO_SYMBOL_NAME_CHAR_COUNT] = 
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@$%^&*()_+-=[]{};\'\\:\"|,./<>?~`";

#[derive(Default)]
pub struct SymbolNameGenerator {
    indices: Vec<usize>,
    result: Vec<u8>,
}

impl SymbolNameGenerator {
    pub fn new() -> Self {
        Self::default()
    }
    
    // since this is a lending iterator, which is not compatible with std::Iterator
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> &str {
        if self.indices.is_empty() {
            self.indices.push(0);
            self.result.push(AUTO_SYMBOL_NAME_CHARS[0]);
            
            return "";
        }
        
        let mut i = self.indices.len() - 1;
        while self.count_up_check_overflow(i) {
            if i == 0 {
                self.indices.push(0);
                self.result.push(AUTO_SYMBOL_NAME_CHARS[0]);
                break;
            }
            i -= 1;
        }
        
        // SAFETY: self.result only ever contains valid ascii characters
        unsafe {
            str::from_utf8_unchecked(&self.result)
        }
    }
    
    fn count_up_check_overflow(&mut self, index: usize) -> bool {
        let value = &mut self.indices[index];
        *value += 1;
        
        let overflow = *value >= AUTO_SYMBOL_NAME_CHAR_COUNT;
        if overflow && index == 0 {
            // first character for some reason is never 'a'
            *value = 1;
        } else if overflow {
            *value = 0;
        }
        
        self.result[index] = AUTO_SYMBOL_NAME_CHARS[*value];
        
        overflow
    }
}


