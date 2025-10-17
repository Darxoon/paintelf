use std::fmt::Display;

use vivibin::HeapToken;

pub mod binutil;
pub mod elf;
pub mod elf_container;
pub mod formats;
pub mod matching;
pub mod util;

#[derive(Clone, Debug)]
pub enum SymbolName {
    None,
    Internal(char),
    InternalNamed(String),
    InternalUnmangled(String),
    Unmangled(String),
}

impl SymbolName {
    pub fn is_internal(&self) -> bool {
        matches!(self, SymbolName::Internal(_) | SymbolName::InternalNamed(_) | SymbolName::InternalUnmangled(_))
    }
    
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SymbolName::None => None,
            SymbolName::Internal(_) => None,
            SymbolName::InternalNamed(name)
            | SymbolName::InternalUnmangled(name)
            | SymbolName::Unmangled(name) => Some(name),
        }
    }
}

impl Display for SymbolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolName::None => write!(f, "<none>"),
            SymbolName::Internal(initial_char) => write!(f, "{initial_char}<???>"),
            SymbolName::InternalNamed(name)
            | SymbolName::InternalUnmangled(name)
            | SymbolName::Unmangled(name) => write!(f, "{name}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SymbolDeclaration {
    pub name: SymbolName,
    pub offset: HeapToken,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct RelDeclaration {
    pub base_location: usize,
    pub target_location: usize,
}
