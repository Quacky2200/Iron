// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Language leftovers: `for...in` over arrays, `Object.assign` into dictionaries and objects,
//! setters in object literals, narrowing that survives a closure (a `const`, or a variable never
//! assigned), `in` / `instanceof` / truthiness narrowing of unions, `flat(depth)` with a depth known
//! at run time. The programs of `bench/lang2_cases.ts` run in the normal, stress GC and
//! collection-off modes and are compared with bun (`bench/gen_cases_expected.ts`). IDs are in
//! TESTS.md (M9-TS-3216..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/lang2_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/lang2_expected.json")).unwrap();
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
    3216 for_in_over_an_array_gives_the_indices_as_strings "for_in_over_arrays" "l20_" 2200000;
    3217 object_assign_into_a_dictionary_and_an_object "object_assign_into_dictionaries_and_objects" "l21_" 2201000;
    3218 an_object_literal_with_a_getter_and_a_setter "setters_in_object_literals" "l22_" 2202000;
    3219 narrowing_survives_a_closure_over_a_const_or_an_unassigned_parameter "narrowing_across_closures" "l24_" 2204000;
    3220 a_closure_over_a_reassigned_variable_does_not_keep_the_narrowing "narrowing_across_closures_stops_at_a_reassignment" "l25_" 2205000;
    3221 in_narrows_a_union_of_object_types "in_narrowing_of_a_union" "l26_" 2206000;
    3222 instanceof_narrows_a_union_of_classes "instanceof_narrowing_of_a_union" "l27_" 2207000;
    3223 truthiness_narrows_a_union_of_number_string_array_and_null "truthiness_narrowing_of_a_union" "l28_" 2208000;
    3224 flat_with_a_run_time_depth_flattens_that_deep "flat_with_a_run_time_depth" "l29_" 2209000;
}
