// SPDX-License-Identifier: Apache-2.0

//! Which program this is. The same source is built as three binaries, told apart by their name
//! at compile time (as `editor::ENABLED` does): `platypus-game` (games: the full engine), `platypus-app`
//! (apps that are only UI: a lighter renderer, no 3D world) and `platypus_editor`.

/// Whether this program is `platypus-app`.
pub(crate) const IS_UI_APP: bool =
    super::editor::same(env!("CARGO_BIN_NAME").as_bytes(), b"platypus-app");
