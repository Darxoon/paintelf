use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanRead, CanReadVec, CanWrite, CanWriteSliceWithArgs, CanWriteWithArgs, HeapCategory, Readable,
    Reader, Writable, WriteSliceWithArgsFallbackExt, default_to_writer_impl, scoped_reader_pos,
};

use crate::{
    SymbolName,
    binutil::{ElfReadDomain, WriteStringArgs},
    formats::FileData,
    util::pointer::Pointer,
};

pub fn read_dispos(reader: &mut impl Reader, domain: ElfReadDomain) -> Result<FileData> {
    eprintln!("Warning: data_dispos is not fully supported yet. The yaml format is not final yet \
    and rebuilding the elf is not implemented yet.");
    
    let data_count_symbol = domain.find_symbol("all_disposDataTblLen__Q2_4data10DisposData")?;
    reader.seek(SeekFrom::Start(data_count_symbol.offset().into()))?;
    let data_count = reader.read_u32::<BigEndian>()?;
    
    let datas_symbol = domain.find_symbol("all_disposDataTbl__Q2_4data10DisposData")?;
    reader.seek(SeekFrom::Start(datas_symbol.offset().into()))?;
    
    let areas: Vec<DisposArea> = (0..data_count - 1)
        .map(|_| DisposArea::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(FileData::Dispos(areas))
}

fn read_dispos_item_vec<D: CanRead<Pointer>, T: Readable<D>>(reader: &mut impl Reader, domain: D) -> Result<Vec<T>> {
    let ptr: Pointer = domain.read(reader)?;
    let count: u32 = u32::from_reader(reader, domain)?;
    
    scoped_reader_pos!(reader);
    reader.seek(SeekFrom::Start(ptr.into()))?;
    
    // for some reason, trailing null value is included in count here
    // TODO: add mechanism for this
    let values: Vec<T> = (0..count - 1)
        .map(|_| T::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(values)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisposArea {
    pub id: String,
    pub map_npcs: Vec<DisposNpc>,
    pub map_mobjs: Vec<DisposMobj>,
    pub map_items: Vec<DisposItem>,
}

impl<D: CanRead<String> + CanRead<Option<String>> + CanRead<Pointer> + CanReadVec> Readable<D> for DisposArea {
    fn from_reader_unboxed<R: vivibin::Reader>(reader: &mut R, domain: D) -> Result<Self> {
        // TODO: provide actual mechanism for this
        let ptr: Pointer = domain.read(reader)?;
        scoped_reader_pos!(reader);
        reader.seek(SeekFrom::Start(ptr.into()))?;
        
        let id: String = domain.read(reader)?;
        let map_npcs: Vec<DisposNpc> = read_dispos_item_vec(reader, domain)?;
        let map_mobjs: Vec<DisposMobj> = read_dispos_item_vec(reader, domain)?;
        let map_items: Vec<DisposItem> = read_dispos_item_vec(reader, domain)?;
        
        Ok(Self { id, map_npcs, map_mobjs, map_items  })
    }
}

#[derive(Debug, Clone, Readable, Serialize, Deserialize)]
#[boxed]
#[extra_read_domain_deps(CanRead<Option<String>>)]
pub struct DisposNpc {
    #[require_domain]
    pub map_id: String,
    pub npcs: Vec<Npc>,
}

impl<C, D> Writable<C, D> for DisposNpc
where
    C: HeapCategory,
    D: CanWriteWithArgs<C, String, WriteStringArgs> + CanWrite<C, Option<String>> + CanWriteSliceWithArgs<C, Npc, Option<SymbolName>>,
{
    type UnboxedPostState = ();
    
    fn to_writer_unboxed(&self, ctx: &mut impl vivibin::WriteCtx<C>, domain: &mut D) -> Result<()> {
        // TODO: turning off deduplication is a hack, figure out serialization order better
        domain.write_args(ctx, &self.map_id, WriteStringArgs { deduplicate: false })?;
        domain.write_slice_args_fallback(ctx, &self.npcs, Some(SymbolName::InternalNamed(self.map_id.clone())))?;
        Ok(())
    }
    
    default_to_writer_impl!(C);
}

#[derive(Debug, Clone, Readable, Writable, Deserialize, Serialize)]
pub struct Npc {
    #[require_domain]
    pub id: String,
    pub r#type: String,
    pub field_0x8: u32,
    pub field_0xc: u32,
    pub field_0x10: f32,
    pub field_0x14: f32,
    pub field_0x18: f32,
    pub field_0x1c: u32,
    pub field_0x20: u32,
    pub field_0x24: u32,
    pub field_0x28: u32,
    pub field_0x2c: u32,
    pub field_0x30: u32,
    pub field_0x34: u32,
    pub field_0x38: u32,
    pub field_0x3c: f32,
    pub field_0x40: u32,
    pub field_0x44: u32,
    pub field_0x48: u32,
    pub field_0x4c: u32,
    pub field_0x50: u32,
    pub field_0x54: u32,
    pub field_0x58: u32,
    pub field_0x5c: u32,
    pub field_0x60: u32,
    pub field_0x64: u32,
    pub field_0x68: u32,
    pub field_0x6c: u32,
    pub field_0x70: u32,
    pub field_0x74: u32,
    pub field_0x78: u32,
    pub field_0x7c: u32,
    pub field_0x80: u32,
    pub field_0x84: u32,
    pub field_0x88: u32,
    pub field_0x8c: u32,
    pub field_0x90: u32,
    pub field_0x94: u32,
    pub field_0x98: u32,
    pub field_0x9c: u32,
    pub field_0xa0: u32,
    pub field_0xa4: u32,
    pub field_0xa8: u32,
    pub field_0xac: u32,
    pub field_0xb0: u32,
    pub field_0xb4: u32,
    pub field_0xb8: u32,
    pub field_0xbc: u32,
    pub field_0xc0: u32,
    pub field_0xc4: u32,
    pub field_0xc8: u32,
    pub field_0xcc: u32,
    pub field_0xd0: u32,
    pub field_0xd4: u32,
    pub field_0xd8: u32,
    pub field_0xdc: u32,
    pub field_0xe0: u32,
    pub field_0xe4: u32,
    pub field_0xe8: u32,
    pub field_0xec: u32,
    pub field_0xf0: u32,
    pub field_0xf4: u32,
    pub field_0xf8: u32,
    pub field_0xfc: u32,
    pub field_0x100: u32,
    pub field_0x104: u32,
    pub field_0x108: u32,
    pub field_0x10c: u32,
    pub field_0x110: u32,
    pub field_0x114: u32,
    #[require_domain]
    pub init_function: Option<String>,
    pub field_0x11c: u32,
    pub main_function: Option<String>,
    pub talk_function: Option<String>,
    pub field_0x128: u32,
    pub field_0x12c: u32,
    pub field_0x130: u32,
    pub field_0x134: u32,
}

#[derive(Debug, Clone, Readable, Serialize, Deserialize)]
#[boxed]
#[extra_read_domain_deps(CanRead<Option<String>>)]
pub struct DisposMobj {
    #[require_domain]
    pub map_id: String,
    pub mobjs: Vec<Mobj>,
}

impl<C, D> Writable<C, D> for DisposMobj
where
    C: HeapCategory,
    D: CanWriteWithArgs<C, String, WriteStringArgs>
        + CanWrite<C, Option<String>>
        + CanWriteSliceWithArgs<C, Mobj, Option<SymbolName>>,
{
    type UnboxedPostState = ();
    
    fn to_writer_unboxed(&self, ctx: &mut impl vivibin::WriteCtx<C>, domain: &mut D) -> Result<()> {
        // TODO: turning off deduplication is a hack, figure out serialization order better
        domain.write_args(ctx, &self.map_id, WriteStringArgs { deduplicate: false })?;
        domain.write_slice_args_fallback(ctx, &self.mobjs, Some(SymbolName::InternalNamed(self.map_id.clone())))?;
        Ok(())
    }
    
    default_to_writer_impl!(C);
}

#[derive(Debug, Clone, Readable, Writable, Deserialize, Serialize)]
pub struct Mobj {
    #[require_domain]
    pub id: String,
    pub r#type: String,
    pub field_0x8: f32,
    pub field_0xc: f32,
    pub field_0x10: f32,
    pub field_0x14: u32,
    pub field_0x18: u32,
    pub field_0x1c: u32,
    pub field_0x20: u32,
    pub field_0x24: u32,
    pub field_0x28: u32,
    pub field_0x2c: u32,
    pub field_0x30: u32,
    pub field_0x34: u32,
    pub field_0x38: u32,
    pub field_0x3c: u32,
    #[require_domain]
    pub field_0x40: Option<String>,
    pub field_0x44: u32,
    pub field_0x48: u32,
    pub field_0x4c: u32,
    pub field_0x50: u32,
    pub field_0x54: u32,
    pub field_0x58: u32,
    pub field_0x5c: u32,
    pub field_0x60: f32,
    pub field_0x64: f32,
    pub field_0x68: u32,
}

#[derive(Debug, Clone, Readable, Serialize, Deserialize)]
#[boxed]
#[extra_read_domain_deps(CanRead<Option<String>>)]
pub struct DisposItem {
    #[require_domain]
    pub map_id: String,
    pub items: Vec<Item>,
}

impl<C, D> Writable<C, D> for DisposItem
where
    C: HeapCategory,
    D: CanWriteWithArgs<C, String, WriteStringArgs>
        + CanWriteSliceWithArgs<C, Item, Option<SymbolName>>,
{
    type UnboxedPostState = ();
    
    fn to_writer_unboxed(&self, ctx: &mut impl vivibin::WriteCtx<C>, domain: &mut D) -> Result<()> {
        // TODO: turning off deduplication is a hack, figure out serialization order better
        domain.write_args(ctx, &self.map_id, WriteStringArgs { deduplicate: false })?;
        domain.write_slice_args_fallback(ctx, &self.items, Some(SymbolName::InternalNamed(self.map_id.clone())))?;
        Ok(())
    }
    
    default_to_writer_impl!(C);
}

#[derive(Debug, Clone, Readable, Writable, Deserialize, Serialize)]
pub struct Item {
    #[require_domain]
    pub id: String,
    pub field1_0x4: String,
    pub field2_0x8: f32,
    pub field3_0xc: f32,
    pub field4_0x10: f32,
    pub field5_0x14: u32,
    pub field6_0x18: u32,
    pub field7_0x1c: u32,
    pub field8_0x20: u32,
    pub field9_0x24: u32,
    pub field10_0x28: u32,
    pub field11_0x2c: u32,
    pub field12_0x30: u32,
    pub field13_0x34: u32,
    pub field14_0x38: u32,
    pub field15_0x3c: u32,
}
