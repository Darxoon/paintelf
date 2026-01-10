use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    CanRead, CanWrite, CanWriteSliceWithArgs, HeapCategory, Readable, Reader, Writable, WriteCtx,
    WriteSliceWithArgsFallbackExt, default_to_writer_impl, scoped_reader_pos,
};

use crate::{
    SymbolName,
    binutil::{ElfCategory, ElfReadDomain, ElfWriteDomain, WriteNullTermiantedSliceArgs},
    formats::FileData,
    util::pointer::Pointer,
};

pub fn read_shops(reader: &mut impl Reader, domain: ElfReadDomain) -> Result<FileData> {
    let shop_list_len_symbol = domain.find_symbol("shopListLen__Q2_4data4shop")?;
    reader.seek(SeekFrom::Start(shop_list_len_symbol.offset().into()))?;
    let shop_list_len = reader.read_u32::<BigEndian>()?;
    
    let shop_list_symbol = domain.find_symbol("shopList__Q2_4data4shop")?;
    reader.seek(SeekFrom::Start(shop_list_symbol.offset().into()))?;
    
    let shop_list: Vec<Shop> = (0..shop_list_len)
        .map(|_| Shop::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(FileData::Shop(shop_list))
}

pub fn write_shops<C: ElfCategory>(ctx: &mut impl WriteCtx<C>, domain: &mut ElfWriteDomain<C>, shops: &[Shop]) -> Result<()> {
    domain.write_symbol(ctx, "shopList__Q2_4data4shop", |domain, ctx| {
        for shop in shops {
            shop.to_writer(ctx, domain)?;
        }
        Ok(())
    })?;
    
    domain.write_symbol(ctx, "shopListLen__Q2_4data4shop", |domain, ctx| {
        (shops.len() as u32).to_writer(ctx, domain)
    })?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shop {
    pub shop_id: String,
    pub items: Vec<SoldItem>,
}

// TODO: vivibin can't pass along SoldItem's Option<String> dependency
impl<D: CanRead<String> + CanRead<Option<String>> + CanRead<Pointer>> Readable<D> for Shop {
    fn from_reader_unboxed<R: vivibin::Reader>(reader: &mut R, domain: D) -> Result<Self> {
        let shop_id: String = domain.read(reader)?;
        let items_ptr: Pointer = domain.read(reader)?;
        
        // TODO: provide abstraction for this
        scoped_reader_pos!(reader);
        reader.seek(SeekFrom::Start(items_ptr.into()))?;
        let mut items = Vec::new();
        loop {
            let value = SoldItem::from_reader(reader, domain)?;
            
            if value == SoldItem::default() {
                break;
            }
            
            items.push(value);
        }
        
        Ok(Self { shop_id, items })
    }
}

impl<C, D> Writable<C, D> for Shop
where
    C: HeapCategory,
    D: CanWrite<C, String>
        + CanWrite<C, Option<String>>
        + CanWriteSliceWithArgs<C, SoldItem, WriteNullTermiantedSliceArgs>,
{
    type UnboxedPostState = ();
    
    fn to_writer_unboxed(&self, ctx: &mut impl vivibin::WriteCtx<C>, domain: &mut D) -> Result<()> {
        domain.write(ctx, &self.shop_id)?;
        domain.write_slice_args_fallback(ctx, &self.items, WriteNullTermiantedSliceArgs {
            symbol_name: Some(SymbolName::Internal('s')),
            write_length: false,
        })?;
        Ok(())
    }
    
    default_to_writer_impl!(C);
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Readable, Writable, Deserialize, Serialize)]
pub struct SoldItem {
    #[require_domain]
    pub item_id: Option<String>,
    pub requirement: Option<String>,
}
