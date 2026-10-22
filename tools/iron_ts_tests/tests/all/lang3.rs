//! TypeScript-semantics fixes: `flat(depth)` with a depth known only at run time, `this` in
//! object literal methods and accessors, `Object.assign` into objects by key, string array
//! parameters of host functions. The programs of `bench/lang3_cases.ts` run in the normal, stress
//! GC and collection-off modes and are compared with bun (`bench/gen_cases_expected.ts`). IDs are
//! in TESTS.md (M9-TS-3313..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/lang3_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/lang3_expected.json")).unwrap();
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
    3313 flat_with_a_run_time_depth_over_two_levels_returns_a_union_array "flat_run_time_depth_two_levels" "m1_" 2300000;
    3314 flat_with_a_run_time_depth_over_three_levels_flattens_exactly_that_deep "flat_run_time_depth_three_levels" "m2_" 2301000;
    3315 flat_with_a_constant_depth_keeps_its_exact_type "flat_constant_depths_keep_exact_types" "m3_" 2302000;
}

// M9-TS-3316
//
// `xs.flat(depth) as number[]` with a depth that does not reach the numbers: bun returns the
// nested arrays under a wrong type; here the cast names the problem with a TypeError (documented
// in docs/iron-ts.md, so no bun comparison).
#[test]
fn a_cast_of_a_flat_result_to_a_member_type_throws_when_the_depth_is_too_shallow() {
    let got = run(
        case(CASES, "flat_cast_with_a_too_shallow_depth"),
        "m4_",
        2_303_000,
    )
    .unwrap();
    assert!(
        got.starts_with("threw:") && got.contains("flat depth"),
        "{got}"
    );
}
