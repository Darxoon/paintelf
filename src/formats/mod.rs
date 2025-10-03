use serde::Serialize;

use crate::formats::maplink::MaplinkArea;

pub mod maplink;

#[derive(Clone, Debug, Serialize)]
pub enum FileData {
    Maplink(Vec<MaplinkArea>),
}
