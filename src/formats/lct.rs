use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanWrite, CanWriteBox, Readable, Reader, Writable, WriteBoxFallbackExt, WriteCtx,
    WriteDomainExt,
};

use crate::{
    binutil::{ElfCategory, ElfReadDomain, ElfWriteDomain},
    formats::FileData,
};

pub fn read_lct(reader: &mut impl Reader, domain: ElfReadDomain) -> Result<FileData> {
    let data_count_symbol = domain.find_symbol("all_lctAnimeDataTblLen__Q2_4data3lct")?;
    reader.seek(SeekFrom::Start(data_count_symbol.offset().into()))?;
    let data_count = reader.read_u32::<BigEndian>()?;
    
    let datas_symbol = domain.find_symbol("all_lctAnimeDataTbl__Q2_4data3lct")?;
    reader.seek(SeekFrom::Start(datas_symbol.offset().into()))?;
    
    let areas: Vec<AreaLct> = (0..data_count - 1)
        .map(|_| AreaLct::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(FileData::Lct(areas))
}

pub fn write_lct<C: ElfCategory>(ctx: &mut impl WriteCtx<Cat = C>, domain: &mut ElfWriteDomain<C>, lcts: &[AreaLct]) -> Result<()> {
    domain.write_symbol(ctx, "all_lctAnimeDataTblLen__Q2_4data3lct", |domain, ctx| {
        domain.write_fallback::<u32>(ctx, &(lcts.len() as u32 + 1))
    })?;
    
    domain.write_symbol(ctx, "all_lctAnimeDataTbl__Q2_4data3lct", |domain, ctx| {
        for lct in lcts {
            lct.to_writer(ctx, domain)?;
        }
        0u32.to_writer(ctx, domain)?;
        Ok(())
    })?;
    
    Ok(())
}

#[derive(Clone, Debug, Readable, Deserialize, Serialize)]
#[boxed]
pub struct AreaLct {
    #[require_domain]
    pub area_id: String,
    pub maps: Vec<MapLct>,
}

impl<D: CanWrite<String> + CanWriteBox> Writable<D> for AreaLct {
    fn to_writer_unboxed(&self, ctx: &mut impl WriteCtx<Cat = D::Cat>, domain: &mut D) -> Result<()> {
        domain.write(ctx, &self.area_id)?;
        domain.write_box_fallback(ctx, &0u32)?;
        domain.write_fallback(ctx, &(self.maps.len() as u32))?;
        Ok(())
    }
    
    fn to_writer(&self, ctx: &mut impl WriteCtx<Cat = D::Cat>, domain: &mut D) -> Result<()> {
        domain.write_box_of(ctx, |domain, ctx| {
            self.to_writer_unboxed(ctx, domain)
        })
    }
}

#[derive(Clone, Debug, Readable, Deserialize, Serialize)]
#[boxed]
pub struct MapLct {
    #[require_domain]
    pub map_id: String,
    pub lcts: Vec<Lct>,
}

#[derive(Clone, Debug, Readable, Deserialize, Serialize)]
pub struct Lct {
    #[require_domain]
    pub id: String,
    pub directory: String,
    pub file_name: String,
    pub field_0xc: u32,
}
