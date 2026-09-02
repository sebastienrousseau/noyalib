// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Scientific numeric profile: LosslessFloat plus a domain newtype.
//!
//! Demonstrates precise numeric types for simulation and scientific
//! computing pipelines that deserialize from YAML. The `Radians`
//! newtype below is defined in this example: domain unit types left
//! the crate with the `robotics` module (v0.0.30), and the intended
//! migration is to copy the handful of lines into your own code.
//!
//! Run: `cargo run --example scientific --features lossless-float`

#[path = "support.rs"]
mod support;

fn main() {
    support::header("noyalib -- scientific (lossless numeric profile)");

    #[cfg(not(feature = "lossless-float"))]
    {
        println!("  This example requires the 'lossless-float' feature.");
        println!("  Run: cargo run --example scientific --features lossless-float");
        println!();
    }

    #[cfg(feature = "lossless-float")]
    run_examples();
}

/// An angle stored in radians but deserialized from degrees in YAML.
/// Serialization emits the raw radian value. This is the former
/// `noyalib::robotics::Radians`, copied here per its migration note.
#[cfg(feature = "lossless-float")]
#[derive(Debug, Clone, Copy, PartialEq)]
struct Radians(pub f64);

#[cfg(feature = "lossless-float")]
impl Radians {
    fn to_degrees(self) -> f64 {
        self.0.to_degrees()
    }
}

#[cfg(feature = "lossless-float")]
impl<'de> serde::Deserialize<'de> for Radians {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let degrees = f64::deserialize(de)?;
        Ok(Self(degrees.to_radians()))
    }
}

#[cfg(feature = "lossless-float")]
impl serde::Serialize for Radians {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_f64(self.0)
    }
}

#[cfg(feature = "lossless-float")]
fn run_examples() {
    use noyalib::lossless_float::LosslessFloat;

    // ── LosslessFloat: valid values ─────────────────────────────────
    support::task_with_output("LosslessFloat: valid values", || {
        let cases: &[(&str, f64)] = &[
            ("0.0", 0.0),
            ("1.0", 1.0),
            ("-1.0", -1.0),
            ("9.81", 9.81),
            ("1.0e10", 1.0e10),
        ];
        let mut lines = Vec::new();
        for &(yaml, expected) in cases {
            let sf: LosslessFloat = noyalib::from_str(yaml).unwrap();
            assert!((sf.get() - expected).abs() < 1e-10);
            lines.push(format!("{yaml:>12} -> {}", sf.get()));
        }
        lines
    });

    // ── LosslessFloat: rejection of NaN/Infinity ────────────────────
    support::task_with_output("LosslessFloat: rejects NaN and Infinity", || {
        let mut lines = Vec::new();
        for yaml in &[".nan", ".inf", "-.inf"] {
            let result: Result<LosslessFloat, _> = noyalib::from_str(yaml);
            assert!(result.is_err());
            lines.push(format!("{yaml:>8} -> rejected ({})", result.unwrap_err()));
        }
        lines
    });

    // ── Radians: degree-to-radian conversion via a local newtype ────
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

    // ── Sensor calibration use case ─────────────────────────────────
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
                j.to_degrees()
            ));
        }
        lines
    });

    // ── Round-trip: serialize Radians back to YAML ──────────────────
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

    // ── LosslessFloat in a struct ───────────────────────────────────
    support::task_with_output("LosslessFloat in a calibration struct", || {
        let yaml = r"
offset_x: 0.001
offset_y: -0.002
scale: 1.00015
";
        #[derive(Debug, serde::Deserialize)]
        struct Calibration {
            offset_x: LosslessFloat,
            offset_y: LosslessFloat,
            scale: LosslessFloat,
        }
        let cal: Calibration = noyalib::from_str(yaml).unwrap();
        vec![
            format!("offset_x = {}", cal.offset_x.get()),
            format!("offset_y = {}", cal.offset_y.get()),
            format!("scale    = {}", cal.scale.get()),
        ]
    });

    support::summary(6);
}
