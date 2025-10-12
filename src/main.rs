use std::{cell::{Cell, RefCell}, cmp::Ordering, collections::HashMap, env, fs, io::{Cursor, Read, Write}, path::{Path, PathBuf}, process::exit};

use anyhow::{anyhow, bail, Error, Result};
use binrw::BinWrite;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use paintelf::{
    elf::{ElfContainer, Relocation, Section, Symbol, SymbolHeader, SymbolNameGenerator}, formats::{maplink::{read_maplink, write_maplink}, FileData}, util::pointer::Pointer, ElfReadDomain, ElfWriteDomain, RelDeclaration, SymbolDeclaration, SymbolName
};
use vivibin::{HeapToken, WriteCtxImpl, WriteDomainExt};

fn main() -> Result<()> {
    let argv = env::args().collect::<Vec<_>>();
    
    if argv.len() < 2 || argv[1] == "-h" || argv[1] == "--help" {
        println!("Usage: paintelf <path to decompressed .elf>");
        println!("(Supported elf files are: data_fld_maplink.elf)");
        return Ok(());
    }
    
    let (is_debug, input_file_path_str) = if argv[1] == "-d" || argv[1] == "--debug" {
        println!("Debug!");
        (true, argv[2].as_str())
    } else {
        (false, argv[1].as_str())
    };
    
    let input_file_path = PathBuf::from(input_file_path_str);
    
    if input_file_path_str.ends_with(".yaml") {
        // Reassemble yaml to elf
        if !is_debug {
            println!("Reassembling yaml files currently does not work yet.");
            println!("If you still want to access the currently experimental implementation, \
            pass the '--debug' flag before the file path (don't be surprised if it doesn't work).");
            exit(1);
        }
        
        reassemble_elf(&input_file_path)
    } else {
        disassemble_elf(&input_file_path, is_debug)
    }
}

fn reassemble_elf(input_file_path: &Path) -> Result<()> {
    let input_file = fs::read_to_string(input_file_path)?;
    let data: FileData = serde_yaml_bw::from_str(&input_file)?;
    
    let mut block_offsets = Vec::new();
    
    let (result_buffer, mut symbol_declarations, relocations) = match data {
        FileData::Maplink(maplink_areas) => {
            let string_map = RefCell::new(HashMap::new());
            let symbol_declarations = RefCell::new(IndexMap::new());
            let relocations = RefCell::new(Vec::new());
            let prev_string_len = Cell::new(0);
            let domain = ElfWriteDomain::new(&string_map, &symbol_declarations, &relocations, &prev_string_len);
            
            let mut ctx: WriteCtxImpl<ElfWriteDomain> = ElfWriteDomain::new_ctx();
            write_maplink(&mut ctx, domain, &maplink_areas)?;
            
            (
                ctx.to_buffer(domain, Some(&mut block_offsets))?,
                symbol_declarations.into_inner(),
                relocations.into_inner(),
            )
        },
    };
    
    let mut base_name = input_file_path.file_stem()
        .ok_or_else(|| anyhow!("Invalid file path {}", input_file_path.display()))?
        .to_owned();
    base_name.push("_serialized.rodata");
    let out_path = input_file_path.with_file_name(base_name);
    
    let symbol_indices = write_symtab(&out_path, &block_offsets, &mut symbol_declarations)?;
    write_relocations(&out_path, &block_offsets, &symbol_indices, &relocations)?;
    
    fs::write(&out_path, &result_buffer)?;
    Ok(())
}

fn write_relocations(
    out_path: &Path,
    block_offsets: &[usize],
    symbol_indices: &HashMap<HeapToken, usize>,
    relocations: &[RelDeclaration],
) -> Result<()> {
    let mut writer = Cursor::new(Vec::new());
    
    for relocation in relocations {
        let base_location = relocation.base_location.resolve(block_offsets);
        let symbol_idx = symbol_indices.get(&relocation.target_location).unwrap();
        
        let raw = Relocation::new(base_location as u32, (symbol_idx << 8 | 1) as u32, 0);
        raw.write(&mut writer)?;
    }
    
    let out_rela_rodata: Vec<u8> = writer.into_inner();
    let out_path = out_path.with_extension("rela.rodata");
    fs::write(&out_path, &out_rela_rodata).map_err(Error::from)
}

fn write_symtab(
    out_path: &Path,
    block_offsets: &[usize],
    symbol_declarations: &mut IndexMap<HeapToken, SymbolDeclaration>,
) -> Result<HashMap<HeapToken, usize>> {
    // name unnamed internal symbols
    {
        let mut symbol_name_gen = SymbolNameGenerator::new();
        
        let mut symbols: Vec<(char, &mut SymbolDeclaration)> = symbol_declarations
            .values_mut()
            .flat_map(|symbol| {
                match symbol.name {
                    SymbolName::Internal(initial_char) => Some((initial_char, symbol)),
                    _ => None
                }
            })
            .collect::<Vec<_>>();
        
        symbols.sort_by_key(|(_, symbol)| symbol.offset);
        
        for (initial_char, symbol) in symbols {
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
        
        let mut symbols: Vec<&mut SymbolDeclaration> = symbol_declarations
            .values_mut()
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
    let mut symbol_indices = HashMap::new();
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
    let named_symbols: Vec<(HeapToken, SymbolDeclaration)> = symbol_declarations
        .extract_if(.., |_, symbol| !symbol.name.is_internal())
        .collect::<Vec<_>>();
    
    symbol_declarations.sort_by_key(|_, symbol| symbol.offset.resolve(block_offsets));
    
    let mut strtab = Cursor::new(Vec::new());
    strtab.write(b"\0data_fld_maplink.cpp\0")?;
    
    let mut write_symbol: _ = |writer: &mut Cursor<Vec<u8>>, token: HeapToken, symbol: &SymbolDeclaration, st_info: u8| -> Result<()> {
        // serialize name
        let name_ptr = if let Some(symbol_name) = symbol.name.as_str() {
            let name_ptr = Pointer::current(&mut strtab)?;
            strtab.write(symbol_name.as_bytes())?;
            strtab.write_u8(0)?;
            name_ptr.into()
        } else {
            0u32
        };
        
        // serialize symbol
        symbol_indices.insert(token, symbol_count);
        symbol_count += 1;
        BinWrite::write(&SymbolHeader {
            st_name: name_ptr,
            st_value: token.resolve(block_offsets) as u32,
            st_size: symbol.size,
            st_info,
            st_other: 0,
            st_shndx: 1,
        }, writer)?;
        
        Ok(())
    };
    
    // serialize unnamed/automatically named/internally linked symbols
    for (token, symbol) in symbol_declarations.iter() {
        write_symbol(&mut writer, *token, symbol, 0x1)?;
    }
    
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
    for (token, symbol) in named_symbols {
        println!("named symbol {symbol:?}");
        write_symbol(&mut writer, token, &symbol, 0x11)?;
    }
    
    let out_symtab: Vec<u8> = writer.into_inner();
    let out_path = out_path.with_extension("symtab");
    fs::write(&out_path, &out_symtab)?;
    
    let out_strtab: Vec<u8> = strtab.into_inner();
    let out_path = out_path.with_extension("strtab");
    fs::write(&out_path, &out_strtab)?;
    Ok(symbol_indices)
}

fn disassemble_elf(input_file_path: &Path, is_debug: bool) -> Result<()> {
    let elf_file_raw = fs::read(input_file_path)?;
    let mut reader = Cursor::new(elf_file_raw.as_slice());
    
    let elf_file = ElfContainer::from_reader(&mut reader)?;
    
    // get necessary sections
    let rodata_section = &elf_file.sections[".rodata"];
    let Some(rodata_relocations) = &rodata_section.relocations else {
        bail!("Could not find section .rela.rodata");
    };
    
    // apply relocations and output the result (debug only)
    if is_debug {
        let write_section_debug: _ = |section_name: &str| -> Result<()> {
            let section = &elf_file.sections[section_name];
            let out_section: Vec<u8> = get_section_linked(section, &elf_file.symbols)?;
            let out_path = input_file_path.with_extension(section_name.strip_prefix(".").unwrap_or(section_name));
            fs::write(out_path, &out_section)?;
            println!("Wrote section '{section_name}' with potential relocations applied");
            Ok(())
        };
        
        write_section_debug(".rodata")?;
        write_section_debug(".rela.rodata")?;
        write_section_debug(".symtab")?;
        write_section_debug(".strtab")?;
    }
    
    // parse maplink file
    let domain = ElfReadDomain::new(&rodata_section.content, &rodata_relocations, &elf_file.symbols);
    
    let mut reader: Cursor<&[u8]> = Cursor::new(&rodata_section.content);
    let maplink = read_maplink(&mut reader, domain)?;
    let yaml = serde_yaml_bw::to_string(&maplink)?;
    
    let out_path = input_file_path.with_extension("yaml");
    fs::write(out_path, yaml)?;
    Ok(())
}

fn get_section_linked(section: &Section, symbols: &IndexMap<String, Symbol>) -> Result<Vec<u8>> {
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
