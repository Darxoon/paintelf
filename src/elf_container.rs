use std::{collections::HashMap, io::{Cursor, SeekFrom, Write}, mem::{self, offset_of}};

use anyhow::{anyhow, bail, Error, Result};
use binrw::{BinRead, BinWrite};
use indexmap::IndexMap;
use memchr::memmem;
use vivibin::{align_to, Reader, Writer};

use crate::{elf::{Relocation, Section, SectionHeader, Symbol, SymbolHeader}, util::{pointer::Pointer, read_string}};

pub const ELF_HEADER_IDENT: [u8; 16] = [0x7F, 0x45, 0x4C, 0x46, 0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];

#[derive(Debug, Clone, BinRead, BinWrite)]
#[brw(big)]
#[repr(C)]
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
    pub header: ElfHeader,
    pub symbols: IndexMap<String, Symbol>,
    pub content_sections: IndexMap<String, Section>,
    pub meta_sections: IndexMap<String, Section>,
}

impl ElfContainer {
    pub fn new(header: ElfHeader) -> Self {
        let mut content_sections = IndexMap::new();
        content_sections.insert("".to_string(), Section::default());
        
        Self {
            header,
            symbols: IndexMap::new(),
            content_sections,
            meta_sections: IndexMap::new(),
        }
    }
    
    pub fn from_reader(reader: &mut impl Reader) -> Result<Self> {
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
        let mut all_section_names: Vec<String> = Vec::with_capacity(section_headers.len());
        let mut content_sections: IndexMap<String, Section> = IndexMap::with_capacity(2);
        let mut meta_sections: IndexMap<String, Section> = IndexMap::with_capacity(section_headers.len() - 1);
        
        let mut symbol_headers: Option<Vec<SymbolHeader>> = None;
        let mut string_table: Option<Vec<u8>> = None;
        
        for header in section_headers {
            let name = read_string(&sh_string_table, header.sh_name)?.to_string();
            let section = Section::from_reader(header, name.clone(), reader)?;
            
            all_section_names.push(name.clone());
            
            if name.starts_with(".rela") {
                let relocation_count = section.content.len() / mem::size_of::<Relocation>();
                let mut reader = Cursor::new(section.content.as_slice());
                
                let relocations: IndexMap<Pointer, Relocation> = (0..relocation_count)
                    .map(|_| match Relocation::read(&mut reader) {
                        Ok(relocation) => Ok((relocation.offset.into(), relocation)),
                        Err(err) => Err(err.into()),
                    })
                    .collect::<Result<_>>()?;
                
                let original_section: &mut Section = content_sections.get_mut(&name[5..])
                    .ok_or_else(|| anyhow!("Could not find section {}", &name[5..]))?;
                original_section.relocations = Some(relocations);
                
                meta_sections.insert(name.clone(), section);
                continue;
            }
            
            match name.as_str() {
                ".strtab" => {
                    string_table = Some(section.content.clone());
                    meta_sections.insert(name, section);
                },
                ".symtab" => {
                    let mut reader = Cursor::new(section.content.as_slice());
                    
                    let symbol_count = section.content.len() / mem::size_of::<SymbolHeader>();
                    let symtab: Vec<SymbolHeader> = (0..symbol_count)
                        .map(|_| SymbolHeader::read(&mut reader).map_err(Error::from))
                        .collect::<Result<_>>()?;
                    
                    symbol_headers = Some(symtab);
                    meta_sections.insert(name, section);
                },
                ".shstrtab" => {
                    meta_sections.insert(name, section);
                },
                _ => {
                    content_sections.insert(name, section);
                },
            }
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
                let name = all_section_names.get(sym_header.st_shndx as usize)
                    .ok_or_else(|| anyhow!("Could not find section with id {}", sym_header.st_shndx))?;
                
                name.clone()
            } else {
                read_string(&string_table, sym_header.st_name)?.to_string()
            };
            
            symbols.insert(name.clone(), Symbol::new(sym_header, name));
        }
        
        Ok(ElfContainer {
            header,
            symbols,
            content_sections,
            meta_sections,
        })
    }
    
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut writer = Cursor::new(Vec::new());
        
        // write header
        self.header.write(&mut writer)?;
        
        let mut section_offsets: HashMap<String, Pointer> = HashMap::new();
        
        // write content sections
        for section in self.content_sections.values() {
            align_to(&mut writer, section.header.sh_addralign as usize)?;
            section_offsets.insert(section.name.clone(), Pointer::current(&mut writer)?);
            writer.write_all(&section.content)?;
        }
        
        // write non-relocation meta sections
        for section in self.meta_sections.values() {
            if section.name.starts_with(".rela") {
                continue;
            }
            
            align_to(&mut writer, section.header.sh_addralign as usize)?;
            section_offsets.insert(section.name.clone(), Pointer::current(&mut writer)?);
            writer.write_all(&section.content)?;
        }
        
        // write relocation sections
        for section in self.meta_sections.values() {
            if !section.name.starts_with(".rela") {
                continue;
            }
            
            align_to(&mut writer, section.header.sh_addralign as usize)?;
            section_offsets.insert(section.name.clone(), Pointer::current(&mut writer)?);
            writer.write_all(&section.content)?;
        }
        
        // write section header table
        let sh_offset = Pointer::current(&mut writer)?;
        let shstrtab = &self.meta_sections[".shstrtab"];
        
        SectionHeader::default().write(&mut writer)?;
        
        for section in self.content_sections.values() {
            if section.name.is_empty() {
                continue;
            }
            
            Self::write_section_header(&mut writer, &section_offsets, &shstrtab.content, section)?;
            
            let relocation_section = self.meta_sections.get(&format!(".rela{}", &section.name));
            if let Some(relocation_section) = relocation_section {
                Self::write_section_header(&mut writer, &section_offsets, &shstrtab.content, relocation_section)?;
            }
        }
        
        Self::write_section_header(&mut writer, &section_offsets, &shstrtab.content, shstrtab)?;
        
        let symtab = &self.meta_sections[".symtab"];
        Self::write_section_header(&mut writer, &section_offsets, &shstrtab.content, symtab)?;
        
        let strtab = &self.meta_sections[".strtab"];
        Self::write_section_header(&mut writer, &section_offsets, &shstrtab.content, strtab)?;
        
        // apply section header offset
        writer.set_position(offset_of!(ElfHeader, e_shoff) as u64);
        sh_offset.write(&mut writer)?;
        
        Ok(writer.into_inner())
    }
    
    fn write_section_header(writer: &mut impl Writer, section_offsets: &HashMap<String, Pointer>, shstrtab: &[u8], section: &Section) -> Result<()> {
        let name_offset = memmem::find(&shstrtab, section.name.as_bytes())
            .unwrap_or(0);
        
        let header = SectionHeader {
            sh_name: name_offset as u32,
            sh_offset: section_offsets[&section.name].into(),
            sh_size: section.content.len() as u32,
            ..section.header
        };
        
        align_to(writer, section.header.sh_addralign as usize)?;
        header.write(writer)?;
        Ok(())
    }
}
