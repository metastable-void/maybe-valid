#![cfg_attr(not(feature = "std"), no_std)]
//! `maybe-valid` provides traits and dedicated outcome enums for
//! structural validation/refinement conversions.
//!
//! Core pieces:
//! - [`Validated`]: declares a canonical diagnostic reason type for a
//!   validated target.
//! - [`AsValidated`]: borrow-based validation (`&Self` -> `&V`).
//! - [`IntoValidated`]: owning validation (`Self` -> `V`, with precursor
//!   recovery on failure).
//! - [`MaybeValidRef`] and [`MaybeValidOwned`]: explicit valid/invalid
//!   outcomes (intentionally distinct from `Result`) that carry
//!   precursor data on the invalid branch.
//!
//! Feature flags:
//! - `std` (default): enables `alloc` and all provided std/alloc-backed impls.
//! - `alloc`: enables owned string/C string conversions.
//! - no default features: keeps core/no-alloc functionality.

#[cfg(feature = "alloc")]
extern crate alloc;

mod api;
mod impls;

pub use api::*;
pub use impls::core_impls::{CStrInvalidReason, ZeroReason};
