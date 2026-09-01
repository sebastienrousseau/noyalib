// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Tree-Planting Robot — polymorphism, unit-aware angles, strict
//! floats, and tagged enum dispatch in one config-driven scenario.
//!
//! A fleet of autonomous tree-planting rovers reads its mission
//! plan from YAML. Each waypoint dispatches to one of several
//! action types — `drive`, `dig`, `plant`, `wait` — modelled as a
//! Rust enum. noyalib's [`robotics`](noyalib::robotics) module
//! brings:
//!
//! - [`Degrees`](noyalib::robotics::Degrees) — units in the source,
//!   ergonomics in the consumer.
//! - [`Radians`](noyalib::robotics::Radians) — auto-converts
//!   degrees-on-the-wire into radians-in-memory.
//! - [`StrictFloat`](noyalib::robotics::StrictFloat) — rejects
//!   `.inf` / `.nan` / values that lose precision through `f64`.
//!
//! The combination — internally tagged enum + strict numeric
//! types + unit-aware fields — covers the realistic shape of an
//! IaC / robotics config without any procedural macro magic.
//!
//! Run: `cargo run --example robotics_polymorphism --features robotics`

// The `robotics` module is deprecated (since 0.0.29) and leaves next
// release: `StrictFloat` is now `lossless_float::LosslessFloat`, and
// the `Degrees`/`Radians` newtypes go with the module. This example
// exercises the compat surface for its final release.
#![allow(deprecated)]

#[path = "support.rs"]
mod support;

use noyalib::robotics::{Degrees, Radians, StrictFloat};

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
#[allow(dead_code)]
enum Step {
    Drive {
        heading: Radians,
        distance_m: StrictFloat,
    },
    Dig {
        depth_m: StrictFloat,
        bit: String,
    },
    Plant {
        species: String,
        spacing_m: StrictFloat,
    },
    Wait {
        seconds: StrictFloat,
    },
    Pivot {
        relative: Degrees,
    },
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct Mission {
    fleet: String,
    site: String,
    plan: Vec<Step>,
}

const MISSION_YAML: &str = "\
fleet: noya-rover-3
site: north-paddock-A
plan:
  - action: drive
    heading: 90.0           # degrees, deserialised into radians
    distance_m: 12.5
  - action: pivot
    relative: 45.0
  - action: dig
    depth_m: 0.30
    bit: tungsten-carbide
  - action: plant
    species: quercus-robur
    spacing_m: 2.5
  - action: wait
    seconds: 3.0
  - action: drive
    heading: 180.0
    distance_m: 12.5
  - action: plant
    species: betula-pendula
    spacing_m: 2.5
";

fn main() {
    support::header("Robotics polymorphism — unit-aware enum dispatch");

    // ── Parse the whole mission ──────────────────────────────────────
    let mission: Mission = match noyalib::from_str(MISSION_YAML) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    };

    support::task_with_output("Mission decoded", || {
        vec![
            format!("fleet     = {}", mission.fleet),
            format!("site      = {}", mission.site),
            format!("waypoints = {}", mission.plan.len()),
        ]
    });

    // ── Dispatch over the polymorphic step list ──────────────────────
    support::task_with_output("Polymorphic dispatch over plan", || {
        let mut lines = Vec::new();
        for (i, step) in mission.plan.iter().enumerate() {
            let line = match step {
                Step::Drive {
                    heading,
                    distance_m,
                } => {
                    let degrees = heading.to_degrees().0;
                    format!(
                        "{:>2}. drive   bearing {:>6.1}° / {:>6.4} rad  for {:>5.2} m",
                        i + 1,
                        degrees,
                        heading.0,
                        distance_m.get()
                    )
                }
                Step::Pivot { relative } => {
                    format!("{:>2}. pivot   {:+6.1}° (relative)", i + 1, relative.0)
                }
                Step::Dig { depth_m, bit } => format!(
                    "{:>2}. dig     {:>5.2} m with bit `{}`",
                    i + 1,
                    depth_m.get(),
                    bit
                ),
                Step::Plant { species, spacing_m } => format!(
                    "{:>2}. plant   `{}` @ {:>5.2} m spacing",
                    i + 1,
                    species,
                    spacing_m.get()
                ),
                Step::Wait { seconds } => {
                    format!("{:>2}. wait    {:>5.2} s", i + 1, seconds.get())
                }
            };
            lines.push(line);
        }
        lines
    });

    // ── StrictFloat catches malformed numerics ──────────────────────
    support::task_with_output("StrictFloat rejects `.inf` and NaN", || {
        let bad = "
fleet: rover
site: test
plan:
  - action: drive
    heading: 0.0
    distance_m: .inf
";
        let res: Result<Mission, _> = noyalib::from_str(bad);
        match res {
            Ok(_) => vec!["unexpected: parse succeeded with .inf".into()],
            Err(e) => vec![format!("rejected (as designed): {e}")],
        }
    });

    // ── Degrees ↔ Radians round-trip ────────────────────────────────
    support::task_with_output("Degrees / Radians round-trip is precise", || {
        let d = Degrees(90.0);
        let r = d.to_radians();
        let d2 = r.to_degrees();
        vec![
            format!("Degrees(90)   -> Radians({:.10})", r.0),
            format!("Radians({:.4}) -> Degrees({:.10})", r.0, d2.0),
            format!("delta from origin: {:.2e}", (d.0 - d2.0).abs()),
        ]
    });

    println!();
    println!("  Robotics-shaped configs benefit twice over: serde tagged");
    println!("  enums dispatch on a `kind` field, while noyalib's robotics");
    println!("  newtypes catch unit / precision bugs at the *parse* boundary");
    println!("  rather than as silent NaNs deep inside a control loop.");

    support::footer();
}
