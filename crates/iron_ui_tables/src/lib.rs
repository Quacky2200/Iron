//! Tables of what a script can say to the UI: style properties, element attributes, elements,
//! events and the React names. Plain data with no dependencies, read by
//!
//! - `tools/iron_ts` (`ui_dts.rs`) to print the UI half of the generated `types/iron.d.ts` and the
//!   compiled prelude (`StyleEntry`, `UiEvent`, ...), so no declaration file is written by hand;
//! - `iron_ui` and `iron_ts_ui` tests, which check every entry against the code that implements it
//!   (`apply_style`, `Attrs::set`, the event layout, the event kinds).

pub mod attrs;
pub mod elements;
pub mod events;
pub mod react;
pub mod style;
