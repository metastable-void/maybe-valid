use core::ffi::CStr;
use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

use maybe_valid::{
    AsValidated, CStrInvalidReason, IntoValidated, MaybeValidOwned, MaybeValidRef, ZeroReason,
};

#[test]
fn utf8_as_validated_valid_and_invalid() {
    let valid_bytes: &[u8] = b"hello";
    let valid: MaybeValidRef<'_, str, [u8]> = valid_bytes.as_validated();
    assert!(valid.is_valid());
    assert!(!valid.is_invalid());
    assert_eq!(valid.valid(), Some("hello"));

    let invalid_bytes: &[u8] = &[0xff, 0x61];
    let invalid: MaybeValidRef<'_, str, [u8]> = invalid_bytes.as_validated();
    assert!(invalid.is_invalid());
    assert!(!invalid.is_valid());
    let (precursor, reason) = invalid.invalid_parts().expect("expected invalid");
    assert_eq!(precursor, invalid_bytes);
    assert_eq!(reason.valid_up_to(), 0);
}

#[test]
fn cstr_as_validated_reasons() {
    let valid_bytes: &[u8] = b"ok\0";
    let valid: MaybeValidRef<'_, CStr, [u8]> = valid_bytes.as_validated();
    match valid {
        MaybeValidRef::Valid(cstr) => assert_eq!(cstr.to_bytes_with_nul(), b"ok\0"),
        MaybeValidRef::Invalid(_, _) => panic!("expected valid CStr"),
    }

    let missing_nul: &[u8] = b"ok";
    let missing: MaybeValidRef<'_, CStr, [u8]> = missing_nul.as_validated();
    match missing {
        MaybeValidRef::Valid(_) => panic!("expected missing nul"),
        MaybeValidRef::Invalid(bytes, reason) => {
            assert_eq!(bytes, missing_nul);
            assert_eq!(reason, CStrInvalidReason::MissingNul);
        }
    }

    let interior_nul: &[u8] = b"a\0b\0";
    let interior: MaybeValidRef<'_, CStr, [u8]> = interior_nul.as_validated();
    match interior {
        MaybeValidRef::Valid(_) => panic!("expected interior nul"),
        MaybeValidRef::Invalid(bytes, reason) => {
            assert_eq!(bytes, interior_nul);
            assert_eq!(reason, CStrInvalidReason::InteriorNul { position: 1 });
        }
    }
}

#[test]
fn char_into_validated() {
    let valid: MaybeValidOwned<char, u32> = 0x41_u32.into_validated();
    assert!(valid.is_valid());
    assert_eq!(valid.valid(), Some('A'));

    let invalid: MaybeValidOwned<char, u32> = 0x11_0000_u32.into_validated();
    assert!(invalid.is_invalid());
    assert_eq!(invalid.invalid_precursor(), Some(0x11_0000_u32));
}

macro_rules! test_nonzero {
    ($raw:ty, $nz:ty, $one:expr, $zero:expr) => {{
        let valid_ref: MaybeValidRef<'_, $nz, $raw> = ($one as $raw).as_validated();
        assert!(valid_ref.is_valid());

        let invalid_ref: MaybeValidRef<'_, $nz, $raw> = ($zero as $raw).as_validated();
        match invalid_ref {
            MaybeValidRef::Valid(_) => panic!("expected invalid non-zero ref"),
            MaybeValidRef::Invalid(v, reason) => {
                assert_eq!(*v, $zero as $raw);
                assert_eq!(reason, ZeroReason);
            }
        }

        let valid_owned: MaybeValidOwned<$nz, $raw> = ($one as $raw).into_validated();
        assert!(valid_owned.is_valid());

        let invalid_owned: MaybeValidOwned<$nz, $raw> = ($zero as $raw).into_validated();
        match invalid_owned {
            MaybeValidOwned::Valid(_) => panic!("expected invalid non-zero owned"),
            MaybeValidOwned::Invalid(v, reason) => {
                assert_eq!(v, $zero as $raw);
                assert_eq!(reason, ZeroReason);
            }
        }
    }};
}

#[test]
fn nonzero_conversions_cover_all_integer_variants() {
    test_nonzero!(u8, NonZeroU8, 1, 0);
    test_nonzero!(u16, NonZeroU16, 1, 0);
    test_nonzero!(u32, NonZeroU32, 1, 0);
    test_nonzero!(u64, NonZeroU64, 1, 0);
    test_nonzero!(u128, NonZeroU128, 1, 0);
    test_nonzero!(usize, NonZeroUsize, 1, 0);
    test_nonzero!(i8, NonZeroI8, 1, 0);
    test_nonzero!(i16, NonZeroI16, 1, 0);
    test_nonzero!(i32, NonZeroI32, 1, 0);
    test_nonzero!(i64, NonZeroI64, 1, 0);
    test_nonzero!(i128, NonZeroI128, 1, 0);
    test_nonzero!(isize, NonZeroIsize, 1, 0);
}

#[cfg(feature = "alloc")]
#[test]
fn maybe_valid_ref_methods_and_into_owned() {
    let valid_bytes: &[u8] = b"ok";
    let valid: MaybeValidRef<'_, str, [u8]> = valid_bytes.as_validated();
    let valid_borrowed = valid.as_ref();
    assert!(matches!(valid_borrowed, MaybeValidRef::Valid("ok")));
    let valid_result = valid.into_result();
    assert_eq!(valid_result, Ok("ok"));

    let invalid_bytes: &[u8] = &[0xff];
    let invalid: MaybeValidRef<'_, str, [u8]> = invalid_bytes.as_validated();
    let invalid_reason = invalid
        .as_ref()
        .into_result_reason_only()
        .expect_err("expected invalid");
    assert_eq!(invalid_reason.valid_up_to(), 0);

    let invalid_owned = invalid.into_owned();
    match invalid_owned {
        MaybeValidOwned::Valid(_) => panic!("expected invalid owned"),
        MaybeValidOwned::Invalid(bytes, reason) => {
            assert_eq!(bytes, vec![0xff]);
            assert_eq!(reason.valid_up_to(), 0);
        }
    }
}

#[cfg(feature = "alloc")]
#[test]
fn maybe_valid_owned_methods() {
    let invalid: MaybeValidOwned<String, Vec<u8>> = vec![0xff].into_validated();
    assert!(invalid.is_invalid());
    assert!(!invalid.is_valid());

    let invalid_ref = invalid.as_ref();
    assert!(invalid_ref.is_invalid());

    let (bytes, reason) = invalid.into_result().expect_err("expected invalid");
    assert_eq!(bytes, vec![0xff]);
    assert_eq!(reason.valid_up_to(), 0);
}

#[cfg(feature = "alloc")]
#[test]
fn string_and_cstring_into_validated() {
    let s_valid: MaybeValidOwned<String, Vec<u8>> = b"hello".to_vec().into_validated();
    assert_eq!(s_valid.valid().as_deref(), Some("hello"));

    let s_invalid: MaybeValidOwned<String, Vec<u8>> = vec![0xff, 0xfe].into_validated();
    match s_invalid {
        MaybeValidOwned::Valid(_) => panic!("expected invalid UTF-8"),
        MaybeValidOwned::Invalid(bytes, reason) => {
            assert_eq!(bytes, vec![0xff, 0xfe]);
            assert_eq!(reason.valid_up_to(), 0);
        }
    }

    let c_valid: MaybeValidOwned<std::ffi::CString, Vec<u8>> = b"ok\0".to_vec().into_validated();
    match c_valid {
        MaybeValidOwned::Valid(cstr) => assert_eq!(cstr.to_bytes_with_nul(), b"ok\0"),
        MaybeValidOwned::Invalid(_, _) => panic!("expected valid CString"),
    }

    let c_invalid_missing: MaybeValidOwned<std::ffi::CString, Vec<u8>> =
        b"ok".to_vec().into_validated();
    match c_invalid_missing {
        MaybeValidOwned::Valid(_) => panic!("expected invalid CString"),
        MaybeValidOwned::Invalid(bytes, reason) => {
            assert_eq!(bytes, b"ok".to_vec());
            assert_eq!(reason, CStrInvalidReason::Unspecified);
        }
    }

    let c_invalid_interior: MaybeValidOwned<std::ffi::CString, Vec<u8>> =
        b"a\0b\0".to_vec().into_validated();
    match c_invalid_interior {
        MaybeValidOwned::Valid(_) => panic!("expected invalid CString"),
        MaybeValidOwned::Invalid(bytes, reason) => {
            assert_eq!(bytes, b"a\0b\0".to_vec());
            assert_eq!(reason, CStrInvalidReason::Unspecified);
        }
    }
}
