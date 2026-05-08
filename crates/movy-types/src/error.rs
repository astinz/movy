use std::string::FromUtf8Error;

#[cfg(feature = "sui")]
use sui_types::base_types::ObjectIDParseError;
use thiserror::Error;

macro_rules! trivial {
    ($err:ty, $var:expr) => {
        impl From<$err> for MovyError {
            fn from(value: $err) -> Self {
                $var(value.into())
            }
        }
    };
}

macro_rules! trivial_other {
    ($err:ty) => {
        trivial!($err, MovyError::Other);
    };
}

#[derive(Error, Debug)]
pub enum MovyError {
    #[error("rpc error: {0}")]
    RPC(i32, String),
    #[error("json error: {0}")]
    STDJSON(#[from] serde_json::Error),
    #[error("toml error: {0}")]
    TOML(#[from] toml::de::Error),
    #[error("io error: {0}")]
    IO(#[from] std::io::Error),
    #[cfg(feature = "sui")]
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("any: {0}")]
    Any(#[from] anyhow::Error),
    #[error("bcs: {0}")]
    BCS(#[from] bcs::Error),
    #[cfg(feature = "sui")]
    #[error("oss: {0}")]
    OSS(#[from] object_store::Error),
    #[cfg(feature = "sui")]
    #[error("mdbx: {0}")]
    MDBX(#[from] mdbx_derive::mdbx::ClientError),
    #[cfg(feature = "sui")]
    #[error("derive: {0}")]
    Derive(#[from] mdbx_derive::Error),
    #[cfg(feature = "sui")]
    #[error("sui: {0}")]
    SUI(#[from] sui_types::error::SuiError),
    #[cfg(feature = "sui")]
    #[error("suisdk: {0}")]
    SUISDK(#[from] sui_sdk::error::Error),
    #[cfg(feature = "sui")]
    #[error("binary: {0}")]
    Binary(#[from] move_binary_format::errors::PartialVMError),
    #[cfg(feature = "fuzz")]
    #[error("libafl: {0}")]
    LIBAFL(#[from] libafl::Error),
    #[error("trace error: {0}, our fork is dead")]
    Trace(String),
    #[error("unsupported: {0}, please file an issue")]
    Unsupported(String),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),
    #[error("invalid seed: {0}")]
    InvalidSeed(String),
    #[cfg(feature = "sui")]
    #[error("grpc: {0}")]
    Tonic(#[from] tonic::Status),
    #[error(transparent)]
    Other(#[from] color_eyre::Report),
}

trivial_other!(FromUtf8Error);
#[cfg(feature = "sui")]
trivial_other!(fastcrypto::error::FastCryptoError);
#[cfg(feature = "sui")]
trivial_other!(sui_types::error::ExecutionError);
#[cfg(feature = "sui")]
trivial_other!(tokio::task::JoinError);
#[cfg(feature = "sui")]
trivial_other!(ObjectIDParseError);
#[cfg(feature = "sui")]
trivial_other!(glob::GlobError);
#[cfg(feature = "sui")]
trivial_other!(glob::PatternError);

#[cfg(feature = "fuzz")]
impl From<MovyError> for libafl::Error {
    fn from(value: MovyError) -> Self {
        match value {
            MovyError::LIBAFL(e) => e,
            _ => libafl::Error::runtime(format!("{:?}", value)),
        }
    }
}
