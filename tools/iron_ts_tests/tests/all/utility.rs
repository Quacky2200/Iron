//! `Partial` / `Required` / `Readonly` / `Pick` / `Omit` over interfaces, aliases, classes and
//! records. The programs of `bench/utility_cases.ts` run in the normal, stress GC and
//! collection-off modes and are compared with bun (`bench/gen_cases_expected.ts`). IDs are in
//! TESTS.md (M9-TS-3120..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/utility_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/utility_expected.json")).unwrap();
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
    3120 partial_default_param "partial_default_param" "ut0_" 500000;
    3121 partial_from_full_value "partial_from_full_value" "ut1_" 501000;
    3122 pick_omit "pick_omit" "ut2_" 502000;
    3123 required_and_readonly "required_and_readonly" "ut3_" 503000;
    3124 partial_of_class "partial_of_class" "ut4_" 504000;
    3125 spread_partial_in_literals "spread_partial_in_literals" "ut5_" 505000;
    3126 partial_records_and_arrays "partial_records_and_arrays" "ut6_" 506000;
}
