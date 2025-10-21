use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{
    scoped_reader_pos, CanRead, CanWrite, CanWriteSliceWithArgs, Readable, Reader, Writable, WriteCtx, WriteDomainExt, WriteSliceWithArgsFallbackExt
};

use crate::{
    SymbolName,
    binutil::{ElfReadDomain, ElfWriteDomain, WriteNullTermiantedSliceArgs},
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

pub fn write_shops(ctx: &mut impl WriteCtx, domain: &mut ElfWriteDomain, shops: &[Shop]) -> Result<()> {
    domain.write_symbol(ctx, "shopList__Q2_4data4shop", |domain, ctx| {
        for shop in shops {
            shop.to_writer(ctx, domain)?;
        }
        Ok(())
    })?;
    
    domain.write_symbol(ctx, "shopListLen__Q2_4data4shop", |domain, ctx| {
        domain.write_fallback::<u32>(ctx, &(shops.len() as u32))
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

impl<D> Writable<D> for Shop
where
    D: CanWrite<String>
        + CanWrite<Option<String>>
        + CanWriteSliceWithArgs<SoldItem, WriteNullTermiantedSliceArgs>,
{
    fn to_writer_unboxed(&self, ctx: &mut impl vivibin::WriteCtx, domain: &mut D) -> Result<()> {
        domain.write(ctx, &self.shop_id)?;
        domain.write_slice_args_fallback(ctx, &self.items, WriteNullTermiantedSliceArgs {
            symbol_name: Some(SymbolName::Internal('s')),
        })?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Readable, Writable, Deserialize, Serialize)]
pub struct SoldItem {
    #[require_domain]
    pub item_id: Option<String>,
    pub requirement: Option<String>,
}
