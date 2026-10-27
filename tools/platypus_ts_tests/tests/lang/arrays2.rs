// SPDX-License-Identifier: Apache-2.0

//! Array and spread features: `Array.from`, `new Array(n)`, `fill`, `flat`, `indexOf`, spread
//! arguments and writes at the length. The programs of `bench/arrays2_cases.ts` run in the normal,
//! stress GC and collection-off modes and are compared with bun (`bench/gen_cases_expected.ts`).
//! IDs are in TESTS.md (M9-TS-3020..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/arrays2_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/arrays2_expected.json")).unwrap();
    v[name].as_str().unwrap().to_string()
}

fn bun_case(name: &str, prefix: &'static str, base: u32) {
    let got = run(case(CASES, name), prefix, base).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(got, expected(name), "{name}");
}

macro_rules! bun_tests {
    ($( $id:literal $fn:ident $name:literal $prefix:literal $base:literal; )*) => {$(
        #[doc = concat!("M9-TS-", stringify!($id))]
        #[test]
        fn $fn() {
            bun_case($name, $prefix, $base);
        }
    )*};
}

bun_tests! {
    3020 from_length "from_length" "ar0_" 400000;
    3021 from_array_and_iterables "from_array_and_iterables" "ar1_" 401000;
    3022 new_array_n "new_array_n" "ar2_" 402000;
    3023 fill_range "fill_range" "ar3_" 403000;
    3024 flat_and_flat_map "flat_and_flat_map" "ar4_" 404000;
    3025 index_of_all_types "index_of_all_types" "ar5_" 405000;
    3026 spread_in_calls "spread_in_calls" "ar6_" 406000;
    3027 spread_into_fixed_params "spread_into_fixed_params" "ar7_" 407000;
    3028 spread_arrays_and_objects "spread_arrays_and_objects" "ar8_" 408000;
    3029 write_at_length_grows "write_at_length_grows" "ar9_" 409000;
    3030 read_out_of_bounds "read_out_of_bounds" "ar10_" 410000;
    3031 spread_edge_cases "spread_edge_cases" "ar11_" 411000;
}

/// Runs `t()` of a program (not compared with bun: JavaScript makes holes where we throw).
fn run_src(src: &str, prefix: &'static str, base: u32) -> String {
    run(src, prefix, base).unwrap_or_else(|e| panic!("{e}"))
}

// M9-TS-3032
#[test]
fn writing_past_the_length_throws_a_range_error_and_keeps_the_array() {
    let got = run_src(
        r#"function t(): string {
  const a = [1, 2, 3];
  const out: string[] = [];
  for (const i of [5, -1, 3.5]) {
    try {
      a[i] = 9;
      out.push("wrote");
    } catch (e) {
      out.push((e as Error).name);
    }
  }
  a[3] = 4;
  return out.join(",") + "|" + a.join("") + "|" + a.length;
}"#,
        "ar12_",
        412_000,
    );
    assert_eq!(got, "RangeError,RangeError,RangeError|1234|4");
}

// M9-TS-3033
#[test]
fn a_write_past_the_length_message_names_the_index() {
    let got = run_src(
        r#"function t(): string {
  const a: string[] = [];
  try {
    a[2] = "x";
  } catch (e) {
    return (e as Error).message;
  }
  return "no error";
}"#,
        "ar13_",
        413_000,
    );
    assert!(got.starts_with("array index 2 is beyond the end"), "{got}");
}

// M9-TS-3034
#[test]
fn writes_to_typed_arrays_out_of_range_are_still_ignored() {
    let got = run_src(
        r#"function t(): string {
  const a = new Float64Array(2);
  a[5] = 1;
  a[0] = 3;
  return a.length + ":" + a[0] + ":" + a[5];
}"#,
        "ar14_",
        414_000,
    );
    assert_eq!(got, "2:3:0");
}
