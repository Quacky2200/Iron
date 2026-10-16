use std::fmt;

/// A script failure with as much location info as the engine provides.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptError {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub stack: Option<String>,
}

impl ScriptError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            file: None,
            line: None,
            stack: None,
        }
    }

    /// Fills `file`/`line` from the first `name:line` location in a JS stack trace,
    /// e.g. `    at onUpdate (hello.js:12:9)` or `    at hello.js:3:1`.
    pub(crate) fn with_location_from_stack(mut self) -> Self {
        if let Some(stack) = &self.stack
            && let Some((file, line)) = stack.lines().find_map(parse_location)
        {
            self.file = Some(file);
            self.line = Some(line);
        }
        self
    }
}

fn parse_location(line: &str) -> Option<(String, u32)> {
    let loc = line.trim().strip_prefix("at ")?;
    let loc = match (loc.rfind('('), loc.ends_with(')')) {
        (Some(open), true) => &loc[open + 1..loc.len() - 1],
        _ => loc,
    };
    // `file:line:col` or `file:line`; file names may contain ':' (C:\...), so split from the right.
    let mut parts = loc.rsplitn(3, ':');
    let last = parts.next()?;
    let mid = parts.next()?;
    let (file, line) = match parts.next() {
        Some(file) if mid.parse::<u32>().is_ok() => (file, mid),
        _ => (mid, last),
    };
    let line = line.parse().ok()?;
    if file.is_empty() || file == "native" {
        return None;
    }
    Some((file.to_owned(), line))
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.file, self.line) {
            (Some(file), Some(line)) => write!(f, "{file}:{line}: {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ScriptError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stack_locations() {
        assert_eq!(
            parse_location("    at onUpdate (hello.js:12:9)"),
            Some(("hello.js".into(), 12))
        );
        assert_eq!(
            parse_location("    at hello.js:3:1"),
            Some(("hello.js".into(), 3))
        );
        assert_eq!(
            parse_location("    at f (C:\\game\\main.js:7:2)"),
            Some(("C:\\game\\main.js".into(), 7))
        );
        assert_eq!(parse_location("    at <eval> (native)"), None);
        assert_eq!(parse_location("not a frame"), None);
    }
}
