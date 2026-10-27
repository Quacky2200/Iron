// SPDX-License-Identifier: Apache-2.0

//! The runtime's strings against JavaScript: `tests/strings_expected.json` holds what bun answers
//! (made by `tests/gen_strings.ts`, regenerate with `bun tests/gen_strings.ts`) for every
//! operation over a corpus with ASCII, Latin-1, CJK, astral and lone-surrogate strings. The
//! operations run here in the same order on the runtime's WTF-8 strings. IDs are in TESTS.md
//! (M9-TS-168..).
use platypus_ts_rt::*;
use serde_json::Value;

fn units(v: &Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u16)
        .collect()
}

fn make(u: &[u16]) -> *mut Str {
    unsafe { ts_rt_str_from_utf16(u.as_ptr(), u.len() as i64) }
}

fn got(p: *mut Str) -> Vec<u16> {
    unsafe { Str::from_raw(p) }.to_utf16()
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

/// Equal as JavaScript numbers (NaN is `null` in the JSON).
fn same(a: f64, b: &Value) -> bool {
    if b.is_null() { a.is_nan() } else { a == num(b) }
}

fn range(a: i64, b: i64) -> Vec<f64> {
    (a..=b).map(|x| x as f64).collect()
}

struct Case {
    expected: Value,
    strs: Vec<*mut Str>,
    lens: Vec<i64>,
}

fn load() -> Case {
    ts_rt_gc_enable(false); // the test holds every string unrooted
    let expected: Value = serde_json::from_str(include_str!("strings_expected.json")).unwrap();
    let corpus: Vec<Vec<u16>> = expected["corpus"]
        .as_array()
        .unwrap()
        .iter()
        .map(units)
        .collect();
    let strs = corpus.iter().map(|u| make(u)).collect();
    let lens = corpus.iter().map(|u| u.len() as i64).collect();
    Case {
        expected,
        strs,
        lens,
    }
}

// M9-TS-168
//
// A string built from UTF-16 units reads back the same units, and `length`, `charCodeAt`,
// `codePointAt` and `charAt` agree with JavaScript at every index, in and out of range.
#[test]
fn units_and_lengths_agree_with_javascript() {
    let c = load();
    let single = &c.expected["single"];
    for (i, &s) in c.strs.iter().enumerate() {
        assert_eq!(got(s), units(&c.expected["corpus"][i]), "string {i}");
        assert_eq!(
            unsafe { ts_rt_str_length(s) },
            num(&single["length"][i]),
            "length {i}"
        );
        for (k, at) in range(-1, c.lens[i] + 1).into_iter().enumerate() {
            let w = &single["charCodeAt"][i][k];
            assert!(
                same(unsafe { ts_rt_str_char_code_at(s, at) }, w),
                "charCodeAt {i} {at}"
            );
            let w = &single["codePointAt"][i][k];
            assert!(
                same(unsafe { ts_rt_str_code_point_at(s, at) }, w),
                "codePointAt {i} {at}"
            );
            let w = units(&single["charAt"][i][k]);
            assert_eq!(
                got(unsafe { ts_rt_str_char_at_gc(s, at) }),
                w,
                "charAt {i} {at}"
            );
        }
    }
}

// M9-TS-169
//
// `slice` and `substring` cut by unit index, through surrogate pairs too, and produce the same
// units as JavaScript for every pair of indices (negative, past the end, swapped).
#[test]
fn slice_and_substring_agree_with_javascript() {
    let c = load();
    let single = &c.expected["single"];
    for (i, &s) in c.strs.iter().enumerate() {
        let grid = range(-2, c.lens[i] + 1);
        let mut k = 0;
        for &a in &grid {
            for &b in &grid {
                let w = units(&single["slice"][i][k]);
                assert_eq!(
                    got(unsafe { ts_rt_str_slice_gc(s, a, b) }),
                    w,
                    "slice {i} {a} {b}"
                );
                let w = units(&single["substring"][i][k]);
                assert_eq!(
                    got(unsafe { ts_rt_str_substring_gc(s, a, b) }),
                    w,
                    "substring {i} {a} {b}"
                );
                k += 1;
            }
        }
        for (k, &a) in grid.iter().enumerate() {
            let w = units(&single["slice1"][i][k]);
            let r = unsafe { ts_rt_str_slice_gc(s, a, f64::INFINITY) };
            assert_eq!(got(r), w, "slice1 {i} {a}");
        }
    }
}

// M9-TS-170
//
// Equality, ordering by code unit (`<`) and concatenation (which rejoins a split pair) over
// every pair of strings in the corpus.
#[test]
fn comparison_and_concatenation_agree_with_javascript() {
    let c = load();
    let pair = &c.expected["pair"];
    let n = c.strs.len();
    for i in 0..n {
        for j in 0..n {
            let k = i * n + j;
            let (a, b) = (c.strs[i], c.strs[j]);
            assert_eq!(
                unsafe { ts_rt_str_compare(a, b) } as f64,
                num(&pair["compare"][k]),
                "compare {i} {j}"
            );
            assert_eq!(
                unsafe { ts_rt_str_eq(a, b) },
                pair["eq"][k].as_bool().unwrap(),
                "eq {i} {j}"
            );
            let joined = unsafe { ts_rt_str_concat_gc(a, b) };
            assert_eq!(got(joined), units(&pair["concat"][k]), "concat {i} {j}");
            // The result is canonical: equal to a string built from its units.
            assert!(
                unsafe { ts_rt_str_eq(joined, make(&got(joined))) },
                "canonical {i} {j}"
            );
        }
    }
}

// M9-TS-171
//
// `indexOf`, `lastIndexOf`, `includes`, `startsWith` and `endsWith` with every position, over
// every pair of strings, including needles that are lone surrogates inside a pair.
#[test]
fn searching_agrees_with_javascript() {
    let c = load();
    let pair = &c.expected["pair"];
    let n = c.strs.len();
    for i in 0..n {
        for j in 0..n {
            let k = i * n + j;
            let (s, t) = (c.strs[i], c.strs[j]);
            let from = range(-1, c.lens[i] + 1);
            let last = &pair["lastIndexOf"][k];
            assert_eq!(
                unsafe { ts_rt_str_last_index_of(s, t, f64::NAN) },
                num(&last[0]),
                "lastIndexOf {i} {j}"
            );
            let ends = &pair["endsWith"][k];
            assert_eq!(
                unsafe { ts_rt_str_ends_with(s, t, f64::NAN) },
                ends[0].as_bool().unwrap(),
                "endsWith {i} {j}"
            );
            for (m, &f) in from.iter().enumerate() {
                let w = num(&pair["indexOf"][k][m]);
                assert_eq!(
                    unsafe { ts_rt_str_index_of(s, t, f) },
                    w,
                    "indexOf {i} {j} {f}"
                );
                assert_eq!(
                    unsafe { ts_rt_str_last_index_of(s, t, f) },
                    num(&last[m + 1]),
                    "lastIndexOf {i} {j} {f}"
                );
                let w = pair["includes"][k][m].as_bool().unwrap();
                assert_eq!(
                    unsafe { ts_rt_str_includes(s, t, f) },
                    w,
                    "includes {i} {j} {f}"
                );
                let w = pair["startsWith"][k][m].as_bool().unwrap();
                assert_eq!(
                    unsafe { ts_rt_str_starts_with(s, t, f) },
                    w,
                    "startsWith {i} {j} {f}"
                );
                let w = ends[m + 1].as_bool().unwrap();
                assert_eq!(
                    unsafe { ts_rt_str_ends_with(s, t, f) },
                    w,
                    "endsWith {i} {j} {f}"
                );
            }
        }
    }
}
