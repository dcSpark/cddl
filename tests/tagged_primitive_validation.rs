#![cfg(feature = "std")]
#![cfg(feature = "cbor")]
#![cfg(not(target_arch = "wasm32"))]

use cddl::validate_cbor_from_slice;

fn assert_valid(name: &str, cddl: &str, cbor: &[u8]) {
  if let Err(error) = validate_cbor_from_slice(cddl, cbor, None) {
    panic!("{}: expected valid, got error: {:?}", name, error);
  }
}

fn assert_invalid(name: &str, cddl: &str, cbor: &[u8]) {
  assert!(
    validate_cbor_from_slice(cddl, cbor, None).is_err(),
    "{}: expected invalid, but it validated",
    name
  );
}

#[test]
fn unrelated_primitive_identifiers_reject_tagged_values() {
  assert_invalid("tstr", "x = tstr", &[0xc2, 0x41, 0x01]);
  assert_invalid("bool", "x = bool", &[0xc2, 0x01]);
  assert_invalid("uint", "x = uint", &[0xc2, 0x41, 0x01]);
  assert_invalid("number", "x = number", &[0xc1, 0x01]);

  // `any` includes every well-formed CBOR data item, including tagged data.
  assert_valid("any", "x = any", &[0xd8, 0x2a, 0x61, 0x78]);
}

#[test]
fn time_prelude_types_require_their_exact_tag_and_payload() {
  assert_valid(
    "tdate",
    "x = tdate",
    &[
      0xc0, 0x74, b'2', b'0', b'2', b'6', b'-', b'0', b'8', b'-', b'1', b'1', b'T', b'0', b'0',
      b':', b'0', b'0', b':', b'0', b'0', b'Z',
    ],
  );
  assert_invalid("tdate wrong tag", "x = tdate", &[0xc1, 0x61, b'x']);
  assert_invalid("tdate wrong payload", "x = tdate", &[0xc0, 0x01]);

  assert_valid("time integer", "x = time", &[0xc1, 0x01]);
  assert_valid("time float", "x = time", &[0xc1, 0xf9, 0x3e, 0x00]);
  assert_invalid("time wrong tag", "x = time", &[0xc0, 0x01]);
  assert_invalid("time wrong payload", "x = time", &[0xc1, 0x61, b'x']);
}

#[test]
fn bignum_prelude_unions_validate_tag_and_payload_membership() {
  assert_valid("biguint", "x = biguint", &[0xc2, 0x41, 0x01]);
  assert_invalid("biguint wrong tag", "x = biguint", &[0xc3, 0x41, 0x01]);
  assert_invalid("biguint wrong payload", "x = biguint", &[0xc2, 0x01]);

  assert_valid("unsigned uint arm", "x = unsigned", &[0x01]);
  assert_valid("unsigned biguint arm", "x = unsigned", &[0xc2, 0x41, 0x01]);
  assert_invalid("unsigned bignint", "x = unsigned", &[0xc3, 0x41, 0x01]);
  assert_invalid("unsigned malformed biguint", "x = unsigned", &[0xc2, 0x01]);

  assert_valid("integer int arm", "x = integer", &[0x20]);
  assert_valid("integer biguint arm", "x = integer", &[0xc2, 0x41, 0x01]);
  assert_valid("integer bignint arm", "x = integer", &[0xc3, 0x41, 0x01]);
  assert_invalid("integer unrelated tag", "x = integer", &[0xc4, 0x41, 0x01]);
  assert_invalid("integer malformed bignum", "x = integer", &[0xc3, 0x01]);

  assert_valid(
    "alias to unsigned",
    "x = magnitude\nmagnitude = unsigned",
    &[0xc2, 0x41, 0x01],
  );
  assert_invalid(
    "alias to unsigned rejects bignint",
    "x = magnitude\nmagnitude = unsigned",
    &[0xc3, 0x41, 0x01],
  );
}

#[test]
fn other_tagged_prelude_types_validate_structural_membership() {
  let valid_cases: &[(&str, &[u8])] = &[
    ("eb64url", &[0xd5, 0x01]),
    ("eb64legacy", &[0xd6, 0x01]),
    ("eb16", &[0xd7, 0x01]),
    ("encoded-cbor", &[0xd8, 0x18, 0x41, 0x01]),
    ("uri", &[0xd8, 0x20, 0x61, b'x']),
    ("b64url", &[0xd8, 0x21, 0x61, b'x']),
    ("b64legacy", &[0xd8, 0x22, 0x61, b'x']),
    ("regexp", &[0xd8, 0x23, 0x61, b'x']),
    ("mime-message", &[0xd8, 0x24, 0x61, b'x']),
    ("cbor-any", &[0xd9, 0xd9, 0xf7, 0x01]),
  ];

  for (prelude_type, cbor) in valid_cases {
    assert_valid(prelude_type, &format!("x = {}", prelude_type), cbor);
  }

  assert_invalid(
    "encoded-cbor wrong tag",
    "x = encoded-cbor",
    &[0xd8, 0x19, 0x41, 0x01],
  );
  assert_invalid(
    "encoded-cbor wrong payload",
    "x = encoded-cbor",
    &[0xd8, 0x18, 0x01],
  );
  assert_invalid("uri wrong tag", "x = uri", &[0xd8, 0x21, 0x61, b'x']);
  assert_invalid("uri wrong payload", "x = uri", &[0xd8, 0x20, 0x01]);
}

#[test]
fn explicit_tag_choices_remain_transactional() {
  let cddl = "x = tstr / #6.42(uint)";

  assert_valid("tagged choice", cddl, &[0xd8, 0x2a, 0x01]);
  assert_invalid(
    "tagged choice wrong payload",
    cddl,
    &[0xd8, 0x2a, 0x61, b'x'],
  );
  assert_invalid("tagged choice wrong tag", cddl, &[0xd8, 0x2b, 0x01]);
  assert_valid("untagged choice", cddl, &[0x61, b'x']);
}
