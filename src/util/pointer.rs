// darxoon's small pointer utility v1, adapted for big endian
use core::{fmt::{self, Debug}, num::TryFromIntError, ops::{Add, Sub}, result};
use std::{io::{Cursor, Read, Seek, Write}};

use anyhow::Result;
use binrw::{BinRead, BinWrite};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

macro_rules! from_type {
    ($t:ident, $from:ty) => {
        impl From<$from> for $t {
            fn from(value: $from) -> Self {
                Pointer(value.into())
            }
        }
        
        impl Add<$from> for $t {
            type Output = Self;
        
            fn add(self, rhs: $from) -> Self {
                $t(self.0 + u32::from(rhs))
            }
        }
        
        impl Sub<$from> for $t {
            type Output = Self;
        
            fn sub(self, rhs: $from) -> Self {
                $t(self.0 - u32::from(rhs))
            }
        }
    };
}

macro_rules! from_type_unwrap {
    ($t:ident, $from:ty) => {
        impl From<$from> for $t {
            fn from(value: $from) -> Self {
                Pointer(value.try_into().unwrap())
            }
        }
        
        impl Add<$from> for $t {
            type Output = Self;
        
            fn add(self, rhs: $from) -> Self {
                // it's beautiful
                $t((i32::try_from(self.0).unwrap() + i32::try_from(rhs).unwrap()).try_into().unwrap())
            }
        }
        
        impl Sub<$from> for $t {
            type Output = Self;
        
            fn sub(self, rhs: $from) -> Self {
                $t((i32::try_from(self.0).unwrap() - i32::try_from(rhs).unwrap()).try_into().unwrap())
            }
        }
    };
}

macro_rules! into_type {
    ($t:ident, $into:ty) => {
        impl From<$t> for $into {
            fn from(value: $t) -> Self {
                value.0.into()
            }
        }
    };
}

macro_rules! into_type_unwrap {
    ($t:ident, $into:ty) => {
        impl From<$t> for $into {
            fn from(value: $t) -> Self {
                value.0.try_into().unwrap()
            }
        }
    };
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, BinRead, BinWrite)]
pub struct Pointer(pub u32);

impl Pointer {
    pub fn new(x: u32) -> Option<Self> {
        (x != 0).then_some(Pointer(x))
    }
    
    pub fn current<S: Seek>(seek: &mut S) -> Result<Self> {
        Ok(Pointer(seek.stream_position()?.try_into()?))
    }
    
    pub fn read(reader: &mut impl Read) -> Result<Option<Pointer>> {
        let value = reader.read_u32::<BigEndian>()?;
        
        if value != 0 {
            Ok(Some(Pointer(value)))
        } else {
            Ok(None)
        }
    }
    
    pub fn read_relative<R: Read + Seek>(reader: &mut R) -> Result<Option<Pointer>> {
        let reader_pos = reader.stream_position()?;
        let value = reader.read_u32::<BigEndian>()?;
        
        if value != 0 {
            Ok(Some(Pointer(value) + reader_pos))
        } else {
            Ok(None)
        }
    }
    
    pub fn write(&self, writer: &mut impl Write) -> Result<()> {
        writer.write_u32::<BigEndian>(self.0)?;
        Ok(())
    }
    
    pub fn write_option(pointer: Option<Self>, writer: &mut impl Write) -> Result<()> {
        if let Some(pointer) = pointer {
            pointer.write(writer)?;
        }
        Ok(())
    }
}

impl Debug for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Pointer({:#x})", self.0))
    }
}

impl Add<Self> for Pointer {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Pointer(self.0 + rhs.0)
    }
}

impl Sub<Self> for Pointer {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Pointer(self.0 - rhs.0)
    }
}

impl<T> TryFrom<&Cursor<T>> for Pointer {
    type Error = TryFromIntError;

    fn try_from(value: &Cursor<T>) -> result::Result<Self, Self::Error> {
        Ok(Pointer(value.position().try_into()?))
    }
}

impl<T> TryFrom<&&Cursor<T>> for Pointer {
    type Error = TryFromIntError;

    fn try_from(value: &&Cursor<T>) -> result::Result<Self, Self::Error> {
        Ok(Pointer(value.position().try_into()?))
    }
}

impl<T> TryFrom<&mut Cursor<T>> for Pointer {
    type Error = TryFromIntError;

    fn try_from(value: &mut Cursor<T>) -> result::Result<Self, Self::Error> {
        Ok(Pointer(value.position().try_into()?))
    }
}

impl<T> TryFrom<&&mut Cursor<T>> for Pointer {
    type Error = TryFromIntError;

    fn try_from(value: &&mut Cursor<T>) -> result::Result<Self, Self::Error> {
        Ok(Pointer(value.position().try_into()?))
    }
}

from_type!(Pointer, u32);

from_type_unwrap!(Pointer, i32);
from_type_unwrap!(Pointer, u64);
from_type_unwrap!(Pointer, i64);
from_type_unwrap!(Pointer, usize);

into_type!(Pointer, u32);
into_type!(Pointer, u64);
into_type!(Pointer, i64);

into_type_unwrap!(Pointer, i32);
into_type_unwrap!(Pointer, usize);
