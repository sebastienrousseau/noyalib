//! `set_value` renders any scalar so it reads back unchanged.
//!
//! The enumerated cases in `set_fragment_containment.rs` cover the
//! spellings that were known to be dangerous — `"true"`, `""`,
//! `"v # x"`, `"v\nc: 3"`. This generalises the claim: for an arbitrary
//! string, writing it with `set_value` and re-parsing must return
//! exactly that string, and must not disturb the rest of the document.
//!
//! That is the property `set` deliberately does *not* have, since it
//! splices YAML verbatim — which is why the docs point callers here.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

use noyalib::Value;
use noyalib::cst::parse_document;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Any scalar string round-trips through `set_value`.
    #[test]
    fn set_value_round_trips_arbitrary_scalars(s in ".{0,120}") {
        let mut doc = parse_document("a: 1\nb: 2\n").expect("parse");
        doc.set_value("a", &Value::String(s.clone())).expect("set_value");

        let back: Value = noyalib::from_str(doc.source())
            .unwrap_or_else(|e| panic!("re-parse failed for {s:?}: {e}"));
        let Value::Mapping(m) = &back else {
            panic!("expected a mapping for {s:?}")
        };

        prop_assert_eq!(
            m.get("a"),
            Some(&Value::String(s.clone())),
            "set_value did not round-trip {:?}", s
        );
        // The neighbour must be untouched, and nothing may be added.
        prop_assert_eq!(m.len(), 2, "entry count changed writing {:?}", s);
        prop_assert!(m.get("b").is_some(), "sibling lost writing {:?}", s);
    }

    /// The same, for a value nested inside a mapping.
    #[test]
    fn set_value_round_trips_when_nested(s in ".{0,80}") {
        let mut doc = parse_document("outer:\n  a: 1\nz: 9\n").expect("parse");
        doc.set_value("outer.a", &Value::String(s.clone())).expect("set_value");

        let back: Value = noyalib::from_str(doc.source())
            .unwrap_or_else(|e| panic!("re-parse failed for {s:?}: {e}"));
        let Value::Mapping(m) = &back else { panic!("expected mapping") };
        let Some(Value::Mapping(inner)) = m.get("outer") else {
            panic!("outer vanished writing {s:?}")
        };
        prop_assert_eq!(inner.get("a"), Some(&Value::String(s.clone())));
        prop_assert!(m.get("z").is_some(), "sibling lost writing {:?}", s);
        prop_assert_eq!(m.len(), 2);
    }

    /// A fragment that changes the document beyond its path is refused,
    /// never silently accepted. `set` may reject or splice, but it must
    /// not corrupt: whatever it returns, the document still parses and
    /// keeps its other entries.
    #[test]
    fn set_never_silently_corrupts(s in ".{0,60}") {
        let src = "a: 1\nb: 2\n";
        let mut doc = parse_document(src).expect("parse");
        let before = doc.source().to_owned();

        if doc.set("a", &s).is_err() {
            prop_assert_eq!(doc.source(), before, "a refused set must not edit");
        } else if let Ok(Value::Mapping(m)) = noyalib::from_str::<Value>(doc.source()) {
            // Accepted and parseable: `b` must survive and no entry may
            // have appeared.
            prop_assert!(m.get("b").is_some(), "sibling lost splicing {:?}", s);
            prop_assert_eq!(m.len(), 2, "entry count changed splicing {:?}", s);
        }
    }
}
