use std::{fs, io::Cursor, path::Path};

use crate::{binutil::ElfReadDomain, elf::container::ElfContainer, formats::maplink::read_maplink, matching::{test_reserialize_directly, test_reserialize_from_content}};

#[test]
fn reserialize_maplink_directly() {
    let path = Path::new("test/data_fld_maplink.elf");
    
    if !path.is_file() {
        panic!("File {} does not exist", path.display());
    }
    
    let input_file = fs::read(path).unwrap();
    let mut reader: Cursor<&[u8]> = Cursor::new(&input_file);
    
    let elf_file = ElfContainer::from_reader(&mut reader).unwrap();
    
    println!("Attempting to re-serialize existing ElfContainer");
    test_reserialize_directly(path, false, &input_file, &elf_file).unwrap();
}

#[test]
fn reserialize_maplink_from_content() {
    let path = Path::new("test/data_fld_maplink.elf");
    
    if !path.is_file() {
        panic!("File {} does not exist", path.display());
    }
    
    let input_file = fs::read(path).unwrap();
    let mut reader: Cursor<&[u8]> = Cursor::new(&input_file);
    
    let elf_file = ElfContainer::from_reader(&mut reader).unwrap();
    
    // get necessary sections
    let rodata_section = &elf_file.content_sections[".rodata"];
    let Some(rodata_relocations) = &rodata_section.relocations else {
        panic!("Could not find section .rela.rodata");
    };
    
    // parse maplink file
    let domain = ElfReadDomain::new(&rodata_section.content, rodata_relocations, &elf_file.symbols);
    
    let mut reader: Cursor<&[u8]> = Cursor::new(&rodata_section.content);
    let data = read_maplink(&mut reader, domain).unwrap();
    
    println!("Attempting to re-serialize data from content");
    test_reserialize_from_content(path, false, &elf_file, &input_file, &data).unwrap();
    
}