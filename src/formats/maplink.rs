use std::io::{Seek, SeekFrom};

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::Serialize;

use crate::{formats::FileData, scoped_ctx_pos, ReadContext};

pub fn read_maplink<'a>(ctx: &'a mut ReadContext<'a>) -> Result<FileData> {
    let data_count_symbol = ctx.find_symbol("dataCount__Q3_4data3fld7maplink")?;
    ctx.reader.seek(SeekFrom::Start(data_count_symbol.offset().into()))?;
    let data_count = ctx.reader.read_u32::<BigEndian>()?;
    
    let datas_symbol = ctx.find_symbol("datas__Q3_4data3fld7maplink")?;
    ctx.reader.seek(SeekFrom::Start(datas_symbol.offset().into()))?;
    
    let areas: Vec<MaplinkArea> = (0..data_count)
        .map(|_| MaplinkArea::from_reader(ctx))
        .collect::<Result<_>>()?;
    
    Ok(FileData::Maplink(areas))
}

#[derive(Clone, Debug, Serialize)]
pub struct MaplinkArea {
    pub map_name: String,
    pub links: Vec<Link>,
}

impl MaplinkArea {
    pub fn from_reader<'a, 'b>(ctx: &'a mut ReadContext<'b>) -> Result<Self> {
        let map_name = ctx.read_string()?;
        let links_ptr = ctx.read_pointer()?;
        let link_count = ctx.reader.read_u32::<BigEndian>()?;
        
        scoped_ctx_pos!(ctx);
        ctx.reader.seek(SeekFrom::Start(links_ptr.into()))?;
        
        let links: Vec<Link> = (0..link_count)
            .map(|_| Link::from_reader(ctx))
            .collect::<Result<_>>()?;
        
        Ok(Self {
            map_name,
            links,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub id: String,
    pub destination: String,
    pub link_type: String,
    pub field_0xc: String,
    pub field_0x10: f32,
    pub field_0x14: String,
    pub field_0x18: String,
    pub field_0x1c: String,
    pub field_0x20: String,
    pub field_0x24: String,
    pub field_0x28: u32,
    pub field_0x2c: String,
    pub field_0x30: String,
    pub field_0x34: String,
    pub field_0x38: String,
}

impl Link {
    pub fn from_reader(ctx: &mut ReadContext) -> Result<Self> {
        let id = ctx.read_string()?;
        let destination = ctx.read_string()?;
        let link_type = ctx.read_string()?;
        let field_0xc = ctx.read_string()?;
        let field_0x10 = ctx.reader.read_f32::<BigEndian>()?;
        let field_0x14 = ctx.read_string()?;
        let field_0x18 = ctx.read_string()?;
        let field_0x1c = ctx.read_string()?;
        let field_0x20 = ctx.read_string()?;
        let field_0x24 = ctx.read_string()?;
        let field_0x28 = ctx.reader.read_u32::<BigEndian>()?;
        let field_0x2c = ctx.read_string()?;
        let field_0x30 = ctx.read_string()?;
        let field_0x34 = ctx.read_string()?;
        let field_0x38 = ctx.read_string()?;
        
        Ok(Self {
            id,
            destination,
            link_type,
            field_0xc,
            field_0x10,
            field_0x14,
            field_0x18,
            field_0x1c,
            field_0x20,
            field_0x24,
            field_0x28,
            field_0x2c,
            field_0x30,
            field_0x34,
            field_0x38,
        })
    }
}

