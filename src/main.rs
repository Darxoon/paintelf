use std::{cell::{Cell, RefCell}, collections::HashMap, env, fs, io::{Cursor, Read, Write}, path::{Path, PathBuf}, process::exit};

use anyhow::{anyhow, bail, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use indexmap::IndexMap;
use paintelf::{elf::{ElfContainer, Section, Symbol}, formats::{maplink::{read_maplink, write_maplink}, FileData}, util::pointer::Pointer, ElfReadDomain, ElfWriteDomain};
use vivibin::{WriteCtxImpl, WriteDomainExt};

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
    
    let result_buffer: Vec<u8> = match data {
        FileData::Maplink(maplink_areas) => {
            let string_map = RefCell::new(HashMap::new());
            let prev_string_len = Cell::new(0);
            let domain = ElfWriteDomain::new(&string_map, &prev_string_len);
            
            let mut ctx: WriteCtxImpl<ElfWriteDomain> = ElfWriteDomain::new_ctx();
            write_maplink(&mut ctx, domain, &maplink_areas)?;
            ctx.to_buffer(domain)?
        },
    };
    
    let mut base_name = input_file_path.file_stem()
        .ok_or_else(|| anyhow!("Invalid file path {}", input_file_path.display()))?
        .to_owned();
    base_name.push("_serialized.rodata");
    let out_path = input_file_path.with_file_name(base_name);
    
    fs::write(&out_path, &result_buffer)?;
    Ok(())
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
