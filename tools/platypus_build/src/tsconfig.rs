// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! The default `tsconfig.json` of a game: self-contained (no `extends` outside the game), what
//! `platypus-build init` writes, what every example has, and what the build's `tsc` check uses
//! for a game without one (`tsc_config`). The compiler itself reads none of it: only editors and
//! the optional `tsc` check do.

use serde_json::Value;

/// The file, written as people read it (ES2022, strict, modules resolved as a bundler would: bare
/// imports are files of `src/modules/`, the compiler's rule; JSX for the React UI; no emit), with
/// `src` and the engine's declarations (`types/platypus.d.ts`, written by `platypus-build types`
/// and `init`) for editors.
const TEXT: &str = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "types": [],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "moduleDetection": "force",
    "allowImportingTsExtensions": true,
    "noEmit": true,
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "skipLibCheck": true,
    "jsx": "react-jsx",
    "paths": { "*": ["./src/modules/*"] }
  },
  "include": ["src"],
  "files": ["types/platypus.d.ts"]
}
"#;

/// The whole file as JSON.
pub fn default_value() -> Value {
    serde_json::from_str(TEXT).unwrap_or(Value::Null)
}

/// Its compiler options.
pub fn compiler_options() -> Value {
    default_value()["compilerOptions"].clone()
}

/// The text `init` writes.
pub fn default_text() -> String {
    TEXT.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // M9-TSC-01
    #[test]
    fn the_default_tsconfig_is_self_contained_and_strict() {
        let v: Value = serde_json::from_str(&default_text()).unwrap();
        assert!(v.get("extends").is_none(), "no extends outside the game");
        let o = &v["compilerOptions"];
        for k in [
            "strict",
            "noUncheckedIndexedAccess",
            "noImplicitOverride",
            "isolatedModules",
            "verbatimModuleSyntax",
        ] {
            assert_eq!(o[k], true, "{k}");
        }
        assert_eq!(o["target"], "ES2022");
        assert_eq!(o["lib"], json!(["ES2022"]));
        assert_eq!(o["moduleResolution"], "bundler");
        assert_eq!(o["jsx"], "react-jsx");
        assert_eq!(o["paths"]["*"], json!(["./src/modules/*"]));
        assert_eq!(v["include"], json!(["src"]));
        assert_eq!(v["files"], json!(["types/platypus.d.ts"]));
    }
}
