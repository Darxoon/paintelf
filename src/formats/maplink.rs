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
    binutil::{ElfReadDomain, ElfWriteDomain, WriteStringArgs},
    formats::FileData,
};

pub fn read_maplink(reader: &mut impl Reader, domain: ElfReadDomain) -> Result<FileData> {
    let data_count_symbol = domain.find_symbol("dataCount__Q3_4data3fld7maplink")?;
    reader.seek(SeekFrom::Start(data_count_symbol.offset().into()))?;
    let data_count = reader.read_u32::<BigEndian>()?;
    
    let datas_symbol = domain.find_symbol("datas__Q3_4data3fld7maplink")?;
    reader.seek(SeekFrom::Start(datas_symbol.offset().into()))?;
    
    let areas: Vec<MaplinkArea> = (0..data_count)
        .map(|_| MaplinkArea::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(FileData::Maplink(areas))
}

pub fn write_maplink(ctx: &mut impl WriteCtx, domain: &mut ElfWriteDomain, areas: &[MaplinkArea]) -> Result<()> {
    domain.write_symbol(ctx, "dataCount__Q3_4data3fld7maplink", |domain, ctx| {
        domain.write_fallback::<u32>(ctx, &(areas.len() as u32))
    })?;
    
    domain.write_symbol(ctx, "datas__Q3_4data3fld7maplink", |domain, ctx| {
        for area in areas {
            area.to_writer(ctx, domain)?;
        }
        Ok(())
    })?;
    
    Ok(())
}

#[derive(Clone, Debug, Readable, Serialize, Deserialize)]
pub struct MaplinkArea {
    #[require_domain]
    pub map_name: String,
    pub links: Vec<Link>,
}

impl<D> Writable<D> for MaplinkArea
where
    D: CanWriteWithArgs<String, WriteStringArgs> + CanWriteSliceWithArgs<Link, Option<SymbolName>>,
{
    fn to_writer(&self, ctx: &mut impl vivibin::WriteCtx, domain: &mut D) -> Result<()> {
        // TODO: turning off deduplication is a hack, figure out serialization order better
        domain.write_args(ctx, &self.map_name, WriteStringArgs { deduplicate: false })?;
        domain.write_slice_args_fallback(ctx, &self.links, Some(SymbolName::InternalNamed(self.map_name.clone())))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Readable, Writable, Serialize, Deserialize)]
pub struct Link {
    #[require_domain]
    pub id: String,
    pub destination: String,
    pub link_type: String,
    pub zone_id: String,
    pub player_direction: f32,
    pub player_facing: String,
    pub door_type: String,
    pub field_0x1c: String,
    pub pipe_cam_script_enter: String,
    pub pipe_cam_script_exit: String,
    pub field_0x28: u32,
    pub field_0x2c: String,
    pub enter_function: String,
    pub exit_function: String,
    pub field_0x38: String,
}
