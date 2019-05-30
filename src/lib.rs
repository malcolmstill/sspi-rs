// FromPrimitive and ToPrimitive causes clippy error, so we disable it until
// https://github.com/rust-num/num-derive/issues/20 is fixed
#![cfg_attr(feature = "cargo-clippy", allow(clippy::useless_attribute))]

mod encryption;
pub mod ntlm;
pub mod sspi;

pub use crate::ntlm::NTLM_VERSION_SIZE;
pub use crate::sspi::{Credentials, SspiError, SspiErrorType};

