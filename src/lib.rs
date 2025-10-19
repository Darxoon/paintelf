use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Display,
    io::{Cursor, Read, Seek, SeekFrom, Write},
};

use anyhow::{Result, anyhow};
use binrw::BinWrite;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use vivibin::{HeapToken, WriteCtxImpl, WriteDomainExt};

use crate::{
    binutil::ElfWriteDomain,
    elf::{
        Relocation, Section, Symbol, SymbolHeader, SymbolNameGenerator,
        container::{ELF_HEADER_IDENT, ElfContainer, ElfHeader},
    },
    formats::{FileData, mapid::write_mapid, maplink::write_maplink, shop::write_shops},
    util::pointer::Pointer,
};

pub mod binutil;
pub mod elf;
pub mod formats;
pub mod matching;
pub mod util;

#[cfg(test)]
mod tests;

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
        matches!(self, SymbolName::Internal(_) | SymbolName::InternalNamed(_) | SymbolName::InternalUnmangled(_))
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
    pub base_location: usize,
    pub target_location: usize,
}

pub fn reassemble_elf_container(data: &FileData, apply_debug_relocations: bool) -> Result<ElfContainer> {
    let mut block_offsets = Vec::new();
    let mut domain = ElfWriteDomain::new(apply_debug_relocations);
    
    // serialize data
    let mut ctx: WriteCtxImpl<ElfWriteDomain> = ElfWriteDomain::new_ctx();
    match data {
        FileData::Maplink(maplink_areas) => {
            write_maplink(&mut ctx, &mut domain, maplink_areas)?;
        },
        FileData::Shop(shop_list) => {
            write_shops(&mut ctx, &mut domain, shop_list)?;
        },
        FileData::MapId(map_groups) => {
            write_mapid(&mut ctx, &mut domain, map_groups)?;
        },
        FileData::Dispos(_) => todo!(),
    };
    let result_buffer = ctx.to_buffer(&mut domain, Some(&mut block_offsets))?;
    
    // serialize elf metadata
    let initial_strtab = format!("\0{}\0", data.cpp_file_name()).into_bytes();
    
    let mut symbol_indices = HashMap::new();
    let (symtab, last_local_symbol, strtab) = write_symtab(
        initial_strtab,
        &block_offsets,
        &mut symbol_indices,
        &mut domain.symbol_declarations
    )?;
    let rela_rodata = write_relocations(&symbol_indices, &mut domain.relocations)?;
    
    // populate new ElfContainer
    // TODO: verify these values are correct in shifted files
    let header = ElfHeader {
        e_ident: ELF_HEADER_IDENT,
        e_ident_padding_unk: data.elf_ident_padding_unk(),
        e_type: 1,
        e_machine: 0x14,
        e_version: 1,
        e_entry: 0,
        e_phoff: 0,
        e_shoff: u32::MAX,
        e_flags: 0x80000000,
        e_ehsize: 0x34,
        e_phentsize: 0,
        e_phnum: 0,
        e_shentsize: 0x28,
        e_shnum: 6,
        e_shstrndx: 3,
    };
    
    let mut result = ElfContainer::new(header);
    result.add_content_section_with_relocations(".rodata", 4, result_buffer, rela_rodata);
    
    const SH_STRING_TAB: &[u8] = b"\0.symtab\0.strtab\0.shstrtab\0.rela.rodata\0";
    result.add_string_table_raw(".shstrtab", 0, 1, SH_STRING_TAB.to_owned());
    result.add_symbol_table_raw(".symtab", 0, last_local_symbol, 4, symtab);
    result.add_string_table_raw(".strtab", 0, 1, strtab);
    
    Ok(result)
}

pub fn write_relocations(
    symbol_indices: &HashMap<usize, usize>,
    relocations: &mut [RelDeclaration],
) -> Result<Vec<u8>> {
    relocations.sort_by_key(|rel| rel.base_location);
    
    let mut writer = Cursor::new(Vec::new());
    
    for relocation in relocations {
        let symbol_idx = symbol_indices.get(&relocation.target_location).unwrap();
        
        let raw = Relocation::new(relocation.base_location as u32, (symbol_idx << 8 | 1) as u32, 0);
        raw.write(&mut writer)?;
    }
    
    Ok(writer.into_inner())
}

pub fn write_symtab(
    initial_content: Vec<u8>,
    block_offsets: &[usize],
    out_symbol_indices: &mut HashMap<usize, usize>,
    symbol_declarations: &mut Vec<SymbolDeclaration>,
) -> Result<(Vec<u8>, u32, Vec<u8>)> {
    // name unnamed internal symbols
    {
        let mut symbols: Vec<(char, &mut SymbolDeclaration)> = symbol_declarations.iter_mut()
            .flat_map(|symbol| match symbol.name {
                    SymbolName::Internal(initial_char) => Some((initial_char, symbol)),
                    _ => None,
            })
            .collect::<Vec<_>>();
        
        symbols.sort_by(|(initial_char1, symbol1), (initial_char2, symbol2)| {
            initial_char1.cmp(initial_char2).then(symbol1.offset.cmp(&symbol2.offset))
        });
        
        let mut symbol_name_gen = SymbolNameGenerator::new();
        let mut prev_initial_char = '\0';
        
        for (initial_char, symbol) in symbols {
            // Make sure every initial_char has its own name gen
            if prev_initial_char  != initial_char {
                symbol_name_gen = SymbolNameGenerator::new();
                prev_initial_char = initial_char;
            }
            
            let tail = symbol_name_gen.next();
            
            let mut name = String::with_capacity(tail.len() + 1);
            name.push(initial_char);
            name.push_str(tail);
            
            symbol.name = SymbolName::InternalUnmangled(name);
        }
    }
    
    // name named internal symbols
    {
        let mut symbol_name_gen = SymbolNameGenerator::new();
        
        let mut symbols: Vec<&mut SymbolDeclaration> = symbol_declarations.iter_mut()
            .filter(|symbol| matches!(symbol.name, SymbolName::InternalNamed(_)))
            .collect::<Vec<_>>();
        
        symbols.sort_by(|a, b| {
            let SymbolName::InternalNamed(name1) = &a.name else {
                unreachable!();
            };
            let SymbolName::InternalNamed(name2) = &b.name else {
                unreachable!();
            };
            
            fn is_less_special(a: &str, b: &str) -> bool {
                let a_bytes = a.as_bytes();
                let b_bytes = b.as_bytes();
                
                if a_bytes.len() == b_bytes.len() || !a_bytes.starts_with(b_bytes) {
                    return false;
                }
                
                // SAFETY: Assuming b is valid utf8, it does not end on a continuation byte,
                // so the first b bytes of a also don't. Therefore, this slice does not start
                // in the middle of a codepoint and assuming a is valid, this slice is too.
                let tail = unsafe { str::from_utf8_unchecked(&a_bytes[b_bytes.len()..]) };
                let first_char = tail.chars().next().unwrap();
                
                // wtf??
                first_char < 'P'
            }
            
            if is_less_special(name1, name2) {
                Ordering::Less
            } else if is_less_special(name2, name1) {
                Ordering::Greater
            } else {
                name1.cmp(name2)
            }
        });
        
        for symbol in symbols {
            let SymbolName::InternalNamed(name) = &symbol.name else {
                unreachable!();
            };
            
            let initial_char = name.chars().next().unwrap();
            let tail = symbol_name_gen.next();
            
            let mut result = String::with_capacity(tail.len() + 1);
            result.push(initial_char);
            result.push_str(tail);
            
            // println!("{name} {result}");
            
            symbol.name = SymbolName::InternalUnmangled(result);
        }
    }
    
    // start serializing
    let mut writer = Cursor::new(Vec::new());
    let mut symbol_count =  0;
    
    // null
    BinWrite::write(&SymbolHeader::default(), &mut writer)?;
    // data_fld_maplink.cpp
    BinWrite::write(&SymbolHeader {
        st_name: 1,
        st_value: 0,
        st_size: 0,
        st_info: 4,
        st_other: 0,
        st_shndx: 0xFFF1,
    }, &mut writer)?;
    // .rodata
    BinWrite::write(&SymbolHeader {
        st_name: 0,
        st_value: 0,
        st_size: 0,
        st_info: 3,
        st_other: 0,
        st_shndx: 1,
    }, &mut writer)?;
    symbol_count += 3;
    
    // setup serialization of symbols
    let named_symbols: Vec<SymbolDeclaration> = symbol_declarations
        .extract_if(.., |symbol| !symbol.name.is_internal())
        .collect::<Vec<_>>();
    
    symbol_declarations.sort_by_key(|symbol| symbol.offset.resolve(block_offsets));
    
    let mut strtab = Cursor::new(initial_content);
    strtab.seek(SeekFrom::End(0))?;
    
    #[allow(clippy::let_with_type_underscore)]
    let mut write_symbol: _ = |writer: &mut Cursor<Vec<u8>>, symbol_count: &mut usize, symbol: &SymbolDeclaration, st_info: u8| -> Result<()> {
        // serialize name
        let name_ptr = if let Some(symbol_name) = symbol.name.as_str() {
            let name_ptr = Pointer::current(&mut strtab)?;
            strtab.write_all(symbol_name.as_bytes())?;
            strtab.write_u8(0)?;
            name_ptr.into()
        } else {
            0u32
        };
        
        // serialize symbol
        out_symbol_indices.insert(symbol.offset.resolve(block_offsets), *symbol_count);
        *symbol_count += 1;
        BinWrite::write(&SymbolHeader {
            st_name: name_ptr,
            st_value: symbol.offset.resolve(block_offsets) as u32,
            st_size: symbol.size,
            st_info,
            st_other: 0,
            st_shndx: 1,
        }, writer)?;
        
        Ok(())
    };
    
    // serialize unnamed/automatically named/internally linked symbols
    for symbol in symbol_declarations.iter() {
        // 0x1: STB_LOCAL | STT_OBJECT
        write_symbol(&mut writer, &mut symbol_count, symbol, 0x1)?;
    }
    
    let last_local_symbol = symbol_count as u32;
    
    // weird unknown symbols (0x10 implies "external reference" (??))
    for _ in 0..12 {
        BinWrite::write(&SymbolHeader {
            st_name: 0,
            st_value: 0,
            st_size: 0,
            st_info: 0x10,
            st_other: 0,
            st_shndx: 0,
        }, &mut writer)?;
    }
    
    // serialize named symbols
    for symbol in named_symbols {
        println!("named symbol {symbol:?}");
        write_symbol(&mut writer, &mut symbol_count, &symbol, 0x11)?;
    }
    
    Ok((writer.into_inner(), last_local_symbol, strtab.into_inner()))
}


pub fn link_section_debug(section: &Section, symbols: &IndexMap<String, Symbol>) -> Result<Vec<u8>> {
    let mut writer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    
    if let Some(relocations) = section.relocations.as_ref() {
        let mut reader: Cursor<&[u8]> = Cursor::new(&section.content);
        
        while reader.position() < section.content.len() as u64 {
            if let Some(relocation) = relocations.get(&Pointer::current(&mut reader)?) {
                let symbol = symbols.get_index((relocation.info >> 8) as usize)
                    .ok_or_else(|| anyhow!("Could not find symbol at index {}", relocation.info >> 8))?
                    .1;
                
                writer.write_u32::<BigEndian>(symbol.offset() | 0x70000000)?;
                assert_eq!(reader.read_u32::<BigEndian>()?, 0);
            } else {
                let mut word: [u8; 4] = Default::default();
                let bytes_read = reader.read(&mut word)?;
                assert!(bytes_read == 4 || reader.position() >= section.content.len() as u64);
                writer.write_all(&word[..bytes_read])?;
            }
        }
    } else {
        writer.write_all(&section.content)?;
    }
    
    Ok(writer.into_inner())
}
