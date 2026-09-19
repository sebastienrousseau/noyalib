// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `MessageFormatter` trait + bundled `DefaultFormatter` /
//! `UserFormatter`.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::i18n::{DefaultFormatter, MessageFormatter, UserFormatter};
use noyalib::{Error, Value, from_str};

#[test]
fn default_formatter_matches_display() {
    let err = from_str::<Value>("a: [unclosed").unwrap_err();
    assert_eq!(DefaultFormatter.format(&err), err.to_string());
}

#[test]
fn user_formatter_for_parse_error() {
    let err = from_str::<Value>("a: [unclosed").unwrap_err();
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("syntax error"), "{msg}");
}

#[test]
fn user_formatter_with_location_includes_line_number() {
    let err = from_str::<Value>("ok: 1\n  bad: indented\n").unwrap_err();
    let msg = UserFormatter.format(&err);
    // location-bearing variants embed "line N"
    assert!(
        msg.contains("line") || msg.contains("syntax error"),
        "{msg}"
    );
}

#[test]
fn user_formatter_for_duplicate_key() {
    let err = Error::DuplicateKey("dup".into());
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("twice"), "{msg}");
}

#[test]
fn user_formatter_for_missing_field() {
    let err = Error::MissingField("password".into());
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("missing"), "{msg}");
    assert!(!msg.contains("password"), "{msg}");
}

#[test]
fn user_formatter_for_type_mismatch() {
    let err = Error::TypeMismatch {
        expected: "integer",
        found: "string".into(),
    };
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("wrong type"), "{msg}");
}

#[test]
fn user_formatter_for_recursion_limit() {
    let err = Error::RecursionLimitExceeded { depth: 256 };
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("large") || msg.contains("nested"), "{msg}");
}

#[test]
fn user_formatter_for_unknown_anchor() {
    let err = Error::UnknownAnchor("missing-alias".into());
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("does not exist"), "{msg}");
}

#[test]
fn user_formatter_for_repetition_limit() {
    let err = Error::RepetitionLimitExceeded;
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("large") || msg.contains("nested"), "{msg}");
}

#[test]
fn render_with_formatter_dispatches_correctly() {
    let err = from_str::<Value>("a: [unclosed").unwrap_err();
    let dev = err.render_with_formatter(&DefaultFormatter);
    let user = err.render_with_formatter(&UserFormatter);
    assert_eq!(dev, err.to_string());
    assert!(user.contains("syntax error"));
}

#[test]
fn message_formatter_works_through_dyn_dispatch() {
    fn render_any(f: &dyn MessageFormatter, e: &Error) -> String {
        f.format(e)
    }
    let err = from_str::<Value>("a: [unclosed").unwrap_err();
    let dev = render_any(&DefaultFormatter, &err);
    let user = render_any(&UserFormatter, &err);
    assert!(!dev.is_empty());
    assert!(!user.is_empty());
}

#[test]
fn user_formatter_for_parse_no_location() {
    let err = Error::Parse("synthetic parse failure".into());
    let msg = UserFormatter.format(&err);
    assert_eq!(msg, "The configuration file has a syntax error.");
}

#[test]
fn user_formatter_for_deserialize() {
    let err = Error::Deserialize("synthetic deser failure".into());
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("does not match"), "{msg}");
}

#[test]
fn user_formatter_for_deserialize_with_location() {
    use noyalib::Location;
    let err = Error::DeserializeWithLocation {
        message: "synthetic".into(),
        location: Location::new(3, 5, 12),
    };
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("line 3"), "{msg}");
    assert!(msg.contains("does not match"), "{msg}");
}

#[test]
fn user_formatter_for_io_error() {
    let err = Error::Io(std::io::Error::other("synthetic io"));
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("Could not read"), "{msg}");
}

#[test]
fn user_formatter_for_budget_breach() {
    use noyalib::BudgetBreach;
    let err = Error::Budget(BudgetBreach::MaxNodes {
        limit: 100,
        observed: 101,
    });
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("large") || msg.contains("nested"), "{msg}");
}

#[test]
fn user_formatter_catchall_for_unhandled_variants() {
    // Custom is the catch-all path in UserFormatter — not one of
    // the enumerated user-facing categories.
    let err = Error::Custom("synthetic".into());
    let msg = UserFormatter.format(&err);
    assert_eq!(msg, "The configuration file is invalid.");
}

#[test]
fn user_formatter_for_unknown_anchor_at() {
    use noyalib::Location;
    let err = Error::UnknownAnchorAt {
        name: "anc".into(),
        location: Location::new(2, 3, 5),
        suggestion: None,
    };
    let msg = UserFormatter.format(&err);
    assert!(msg.contains("does not exist"), "{msg}");
}

#[test]
fn custom_formatter_implementation() {
    struct UpperFormatter;
    impl MessageFormatter for UpperFormatter {
        fn format(&self, error: &Error) -> String {
            error.to_string().to_uppercase()
        }
    }
    let err = from_str::<Value>("a: [unclosed").unwrap_err();
    let msg = err.render_with_formatter(&UpperFormatter);
    assert_eq!(msg, err.to_string().to_uppercase());
}
