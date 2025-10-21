use std::{borrow::Cow, io::SeekFrom};

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{CanRead, Readable, Reader, Writable};

use crate::{binutil::ElfReadDomain, formats::FileData, scoped_reader_pos, util::pointer::Pointer};

pub fn read_chr(reader: &mut impl Reader, domain: ElfReadDomain) -> Result<FileData> {
    // npcs
    let npc_count_symbol = domain.find_symbol("npcDataTableLen__Q2_4data3chr")?;
    reader.seek(SeekFrom::Start(npc_count_symbol.offset().into()))?;
    let npc_count = reader.read_u32::<BigEndian>()?;
    
    let npc_data_symbol = domain.find_symbol("npcDataTable__Q2_4data3chr")?;
    reader.seek(SeekFrom::Start(npc_data_symbol.offset().into()))?;
    
    let npc_data: Vec<NpcDefPtr> = (0..npc_count - 1)
        .map(|_| NpcDefPtr::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    // mobjs
    let mobj_count_symbol = domain.find_symbol("mobjDataTableLen__Q2_4data3chr")?;
    reader.seek(SeekFrom::Start(mobj_count_symbol.offset().into()))?;
    let mobj_count = reader.read_u32::<BigEndian>()?;
    
    let mobj_data_symbol = domain.find_symbol("mobjDataTable__Q2_4data3chr")?;
    reader.seek(SeekFrom::Start(mobj_data_symbol.offset().into()))?;
    
    let mobj_data: Vec<MobjDefPtr> = (0..mobj_count - 1)
        .map(|_| MobjDefPtr::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(FileData::Chr(ChrData {
        models: Cow::Borrowed("TODO"),
        kusya_models: Cow::Borrowed("TODO"),
        painky_models: Cow::Borrowed("TODO"),
        
        npc_data,
        mobj_data,
        player_data: Cow::Borrowed("TODO"),
        party_data: Cow::Borrowed("TODO"),
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChrData {
    pub models: Cow<'static, str>,
    pub kusya_models: Cow<'static, str>,
    pub painky_models: Cow<'static, str>,
    
    pub npc_data: Vec<NpcDefPtr>,
    pub mobj_data: Vec<MobjDefPtr>,
    pub player_data: Cow<'static, str>,
    pub party_data: Cow<'static, str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDefPtr(NpcDef);

impl<D: CanRead<String> + CanRead<Option<String>> + CanRead<Pointer>> Readable<D> for NpcDefPtr {
    fn from_reader<R: Reader>(reader: &mut R, domain: D) -> Result<Self> {
        let ptr: Pointer = domain.read(reader)?;
        scoped_reader_pos!(reader);
        
        reader.seek(SeekFrom::Start(ptr.into()))?;
        NpcDef::from_reader(reader, domain).map(NpcDefPtr)
    }
}

#[derive(Debug, Clone, Readable, Writable, Serialize, Deserialize)]
pub struct NpcDef {
    #[require_domain]
    pub id: String,
    pub description: String,
    #[require_domain]
    pub model_ptr: Pointer,
    pub field_0xc: String,
    pub field_0x10: u32,
    pub field_0x14: String,
    #[require_domain]
    pub field_0x18: Option<String>,
    pub field_0x1c: Option<String>,
    pub main_function: Option<String>,
    pub field_0x24: u32,
    pub action_function: Option<String>,
    pub field_0x2c: Option<String>,
    pub field_0x30: Option<String>,
    pub field_0x34: Option<String>,
    pub field_0x38: Option<String>,
    pub field_0x3c: Option<String>,
    pub field_0x40: Option<String>,
    pub field_0x44: u32,
    pub field_0x48: Option<String>,
    pub field_0x4c: Option<String>,
    pub field_0x50: f32,
    pub field_0x54: f32,
    pub field_0x58: u32,
    pub field_0x5c: u32,
    pub field_0x60: u32,
    pub field_0x64: u32,
    pub field_0x68: Option<String>,
    pub field_0x6c: Option<String>,
    pub field_0x70: Option<String>,
    pub field_0x74: u32,
    pub field_0x78: Option<String>,
    pub field_0x7c: Option<String>,
    pub field_0x80: Option<String>,
    pub field_0x84: Option<String>,
    pub field_0x88: Option<String>,
    pub field_0x8c: f32,
    pub field_0x90: u32,
    pub field_0x94: Option<String>,
    pub field_0x98: Option<String>,
    pub field_0x9c: Option<String>,
    pub field_0xa0: f32,
    pub field_0xa4: Option<String>,
    pub field_0xa8: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobjDefPtr(MobjDef);

impl<D: CanRead<String> + CanRead<Option<String>> + CanRead<Pointer>> Readable<D> for MobjDefPtr {
    fn from_reader<R: Reader>(reader: &mut R, domain: D) -> Result<Self> {
        let ptr: Pointer = domain.read(reader)?;
        scoped_reader_pos!(reader);
        
        reader.seek(SeekFrom::Start(ptr.into()))?;
        MobjDef::from_reader(reader, domain).map(MobjDefPtr)
    }
}

#[derive(Debug, Clone, Readable, Writable, Serialize, Deserialize)]
pub struct MobjDef {
    #[require_domain]
    pub id: String,
    pub description: String,
    #[require_domain]
    pub model_ptr: Pointer,
    pub field_0xc: u32,
    pub field_0x10: String,
    pub field_0x14: String,
    pub field_0x18: String,
    pub field_0x1c: String,
    pub field_0x20: u32,
    pub field_0x24: u32,
    #[require_domain]
    pub field_0x28: Option<String>,
}
