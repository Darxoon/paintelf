use std::{ffi::OsStr, fs, io::Cursor, path::Path};

use anyhow::Result;

use crate::{
    binutil::ElfReadDomain,
    elf::container::ElfContainer,
    formats::{FileData, maplink::read_maplink, shop::read_shops},
    matching::{test_reserialize_directly, test_reserialize_from_content},
};

fn reserialize_any_directly<S: AsRef<OsStr> + ?Sized>(path: &S) {
    let path = Path::new(path);
    
    if !path.is_file() {
        panic!("File {} does not exist", path.display());
    }
    
    let input_file = fs::read(path).unwrap();
    let mut reader: Cursor<&[u8]> = Cursor::new(&input_file);
    
    let elf_file = ElfContainer::from_reader(&mut reader).unwrap();
    
    println!("Attempting to re-serialize existing ElfContainer");
    test_reserialize_directly(path, false, &input_file, &elf_file).unwrap();
}

fn reserialize_any_from_content<S: AsRef<OsStr> + ?Sized>(
    path: &S,
    content_callback: impl FnOnce(&mut Cursor<&[u8]>, ElfReadDomain) -> Result<FileData>,
) {
    let path = Path::new(path);
    
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
    let domain = ElfReadDomain::new(
        &rodata_section.content,
        rodata_relocations,
        &elf_file.symbols,
    );
    
    let mut reader: Cursor<&[u8]> = Cursor::new(&rodata_section.content);
    let data = content_callback(&mut reader, domain).unwrap();
    
    println!("Attempting to re-serialize data from content");
    test_reserialize_from_content(path, false, &elf_file, &input_file, &data).unwrap();
}

#[test]
fn reserialize_maplink_directly() {
    reserialize_any_directly("test/data_fld_maplink.elf");
}

#[test]
fn reserialize_maplink_from_content() {
    reserialize_any_from_content("test/data_fld_maplink.elf", |reader, domain| {
        read_maplink(reader, domain)
    });
}

#[test]
fn reserialize_shop_directly() {
    reserialize_any_directly("test/data_shop.elf");
}

#[test]
fn reserialize_shop_from_content() {
    reserialize_any_from_content("test/data_shop.elf", |reader, domain| {
        read_shops(reader, domain)
    });
}
