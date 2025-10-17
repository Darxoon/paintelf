use std::{fs, path::Path};

use anyhow::{anyhow, bail, Result};

use crate::{elf::{container::ElfContainer, Section}, formats::FileData, link_section_debug, reassemble_elf_container};

pub fn test_reserialize_directly(input_file_path: &Path, output_file: bool, original: &[u8], deserialized: &ElfContainer) -> Result<()> {
    let out_elf = deserialized.to_bytes()?;
    
    if output_file {
        let out_path = input_file_path.with_extension("elf2");
        
        fs::write(&out_path, &out_elf)?;
        println!("[debug] Directly re-serialized elf file to {}", out_path.file_name().unwrap().display());
    }
    
    assert_eq!(original, out_elf, "Directly re-serialized elf does not match");
    
    Ok(())
}

pub fn test_reserialize_from_content(input_file_path: &Path, output_file: bool, original: &ElfContainer, original_bytes: &[u8], deserialized: &FileData) -> Result<()> {
    // test all sections for matching directly
    // (apply relocations directly into section content to make this easier)
    let debug_elf = reassemble_elf_container(deserialized, true)?;
    
    let mut base_name = input_file_path.file_stem()
        .ok_or_else(|| anyhow!("Invalid file path {}", input_file_path.display()))?
        .to_owned();
    base_name.push("_match_test");
    let mut out_path = input_file_path.with_file_name(base_name);
    
    let mut write_section_debug: _ = |section: &Section| -> Result<()> {
        let name = section.name
            .strip_prefix(".")
            .unwrap_or(&section.name)
            .replace(".", "_");
        
        if output_file {
            out_path.set_extension(name);
            fs::write(&out_path, &section.content)?;
            println!("[debug] Wrote re-serialized section '{}' with potential relocations applied", section.name);
        }
        
        let Some(original_section) = original.get_section(&section.name) else {
            bail!("Elf file contains section '{}', which did not exist originally", section.name);
        };
        
        let original_content = link_section_debug(original_section, &original.symbols)?;
        
        assert!(&original_content == &section.content, "Re-serialized section '{}' does not match", section.name);
        
        if !output_file {
            println!("Section '{}' matches", section.name)
        }
        
        Ok(())
    };
    
    for section in debug_elf.content_sections.values() {
        if section.name.is_empty() {
            continue;
        }
        
        write_section_debug(section)?;
    }
    for section in debug_elf.meta_sections.values() {
        write_section_debug(section)?;
    }
    
    // test the entire elf file for matching
    let final_elf = reassemble_elf_container(deserialized, false)?;
    let final_elf_bytes = final_elf.to_bytes()?;
    
    if output_file {
        out_path.set_extension("elf");
        fs::write(&out_path, &final_elf_bytes)?;
        println!("[debug] Re-serialized elf file to {}", out_path.file_name().unwrap().display());
    }
    
    assert!(original_bytes == &final_elf_bytes, "Re-serialized elf file does not match");
    
    Ok(())
}
