// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Auto-formatting for values spliced by the CST insertion mutators.
//!
//! The fragment-taking mutators — [`Document::set`],
//! [`Document::insert_entry`], [`Document::push_back`],
//! [`Document::insert_after`] — splice their `&str` argument
//! **verbatim**. They synthesise indentation, but not quoting: a
//! fragment holding `a: b`, a leading `-`, a `#`, or a line break is
//! pasted as YAML *syntax*, not as data. The re-parse guard behind
//! those mutators rejects a splice that produces **invalid** YAML; it
//! cannot reject one that produces **valid but different** YAML, so
//! `push_back("items", "- x")` silently grows a nested sequence and
//! `insert_entry(m, "k", "a: b")` silently grows a nested mapping.
//!
//! This module closes that hole. [`Emit`] turns a typed value into the
//! YAML spelling that re-parses to exactly that value at a given site,
//! and pairs it with [`Emit::expected_value`] — the typed value the
//! spliced fragment must load back as. The insertion mutators that
//! take an `impl Emit` use the second as an oracle for the first: after
//! the splice the document must re-parse *and* its typed view must
//! equal the pre-edit value with exactly that one insertion applied,
//! or the edit is rolled back.
//!
//! # Style matching
//!
//! Unlike [`Document::set_value`], which matches the scalar style of
//! the leaf it replaces, an *insertion* has no existing leaf to match.
//! [`EmitCtx`] therefore carries the document's own conventions —
//! detected indent unit, dominant quote style, dominant collection
//! style — so a new entry looks like the file it lands in. Quoting is
//! forced regardless of those conventions whenever the plain spelling
//! would re-parse to something other than the intended value.
//!
//! # Examples
//!
//! ```
//! use noyalib::cst::parse_document;
//!
//! let mut doc = parse_document("items:\n  - one\n").unwrap();
//! // The verbatim path would splice a nested sequence here.
//! doc.push_back_value("items", "- two").unwrap();
//! assert_eq!(doc.to_string(), "items:\n  - one\n  - \"- two\"\n");
//! ```
//!
//! [`Document::set`]: crate::cst::Document::set
//! [`Document::set_value`]: crate::cst::Document::set_value
//! [`Document::insert_entry`]: crate::cst::Document::insert_entry
//! [`Document::push_back`]: crate::cst::Document::push_back
//! [`Document::insert_after`]: crate::cst::Document::insert_after

use crate::cst::document::{
    can_use_block_literal, format_block_literal, format_double_quoted, format_number,
    format_single_quoted, is_plain_safe,
};
use crate::error::{Error, Result};
use crate::prelude::*;
use crate::value::Value;
use crate::{FlowStyle, ScalarStyle};

/// The site an emitted fragment will be spliced into.
///
/// Built by the insertion mutators from the document's own detected
/// conventions ([`Document::indent_unit`],
/// [`Document::dominant_quote_style`],
/// [`Document::dominant_flow_style`]) plus the column the new entry
/// starts at, so an [`Emit`] implementation can match the file it is
/// landing in rather than a serializer default.
///
/// [`Document::indent_unit`]: crate::cst::Document::indent_unit
/// [`Document::dominant_quote_style`]: crate::cst::Document::dominant_quote_style
/// [`Document::dominant_flow_style`]: crate::cst::Document::dominant_flow_style
///
/// # Examples
///
/// ```
/// use noyalib::cst::{Emit, EmitCtx};
/// use noyalib::{FlowStyle, ScalarStyle};
///
/// let ctx = EmitCtx::new(ScalarStyle::Plain, FlowStyle::Block, 2, 0);
/// // Plain is safe here, and the site prefers plain.
/// assert_eq!("noyalib".emit(&ctx).unwrap(), "noyalib");
/// // Plain would re-parse as a boolean — quoting is forced.
/// assert_eq!("true".emit(&ctx).unwrap(), "\"true\"");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitCtx {
    quote: ScalarStyle,
    flow: FlowStyle,
    indent_unit: usize,
    column: usize,
}

impl EmitCtx {
    /// Build a context from a site's conventions.
    ///
    /// `column` is the zero-based column the emitted entry starts at —
    /// the mapping key's column, or the `-` indicator's column for a
    /// sequence item.
    ///
    /// It is **advisory**: an emitted fragment indents its own
    /// continuation lines *relative to its first line*, and the mutator
    /// shifts the whole fragment to the site's content column
    /// afterwards. `column` is there for implementations that need to
    /// know where they land anyway — line-width budgets, for
    /// instance — not as the indent to emit at.
    #[must_use]
    pub fn new(quote: ScalarStyle, flow: FlowStyle, indent_unit: usize, column: usize) -> Self {
        Self {
            quote,
            flow,
            indent_unit,
            column,
        }
    }

    /// The dominant scalar quote style at this site.
    #[must_use]
    pub fn quote_style(&self) -> ScalarStyle {
        self.quote
    }

    /// The dominant collection style at this site.
    #[must_use]
    pub fn flow_style(&self) -> FlowStyle {
        self.flow
    }

    /// The document's detected indent step, in spaces.
    #[must_use]
    pub fn indent_unit(&self) -> usize {
        self.indent_unit
    }

    /// The zero-based column the emitted entry starts at.
    #[must_use]
    pub fn column(&self) -> usize {
        self.column
    }
}

/// A value that can be spliced into a document by the auto-formatting
/// mutators.
///
/// Implementors provide two halves of one contract:
///
/// - [`emit`](Self::emit) — the YAML spelling to splice, quoted and
///   escaped so that it re-parses to this value at `ctx`'s site.
/// - [`expected_value`](Self::expected_value) — the typed value that
///   spelling must load back as.
///
/// The mutators check the second against the document after splicing
/// the first, so an implementation whose two halves disagree causes a
/// **refusal with the document left unchanged**, never a silent
/// corruption.
///
/// Implementations ship for `str`, `String`, `bool`, the integer and
/// float primitives, [`Value`], and any reference to one of those. A
/// `Serialize` type is emitted by converting it first:
/// `doc.push_back_value(path, &noyalib::to_value(&my_struct)?)`.
///
/// # Examples
///
/// ```
/// use noyalib::cst::{Emit, EmitCtx};
/// use noyalib::{FlowStyle, ScalarStyle, Value};
///
/// let ctx = EmitCtx::new(ScalarStyle::Plain, FlowStyle::Block, 2, 0);
/// assert_eq!(42_i64.emit(&ctx).unwrap(), "42");
/// assert_eq!(42_i64.expected_value().unwrap(), Value::from(42_i64));
/// ```
pub trait Emit {
    /// The YAML spelling of this value at `ctx`'s site.
    ///
    /// # Errors
    ///
    /// Returns `Error::Parse` when the value has no representation the
    /// emitter can produce for that site (for example a tagged scalar,
    /// whose tag the scalar emitter would drop), and propagates
    /// serializer errors for collections.
    fn emit(&self, ctx: &EmitCtx) -> Result<String>;

    /// The typed value [`emit`](Self::emit)'s output must re-parse to.
    ///
    /// # Errors
    ///
    /// Returns `Error::Parse` for the same cases as
    /// [`emit`](Self::emit) — a value that cannot be emitted has no
    /// meaningful oracle.
    fn expected_value(&self) -> Result<Value>;
}

impl Emit for str {
    fn emit(&self, ctx: &EmitCtx) -> Result<String> {
        Ok(emit_string(self, ctx))
    }

    fn expected_value(&self) -> Result<Value> {
        Ok(Value::String(self.to_owned()))
    }
}

impl Emit for String {
    fn emit(&self, ctx: &EmitCtx) -> Result<String> {
        Ok(emit_string(self, ctx))
    }

    fn expected_value(&self) -> Result<Value> {
        Ok(Value::String(self.clone()))
    }
}

impl Emit for bool {
    fn emit(&self, _ctx: &EmitCtx) -> Result<String> {
        Ok(if *self {
            "true".to_owned()
        } else {
            "false".to_owned()
        })
    }

    fn expected_value(&self) -> Result<Value> {
        Ok(Value::Bool(*self))
    }
}

/// Emit the integer / float primitives through `Value`'s own
/// conversions, so the spelling and the oracle come from one place.
macro_rules! impl_emit_via_value {
    ($($t:ty),* $(,)?) => {
        $(
            impl Emit for $t {
                fn emit(&self, ctx: &EmitCtx) -> Result<String> {
                    Value::from(*self).emit(ctx)
                }

                fn expected_value(&self) -> Result<Value> {
                    Ok(Value::from(*self))
                }
            }
        )*
    };
}

impl_emit_via_value!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

impl Emit for Value {
    fn emit(&self, ctx: &EmitCtx) -> Result<String> {
        match self {
            Self::Null => Ok("null".to_owned()),
            Self::Bool(b) => b.emit(ctx),
            Self::Number(n) => Ok(emit_number(n)),
            Self::String(s) => Ok(emit_string(s, ctx)),
            Self::Sequence(_) | Self::Mapping(_) => emit_collection(self, ctx),
            Self::Tagged(_) => Err(Error::Parse(
                "emit: tagged values are not auto-formatted yet — the scalar emitter would \
                 drop the tag; splice the `!tag value` spelling with `set` / `insert_entry` \
                 instead"
                    .into(),
            )),
        }
    }

    fn expected_value(&self) -> Result<Value> {
        if matches!(self, Self::Tagged(_)) {
            return Err(Error::Parse(
                "emit: tagged values are not auto-formatted yet — the scalar emitter would \
                 drop the tag; splice the `!tag value` spelling with `set` / `insert_entry` \
                 instead"
                    .into(),
            ));
        }
        Ok(self.clone())
    }
}

impl<T: Emit + ?Sized> Emit for &T {
    fn emit(&self, ctx: &EmitCtx) -> Result<String> {
        (**self).emit(ctx)
    }

    fn expected_value(&self) -> Result<Value> {
        (**self).expected_value()
    }
}

/// YAML spelling for a numeric scalar.
///
/// Floats must not use `Display` (what [`format_number`] reaches for):
/// it renders `1.0_f64` as `"1"`, which re-parses as an **integer** and
/// fails the insertion oracle, and the special floats need `.inf` /
/// `-.inf` / `.nan`. Routing the number through the serializer reuses
/// its tested, canonical formatting (including the `fast-float` path),
/// so `1.0`, `-0.0`, `±inf`, and `NaN` each re-parse back to the same
/// `Number::Float`. Integers are unaffected. The fallback to
/// [`format_number`] is unreachable for a bare numeric scalar (the
/// serializer cannot error on one) but keeps the function total.
fn emit_number(n: &crate::value::Number) -> String {
    // A plain `match` (not `.unwrap_or_else`) so the defensive fallback
    // is an inline branch, not a nested closure the coverage tools would
    // count as its own perpetually-uncovered function. The serializer
    // cannot fail on a bare numeric scalar, so `Err` is never taken.
    match crate::to_string_value_with_config(&Value::Number(*n), &crate::SerializerConfig::new()) {
        Ok(s) => s.trim_end_matches('\n').to_owned(),
        Err(_) => format_number(n),
    }
}

/// YAML spelling for a **string** at an insertion site.
///
/// Multi-line strings prefer a literal block scalar, matching
/// `set_value`'s behaviour at block sites. Otherwise the site's
/// dominant quote style is honoured — except that a string whose plain
/// spelling would re-parse to something else (`true`, `8080`, `- x`,
/// `a: b`, a `#` comment lead-in) is always quoted, whatever the file's
/// convention.
fn emit_string(s: &str, ctx: &EmitCtx) -> String {
    if s.contains('\n') && can_use_block_literal(s) {
        // Relative indentation: the body sits two columns in from the
        // fragment's own first line, and the mutator shifts the whole
        // block to the site's content column (see [`EmitCtx::column`]).
        return format_block_literal(s, 0);
    }
    quote_for_site(s, ctx.quote)
}

/// Pick a one-line YAML spelling for `s` under a site's dominant
/// scalar style.
///
/// The dominant style is a *preference*, not a licence: it decides
/// between the spellings that faithfully represent `s`, and the plain
/// spelling is only among them when [`is_plain_safe`] says so. Single
/// quotes have no escapes, so they cannot carry control characters,
/// and a line break inside them folds — a string holding either is
/// double-quoted whatever the file prefers.
fn quote_for_site(s: &str, style: ScalarStyle) -> String {
    let single_representable = !s.bytes().any(|b| b < 0x20 || b == 0x7F);
    match style {
        ScalarStyle::SingleQuoted if single_representable => format_single_quoted(s),
        ScalarStyle::DoubleQuoted | ScalarStyle::SingleQuoted => format_double_quoted(s),
        _ if is_plain_safe(s) => s.to_owned(),
        _ => format_double_quoted(s),
    }
}

/// YAML spelling for a sequence / mapping at an insertion site.
///
/// Delegates to the serializer with the file's detected indent unit and
/// collection style, then strips the one line break that terminates the
/// emission's last line — the splice templates supply their own. Only
/// that one: a keep-chomped block scalar (`|+`) ends in the empty lines
/// that *are* its value, and stripping every trailing break lost them,
/// so the spliced entry failed the integrity check (#386). The result
/// may be multi-line; callers re-indent continuation lines to the
/// site's column before splicing.
///
/// Nested scalars take the serializer's conservative `Auto` quoting,
/// which always round-trips (the oracle would reject anything that did
/// not) — so a plain-styled file may receive `cpu: "100m"` where it
/// writes `cpu: 100m` elsewhere. Matching the file's dominant scalar
/// style for *nested* scalars would need `SerializerConfig::scalar_style`
/// to be honoured by the serializer, which it is not yet.
fn emit_collection(value: &Value, ctx: &EmitCtx) -> Result<String> {
    let cfg = crate::SerializerConfig::new()
        .indent(ctx.indent_unit)
        .flow_style(ctx.flow);
    let emitted = crate::to_string_value_with_config(value, &cfg)?;
    Ok(emitted.strip_suffix('\n').unwrap_or(&emitted).to_owned())
}

/// YAML spelling for a **mapping key** being inserted.
///
/// Keys are quoted only when they must be. The site's dominant style
/// is a statement about *values* — a file that single-quotes its
/// strings is not asking for `'name':` on every key — so it chooses
/// only which quote to reach for once `key` has forced the issue by
/// not being plain-safe. Unlike a value, a key has no block-scalar
/// fallback; a key that cannot be spelled on one line is refused by
/// the caller.
pub(super) fn emit_key(key: &str, ctx: &EmitCtx) -> String {
    if is_plain_safe(key) {
        return key.to_owned();
    }
    let single_representable = !key.bytes().any(|b| b < 0x20 || b == 0x7F);
    if ctx.quote == ScalarStyle::SingleQuoted && single_representable {
        format_single_quoted(key)
    } else {
        format_double_quoted(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_ctx() -> EmitCtx {
        EmitCtx::new(ScalarStyle::Plain, FlowStyle::Block, 2, 0)
    }

    #[test]
    fn plain_safe_string_stays_plain() {
        assert_eq!(emit_string("noyalib", &plain_ctx()), "noyalib");
    }

    #[test]
    fn type_changing_spellings_are_quoted() {
        let ctx = plain_ctx();
        for s in ["true", "null", "8080", "- x", "a: b", "#lead", "~"] {
            let out = emit_string(s, &ctx);
            assert!(
                out.starts_with('"'),
                "{s:?} must be quoted, emitted {out:?}"
            );
        }
    }

    #[test]
    fn dominant_single_quote_is_honoured() {
        let ctx = EmitCtx::new(ScalarStyle::SingleQuoted, FlowStyle::Block, 2, 0);
        assert_eq!(emit_string("plain", &ctx), "'plain'");
    }

    #[test]
    fn control_characters_defeat_single_quoting() {
        let ctx = EmitCtx::new(ScalarStyle::SingleQuoted, FlowStyle::Block, 2, 0);
        assert_eq!(emit_string("a\tb", &ctx), "\"a\\tb\"");
    }

    #[test]
    fn multiline_string_becomes_a_block_literal() {
        let out = emit_string("one\ntwo\n", &plain_ctx());
        assert!(
            out.starts_with("|\n"),
            "expected a block literal, got {out:?}"
        );
    }

    #[test]
    fn key_spelling_matches_value_rules() {
        let ctx = plain_ctx();
        assert_eq!(emit_key("name", &ctx), "name");
        assert_eq!(emit_key("a: b", &ctx), "\"a: b\"");
        assert_eq!(emit_key("true", &ctx), "\"true\"");
    }

    #[test]
    fn emit_and_oracle_agree_for_primitives() {
        let ctx = plain_ctx();
        assert_eq!(true.emit(&ctx).unwrap(), "true");
        assert_eq!(true.expected_value().unwrap(), Value::Bool(true));
        assert_eq!(7_i64.emit(&ctx).unwrap(), "7");
        assert_eq!(7_i64.expected_value().unwrap(), Value::from(7_i64));
    }

    #[test]
    fn tagged_values_are_refused_on_both_halves() {
        let ctx = plain_ctx();
        let tagged = crate::from_str::<Value>("!custom 1").unwrap();
        assert!(tagged.emit(&ctx).is_err());
        assert!(tagged.expected_value().is_err());
    }
}
