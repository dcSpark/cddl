//! Incremental type-choice resolution regressions.
//!
//! RFC 8610 §2.2.2 makes every `/=` right-hand side an additional arm of
//! the named type choice. Appendix C requires those arms to be populated in
//! source order and says that a later plain `=` is an error once `/=` has
//! already created the named choice.

#![cfg(feature = "std")]
#![cfg(all(feature = "cbor", feature = "json"))]
#![cfg(not(target_arch = "wasm32"))]

use cddl::{cddl_from_str, validate_cbor_from_slice, validate_json_from_str};

const BASE_FIRST_ROOT: &str = r#"
extended = bool
extended /= text
extended /= uint
"#;

const BASE_FIRST_ALIAS: &str = r#"
root = extended
extended = bool
extended /= text
extended /= uint
"#;

const ALTERNATE_ONLY_ROOT: &str = r#"
extended /= text
extended /= uint
"#;

const ALTERNATE_ONLY_ALIAS: &str = r#"
root = extended
extended /= text
extended /= uint
"#;

fn assert_json_three_arm_choice(schema: &str) {
  // Accept the base after two failed additions, and accept either addition.
  validate_json_from_str(schema, "true", None).unwrap();
  validate_json_from_str(schema, r#""x""#, None).unwrap();
  validate_json_from_str(schema, "0", None).unwrap();

  // Reject a value outside every arm.
  validate_json_from_str(schema, "null", None).unwrap_err();
}

fn assert_cbor_three_arm_choice(schema: &str) {
  // Accept the base after two failed additions, and accept either addition.
  validate_cbor_from_slice(schema, &[0xf5], None).unwrap();
  validate_cbor_from_slice(schema, &[0x61, b'x'], None).unwrap();
  validate_cbor_from_slice(schema, &[0x00], None).unwrap();

  // Reject a value outside every arm.
  validate_cbor_from_slice(schema, &[0xf6], None).unwrap_err();
}

#[test]
fn json_incremental_choice_is_root_independent_and_transactional() {
  assert_json_three_arm_choice(BASE_FIRST_ROOT);
  assert_json_three_arm_choice(BASE_FIRST_ALIAS);
}

#[test]
fn cbor_incremental_choice_is_root_independent_and_transactional() {
  assert_cbor_three_arm_choice(BASE_FIRST_ROOT);
  assert_cbor_three_arm_choice(BASE_FIRST_ALIAS);
}

#[test]
fn alternate_only_choice_remains_valid_at_root_and_through_alias() {
  for schema in [ALTERNATE_ONLY_ROOT, ALTERNATE_ONLY_ALIAS] {
    validate_json_from_str(schema, r#""x""#, None).unwrap();
    validate_json_from_str(schema, "0", None).unwrap();
    validate_json_from_str(schema, "true", None).unwrap_err();

    validate_cbor_from_slice(schema, &[0x61, b'x'], None).unwrap();
    validate_cbor_from_slice(schema, &[0x00], None).unwrap();
    validate_cbor_from_slice(schema, &[0xf5], None).unwrap_err();
  }
}

#[test]
fn base_definition_cannot_follow_an_incremental_definition() {
  for schema in [
    "extended /= text\nextended = bool\n",
    "root = extended\nextended /= text\nextended = bool\n",
  ] {
    let error = cddl_from_str(schema, false).unwrap_err();
    assert!(
      error.contains("already defined"),
      "unexpected parser error: {}",
      error
    );
  }
}
