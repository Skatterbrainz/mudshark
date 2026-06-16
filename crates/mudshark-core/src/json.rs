//! A tiny, dependency-free JSON value type with stable pretty-printing.
//!
//! This is the std-only stand-in for `serde_json::Value`: build a [`Json`]
//! tree and call [`Json::to_pretty_string`]. Centralising it here means every
//! command serialises — and escapes strings — identically.

/// A JSON value.
#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    U64(u64),
    I64(i64),
    F64(f64),
    Str(String),
    Array(Vec<Json>),
    /// Insertion-ordered object so field order in output is deterministic.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Serialise to a pretty-printed string (2-space indent, no trailing newline).
    pub fn to_pretty_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, indent: usize) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::U64(n) => out.push_str(&n.to_string()),
            Json::I64(n) => out.push_str(&n.to_string()),
            Json::F64(n) => out.push_str(&format_f64(*n)),
            Json::Str(s) => write_escaped(out, s),
            Json::Array(items) => {
                if items.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    indent_by(out, indent + 1);
                    item.write(out, indent + 1);
                    if i + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent_by(out, indent);
                out.push(']');
            }
            Json::Object(entries) => {
                if entries.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                for (i, (key, value)) in entries.iter().enumerate() {
                    indent_by(out, indent + 1);
                    write_escaped(out, key);
                    out.push_str(": ");
                    value.write(out, indent + 1);
                    if i + 1 < entries.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                indent_by(out, indent);
                out.push('}');
            }
        }
    }

    /// Serialise to a compact single-line string (no whitespace).
    pub fn to_compact_string(&self) -> String {
        let mut out = String::new();
        self.write_compact(&mut out);
        out
    }

    fn write_compact(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::U64(n) => out.push_str(&n.to_string()),
            Json::I64(n) => out.push_str(&n.to_string()),
            Json::F64(n) => out.push_str(&format_f64(*n)),
            Json::Str(s) => write_escaped(out, s),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            Json::Object(entries) => {
                out.push('{');
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_escaped(out, key);
                    out.push(':');
                    value.write_compact(out);
                }
                out.push('}');
            }
        }
    }
}

impl std::fmt::Display for Json {
    /// `{}` formats as compact JSON, so `value.to_string()` is compact.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_compact_string())
    }
}

fn indent_by(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn format_f64(n: f64) -> String {
    // JSON has no NaN/Infinity; emit null to keep output valid.
    if n.is_finite() {
        format!("{n}")
    } else {
        "null".to_string()
    }
}

/// Append `s` to `out` as a quoted, escaped JSON string (RFC 8259).
fn write_escaped(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

// Ergonomic constructors so commands can write `value.into()`.
impl From<u64> for Json {
    fn from(v: u64) -> Self {
        Json::U64(v)
    }
}
impl From<i64> for Json {
    fn from(v: i64) -> Self {
        Json::I64(v)
    }
}
impl From<f64> for Json {
    fn from(v: f64) -> Self {
        Json::F64(v)
    }
}
impl From<bool> for Json {
    fn from(v: bool) -> Self {
        Json::Bool(v)
    }
}
impl From<&str> for Json {
    fn from(v: &str) -> Self {
        Json::Str(v.to_string())
    }
}
impl From<String> for Json {
    fn from(v: String) -> Self {
        Json::Str(v)
    }
}
