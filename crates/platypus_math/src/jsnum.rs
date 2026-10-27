// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! JavaScript number semantics the math types need: `Math.min` / `Math.max` / `Math.sign` with
//! their NaN and signed-zero rules, `Math.hypot`, `ToInt32`, `Math.round`, and `String(number)`.
//! Results equal what the original TypeScript computed, bit for bit.

/// `Math.max(a, b)`: NaN wins, and +0 is above -0.
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() { b } else { a }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `Math.min(a, b)`: NaN wins, and -0 is below +0.
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.sign`: NaN and the zeros stay as they are.
pub fn sign(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else if x > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// `Math.round`: ties go up; the sign of a zero result follows the input.
pub fn round(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let r = x.floor();
    let v = if x - r >= 0.5 { r + 1.0 } else { r };
    if v == 0.0 && x < 0.0 { -0.0 } else { v }
}

/// `Math.clamp`-style `Math.min(hi, Math.max(lo, v))`, as the TypeScript wrote it.
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    min(hi, max(lo, v))
}

pub fn clamp01(v: f64) -> f64 {
    clamp(v, 0.0, 1.0)
}

/// `Math.hypot(..)`. Two arguments are the C library's `hypot` (as the reference engine does);
/// more are scaled by the largest and summed (the engines differ in the last bit here: the
/// reference engine's result matches this in nearly every case, and the tests that compare with
/// it replace `Math.hypot` by the same algorithm).
pub fn hypot(args: &[f64]) -> f64 {
    let mut any_nan = false;
    let mut biggest = 0.0f64;
    for a in args {
        if a.is_infinite() {
            return f64::INFINITY;
        }
        if a.is_nan() {
            any_nan = true;
        } else {
            biggest = biggest.max(a.abs());
        }
    }
    if any_nan {
        return f64::NAN;
    }
    if biggest == 0.0 {
        return 0.0;
    }
    if let [a, b] = args {
        return a.hypot(*b);
    }
    let mut sum = 0.0;
    for a in args {
        let n = a / biggest;
        sum += n * n;
    }
    f64::sqrt(sum) * biggest
}

/// `ToInt32`.
pub fn to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    (x.trunc().rem_euclid(4_294_967_296.0)) as u32 as i32
}

/// `String(x)`: the shortest digits that read back as `x`, with JavaScript's choice of fixed
/// or exponent form.
pub fn to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let (neg, x) = (x < 0.0, x.abs());
    // `{:e}` gives the shortest round-trip digits: "1.2345e-7".
    let sci = format!("{x:e}");
    let (mantissa, exp) = sci.split_once('e').expect("exponent form");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let n = digits.len() as i32;
    let e: i32 = exp.parse().expect("exponent");
    // The value is 0.<digits> * 10^k with k = e + 1.
    let k = e + 1;
    let body = if (n..=21).contains(&k) {
        format!("{digits}{}", "0".repeat((k - n) as usize))
    } else if 0 < k && k <= 21 {
        format!("{}.{}", &digits[..k as usize], &digits[k as usize..])
    } else if -6 < k && k <= 0 {
        format!("0.{}{digits}", "0".repeat((-k) as usize))
    } else {
        let sign = if k - 1 < 0 { '-' } else { '+' };
        let e = (k - 1).abs();
        if n == 1 {
            format!("{digits}e{sign}{e}")
        } else {
            format!("{}.{}e{sign}{e}", &digits[..1], &digits[1..])
        }
    };
    if neg { format!("-{body}") } else { body }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_as_javascript_prints_them() {
        for (x, s) in [
            (1.0, "1"),
            (-0.5, "-0.5"),
            (1e21, "1e+21"),
            (1.5e-7, "1.5e-7"),
            (123456789012345680000.0, "123456789012345680000"),
            (0.000001, "0.000001"),
            (1e-7, "1e-7"),
            (100.0, "100"),
            (2.5, "2.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (f64::MAX, "1.7976931348623157e+308"),
            (5e-324, "5e-324"),
        ] {
            assert_eq!(to_string(x), s);
        }
    }
}
