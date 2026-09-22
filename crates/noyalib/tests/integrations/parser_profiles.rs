// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Contract tests for named parser profiles and isolated resource limits.

use noyalib::{
    DuplicateKeyPolicy, MergeKeyPolicy, NonScalarKeyPolicy, ParserConfig, ParserLimits,
    ParserProfile, RequireIndent, YamlVersion,
};

#[test]
fn standard_profile_matches_default_config() {
    let profile = ParserConfig::profile(ParserProfile::Standard);
    let default = ParserConfig::default();

    assert_eq!(profile.limits(), default.limits());
    assert_eq!(profile.yaml_version, default.yaml_version);
    assert_eq!(profile.duplicate_key_policy, default.duplicate_key_policy);
    assert_eq!(profile.require_indent, default.require_indent);
}

#[test]
fn strict_profile_preserves_existing_contract() {
    let profile = ParserConfig::profile(ParserProfile::Strict);
    let strict = ParserConfig::strict();

    assert_eq!(profile.limits(), ParserLimits::strict());
    assert_eq!(profile.limits(), strict.limits());
    assert_eq!(profile.yaml_version, YamlVersion::V1_2);
    assert_eq!(profile.duplicate_key_policy, DuplicateKeyPolicy::Error);
    assert!(profile.strict_booleans);
    assert_eq!(profile.require_indent, RequireIndent::Even);
    #[cfg(feature = "std")]
    assert!(profile.strict_properties);
}

#[test]
fn serde_yaml_profile_preserves_existing_contract() {
    let profile = ParserConfig::profile(ParserProfile::SerdeYaml);
    let compatibility = ParserConfig::serde_yaml_compat();

    assert_eq!(profile.limits(), compatibility.limits());
    assert_eq!(
        profile.limits(),
        ParserLimits::profile(ParserProfile::SerdeYaml)
    );
    assert_eq!(profile.merge_key_policy, MergeKeyPolicy::AsOrdinary);
    assert_eq!(profile.non_scalar_key_policy, NonScalarKeyPolicy::Error);
    assert_eq!(profile.alias_jump_event_factor, Some(100));
}

#[test]
fn replacing_limits_preserves_semantic_settings() {
    let config = ParserConfig::new()
        .version(YamlVersion::V1_1)
        .merge_key_policy(MergeKeyPolicy::Error)
        .with_limits(ParserLimits::strict());

    assert_eq!(config.limits(), ParserLimits::strict());
    assert_eq!(config.yaml_version, YamlVersion::V1_1);
    assert_eq!(config.merge_key_policy, MergeKeyPolicy::Error);
    assert!(config.legacy_booleans);
    assert!(config.legacy_octal_numbers);
    assert!(config.legacy_sexagesimal);
}

#[test]
fn extracted_limits_can_be_refined_and_reapplied() {
    let mut limits = ParserConfig::strict().limits();
    limits.max_document_length = 32 * 1024;
    limits.max_documents = 2;

    let config = ParserConfig::new().with_limits(limits);
    assert_eq!(config.max_document_length, 32 * 1024);
    assert_eq!(config.max_documents, 2);
    assert_eq!(config.limits(), limits);
}
