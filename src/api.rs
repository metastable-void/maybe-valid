/// A type whose values are guaranteed to satisfy a validation predicate.
///
/// Implementing `Validated` marks `Self` as the *validated form* of some
/// underlying data — typically a newtype around a raw type, or a `?Sized`
/// view type like [`str`] over [`[u8]`]. The implementation asserts that
/// every value of `Self` that can be constructed through the crate's
/// conversion traits satisfies the type's invariant.
///
/// This trait does not itself perform validation. It declares the
/// *canonical failure explanation* for the type, via the [`InvalidReason`]
/// associated type. Conversions into `Self` — whether borrowing
/// ([`AsValidated`]) or owning ([`IntoValidated`]) — report failure
/// using this type.
///
/// [`InvalidReason`]: Validated::InvalidReason
/// [`AsValidated`]: crate::AsValidated
/// [`IntoValidated`]: crate::IntoValidated
///
/// # Scope
///
/// This trait models structural refinement — the validated type shares
/// a representation with its precursor and differs only in which values
/// are permitted. It is not intended for general fallible conversion,
/// such as parsing a string into a binary value with different layout.
/// For those cases, use [`FromStr`] or [`TryFrom`].
///
/// # The role of `InvalidReason`
///
/// `InvalidReason` describes *why* a candidate value failed to satisfy
/// the predicate. It is purely diagnostic: it must not carry the
/// candidate value itself.
///
/// This restriction is deliberate. Precursor recovery — giving the
/// caller back their unvalidated input on failure — is handled
/// structurally by the conversion traits:
///
/// - [`AsValidated::as_validated`] returns a reference into `&self`,
///   so the caller retains access to the precursor through their
///   existing binding.
/// - [`IntoValidated::into_validated`] returns the owned precursor
///   alongside the `InvalidReason` in the invalid branch of
///   [`MaybeValidOwned`].
///
/// Keeping `InvalidReason` precursor-free allows a single
/// `Validated` impl to serve both borrowing and owning conversions
/// without duplicating data or forcing a clone on the owning path.
///
/// [`MaybeValidOwned`]: crate::MaybeValidOwned
///
/// # Contract
///
/// Implementers of `Validated` promise the following:
///
/// 1. **Predicate stability.** Whether a value of the underlying type
///    satisfies the predicate must depend only on the value's observable
///    state, not on external state, time, or interior mutability. A value
///    that was valid yesterday is valid today.
///
/// 2. **`InvalidReason` is diagnostic only.** The `InvalidReason` type
///    must not contain the candidate value or a copy of it. It may
///    contain positional information (byte offsets, field indices),
///    expected-versus-actual summaries, or any other explanation,
///    provided these are cheap to construct relative to the cost of
///    validation itself.
///
/// 3. **`InvalidReason` is cheaply constructible.** Constructing an
///    `InvalidReason` on the failure path should not be dramatically
///    more expensive than running the validation itself. In particular,
///    it should not allocate proportional to the candidate's size.
///
/// These are logical contracts, not compiler-enforced ones. Violating
/// them will not cause undefined behavior, but will break the guarantees
/// that generic code written against `Validated` relies on.
///
/// # Examples
///
/// The canonical example is [`str`] as the validated form of `[u8]`:
///
/// ```
/// # use maybe_valid::Validated;
/// # use std::str::Utf8Error;
/// fn assert_validated_utf8<T: Validated<InvalidReason = Utf8Error> + ?Sized>() {}
/// assert_validated_utf8::<str>();
/// ```
///
/// `Utf8Error` carries a `valid_up_to: usize` and an `error_len:
/// Option<u8>`. It explains where UTF-8 validation failed, without
/// holding the `[u8]` that failed — the caller retains that through
/// their own binding or through [`IntoValidated`]'s return.
///
/// A custom newtype over `[u8]` restricting to ASCII:
///
/// ```
/// # use maybe_valid::Validated;
/// #[repr(transparent)]
/// pub struct Ascii([u8]);
///
/// pub struct NonAsciiReason {
///     /// Byte offset of the first non-ASCII byte.
///     pub position: usize,
///     /// The offending byte value.
///     pub byte: u8,
/// }
///
/// impl Validated for Ascii {
///     type InvalidReason = NonAsciiReason;
/// }
/// ```
///
/// A refinement of an integer type:
///
/// ```
/// # use maybe_valid::{Validated, ZeroReason};
/// # use std::num::NonZeroU32;
/// fn assert_nonzero_reason<T: Validated<InvalidReason = ZeroReason>>() {}
/// assert_nonzero_reason::<NonZeroU32>();
/// ```
///
/// # When *not* to implement `Validated`
///
/// `Validated` is not a general-purpose "this type has an invariant"
/// marker. Implement it only when:
///
/// - The type is the canonical validated form of some underlying data,
///   in the sense that asking "is this underlying value a valid `Self`?"
///   is a meaningful question with a single canonical predicate.
/// - You intend to provide [`AsValidated`] or [`IntoValidated`] impls
///   from one or more precursor types. A `Validated` impl with no
///   corresponding conversions is inert.
///
/// Types with multiple equally canonical predicates (e.g., a byte slice
/// that might be validated as UTF-8, as ASCII, or as valid JSON
/// depending on context) should be modeled by having *multiple*
/// validated target types, each implementing `Validated` with its own
/// `InvalidReason`, rather than a single type with a configurable
/// predicate.
///
/// # Relationship to other traits
///
/// `Validated` occupies a different niche from:
///
/// - [`TryFrom`]: expresses any fallible conversion with a
///   caller-chosen error type. `Validated` pins the error type to the
///   target and specifies its role (diagnostic only).
/// - [`FromStr`]: fallible parsing from `&str` with an associated
///   `Err`. Similar shape, but single-source and not a trait family
///   for both borrowing and owning conversions.
/// - [`Borrow`]: infallible borrowing with a `Hash`/`Eq`/`Ord`
///   agreement contract. `Validated` makes no hash-agreement claim;
///   conversions are permitted to produce views with different hashing
///   semantics.
///
/// [`TryFrom`]: core::convert::TryFrom
/// [`FromStr`]: core::str::FromStr
/// [`Borrow`]: core::borrow::Borrow
pub trait Validated {
    /// The explanation returned when a candidate value fails to
    /// satisfy this type's validation predicate.
    ///
    /// Must be diagnostic-only: see the trait-level contract for
    /// the restrictions on what this type may contain.
    type InvalidReason;
}

/// The outcome of borrowing a value as a validated view of type `V`.
///
/// Returned by [`AsValidated::as_validated`]. Both variants carry a
/// reference into the caller's original value — the `Valid` variant
/// as `&V`, the `Invalid` variant as `&P` (the precursor). The
/// `Invalid` variant additionally carries the diagnostic reason.
///
/// [`AsValidated::as_validated`]: crate::AsValidated::as_validated
///
/// # Why the precursor is repeated in `Invalid`
///
/// On the invalid path, the caller could reach the precursor through
/// their existing `&self` binding; including it in the `Invalid`
/// variant is structurally redundant. It is retained anyway because:
///
/// - The type then honestly describes both outcomes as "a reference
///   into the caller's value, plus (on the invalid path) a reason."
///   Readers do not have to infer the precursor's availability from
///   context.
///
/// - The shape mirrors [`MaybeValidOwned`], where the precursor must
///   be returned structurally because consumption would otherwise lose
///   it. Generic code and documentation can describe both enums in
///   parallel.
///
/// - The cost is a pointer-sized move, not a clone or allocation.
///
/// [`MaybeValidOwned`]: crate::MaybeValidOwned
///
/// # Why both variants matter
///
/// `MaybeValidRef` is deliberately not a [`Result`]. The `Invalid`
/// variant is not an error to handle and discard: it is a structured
/// peer of `Valid`, describing the state of data the caller may wish
/// to continue working with (rendering a partial view, producing a
/// repair, emitting a diagnostic that references the caller's own
/// bytes).
///
/// # Examples
///
/// ```
/// # use maybe_valid::{AsValidated, MaybeValidRef};
/// let bytes: &[u8] = b"hello";
/// let validated: MaybeValidRef<'_, str, [u8]> = bytes.as_validated();
/// match validated {
///     MaybeValidRef::Valid(s) => assert_eq!(s, "hello"),
///     MaybeValidRef::Invalid(bytes, reason) => {
///         eprintln!(
///             "invalid at byte {} of {} total",
///             reason.valid_up_to(),
///             bytes.len(),
///         );
///     }
/// }
/// ```
///
/// # Construction
///
/// `MaybeValidRef` values are produced by [`AsValidated`]
/// implementations. Direct construction is public and unrestricted:
/// `V`'s own invariants are enforced by `V`, not by this enum.
///
/// [`AsValidated`]: crate::AsValidated
pub enum MaybeValidRef<'a, V: Validated + ?Sized, P: ?Sized> {
    /// The borrowed value satisfies `V`'s predicate.
    Valid(&'a V),

    /// The borrowed value does not satisfy `V`'s predicate.
    ///
    /// Holds a reference to the original precursor (aliasing the
    /// caller's value) and the diagnostic reason.
    Invalid(&'a P, V::InvalidReason),
}

impl<'a, V: Validated + ?Sized, P: ?Sized> MaybeValidRef<'a, V, P> {
    /// Returns `true` if this is the `Valid` variant.
    pub fn is_valid(&self) -> bool {
        matches!(self, MaybeValidRef::Valid(_))
    }

    /// Returns `true` if this is the `Invalid` variant.
    pub fn is_invalid(&self) -> bool {
        matches!(self, MaybeValidRef::Invalid(_, _))
    }

    /// Returns the validated reference, or `None` if invalid.
    pub fn valid(self) -> Option<&'a V> {
        match self {
            MaybeValidRef::Valid(v) => Some(v),
            MaybeValidRef::Invalid(_, _) => None,
        }
    }

    /// Returns the precursor reference on the invalid path, or `None`
    /// if valid.
    pub fn invalid_precursor(self) -> Option<&'a P> {
        match self {
            MaybeValidRef::Valid(_) => None,
            MaybeValidRef::Invalid(p, _) => Some(p),
        }
    }

    /// Returns the invalid reason, or `None` if valid.
    ///
    /// Discards the precursor reference; use [`invalid_parts`] to
    /// retain both.
    ///
    /// [`invalid_parts`]: MaybeValidRef::invalid_parts
    pub fn invalid_reason(self) -> Option<V::InvalidReason> {
        match self {
            MaybeValidRef::Valid(_) => None,
            MaybeValidRef::Invalid(_, r) => Some(r),
        }
    }

    /// Returns the precursor reference and reason on the invalid path,
    /// or `None` if valid.
    pub fn invalid_parts(self) -> Option<(&'a P, V::InvalidReason)> {
        match self {
            MaybeValidRef::Valid(_) => None,
            MaybeValidRef::Invalid(p, r) => Some((p, r)),
        }
    }

    /// Returns a `MaybeValidRef` that borrows from this one, with the
    /// same variant structure.
    ///
    /// Useful when a caller holds a `MaybeValidRef` by value but needs
    /// to inspect it without consuming it. The returned value borrows
    /// the precursor/validated references from `self` and clones the
    /// `InvalidReason` on the invalid path.
    pub fn as_ref(&self) -> MaybeValidRef<'_, V, P>
    where
        V::InvalidReason: Clone,
    {
        match self {
            MaybeValidRef::Valid(v) => MaybeValidRef::Valid(v),
            MaybeValidRef::Invalid(p, r) => MaybeValidRef::Invalid(p, r.clone()),
        }
    }

    /// Converts into a `Result`, discarding the peer framing and
    /// bundling the precursor reference into the error.
    ///
    /// Useful when integrating with code written against `Result` and
    /// `?`, at the cost of the explicit-match ergonomics
    /// `MaybeValidRef` encourages.
    pub fn into_result(self) -> Result<&'a V, (&'a P, V::InvalidReason)> {
        match self {
            MaybeValidRef::Valid(v) => Ok(v),
            MaybeValidRef::Invalid(p, r) => Err((p, r)),
        }
    }

    /// Converts into a `Result` that carries only the reason on the
    /// error path, discarding the precursor reference.
    ///
    /// Prefer [`into_result`] when the precursor is still useful to
    /// the caller; this method is a convenience for call sites that
    /// only need the diagnostic.
    ///
    /// [`into_result`]: MaybeValidRef::into_result
    pub fn into_result_reason_only(self) -> Result<&'a V, V::InvalidReason> {
        match self {
            MaybeValidRef::Valid(v) => Ok(v),
            MaybeValidRef::Invalid(_, r) => Err(r),
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, V, P> MaybeValidRef<'a, V, P>
where
    V: Validated + ::alloc::borrow::ToOwned + ?Sized,
    V::Owned: Validated<InvalidReason = V::InvalidReason>,
    P: ::alloc::borrow::ToOwned + ?Sized,
{
    /// Produces an owned version of this outcome by cloning the
    /// borrowed `V` (or `P`) into its owned form.
    pub fn into_owned(self) -> MaybeValidOwned<V::Owned, P::Owned> {
        match self {
            MaybeValidRef::Valid(v) => MaybeValidOwned::Valid(v.to_owned()),
            MaybeValidRef::Invalid(p, r) => MaybeValidOwned::Invalid(p.to_owned(), r),
        }
    }
}

/// The outcome of consuming a value into a validated form of type `V`.
///
/// Returned by [`IntoValidated::into_validated`]. The `Valid` variant
/// holds the constructed `V`; the `Invalid` variant holds the original
/// precursor (returned unchanged, by move) alongside the diagnostic.
///
/// [`IntoValidated::into_validated`]: crate::IntoValidated::into_validated
///
/// # Why both variants matter
///
/// `MaybeValidOwned` is deliberately not a [`Result`]. The `Invalid`
/// variant is not an error to handle and discard: it returns the
/// caller's input to them, intact, so they can retry, repair, log,
/// or fall through to an alternative. Routing this through `Result`
/// would frame precursor recovery as error-handling boilerplate;
/// `MaybeValidOwned` frames it as a first-class outcome.
///
/// The precursor is returned by move, not by clone. The trait does
/// not require `Self: Clone`, and no allocation or duplication
/// occurs on the invalid path beyond what constructing the
/// diagnostic requires.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "alloc")]
/// # {
/// # use maybe_valid::{IntoValidated, MaybeValidOwned};
/// let bytes = vec![0xff, 0xfe];
/// let validated: MaybeValidOwned<String, Vec<u8>> = bytes.into_validated();
/// match validated {
///     MaybeValidOwned::Valid(s) => println!("got: {}", s),
///     MaybeValidOwned::Invalid(bytes, reason) => {
///         // `bytes` is the original Vec<u8>, moved back to us.
///         assert_eq!(bytes, vec![0xff, 0xfe]);
///         eprintln!("invalid at byte {}", reason.valid_up_to());
///     }
/// }
/// # }
/// ```
///
/// # Construction
///
/// `MaybeValidOwned` values are produced by [`IntoValidated`]
/// implementations. Direct construction is public and unrestricted:
/// `V`'s own invariants are enforced by `V`, not by this enum.
///
/// [`IntoValidated`]: crate::IntoValidated
pub enum MaybeValidOwned<V: Validated, P> {
    /// The precursor satisfied `V`'s predicate.
    ///
    /// Holds the constructed validated value.
    Valid(V),

    /// The precursor did not satisfy `V`'s predicate.
    ///
    /// Holds the original precursor (returned unchanged, by move)
    /// and the diagnostic reason.
    Invalid(P, V::InvalidReason),
}

impl<V: Validated, P> MaybeValidOwned<V, P> {
    /// Returns `true` if this is the `Valid` variant.
    pub fn is_valid(&self) -> bool {
        matches!(self, MaybeValidOwned::Valid(_))
    }

    /// Returns `true` if this is the `Invalid` variant.
    pub fn is_invalid(&self) -> bool {
        matches!(self, MaybeValidOwned::Invalid(_, _))
    }

    /// Returns the validated value, or `None` if invalid.
    ///
    /// Discards the precursor on the invalid path; use
    /// [`invalid_parts`] to retain both the precursor and the reason.
    ///
    /// [`invalid_parts`]: MaybeValidOwned::invalid_parts
    pub fn valid(self) -> Option<V> {
        match self {
            MaybeValidOwned::Valid(v) => Some(v),
            MaybeValidOwned::Invalid(_, _) => None,
        }
    }

    /// Returns the precursor on the invalid path, or `None` if valid.
    ///
    /// Discards the reason; use [`invalid_parts`] to retain both.
    ///
    /// [`invalid_parts`]: MaybeValidOwned::invalid_parts
    pub fn invalid_precursor(self) -> Option<P> {
        match self {
            MaybeValidOwned::Valid(_) => None,
            MaybeValidOwned::Invalid(p, _) => Some(p),
        }
    }

    /// Returns the invalid reason, or `None` if valid.
    ///
    /// Discards the precursor; use [`invalid_parts`] to retain both.
    ///
    /// [`invalid_parts`]: MaybeValidOwned::invalid_parts
    pub fn invalid_reason(self) -> Option<V::InvalidReason> {
        match self {
            MaybeValidOwned::Valid(_) => None,
            MaybeValidOwned::Invalid(_, r) => Some(r),
        }
    }

    /// Returns the precursor and reason on the invalid path, or
    /// `None` if valid.
    pub fn invalid_parts(self) -> Option<(P, V::InvalidReason)> {
        match self {
            MaybeValidOwned::Valid(_) => None,
            MaybeValidOwned::Invalid(p, r) => Some((p, r)),
        }
    }

    /// Returns a `MaybeValidRef` that borrows from this one, with the
    /// same variant structure.
    ///
    /// Useful for inspecting an owned outcome without consuming it.
    /// The returned value borrows `V` as `&V` and the precursor as
    /// `&P`, and clones the `InvalidReason`.
    pub fn as_ref(&self) -> MaybeValidRef<'_, V, P>
    where
        V::InvalidReason: Clone,
    {
        match self {
            MaybeValidOwned::Valid(v) => MaybeValidRef::Valid(v),
            MaybeValidOwned::Invalid(p, r) => MaybeValidRef::Invalid(p, r.clone()),
        }
    }

    /// Converts into a `Result`, discarding the peer framing and
    /// bundling the precursor into the error.
    ///
    /// Useful when integrating with code written against `Result` and
    /// `?`, at the cost of the explicit-match ergonomics
    /// `MaybeValidOwned` encourages.
    pub fn into_result(self) -> Result<V, (P, V::InvalidReason)> {
        match self {
            MaybeValidOwned::Valid(v) => Ok(v),
            MaybeValidOwned::Invalid(p, r) => Err((p, r)),
        }
    }

    /// Converts into a `Result` that carries only the reason on the
    /// error path, discarding the precursor.
    ///
    /// Prefer [`into_result`] when the precursor is still useful to
    /// the caller; this method is a convenience for call sites that
    /// only need the diagnostic.
    ///
    /// [`into_result`]: MaybeValidOwned::into_result
    pub fn into_result_reason_only(self) -> Result<V, V::InvalidReason> {
        match self {
            MaybeValidOwned::Valid(v) => Ok(v),
            MaybeValidOwned::Invalid(_, r) => Err(r),
        }
    }
}

/// Borrows `Self` as a validated view of type `V`, if `self` satisfies
/// `V`'s predicate.
///
/// `AsValidated<V>` expresses that a reference to `Self` can be
/// reinterpreted as a reference to `V` whenever `self`'s contents
/// satisfy the validation predicate declared by `V: Validated`. The
/// conversion is a borrow: the returned `&V` aliases memory owned by
/// `self`, and no allocation occurs on either the valid or invalid
/// path.
///
/// This is the fallible analog of [`AsRef`], restricted to
/// *structural refinements* — cases where `V` shares a representation
/// with `Self` and differs only in which values are permitted. For
/// non-structural fallible conversions (parsing a string into a binary
/// value, for example), use [`TryFrom`] or [`FromStr`].
///
/// [`AsRef`]: core::convert::AsRef
/// [`TryFrom`]: core::convert::TryFrom
/// [`FromStr`]: core::str::FromStr
///
/// # Contract
///
/// Implementers of `AsValidated<V>` promise:
///
/// 1. **Structural conversion.** On the valid path, the returned
///    `&V` must alias memory within `self`. No new storage is
///    allocated, and no owned intermediate value is constructed.
///
/// 2. **Cost bounded by validation.** The method performs at most the
///    work inherent to deciding whether `self` satisfies `V`'s
///    predicate. It does not do additional work that could be deferred
///    or cached elsewhere.
///
/// 3. **Diagnostic-only reason.** The [`InvalidReason`] returned on
///    the invalid path does not carry a copy of `self`'s contents.
///    The precursor is returned as a reference alongside the reason,
///    aliasing the same `&self` the caller passed in.
///
/// 4. **Deterministic classification.** Whether a given `self`
///    produces `Valid` or `Invalid` depends only on `self`'s observable
///    state. Repeated calls with the same state produce the same
///    classification.
///
/// These are logical contracts, not compiler-enforced ones. Violating
/// them does not cause undefined behavior but breaks the guarantees
/// that generic code written against `AsValidated` relies on.
///
/// [`InvalidReason`]: crate::Validated::InvalidReason
///
/// # Examples
///
/// Borrowing `&[u8]` as `&str` when the bytes are valid UTF-8:
///
/// ```
/// # use maybe_valid::{AsValidated, MaybeValidRef};
/// let bytes: &[u8] = b"hello";
/// let validated: MaybeValidRef<'_, str, [u8]> = bytes.as_validated();
/// match validated {
///     MaybeValidRef::Valid(s) => {
///         assert_eq!(s, "hello");
///     }
///     MaybeValidRef::Invalid(bytes, reason) => {
///         eprintln!(
///             "invalid at byte {} of {} total",
///             reason.valid_up_to(),
///             bytes.len(),
///         );
///     }
/// }
/// ```
///
/// The `&str` returned in the valid branch aliases the original byte
/// slice. No allocation occurred, and the bytes remain accessible
/// through both the `bytes` binding and the `Invalid` branch's first
/// component.
///
/// # Relationship to `IntoValidated`
///
/// `AsValidated` and [`IntoValidated`] are peers: the first borrows,
/// the second consumes. A type that can be validated by borrow can
/// typically also be validated by ownership. Both route through the
/// same [`Validated`] target type and share its [`InvalidReason`].
///
/// Paired borrowed/owned validated types — such as [`str`] / `String`
/// or [`CStr`] / `CString` — should share an `InvalidReason` so that
/// `MaybeValidRef::into_owned` (when the `alloc` feature is enabled) can convert between them without
/// requiring a reason-type conversion.
///
/// [`IntoValidated`]: crate::IntoValidated
/// [`Validated`]: crate::Validated
/// [`str`]: prim@str
/// [`CStr`]: core::ffi::CStr
pub trait AsValidated<V: Validated + ?Sized> {
    /// Borrows `self` as a validated `V`, if valid.
    ///
    /// Returns `MaybeValidRef::Valid(&v)` when `self` satisfies `V`'s
    /// predicate, with `v` aliasing memory in `self`. Returns
    /// `MaybeValidRef::Invalid(&self, reason)` otherwise, with the
    /// precursor reference and the diagnostic reason.
    fn as_validated(&self) -> MaybeValidRef<'_, V, Self>;
}

/// Consumes `Self` into a validated value of type `V`, if `self`
/// satisfies `V`'s predicate.
///
/// `IntoValidated<V>` expresses that `Self` can be converted into `V`
/// whenever `self`'s contents satisfy the validation predicate declared
/// by `V: Validated`. On failure, `self` is returned unchanged
/// alongside the diagnostic, so the caller does not lose their input.
///
/// This is the owning counterpart to [`AsValidated`]. Use it when the
/// validated form is an owned value (for example, converting
/// `Vec<u8>` into `String`) rather than a borrowed view.
///
/// [`AsValidated`]: crate::AsValidated
///
/// # Contract
///
/// Implementers of `IntoValidated<V>` promise:
///
/// 1. **Precursor recovery on failure.** When validation fails, the
///    returned [`MaybeValidOwned::Invalid`] variant contains `self`
///    unchanged. The caller can always recover their input.
///
/// 2. **No precursor cloning.** Recovery on the invalid path returns
///    the original `self` by move, not a clone. The trait does not
///    require `Self: Clone`.
///
/// 3. **Cost bounded by validation and construction.** The method
///    performs at most the work inherent to deciding whether `self`
///    satisfies `V`'s predicate and constructing the resulting `V`.
///    Unlike [`AsValidated`], construction may involve allocation or
///    transformation when producing the owned `V` requires it.
///
/// 4. **Diagnostic-only reason.** The [`InvalidReason`] component of
///    the invalid branch does not carry a copy of `self`; the
///    precursor is returned structurally via the tuple.
///
/// 5. **Deterministic classification.** Whether a given `self`
///    produces `Valid` or `Invalid` depends only on `self`'s observable
///    state.
///
/// These are logical contracts, not compiler-enforced ones.
///
/// [`InvalidReason`]: crate::Validated::InvalidReason
/// [`MaybeValidOwned::Invalid`]: crate::MaybeValidOwned::Invalid
///
/// # Examples
///
/// Consuming `Vec<u8>` into `String`, recovering the bytes on failure
/// without a clone:
///
/// ```
/// # #[cfg(feature = "alloc")]
/// # {
/// # use maybe_valid::{IntoValidated, MaybeValidOwned};
/// let bytes = vec![0xff, 0xfe, 0xfd];
/// let validated: MaybeValidOwned<String, Vec<u8>> = bytes.into_validated();
/// match validated {
///     MaybeValidOwned::Valid(s) => {
///         println!("got string: {}", s);
///     }
///     MaybeValidOwned::Invalid(bytes, reason) => {
///         // `bytes` is the original Vec<u8>, moved back to us.
///         // No clone occurred on the failure path.
///         eprintln!(
///             "invalid UTF-8 at byte {}; recovered {} bytes",
///             reason.valid_up_to(),
///             bytes.len(),
///         );
///     }
/// }
/// # }
/// ```
///
/// Converting `u32` into `NonZeroU32`:
///
/// ```
/// # use maybe_valid::{IntoValidated, MaybeValidOwned};
/// # use std::num::NonZeroU32;
/// let candidates = [1u32, 0, 42];
/// for n in candidates {
///     let validated: MaybeValidOwned<NonZeroU32, u32> = n.into_validated();
///     match validated {
///         MaybeValidOwned::Valid(nz) => println!("{} is nonzero", nz),
///         MaybeValidOwned::Invalid(original, _) => {
///             assert_eq!(original, 0);
///             println!("zero, skipping");
///         }
///     }
/// }
/// ```
///
/// # Relationship to `AsValidated`
///
/// `IntoValidated` and [`AsValidated`] are peers: the second borrows,
/// the first consumes. Both route through the same [`Validated`]
/// target type and share its [`InvalidReason`]. Paired borrowed/owned
/// validated types should share an `InvalidReason` so that outcomes
/// can round-trip between the two via
/// `MaybeValidRef::into_owned` (when the `alloc` feature is enabled).
///
/// [`Validated`]: crate::Validated
///
/// # Relationship to `TryFrom`
///
/// `IntoValidated<V>` overlaps in shape with [`TryFrom<Self>`] for
/// `V`, but differs in three ways:
///
/// - **Canonical reason type.** `IntoValidated<V>` routes all failures
///   through `V::InvalidReason`, which is fixed by the target type.
///   `TryFrom` lets each impl choose its own error.
///
/// - **Structural precursor recovery.** `IntoValidated` guarantees via
///   its signature that `self` is returned on failure.
///   `TryFrom::Error` may or may not carry the precursor, depending on
///   the impl; callers cannot rely on recovery in generic code.
///
/// - **Scope.** `IntoValidated` is intended for *structural
///   refinements*, where `V` is a subset of `Self`'s representation
///   with a validation predicate. Use `TryFrom` for conversions that
///   change representation (parsing, decoding, reinterpretation).
///
/// These differences make `IntoValidated` a better fit for generic
/// code that needs to rely on precursor recovery or uniform error
/// handling, and `TryFrom` a better fit for general fallible
/// conversion.
///
/// [`TryFrom<Self>`]: core::convert::TryFrom
///
/// # Relationship to `FromStr`
///
/// [`FromStr`] parses a string into a value, which is typically a
/// representation-changing operation. `IntoValidated` is for
/// structural refinements and does not apply to parsing. Types like
/// [`IpAddr`] or [`Duration`], which are constructed from strings but
/// have internal binary representations unrelated to the string's
/// bytes, use `FromStr`, not `IntoValidated`.
///
/// [`FromStr`]: core::str::FromStr
/// [`IpAddr`]: core::net::IpAddr
/// [`Duration`]: core::time::Duration
pub trait IntoValidated<V: Validated>: Sized {
    /// Consumes `self` into a validated `V`, if valid.
    ///
    /// Returns `MaybeValidOwned::Valid(v)` when `self` satisfies `V`'s
    /// predicate. Returns `MaybeValidOwned::Invalid(self, reason)`
    /// otherwise, with `self` returned unchanged.
    fn into_validated(self) -> MaybeValidOwned<V, Self>;
}
