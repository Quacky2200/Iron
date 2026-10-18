//! Strings, part two, against JavaScript: trimming, case mapping, `at`, `repeat`, padding,
//! `split` / `join`, and number <-> string conversion. `tests/strings_more_expected.json` is what
//! bun answers (second half of `tests/gen_strings.ts`). IDs are in TESTS.md (M9-TS-173..).
use iron_ts_rt::*;
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

fn text(s: &str) -> *mut Str {
    unsafe { ts_rt_str_new(s.as_ptr(), s.len() as i64) }
}

/// A number from the JSON: non-finite values are strings there.
fn num(v: &Value) -> f64 {
    match v {
        Value::String(s) => match s.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            other => panic!("number {other}"),
        },
        other => other.as_f64().unwrap(),
    }
}

fn same(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b
}

struct Data {
    j: Value,
    texts: Vec<*mut Str>,
    text_lens: Vec<usize>,
    pads: Vec<*mut Str>,
    seps: Vec<*mut Str>,
    joiners: Vec<*mut Str>,
}

fn load() -> Data {
    ts_rt_gc_enable(false); // the test holds every string unrooted
    let j: Value = serde_json::from_str(include_str!("strings_more_expected.json")).unwrap();
    let list = |k: &str| -> Vec<Vec<u16>> { j[k].as_array().unwrap().iter().map(units).collect() };
    let texts_u = list("texts");
    Data {
        texts: texts_u.iter().map(|u| make(u)).collect(),
        text_lens: texts_u.iter().map(|u| u.len()).collect(),
        pads: list("pads").iter().map(|u| make(u)).collect(),
        seps: list("seps").iter().map(|u| make(u)).collect(),
        joiners: list("joiners").iter().map(|u| make(u)).collect(),
        j,
    }
}

// M9-TS-173
//
// `trim`, `trimStart`, `trimEnd`, `toUpperCase` (`ß` -> `SS`, `ﬃ` -> `FFI`) and `toLowerCase` (final
// sigma, `İ`), on strings with Unicode white space, accents, CJK and lone surrogates.
#[test]
fn trimming_and_case_mapping_agree_with_javascript() {
    let d = load();
    for (i, &s) in d.texts.iter().enumerate() {
        let t = &d.j["trim"][i];
        assert_eq!(
            got(unsafe { ts_rt_str_trim_gc(s) }),
            units(&t[0]),
            "trim {i}"
        );
        assert_eq!(
            got(unsafe { ts_rt_str_trim_start_gc(s) }),
            units(&t[1]),
            "trimStart {i}"
        );
        assert_eq!(
            got(unsafe { ts_rt_str_trim_end_gc(s) }),
            units(&t[2]),
            "trimEnd {i}"
        );
        let c = &d.j["case"][i];
        assert_eq!(
            got(unsafe { ts_rt_str_to_upper_gc(s) }),
            units(&c[0]),
            "upper {i}"
        );
        assert_eq!(
            got(unsafe { ts_rt_str_to_lower_gc(s) }),
            units(&c[1]),
            "lower {i}"
        );
    }
}

// M9-TS-174
//
// `at` (negative from the end, null out of range), `repeat`, `padStart` and `padEnd` with
// filler strings that are cut in the middle of a surrogate pair.
#[test]
fn at_repeat_and_padding_agree_with_javascript() {
    let d = load();
    for (i, &s) in d.texts.iter().enumerate() {
        let n = d.text_lens[i] as i64;
        for (k, at) in (-3..=n + 1).enumerate() {
            let w = &d.j["at"][i][k];
            let r = unsafe { ts_rt_str_at_gc(s, at as f64) };
            if w.is_null() {
                assert!(r.is_null(), "at {i} {at}");
            } else {
                assert_eq!(got(r), units(w), "at {i} {at}");
            }
        }
        for count in 0..=3 {
            let w = units(&d.j["repeat"][i][count]);
            assert_eq!(
                got(unsafe { ts_rt_str_repeat_gc(s, count as f64) }),
                w,
                "repeat {i} {count}"
            );
        }
        let mut k = 0;
        for target in 0..=n + 3 {
            for &p in &d.pads {
                let w = units(&d.j["pad"][i][k]);
                let r = unsafe { ts_rt_str_pad_start_gc(s, target as f64, p) };
                assert_eq!(got(r), w, "padStart {i} {target}");
                let w = units(&d.j["pad"][i][k + 1]);
                let r = unsafe { ts_rt_str_pad_end_gc(s, target as f64, p) };
                assert_eq!(got(r), w, "padEnd {i} {target}");
                k += 2;
            }
        }
    }
}

// M9-TS-175
//
// `split` into an array of strings (empty, one-unit, multi-unit and lone-surrogate separators)
// and `join` of the pieces with several joiners, which rejoins pairs that were split.
#[test]
fn split_and_join_agree_with_javascript() {
    let d = load();
    let mut k = 0;
    for &s in &d.texts {
        for &sep in &d.seps {
            let arr = unsafe { ts_rt_str_split_gc(s, sep) };
            let a = unsafe { &*arr };
            let want = d.j["split"][k].as_array().unwrap();
            assert_eq!(a.len(), want.len(), "split {k}");
            for (n, w) in want.iter().enumerate() {
                let piece: *mut Str = unsafe { a.get::<*mut Str>(n) };
                assert_eq!(got(piece), units(w), "split {k} piece {n}");
            }
            for (m, &j) in d.joiners.iter().enumerate() {
                let r = unsafe { ts_rt_str_join_gc(arr, j) };
                assert_eq!(got(r), units(&d.j["join"][k][m]), "join {k} {m}");
            }
            k += 1;
        }
    }
}

// M9-TS-176
//
// `String(x)`, `x.toFixed(0..6)` and `x.toString(radix)`: shortest round-trip digits, the exponent
// layouts at 1e21 and 1e-7, and `toFixed` rounding exact ties up (2.5 -> "3", 1.005 -> "1.00").
#[test]
fn number_to_string_agrees_with_javascript() {
    let d = load();
    for (i, x) in d.j["numbers"].as_array().unwrap().iter().enumerate() {
        let x = num(x);
        let want = d.j["numToString"][i].as_str().unwrap();
        assert_eq!(number_to_string(x), want, "String({x:e})");
        assert_eq!(
            got(ts_rt_num_to_str_gc(x)),
            want.encode_utf16().collect::<Vec<_>>()
        );
        for digits in 0..=6 {
            let want = d.j["numToFixed"][i][digits].as_str().unwrap();
            assert_eq!(
                number_to_fixed(x, digits),
                want,
                "({x:e}).toFixed({digits})"
            );
        }
    }
    for (i, x) in d.j["ints"].as_array().unwrap().iter().enumerate() {
        let x = x.as_f64().unwrap();
        for (m, radix) in [2.0, 8.0, 16.0, 36.0].into_iter().enumerate() {
            let want = d.j["numRadix"][i][m].as_str().unwrap();
            let r = got(ts_rt_num_to_radix_gc(x, radix));
            assert_eq!(
                r,
                want.encode_utf16().collect::<Vec<_>>(),
                "({x}).toString({radix})"
            );
        }
    }
}

// M9-TS-177
//
// `Number(s)`, `parseFloat(s)` and `parseInt(s, radix)` over decimal, hex, binary, octal, signs,
// exponents, Unicode white space, trailing garbage and invalid radixes.
#[test]
fn parsing_numbers_agrees_with_javascript() {
    let d = load();
    for (i, u) in d.j["parseCases"].as_array().unwrap().iter().enumerate() {
        let s = make(&units(u));
        let label = String::from_utf16_lossy(&units(u));
        let w = num(&d.j["number"][i]);
        let r = unsafe { ts_rt_str_to_number(s) };
        assert!(same(r, w), "Number({label:?}): {r} vs {w}");
        let w = num(&d.j["parseFloat"][i]);
        let r = unsafe { ts_rt_str_parse_float(s) };
        assert!(same(r, w), "parseFloat({label:?}): {r} vs {w}");
        let radixes = [f64::NAN, 10.0, 16.0, 2.0, 36.0, 37.0, 1.0, 0.0];
        for (m, radix) in radixes.into_iter().enumerate() {
            let w = num(&d.j["parseInt"][i][m]);
            let r = unsafe { ts_rt_str_parse_int(s, radix) };
            assert!(same(r, w), "parseInt({label:?}, {radix}): {r} vs {w}");
        }
    }
    let _ = text("");
}
