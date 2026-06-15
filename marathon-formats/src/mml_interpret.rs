//! MML interpretation layer.
//!
//! The parser in [`crate::mml`] is purely structural: it turns MML/XML bytes
//! into [`MmlElement`](crate::mml::MmlElement) trees without interpreting any
//! attribute values. This module reads those trees and produces typed override
//! structs for each recognized section.
//!
//! Attribute parsing follows AlephOne's lenient conventions: integers may be
//! decimal or hex (`0x` prefix), booleans accept `1`/`t`/`true` and
//! `0`/`f`/`false`, and a malformed value logs a warning and yields `None`
//! rather than failing the whole document — matching decades of community MML
//! written against AlephOne's forgiving parser.

/// Emit a non-fatal warning for a malformed attribute value.
///
/// `marathon-formats` has no `log`/`tracing` dependency, so warnings go to
/// stderr. Interpretation never fails on a bad value; it returns `None` and
/// lets the caller fall back to the engine default.
fn warn_malformed(kind: &str, raw: &str) {
    eprintln!("[mml] warning: malformed {kind} attribute value: {raw:?}");
}

/// Split a trimmed integer literal into `(radix, digits)`, honoring an optional
/// sign and an AlephOne-style `0x`/`0X` hex prefix. The returned `digits` string
/// is suitable for `from_str_radix` (sign preserved, prefix stripped).
fn normalize_int(s: &str) -> (u32, String) {
    let t = s.trim();
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => ("-", r),
        None => match t.strip_prefix('+') {
            Some(r) => ("", r),
            None => ("", t),
        },
    };
    match rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        Some(hex) => (16, format!("{sign}{hex}")),
        None => (10, t.to_string()),
    }
}

/// Parse an MML attribute as `i16` (decimal or `0x` hex). Returns `None` and
/// warns on a malformed or out-of-range value.
pub fn parse_mml_i16(s: &str) -> Option<i16> {
    let (radix, digits) = normalize_int(s);
    match i16::from_str_radix(&digits, radix) {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("i16", s);
            None
        }
    }
}

/// Parse an MML attribute as `i32` (decimal or `0x` hex). Returns `None` and
/// warns on a malformed or out-of-range value.
pub fn parse_mml_i32(s: &str) -> Option<i32> {
    let (radix, digits) = normalize_int(s);
    match i32::from_str_radix(&digits, radix) {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("i32", s);
            None
        }
    }
}

/// Parse an MML attribute as `u32` (decimal or `0x` hex). Negative values are
/// rejected. Returns `None` and warns on a malformed or out-of-range value.
pub fn parse_mml_u32(s: &str) -> Option<u32> {
    let (radix, digits) = normalize_int(s);
    match u32::from_str_radix(&digits, radix) {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("u32", s);
            None
        }
    }
}

/// Parse an MML attribute as `f32` (decimal). Returns `None` and warns on a
/// malformed value.
pub fn parse_mml_f32(s: &str) -> Option<f32> {
    match s.trim().parse::<f32>() {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("f32", s);
            None
        }
    }
}

/// Parse an MML attribute as `bool`. Accepts `1`/`t`/`true` (case-insensitive)
/// for true and `0`/`f`/`false` for false. Returns `None` and warns otherwise.
pub fn parse_mml_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "t" | "true" => Some(true),
        "0" | "f" | "false" => Some(false),
        _ => {
            warn_malformed("bool", s);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i16_decimal_and_hex() {
        assert_eq!(parse_mml_i16("100"), Some(100));
        assert_eq!(parse_mml_i16("-100"), Some(-100));
        assert_eq!(parse_mml_i16("  42 "), Some(42)); // whitespace tolerated
        assert_eq!(parse_mml_i16("0x10"), Some(16));
        assert_eq!(parse_mml_i16("0XFF"), Some(255));
        assert_eq!(parse_mml_i16("-0x10"), Some(-16));
    }

    #[test]
    fn i16_rejects_malformed_and_overflow() {
        assert_eq!(parse_mml_i16("abc"), None);
        assert_eq!(parse_mml_i16(""), None);
        assert_eq!(parse_mml_i16("70000"), None); // > i16::MAX
        assert_eq!(parse_mml_i16("0xZZ"), None);
    }

    #[test]
    fn i32_decimal_and_hex() {
        assert_eq!(parse_mml_i32("2147483647"), Some(i32::MAX));
        assert_eq!(parse_mml_i32("-5"), Some(-5));
        assert_eq!(parse_mml_i32("0x7FFFFFFF"), Some(i32::MAX));
        assert_eq!(parse_mml_i32("nope"), None);
    }

    #[test]
    fn u32_decimal_hex_and_sign_rejection() {
        assert_eq!(parse_mml_u32("0"), Some(0));
        assert_eq!(parse_mml_u32("4294967295"), Some(u32::MAX));
        assert_eq!(parse_mml_u32("0xDEADBEEF"), Some(0xDEAD_BEEF));
        assert_eq!(parse_mml_u32("-1"), None); // unsigned rejects negative
        assert_eq!(parse_mml_u32("-0x1"), None);
    }

    #[test]
    fn f32_decimal() {
        assert_eq!(parse_mml_f32("1.5"), Some(1.5));
        assert_eq!(parse_mml_f32("-0.25"), Some(-0.25));
        assert_eq!(parse_mml_f32("  3 "), Some(3.0));
        assert_eq!(parse_mml_f32("0x1"), None); // no hex floats
        assert_eq!(parse_mml_f32("bad"), None);
    }

    #[test]
    fn bool_accepts_alephone_forms() {
        for t in ["1", "t", "true", "TRUE", "True", " t "] {
            assert_eq!(parse_mml_bool(t), Some(true), "{t:?} should be true");
        }
        for f in ["0", "f", "false", "FALSE", "False", " f "] {
            assert_eq!(parse_mml_bool(f), Some(false), "{f:?} should be false");
        }
        for bad in ["2", "yes", "no", "", "tru"] {
            assert_eq!(parse_mml_bool(bad), None, "{bad:?} should be None");
        }
    }
}
