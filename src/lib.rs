#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod api;
mod impls;

pub use api::*;
pub use impls::core_impls::{CStrInvalidReason, ZeroReason};
