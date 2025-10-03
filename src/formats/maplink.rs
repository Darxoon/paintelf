use std::io::SeekFrom;

use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt};
use serde::Serialize;
use vivibin::{scoped_reader_pos, CanRead, ReadDomainExt, Readable, Reader};

use crate::{formats::FileData, util::pointer::Pointer, ElfDomain};

pub fn read_maplink<'a>(reader: &mut impl Reader, domain: ElfDomain) -> Result<FileData> {
    let data_count_symbol = domain.find_symbol("dataCount__Q3_4data3fld7maplink")?;
    reader.seek(SeekFrom::Start(data_count_symbol.offset().into()))?;
    let data_count = reader.read_u32::<BigEndian>()?;
    
    let datas_symbol = domain.find_symbol("datas__Q3_4data3fld7maplink")?;
    reader.seek(SeekFrom::Start(datas_symbol.offset().into()))?;
    
    let areas: Vec<MaplinkArea> = (0..data_count)
        .map(|_| MaplinkArea::from_reader(reader, domain))
        .collect::<Result<_>>()?;
    
    Ok(FileData::Maplink(areas))
}

#[derive(Clone, Debug, Serialize)]
pub struct MaplinkArea {
    pub map_name: String,
    pub links: Vec<Link>,
}

impl<D: CanRead<String> + CanRead<Pointer>> Readable<D> for MaplinkArea {
    fn from_reader<R: vivibin::Reader>(reader: &mut R, domain: D) -> Result<Self> {
        let map_name: String = domain.read(reader)?;
        let links_ptr: Pointer = domain.read(reader)?;
        let link_count: u32 = domain.read_fallback(reader)?;
        
        scoped_reader_pos!(reader);
        reader.seek(SeekFrom::Start(links_ptr.into()))?;
        
        let links: Vec<Link> = (0..link_count)
            .map(|_| Link::from_reader(reader, domain))
            .collect::<Result<_>>()?;
        
        Ok(Self {
            map_name,
            links,
        })
    }
}

#[derive(Clone, Debug, Readable, Serialize)]
pub struct Link {
    #[require_domain]
    pub id: String,
    pub destination: String,
    pub link_type: String,
    pub field_0xc: String,
    pub field_0x10: f32,
    pub field_0x14: String,
    pub field_0x18: String,
    pub field_0x1c: String,
    pub field_0x20: String,
    pub field_0x24: String,
    pub field_0x28: u32,
    pub field_0x2c: String,
    pub field_0x30: String,
    pub field_0x34: String,
    pub field_0x38: String,
}
