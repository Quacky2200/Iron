// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Language features the original (QuickJS-era) example sources need: generic inference from a
//! closure that returns nothing, unions of different types, `ReturnType<typeof f<T>>` and the
//! other utility types. The programs of `bench/originals_cases.ts` run in the normal, stress GC
//! and collection-off modes and are compared with bun (`bench/gen_cases_expected.ts`). IDs are
//! in TESTS.md (M9-TS-3174..).
use crate::cases::{case, run};
use serde_json::Value;

const CASES: &str = include_str!("../../bench/originals_cases.ts");

fn expected(name: &str) -> String {
    let v: Value =
        serde_json::from_str(include_str!("../../bench/originals_expected.json")).unwrap();
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
    3174 generic_void_from_closure_body "generic_void_from_closure_body" "og0_" 2000000;
    3175 tagged_unions "tagged_unions" "og1_" 2001000;
    3176 callback_return_annotation "callback_return_annotation" "og2_" 2002000;
    3177 return_type_and_utility_types "return_type_and_utility_types" "og3_" 2003000;
    3178 typed_array_constructor_and_local_generic_arrow "typed_array_constructor_and_local_generic_arrow" "og4_" 2004000;
    3179 self_referential_const_object "self_referential_const_object" "og5_" 2005000;
    3180 object_literal_getter "object_literal_getter" "og6_" 2006000;
    3181 string_and_object_logical_and "string_and_object_logical_and" "og7_" 2007000;
    3182 large_record_as_dictionary "large_record_as_dictionary" "og8_" 2008000;
    3183 strings_become_union_members "strings_become_union_members" "og9_" 2009000;
}
