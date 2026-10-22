//! Small conveniences: functions as values (`Util.twice`, `Math.abs`), casts of empty literals,
//! field types from constructor calls, `!` on assignment targets, closures over destructured
//! constants and spread strings. The programs of `bench/misc_cases.ts` run in the normal, stress
//! GC and collection-off modes and are compared with bun (`bench/gen_cases_expected.ts`). IDs are
//! in TESTS.md (M9-TS-3128..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/misc_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/misc_expected.json")).unwrap();
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
    3128 static_function_values "static_function_values" "mi0_" 600000;
    3129 math_in_callbacks "math_in_callbacks" "mi1_" 601000;
    3130 casts_of_empty_literals "casts_of_empty_literals" "mi2_" 602000;
    3131 fields_from_constructor_calls "fields_from_constructor_calls" "mi3_" 603000;
    3132 non_null_assignment_targets "non_null_assignment_targets" "mi4_" 604000;
    3133 closures_over_destructured "closures_over_destructured" "mi5_" 605000;
    3134 spread_strings "spread_strings" "mi6_" 606000;
    3136 empty_array_in_conditional "empty_array_in_conditional" "mi7_" 607000;
    3137 field_types_from_expressions "field_types_from_expressions" "mi8_" 608000;
    3162 top_level_destructuring "top_level_destructuring" "mi9_" 609000;
    3163 spread_into_function_values_and_constructors "spread_into_function_values_and_constructors" "mi10_" 610000;
}
