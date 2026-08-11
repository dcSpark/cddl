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
fn retries_after_a_group_choice_fails_closed_map_coverage() {
  let one_pair_json = r#"{"a":5}"#;
  let one_pair_cbor = "a1616105";
  let two_pair_json = r#"{"a":5,"b":1}"#;
  let two_pair_cbor = "a2616105616201";

  // RFC 8610 Sections 2.2.2 and 3.5.3: the first alternative does not
  // cover the two-entry map, so the wildcard alternative must be tried from
  // the original ownership state. Either alternative order accepts.
  both_validate(
    "m = { a: int // * any => any }",
    two_pair_json,
    two_pair_cbor,
  );
  both_validate(
    "m = { * any => any // a: int }",
    two_pair_json,
    two_pair_cbor,
  );

  // Positive controls for each alternative in isolation.
  both_validate("m = { a: int }", one_pair_json, one_pair_cbor);
  both_validate("m = { * any => any }", two_pair_json, two_pair_cbor);

  // A map remains closed when no alternative covers every pair.
  both_reject("m = { a: int // b: int }", two_pair_json, two_pair_cbor);
}

#[test]
fn retries_complete_pair_and_occurrence_alternatives_from_a_checkpoint() {
  // A06 establishes that a repeating table claims only complete key/value
  // matches. The uint arm therefore has width zero and fails only at final
  // map coverage; the tstr arm must then get an uncontaminated attempt.
  both_validate(
    "m = { (* tstr => uint // * tstr => tstr) }",
    r#"{"a":"x"}"#,
    "a161616178",
  );
  both_validate(
    "m = { (* tstr => tstr // * tstr => uint) }",
    r#"{"a":1}"#,
    "a1616101",
  );

  // The first arm greedily claims one complete pair and satisfies its own
  // 1*1 bound, but does not cover the map. The 2*2 arm can succeed only if
  // both that claim and its occurrence state are rolled back before retry.
  both_validate(
    "m = { (1*1 tstr => uint // 2*2 tstr => uint) }",
    r#"{"a":1,"b":2}"#,
    "a2616101616202",
  );

  // If every alternative misses a complete pair, closed coverage still
  // rejects rather than treating a zero-width `*` as a successful map.
  both_reject(
    "m = { (* tstr => uint // * tstr => bool) }",
    r#"{"a":"x"}"#,
    "a161616178",
  );
}

#[test]
fn retries_inline_and_named_group_choices_with_surrounding_entries() {
  // Parentheses do not change group-choice union semantics (RFC 8610
  // Appendix C). The first nested arm and the following `c` member leave one
  // pair uncovered; the second nested arm plus `c` covers the whole map.
  both_validate(
    "m = { (1*1 tstr => uint // 2*2 tstr => uint), c: uint }",
    r#"{"a":1,"b":2,"c":3}"#,
    "a3616101616202616303",
  );

  // A named group is semantically equivalent to its parenthesized group.
  // Coverage failure must retry choices from the rule definition as part of
  // the enclosing map candidate.
  both_validate(
    "g = (a: int // * any => any)\nm = { g }",
    r#"{"a":5,"b":1}"#,
    "a2616105616201",
  );

  // Expanding a generic group choice retains the invocation that gives its
  // member types meaning while the complete map candidate is evaluated.
  both_validate(
    "g<T> = (a: T // * any => any)\nm = { g<int> }",
    r#"{"a":5,"b":1}"#,
    "a2616105616201",
  );
  both_validate(
    "g<T> = (a: T // b: bool)\nm = { g<int> }",
    r#"{"a":5}"#,
    "a1616105",
  );
  both_reject(
    "g<T> = (a: T // b: bool)\nm = { g<int> }",
    r#"{"a":"x"}"#,
    "a161616178",
  );
}

#[test]
fn group_choice_isolation_preserves_cut_scope_and_diagnostics() {
  let json = r#"{"a":"x","z":9}"#;
  let cbor = "a261616178617a09";

  // The colon shortcut's cut prevents a later entry in the same alternative
  // from rescuing a bad `a` value (RFC 8610 Section 3.5.4). It does not make
  // a failed alternative mutate the separate wildcard alternative.
  both_validate("m = { a: uint // * any => any }", json, cbor);
  both_reject("m = { a: uint, * any => any }", json, cbor);

  // When every isolated alternative fails, retain useful value-mismatch and
  // closed-map diagnostics instead of returning an empty error set.
  let json_error = validate_json_from_str("m = { a: uint // b: uint }", json, None).unwrap_err();
  assert!(!json_error.to_string().is_empty());

  let bytes = hex::decode(cbor).unwrap();
  let cbor_error =
    validate_cbor_from_slice("m = { a: uint // b: uint }", &bytes, None).unwrap_err();
  assert!(!cbor_error.to_string().is_empty());
}
