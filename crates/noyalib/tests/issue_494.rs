// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Run the source-spelling regression suite against the alloc-only library.

#![cfg(not(feature = "std"))]

#[path = "regressions/issue_494.rs"]
mod issue_494;
