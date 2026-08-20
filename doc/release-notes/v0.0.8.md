<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# noyalib v0.0.8 Release Notes

The **FlowStyle Fix** cut. The headline is a real bug fix:
`SerializerConfig::flow_style` is now honored by the serializer, so
`FlowStyle::Flow` and `FlowStyle::Auto` finally produce the inline
collections they always advertised. Alongside it ships the routine
Dependabot backlog (cargo, GitHub Actions, and Docker base images) and
a pair of supply-chain gate updates for a new upstream advisory.

No public API change. No MSRV change (still 1.85). The one behavior
change is the FlowStyle fix itself: callers who set `FlowStyle::Flow`
or `FlowStyle::Auto` will now get inline output where before they
silently got block output. Callers on the default (`FlowStyle::Block`)
see identical output and can upgrade with no code edits.

## Highlights

* **`FlowStyle::Flow` and `FlowStyle::Auto` now work (#84).** The
  `flow_style` and `flow_threshold` fields were stored on
  `SerializerConfig` but never read by the emit path, so every
  collection rendered as block style regardless of config. The
  serializer now routes through the flow emitters:
  * `Flow` always renders inline: `[0, 1, 2, 3, 4]`, `{a: 1, b: 2}`.
  * `Auto` renders inline when the collection and its whole subtree
    stay within `flow_threshold` (default 4), and falls back to block
    otherwise. A block child nested inside a flow collection would be
    invalid YAML, so `Auto` is deliberately conservative.
  * `Block` is unchanged and remains the default.
* **Batched Dependabot backlog.** Eleven open bot PRs (#85 to #96)
  folded into one cut: three cargo bumps, five GitHub Actions bumps,
  and three Docker base-image digest bumps.
* **Supply-chain gates updated for RUSTSEC-2026-0173.**
  `proc-macro-error2` was marked unmaintained upstream. It reaches the
  tree build-time only via `validator_derive` and never ships in a
  release artefact, so it is ignored in both `deny.toml` and
  `.cargo/audit.toml` with a note to revisit once `validator` releases
  off it.

## What ships

### Fixed: SerializerConfig::flow_style is honored (#84)

`write_sequence` and `write_mapping` now consult `config.flow_style`
and dispatch to the existing `write_flow_sequence` /
`write_flow_mapping` emitters. Previously those emitters were only
reachable through the explicit `Flow(..)` value wrapper, so the global
config knob was dead. A new `auto_flow_eligible` helper enforces the
"flow is sticky downward" rule for `Auto` mode so the serializer never
emits an invalid block-inside-flow document. Four regression tests
cover `Flow`, the `Auto` threshold, the `Auto` nested fall-back, and
the unchanged `Block` default.

### Dependencies: batched Dependabot bumps (#85 to #96)

* cargo: `clap_complete` 4.6.3 to 4.6.5, `smallvec` 1.15.1 to 1.15.2,
  `memchr` 2.8.0 to 2.8.2.
* github-actions: `actions/checkout` 6.0.2 to 6.0.3,
  `taiki-e/install-action` 2.81.1 to 2.81.8,
  `KSXGitHub/github-actions-deploy-aur` 3.0.1 to 4.1.3,
  `docker/setup-buildx-action` 3.12.0 to 4.1.0,
  `docker/login-action` 3.7.0 to 4.2.0.
* docker: `rust:1.96-bookworm`, `debian:bookworm-slim`, and
  `gcr.io/distroless/cc-debian12` digests re-pinned.

### CI: supply-chain gates

* `supply-chain/config.toml`: `safe-to-deploy` exemptions added for
  `clap_complete` 4.6.5, `memchr` 2.8.2, and `smallvec` 1.15.2, since
  the patch bumps outran the pinned `imports.lock`.
* `deny.toml` and `.cargo/audit.toml`: `RUSTSEC-2026-0173`
  (`proc-macro-error2` unmaintained) ignored, build-time-only via
  `validator_derive`.

## Upgrading

```toml
[dependencies]
noyalib = "0.0.8"
```

Default-configuration callers need no changes. If you were setting
`FlowStyle::Flow` or `FlowStyle::Auto` and working around the missing
inline output, you can now drop the workaround and rely on the config.
