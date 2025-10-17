use std::{
    env, fs,
    io::Cursor,
    panic,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow, bail};
use paintelf::{
    binutil::ElfReadDomain, elf::{container::ElfContainer, Section}, formats::{
        maplink::read_maplink, FileData
    }, link_section_debug, matching::{test_reserialize_directly, test_reserialize_from_content}, reassemble_elf_container
};

fn main() -> Result<()> {
    if !cfg!(debug_assertions) {
        panic::set_hook(Box::new(|info| {
            println!("An unexpected error occured! Please send the following message and \
            file this crashed on to the developer (Darxoon) so this can be fixed.\n{}", info);
        }));
    }
    
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
        reassemble_elf(&input_file_path)
    } else {
        disassemble_elf(&input_file_path, is_debug)
    }
}

fn reassemble_elf(input_file_path: &Path) -> Result<()> {
    let input_file = fs::read_to_string(input_file_path)?;
    let data: FileData = serde_yaml_bw::from_str(&input_file)?;
    
    let out_elf = reassemble_elf_container(&data, false)?;
    
    // write resulting elf
    let mut base_name = input_file_path.file_stem()
        .ok_or_else(|| anyhow!("Invalid file path {}", input_file_path.display()))?
        .to_owned();
    base_name.push("_modified.elf");
    let out_path = input_file_path.with_file_name(base_name);
    
    fs::write(&out_path, &out_elf.to_bytes()?)?;
    
    Ok(())
}

fn disassemble_elf(input_file_path: &Path, is_debug: bool) -> Result<()> {
    let elf_file_raw = fs::read(input_file_path)?;
    let mut reader: Cursor<&[u8]> = Cursor::new(&elf_file_raw);
    
    let elf_file = ElfContainer::from_reader(&mut reader)?;
    
    // get necessary sections
    let rodata_section = &elf_file.content_sections[".rodata"];
    let Some(rodata_relocations) = &rodata_section.relocations else {
        bail!("Could not find section .rela.rodata");
    };
    
    // parse maplink file
    let domain = ElfReadDomain::new(&rodata_section.content, rodata_relocations, &elf_file.symbols);
    
    let mut reader: Cursor<&[u8]> = Cursor::new(&rodata_section.content);
    let maplink = read_maplink(&mut reader, domain)?;
    let yaml = serde_yaml_bw::to_string(&maplink)?;
    
    let out_path = input_file_path.with_extension("yaml");
    fs::write(out_path, yaml)?;
    
    // debug features to facilitate matching re-serializing
    if is_debug {
        // apply relocations and output the result (debug only)
        let write_section_debug: _ = |section: &Section| -> Result<()> {
            let out_section: Vec<u8> = link_section_debug(section, &elf_file.symbols)?;
            let out_path = input_file_path.with_extension(section.name.strip_prefix(".").unwrap_or(&section.name));
            fs::write(out_path, &out_section)?;
            println!("[debug] Wrote section '{}' with potential relocations applied", section.name);
            Ok(())
        };
        
        for section in elf_file.content_sections.values() {
            if section.name.is_empty() {
                continue;
            }
            
            write_section_debug(section)?;
        }
        for section in elf_file.meta_sections.values() {
            write_section_debug(section)?;
        }
        
        // try re-serializing elf file without going through content
        test_reserialize_directly(input_file_path, &elf_file_raw, &elf_file)?;
        
        // try re-serializing elf file from just content
        test_reserialize_from_content(input_file_path, &elf_file, &elf_file_raw, &maplink)?;
    }
    
    Ok(())
}
