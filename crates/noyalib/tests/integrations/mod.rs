// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Optional-feature adapters and compatibility shims.
//!
//! 11 files, compiled into one test binary.

#![allow(missing_docs)]

mod coerce_to_schema;
mod coerce_to_schema_extra;
mod compat_serde_yaml_messages;
mod config_macros;
mod figment_provider;
mod i18n_formatters;
mod include_directive;
mod parser_profiles;
mod serde_yaml_compat_config;
mod serde_yaml_contract;
mod validated_garde;
mod validated_validator;
