use std::{fmt::Debug, io::SeekFrom};

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanWrite, CanWriteBox, CanWriteSlice, CanWriteSliceNested, CanWriteSliceWithArgs,
    CanWriteSliceWithArgsNested, HeapToken, Readable, Reader, Writable, WritableExtraState, WritableNested,
    WritableNestedUnboxed, WriteCtx, WriteSliceExt, WriteSliceNestedExt, WriteSliceWithArgsExt,
};

use crate::{
    SymbolDeclaration, SymbolName,
    binutil::{DataCategory, ElfReadDomain, ElfWriteDomain, PutSymbol, WriteNullTerminatedSliceArgs},
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
pub trait AreaLct__Reqs: CanWrite<String>
    + CanWriteSliceWithArgsNested<MapLct, WriteNullTerminatedSliceArgs>
    + MapLct__Reqs
    + PutSymbol
{
}

impl<D> AreaLct__Reqs for D
where
    D: CanWrite<String>
        + CanWriteSliceWithArgsNested<MapLct, WriteNullTerminatedSliceArgs>
        + MapLct__Reqs
        + PutSymbol
{
}

impl<D: AreaLct__Reqs> Writable<D> for AreaLct {
    type UnboxedPostState = AreaLct__PostState<D>;
    type PostState = HeapToken;
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<Self::UnboxedPostState> {
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
    
    fn to_writer_unboxed_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: Self::UnboxedPostState) -> Result<()> {
        let extra_state = self.to_writer_unboxed_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, domain)?;
        Ok(token)
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: Self::PostState) -> Result<()> {
        let extra_state = self.to_writer_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
}

impl<D: AreaLct__Reqs> WritableNestedUnboxed<D> for AreaLct {
    type UnboxedExtraState = AreaLct__UnboxedExtraState<D>;

    fn to_writer_unboxed_nested_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: Self::UnboxedPostState) -> Result<Self::UnboxedExtraState> {
        domain.write_post(ctx, &self.area_id, state.area_id)?;
        let maps = domain.write_slice_args_nested_of_post(ctx, &self.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, state.maps, |domain, ctx, value| {
            value.to_writer(ctx, domain)
        }, |domain, ctx, value, state| {
            value.to_writer_post(ctx, domain, state)
        })?;
        Ok(AreaLct__UnboxedExtraState { maps })
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__UnboxedExtraState<D: AreaLct__Reqs> {
    pub maps: <D as CanWriteSliceWithArgsNested<MapLct, WriteNullTerminatedSliceArgs>>::ExtraState<HeapToken>,
}

impl<D: AreaLct__Reqs> WritableExtraState<D, AreaLct> for AreaLct__UnboxedExtraState<D> {
    type OutExtraState = AreaLct__UnboxedExtraState2<D>;
    
    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<AreaLct__UnboxedExtraState2<D>> {
        let extra_states = domain.write_slice_args_of_extra_post(ctx, &value.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, self.maps, |domain, ctx, value, state| {
            value.to_writer_nested_post(ctx, domain, state)
        })?;
        Ok(AreaLct__UnboxedExtraState2(extra_states))
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__UnboxedExtraState2<D: AreaLct__Reqs>(
    <D as CanWriteSliceWithArgsNested<MapLct, WriteNullTerminatedSliceArgs>>::ExtraState<MapLct__PostState<D>>,
);

impl<D: AreaLct__Reqs> WritableExtraState<D, AreaLct> for AreaLct__UnboxedExtraState2<D> {
    type OutExtraState = AreaLct__UnboxedExtraState3<D>;

    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<Self::OutExtraState> {
        let extra_states = domain.write_slice_args_of_extra_post(ctx, &value.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, self.0, |domain, ctx, value, state| {
            state.to_writer_nested_extra_post(value, ctx, domain)
        })?;
        Ok(AreaLct__UnboxedExtraState3(extra_states))
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__UnboxedExtraState3<D: AreaLct__Reqs>(
    <D as CanWriteSliceWithArgsNested<MapLct, WriteNullTerminatedSliceArgs>>::ExtraState<MapLct__UnboxedExtraState<D>>,
);

impl<D: AreaLct__Reqs> WritableExtraState<D, AreaLct> for AreaLct__UnboxedExtraState3<D> {
    type OutExtraState = ();

    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<()> {
        let _: <D as CanWriteSliceWithArgsNested<MapLct, WriteNullTerminatedSliceArgs>>::ExtraState<()> = domain.write_slice_args_of_extra_post(ctx, &value.maps, WriteNullTerminatedSliceArgs {
            symbol_name: None,
            write_length: true,
        }, self.0, |domain, ctx, value, state| {
            state.to_writer_nested_extra_post(value, ctx, domain)
        })?;
        
        Ok(())
    }
}

impl<D: AreaLct__Reqs> WritableNested<D> for AreaLct {
    type ExtraState = AreaLct__PostState<D>;

    fn to_writer_nested_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: Self::PostState) -> Result<Self::ExtraState> {
        // TODO: boilerplate shared with binutil
        let start_pos = ctx.position() as usize;
        let token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(state, token)?;
        
        let extra_state = self.to_writer_unboxed(ctx, domain)?;
        
        let size = ctx.position() as usize - start_pos;
        domain.put_symbol(SymbolDeclaration {
            name: SymbolName::Internal('l'),
            offset: token,
            size: size as u32,
        });
        
        Ok(extra_state)
    }
}

#[allow(non_camel_case_types)]
pub struct AreaLct__PostState<D: AreaLct__Reqs> {
    pub area_id: <D as CanWrite<String>>::PostState,
    pub maps: <D as CanWriteSliceWithArgs<MapLct, WriteNullTerminatedSliceArgs>>::PostState,
}

impl<D: AreaLct__Reqs> WritableExtraState<D, AreaLct> for AreaLct__PostState<D> {
    type OutExtraState = AreaLct__UnboxedExtraState<D>;
    
    fn to_writer_nested_extra_post(self, value: &AreaLct, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<Self::OutExtraState> {
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
pub trait MapLct__Reqs: CanWrite<String>
    + CanWriteBox
    + CanWriteSliceNested
{
}

impl<D> MapLct__Reqs for D
where
    D: CanWrite<String> + CanWriteBox + CanWriteSliceNested
{
}

impl<D: MapLct__Reqs> Writable<D> for MapLct {
    type UnboxedPostState = MapLct__PostState<D>;
    type PostState = HeapToken;
    
    fn to_writer_unboxed(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<MapLct__PostState<D>> {
        let map_id = domain.write(ctx, &self.map_id)?;
        let lcts = domain.write_slice(ctx, &self.lcts)?;
        Ok(MapLct__PostState {
            map_id,
            lcts,
        })
    }
    
    fn to_writer_unboxed_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: MapLct__PostState<D>) -> Result<()> {
        let extra_state = self.to_writer_unboxed_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
    
    fn to_writer(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<HeapToken> {
        let token = ctx.heap_token_at_current_pos()?;
        0u32.to_writer(ctx, domain)?;
        Ok(token)
    }
    
    fn to_writer_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: Self::PostState) -> Result<()> {
        let extra_state = self.to_writer_nested_post(ctx, domain, state)?;
        extra_state.to_writer_basic_extra_post(self, ctx, domain)
    }
}

impl<D: MapLct__Reqs> WritableNestedUnboxed<D> for MapLct {
    type UnboxedExtraState = MapLct__UnboxedExtraState<D>;

    fn to_writer_unboxed_nested_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: MapLct__PostState<D>) -> Result<MapLct__UnboxedExtraState<D>> {
        domain.write_post(ctx, &self.map_id, state.map_id)?;
        let lcts = domain.write_slice_nested_post(ctx, &self.lcts, state.lcts)?;
        Ok(MapLct__UnboxedExtraState { lcts })
    }
}

#[allow(non_camel_case_types)]
pub struct MapLct__UnboxedExtraState<D: MapLct__Reqs> {
    lcts: <D as CanWriteSliceNested>::ExtraState<Lct__PostState<D>>,
}

impl<D: MapLct__Reqs> WritableExtraState<D, MapLct> for MapLct__UnboxedExtraState<D> {
    type OutExtraState = ();

    fn to_writer_nested_extra_post(self, value: &MapLct, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<Self::OutExtraState> {
        let _: <D as CanWriteSliceNested>::ExtraState<()> = domain.write_slice_of_nested_extra_post(ctx, &value.lcts, self.lcts, |domain, ctx, value, state| {
            value.to_writer_post(ctx, domain, state)
        })?;
        Ok(())
    }
}

impl<D: MapLct__Reqs> WritableNested<D> for MapLct {
    type ExtraState = MapLct__PostState<D>;

    fn to_writer_nested_post(&self, ctx: &mut WriteCtx<D::Cat>, domain: &mut D, state: Self::PostState) -> Result<Self::ExtraState> {
        let token = ctx.heap_token_at_current_pos()?;
        ctx.add_relocation(state, token)?;
        
        self.to_writer_unboxed(ctx, domain)
    }
}

#[allow(non_camel_case_types)]
pub struct MapLct__PostState<D: MapLct__Reqs> {
    pub map_id: <D as CanWrite<String>>::PostState,
    pub lcts: <D as CanWriteSlice>::PostState,
}

impl<D: MapLct__Reqs> WritableExtraState<D, MapLct> for MapLct__PostState<D> {
    type OutExtraState = MapLct__UnboxedExtraState<D>;

    fn to_writer_nested_extra_post(self, value: &MapLct, ctx: &mut WriteCtx<D::Cat>, domain: &mut D) -> Result<MapLct__UnboxedExtraState<D>> {
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
