// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! The schema codegen and validation suites.
//!
//! 4 files, compiled into one test binary instead of
//! 4 executables. The files themselves are unchanged.

#![allow(missing_docs)]

mod schema_codegen;
mod schema_compiled;
mod schema_hardening;
mod schema_validate;
