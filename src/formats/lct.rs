use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanWrite, CanWriteBox, CanWriteSlice, CanWriteSliceWithArgs, HeapCategory, HeapToken, Readable, Reader, Writable, WriteCtx, WriteSliceFallbackExt, WriteSliceWithArgsFallbackExt,
};

use crate::{
    binutil::{DataCategory, ElfReadDomain, ElfWriteDomain, WriteNullTerminatedSliceArgs},
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

pub fn write_lct(ctx: &mut WriteCtx<DataCategory>, domain: &mut ElfWriteDomain, lcts: &[AreaLct]) -> Result<()> {
    domain.write_symbol(ctx, "all_lctAnimeDataTblLen__Q2_4data3lct", |domain, ctx| {
        (lcts.len() as u32 + 1).to_writer(ctx, domain)
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

impl<C, D> Writable<C, D> for AreaLct
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteBox<C>
        + CanWriteSlice<C>
        + CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>,
{
    type UnboxedPostState = <D as CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>>::PostState;
    type PostState = HeapToken;
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<Self::UnboxedPostState> {
        domain.write(ctx, &self.area_id)?;
        domain.write_slice_args_fallback(ctx, &self.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        })
    }
    
    fn to_writer_unboxed_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::UnboxedPostState) -> Result<()> {
        domain.write_slice_args_post_fallback(ctx, &self.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, state)
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<C>, _domain: &mut D) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        Ok(token)
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: HeapToken) -> Result<()> {
        let current_token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(state, current_token)?;
        
        // issue: these need to be separate
        // writes arealct
        let state = self.to_writer_unboxed(ctx, domain)?;
        // writes maplcts
        self.to_writer_unboxed_post(ctx, domain, state)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Readable, Deserialize, Serialize)]
#[boxed]
pub struct MapLct {
    #[require_domain]
    pub map_id: String,
    pub lcts: Vec<Lct>,
}

impl<C: HeapCategory, D: CanWrite<C, String> + CanWriteBox<C> + CanWriteSlice<C>> Writable<C, D> for MapLct {
    type UnboxedPostState = ();
    type PostState = ();
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<()> {
        domain.write(ctx, &self.map_id)?;
        domain.write_slice_fallback(ctx, &self.lcts)?;
        Ok(())
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<()> {
        // TODO: WriteNullTermiantedSliceArgs does not interact well with boxing
        if self.map_id.is_empty() && self.lcts.is_empty() {
            0u32.to_writer(ctx, domain)
        } else {
            domain.write_box_of(ctx, |domain, ctx| {
                self.to_writer_unboxed(ctx, domain)
            })
        }
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<()> {
        self.to_writer_unboxed_post(ctx, domain, state)
    }
}

#[derive(Clone, Debug, Readable, Writable, Deserialize, Serialize)]
pub struct Lct {
    #[require_domain]
    pub id: String,
    pub directory: String,
    pub file_name: String,
    pub field_0xc: u32,
}
