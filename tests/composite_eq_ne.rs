#![cfg(feature = "std")]
#![cfg(feature = "cbor")]
#![cfg(feature = "json")]
#![cfg(not(feature = "lsp"))]
#![cfg(not(target_arch = "wasm32"))]

use cddl::{validate_cbor_from_slice, validate_json_from_str};

fn json_validates(schema: &str, instance: &str) -> bool {
  validate_json_from_str(schema, instance, None).is_ok()
}

fn cbor_validates(schema: &str, hex: &str) -> bool {
  let bytes = hex::decode(hex).unwrap();
  validate_cbor_from_slice(schema, &bytes, None).is_ok()
}

fn both_validate(schema: &str, json: &str, cbor_hex: &str) {
  let json_result = json_validates(schema, json);
  let cbor_result = cbor_validates(schema, cbor_hex);
  assert!(
    json_result && cbor_result,
    "expected both formats to match {schema}; JSON {json}: {json_result}, CBOR {cbor_hex}: {cbor_result}",
    schema = schema,
    json = json,
    json_result = json_result,
    cbor_hex = cbor_hex,
    cbor_result = cbor_result,
  );
}

fn both_reject(schema: &str, json: &str, cbor_hex: &str) {
  let json_result = json_validates(schema, json);
  let cbor_result = cbor_validates(schema, cbor_hex);
  assert!(
    !json_result && !cbor_result,
    "expected both formats to reject {schema}; JSON {json}: {json_result}, CBOR {cbor_hex}: {cbor_result}",
    schema = schema,
    json = json,
    json_result = json_result,
    cbor_hex = cbor_hex,
    cbor_result = cbor_result,
  );
}

#[test]
fn array_eq_ne_apply_one_aggregate_predicate() {
  let ne = "x = [uint, uint] .ne [1, 2]";
  both_reject(ne, "[1,2]", "820102");
  both_validate(ne, "[1,3]", "820103");

  let eq = "x = [uint, uint] .eq [1, 2]";
  both_validate(eq, "[1,2]", "820102");
  both_reject(eq, "[1,3]", "820103");

  // A length difference proves aggregate inequality; it is not an error from
  // the controller's element-count check.
  both_validate("x = [* uint] .ne [1, 2]", "[1]", "8101");
  both_reject("x = [* uint] .eq [1, 2]", "[1]", "8101");
}

#[test]
fn map_eq_ne_are_order_independent_complete_pair_predicates() {
  let ne = r#"x = { tstr => uint } .ne { "a": 1 }"#;
  both_reject(ne, r#"{"a":1}"#, "a1616101");
  both_validate(ne, r#"{"a":2}"#, "a1616102");
  both_validate(ne, r#"{"b":1}"#, "a1616201");

  let eq = r#"x = { * tstr => uint } .eq { "a": 1, "b": 2 }"#;
  both_validate(eq, r#"{"b":2,"a":1}"#, "a2616202616101");
  both_reject(eq, r#"{"a":1}"#, "a1616101");

  // A pair-count difference likewise proves inequality as a whole.
  both_validate(
    r#"x = { * tstr => uint } .ne { "a": 1, "b": 2 }"#,
    r#"{"a":1}"#,
    "a1616101",
  );
}

#[test]
fn composite_controls_validate_the_left_hand_side_first() {
  both_reject(
    r#"x = { "required": uint } .eq { "other": 1 }"#,
    r#"{"other":1}"#,
    "a1656f7468657201",
  );
  both_reject(
    r#"x = { "required": uint } .ne { "other": 1 }"#,
    r#"{"other":2}"#,
    "a1656f7468657202",
  );
  both_reject("x = [uint, uint] .ne [1, 2]", r#"[1,"x"]"#, "82016178");
  both_reject("x = [uint] .ne [1]", r#"{"a":1}"#, "a1616101");
}

#[test]
fn nested_and_numeric_composite_equality_follows_rfc_8610() {
  let nested_eq = r#"x = [any] .eq [{ "a": [1, 2] }]"#;
  both_validate(nested_eq, r#"[{"a":[1,2]}]"#, "81a16161820102");
  both_reject(nested_eq, r#"[{"a":[1,3]}]"#, "81a16161820103");
  both_validate(
    r#"x = [any] .ne [{ "a": [1, 2] }]"#,
    r#"[{"a":[1,3]}]"#,
    "81a16161820103",
  );

  // Section 3.8.6 requires equal numeric values inside composites to remain
  // unequal when one is an integer and the other is floating point.
  both_reject("x = [number] .eq [1.0]", "[1]", "8101");
  both_validate("x = [number] .ne [1.0]", "[1]", "8101");
  both_validate("x = [number] .eq [1.0]", "[1.0]", "81f93c00");
  both_reject("x = [number] .ne [1.0]", "[1.0]", "81f93c00");

  both_reject(
    r#"x = { * tstr => number } .eq { "a": 1.0 }"#,
    r#"{"a":1}"#,
    "a1616101",
  );
  both_validate(
    r#"x = { * tstr => number } .ne { "a": 1.0 }"#,
    r#"{"a":1}"#,
    "a1616101",
  );
}

#[test]
fn cbor_composites_preserve_tags_and_major_types() {
  let tagged_eq = "x = [any] .eq [#6.42(1)]";
  assert!(cbor_validates(tagged_eq, "81d82a01"));
  assert!(!cbor_validates(tagged_eq, "81d82b01"));
  assert!(!cbor_validates(tagged_eq, "8101"));

  let tagged_ne = "x = [any] .ne [#6.42(1)]";
  assert!(!cbor_validates(tagged_ne, "81d82a01"));
  assert!(cbor_validates(tagged_ne, "81d82b01"));
  assert!(cbor_validates(tagged_ne, "8101"));

  // A text controller remains distinct from a different CBOR major type.
  assert!(!cbor_validates(r#"x = [any] .eq ["a"]"#, "8101"));
  assert!(cbor_validates(r#"x = [any] .ne ["a"]"#, "8101"));
}

#[test]
fn composite_default_is_aggregate_ne_in_an_optional_context() {
  let schema = "x = { ? value: ([uint, uint] .default [1, 2]) }";
  both_validate(schema, "{}", "a0");
  both_reject(schema, r#"{"value":[1,2]}"#, "a16576616c7565820102");
  both_validate(schema, r#"{"value":[1,3]}"#, "a16576616c7565820103");
  both_reject(schema, r#"{"value":[1,"x"]}"#, "a16576616c756582016178");
}
