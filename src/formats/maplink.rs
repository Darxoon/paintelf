use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{Readable, Reader, Writable, WriteCtx};

use crate::{
    SymbolName,
    binutil::{DataCategory, ElfReadDomain, ElfWriteDomain, WriteSliceArgs, WriteStringArgs},
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

pub fn write_maplink(ctx: &mut impl WriteCtx<DataCategory>, domain: &mut ElfWriteDomain, areas: &[MaplinkArea]) -> Result<()> {
    domain.write_symbol(ctx, "dataCount__Q3_4data3fld7maplink", |domain, ctx| {
        (areas.len() as u32).to_writer(ctx, domain)
    })?;
    
    domain.write_symbol(ctx, "datas__Q3_4data3fld7maplink", |domain, ctx| {
        for area in areas {
            area.to_writer(ctx, domain)?;
        }
        Ok(())
    })?;
    
    Ok(())
}

#[derive(Clone, Debug, Readable, Writable, Serialize, Deserialize)]
pub struct MaplinkArea {
    #[require_domain]
    #[write_args(WriteStringArgs { deduplicate: false })]
    pub map_name: String,
    
    #[write_args(WriteSliceArgs {
        symbol_name: Some(SymbolName::InternalNamed(self.map_name.clone())),
    })]
    pub links: Vec<Link>,
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
