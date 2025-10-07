use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{scoped_reader_pos, CanRead, CanWrite, ReadDomainExt, Readable, Reader, Writable, WriteCtx, WriteDomain, WriteDomainExt};

use crate::{formats::FileData, util::pointer::Pointer, ElfReadDomain, ElfWriteDomain};

pub fn read_maplink<'a>(reader: &mut impl Reader, domain: ElfReadDomain) -> Result<FileData> {
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

pub fn write_maplink(ctx: &mut impl WriteCtx, domain: ElfWriteDomain, areas: &[MaplinkArea]) -> Result<()> {
    domain.write_fallback::<u32>(ctx, &(areas.len() as u32))?;
    
    for area in areas {
        area.to_writer(ctx, domain)?;
    }
    
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaplinkArea {
    pub map_name: String,
    pub links: Vec<Link>,
}

impl<D: CanRead<String> + CanRead<Pointer>> Readable<D> for MaplinkArea {
    fn from_reader<R: vivibin::Reader>(reader: &mut R, domain: D) -> Result<Self> {
        let map_name: String = domain.read(reader)?;
        let links_ptr: Pointer = domain.read(reader)?;
        let link_count: u32 = domain.read_fallback(reader)?;
        
        scoped_reader_pos!(reader);
        reader.seek(SeekFrom::Start(links_ptr.into()))?;
        
        let links: Vec<Link> = (0..link_count)
            .map(|_| Link::from_reader(reader, domain))
            .collect::<Result<_>>()?;
        
        Ok(Self {
            map_name,
            links,
        })
    }
}

impl<D: CanWrite<String>> Writable<D> for MaplinkArea {
    fn to_writer(&self, ctx: &mut impl vivibin::WriteCtx, domain: D) -> Result<()> {
        domain.write(ctx, &self.map_name)?;
        
        let token = ctx.allocate_next_block(|ctx| {
            for link in &self.links {
                link.to_writer(ctx, domain)?;
            }
            Ok(())
        })?;
        ctx.write_token::<4>(token)?;
        
        domain.write_fallback::<u32>(ctx, &(self.links.len() as u32))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Readable, Serialize, Deserialize)]
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

impl<D: WriteDomain> Writable<D> for Link {
    fn to_writer(&self, ctx: &mut impl vivibin::WriteCtx, domain: D) -> Result<()> {
        // TODO: actual string writing implementation
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<f32>(ctx, &self.player_direction)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &self.field_0x28)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        domain.write_fallback::<u32>(ctx, &0)?;
        Ok(())
    }
}
