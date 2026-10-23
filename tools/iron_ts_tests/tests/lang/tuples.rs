// SPDX-License-Identifier: Apache-2.0

//! Tuples (`.length`, writes through `t[0]`, rest patterns, spreading a tuple into an array or a
//! call), destructuring assignment (`[a, b] = [b, a]`) and defaulted boolean closure parameters.
//! The programs of `bench/tuples_cases.ts` run in the normal, stress GC and collection-off modes and
//! are compared with bun (`bench/gen_cases_expected.ts`). IDs are in TESTS.md (M9-TS-3149..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/tuples_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/tuples_expected.json")).unwrap();
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
    3149 length_and_index_writes "length_and_index_writes" "tu0_" 800000;
    3150 rest_and_spread "rest_and_spread" "tu1_" 801000;
    3151 tuple_params_and_loops "tuple_params_and_loops" "tu2_" 802000;
    3152 destructuring_assignment "destructuring_assignment" "tu3_" 803000;
    3153 swap_in_a_loop "swap_in_a_loop" "tu4_" 804000;
    3154 defaulted_boolean_closure_params "defaulted_boolean_closure_params" "tu5_" 805000;
}
