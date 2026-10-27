// SPDX-License-Identifier: Apache-2.0

//! Native `#[platypus_api]` namespaces in the QuickJS runtime. IDs are in TESTS.md (M9-API-*).
use platypus_api::{Handle, platypus_api};
use platypus_script::{QuickJsRuntime, ScriptRuntime, Value};

#[derive(Clone, Copy)]
pub struct BoxId(pub u64);
impl Handle for BoxId {
    fn to_raw(self) -> u64 {
        self.0
    }
    fn from_raw(raw: u64) -> Self {
        BoxId(raw)
    }
}

#[platypus_api(namespace = "Box", quickjs, handles(Box = BoxId))]
pub mod box_api {
    use super::BoxId;

    impl BoxId {
        pub fn create(size: f64) -> BoxId {
            BoxId(size as u64)
        }

        #[property]
        pub fn size(self) -> f64 {
            self.0 as f64
        }
    }
}

// M9-API-113
//
// `add_native` installs the API now and again after every `reset()` (a hot reload makes a new
// context), and a script loaded afterwards sees the handle class.
#[test]
fn native_apis_survive_a_runtime_reset() {
    let mut rt = QuickJsRuntime::new();
    rt.add_native(box_api::register_quickjs).unwrap();
    let src = "JSON.stringify([new Box(3).size, Box.create(5).size, String(Box.create(1))])";
    let want = Value::String(r#"[3,5,"[object Box]"]"#.into());
    assert_eq!(rt.eval(src).unwrap(), want);
    rt.reset();
    assert_eq!(rt.eval(src).unwrap(), want);
}
