//! Which program this is. The same source is built as three binaries, told apart by their name
//! at compile time (as `editor::ENABLED` does): `iron-game` (games: the full engine), `iron-app`
//! (apps that are only UI: a lighter renderer, no 3D world) and `iron_editor`.

/// Whether this program is `iron-app`.
pub(crate) const IS_UI_APP: bool =
    super::editor::same(env!("CARGO_BIN_NAME").as_bytes(), b"iron-app");
