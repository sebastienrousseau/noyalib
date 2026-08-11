//! Regression test for #239: `from_str_strict` rejected any populated
//! `Option<T>` field.
//!
//! The report was `Option<String>` with `""`, but the defect was in the
//! `&Value` deserialiser forwarding `deserialize_option` to
//! `deserialize_any`, so it applied to every `T` and every non-null
//! value. `~` was the only input that worked, because `deserialize_any`
//! maps null to `visit_unit` and serde reads that as `None`.

use noyalib::from_str_strict;

#[derive(serde::Deserialize, Debug, PartialEq)]
struct WithString {
    f: Option<String>,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct WithInt {
    f: Option<i64>,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct WithSeq {
    f: Option<Vec<u8>>,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct Inner {
    a: u8,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
struct WithStruct {
    f: Option<Inner>,
}

#[test]
fn empty_string_the_case_reported() {
    let got: WithString = from_str_strict(r#"f: """#).expect("empty string into Option<String>");
    assert_eq!(got.f, Some(String::new()));
}

#[test]
fn non_empty_string_was_equally_broken() {
    let got: WithString = from_str_strict(r#"f: "hello""#).expect("string into Option<String>");
    assert_eq!(got.f, Some("hello".to_owned()));
}

#[test]
fn integers_were_broken_too() {
    let got: WithInt = from_str_strict("f: 7").expect("integer into Option<i64>");
    assert_eq!(got.f, Some(7));
}

#[test]
fn sequences_and_structs_round_trip() {
    let seq: WithSeq = from_str_strict("f: [1, 2]").expect("sequence into Option<Vec<u8>>");
    assert_eq!(seq.f, Some(vec![1, 2]));

    let nested: WithStruct = from_str_strict("f:\n  a: 3").expect("map into Option<Inner>");
    assert_eq!(nested.f, Some(Inner { a: 3 }));
}

#[test]
fn null_and_absent_still_deserialise_to_none() {
    let tilde: WithString = from_str_strict("f: ~").expect("~ into Option<String>");
    assert_eq!(tilde.f, None);

    let empty_value: WithString = from_str_strict("f:\n").expect("empty value into Option<String>");
    assert_eq!(empty_value.f, None);
}

#[test]
fn lenient_and_strict_now_agree() {
    for src in [r#"f: """#, r#"f: "hello""#, "f: ~", "f:\n", "f: null"] {
        let lenient: WithString = noyalib::from_str(src).expect(src);
        let strict: WithString = from_str_strict(src).expect(src);
        assert_eq!(lenient, strict, "lenient and strict disagreed on {src:?}");
    }
}
