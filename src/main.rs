use std::{env, fs, io::{Cursor, Read, Write}, path::PathBuf};

use anyhow::{bail, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use paintelf::{elf::ElfContainer, formats::maplink::read_maplink, util::pointer::Pointer, ElfDomain};

fn main() -> Result<()> {
    let argv = env::args().collect::<Vec<_>>();
    
    if argv.len() < 2 || argv[1] == "-h" || argv[1] == "--help" {
        println!("Usage: paintelf <path to decompressed .elf>");
        println!("(Supported elf files are: data_fld_maplink.elf)");
        return Ok(());
    }
    
    let (is_debug, input_file_path) = if argv[1] == "-d" || argv[1] == "--debug" {
        println!("Debug!");
        (true, argv[2].as_str())
    } else {
        (false, argv[1].as_str())
    };
    
    let input_file_path = PathBuf::from(input_file_path);
    
    let elf_file_raw = fs::read(&input_file_path)?;
    let mut reader = Cursor::new(elf_file_raw.as_slice());
    
    let elf_file = ElfContainer::from_reader(&mut reader)?;
    
    // get necessary sections
    let rodata_section = &elf_file.sections[".rodata"];
    let Some(rodata_relocations) = &rodata_section.relocations else {
        bail!("Could not find section .rela.rodata");
    };
    
    // apply relocations and output the result (debug only)
    if is_debug {
        let mut reader: Cursor<&[u8]> = Cursor::new(&rodata_section.content);
        let mut writer: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        
        while reader.position() < rodata_section.content.len() as u64 {
            if let Some(relocation) = rodata_relocations.get(&Pointer::current(&mut reader)?) {
                writer.write_u32::<BigEndian>(relocation.addend | 0x70000000)?;
                assert_eq!(reader.read_u32::<BigEndian>()?, 0);
            } else {
                let mut word: [u8; 4] = Default::default();
                let bytes_read = reader.read(&mut word)?;
                assert!(bytes_read == 4 || reader.position() >= rodata_section.content.len() as u64);
                writer.write_all(&word)?;
            }
        }
        
        let out_path = input_file_path.with_extension("rodata");
        fs::write(out_path, writer.get_ref())?;
        println!("Wrote .rodata with relocations applied");
    }
    
    // parse maplink file
    let domain = ElfDomain::new(&rodata_section.content, &rodata_relocations, &elf_file.symbols);
    
    let mut reader: Cursor<&[u8]> = Cursor::new(&rodata_section.content);
    let maplink = read_maplink(&mut reader, domain)?;
    let yaml = serde_yaml_bw::to_string(&maplink)?;
    
    let out_path = input_file_path.with_extension("yaml");
    fs::write(out_path, yaml)?;
    
    Ok(())
}
