// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Integration tests for the `Spanned<T>` + garde/validator →
//! miette bridge.

#![cfg(feature = "miette")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use noyalib::Spanned;

#[cfg(feature = "garde")]
mod garde_bridge {
    use super::*;
    use garde::Validate;
    use noyalib::validated_miette::garde_errors_to_miette;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, Validate)]
    struct Cfg {
        #[garde(range(min = 1024, max = 65535))]
        port: u16,
    }

    #[test]
    fn out_of_range_port_renders_with_source_label() {
        let yaml = "port: 22\n";
        let wrapped: Spanned<Cfg> = noyalib::from_str(yaml).unwrap();
        let errs = wrapped.value.validate().unwrap_err();
        let report = garde_errors_to_miette(&wrapped, &errs, yaml, "config.yaml");
        let rendered = format!("{report:?}");
        // Render path goes through miette's Debug → ReportHandler;
        // the summary should land in the output verbatim.
        assert!(rendered.contains("validation failed"));
    }

    #[test]
    fn report_summary_via_display() {
        let yaml = "port: 22\n";
        let wrapped: Spanned<Cfg> = noyalib::from_str(yaml).unwrap();
        let errs = wrapped.value.validate().unwrap_err();
        let report = garde_errors_to_miette(&wrapped, &errs, yaml, "config.yaml");
        let s = format!("{report}");
        assert!(s.contains("validation failed"), "{s}");
        assert!(s.contains("port"), "{s}");
    }

    #[test]
    fn valid_input_does_not_call_bridge() {
        // Sanity: a valid payload's `validate()` returns Ok, so
        // the bridge is never engaged. Smoke-test the happy path.
        let yaml = "port: 8080\n";
        let wrapped: Spanned<Cfg> = noyalib::from_str(yaml).unwrap();
        assert!(wrapped.value.validate().is_ok());
    }
}

#[cfg(feature = "validator")]
mod validator_bridge {
    use super::*;
    use noyalib::validated_miette::validator_errors_to_miette;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct Cfg {
        #[validate(range(min = 1024, max = 65535))]
        port: u16,
    }

    #[test]
    fn out_of_range_port_renders() {
        let yaml = "port: 22\n";
        let wrapped: Spanned<Cfg> = noyalib::from_str(yaml).unwrap();
        let errs = wrapped.value.validate().unwrap_err();
        let report = validator_errors_to_miette(&wrapped, &errs, yaml, "config.yaml");
        let s = format!("{report}");
        assert!(s.contains("validation failed"), "{s}");
    }
}

#[test]
fn clamped_span_handles_overflow() {
    use noyalib::validated_miette::clamped_span;
    let span = clamped_span(0, 50, 10);
    assert_eq!(span.offset(), 0);
    assert_eq!(span.len(), 10);

    let degenerate = clamped_span(5, 5, 10);
    assert!(!degenerate.is_empty());
}
