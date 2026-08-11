//! JSON generic-group object ownership regressions.
//!
//! RFC 8610 Section 3.2 and normative Appendix C make a named group part of
//! the enclosing map assignment. Expanding a generic group must therefore use
//! the parent's already-consumed string keys and commit new keys only if the
//! complete child group succeeds.

#![cfg(feature = "std")]
#![cfg(feature = "json")]
#![cfg(not(feature = "lsp"))]
#![cfg(not(target_arch = "wasm32"))]

use cddl::validate_json_from_str;

#[cfg(feature = "additional-controls")]
fn validates(schema: &str, instance: &str) -> bool {
  validate_json_from_str(schema, instance, None).is_ok()
}

#[cfg(not(feature = "additional-controls"))]
fn validates(schema: &str, instance: &str) -> bool {
  validate_json_from_str(schema, instance).is_ok()
}

#[test]
fn sibling_generic_groups_claim_distinct_object_keys() {
  let identical_optionals = "m = { g<tstr, any>, g<tstr, any> }\ng<K, V> = (? K => V)";
  assert!(validates(identical_optionals, r#"{"a":true,"b":false}"#,));

  // Each invocation retains its own argument binding while consuming the
  // next unclaimed key. Exercise both sibling orders.
  assert!(validates(
    "m = { g<tstr, int>, g<tstr, bool> }\ng<K, V> = (? K => V)",
    r#"{"a":1,"b":true}"#,
  ));
  assert!(validates(
    "m = { g<tstr, bool>, g<tstr, int> }\ng<K, V> = (? K => V)",
    r#"{"a":true,"b":1}"#,
  ));
}

#[test]
fn sibling_generic_groups_do_not_blur_value_arguments() {
  assert!(!validates(
    "m = { g<tstr, any>, g<tstr, int> }\ng<K, V> = (? K => V)",
    r#"{"a":true,"b":false}"#,
  ));
}

#[test]
fn generic_repetition_observes_parent_claims() {
  let plus = "m = { ? tstr => tstr, g<tstr, tstr> }\ng<K, V> = (+ K => V)";
  let star = "m = { ? tstr => tstr, g<tstr, tstr> }\ng<K, V> = (* K => V)";

  // The parent owns the only pair, so the child has width zero.
  assert!(!validates(plus, r#"{"a":"x"}"#));
  assert!(validates(star, r#"{"a":"x"}"#));
}

#[test]
fn failed_generic_child_rolls_back_before_group_choice_fallback() {
  let schema = "m = { g<tstr, int> // g<tstr, bool> }\ng<K, V> = (K => V)";

  // The first child tentatively selects `a` but rejects its Boolean value.
  // The successful alternative must still be able to own the same key.
  assert!(validates(schema, r#"{"a":true}"#));
}

#[test]
fn successful_generic_child_keeps_closed_map_coverage_strict() {
  let schema = "m = { g<tstr, any> }\ng<K, V> = (? K => V)";

  assert!(validates(schema, r#"{"a":true}"#));
  assert!(!validates(schema, r#"{"a":true,"b":false}"#));
}
