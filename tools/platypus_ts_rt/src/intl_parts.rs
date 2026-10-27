// SPDX-License-Identifier: Apache-2.0

//! `formatToParts`: the pieces of a formatted string and what each is. The result crosses the
//! runtime boundary as one string: `type \u{2} value` records joined by `\u{1}`.

use std::fmt;
use writeable::{Part, PartsWrite};

/// A sink that records which part wrote which bytes.
#[derive(Default)]
pub(crate) struct Collect {
    text: String,
    spans: Vec<(usize, usize, &'static str)>,
}

impl fmt::Write for Collect {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.text.push_str(s);
        Ok(())
    }
}

impl PartsWrite for Collect {
    type SubPartsWrite = Collect;

    fn with_part(
        &mut self,
        part: Part,
        mut f: impl FnMut(&mut Self::SubPartsWrite) -> fmt::Result,
    ) -> fmt::Result {
        let start = self.text.len();
        f(self)?;
        self.spans.push((start, self.text.len(), part.value));
        Ok(())
    }
}

impl Collect {
    /// The text and its parts: bytes no part claims are `literal`; the innermost part wins.
    pub(crate) fn into_parts(self, fix: impl Fn(&str) -> String) -> Vec<(String, String)> {
        let mut label: Vec<&str> = vec!["literal"; self.text.len()];
        let mut spans = self.spans;
        spans.sort_by_key(|(a, b, _)| std::cmp::Reverse(b - a));
        for (a, b, name) in spans {
            for l in &mut label[a..b] {
                *l = name;
            }
        }
        let mut out: Vec<(String, String)> = Vec::new();
        let mut start = 0;
        for i in 1..=self.text.len() {
            let boundary =
                i == self.text.len() || (self.text.is_char_boundary(i) && label[i] != label[start]);
            if boundary && self.text.is_char_boundary(i) {
                out.push((label[start].to_string(), fix(&self.text[start..i])));
                start = i;
            }
        }
        out
    }
}

/// `type \u{2} value` records joined by `\u{1}`.
pub(crate) fn encode(parts: &[(String, String)]) -> String {
    parts
        .iter()
        .map(|(t, v)| format!("{t}\u{2}{v}"))
        .collect::<Vec<_>>()
        .join("\u{1}")
}
