use ::core::str::Utf8Error;

use ::alloc::ffi::CString;
use ::alloc::string::String;

use crate::*;

impl Validated for String {
    type InvalidReason = Utf8Error;
}

impl IntoValidated<String> for Vec<u8> {
    fn into_validated(self) -> MaybeValidOwned<String, Self> {
        match String::from_utf8(self) {
            Ok(v) => MaybeValidOwned::Valid(v),
            Err(e) => {
                let err = e.utf8_error();
                MaybeValidOwned::Invalid(e.into_bytes(), err)
            }
        }
    }
}

impl Validated for CString {
    type InvalidReason = CStrInvalidReason;
}

impl IntoValidated<CString> for Vec<u8> {
    fn into_validated(self) -> MaybeValidOwned<CString, Self> {
        match CString::from_vec_with_nul(self) {
            Ok(v) => MaybeValidOwned::Valid(v),
            Err(e) => MaybeValidOwned::Invalid(e.into_bytes(), CStrInvalidReason::Unspecified),
        }
    }
}
