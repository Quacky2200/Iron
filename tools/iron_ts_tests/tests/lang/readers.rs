//! TypeScript the md-reader and launcher examples use, compiled as written: intersections and
//! recursive object aliases, a discriminated union whose variants give a field different types,
//! `filter(Boolean)`, hoisted function declarations, `x || null`, `s ?? 1` as a `string | number`,
//! closure result and default-parameter inference, negative literal types, `new Set(set)`,
//! `find` over literals, `re.exec(s)?.[0]`, text-level passes that leave strings alone, literal
//! unions from and to missing strings, `id<F>(f: F)`, nullable copies, `clearTimeout(undefined)`.
//! The programs of `bench/readers_cases.ts` run in the normal, stress GC and collection-off modes
//! and are compared with bun (`bench/gen_cases_expected.ts`). IDs are in TESTS.md (M9-TS-3365..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/readers_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/readers_expected.json")).unwrap();
    v[name].as_str().unwrap().to_string()
}

fn bun_case(name: &str, prefix: &'static str, base: u32) {
    let got = run(case(CASES, name), prefix, base).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(got, expected(name), "{name}");
}

macro_rules! bun_tests {
    ($( $id:literal $fn:ident $prefix:literal $base:literal; )*) => {$(
        #[doc = concat!("M9-TS-", stringify!($id))]
        #[test]
        fn $fn() {
            bun_case(stringify!($fn), $prefix, $base);
        }
    )*};
}

bun_tests! {
    3365 intersection_with_a_recursive_union "rd0_" 2_400_000;
    3366 a_tag_test_narrows_a_field_of_another_type "rd1_" 2_401_000;
    3367 filter_boolean_and_boolean_calls "rd2_" 2_402_000;
    3368 function_declarations_inside_a_body_are_hoisted "rd3_" 2_403_000;
    3369 or_null_and_a_string_or_a_number "rd4_" 2_404_000;
    3370 closures_infer_results_and_default_parameter_types "rd5_" 2_405_000;
    3371 negative_number_literal_types "rd6_" 2_406_000;
    3372 sets_and_maps_copied_from_others "rd7_" 2_407_000;
    3373 find_over_a_tuple_of_literals "rd8_" 2_408_000;
    3374 optional_index_of_a_match "rd9_" 2_409_000;
    3375 strings_that_mention_code_keep_their_text "rd10_" 2_410_000;
    3376 literal_unions_from_and_to_strings_that_may_be_missing "rd11_" 2_411_000;
    3377 a_generic_function_infers_a_closure_type "rd12_" 2_412_000;
    3378 a_copy_where_a_nullable_type_goes_keeps_null "rd13_" 2_413_000;
    3379 clear_timeout_takes_undefined "rd14_" 2_414_000;
}
