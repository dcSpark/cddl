#![cfg(feature = "std")]
#![cfg(feature = "json")]
#![cfg(feature = "additional-controls")]
#![cfg(not(feature = "lsp"))]
#![cfg(not(target_arch = "wasm32"))]

use cddl::validate_json_from_str;

fn validates(schema: &str, instance: &str) -> bool {
  validate_json_from_str(schema, instance, None).is_ok()
}

#[test]
fn direct_literal_without_cut_falls_through_after_value_mismatch() {
  let instance = r#"{"optional-key":"nonsense"}"#;

  // RFC 8610 Section 3.5.4's exact extensible-map shape: the optional
  // non-cut entry does not match the complete pair, so the wildcard can.
  assert!(validates(
    r#"m = { ? "optional-key" => int, * tstr => any }"#,
    instance,
  ));

  // `any` has the same extension-key behavior as `tstr` for JSON objects.
  assert!(validates(
    r#"m = { ? "optional-key" => int, * any => any }"#,
    instance,
  ));
}

#[test]
fn direct_type_domain_without_cut_falls_through_after_value_mismatch() {
  assert!(validates(
    r#"m = { ? tstr => uint, tstr => tstr }"#,
    r#"{"a":"x"}"#,
  ));
}

#[test]
fn cut_direct_members_retain_value_mismatch() {
  let instance = r#"{"optional-key":"nonsense"}"#;

  // An explicit cut commits as soon as the key matches.
  assert!(!validates(
    r#"m = { ? "optional-key" ^ => int, * any => any }"#,
    instance,
  ));
  assert!(!validates(
    r#"m = { ? tstr ^ => int, * any => any }"#,
    instance,
  ));

  // The colon shortcut carries the same cut semantics.
  assert!(!validates(
    r#"m = { ? "optional-key": int, * any => any }"#,
    instance,
  ));
}

#[test]
fn non_cut_fallthrough_still_requires_a_compatible_later_member() {
  assert!(!validates(
    r#"m = { ? "optional-key" => int, * tstr => uint }"#,
    r#"{"optional-key":"nonsense"}"#,
  ));

  // A required entry cannot take the optional entry's zero-width path.
  assert!(!validates(
    r#"m = { "optional-key" => int, * tstr => any }"#,
    r#"{"optional-key":"nonsense"}"#,
  ));
}

#[test]
fn successful_non_cut_direct_member_keeps_greedy_ownership() {
  assert!(!validates(
    r#"m = { ? "a" => uint, a: uint }"#,
    r#"{"a":1}"#,
  ));
}
