use std::{
    env, fs,
    io::Cursor,
    panic,
    path::{Path, PathBuf},
    process::exit,
};

use anyhow::{Result, anyhow, bail};
use indoc::printdoc;
use paintelf::{
    binutil::{ElfReadDomain, UnitCategory},
    elf::{Section, container::ElfContainer},
    formats::{
        FileData, FileType, chr::read_chr, dispos::read_dispos, lct::read_lct, mapid::read_mapid,
        maplink::read_maplink, shop::read_shops,
    },
    link_section_debug,
    matching::{test_reserialize_directly, test_reserialize_from_content},
    reassemble_elf_container,
};

fn main() -> Result<()> {
    if !cfg!(debug_assertions) {
        panic::set_hook(Box::new(|info| {
            println!("An unexpected error occured! Please send the following message and \
            file this crashed on to the developer (Darxoon) so this can be fixed.\n{}", info);
        }));
    }
    
    let mut argv = env::args();
    
    let mut help = false;
    let mut is_debug = false;
    let mut file_type = None;
    let mut positional = Vec::new();
    
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                help = true;
            },
            "-d" | "--debug" => {
                is_debug = true;
            },
            "-t" | "--type" => {
                if file_type.is_some() {
                    bail!("Cannot define --type argument twice");
                }
                
                let name = argv.next()
                    .ok_or_else(|| anyhow!("Expected one of these file types: {}", FileType::ALL_VALUES.join(", ")))?;
                file_type = FileType::from_string(&name);
                
                if file_type.is_none() {
                    bail!("Unknown file type {name}, expected one of these: {}", FileType::ALL_VALUES.join(", "));
                }
            },
            _ => {
                positional.push(arg);
            },
        }
    }
    
    if positional.len() != 2 || help {
        printdoc!("
            Usage: paintelf [options] <path to decompressed .elf>
            
            Options:
              -h | --help: Shows this text.
              -t | --type <{}>: Type of the elf file
            
            (Supported elf files are: data_fld_maplink.elf, data_shop.elf)
            ",
            FileType::ALL_VALUES.join("|")
        );
        return Ok(());
    }
    
    let input_file_path_str = positional[1].as_str();
    let input_file_path = PathBuf::from(input_file_path_str);
    
    if input_file_path_str.ends_with(".yaml") {
        reassemble_elf(&input_file_path, is_debug)
    } else {
        let Some(file_type) = file_type else {
            bail!("Expected one of these file types to be passed with '--type' argument: {}",
                FileType::ALL_VALUES.join(", "));
        };
        
        disassemble_elf(&input_file_path, file_type, is_debug)
    }
}

fn reassemble_elf(input_file_path: &Path, is_debug: bool) -> Result<()> {
    let input_file = fs::read_to_string(input_file_path)?;
    let data: FileData = serde_yaml_bw::from_str(&input_file)?;
    
    if matches!(data, FileData::Dispos(_) | FileData::Chr(_)) && !is_debug {
        eprintln!("Rebuilding data_dispos.elf is not supported yet!");
        exit(1);
    }
    
    let out_elf = reassemble_elf_container::<UnitCategory>(&data, false)?;
    
    // write resulting elf
    let mut base_name = input_file_path.file_stem()
        .ok_or_else(|| anyhow!("Invalid file path {}", input_file_path.display()))?
        .to_owned();
    base_name.push("_modified.elf");
    let out_path = input_file_path.with_file_name(base_name);
    
    fs::write(&out_path, &out_elf.to_bytes()?)?;
    
    Ok(())
}

fn disassemble_elf(input_file_path: &Path, file_type: FileType, is_debug: bool) -> Result<()> {
    let elf_file_raw = fs::read(input_file_path)?;
    let mut reader: Cursor<&[u8]> = Cursor::new(&elf_file_raw);
    
    let elf_file = ElfContainer::from_reader(&mut reader)?;
    
    // get necessary sections
    let content_section = &elf_file.content_sections[file_type.content_section_name()];
    let Some(content_relocations) = &content_section.relocations else {
        bail!("Could not find section .rela{}", file_type.content_section_name());
    };
    
    let rodata_section = match file_type.content_section_name() {
        ".rodata" => content_section,
        _ => &elf_file.content_sections[".rodata"],
    };
    
    // parse maplink file
    let domain = ElfReadDomain::new(&rodata_section.content, content_relocations, &elf_file.symbols);
    
    let mut reader: Cursor<&[u8]> = Cursor::new(&content_section.content);
    let maplink = match file_type {
        FileType::Maplink => read_maplink(&mut reader, domain)?,
        FileType::MapId => read_mapid(&mut reader, domain)?,
        FileType::Shop => read_shops(&mut reader, domain)?,
        FileType::Dispos => read_dispos(&mut reader, domain)?,
        FileType::Chr => read_chr(&mut reader, domain)?,
        FileType::Lct => read_lct(&mut reader, domain)?,
    };
    
    let yaml = serde_yaml_bw::to_string(&maplink)?;
    
    let out_path = input_file_path.with_extension("yaml");
    fs::write(out_path, yaml)?;
    
    // debug features to facilitate matching re-serializing
    if is_debug {
        // apply relocations and output the result (debug only)
        let write_section_debug = |section: &Section| -> Result<()> {
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
        test_reserialize_directly(input_file_path, true, &elf_file_raw, &elf_file)?;
        
        // try re-serializing elf file from just content
        test_reserialize_from_content(input_file_path, true, &elf_file, &elf_file_raw, &maplink)?;
    }
    
    Ok(())
}
