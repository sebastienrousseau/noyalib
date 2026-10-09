// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Plain hex and octal integers resolve only in the spelling the YAML
//! 1.2 core schema defines: `0x[0-9a-fA-F]+` and `0o[0-7]+`. Found by
//! fuzz_diff (input `0X0`): an uppercase prefix, or a sign after the
//! prefix, used to resolve to an integer.

use noyalib::{Value, from_str};

const STRINGS: &[&str] = &["0X0", "0X10", "0O17", "0x-1", "0x+1", "0o-7", "0o+7"];
const INTS: &[(&str, i64)] = &[("0x10", 16), ("0x1F", 31), ("0xff", 255), ("0o17", 15)];

#[test]
fn off_schema_radix_spellings_are_strings() {
    for s in STRINGS {
        let v: Value = from_str(s).unwrap();
        assert_eq!(v.as_str(), Some(*s), "owned loader resolved {s:?}");
        let b = noyalib::borrowed::from_str_borrowed(s).unwrap();
        assert_eq!(b.as_str(), Some(*s), "borrowed loader resolved {s:?}");
        let docs: Vec<Value> = noyalib::load_all_as(s).unwrap();
        assert_eq!(docs[0].as_str(), Some(*s), "load_all resolved {s:?}");
        assert!(from_str::<i64>(s).is_err(), "typed i64 accepted {s:?}");
    }
}

#[test]
fn core_schema_radix_spellings_are_integers() {
    for &(s, n) in INTS {
        let v: Value = from_str(s).unwrap();
        assert_eq!(v.as_i64(), Some(n), "{s:?}");
        let b = noyalib::borrowed::from_str_borrowed(s).unwrap();
        assert_eq!(b.as_i64(), Some(n), "borrowed {s:?}");
        assert_eq!(from_str::<i64>(s).unwrap(), n, "typed {s:?}");
    }
}
