#![cfg(feature = "std")]
#![cfg(feature = "cbor")]
#![cfg(not(feature = "lsp"))]
#![cfg(not(target_arch = "wasm32"))]

use cddl::validate_cbor_from_slice;

#[test]
fn nested_generic_claims_replay_their_complete_instantiation_context() {
  let direct_schema = "inner<K, V> = (? K => V)\n\
    m = { inner<tstr, any>, tstr => tstr }";
  let concrete_nested_schema = "inner<K, V> = (? K => V)\n\
    outer<X, Y> = (inner<tstr, any>)\n\
    m = { outer<tstr, any>, tstr => tstr }";
  let symbolic_nested_schema = "inner<K, V> = (? K => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { outer<tstr, any>, tstr => tstr }";
  let three_level_shadowed_schema = "inner<X, V> = (? X => V)\n\
    middle<X, Y> = (inner<X, Y>)\n\
    outer<X, Y> = (middle<X, Y>)\n\
    m = { outer<tstr, any>, tstr => tstr }";
  let text_then_integer = b"\xa2\x61a\x61x\x61b\x01"; // {"a": "x", "b": 1}

  // RFC 8610 Appendix C admits an ordering in which the required member owns
  // "a" and the optional generic member owns "b". Adding an enclosing
  // generic invocation must not change that pair compatibility.
  validate_cbor_from_slice(direct_schema, text_then_integer, None).unwrap();
  validate_cbor_from_slice(concrete_nested_schema, text_then_integer, None).unwrap();
  validate_cbor_from_slice(symbolic_nested_schema, text_then_integer, None).unwrap();
  validate_cbor_from_slice(three_level_shadowed_schema, text_then_integer, None).unwrap();

  let nested_control_schema = "inner<K, V> = (? K .size 1 => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { outer<tstr, any>, tstr => tstr }";
  validate_cbor_from_slice(nested_control_schema, text_then_integer, None).unwrap();
}

#[test]
fn nested_generic_replay_keeps_distinct_argument_sets() {
  let schema = "inner<K, V> = (? K => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { outer<tstr, any>, outer<tstr, bool>, tstr => tstr }";
  let bool_integer_text = b"\xa3\x61a\xf5\x61b\x01\x61c\x61x";

  // The second nested claim initially selects the integer-valued pair. A
  // valid replay moves the first (`any`) claim there and gives the Boolean
  // pair to the second invocation, without conflating their arguments.
  validate_cbor_from_slice(schema, bool_integer_text, None).unwrap();

  let rejecting_schema = "inner<K, V> = (? K => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { outer<tstr, any>, outer<tstr, int> }";
  let two_booleans = b"\xa2\x61a\xf5\x61b\xf4";
  validate_cbor_from_slice(rejecting_schema, two_booleans, None).unwrap_err();
}

#[test]
fn nested_generic_replay_preserves_physical_key_identity() {
  let nan_schema = "inner<K, V> = (? K => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { outer<float, any>, float => tstr }";
  let nan_text_then_nan_integer = b"\xa2\xf9\x7e\x00\x61x\xf9\x7e\x01\x01";
  validate_cbor_from_slice(nan_schema, nan_text_then_nan_integer, None).unwrap();

  let composite_schema = "inner<K, V> = (? K => V)\n\
    outer<X, Y> = (inner<[X], Y>)\n\
    m = { outer<float, any>, [float] => tstr }";
  let array_nan_text_then_array_nan_integer = b"\xa2\x81\xf9\x7e\x00\x61x\x81\xf9\x7e\x01\x01";
  validate_cbor_from_slice(
    composite_schema,
    array_nan_text_then_array_nan_integer,
    None,
  )
  .unwrap();
}

#[test]
fn nested_generic_parent_ownership_and_failed_replay_are_transactional() {
  let one_text_pair = b"\xa1\x61a\x01";
  let zero_or_more = "inner<K, V> = (* K => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { ? tstr => any, outer<tstr, any> }";
  let one_or_more = "inner<K, V> = (+ K => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { ? tstr => any, outer<tstr, any> }";

  validate_cbor_from_slice(zero_or_more, one_text_pair, None).unwrap();
  validate_cbor_from_slice(one_or_more, one_text_pair, None).unwrap_err();

  let rollback_schema = "inner<K, V> = (? K => V)\n\
    outer<X, Y> = (inner<X, Y>)\n\
    m = { (outer<tstr, tstr>, tstr => bytes) // * any => any }";
  let text_then_integer = b"\xa2\x61a\x61x\x61b\x01";
  validate_cbor_from_slice(rollback_schema, text_then_integer, None).unwrap();
}
