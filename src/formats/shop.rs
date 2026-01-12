use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{CanRead, CanWriteWithArgs, Readable, Reader, Writable, WriteCtx, scoped_reader_pos};

use crate::{
    SymbolName,
    binutil::{DataCategory, ElfReadDomain, ElfWriteDomain, NewWriteNullTermiantedSliceArgs, NewWriteStringArgs},
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

pub fn write_shops(ctx: &mut impl WriteCtx<DataCategory>, domain: &mut ElfWriteDomain, shops: &[Shop]) -> Result<()> {
    let mut states = Vec::new();
    
    domain.write_symbol(ctx, "shopList__Q2_4data4shop", |domain, ctx| {
        for shop in shops {
            states.push(shop.to_writer(ctx, domain)?);
        }
        Ok(())
    })?;
    
    domain.write_symbol(ctx, "shopListLen__Q2_4data4shop", |domain, ctx| {
        (shops.len() as u32).to_writer(ctx, domain)
    })?;
    
    for (shop, state) in shops.iter().zip(states) {
        shop.to_writer_post(ctx, domain, state)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Writable, Deserialize, Serialize)]
#[extra_write_domain_deps(CanWriteWithArgs<Cat, Option<String>, NewWriteStringArgs>)]
#[new_serialization]
pub struct Shop {
    #[require_domain]
    #[write_args(NewWriteStringArgs::default())]
    pub shop_id: String,
    
    #[write_args(NewWriteNullTermiantedSliceArgs {
        symbol_name: Some(SymbolName::Internal('s')),
        write_length: false,
    })]
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Readable, Writable, Deserialize, Serialize)]
#[new_serialization]
pub struct SoldItem {
    #[require_domain]
    #[write_args(NewWriteStringArgs::default())]
    pub item_id: Option<String>,
    #[write_args(NewWriteStringArgs::default())]
    pub requirement: Option<String>,
}
