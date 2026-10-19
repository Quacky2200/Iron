//! `BigInt`: an arbitrary-precision integer, a leaf GC object `[header][sign][byte count][magnitude
//! bytes, little endian]` (sign -1, 0 or 1). Every operation reads its operands into
//! `num_bigint::BigInt` values first and allocates the result last, so an unrooted temporary operand
//! is never read after a collection. Errors are `RangeError`s / `SyntaxError`s with
//! JavaScriptCore's text.

use super::*;
use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{FromPrimitive, One, Signed, ToPrimitive, Zero};

#[repr(C)]
pub struct BigObj {
    header: u64,
    sign: i64,
    nbytes: i64,
}

fn read(p: *const BigObj) -> BigInt {
    // SAFETY: `p` is a live BigInt object (callers pass what compiled code holds).
    unsafe {
        let o = &*p;
        let bytes = std::slice::from_raw_parts(
            (p as *const u8).add(std::mem::size_of::<BigObj>()),
            o.nbytes as usize,
        );
        let sign = match o.sign {
            s if s < 0 => Sign::Minus,
            0 => Sign::NoSign,
            _ => Sign::Plus,
        };
        BigInt::from_bytes_le(sign, bytes)
    }
}

fn alloc(v: &BigInt) -> *mut BigObj {
    let (sign, bytes) = v.to_bytes_le();
    let bytes = if v.is_zero() { Vec::new() } else { bytes };
    let size = std::mem::size_of::<BigObj>() + bytes.len();
    with_heap(|h| {
        maybe_collect(h);
        let p = new_obj(h, HDR_BIGINT, size) as *mut BigObj;
        // SAFETY: `size` bytes were allocated.
        unsafe {
            (*p).sign = match sign {
                Sign::Minus => -1,
                Sign::NoSign => 0,
                Sign::Plus => 1,
            };
            (*p).nbytes = bytes.len() as i64;
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (p as *mut u8).add(std::mem::size_of::<BigObj>()),
                bytes.len(),
            );
        }
        p
    })
}

fn too_big() -> *mut BigObj {
    raise(
        1.0,
        "Out of memory: BigInt generated from this operation is too big",
    );
    alloc(&BigInt::zero())
}

/// A literal or numeric string parsed as `BigInt("...")` does; None for a syntax error.
fn parse_text(text: &str) -> Option<BigInt> {
    let t = text.trim();
    if t.is_empty() {
        return Some(BigInt::zero());
    }
    let lower = t.to_ascii_lowercase();
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = lower.strip_prefix(prefix) {
            return if digits.is_empty() || digits.starts_with(['+', '-']) {
                None
            } else {
                BigInt::parse_bytes(digits.as_bytes(), radix)
            };
        }
    }
    let digits = lower.strip_prefix(['+', '-']).unwrap_or(&lower);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    t.parse::<BigInt>().ok()
}

/// The source literal `123n`, `0xFFn`, `1_000n`: the text without `n` and underscores.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_literal_gc(s: *const Str) -> *mut BigObj {
    // SAFETY: by contract.
    let text = unsafe { (*s).to_string_lossy().replace('_', "") };
    alloc(&parse_text(&text).unwrap_or_default())
}

/// `BigInt("text")`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_from_text_gc(s: *const Str) -> *mut BigObj {
    // SAFETY: by contract.
    let text = unsafe { (*s).to_string_lossy().to_string() };
    match parse_text(&text) {
        Some(v) => alloc(&v),
        None => {
            raise(3.0, "Failed to parse String to BigInt");
            alloc(&BigInt::zero())
        }
    }
}

/// `BigInt(number)`: the number must be an integer.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_bigint_from_f64_gc(x: f64) -> *mut BigObj {
    match BigInt::from_f64(x).filter(|_| x.is_finite() && x.fract() == 0.0) {
        Some(v) => alloc(&v),
        None => {
            raise(1.0, "Not an integer");
            alloc(&BigInt::zero())
        }
    }
}

/// `Number(big)`: the nearest double.
///
/// # Safety
/// `a` must be a live BigInt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_to_f64(a: *const BigObj) -> f64 {
    read(a).to_f64().unwrap_or(f64::INFINITY)
}

/// The operation codes of `ts_rt_bigint_op_gc`.
const ADD: f64 = 0.0;
const SUB: f64 = 1.0;
const MUL: f64 = 2.0;
const DIV: f64 = 3.0;
const REM: f64 = 4.0;
const POW: f64 = 5.0;
const AND: f64 = 6.0;
const OR: f64 = 7.0;
const XOR: f64 = 8.0;
const SHL: f64 = 9.0;
const SHR: f64 = 10.0;

/// `a op b` for the binary operators.
///
/// # Safety
/// Both must be live BigInts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_op_gc(
    op: f64,
    a: *const BigObj,
    b: *const BigObj,
) -> *mut BigObj {
    let (x, y) = (read(a), read(b));
    let r = match op {
        ADD => x + y,
        SUB => x - y,
        MUL => x * y,
        DIV | REM => {
            if y.is_zero() {
                raise(1.0, "0 is an invalid divisor value.");
                return alloc(&BigInt::zero());
            }
            // Truncating division, the remainder takes the sign of the dividend.
            if op == DIV { &x / &y } else { &x % &y }
        }
        POW => {
            if y.is_negative() {
                raise(1.0, "Negative exponent is not allowed");
                return alloc(&BigInt::zero());
            }
            if x.is_zero() || x.is_one() || y.is_zero() {
                return alloc(&if y.is_zero() { BigInt::one() } else { x });
            }
            if x == BigInt::from(-1) {
                return alloc(&if y.is_even() { BigInt::one() } else { x });
            }
            match y.to_u32() {
                Some(e) if (x.bits() as u128) * (e as u128) < (1u128 << 30) => {
                    num_traits::pow::pow(x, e as usize)
                }
                _ => return too_big(),
            }
        }
        AND => x & y,
        OR => x | y,
        XOR => x ^ y,
        SHL | SHR => {
            let left = (op == SHL) == !y.is_negative();
            let n = y.abs();
            match n.to_u64() {
                Some(n) if n < (1 << 30) => {
                    if left {
                        x << n as usize
                    } else {
                        // `>>` rounds toward negative infinity, as `BigInt`'s shift does.
                        x >> n as usize
                    }
                }
                _ if x.is_zero() => x,
                _ if left => return too_big(),
                _ => {
                    if x.is_negative() {
                        BigInt::from(-1)
                    } else {
                        BigInt::zero()
                    }
                }
            }
        }
        _ => BigInt::zero(),
    };
    alloc(&r)
}

/// `-a` (`negate`) and `~a`.
///
/// # Safety
/// `a` must be a live BigInt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_unary_gc(negate: bool, a: *const BigObj) -> *mut BigObj {
    let x = read(a);
    alloc(&if negate { -x } else { -x - BigInt::one() })
}

/// -1, 0 or 1.
///
/// # Safety
/// Both must be live BigInts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_cmp(a: *const BigObj, b: *const BigObj) -> f64 {
    match read(a).cmp(&read(b)) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    }
}

/// `big` against a number: -1, 0, 1, or NaN when the number is NaN.
///
/// # Safety
/// `a` must be a live BigInt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_cmp_f64(a: *const BigObj, x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x.is_infinite() {
        return if x > 0.0 { -1.0 } else { 1.0 };
    }
    let big = read(a);
    let whole = BigInt::from_f64(x.trunc()).unwrap_or_default();
    let ord = big.cmp(&whole).then_with(|| {
        // Equal to the integer part: the fraction decides.
        0.0f64
            .partial_cmp(&x.fract())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    match ord {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    }
}

/// -1, 0 or 1: the sign (a BigInt is false only at zero).
///
/// # Safety
/// `a` must be a live BigInt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_sign(a: *const BigObj) -> f64 {
    // SAFETY: by contract.
    unsafe { (*a).sign as f64 }
}

/// `big.toString(radix)`.
///
/// # Safety
/// `a` must be a live BigInt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_to_string_gc(a: *const BigObj, radix: f64) -> *mut Str {
    let r = if radix.is_nan() { 10.0 } else { radix.trunc() };
    let text = if (2.0..=36.0).contains(&r) {
        read(a).to_str_radix(r as u32)
    } else {
        raise(1.0, "toString() radix argument must be between 2 and 36");
        String::new()
    };
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(text.as_ptr(), text.len() as i64) }
}

/// `BigInt.asIntN(bits, big)` (`signed`) and `BigInt.asUintN`.
///
/// # Safety
/// `a` must be a live BigInt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_bigint_as_n_gc(
    signed: bool,
    bits: f64,
    a: *const BigObj,
) -> *mut BigObj {
    let x = read(a);
    if bits < 0.0 {
        raise(1.0, "number of bits cannot be negative");
        return alloc(&BigInt::zero());
    }
    if !(0.0..9007199254740992.0).contains(&bits) || bits.fract() != 0.0 {
        raise(1.0, "Invalid value: not (convertible to) a safe integer");
        return alloc(&BigInt::zero());
    }
    let bits = bits as u64;
    if bits == 0 {
        return alloc(&BigInt::zero());
    }
    if bits > (1 << 30) {
        return alloc(&x);
    }
    let modulus = BigInt::one() << bits as usize;
    let mut r = x.mod_floor(&modulus);
    if signed && r >= (BigInt::one() << (bits as usize - 1)) {
        r -= &modulus;
    }
    alloc(&r)
}
