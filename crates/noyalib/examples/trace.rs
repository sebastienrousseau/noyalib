//! Error path tracking example for noyalib.
//!
//! Demonstrates the `Path` type for tracking locations within YAML structures.
//! Useful for providing detailed error messages that show exactly where
//! in the document a problem occurred.
//!
//! Run: `cargo run --example trace`

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

#[path = "support.rs"]
mod support;

use noyalib::Path;

fn main() {
    support::header("noyalib -- trace");

    support::task_with_output("Building paths step by step", || {
        let root = Path::Root;
        let config = root.key("config");
        let database = config.key("database");
        let host = database.key("host");

        vec![
            format!("Root path: {root}"),
            format!("config: {config}"),
            format!("config.database: {database}"),
            format!("config.database.host: {host}"),
        ]
    });

    support::task_with_output("Sequence indices", || {
        let root = Path::Root;

        let servers = root.key("servers");
        let first_server = servers.index(0);
        let server_host = first_server.key("host");

        let clusters = root.key("clusters");
        let cluster0 = clusters.index(0);
        let nodes = cluster0.key("nodes");
        let node2 = nodes.index(2);
        let ip = node2.key("ip");

        vec![
            format!("servers: {servers}"),
            format!("servers[0]: {first_server}"),
            format!("servers[0].host: {server_host}"),
            format!("clusters[0].nodes[2].ip: {ip}"),
        ]
    });

    support::task_with_output("Error reporting simulation", || {
        struct ValidationError {
            path: String,
            message: String,
        }

        fn validate_config(path: &Path<'_>, errors: &mut Vec<ValidationError>) {
            let db_path = path.key("database");
            let port_path = db_path.key("port");

            errors.push(ValidationError {
                path: port_path.to_string(),
                message: "port must be between 1 and 65535".to_string(),
            });

            let users_path = path.key("users");
            let user_path = users_path.index(2);
            let email_path = user_path.key("email");

            errors.push(ValidationError {
                path: email_path.to_string(),
                message: "invalid email format".to_string(),
            });
        }

        let mut errors = Vec::new();
        validate_config(&Path::Root, &mut errors);

        let mut lines = vec!["Validation errors:".to_string()];
        for error in &errors {
            lines.push(format!("  - {}: {}", error.path, error.message));
        }
        lines
    });

    support::task_with_output("Path navigation", || {
        let level1 = Path::Root.key("level1");
        let level2 = level1.key("level2");
        let level2_0 = level2.index(0);
        let level3 = level2_0.key("level3");

        let mut lines = vec![format!("Full path: {level3}")];

        if let Some(parent) = level3.parent() {
            lines.push(format!("Parent 1: {parent}"));
            if let Some(grandparent) = parent.parent() {
                lines.push(format!("Parent 2: {grandparent}"));
                if let Some(great_grandparent) = grandparent.parent() {
                    lines.push(format!("Parent 3: {great_grandparent}"));
                }
            }
        }
        lines
    });

    support::task_with_output("Alias tracking", || {
        let defaults = Path::Root.key("defaults");
        let production = Path::Root.key("production");
        let alias_usage = production.alias();

        vec![
            format!("Anchor location: {defaults}"),
            format!("Alias usage: {alias_usage}"),
        ]
    });

    support::task_with_output("Unknown path elements", || {
        let data = Path::Root.key("data");
        let unknown = data.unknown();

        vec![format!("Unknown element: {unknown}")]
    });

    support::task_with_output("Formatted error messages", || {
        fn format_error(path: &Path<'_>, expected: &str, found: &str) -> String {
            format!(
                "Type mismatch at `{}`:\n  expected: {}\n  found: {}",
                path, expected, found
            )
        }

        let services = Path::Root.key("services");
        let service0 = services.index(0);
        let port = service0.key("port");

        let error_msg = format_error(&port, "integer", "string \"http\"");
        error_msg.lines().map(|l| l.to_string()).collect()
    });

    support::task_with_output("Path comparison", || {
        let config1 = Path::Root.key("config");
        let port1 = config1.key("port");

        let config2 = Path::Root.key("config");
        let port2 = config2.key("port");

        let config3 = Path::Root.key("config");
        let host3 = config3.key("host");

        vec![
            format!("path1: {port1}"),
            format!("path2: {port2}"),
            format!("path3: {host3}"),
            format!("path1 == path2: {}", port1 == port2),
            format!("path1 == path3: {}", port1 == host3),
        ]
    });

    support::task_with_output("Building paths dynamically", || {
        let items = Path::Root.key("items");

        (0..3)
            .map(|i| {
                let item = items.index(i);
                let name = item.key("name");
                format!("Item {i} path: {name}")
            })
            .collect()
    });

    support::summary(9);
}
