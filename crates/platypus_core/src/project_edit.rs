// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Editing `project.json` as text (the editor's Project Settings): one key (top-level, or a dotted
//! path into the `layers`, `quality` and `display` sections) is set, added or removed by changing
//! only its own span, so everything else in the file (unknown keys, key order, `${templates}`,
//! spacing, the platform sections) stays as written. [`FIELDS`] lists the keys the settings form
//! edits, beside the loader that reads them ([`super::project`]).

use serde_json::Value;

/// A key of `project.json` as the Project Settings form shows it.
pub struct EditField {
    pub key: &'static str,
    /// `string`, `bool`, `number`, `enum` (options) or `path` (a project file).
    pub ty: &'static str,
    pub group: &'static str,
    pub options: &'static [&'static str],
    pub doc: &'static str,
}

const fn field(
    key: &'static str,
    ty: &'static str,
    group: &'static str,
    doc: &'static str,
) -> EditField {
    EditField {
        key,
        ty,
        group,
        options: &[],
        doc,
    }
}

/// The keys the settings form edits, in the form's order. Templates (`${firstName}`) are kept as
/// written; the loader resolves them.
pub const FIELDS: &[EditField] = &[
    field(
        "name",
        "string",
        "Identity",
        "The project's name (required).",
    ),
    field("title", "string", "Identity", "The window and store title."),
    field(
        "subtitle",
        "string",
        "Identity",
        "A second line under the title.",
    ),
    field("company", "string", "Identity", "Who makes it."),
    field(
        "identifier",
        "string",
        "Identity",
        "Reverse-domain id, e.g. com.example.game.",
    ),
    field(
        "version",
        "string",
        "Identity",
        "The version shown and packaged.",
    ),
    field(
        "description",
        "string",
        "Identity",
        "One sentence about the game.",
    ),
    field("copyright", "string", "Identity", "The copyright line."),
    field(
        "license",
        "string",
        "Identity",
        "`private`, `custom` or an SPDX identifier.",
    ),
    field("licenseFile", "path", "Identity", "The licence text file."),
    field(
        "startScene",
        "path",
        "Game",
        "The scene the game opens with.",
    ),
    field(
        "main",
        "path",
        "Game",
        "The entry file (default src/main.ts or .tsx).",
    ),
    EditField {
        key: "renderer",
        ty: "enum",
        group: "Game",
        options: &["", "vulkan", "gles", "cpu"],
        doc: "Left out: chosen from what the program uses.",
    },
    field("fps", "bool", "Game", "Show the FPS counter."),
    field(
        "runInBackground",
        "bool",
        "Game",
        "Keep running while the window is in the background.",
    ),
    field(
        "videoSettingsPrompt",
        "bool",
        "Game",
        "Ask for the video settings before the window opens.",
    ),
    field(
        "exe",
        "string",
        "Build",
        "The executable's file name, without extension.",
    ),
    field("icon", "path", "Build", "The window and exe icon."),
    field(
        "dataDir",
        "string",
        "Build",
        "The data folder beside a packaged exe.",
    ),
    field(
        "prebuild",
        "string",
        "Build",
        "A command run before compiling.",
    ),
    EditField {
        key: "display.mode",
        ty: "enum",
        group: "Display",
        options: &[
            "",
            "Windowed",
            "FullScreenWindow",
            "ExclusiveFullScreen",
            "MaximizedWindow",
        ],
        doc: "How the window opens the first time (later, the player's choice).",
    },
    field(
        "display.width",
        "number",
        "Display",
        "The window's width at first.",
    ),
    field(
        "display.height",
        "number",
        "Display",
        "The window's height at first.",
    ),
    field(
        "display.refreshRate",
        "number",
        "Display",
        "Exclusive full screen's refresh rate (Hz).",
    ),
    field(
        "display.resizable",
        "bool",
        "Display",
        "Whether the player can resize or maximize the window.",
    ),
    EditField {
        key: "quality.default",
        ty: "enum",
        group: "Quality",
        options: &["", "Low", "Medium", "High", "Ultra"],
        doc: "The quality level a new player starts at.",
    },
    layer(3),
    layer(6),
    layer(7),
    layer(8),
    layer(9),
    layer(10),
    layer(11),
    layer(12),
    layer(13),
    layer(14),
    layer(15),
    layer(16),
    layer(17),
    layer(18),
    layer(19),
    layer(20),
    layer(21),
    layer(22),
    layer(23),
    layer(24),
    layer(25),
    layer(26),
    layer(27),
    layer(28),
    layer(29),
    layer(30),
    layer(31),
];

/// The keys of the layers a project can name (`layers.3`, ...): Unity's user layers.
const LAYER_KEYS: [&str; 32] = [
    "layers.0",
    "layers.1",
    "layers.2",
    "layers.3",
    "layers.4",
    "layers.5",
    "layers.6",
    "layers.7",
    "layers.8",
    "layers.9",
    "layers.10",
    "layers.11",
    "layers.12",
    "layers.13",
    "layers.14",
    "layers.15",
    "layers.16",
    "layers.17",
    "layers.18",
    "layers.19",
    "layers.20",
    "layers.21",
    "layers.22",
    "layers.23",
    "layers.24",
    "layers.25",
    "layers.26",
    "layers.27",
    "layers.28",
    "layers.29",
    "layers.30",
    "layers.31",
];

/// A user layer's name setting.
const fn layer(n: usize) -> EditField {
    field(
        LAYER_KEYS[n],
        "string",
        "Layers",
        "The layer's name (empty: unnamed).",
    )
}

/// Where a top-level member of a JSON object is: the key's string, its value, and the separator
/// span after it (a comma) if any.
#[derive(Clone, Debug)]
struct Member {
    key: String,
    /// From the key's opening quote to the end of its value.
    start: usize,
    value: (usize, usize),
    /// The comma after the value, if any.
    comma: Option<usize>,
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// The end of the string starting at `i` (its opening quote).
fn skip_string(b: &[u8], mut i: usize) -> Result<usize, String> {
    i += 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Ok(i + 1),
            _ => i += 1,
        }
    }
    Err("a string isn't closed".into())
}

/// The end of the value starting at `i`.
fn skip_value(b: &[u8], i: usize) -> Result<usize, String> {
    match b.get(i) {
        Some(b'"') => skip_string(b, i),
        Some(b'{' | b'[') => {
            let mut depth = 0usize;
            let mut j = i;
            while j < b.len() {
                match b[j] {
                    b'"' => {
                        j = skip_string(b, j)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(j + 1);
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            Err("an object or array isn't closed".into())
        }
        Some(_) => {
            let mut j = i;
            while j < b.len() && !matches!(b[j], b',' | b'}' | b']') && !b[j].is_ascii_whitespace()
            {
                j += 1;
            }
            Ok(j)
        }
        None => Err("a value is missing".into()),
    }
}

/// The top-level members of the object in `text`, and where its closing brace is.
fn members(text: &str) -> Result<(Vec<Member>, usize), String> {
    let b = text.as_bytes();
    let mut i = skip_ws(b, 0);
    if b.get(i) != Some(&b'{') {
        return Err("project.json isn't a JSON object".into());
    }
    i += 1;
    let mut out = Vec::new();
    loop {
        i = skip_ws(b, i);
        match b.get(i) {
            Some(b'}') => return Ok((out, i)),
            Some(b'"') => {}
            _ => return Err(format!("unexpected text at byte {i}")),
        }
        let start = i;
        let key_end = skip_string(b, i)?;
        let key: String = serde_json::from_str(&text[i..key_end]).map_err(|e| e.to_string())?;
        i = skip_ws(b, key_end);
        if b.get(i) != Some(&b':') {
            return Err(format!("`:` missing after {key:?}"));
        }
        i = skip_ws(b, i + 1);
        let v_end = skip_value(b, i)?;
        let value = (i, v_end);
        i = skip_ws(b, v_end);
        let comma = (b.get(i) == Some(&b',')).then_some(i);
        if comma.is_some() {
            i += 1;
        }
        out.push(Member {
            key,
            start,
            value,
            comma,
        });
    }
}

/// The raw value of a key (JSON text, as written), if the file has it. A dotted key
/// (`display.width`, `layers.8`) names a member of nested objects.
pub fn get_key(text: &str, key: &str) -> Result<Option<String>, String> {
    let (first, rest) = split_key(key);
    let (ms, _) = members(text)?;
    let Some(m) = ms.iter().find(|m| m.key == first) else {
        return Ok(None);
    };
    let value = &text[m.value.0..m.value.1];
    match rest {
        None => Ok(Some(value.to_owned())),
        Some(rest) if value.starts_with('{') => get_key(value, rest),
        Some(_) => Ok(None),
    }
}

/// A dotted key's first part and the rest, if any.
fn split_key(key: &str) -> (&str, Option<&str>) {
    match key.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (key, None),
    }
}

/// `value` under the dotted path `rest` (`a.b` -> `{"a": {"b": value}}`).
fn nested(rest: &str, value: &Value) -> Value {
    rest.rsplit('.').fold(value.clone(), |inner, k| {
        let mut m = serde_json::Map::new();
        m.insert(k.to_owned(), inner);
        Value::Object(m)
    })
}

/// The indentation of the first member (two spaces when the object is empty or on one line).
fn indent_of(text: &str, ms: &[Member]) -> String {
    ms.first()
        .and_then(|m| {
            let line_start = text[..m.start].rfind('\n')? + 1;
            Some(text[line_start..m.start].to_owned())
        })
        .unwrap_or_else(|| "  ".into())
}

/// Sets a key to `value`: its value is replaced where it is, or the key is added last. A dotted
/// key is set inside its nested object (made when missing). Nothing else in the text changes.
pub fn set_key(text: &str, key: &str, value: &Value) -> Result<String, String> {
    let (first, rest) = split_key(key);
    let Some(rest) = rest else {
        return set_member(text, key, value);
    };
    let (ms, _) = members(text)?;
    match ms.iter().find(|m| m.key == first) {
        Some(m) if text[m.value.0..].starts_with('{') => {
            let inner = set_key(&text[m.value.0..m.value.1], rest, value)?;
            Ok(format!(
                "{}{inner}{}",
                &text[..m.value.0],
                &text[m.value.1..]
            ))
        }
        Some(_) => Err(format!("`{first}` isn't an object")),
        None => set_member(text, first, &nested(rest, value)),
    }
}

/// Sets a member of the object in `text` (see [`set_key`]).
fn set_member(text: &str, key: &str, value: &Value) -> Result<String, String> {
    let (ms, close) = members(text)?;
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    if let Some(m) = ms.iter().find(|m| m.key == key) {
        return Ok(format!(
            "{}{json}{}",
            &text[..m.value.0],
            &text[m.value.1..]
        ));
    }
    let key_json = serde_json::to_string(key).map_err(|e| e.to_string())?;
    let indent = indent_of(text, &ms);
    match ms.last() {
        Some(last) => {
            // After the last value: `,` then the new member on its own line.
            let at = last.value.1;
            let newline =
                if text[last.start..close].contains('\n') || text[..last.start].contains('\n') {
                    format!("\n{indent}")
                } else {
                    " ".into()
                };
            let rest = match last.comma {
                // A trailing comma (not JSON, but keep it working): the new member goes after it.
                Some(c) => format!(
                    "{}{newline}{key_json}: {json}{}",
                    &text[at..=c],
                    &text[c + 1..]
                ),
                None => format!(",{newline}{key_json}: {json}{}", &text[at..]),
            };
            Ok(format!("{}{rest}", &text[..at]))
        }
        None => Ok(format!(
            "{}\n{indent}{key_json}: {json}\n{}",
            &text[..close],
            &text[close..]
        )),
    }
}

/// Removes a key (and its comma); nothing else changes. Unchanged when it isn't there. A dotted
/// key is removed from its nested object, and an object left empty goes too.
pub fn remove_key(text: &str, key: &str) -> Result<String, String> {
    let (first, rest) = split_key(key);
    let Some(rest) = rest else {
        return remove_member(text, key);
    };
    let (ms, _) = members(text)?;
    let Some(m) = ms.iter().find(|m| m.key == first) else {
        return Ok(text.to_owned());
    };
    let value = &text[m.value.0..m.value.1];
    if !value.starts_with('{') {
        return Ok(text.to_owned());
    }
    let inner = remove_key(value, rest)?;
    if members(&inner)?.0.is_empty() {
        return remove_member(text, first);
    }
    Ok(format!(
        "{}{inner}{}",
        &text[..m.value.0],
        &text[m.value.1..]
    ))
}

/// Removes a member of the object in `text` (see [`remove_key`]).
fn remove_member(text: &str, key: &str) -> Result<String, String> {
    let (ms, _) = members(text)?;
    let Some(i) = ms.iter().position(|m| m.key == key) else {
        return Ok(text.to_owned());
    };
    let m = &ms[i];
    // From the end of the previous member's value (or the line start) to the end of this one.
    let (from, to) = match (i.checked_sub(1).map(|p| &ms[p]), m.comma) {
        // Not the first: drop the comma before it and the member.
        (Some(prev), None) => (prev.value.1, m.value.1),
        (Some(_), Some(c)) | (None, Some(c)) => {
            let line_start = text[..m.start].rfind('\n').map_or(m.start, |n| n + 1);
            let start = if text[line_start..m.start].trim().is_empty() {
                line_start
            } else {
                m.start
            };
            let mut end = c + 1;
            // The rest of the line too, when the member had it to itself.
            let after = &text[end..];
            if let Some(n) = after.find('\n')
                && after[..n].trim().is_empty()
                && start == line_start
            {
                end += n + 1;
            }
            (start, end)
        }
        (None, None) => (m.start, m.value.1),
    };
    Ok(format!("{}{}", &text[..from], &text[to..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "{\n  \"name\": \"${firstName}'s game\",\n  \"zeta\": { \"keep\": [1, 2] },\n  \"title\": \"Old\",\n  \"windows\": { \"exe\": \"g\" }\n}\n";

    // M5-PRJ-01
    #[test]
    fn a_key_is_set_in_place_and_everything_else_is_kept() {
        let out = set_key(FILE, "title", &Value::from("New")).unwrap();
        assert_eq!(out, FILE.replace("\"Old\"", "\"New\""));
        assert!(out.contains("${firstName}'s game"), "templates kept");
        assert!(
            out.contains("\"zeta\": { \"keep\": [1, 2] }"),
            "unknown keys kept"
        );
    }

    // M5-PRJ-02
    #[test]
    fn new_keys_go_last_with_the_files_indentation() {
        let out = set_key(FILE, "fps", &Value::Bool(false)).unwrap();
        assert!(
            out.ends_with("\"windows\": { \"exe\": \"g\" },\n  \"fps\": false\n}\n"),
            "{out}"
        );
        let parsed: Value = serde_json::from_str(&out).unwrap();
        let keys: Vec<&String> = parsed.as_object().unwrap().keys().collect();
        assert_eq!(keys.len(), 5);
        let one = set_key("{\"name\":\"a\"}", "title", &Value::from("t")).unwrap();
        assert_eq!(one, "{\"name\":\"a\", \"title\": \"t\"}");
        let empty = set_key("{}", "name", &Value::from("n")).unwrap();
        assert_eq!(empty, "{\n  \"name\": \"n\"\n}");
    }

    // M5-PRJ-03
    #[test]
    fn keys_are_removed_with_their_comma_and_line() {
        let out = remove_key(FILE, "zeta").unwrap();
        assert_eq!(out, FILE.replace("  \"zeta\": { \"keep\": [1, 2] },\n", ""));
        let last = remove_key(FILE, "windows").unwrap();
        assert!(last.ends_with("\"title\": \"Old\"\n}\n"), "{last}");
        serde_json::from_str::<Value>(&last).unwrap();
        let first = remove_key(FILE, "name").unwrap();
        assert!(first.starts_with("{\n  \"zeta\""), "{first}");
        assert_eq!(remove_key(FILE, "nope").unwrap(), FILE);
        assert_eq!(
            get_key(FILE, "zeta").unwrap().as_deref(),
            Some("{ \"keep\": [1, 2] }")
        );
        assert!(set_key("[1]", "a", &Value::Null).is_err(), "not an object");
        assert!(set_key("{\"a\": \"x", "a", &Value::Null).is_err(), "broken");
    }

    // M5-PRJ-06
    #[test]
    fn dotted_keys_edit_nested_sections_in_place() {
        let file = "{\n  \"name\": \"${user}\",\n  \"layers\": { \"8\": \"Detail\" },\n  \"display\": {\n    \"width\": 1280,\n    \"height\": 760\n  },\n  \"zz\": 1\n}\n";
        assert_eq!(
            get_key(file, "display.width").unwrap().as_deref(),
            Some("1280")
        );
        assert_eq!(get_key(file, "layers.9").unwrap(), None);
        assert_eq!(get_key(file, "name.x").unwrap(), None, "not an object");
        // Replaced in place; added inside on the object's own line style.
        let w = set_key(file, "display.width", &Value::from(1600)).unwrap();
        assert_eq!(w, file.replace("1280", "1600"));
        let l = set_key(file, "layers.9", &Value::from("Player")).unwrap();
        assert!(
            l.contains("\"layers\": { \"8\": \"Detail\", \"9\": \"Player\" }"),
            "{l}"
        );
        let r = set_key(file, "display.resizable", &Value::Bool(false)).unwrap();
        assert!(
            r.contains("\"height\": 760,\n    \"resizable\": false\n  }"),
            "{r}"
        );
        // A missing section is made, last.
        let q = set_key(file, "quality.default", &Value::from("High")).unwrap();
        assert!(
            q.ends_with("\"zz\": 1,\n  \"quality\": {\"default\":\"High\"}\n}\n"),
            "{q}"
        );
        serde_json::from_str::<Value>(&q).unwrap();
        // Removed from the section; a section left empty goes too.
        let gone = remove_key(file, "layers.8").unwrap();
        assert!(!gone.contains("layers"), "{gone}");
        serde_json::from_str::<Value>(&gone).unwrap();
        let h = remove_key(file, "display.height").unwrap();
        assert!(
            h.contains("\"display\": {\n    \"width\": 1280\n  }"),
            "{h}"
        );
        assert_eq!(remove_key(file, "quality.default").unwrap(), file);
        assert!(set_key(file, "name.x", &Value::Null).is_err());
    }
}
