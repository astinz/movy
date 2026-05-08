#[cfg(feature = "sui")]
pub use movy_replay::tracer::op::{CastLog, CmpLog, CmpOp, Log, Magic, ShlLog};

#[cfg(not(feature = "sui"))]
use std::fmt::Display;

#[cfg(not(feature = "sui"))]
use alloy_primitives::U256;
#[cfg(not(feature = "sui"))]
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "sui"))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Magic {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(U256),
    Bytes(Vec<u8>),
}

#[cfg(not(feature = "sui"))]
impl Display for Magic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::U8(v) => f.write_fmt(format_args!("U8({})", v)),
            Self::U16(v) => f.write_fmt(format_args!("U16({})", v)),
            Self::U32(v) => f.write_fmt(format_args!("U32({})", v)),
            Self::U64(v) => f.write_fmt(format_args!("U64({})", v)),
            Self::U128(v) => f.write_fmt(format_args!("U128({})", v)),
            Self::U256(v) => f.write_fmt(format_args!("U256({})", v)),
            Self::Bytes(v) => f.write_fmt(format_args!("Bytes({})", const_hex::encode(v))),
        }
    }
}

#[cfg(not(feature = "sui"))]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy)]
pub enum CmpOp {
    LT,
    LE,
    GT,
    GE,
    NEQ,
    EQ,
}

#[cfg(not(feature = "sui"))]
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CmpLog {
    pub lhs: Magic,
    pub rhs: Magic,
    pub op: CmpOp,
}

#[cfg(not(feature = "sui"))]
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ShlLog {
    pub lhs: Magic,
    pub rhs: Magic,
}

#[cfg(not(feature = "sui"))]
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct CastLog {
    pub lhs: Magic,
}

#[cfg(not(feature = "sui"))]
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Log {
    CmpLog(CmpLog),
    ShlLog(ShlLog),
    CastLog(CastLog),
}
