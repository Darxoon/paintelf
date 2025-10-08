use std::{
    fmt::Debug,
    io::{Cursor, Read, Seek, SeekFrom}, mem,
};

use anyhow::{anyhow, bail, Error, Result};
use binrw::{BinRead, BinWrite};
use indexmap::IndexMap;

use crate::{util::{pointer::Pointer, read_string}};

// const ELF_HEADER_IDENT: [u8; 16] = [0x7F, 0x45, 0x4C, 0x46, 0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];

#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(big)]
pub struct ElfHeader {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u32,
    pub e_phoff: u32,
    /// Offset into the file of the section header table
    pub e_shoff: u32,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    /// Section header count
    pub e_shnum: u16,
    /// Index of the section header string table into section header table
    pub e_shstrndx: u16,
}

#[derive(Debug)]
pub struct ElfContainer {
    pub symbols: IndexMap<String, Symbol>,
    pub sections: IndexMap<String, Section>,
}

impl ElfContainer {
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let header = ElfHeader::read(reader)?;
        
        reader.seek(SeekFrom::Start(header.e_shoff as u64))?;
        
        let section_headers: Vec<SectionHeader> = (0..header.e_shnum)
            .map(|_| SectionHeader::read(reader).map_err(Error::from))
            .collect::<Result<_>>()?;
        
        // Read section header string table
        let sh_string_table_header = &section_headers[header.e_shstrndx as usize];
        let mut sh_string_table = vec![0; sh_string_table_header.sh_size as usize];
        reader.seek(SeekFrom::Start(sh_string_table_header.sh_offset as u64))?;
        reader.read_exact(&mut sh_string_table)?;
        
        // Read other sections
        let mut sections: IndexMap<String, Section> = IndexMap::with_capacity(section_headers.len());
        let mut symbol_headers: Option<Vec<SymbolHeader>> = None;
        let mut string_table: Option<Vec<u8>> = None;
        
        for header in section_headers {
            let name = read_string(&sh_string_table, header.sh_name)?.to_string();
            let section = Section::from_reader(header, name.clone(), reader)?;
            
            if name.starts_with(".rela") {
                let relocation_count = section.content.len() / mem::size_of::<Relocation>();
                let mut reader = Cursor::new(section.content.as_slice());
                
                let relocations: IndexMap<Pointer, Relocation> = (0..relocation_count)
                    .map(|_| match Relocation::read(&mut reader) {
                        Ok(relocation) => Ok((relocation.offset.into(), relocation)),
                        Err(err) => Err(err.into()),
                    })
                    .collect::<Result<_>>()?;
                
                let original_section: &mut Section = sections.get_mut(&name[5..])
                    .ok_or_else(|| anyhow!("Could not find section {}", &name[5..]))?;
                original_section.relocations = Some(relocations);
            }
            
            match name.as_str() {
                ".strtab" => {
                    string_table = Some(section.content.clone());
                },
                ".symtab" => {
                    let mut reader = Cursor::new(section.content.as_slice());
                    
                    let symbol_count = section.content.len() / mem::size_of::<SymbolHeader>();
                    let symtab: Vec<SymbolHeader> = (0..symbol_count)
                        .map(|_| SymbolHeader::read(&mut reader).map_err(Error::from))
                        .collect::<Result<_>>()?;
                    
                    symbol_headers = Some(symtab);
                },
                _ => {},
            }
            
            sections.insert(name, section);
        }
        
        let Some(string_table) = string_table else {
            bail!("Could not find section .strtab");
        };
        let Some(symbol_headers) = symbol_headers else {
            bail!("Could not find section .symtab");
        };
        
        let mut symbols: IndexMap<String, Symbol> = IndexMap::with_capacity(symbol_headers.len());
        
        for sym_header in symbol_headers {
            let name = if sym_header.st_info == 3 {
                // section symbol
                let section = sections.get_index(sym_header.st_shndx as usize)
                    .ok_or_else(|| anyhow!("Could not find section with id {}", sym_header.st_shndx))?
                    .1;
                
                section.name.clone()
            } else {
                read_string(&string_table, sym_header.st_name)?.to_string()
            };
            
            symbols.insert(name.clone(), Symbol::new(sym_header, name));
        }
        
        Ok(ElfContainer {
            symbols,
            sections
        })
    }
}

#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(big)]
pub struct SectionHeader {
	pub sh_name: u32,
	pub sh_type: u32,
	pub sh_flags: u32,
	pub sh_addr: u32,
	pub sh_offset: u32,
	pub sh_size: u32,
	pub sh_link: u32,
	pub sh_info: u32,
	pub sh_addralign: u32,
	pub sh_entsize: u32,
}

#[derive(Clone)]
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

#[derive(Debug, Clone, BinRead, BinWrite)]
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
