#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Noyalib
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Check float behavior and math dependencies from isolated consumers."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


SOURCE = r'''
#[test]
fn sexagesimal_preserves_fused_rounding() {
    let config = noyalib::ParserConfig::new().legacy_sexagesimal(true);
    let a = 1_125_899_906_842_627.0_f64;
    let expected = a.mul_add(60.0, 0.1);
    assert_ne!(expected.to_bits(), (a * 60.0 + 0.1).to_bits());
    let input = "1125899906842627:0.1";
    let typed: f64 = noyalib::from_str_with_config(input, &config).unwrap();
    let value: noyalib::Value = noyalib::from_str_with_config(input, &config).unwrap();
    assert_eq!(typed.to_bits(), expected.to_bits());
    assert_eq!(value.as_f64().unwrap().to_bits(), expected.to_bits());
}

#[test]
fn float_roundtrips_preserve_bits() {
    for bits in [
        0, 1 << 63, 1, (1 << 63) | 1,
        0x000f_ffff_ffff_ffff, 0x0010_0000_0000_0000,
        0x3ff0_0000_0000_0001, 0xbff0_0000_0000_0001,
        0x433f_ffff_ffff_ffff, 0x4340_0000_0000_0000,
        0x7fef_ffff_ffff_ffff, 0xffef_ffff_ffff_ffff,
    ] {
        let value = f64::from_bits(bits);
        let yaml = noyalib::to_string(&value).unwrap();
        let roundtrip: f64 = noyalib::from_str(&yaml).unwrap();
        assert_eq!(roundtrip.to_bits(), bits, "{yaml}");
    }
}

#[test]
fn integer_coercion_checks_fractional_parts() {
    for value in [-2.0, -0.0, 0.0, 2.0, 4_503_599_627_370_496.0] {
        let parsed: i64 = noyalib::from_value(&noyalib::Value::from(value)).unwrap();
        assert_eq!(parsed, value as i64);
    }
    for value in [-1.5, -f64::from_bits(1), f64::from_bits(1), 1.5] {
        assert!(noyalib::from_value::<i64>(&noyalib::Value::from(value)).is_err());
    }
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=["default", "std", "alloc", "all"], default="all")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    target = Path(os.environ.get("CARGO_TARGET_DIR", root / "target/float-consumers"))
    modes = ["default", "std", "alloc"] if args.mode == "all" else [args.mode]
    failed = []
    with tempfile.TemporaryDirectory(prefix="noyalib-float-consumers-") as scratch:
        for mode in modes:
            consumer = Path(scratch) / mode
            (consumer / "src").mkdir(parents=True)
            features = '["std"]' if mode == "std" else "[]"
            defaults = "true" if mode == "default" else "false"
            (consumer / "Cargo.toml").write_text(
                '[package]\nname = "noyalib-float-consumer-' + mode + '"\n'
                'version = "0.0.0"\nedition = "2024"\nrust-version = "1.89"\n'
                '[workspace]\nresolver = "3"\n[dependencies]\n'
                'noyalib = { path = ' + json.dumps(str(root / "crates/noyalib"))
                + ', default-features = ' + defaults + ', features = ' + features + ' }\n'
            )
            (consumer / "src/lib.rs").write_text(SOURCE)
            env = dict(os.environ, CARGO_TARGET_DIR=str(target / mode))
            manifest = ["--manifest-path", str(consumer / "Cargo.toml")]
            subprocess.run(["cargo", "generate-lockfile", *manifest], env=env, check=True)
            subprocess.run(["cargo", "test", "--locked", *manifest], env=env, check=True)
            graph = subprocess.check_output(
                ["cargo", "tree", "--locked", *manifest, "--edges", "normal,build",
                 "--prefix", "none", "--format", "{p}"],
                env=env, text=True,
            )
            if any(line.startswith("libm ") for line in graph.splitlines()):
                failed.append(mode)
                print(f"FAIL: libm is an active dependency of the isolated {mode} consumer", flush=True)
            else:
                print(f"PASS: isolated {mode} consumer has no active libm dependency", flush=True)
    if failed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
