use serde::{Deserialize, Serialize};

use crate::formats::maplink::MaplinkArea;

pub mod maplink;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileData {
    Maplink(Vec<MaplinkArea>),
}
