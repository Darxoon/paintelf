use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanWriteSliceWithArgs, CanWriteWithArgs, Readable, Reader, Writable, WriteCtx, WriteDomainExt,
    WriteSliceWithArgsFallbackExt,
};

use crate::{
    SymbolName,
    binutil::{ElfCategory, ElfReadDomain, ElfWriteDomain, WriteStringArgs},
    formats::FileData,
};

pub fn read_mapid(reader: &mut impl Reader, domain: ElfReadDomain) -> Result<FileData> {
    let data_count_symbol = domain.find_symbol("dataCount__Q3_4data3fld5mapid")?;
    reader.seek(SeekFrom::Start(data_count_symbol.offset().into()))?;
    let data_count = reader.read_u32::<BigEndian>()?;
    
    let datas_symbol = domain.find_symbol("datas__Q3_4data3fld5mapid")?;
    reader.seek(SeekFrom::Start(datas_symbol.offset().into()))?;
    
    let areas: Vec<MapGroup> = (0..data_count)
        .map(|_| MapGroup::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(FileData::MapId(areas))
}

pub fn write_mapid<C: ElfCategory>(ctx: &mut impl WriteCtx, domain: &mut ElfWriteDomain<C>, areas: &[MapGroup]) -> Result<()> {
    domain.write_symbol(ctx, "dataCount__Q3_4data3fld5mapid", |domain, ctx| {
        domain.write_fallback::<u32>(ctx, &(areas.len() as u32))
    })?;
    
    domain.write_symbol(ctx, "datas__Q3_4data3fld5mapid", |domain, ctx| {
        for area in areas {
            area.to_writer(ctx, domain)?;
        }
        Ok(())
    })?;
    
    Ok(())
}

#[derive(Clone, Debug, Readable, Serialize, Deserialize)]
pub struct MapGroup {
    #[require_domain]
    pub id: String,
    pub maps: Vec<MapDefinition>,
}

impl<D> Writable<D> for MapGroup
where
    D: CanWriteWithArgs<String, WriteStringArgs> + CanWriteSliceWithArgs<MapDefinition, Option<SymbolName>>,
{
    fn to_writer_unboxed(&self, ctx: &mut impl vivibin::WriteCtx, domain: &mut D) -> Result<()> {
        // TODO: turning off deduplication is a hack, figure out serialization order better
        domain.write_args(ctx, &self.id, WriteStringArgs { deduplicate: false })?;
        domain.write_slice_args_fallback(ctx, &self.maps, Some(SymbolName::InternalNamed(self.id.clone())))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Readable, Writable, Serialize, Deserialize)]
pub struct MapDefinition {
    #[require_domain]
    pub group_id: String,
    pub map_id: String,
    pub level_id: String,
    pub description: String,
    pub field_0x10: String,
    pub field_0x14: String,
    pub field_0x18: String,
    pub field_0x1c: String,
    pub field_0x20: u32,
    pub field_0x24: String,
    pub field_0x28: String,
    pub field_0x2c: u32,
    pub field_0x30: u32,
    pub field_0x34: u32,
    pub field_0x38: u32,
    pub field_0x3c: u32,
    pub field_0x40: u32,
    pub field_0x44: u32,
    pub field_0x48: u32,
    pub field_0x4c: u32,
    pub field_0x50: u32,
    pub field_0x54: String,
    pub field_0x58: String,
    pub field_0x5c: String,
    pub field_0x60: String,
    pub field_0x64: String,
    pub field_0x68: String,
    pub field_0x6c: String,
    pub field_0x70: String,
    pub field_0x74: String,
    pub field_0x78: String,
    pub field_0x7c: String,
}
