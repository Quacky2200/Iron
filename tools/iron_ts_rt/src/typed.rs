//! Typed arrays (`Int8Array` .. `Float64Array`). An array object with the `HDR_BYTES` header: `len`
//! is the element count, `cap` the bytes allocated, `data` the elements, raw and little endian. The
//! compiler knows the element kind statically and passes its code (the index in `TypedKind::ALL`):
//! 0 `Int8`, 1 `Uint8`, 2 `Uint8Clamped`, 3 `Int16`, 4 `Uint16`, 5 `Int32`, 6 `Uint32`,
//! 7 `Float32`, 8 `Float64`. Elements read and write as JavaScript numbers; a store converts as the
//! spec's `ToInt8` .. `ToUint32`, `ToUint8Clamp` and float rounding do.

use super::*;

const SIZES: [usize; 9] = [1, 1, 1, 2, 2, 4, 4, 4, 8];

fn size_of_kind(kind: u8) -> usize {
    SIZES[(kind as usize).min(8)]
}

/// ToIntegerOrInfinity as the low 64 bits (two's complement): `ToInt32` and friends take the low
/// bits of it. A double of 2^63 or more has its low bits zero from 2^85 up; below that it is rare,
/// so those are 0 as well.
fn wrap_int(v: f64) -> u64 {
    if !v.is_finite() || v.abs() >= 9.223372036854776e18 {
        return 0;
    }
    (v.trunc() as i64) as u64
}

/// ToUint8Clamp: round half to even, clamp to 0..=255, NaN is 0.
fn clamp_u8(v: f64) -> u8 {
    if v.is_nan() {
        return 0;
    }
    let c = v.clamp(0.0, 255.0);
    let r = c.round();
    // Round half to even.
    let r = if (c - c.floor() - 0.5).abs() < f64::EPSILON && r % 2.0 != 0.0 {
        r - 1.0
    } else {
        r
    };
    r as u8
}

/// Writes element `i` of `data` as `kind`.
///
/// # Safety
/// `data` must hold at least `i + 1` elements of `kind`.
unsafe fn store(data: *mut u8, kind: u8, i: usize, v: f64) {
    // SAFETY: by contract.
    unsafe {
        match kind {
            0 | 1 => *data.add(i) = wrap_int(v) as u8,
            2 => *data.add(i) = clamp_u8(v),
            3 | 4 => (data as *mut u16)
                .add(i)
                .write_unaligned(wrap_int(v) as u16),
            5 | 6 => (data as *mut u32)
                .add(i)
                .write_unaligned(wrap_int(v) as u32),
            7 => (data as *mut f32).add(i).write_unaligned(v as f32),
            _ => (data as *mut f64).add(i).write_unaligned(v),
        }
    }
}

/// Reads element `i` of `data` as `kind`.
///
/// # Safety
/// `data` must hold at least `i + 1` elements of `kind`.
unsafe fn load(data: *const u8, kind: u8, i: usize) -> f64 {
    // SAFETY: by contract.
    unsafe {
        match kind {
            0 => f64::from(*(data as *const i8).add(i)),
            1 | 2 => f64::from(*data.add(i)),
            3 => f64::from((data as *const i16).add(i).read_unaligned()),
            4 => f64::from((data as *const u16).add(i).read_unaligned()),
            5 => f64::from((data as *const i32).add(i).read_unaligned()),
            6 => f64::from((data as *const u32).add(i).read_unaligned()),
            7 => f64::from((data as *const f32).add(i).read_unaligned()),
            _ => (data as *const f64).add(i).read_unaligned(),
        }
    }
}

fn new_typed(h: &mut Heap, kind: u8, len: i64) -> *mut Array {
    let a = new_obj(h, HDR_BYTES, std::mem::size_of::<Array>()) as *mut Array;
    let len = len.max(0);
    let bytes = len as usize * size_of_kind(kind);
    let d = raw_alloc(h, bytes, true);
    // SAFETY: just allocated.
    unsafe {
        (*a).len = len;
        (*a).cap = bytes as i64;
        (*a).data = d;
    }
    a
}

/// `new Float32Array(length)`: zeroed.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_typed_new_gc(kind: f64, len: f64) -> *mut Array {
    let len = if len.is_finite() && len >= 0.0 {
        len as i64
    } else {
        0
    };
    with_heap(|h| {
        maybe_collect(h);
        new_typed(h, kind as u8, len)
    })
}

/// `new Float32Array(numbers)`: copies a `number[]`.
///
/// # Safety
/// `src` must be a live array of numbers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_typed_from_array_gc(kind: f64, src: *const Array) -> *mut Array {
    // The source is copied out first: it may be an unrooted temporary the allocation would free.
    // SAFETY: by contract.
    let vals: Vec<f64> = unsafe {
        (0..(*src).len as usize)
            .map(|i| f64::from_bits(*((*src).data as *const u64).add(i)))
            .collect()
    };
    let a = with_heap(|h| {
        maybe_collect(h);
        new_typed(h, kind as u8, vals.len() as i64)
    });
    for (i, v) in vals.into_iter().enumerate() {
        // SAFETY: `a` has `vals.len()` elements.
        unsafe { store((*a).data, kind as u8, i, v) };
    }
    a
}

/// `new Float32Array(other)`: copies another typed array, converting the elements.
///
/// # Safety
/// `src` must be a live typed array of `src_kind`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_typed_from_typed_gc(
    kind: f64,
    src_kind: f64,
    src: *const Array,
) -> *mut Array {
    // SAFETY: by contract.
    let vals: Vec<f64> = unsafe {
        (0..(*src).len as usize)
            .map(|i| load((*src).data, src_kind as u8, i))
            .collect()
    };
    let a = with_heap(|h| {
        maybe_collect(h);
        new_typed(h, kind as u8, vals.len() as i64)
    });
    for (i, v) in vals.into_iter().enumerate() {
        // SAFETY: `a` has `vals.len()` elements.
        unsafe { store((*a).data, kind as u8, i, v) };
    }
    a
}

/// A `number[]` copy of a typed array (for the array methods that only read).
///
/// # Safety
/// `src` must be a live typed array of `kind`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_typed_to_array_gc(kind: f64, src: *const Array) -> *mut Array {
    // SAFETY: by contract.
    let vals: Vec<f64> = unsafe {
        (0..(*src).len as usize)
            .map(|i| load((*src).data, kind as u8, i))
            .collect()
    };
    let out = ts_rt_arr_new_gc(vals.len() as i64, 0);
    for v in vals {
        // SAFETY: `out` has room; `ts_rt_arr_push` does not collect.
        unsafe { ts_rt_arr_push(out, v.to_bits()) };
    }
    out
}

fn range(len: i64, start: f64, end: f64) -> (usize, usize) {
    let rel = |v: f64, default: i64| -> i64 {
        if v.is_nan() {
            return default;
        }
        let t = v.trunc();
        if t < 0.0 {
            (len as f64 + t).max(0.0) as i64
        } else {
            t.min(len as f64) as i64
        }
    };
    let s = rel(start, 0);
    let e = if end == f64::INFINITY {
        len
    } else {
        rel(end, len)
    };
    (s as usize, e.max(s) as usize)
}

/// `typed.fill(value, start, end)`.
///
/// # Safety
/// `a` must be a live typed array of `kind`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_typed_fill(
    a: *mut Array,
    kind: f64,
    value: f64,
    start: f64,
    end: f64,
) {
    // SAFETY: by contract.
    unsafe {
        let (s, e) = range((*a).len, start, end);
        for i in s..e {
            store((*a).data, kind as u8, i, value);
        }
    }
}

/// `typed.set(source, offset)` from a `number[]` (`src_kind` 255) or another typed array. A
/// `RangeError` if it does not fit.
///
/// # Safety
/// Both must be live arrays of the kinds given.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_typed_set(
    a: *mut Array,
    kind: f64,
    src: *const Array,
    src_kind: f64,
    offset: f64,
) {
    // SAFETY: by contract.
    unsafe {
        let off = if offset.is_nan() { 0.0 } else { offset.trunc() };
        if off < 0.0 {
            raise(1.0, "Offset should not be negative");
            return;
        }
        if off + (*src).len as f64 > (*a).len as f64 {
            raise(
                1.0,
                "Range consisting of offset and length are out of bounds",
            );
            return;
        }
        let off = off as usize;
        // Copy through a buffer: the source may be the same array.
        let vals: Vec<f64> = (0..(*src).len as usize)
            .map(|i| {
                if src_kind as u8 == 255 {
                    f64::from_bits(*((*src).data as *const u64).add(i))
                } else {
                    load((*src).data, src_kind as u8, i)
                }
            })
            .collect();
        for (i, v) in vals.into_iter().enumerate() {
            store((*a).data, kind as u8, off + i, v);
        }
    }
}

/// `typed.slice(start, end)`: a copy.
///
/// # Safety
/// `a` must be a live typed array of `kind`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_typed_slice_gc(
    a: *const Array,
    kind: f64,
    start: f64,
    end: f64,
) -> *mut Array {
    let size = size_of_kind(kind as u8);
    // SAFETY: by contract; the bytes are copied out before the allocation can free `a`.
    let bytes: Vec<u8> = unsafe {
        let (s, e) = range((*a).len, start, end);
        std::slice::from_raw_parts((*a).data.add(s * size), (e - s) * size).to_vec()
    };
    let out = with_heap(|h| {
        maybe_collect(h);
        new_typed(h, kind as u8, (bytes.len() / size) as i64)
    });
    // SAFETY: `out` holds `bytes.len()` bytes.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), (*out).data, bytes.len()) };
    out
}

/// `typed.reverse()` and `typed.sort()` (ascending, NaN last), in place.
///
/// # Safety
/// `a` must be a live typed array of `kind`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_typed_reorder(a: *mut Array, kind: f64, sort: bool) {
    // SAFETY: by contract.
    unsafe {
        let n = (*a).len as usize;
        let mut vals: Vec<f64> = (0..n).map(|i| load((*a).data, kind as u8, i)).collect();
        if sort {
            vals.sort_by(|x, y| x.total_cmp(y));
            // total_cmp orders -0 before +0 and negative NaN first: only the NaN placement differs.
            let nans = vals.iter().filter(|v| v.is_nan()).count();
            vals.retain(|v| !v.is_nan());
            vals.extend(std::iter::repeat_n(f64::NAN, nans));
        } else {
            vals.reverse();
        }
        for (i, v) in vals.into_iter().enumerate() {
            store((*a).data, kind as u8, i, v);
        }
    }
}
