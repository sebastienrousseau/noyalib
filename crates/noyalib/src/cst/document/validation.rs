// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Typed-cache validation and atomic document-state replacement.

use super::{Document, RepairScope};
use crate::cst::builder::parse_full;
#[cfg(test)]
use crate::error::Error;
use crate::error::Result;

impl Document {
    /// Populate the typed cache from `self.source` if it is empty.
    /// Every public edit validates the complete resulting document
    /// before committing it, so a failure here indicates an internal
    /// invariant violation rather than caller-provided YAML.
    pub(super) fn ensure_cache(&self) {
        if self.cache.borrow().is_some() {
            return;
        }
        let parsed = crate::parser::parse_exactly_one(&self.source, &self.config)
            .expect("Document source must always parse — local repair invariant violated");
        *self.cache.borrow_mut() = Some(parsed);
    }

    /// Verify that the current source re-parses cleanly.
    ///
    /// Public mutators validate the complete result before committing,
    /// so this method normally returns immediately from the populated
    /// typed cache. It remains useful as an explicit integrity check for
    /// callers that accept a `Document` from another component.
    ///
    /// # Errors
    ///
    /// Returns the underlying parse error if the source no longer
    /// parses as a single YAML document.
    ///
    /// # Examples
    ///
    /// A malformed edit is rejected atomically and the original remains
    /// valid:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let mut doc = parse_document("name: foo\n").unwrap();
    /// assert!(doc.set("name", "[").is_err());
    /// assert!(doc.validate().is_ok());
    /// assert_eq!(doc.to_string(), "name: foo\n");
    /// ```
    ///
    /// Validate a freshly-parsed document — always succeeds:
    ///
    /// ```
    /// use noyalib::cst::parse_document;
    ///
    /// let doc = parse_document("name: foo\n").unwrap();
    /// assert!(doc.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<()> {
        #[cfg(test)]
        if super::fault::validate_should_fail() {
            return Err(Error::Parse("injected validate failure".into()));
        }
        if self.cache.borrow().is_some() {
            return Ok(());
        }
        let parsed = crate::parser::parse_exactly_one(&self.source, &self.config)?;
        *self.cache.borrow_mut() = Some(parsed);
        Ok(())
    }

    /// Validate `candidate` and atomically replace every document view.
    pub(super) fn commit_source(
        &mut self,
        candidate: &str,
        repair_scope: RepairScope,
    ) -> Result<()> {
        let parsed = parse_full(candidate, &self.config)?;
        self.source = parsed.source;
        self.green = parsed.green;
        let _ = self.cache.replace(Some((parsed.value, parsed.span_tree)));
        self.last_repair_scope.set(Some(repair_scope));
        Ok(())
    }
}
