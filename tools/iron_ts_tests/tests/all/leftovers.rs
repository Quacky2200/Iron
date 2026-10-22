//! Dictionaries and small language leftovers the original example sources use: iterating, testing
//! and deleting the keys of `Record<Union, V>` and `Record<string, V>`, `Object.entries` /
//! `fromEntries`, `as const` arrays as literal unions, `flat(depth)` with a depth known at run
//! time, a tuple as a rest parameter. The programs of `bench/leftovers_cases.ts` run in the
//! normal, stress GC and collection-off modes and are compared with bun
//! (`bench/gen_cases_expected.ts`). IDs are in TESTS.md (M9-TS-3190..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/leftovers_cases.ts");

fn expected(name: &str) -> String {
    let v: Value =
        serde_json::from_str(include_str!("../../bench/leftovers_expected.json")).unwrap();
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
    3190 record_over_a_union_is_iterable "record_over_a_union_is_iterable" "lo0_" 2100000;
    3191 partial_record_presence_and_delete "partial_record_presence_and_delete" "lo1_" 2101000;
    3192 string_dictionary_operations "string_dictionary_operations" "lo2_" 2102000;
    3193 const_arrays_are_literal_unions "const_arrays_are_literal_unions" "lo3_" 2103000;
    3194 flat_with_a_depth_that_is_not_a_literal "flat_with_a_depth_that_is_not_a_literal" "lo4_" 2104000;
    3195 a_tuple_as_a_rest_parameter "a_tuple_as_a_rest_parameter" "lo5_" 2105000;
    3196 const_array_with_a_string_argument "const_array_with_a_string_argument" "lo6_" 2106000;
}
