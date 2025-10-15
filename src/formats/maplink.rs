use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    scoped_reader_pos, CanRead, CanWrite, CanWriteWithArgs, ReadDomainExt, Readable, Reader, Writable, WriteCtx, WriteDomainExt, Writer
};

use crate::{
    formats::FileData, util::pointer::Pointer, ElfReadDomain, ElfWriteDomain, SymbolDeclaration, SymbolName, WriteStringArgs
};

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
    let token =  ctx.heap_token_at_current_pos()?;
    domain.write_fallback::<u32>(ctx, &(areas.len() as u32))?;
    
    domain.put_symbol(SymbolDeclaration {
        name: SymbolName::Unmangled("dataCount__Q3_4data3fld7maplink".to_string()),
        offset: token,
        size: 4,
    });
    
    let token = ctx.heap_token_at_current_pos()?;
    let areas_start = ctx.position()?;
    
    for area in areas {
        area.to_writer(ctx, domain)?;
    }
    
    let areas_size = ctx.position()? - areas_start;
    
    domain.put_symbol(SymbolDeclaration {
        name: SymbolName::Unmangled("datas__Q3_4data3fld7maplink".to_string()),
        offset: token,
        size: areas_size as u32,
    });
    
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
        
        Ok(Self { map_name, links })
    }
}

impl<D: CanWriteWithArgs<String, WriteStringArgs> + CanWrite<SymbolDeclaration>> Writable<D> for MaplinkArea {
    fn to_writer(&self, ctx: &mut impl vivibin::WriteCtx, domain: D) -> Result<()> {
        // TODO: turning off deduplication is a hack, figure out serialization order better
        domain.write_args(ctx, &self.map_name, WriteStringArgs { deduplicate: false })?;
        
        let mut links_size: usize = 0;
        let token = ctx.allocate_next_block_aligned(4, |ctx| {
            let start_pos = ctx.position()? as usize;
            for link in &self.links {
                link.to_writer(ctx, domain)?;
            }
            links_size = ctx.position()? as usize - start_pos;
            Ok(())
        })?;
        ctx.write_token::<4>(token)?;
        domain.write(ctx, &SymbolDeclaration {
            name: SymbolName::InternalNamed(self.map_name.clone()),
            offset: token,
            size: links_size as u32,
        })?;
        
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

impl<D: CanWrite<String>> Writable<D> for Link {
    fn to_writer(&self, ctx: &mut impl vivibin::WriteCtx, domain: D) -> Result<()> {
        // TODO: actual string writing implementation
        domain.write(ctx, &self.id)?;
        domain.write(ctx, &self.destination)?;
        domain.write(ctx, &self.link_type)?;
        domain.write(ctx, &self.zone_id)?;
        domain.write_fallback::<f32>(ctx, &self.player_direction)?;
        domain.write(ctx, &self.player_facing)?;
        domain.write(ctx, &self.door_type)?;
        domain.write(ctx, &self.field_0x1c)?;
        domain.write(ctx, &self.pipe_cam_script_enter)?;
        domain.write(ctx, &self.pipe_cam_script_exit)?;
        domain.write_fallback::<u32>(ctx, &self.field_0x28)?;
        domain.write(ctx, &self.field_0x2c)?;
        domain.write(ctx, &self.enter_function)?;
        domain.write(ctx, &self.exit_function)?;
        domain.write(ctx, &self.field_0x38)?;
        Ok(())
    }
}
