use std::{fmt::Debug, io::SeekFrom};

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanWrite, CanWriteBox, CanWriteSlice, CanWriteSliceWithArgs, CanWriteSliceWithArgsNested, HeapCategory,
    HeapToken, Readable, Reader, Writable, WritableExtraState, WritableNested, WritableNestedUnboxed,
    WriteCtx, WriteSliceExt, WriteSliceWithArgsExt, WriteSliceWithArgsNestedExt,
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
    
    let mut states = Vec::with_capacity(lcts.len());
    
    domain.write_symbol(ctx, "all_lctAnimeDataTbl__Q2_4data3lct", |domain, ctx| {
        for lct in lcts {
            states.push(lct.to_writer(ctx, domain)?);
        }
        0u32.to_writer(ctx, domain)?;
        Ok(())
    })?;
    
    let mut extra_states = Vec::with_capacity(lcts.len());
    
    for (lct, state) in lcts.iter().zip(states) {
        extra_states.push(lct.to_writer_nested_post(ctx, domain, state)?);
    }
    
    for (lct, state) in lcts.iter().zip(extra_states) {
        state.to_writer_extra_post(lct, ctx, domain)?;
    }
    
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
        + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>,
{
    type UnboxedPostState = Area__PostState<C, D>;
    type PostState = HeapToken;
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<Self::UnboxedPostState> {
        println!("arealct {}", self.area_id);
        let area_id = domain.write(ctx, &self.area_id)?;
        let maps = domain.write_slice_args(ctx, &self.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        })?;
        Ok(Area__PostState {
            area_id,
            maps,
        })
    }
    
    fn to_writer_unboxed_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::UnboxedPostState) -> Result<()> {
        println!("arealct post {}", self.area_id);
        let extra_state = self.to_writer_unboxed_nested_post(ctx, domain, state)?;
        extra_state.to_writer_extra_post(self, ctx, domain)
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, domain)?;
        Ok(token)
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<()> {
        let extra_state = self.to_writer_nested_post(ctx, domain, state)?;
        extra_state.to_writer_extra_post(self, ctx, domain)
    }
}

impl<C, D> WritableNestedUnboxed<C, D> for AreaLct
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteBox<C>
        + CanWriteSlice<C>
        + CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>
        + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>,
{
    type UnboxedExtraState = AreaLct__UnboxedExtraState<C, D>;

    fn to_writer_unboxed_nested_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::UnboxedPostState) -> Result<Self::UnboxedExtraState> {
        domain.write_post(ctx, &self.area_id, state.area_id)?;
        let maps = domain.write_slice_args_nested_post(ctx, &self.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, state.maps)?;
        Ok(AreaLct__UnboxedExtraState(maps))
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__UnboxedExtraState<C, D>(
    <D as CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>>::ExtraState,
)
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteBox<C>
        + CanWriteSlice<C>
        + CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>
        + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>;

impl<C, D> WritableExtraState<C, D, AreaLct> for AreaLct__UnboxedExtraState<C, D>
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteBox<C>
        + CanWriteSlice<C>
        + CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>
        + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>,
{
    fn to_writer_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<()> {
        domain.write_slice_args_of_extra_post(ctx, &value.maps, self.0, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, |domain, ctx, value, state| {
            value.to_writer_post(ctx, domain, state)
        })
    }
}

impl<C, D> WritableNested<C, D> for AreaLct
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteBox<C>
        + CanWriteSlice<C>
        + CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>
        + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>,
{
    type ExtraState = Area__PostState<C, D>;

    fn to_writer_nested_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<Self::ExtraState> {
        let token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(state, token)?;
        
        self.to_writer_unboxed(ctx, domain)
    }
}

#[allow(non_camel_case_types)]
pub struct Area__PostState<C, D>
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteBox<C>
        + CanWriteSlice<C>
        + CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>,
{
    pub area_id: <D as CanWrite<C, String>>::PostState,
    pub maps: <D as CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>>::PostState,
}

impl<C, D> WritableExtraState<C, D, AreaLct> for Area__PostState<C, D>
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteBox<C>
        + CanWriteSlice<C>
        + CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>
        + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>,
{
    fn to_writer_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<()> {
        value.to_writer_unboxed_post(ctx, domain, self)
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
    type UnboxedPostState = MapLct__PostState<C, D>;
    type PostState = HeapToken;
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<Self::UnboxedPostState> {
        let map_id = domain.write(ctx, &self.map_id)?;
        let lcts = domain.write_slice(ctx, &self.lcts)?;
        Ok(MapLct__PostState {
            map_id,
            lcts,
        })
    }
    
    fn to_writer_unboxed_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::UnboxedPostState) -> Result<()> {
        domain.write_post(ctx, &self.map_id, state.map_id)?;
        domain.write_slice_post(ctx, &self.lcts, state.lcts)?;
        Ok(())
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, domain)?;
        Ok(token)
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<()> {
        // TODO: WriteNullTermiantedSliceArgs does not interact well with boxing
        if self.map_id.is_empty() && self.lcts.is_empty() {
            return Ok(());
        }
        
        let token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(state, token)?;
        
        let state = self.to_writer_unboxed(ctx, domain)?;
        self.to_writer_unboxed_post(ctx, domain, state)?;
        Ok(())
    }
}

#[allow(non_camel_case_types)]
pub struct MapLct__PostState<C: HeapCategory, D: CanWrite<C, String> + CanWriteBox<C> + CanWriteSlice<C>> {
    pub map_id: <D as CanWrite<C, String>>::PostState,
    pub lcts: <D as CanWriteSlice<C>>::PostState,
}

#[derive(Clone, Debug, Readable, Writable, Deserialize, Serialize)]
pub struct Lct {
    #[require_domain]
    pub id: String,
    pub directory: String,
    pub file_name: String,
    pub field_0xc: u32,
}
