use std::{env, fs, io::Cursor, path::PathBuf};

use anyhow::{bail, Result};
use paintelf::{elf::ElfContainer, formats::maplink::read_maplink, ReadContext};

fn main() -> Result<()> {
    let input_file_path = env::args().nth(1);
    
    if input_file_path.as_deref().is_none_or(|value| value == "-h" || value == "--help") {
        println!("Usage: paintelf <path to decompressed .elf>");
        println!("(Supported elf files are: data_fld_maplink.elf)");
        return Ok(());
    }
    
    let input_file_path = PathBuf::from(input_file_path.unwrap());
    
    let elf_file_raw = fs::read(&input_file_path)?;
    let mut reader = Cursor::new(elf_file_raw.as_slice());
    
    let elf_file = ElfContainer::from_reader(&mut reader)?;
    
    // parse maplink file
    let rodata_section = &elf_file.sections[".rodata"];
    let Some(rodata_relocations) = &rodata_section.relocations else {
        bail!("Could not find section .rela.rodata");
    };
    
    let reader: Cursor<&[u8]> = Cursor::new(&rodata_section.content);
    let mut ctx = ReadContext::new(reader, &rodata_section.content, &rodata_relocations, &elf_file.symbols);
    
    let maplink = read_maplink(&mut ctx)?;
    let yaml = serde_yaml_bw::to_string(&maplink)?;
    
    let out_path = input_file_path.with_extension("yaml");
    fs::write(out_path, yaml)?;
    
    Ok(())
}
