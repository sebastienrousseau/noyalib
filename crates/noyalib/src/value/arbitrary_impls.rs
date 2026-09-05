// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! [`arbitrary::Arbitrary`] for the public value types, behind the
//! `arbitrary` Cargo feature.
//!
//! Structure-aware fuzzing needs semantically valid trees rather than
//! bytes. With this feature a fuzz target, a property test, or a
//! satellite's own harness writes `fuzz_target!(|v: Value| ...)` and
//! gets every variant, every number kind, tags, and nested collections
//! from one generator, instead of each harness rebuilding its own.
//!
//! Depth is bounded by the remaining entropy: once fewer than a few
//! bytes are left, only scalars are produced, so a tree terminates
//! without a recursion counter. Floats are generated as-is, including
//! NaN; a harness that asserts equality filters NaN itself, since the
//! library preserves it and `NaN != NaN` is the IEEE contract, not a
//! defect.
//! Tagged payloads are strings or collections only, because a custom
//! tag on a scalar suppresses resolution and the scalar reads back as a
//! string, which is the YAML data model rather than a defect.

use crate::prelude::*;
use arbitrary::{Arbitrary, Result, Unstructured};

use super::{Mapping, Number, Tag, TaggedValue, Value};

/// Below this many remaining bytes the generator emits scalars only,
/// which bounds recursion without a depth parameter.
const LEAF_THRESHOLD: usize = 4;

impl<'a> Arbitrary<'a> for Number {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        #[cfg(feature = "lossless-u64")]
        let kinds = 3u8;
        #[cfg(not(feature = "lossless-u64"))]
        let kinds = 2u8;
        Ok(match u.int_in_range(0..=kinds - 1)? {
            0 => Self::Integer(i64::arbitrary(u)?),
            1 => Self::Float(f64::arbitrary(u)?),
            // `Unsigned` is canonical only above `i64::MAX`; a smaller value
            // reads back as `Integer`, so the top bit is forced.
            #[cfg(feature = "lossless-u64")]
            _ => Self::Unsigned(u64::arbitrary(u)? | (1 << 63)),
            #[cfg(not(feature = "lossless-u64"))]
            _ => Self::Integer(i64::arbitrary(u)?),
        })
    }
}

impl<'a> Arbitrary<'a> for Tag {
    /// A local tag (`!name`) or a global one (`!!name`); the name is
    /// restricted to characters every YAML tag handle accepts so the
    /// emitted document always re-parses.
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let handle = if bool::arbitrary(u)? { "!" } else { "!!" };
        let len = u.int_in_range(1..=12usize)?;
        let mut name = String::with_capacity(len + 2);
        name.push_str(handle);
        for _ in 0..len {
            let c = u.choose(&['a', 'b', 'c', 'x', 'y', 'z', 'T', 'N', '_', '-', '0', '9'])?;
            name.push(*c);
        }
        Ok(Self::new(name))
    }
}

impl<'a> Arbitrary<'a> for TaggedValue {
    /// The payload is a string or a collection, never another scalar
    /// kind: an explicit non-core tag suppresses scalar resolution, so
    /// `!!custom null` reads back as the string `null`. A tagged `Null`,
    /// `Bool`, or `Number` has no textual spelling and would only ever
    /// fail a round trip for a reason that is not a defect.
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let payload = match u.int_in_range(0..=2u8)? {
            0 => Value::String(String::arbitrary(u)?),
            1 => Value::Sequence(Vec::<Value>::arbitrary(u)?),
            _ => Value::Mapping(Mapping::arbitrary(u)?),
        };
        Ok(Self::new(Tag::arbitrary(u)?, payload))
    }
}

impl<'a> Arbitrary<'a> for Mapping {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let mut m = Self::new();
        for _ in 0..u.arbitrary_len::<(String, Value)>()? {
            let key = String::arbitrary(u)?;
            let value = Value::arbitrary(u)?;
            let _replaced = m.insert(key, value);
        }
        Ok(m)
    }
}

impl<'a> Arbitrary<'a> for Value {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let variants = if u.len() < LEAF_THRESHOLD { 4u8 } else { 7u8 };
        Ok(match u.int_in_range(0..=variants - 1)? {
            0 => Self::Null,
            1 => Self::Bool(bool::arbitrary(u)?),
            2 => Self::Number(Number::arbitrary(u)?),
            3 => Self::String(String::arbitrary(u)?),
            4 => Self::Sequence(Vec::<Self>::arbitrary(u)?),
            5 => Self::Mapping(Mapping::arbitrary(u)?),
            _ => Self::Tagged(Box::new(TaggedValue::arbitrary(u)?)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every generated value survives an emit-then-parse round trip
    /// unless it carries NaN, and generation terminates on any input.
    #[test]
    fn generated_values_round_trip() {
        let mut seed: Vec<u8> = (0..=255u8).cycle().take(4096).collect();
        for salt in 0..64u8 {
            let by = usize::from(salt) * 7 % seed.len().max(1);
            seed.rotate_left(by);
            let mut u = Unstructured::new(&seed);
            let v = Value::arbitrary(&mut u).expect("generation never fails on 4 KiB");
            let text = crate::to_string(&v).expect("every generated Value serialises");
            // `Unsigned` above `i64::MAX` is emitted exactly and reads back as
            // an unsigned only when the parser is told to keep u64 precision;
            // the default widens it to f64 by design.
            #[cfg(feature = "lossless-u64")]
            let cfg = crate::ParserConfig::new().lossless_u64_integers(true);
            #[cfg(not(feature = "lossless-u64"))]
            let cfg = crate::ParserConfig::new();
            let back: Value =
                crate::from_str_with_config(&text, &cfg).expect("emitted YAML re-parses");
            if !text.to_ascii_lowercase().contains("nan") {
                assert_eq!(back, v, "round-trip drift on:\n{text}");
            }
        }
    }

    #[test]
    fn empty_entropy_yields_a_scalar() {
        let mut u = Unstructured::new(&[]);
        let v = Value::arbitrary(&mut u).expect("no entropy is still a value");
        assert!(
            !matches!(v, Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_)),
            "{v:?}"
        );
    }
}
