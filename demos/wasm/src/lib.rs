// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! noyalib WASM bindings.
//!
//! Exposes YAML parse/serialize to JavaScript via wasm-bindgen.
//!
//! Build: `cd demos/wasm && wasm-pack build --target web`

use wasm_bindgen::prelude::*;

/// Parse a YAML string and return a JS object.
#[wasm_bindgen]
pub fn parse(yaml: &str) -> Result<JsValue, JsError> {
    let value: noyalib::Value =
        noyalib::from_str(yaml).map_err(|e| JsError::new(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&value).map_err(|e| JsError::new(&e.to_string()))
}

/// Serialize a JS object to a YAML string.
#[wasm_bindgen]
pub fn stringify(value: JsValue) -> Result<String, JsError> {
    let v: noyalib::Value =
        serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?;
    noyalib::to_string(&v).map_err(|e| JsError::new(&e.to_string()))
}

/// Validate YAML against the JSON schema.
#[wasm_bindgen]
pub fn validate_json(yaml: &str) -> Result<bool, JsError> {
    let value: noyalib::Value =
        noyalib::from_str(yaml).map_err(|e| JsError::new(&e.to_string()))?;
    match noyalib::validate_yaml_json_schema(&value) {
        Ok(()) => Ok(true),
        Err(e) => Err(JsError::new(&e.to_string())),
    }
}

/// Get a value at a dotted path from a YAML string.
#[wasm_bindgen]
pub fn get_path(yaml: &str, path: &str) -> Result<JsValue, JsError> {
    let value: noyalib::Value =
        noyalib::from_str(yaml).map_err(|e| JsError::new(&e.to_string()))?;
    match value.get_path(path) {
        Some(v) => serde_wasm_bindgen::to_value(v).map_err(|e| JsError::new(&e.to_string())),
        None => Ok(JsValue::NULL),
    }
}

/// Merge two YAML documents.
#[wasm_bindgen]
pub fn merge(base_yaml: &str, override_yaml: &str) -> Result<String, JsError> {
    let mut base: noyalib::Value =
        noyalib::from_str(base_yaml).map_err(|e| JsError::new(&e.to_string()))?;
    let overrides: noyalib::Value =
        noyalib::from_str(override_yaml).map_err(|e| JsError::new(&e.to_string()))?;
    base.merge(overrides);
    noyalib::to_string(&base).map_err(|e| JsError::new(&e.to_string()))
}
