// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Robotics/scientific numeric profile: StrictFloat, Radians, Degrees.
//!
//! Demonstrates precise numeric types for robotics, simulation, and
//! scientific computing pipelines that deserialize from YAML.
//!
//! Run: `cargo run --example scientific --features robotics`

// The `robotics` module is deprecated (since 0.0.29) and leaves next
// release: `StrictFloat` is now `lossless_float::LosslessFloat`, and
// the `Degrees`/`Radians` newtypes go with the module. This example
// exercises the compat surface for its final release.
#![allow(deprecated)]

#[path = "support.rs"]
mod support;

fn main() {
    support::header("noyalib -- scientific (robotics numeric profile)");

    #[cfg(not(feature = "robotics"))]
    {
        println!("  This example requires the 'robotics' feature.");
        println!("  Run: cargo run --example scientific --features robotics");
        println!();
    }

    #[cfg(feature = "robotics")]
    run_robotics_examples();
}

#[cfg(feature = "robotics")]
fn run_robotics_examples() {
    use noyalib::robotics::{Degrees, Radians, StrictFloat};

    // ── StrictFloat: valid values ────────────────────────────────────
    support::task_with_output("StrictFloat: valid values", || {
        let cases: &[(&str, f64)] = &[
            ("0.0", 0.0),
            ("1.0", 1.0),
            ("-1.0", -1.0),
            ("9.81", 9.81),
            ("1.0e10", 1.0e10),
        ];
        let mut lines = Vec::new();
        for &(yaml, expected) in cases {
            let sf: StrictFloat = noyalib::from_str(yaml).unwrap();
            assert!((sf.get() - expected).abs() < 1e-10);
            lines.push(format!("{yaml:>12} -> {}", sf.get()));
        }
        lines
    });

    // ── StrictFloat: rejection of NaN/Infinity ──────────────────────
    support::task_with_output("StrictFloat: rejects NaN and Infinity", || {
        let mut lines = Vec::new();
        for yaml in &[".nan", ".inf", "-.inf"] {
            let result: Result<StrictFloat, _> = noyalib::from_str(yaml);
            assert!(result.is_err());
            lines.push(format!("{yaml:>8} -> rejected ({})", result.unwrap_err()));
        }
        lines
    });

    // ── Radians: degree-to-radian conversion ─────────────────────────
    support::task_with_output("Radians: degrees in YAML -> radians in Rust", || {
        let cases: &[(&str, f64)] = &[
            ("0.0", 0.0),
            ("90.0", core::f64::consts::FRAC_PI_2),
            ("180.0", core::f64::consts::PI),
            ("360.0", core::f64::consts::TAU),
            ("-90.0", -core::f64::consts::FRAC_PI_2),
        ];
        let mut lines = Vec::new();
        for &(yaml, expected_rad) in cases {
            let r: Radians = noyalib::from_str(yaml).unwrap();
            assert!((r.0 - expected_rad).abs() < 1e-10);
            lines.push(format!("{yaml:>8} deg -> {:.6} rad", r.0));
        }
        lines
    });

    // ── Degrees: transparent wrapper ─────────────────────────────────
    support::task_with_output("Degrees: transparent wrapper and conversion", || {
        let d: Degrees = noyalib::from_str("45.0").unwrap();
        assert!((d.0 - 45.0).abs() < 1e-15);
        let r = d.to_radians();
        assert!((r.0 - core::f64::consts::FRAC_PI_4).abs() < 1e-10);
        let back = r.to_degrees();
        assert!((back.0 - 45.0).abs() < 1e-10);
        vec![
            format!("Degrees(45.0) -> Radians({:.6})", r.0),
            format!("Radians({:.6}) -> Degrees({:.1}) (roundtrip)", r.0, back.0),
        ]
    });

    // ── Sensor calibration use case ──────────────────────────────────
    support::task_with_output("Sensor calibration: joint angles from YAML", || {
        let yaml = r"
joint1: 90.0
joint2: -45.0
joint3: 180.0
joint4: 0.0
joint5: 270.0
joint6: 135.0
";
        #[derive(Debug, serde::Deserialize)]
        struct RobotArm {
            joint1: Radians,
            joint2: Radians,
            joint3: Radians,
            joint4: Radians,
            joint5: Radians,
            joint6: Radians,
        }
        let arm: RobotArm = noyalib::from_str(yaml).unwrap();
        let joints = [
            arm.joint1, arm.joint2, arm.joint3, arm.joint4, arm.joint5, arm.joint6,
        ];
        let mut lines = Vec::new();
        for (i, j) in joints.iter().enumerate() {
            lines.push(format!(
                "joint{}: {:.4} rad ({:.1} deg)",
                i + 1,
                j.0,
                j.to_degrees().0
            ));
        }
        lines
    });

    // ── Round-trip: serialize Radians back to YAML ───────────────────
    support::task_with_output("Round-trip: serialize Radians back to YAML", || {
        let r = Radians(core::f64::consts::PI);
        let yaml = noyalib::to_string(&r).unwrap();
        let parsed: f64 = noyalib::from_str(yaml.trim()).unwrap();
        assert!((parsed - core::f64::consts::PI).abs() < 1e-10);
        vec![
            format!("Radians(PI) serialized as: {}", yaml.trim()),
            format!("Deserialized back as f64: {parsed:.10}"),
        ]
    });

    // ── StrictFloat in a struct ──────────────────────────────────────
    support::task_with_output("StrictFloat in a calibration struct", || {
        let yaml = r"
offset_x: 0.001
offset_y: -0.002
scale: 1.00015
";
        #[derive(Debug, serde::Deserialize)]
        struct Calibration {
            offset_x: StrictFloat,
            offset_y: StrictFloat,
            scale: StrictFloat,
        }
        let cal: Calibration = noyalib::from_str(yaml).unwrap();
        vec![
            format!("offset_x = {}", cal.offset_x.get()),
            format!("offset_y = {}", cal.offset_y.get()),
            format!("scale    = {}", cal.scale.get()),
        ]
    });

    support::summary(7);
}
