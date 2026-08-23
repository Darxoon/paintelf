use std::{fmt::Debug, io::SeekFrom};

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanWrite, CanWriteBox, CanWriteSlice, CanWriteSliceNested, CanWriteSliceWithArgs,
    CanWriteSliceWithArgsNested, HeapCategory, HeapToken, Readable, Reader, Writable, WritableExtraState,
    WritableNested, WritableNestedUnboxed, WriteCtx, WriteSliceExt, WriteSliceNestedExt,
    WriteSliceWithArgsExt,
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
    
    let mut extra_states2 = Vec::with_capacity(lcts.len());
    
    for (lct, state) in lcts.iter().zip(extra_states) {
        extra_states2.push(state.to_writer_nested_extra_post(lct, ctx, domain)?);
    }
    
    let mut extra_states3 = Vec::with_capacity(lcts.len());
    
    for (lct, state) in lcts.iter().zip(extra_states2) {
        extra_states3.push(state.to_writer_nested_extra_post(lct, ctx, domain)?);
    }
    
    let mut extra_states4 = Vec::with_capacity(lcts.len());
    
    for (lct, state) in lcts.iter().zip(extra_states3) {
        extra_states4.push(state.to_writer_nested_extra_post(lct, ctx, domain)?);
    }
    
    for (lct, state) in lcts.iter().zip(extra_states4) {
        () = state.to_writer_nested_extra_post(lct, ctx, domain)?;
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

#[allow(non_camel_case_types)]
pub trait AreaLct__Reqs<C: HeapCategory>: CanWrite<C, String>
    + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>
    + MapLct__Reqs<C>
{
}

impl<C, D> AreaLct__Reqs<C> for D
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>
        + MapLct__Reqs<C>,
{
}

impl<C: HeapCategory, D: AreaLct__Reqs<C>> Writable<C, D> for AreaLct {
    type UnboxedPostState = AreaLct__PostState<C, D>;
    type PostState = HeapToken;
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<Self::UnboxedPostState> {
        let area_id = domain.write(ctx, &self.area_id)?;
        let maps = domain.write_slice_args(ctx, &self.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        })?;
        Ok(AreaLct__PostState {
            area_id,
            maps,
        })
    }
    
    fn to_writer_unboxed_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::UnboxedPostState) -> Result<()> {
        let extra_state = self.to_writer_unboxed_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, domain)?;
        Ok(token)
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<()> {
        let extra_state = self.to_writer_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
}

impl<C: HeapCategory, D: AreaLct__Reqs<C>> WritableNestedUnboxed<C, D> for AreaLct {
    type UnboxedExtraState = AreaLct__UnboxedExtraState<C, D>;

    fn to_writer_unboxed_nested_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::UnboxedPostState) -> Result<Self::UnboxedExtraState> {
        domain.write_post(ctx, &self.area_id, state.area_id)?;
        let maps = domain.write_slice_args_nested_of_post(ctx, &self.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, state.maps, |domain, ctx, value| {
            value.to_writer(ctx, domain)
        }, |domain, ctx, value, state| {
            value.to_writer_post(ctx, domain, state)
        })?;
        Ok(AreaLct__UnboxedExtraState(maps))
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__UnboxedExtraState<C: HeapCategory, D: AreaLct__Reqs<C>>(
    <D as CanWriteSliceWithArgsNested<C, MapLct, WriteNullTerminatedSliceArgs, HeapToken>>::ExtraState,
);

impl<C: HeapCategory, D: AreaLct__Reqs<C>> WritableExtraState<C, D, AreaLct> for AreaLct__UnboxedExtraState<C, D> {
    type OutExtraState = AreaLct__UnboxedExtraState2<C, D>;
    
    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<AreaLct__UnboxedExtraState2<C, D>> {
        let extra_states = domain.write_slice_args_of_extra_post(ctx, &value.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, self.0, |domain, ctx, value, state| {
            value.to_writer_nested_post(ctx, domain, state)
        })?;
        Ok(AreaLct__UnboxedExtraState2(extra_states))
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__UnboxedExtraState2<C: HeapCategory, D: AreaLct__Reqs<C>>(Vec<MapLct__PostState<C, D>>);

impl<C: HeapCategory, D: AreaLct__Reqs<C>> WritableExtraState<C, D, AreaLct> for AreaLct__UnboxedExtraState2<C, D> {
    type OutExtraState = AreaLct__UnboxedExtraState3<C, D>;

    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<Self::OutExtraState> {
        let mut extra_states = Vec::with_capacity(value.maps.len());
        
        for (map, state) in value.maps.iter().zip(self.0) {
            extra_states.push(state.to_writer_nested_extra_post(map, ctx, domain)?);
        }
        
        Ok(AreaLct__UnboxedExtraState3(extra_states))
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__UnboxedExtraState3<C: HeapCategory, D: AreaLct__Reqs<C>>(Vec<MapLct__UnboxedExtraState<C, D>>);

impl<C: HeapCategory, D: AreaLct__Reqs<C>> WritableExtraState<C, D, AreaLct> for AreaLct__UnboxedExtraState3<C, D> {
    type OutExtraState = ();

    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<()> {
        for (map, state) in value.maps.iter().zip(self.0) {
            () = state.to_writer_nested_extra_post(map, ctx, domain)?;
        }
        
        Ok(())
    }
}

impl<C: HeapCategory, D: AreaLct__Reqs<C>> WritableNested<C, D> for AreaLct {
    type ExtraState = AreaLct__PostState<C, D>;

    fn to_writer_nested_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<Self::ExtraState> {
        let token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(state, token)?;
        
        self.to_writer_unboxed(ctx, domain)
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__PostState<C: HeapCategory, D: AreaLct__Reqs<C>> {
    pub area_id: <D as CanWrite<C, String>>::PostState,
    pub maps: <D as CanWriteSliceWithArgs<C, MapLct, WriteNullTerminatedSliceArgs>>::PostState,
}

impl<C: HeapCategory, D: AreaLct__Reqs<C>> WritableExtraState<C, D, AreaLct> for AreaLct__PostState<C, D> {
    type OutExtraState = AreaLct__UnboxedExtraState<C, D>;
    
    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<Self::OutExtraState> {
        value.to_writer_unboxed_nested_post(ctx, domain, self)
    }
}

#[derive(Clone, Debug, Default, Readable, Deserialize, Serialize)]
#[boxed]
pub struct MapLct {
    #[require_domain]
    pub map_id: String,
    pub lcts: Vec<Lct>,
}

#[allow(non_camel_case_types)]
pub trait MapLct__Reqs<C: HeapCategory>: CanWrite<C, String>
    + CanWriteBox<C>
    + CanWriteSliceNested<C, Lct__PostState<C, Self>>
{
}

impl<C, D> MapLct__Reqs<C> for D
where
    C: HeapCategory,
    D: CanWrite<C, String> + CanWriteBox<C> + CanWriteSliceNested<C, Lct__PostState<C, D>>
{
}

impl<C: HeapCategory, D: MapLct__Reqs<C>> Writable<C, D> for MapLct {
    type UnboxedPostState = MapLct__PostState<C, D>;
    type PostState = HeapToken;
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<MapLct__PostState<C, D>> {
        let map_id = domain.write(ctx, &self.map_id)?;
        let lcts = domain.write_slice(ctx, &self.lcts)?;
        Ok(MapLct__PostState {
            map_id,
            lcts,
        })
    }
    
    fn to_writer_unboxed_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: MapLct__PostState<C, D>) -> Result<()> {
        let extra_state = self.to_writer_unboxed_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, domain)?;
        Ok(token)
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<()> {
        let extra_state = self.to_writer_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
}

impl<C: HeapCategory, D: MapLct__Reqs<C>> WritableNestedUnboxed<C, D> for MapLct {
    type UnboxedExtraState = MapLct__UnboxedExtraState<C, D>;

    fn to_writer_unboxed_nested_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: MapLct__PostState<C, D>) -> Result<MapLct__UnboxedExtraState<C, D>> {
        domain.write_post(ctx, &self.map_id, state.map_id)?;
        let lcts = domain.write_slice_nested_post(ctx, &self.lcts, state.lcts)?;
        Ok(MapLct__UnboxedExtraState { lcts })
    }
}

#[allow(non_camel_case_types)]
pub struct MapLct__UnboxedExtraState<C: HeapCategory, D: MapLct__Reqs<C>> {
    lcts: <D as CanWriteSliceNested<C, Lct__PostState<C, D>>>::ExtraState,
}

impl<C: HeapCategory, D: MapLct__Reqs<C>> WritableExtraState<C, D, MapLct> for MapLct__UnboxedExtraState<C, D> {
    type OutExtraState = ();

    fn to_writer_nested_extra_post(self, value: &MapLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<Self::OutExtraState> {
        let _: Vec<()> = domain.write_slice_of_extra_post(ctx, &value.lcts, self.lcts, |domain, ctx, value, state| {
            value.to_writer_post(ctx, domain, state)
        })?;
        Ok(())
    }
}

impl<C: HeapCategory, D: MapLct__Reqs<C>> WritableNested<C, D> for MapLct {
    type ExtraState = MapLct__PostState<C, D>;

    fn to_writer_nested_post(&self, ctx: &mut WriteCtx<C>, domain: &mut D, state: Self::PostState) -> Result<Self::ExtraState> {
        let token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(state, token)?;
        
        self.to_writer_unboxed(ctx, domain)
    }
}

#[allow(non_camel_case_types)]
pub struct MapLct__PostState<C: HeapCategory, D: MapLct__Reqs<C>> {
    pub map_id: <D as CanWrite<C, String>>::PostState,
    pub lcts: <D as CanWriteSlice<C>>::PostState,
}

impl<C: HeapCategory, D: MapLct__Reqs<C>> WritableExtraState<C, D, MapLct> for MapLct__PostState<C, D> {
    type OutExtraState = MapLct__UnboxedExtraState<C, D>;

    fn to_writer_nested_extra_post(self, value: &MapLct, ctx: &mut WriteCtx<C>, domain: &mut D) -> Result<MapLct__UnboxedExtraState<C, D>> {
        value.to_writer_unboxed_nested_post(ctx, domain, self)
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
