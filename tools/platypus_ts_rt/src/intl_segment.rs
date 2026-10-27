// SPDX-License-Identifier: Apache-2.0

//! `Intl.Segmenter` on ICU4X: grapheme clusters, words and sentences. A segmenter is just its
//! granularity (0 grapheme, 1 word, 2 sentence); `ts_rt_segments_gc` returns the segments of a string
//! as a flat `number[]` of `start, end, isWordLike` triples in UTF-16 units, which the prelude turns
//! into `Intl.Segments`.

use super::*;
use icu::segmenter::options::{SentenceBreakInvariantOptions, WordBreakInvariantOptions};
use icu::segmenter::{GraphemeClusterSegmenter, SentenceSegmenter, WordSegmenter};
use std::collections::HashMap;

/// `new Intl.Segmenter(locales, { granularity })`: the granularity as a number.
///
/// # Safety
/// `options` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_segmenter_new(_locales: *const Str, options: *const Str) -> f64 {
    // SAFETY: by contract.
    let spec = unsafe { (*options).to_string_lossy().to_string() };
    let o: HashMap<&str, &str> = spec
        .split(';')
        .filter_map(|kv| kv.split_once('='))
        .collect();
    match o.get("granularity").copied() {
        None | Some("grapheme") => 0.0,
        Some("word") => 1.0,
        Some("sentence") => 2.0,
        Some(_) => {
            raise(
                1.0,
                "granularity must be either \"grapheme\", \"word\", or \"sentence\"",
            );
            0.0
        }
    }
}

/// The segments of `s`: `[start, end, wordLike, ...]`.
///
/// # Safety
/// `s` must be a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_segments_gc(granularity: f64, s: *const Str) -> *mut Array {
    // SAFETY: by contract; copied out before anything allocates.
    let text: Vec<u16> = unsafe { (*s).to_utf16() };
    let mut out: Vec<f64> = Vec::new();
    match granularity as u8 {
        0 => {
            let mut prev = 0usize;
            for b in GraphemeClusterSegmenter::new().segment_utf16(&text) {
                if b > prev {
                    out.extend([prev as f64, b as f64, 0.0]);
                    prev = b;
                }
            }
        }
        1 => {
            let seg = WordSegmenter::new_auto(WordBreakInvariantOptions::default());
            let mut it = seg.segment_utf16(&text);
            let mut prev = 0usize;
            while let Some(b) = it.next() {
                if b > prev {
                    out.extend([
                        prev as f64,
                        b as f64,
                        f64::from(u8::from(it.is_word_like())),
                    ]);
                    prev = b;
                }
            }
        }
        _ => {
            let seg = SentenceSegmenter::new(SentenceBreakInvariantOptions::default());
            let mut prev = 0usize;
            for b in seg.segment_utf16(&text) {
                if b > prev {
                    out.extend([prev as f64, b as f64, 0.0]);
                    prev = b;
                }
            }
        }
    }
    let a = ts_rt_arr_new_gc(out.len() as i64, 0);
    for v in out {
        // SAFETY: `a` is a fresh array; pushing never collects.
        unsafe { ts_rt_arr_push(a, v.to_bits()) };
    }
    a
}
