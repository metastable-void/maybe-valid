# maybe-valid

`maybe-valid` is a small Rust crate for structural validation/refinement.

This crate intentionally uses dedicated outcome enums (`MaybeValidRef` and
`MaybeValidOwned`) rather than a `Result`-shaped API, so valid/invalid states
and precursor recovery stay explicit.

## API at a glance

- `Validated`: target type defines a canonical `InvalidReason`.
- `AsValidated<V>`: borrow-based validation into `&V`.
- `IntoValidated<V>`: owning validation into `V`, returning the original input on invalid.
- `MaybeValidRef<V, P>` and `MaybeValidOwned<V, P>`: outcome enums for borrowed/owned paths.

## Built-in conversions

- `[u8] -> str` (UTF-8 validation)
- `[u8] -> CStr` (nul checks)
- `u32 -> char`
- integer primitives -> `NonZero*`
- `Vec<u8> -> String` (`alloc`)
- `Vec<u8> -> CString` (`alloc`)

## Features

- `std` (default): enables `alloc` and std-friendly usage.
- `alloc`: enables owned string/C string conversions.
- no default features: core/no-alloc functionality.

## Example

```rust
use maybe_valid::{AsValidated, IntoValidated, MaybeValidOwned, MaybeValidRef};

let bytes: &[u8] = b"hello";
let borrowed: MaybeValidRef<'_, str, [u8]> = bytes.as_validated();
assert!(borrowed.is_valid());

let raw = vec![0xff, 0xfe];
let owned: MaybeValidOwned<String, Vec<u8>> = raw.into_validated();
assert!(owned.is_invalid());
```


## License

**This crate is dual-licensed under `Apache-2.0 OR MPL-2.0`**;
either license is sufficient; choose whichever fits your project.

**Rationale**: We generally want our reusable Rust crates to be
under a license permissive enough to be friendly for the Rust
community as a whole, while maintaining GPL-2.0 compatibility via
the MPL-2.0 arm. This is FSF-safer for everyone than `MIT OR Apache-2.0`,
still being permissive. **This is the standard licensing** for our reusable
Rust crate projects. Someone's `GPL-2.0-or-later` project should not be
forced to drop the `GPL-2.0` option because of our crates,
while `Apache-2.0` is the non-copyleft (permissive) license recommended
by the FSF, which we base our decisions on.
