<!-- SPDX-FileCopyrightText: 2026 Noyalib -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# Internals

A map of the crate for people changing it. Read
[ARCHITECTURE.md](ARCHITECTURE.md) first for *why* the pieces are
shaped this way; this is *where* they are.

**Generated** by `scripts/generate-reference-docs.sh`. Do not edit by
hand. `tests/reference_docs_are_complete.rs` fails when a module
exists that this map does not list.

## Hot paths

The longest functions in the crate. Length is a proxy, not a verdict —
a scanner's token dispatch is long because a YAML token has many
shapes — but these are where the time goes and where a change is most
likely to cost something. The `benches/` directory measures them.

| Lines | Location | Function |
| --- | --- | --- |
| 603 | `src/cst/document.rs` | `insert_entry_value` |
| 510 | `src/cst/document.rs` | `is_flow_collection` |
| 372 | `src/parser/loader.rs` | `process_event` |
| 302 | `src/parser/loader.rs` | `process_event` |
| 298 | `src/parser/scanner/scalars.rs` | `scan_block_scalar` |
| 234 | `src/cst/document.rs` | `entry_line_span` |
| 230 | `src/parser/loader.rs` | `push_node` |
| 214 | `src/parser/scanner/scalars.rs` | `scan_double_quoted_scalar` |
| 200 | `src/parser/scanner.rs` | `fetch_tag` |
| 198 | `src/parser/scanner.rs` | `fetch_value` |
| 177 | `src/parser/loader.rs` | `push_value` |
| 167 | `src/parser/events.rs` | `parse_node` |

## Module map

68 modules.

| Module | Lines | Purpose |
| --- | --- | --- |
| `anchors.rs` | 1151 | Smart pointer anchor types for shared/DAG structures. |
| `ariadne_adapter.rs` | 99 | [`ariadne`] adapter for [`crate::Error`]. |
| `base64.rs` | 262 | Internal base64 codec for `!!binary` scalars (YAML 1.2.2 §10.4). |
| `borrowed.rs` | 869 | Zero-copy YAML values that borrow strings from the input. |
| `comments.rs` | 244 | Comment capture on the parse path. |
| `compat/mod.rs` | 19 | Compatibility shims for downstream crates migrating to `noyalib`. |
| `compat/serde_yaml.rs` | 920 | Drop-in API surface compatible with `serde_yaml` 0.9. |
| `cst/anchor.rs` | 709 | Anchor and alias management. |
| `cst/annotated.rs` | 905 | Comment-aware read view over a [`crate::cst::Document`]. |
| `cst/builder.rs` | 597 | Build the parts of a [`crate::cst::Document`] from input bytes. |
| `cst/coerce.rs` | 232 | Lossless schema-driven type coercion on the CST path. |
| `cst/document.rs` | 7684 | Public `Document` handle and parse / mutation entry points. |
| `cst/emit.rs` | 463 | Auto-formatting for values spliced by the CST insertion mutators. |
| `cst/entry.rs` | 671 | Path-shaped mutable handle to a CST node — the `Entry` "pro" |
| `cst/format.rs` | 455 | Formatter for YAML CST. |
| `cst/green.rs` | 231 | Immutable green-node primitive with relative-length leaves. |
| `cst/mod.rs` | 126 | Side-table CST (concrete syntax tree) for lossless round-tripping. |
| `cst/syntax.rs` | 135 | Syntax-kind tags for green-tree nodes and tokens. |
| `de/config.rs` | 1177 | Parser configuration types. |
| `de/deserializer.rs` | 1041 | The serde `Deserializer` over a `&Value` and its access types. |
| `de.rs` | 1137 | YAML Deserialization. |
| `diagnostic.rs` | 188 | Spanned value to `miette::Report` bridge. |
| `doc_boundary.rs` | 217 | Workspace-private `---` document-boundary scanner. |
| `document.rs` | 436 | Multi-document YAML loading. |
| `error.rs` | 2137 | Error handling types. |
| `figment.rs` | 81 | [`figment`] provider for noyalib YAML. |
| `flattened.rs` | 165 | `Flattened<T>` — capture the underlying [`Value`] alongside the |
| `fmt.rs` | 635 | Formatting wrappers for fine-grained control over YAML output style. |
| `i18n.rs` | 172 | Pluggable error-message formatters for user-facing rendering. |
| `include.rs` | 316 | `!include` directive support — compose YAML documents from |
| `interner.rs` | 304 | Key interning for memory-efficient repeated-key workloads. |
| `lossless_float.rs` | 180 | A float that refuses to silently lose information. |
| `macros.rs` | 99 | Declarative builders for the public config types. |
| `parallel.rs` | 356 | Parallel multi-document YAML parsing — the "MapReduce" path. |
| `parser/budget.rs` | 253 | The resource budgets the loaders enforce, as pure predicates. |
| `parser/events.rs` | 783 | YAML 1.2 event-based parser. |
| `parser/loader.rs` | 2540 | Event-to-Value tree builder with security limits. |
| `parser/mod.rs` | 103 | Native YAML 1.2 parser. |
| `parser/scanner/scalars.rs` | 1191 | Scalar scanning for the YAML scanner: plain, single/double-quoted |
| `parser/scanner.rs` | 2312 | YAML 1.2 lexical scanner. |
| `path.rs` | 867 | Path tracking for YAML structure locations. |
| `policy.rs` | 271 | Pluggable parser policies for "Safe YAML" enforcement. |
| `recovery.rs` | 581 | Error-recovering YAML parser for LSP / IDE partial parsing. |
| `schema.rs` | 334 | YAML 1.2 schema validation helpers. |
| `schema_codegen.rs` | 322 | JSON Schema codegen for Rust types. |
| `schema_validate.rs` | 704 | JSON Schema 2020-12 validation against a parsed [`crate::Value`]. |
| `ser.rs` | 2390 | YAML serialization. |
| `simd.rs` | 1531 | SIMD-friendly structural-scanning primitives. |
| `span_context.rs` | 176 | Thread-local span context for wiring source locations into `Spanned<T>`. |
| `spanned.rs` | 291 | Source location tracking for deserialized values. |
| `streaming.rs` | 2358 | Streaming YAML deserializer that operates directly on parser events. |
| `sval_adapter.rs` | 442 | `sval` adapter — stream noyalib values through any |
| `tag_registry.rs` | 211 | Streaming-path registry for custom YAML tag pass-through. |
| `tokio_async.rs` | 547 | Native async YAML parsing for [`tokio`](https://tokio.rs) |
| `validated.rs` | 262 | Declarative validation via [`garde`] or [`validator`]. |
| `validated_miette.rs` | 265 | [`Spanned<T>`] + `garde` / `validator` → `miette::Report` |
| `value/arbitrary_impls.rs` | 150 | [`arbitrary::Arbitrary`] for the public value types, behind the |
| `value/convert.rs` | 237 | `From<T> for Value` conversions and `Index`/`IndexMut`. |
| `value/mapping.rs` | 1356 | YAML mapping types (`Mapping`, `MappingAny`). |
| `value/number.rs` | 631 | YAML number type (`Number`). |
| `value/serde_impl.rs` | 335 | serde `Serialize`/`Deserialize` for `Value`. |
| `value/tag.rs` | 505 | YAML tag types (`Tag`, `TaggedValue`) and tag utilities. |
| `value.rs` | 1723 | YAML value types. |
| `with/mod.rs` | 54 | Helper modules for customizing serialization and deserialization. |
| `with/singleton_map.rs` | 212 | Serialize enums as single-entry maps. |
| `with/singleton_map_optional.rs` | 265 | Serialize optional enums as single-entry maps. |
| `with/singleton_map_recursive.rs` | 176 | Recursively serialize enums as single-entry maps. |
| `with/singleton_map_with.rs` | 460 | Serialize enums as single-entry maps with custom key transformation. |
