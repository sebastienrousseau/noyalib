//! Fuzz target: parse YAML, then deserialize the Value into typed structs.
//!
//! Exercises the `from_value` deserializer and type coercion logic.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#![no_main]

use libfuzzer_sys::fuzz_target;
use noyalib::Value;
use serde_core::Deserialize as _;
use std::collections::HashMap;

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct Config {
    name: Option<String>,
    port: Option<u16>,
    tags: Option<Vec<String>>,
    meta: Option<HashMap<String, Value>>,
}

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };
    let Ok(value) = noyalib::from_str::<Value>(s) else { return };

    // Try various typed deserializations — none should panic.
    let _ = noyalib::from_value::<Config>(&value);
    let _ = noyalib::from_value::<Vec<Value>>(&value);
    let _ = noyalib::from_value::<HashMap<String, Value>>(&value);
    let _ = noyalib::from_value::<String>(&value);
    let _ = noyalib::from_value::<i64>(&value);
    let _ = noyalib::from_value::<bool>(&value);
});
