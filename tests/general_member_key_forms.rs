//! General map member-key regressions.
//!
//! RFC 8610 Sections 2.1.2 and 3.5 define a map member's key as a type and
//! require both that key type and the member's value type to match a pair.
//! The key type is not restricted to a direct primitive identifier: aliases,
//! choices, controls, tags, and composite types use the same Type1 grammar.

#![cfg(feature = "std")]
#![cfg(all(feature = "cbor", feature = "json"))]
#![cfg(not(feature = "lsp"))]
#![cfg(not(target_arch = "wasm32"))]

use cddl::{validate_cbor_from_slice, validate_json_from_str};
use ciborium::value::Value;

fn json_validates(schema: &str, instance: &str) -> bool {
  validate_json_from_str(schema, instance, None).is_ok()
}

fn encode(value: Value) -> Vec<u8> {
  let mut bytes = Vec::new();
  ciborium::ser::into_writer(&value, &mut bytes).unwrap();
  bytes
}

fn cbor_validates(schema: &str, value: Value) -> bool {
  validate_cbor_from_slice(schema, &encode(value), None).is_ok()
}

fn map(entries: Vec<(Value, Value)>) -> Value {
  Value::Map(entries)
}

fn text(value: &str) -> Value {
  Value::Text(value.into())
}

fn uint(value: u64) -> Value {
  Value::Integer(value.into())
}

#[test]
fn direct_member_keys_and_ordinary_value_validation_remain_controls() {
  assert!(json_validates("m = { * tstr => uint }", r#"{"a":1}"#));
  assert!(json_validates("m = { * any => uint }", r#"{"a":1}"#));
  assert!(!json_validates(
    "m = { * tstr => uint }",
    r#"{"a":"wrong"}"#,
  ));

  assert!(cbor_validates(
    "m = { * tstr => uint }",
    map(vec![(text("a"), uint(1))]),
  ));
  assert!(cbor_validates(
    "m = { * any => uint }",
    map(vec![(uint(1), uint(1))]),
  ));

  let tagged = Value::Tag(24, Box::new(uint(5)));
  assert!(cbor_validates("x = #6.24(uint)", tagged.clone()));
  assert!(!cbor_validates(
    "x = #6.24(uint)",
    Value::Tag(25, Box::new(uint(5))),
  ));
  assert!(!cbor_validates(
    "x = #6.24(uint)",
    Value::Tag(24, Box::new(text("wrong"))),
  ));

  // A direct primitive nested table already works on the exact base and
  // protects the value/key context boundary while general forms are added.
  assert!(cbor_validates(
    "m = { * uint => { * uint => tstr } }",
    map(vec![(uint(1), map(vec![(uint(2), text("a"))]))]),
  ));
}

#[test]
fn json_type_and_literal_choices_claim_matching_keys() {
  for schema in [
    "m = { * (tstr / int) => uint }",
    r#"m = { * ("a" / "b") => uint }"#,
    r#"m = { fixed: bool, * ("a" / "b") => uint }"#,
    r#"m = { * ("a" / "b") => uint, fixed: bool }"#,
  ] {
    let instance = if schema.contains("fixed") {
      r#"{"fixed":true,"a":1}"#
    } else {
      r#"{"a":1}"#
    };
    assert!(
      json_validates(schema, instance),
      "choice member key rejected {} against {}",
      schema,
      instance,
    );
  }

  assert!(!json_validates(
    r#"m = { * ("a" / "b") => uint }"#,
    r#"{"c":1}"#,
  ));
  assert!(!json_validates(
    r#"m = { * ("a" / "b") => uint }"#,
    r#"{"a":"wrong"}"#,
  ));
}

#[test]
fn aliases_claim_json_keys_through_their_complete_type() {
  for schema in [
    "m = { * key => uint }\nkey = tstr",
    "m = { * key => uint }\nkey = any",
    r#"m = { * key => uint }
key = "a" / "b""#,
    r#"m = { * key => uint }
key = "a""#,
  ] {
    assert!(
      json_validates(schema, r#"{"a":1}"#),
      "aliased member key rejected schema {}",
      schema,
    );
  }

  assert!(!json_validates(
    r#"m = { * key => uint }
key = "a" / "b""#,
    r#"{"c":1}"#,
  ));
}

#[test]
fn controls_validate_json_and_cbor_keys() {
  let json_size = "m = { * (tstr .size 1) => uint }";
  assert!(json_validates(json_size, r#"{"a":1}"#));
  assert!(!json_validates(json_size, r#"{"ab":1}"#));

  let json_and = "m = { * (any .and tstr) => uint }";
  assert!(json_validates(json_and, r#"{"a":1}"#));

  let cbor_size = "m = { * (tstr .size 1) => uint }";
  assert!(cbor_validates(cbor_size, map(vec![(text("a"), uint(1))])));
  assert!(!cbor_validates(cbor_size, map(vec![(text("ab"), uint(1))])));

  let cbor_and = "m = { * (any .and uint) => tstr }";
  assert!(cbor_validates(cbor_and, map(vec![(uint(1), text("x"))])));
  assert!(!cbor_validates(cbor_and, map(vec![(text("a"), text("x"))])));
}

#[test]
fn cbor_aliases_and_choices_preserve_primitive_value_domains() {
  let choice = "m = { * (tstr / int) => uint }";
  assert!(cbor_validates(choice, map(vec![(text("a"), uint(1))])));
  assert!(cbor_validates(choice, map(vec![(uint(1), uint(1))])));
  assert!(!cbor_validates(
    choice,
    map(vec![(Value::Bool(true), uint(1))]),
  ));
  assert!(!cbor_validates(choice, map(vec![(uint(1), text("wrong"))])));

  for schema in [
    "m = { * key => uint }\nkey = tstr",
    "m = { * key => uint }\nkey = any",
    "m = { * key => uint }\nkey = 0 / 1 / 2",
    "m = { * key => uint }\nkey = 0",
  ] {
    let key = if schema.contains("tstr") {
      text("a")
    } else {
      uint(0)
    };
    assert!(
      cbor_validates(schema, map(vec![(key, uint(1))])),
      "aliased member key rejected schema {}",
      schema,
    );
  }

  let bytes_alias = "m = { * key => uint }\nkey = h'AA'";
  assert!(cbor_validates(
    bytes_alias,
    map(vec![(Value::Bytes(vec![0xaa]), uint(1))]),
  ));
  assert!(!cbor_validates(
    bytes_alias,
    map(vec![(Value::Bytes(vec![0xbb]), uint(1))]),
  ));

  // S03's integer sign domains and S01's float precision bounds remain
  // authoritative after resolving the alias in member-key position.
  let unsigned = "m = { * key => uint }\nkey = uint";
  assert!(cbor_validates(unsigned, map(vec![(uint(0), uint(1))])));
  assert!(!cbor_validates(
    unsigned,
    map(vec![(Value::Integer((-1).into()), uint(1))]),
  ));

  let float16 = "m = { * key => uint }\nkey = float16";
  assert!(cbor_validates(
    float16,
    map(vec![(Value::Float(1.5), uint(1))]),
  ));
  assert!(!cbor_validates(
    float16,
    map(vec![(Value::Float(1.1), uint(1))]),
  ));
}

#[test]
fn tagged_member_keys_reuse_strict_tag_and_payload_validation() {
  let tagged = "m = { * #6.24(uint) => tstr }";
  let valid_key = Value::Tag(24, Box::new(uint(5)));
  assert!(cbor_validates(
    tagged,
    map(vec![(valid_key.clone(), text("x"))]),
  ));
  assert!(cbor_validates(
    "m = { #6.24(uint) => tstr }",
    map(vec![(valid_key.clone(), text("x"))]),
  ));

  let siblings = "m = { fixed: bool, * #6.24(uint) => tstr }";
  for entries in [
    vec![
      (text("fixed"), Value::Bool(true)),
      (valid_key.clone(), text("x")),
    ],
    vec![
      (valid_key.clone(), text("x")),
      (text("fixed"), Value::Bool(true)),
    ],
  ] {
    assert!(cbor_validates(siblings, map(entries)));
  }

  assert!(!cbor_validates(
    tagged,
    map(vec![(Value::Tag(25, Box::new(uint(5))), text("x"))]),
  ));
  assert!(!cbor_validates(
    tagged,
    map(vec![(Value::Tag(24, Box::new(text("bad"))), text("x"))]),
  ));

  // I05's standard-prelude tagged unions must keep exact tag/payload
  // membership when the same ordinary-value path is used for a key.
  let unsigned = "m = { * unsigned => tstr }";
  assert!(cbor_validates(
    unsigned,
    map(vec![(
      Value::Tag(2, Box::new(Value::Bytes(vec![1]))),
      text("x"),
    )]),
  ));
  assert!(!cbor_validates(
    unsigned,
    map(vec![(
      Value::Tag(3, Box::new(Value::Bytes(vec![1]))),
      text("x"),
    )]),
  ));
  assert!(!cbor_validates(
    unsigned,
    map(vec![(Value::Tag(2, Box::new(uint(1))), text("x"))]),
  ));
}

#[test]
fn repeating_composite_keys_claim_every_complete_pair() {
  let schema = "m = { * [+ uint] => uint }";
  let first_key = Value::Array(vec![uint(5)]);
  let second_key = Value::Array(vec![uint(6), uint(7)]);

  assert!(cbor_validates(
    schema,
    map(vec![(first_key.clone(), uint(5))]),
  ));
  assert!(cbor_validates(
    schema,
    map(vec![(first_key, uint(5)), (second_key.clone(), uint(8))]),
  ));
  assert!(!cbor_validates(
    schema,
    map(vec![(second_key, text("wrong"))]),
  ));
}

#[test]
fn general_keys_retain_occurrence_bounds_and_complete_pair_claims() {
  let json_schema = r#"m = {
    1*1 key => uint,
    * key => tstr
  }
  key = "a" / "b""#;
  assert!(json_validates(json_schema, r#"{"a":"x","b":1}"#));
  assert!(!json_validates(
    "m = { + key => uint }\nkey = tstr",
    r#"{}"#,
  ));

  let cbor_schema = "m = { 1*1 key => uint, * key => tstr }\nkey = 0 / 1";
  assert!(cbor_validates(
    cbor_schema,
    map(vec![(uint(0), text("x")), (uint(1), uint(1))]),
  ));
  assert!(!cbor_validates(
    "m = { + key => uint }\nkey = tstr / int",
    map(vec![]),
  ));
}

#[test]
fn incremental_type_choices_are_transactional_in_member_key_context() {
  let schema = "m = { * key => tstr }\n\
    key = bool\n\
    key /= tstr\n\
    key /= uint";
  assert!(cbor_validates(
    schema,
    map(vec![
      (Value::Bool(true), text("b")),
      (text("key"), text("s")),
      (uint(1), text("u")),
    ]),
  ));
  assert!(json_validates(schema, r#"{"key":"s"}"#));

  let alternate_only = "m = { * key => tstr }\nkey /= tstr\nkey /= uint";
  assert!(cbor_validates(
    alternate_only,
    map(vec![(uint(1), text("u"))]),
  ));
}
