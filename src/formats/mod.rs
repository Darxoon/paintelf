use core::fmt::{self, Display};
use std::borrow::Cow;

use miniserde::{de::Visitor, make_place, ser::Fragment, Deserialize, Serialize};

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

#[derive(Clone, Debug)]
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

// deserialize
make_place!(Place);

impl Visitor for Place<FileData> {
    fn map(&mut self) -> miniserde::Result<Box<dyn miniserde::de::Map + '_>> {
        Ok(Box::new(FileDataBuilder {
            maplink: None,
            mapid: None,
            shop: None,
            dispos: None,
            out: &mut self.out,
        }))
    }
}

struct FileDataBuilder<'a> {
    maplink: Option<Vec<MaplinkArea>>,
    mapid: Option<Vec<MapGroup>>,
    shop: Option<Vec<Shop>>,
    dispos: Option<Vec<DisposArea>>,
    
    out: &'a mut Option<FileData>,
}

impl<'a> miniserde::de::Map for FileDataBuilder<'a> {
    fn key(&mut self, k: &str) -> miniserde::Result<&mut dyn Visitor> {
        match k {
            "Maplink" => Ok(Deserialize::begin(&mut self.mapid)),
            "MapId" => Ok(Deserialize::begin(&mut self.mapid)),
            "Shop" => Ok(Deserialize::begin(&mut self.shop)),
            "Dispos" => Ok(Deserialize::begin(&mut self.dispos)),
            _ => Ok(<dyn Visitor>::ignore()),
        }
    }

    fn finish(&mut self) -> miniserde::Result<()> {
        if let Some(val) = self.maplink.take() {
            *self.out = Some(FileData::Maplink(val));
            return Ok(());
        }
        if let Some(val) = self.mapid.take() {
            *self.out = Some(FileData::MapId(val));
            return Ok(());
        }
        if let Some(val) = self.shop.take() {
            *self.out = Some(FileData::Shop(val));
            return Ok(());
        }
        if let Some(val) = self.dispos.take() {
            *self.out = Some(FileData::Dispos(val));
            return Ok(());
        }
        Err(miniserde::Error)
    }
}

impl Deserialize for FileData {
    fn begin(out: &mut Option<Self>) -> &mut dyn Visitor {
        Place::new(out)
    }
}

// serialize
struct FileDataStream<'a> {
    data: &'a FileData,
    state: bool,
}

impl<'a> miniserde::ser::Map for FileDataStream<'a> {
    fn next(&mut self) -> Option<(Cow<'_, str>, &dyn Serialize)> {
        if self.state { return None; }
        self.state = true;
        Some(match self.data {
            FileData::Maplink(maplink_areas) => (Cow::Borrowed("Maplink"), maplink_areas),
            FileData::MapId(map_groups) => (Cow::Borrowed("MapId"), map_groups),
            FileData::Shop(shops) => (Cow::Borrowed("Shop"), shops),
            FileData::Dispos(dispos_areas) => (Cow::Borrowed("Dispos"), dispos_areas),
        })
    }
}

impl Serialize for FileData {
    fn begin(&self) -> miniserde::ser::Fragment<'_> {
        Fragment::Map(Box::new(FileDataStream {
            data: self,
            state: false,
        }))
    }
}

