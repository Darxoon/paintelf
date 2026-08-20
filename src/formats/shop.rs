use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use vivibin::{CanRead, CanWrite, Readable, Reader, Writable, WriteCtx};

use crate::{
    SymbolName,
    binutil::{
        DataCategory, ElfReadDomain, ElfWriteDomain, InlineString, ReadNullTerminatedVecArgs,
        WriteNullTerminatedSliceArgs,
    },
    formats::FileData,
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

pub fn write_shops(ctx: &mut WriteCtx<DataCategory>, domain: &mut ElfWriteDomain, shops: &[Shop]) -> Result<()> {
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

#[derive(Clone, Debug, Readable, Writable, Deserialize, Serialize)]
#[extra_read_domain_deps(CanRead<Option<String>>)]
#[extra_write_domain_deps(CanWrite<Cat, Option<String>>)]
pub struct Shop {
    #[require_domain]
    #[write_args(InlineString)]
    pub shop_id: String,
    
    #[read_args(ReadNullTerminatedVecArgs)]
    #[write_args(WriteNullTerminatedSliceArgs {
        symbol_name: Some(SymbolName::Internal('s')),
        write_length: false,
    })]
    pub items: Vec<SoldItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Readable, Writable, Deserialize, Serialize)]
pub struct SoldItem {
    #[require_domain]
    pub item_id: Option<String>,
    pub requirement: Option<String>,
}
