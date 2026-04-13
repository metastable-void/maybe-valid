use ::core::char::CharTryFromError;
use ::core::ffi::CStr;
use ::core::str::Utf8Error;

use ::core::ffi::FromBytesWithNulError;
use ::core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

use crate::*;

/// Returned on the invalid path when validating an integer as a
/// corresponding `NonZero*` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZeroReason;

/// Returned on the invalid path when validating bytes as `CStr` or
/// `CString`.
///
/// `CStr` borrow validation can report specific positional reasons.
/// `CString` owned validation in `alloc` mode may only report
/// `Unspecified`, because `FromVecWithNulError` does not expose
/// detailed failure information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CStrInvalidReason {
    /// No nul terminator found within the byte slice.
    MissingNul,
    /// An interior nul byte was found at this position.
    InteriorNul { position: usize },
    /// Unspecified reason.
    ///
    /// Used when validating into `CString`, where the standard library
    /// does not expose more specific diagnostics.
    Unspecified,
}

impl Validated for str {
    type InvalidReason = Utf8Error;
}

impl AsValidated<str> for [u8] {
    fn as_validated(&self) -> MaybeValidRef<'_, str, Self> {
        match ::core::str::from_utf8(self) {
            Ok(v) => MaybeValidRef::Valid(v),
            Err(e) => MaybeValidRef::Invalid(self, e),
        }
    }
}

impl Validated for CStr {
    type InvalidReason = CStrInvalidReason;
}

impl AsValidated<CStr> for [u8] {
    fn as_validated(&self) -> MaybeValidRef<'_, CStr, Self> {
        match CStr::from_bytes_with_nul(self) {
            Ok(v) => MaybeValidRef::Valid(v),
            Err(FromBytesWithNulError::InteriorNul { position }) => {
                MaybeValidRef::Invalid(self, CStrInvalidReason::InteriorNul { position })
            }
            Err(FromBytesWithNulError::NotNulTerminated) => {
                MaybeValidRef::Invalid(self, CStrInvalidReason::MissingNul)
            }
        }
    }
}

impl Validated for char {
    type InvalidReason = CharTryFromError;
}

impl IntoValidated<char> for u32 {
    fn into_validated(self) -> MaybeValidOwned<char, Self> {
        match char::try_from(self) {
            Ok(v) => MaybeValidOwned::Valid(v),
            Err(e) => MaybeValidOwned::Invalid(self, e),
        }
    }
}

macro_rules! impl_nonzero {
    ($nz:ty, $raw:ty) => {
        impl Validated for $nz {
            type InvalidReason = ZeroReason;
        }

        impl AsValidated<$nz> for $raw {
            fn as_validated(&self) -> MaybeValidRef<'_, $nz, $raw> {
                // SAFETY: $nz is #[repr(transparent)] over $raw in std,
                // and the predicate (value != 0) holds here, so the
                // bit pattern is a valid $nz.
                if *self != 0 {
                    let nz = unsafe { &*(self as *const $raw as *const $nz) };
                    MaybeValidRef::Valid(nz)
                } else {
                    MaybeValidRef::Invalid(self, ZeroReason)
                }
            }
        }

        impl IntoValidated<$nz> for $raw {
            fn into_validated(self) -> MaybeValidOwned<$nz, Self> {
                match <$nz>::new(self) {
                    Some(nz) => MaybeValidOwned::Valid(nz),
                    None => MaybeValidOwned::Invalid(self, ZeroReason),
                }
            }
        }
    };
}

impl_nonzero!(NonZeroU8, u8);
impl_nonzero!(NonZeroU16, u16);
impl_nonzero!(NonZeroU32, u32);
impl_nonzero!(NonZeroU64, u64);
impl_nonzero!(NonZeroU128, u128);
impl_nonzero!(NonZeroUsize, usize);
impl_nonzero!(NonZeroI8, i8);
impl_nonzero!(NonZeroI16, i16);
impl_nonzero!(NonZeroI32, i32);
impl_nonzero!(NonZeroI64, i64);
impl_nonzero!(NonZeroI128, i128);
impl_nonzero!(NonZeroIsize, isize);
