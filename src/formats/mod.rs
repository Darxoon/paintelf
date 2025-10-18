use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::formats::{mapid::MapGroup, maplink::MaplinkArea, shop::Shop};

pub mod mapid;
pub mod maplink;
pub mod shop;

#[derive(Clone, Copy, Debug)]
pub enum FileType {
    Maplink,
    MapId,
    Shop,
}

impl FileType {
    pub const ALL_VALUES: [&str; 3] = ["maplink", "mapid", "shop"];
    
    pub fn from_string(string: &str) -> Option<FileType> {
        match string {
            "maplink" => Some(FileType::Maplink),
            "mapid" => Some(FileType::MapId),
            "shop" => Some(FileType::Shop),
            _ => None,
        }
    }
}

impl Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FileType::Maplink => "maplink",
            FileType::MapId => "mapid",
            FileType::Shop => "shop",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileData {
    Maplink(Vec<MaplinkArea>),
    MapId(Vec<MapGroup>),
    Shop(Vec<Shop>),
}

impl FileData {
    pub fn cpp_file_name(&self) -> &'static str {
        match self {
            FileData::Maplink(_) => "data_fld_maplink.cpp",
            FileData::MapId(_) => "data_fld_mapid.cpp",
            FileData::Shop(_) => "data_shop.cpp",
        }
    }
    
    pub fn elf_ident_padding_unk(&self) -> u32 {
        match self {
            FileData::Maplink(_) => 1,
            _ => 0,
        }
    }
}
