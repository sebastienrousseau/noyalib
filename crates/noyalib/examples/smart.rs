// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Smart pointer anchors: RcAnchor, ArcAnchor for shared YAML data.
//!
//! Run: `cargo run --example smart`

#[path = "support.rs"]
mod support;

use std::sync::Arc;

use noyalib::{ArcAnchor, RcAnchor, Value, from_str, to_string};

fn main() {
    support::header("noyalib -- smart");

    support::task_with_output("RcAnchor: serialize via Rc<T>", || {
        let shared: RcAnchor<String> = RcAnchor::from("shared-config".to_string());
        let yaml = to_string(&shared).unwrap();
        vec![format!("Output: {}", yaml.trim())]
    });

    support::task_with_output("ArcAnchor: serialize via Arc<T>", || {
        let shared: ArcAnchor<i64> = ArcAnchor::from(42i64);
        let yaml = to_string(&shared).unwrap();
        vec![format!("Output: {}", yaml.trim())]
    });

    let yaml = r#"
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  database: dev_db

production:
  <<: *defaults
  database: prod_db
  host: db.example.com
"#;
    let config: Value = from_str(yaml).unwrap();

    support::task_with_output("anchor/alias merge: development inherits defaults", || {
        vec![
            format!(
                "adapter  = {} (inherited via *defaults)",
                config["development"]
                    .get("adapter")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "database = {} (local)",
                config["development"]
                    .get("database")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
        ]
    });

    support::task_with_output("anchor/alias merge: production overrides host", || {
        vec![
            format!(
                "host     = {} (overridden)",
                config["production"]
                    .get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
            format!(
                "adapter  = {} (inherited)",
                config["production"]
                    .get("adapter")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            ),
        ]
    });

    support::task_with_output("ArcAnchor from Arc (shared ownership)", || {
        let data = Arc::new("thread-safe-value".to_string());
        let anchor: ArcAnchor<String> = ArcAnchor::from(data.clone());
        vec![
            format!("Value:        {}", *anchor),
            format!(
                "Strong count: {} (Arc + ArcAnchor)",
                Arc::strong_count(&data)
            ),
        ]
    });

    support::summary(5);
}
