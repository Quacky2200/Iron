// SPDX-License-Identifier: Apache-2.0

//! Objects indexed by a key (`state[key] = !state[key]`, `TINTS[tint]`) and `Record<string, V>` as a
//! dictionary (`dict[k]`, `dict[k] = v`, a record as a dictionary). The programs of
//! `bench/tables_cases.ts` run in the normal, stress GC and collection-off modes and are compared
//! with bun (`bench/gen_cases_expected.ts`). IDs are in TESTS.md (M9-TS-3165..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/tables_cases.ts");

fn expected(name: &str) -> String {
    let v: Value = serde_json::from_str(include_str!("../../bench/tables_expected.json")).unwrap();
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
    3165 keyed_writes "keyed_writes" "tb0_" 1000000;
    3166 tables_by_tag "tables_by_tag" "tb1_" 1001000;
    3167 string_dictionary "string_dictionary" "tb2_" 1002000;
    3168 record_struct_as_dictionary "record_struct_as_dictionary" "tb3_" 1003000;
    3169 keys_that_name_some_fields "keys_that_name_some_fields" "tb4_" 1004000;
    3170 types_from_values_and_small_forms "types_from_values_and_small_forms" "tb5_" 1005000;
}
