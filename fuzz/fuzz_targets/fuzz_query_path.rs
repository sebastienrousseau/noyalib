// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::QueryPath;

fuzz_target!(|input: &str| {
    if let Ok(path) = input.parse::<QueryPath>() {
        let canonical = path.to_string();
        let reparsed = canonical
            .parse::<QueryPath>()
            .expect("a canonical query path must parse");
        assert_eq!(reparsed, path);
    }
});
