use core::fmt::{self, Display};

use serde::{Deserialize, Serialize};

use crate::formats::{dispos::DisposArea, mapid::MapGroup, maplink::MaplinkArea, shop::Shop};

pub mod dispos;
pub mod mapid;
pub mod maplink;
pub mod shop;

#[derive(Clone, Copy, Debug)]
pub enum FileType {
    Maplink,
    MapId,
    Shop,
    Dispos,
}

impl FileType {
    pub const ALL_VALUES: [&str; 4] = ["maplink", "mapid", "shop", "dispos"];
    
    pub fn from_string(string: &str) -> Option<FileType> {
        match string {
            "maplink" => Some(FileType::Maplink),
            "mapid" => Some(FileType::MapId),
            "shop" => Some(FileType::Shop),
            "dispos" => Some(FileType::Dispos),
            _ => None,
        }
    }
    
    pub fn content_section_name(self) -> &'static str {
        match self {
            FileType::Dispos => ".data",
            _ => ".rodata",
        }
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FileType::Maplink => "maplink",
            FileType::MapId => "mapid",
            FileType::Shop => "shop",
            FileType::Dispos => "dispos",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileData {
    Maplink(Vec<MaplinkArea>),
    MapId(Vec<MapGroup>),
    Shop(Vec<Shop>),
    Dispos(Vec<DisposArea>),
}

impl FileData {
    pub fn cpp_file_name(&self) -> &'static str {
        match self {
            FileData::Maplink(_) => "data_fld_maplink.cpp",
            FileData::MapId(_) => "data_fld_mapid.cpp",
            FileData::Shop(_) => "data_shop.cpp",
            FileData::Dispos(_) => todo!(),
        }
    }
    
    pub fn elf_ident_padding_unk(&self) -> u32 {
        match self {
            FileData::Maplink(_) => 1,
            _ => 0,
        }
    }
    
    // this is so confusing
    pub fn string_dedup_size(&self) -> u64 {
        match self {
            FileData::Maplink(_) => 0xc32c,
            FileData::MapId(_) => 0xa028,
            // ?
            _ => 0xc32c,
        }
    }
}
