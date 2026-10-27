// SPDX-License-Identifier: Apache-2.0

//! Tables of what a script can say to the UI: style properties, element attributes, elements,
//! events and the React names. Plain data (the module has no dependencies on the rest of the crate), read by
//!
//! - `tools/platypus_ts` (`ui_dts.rs`) to print the UI half of the generated `types/platypus.d.ts` and the
//!   compiled prelude (`StyleEntry`, `UiEvent`, ...), so no declaration file is written by hand;
//! - `platypus_ui` and `platypus_ts_ui` tests, which check every entry against the code that implements it
//!   (`apply_style`, `Attrs::set`, the event layout, the event kinds).

pub mod attrs;
pub mod elements;
pub mod events;
pub mod react;
pub mod style;
