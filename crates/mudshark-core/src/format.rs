//! Output format selection shared by every command.

/// Human table, pretty JSON, or compact (single-line) JSON.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Table,
    Json,
    JsonCompact,
}

impl Format {
    /// Parse an `--output` value (`table`, `json`, or `json-compact`).
    pub fn parse(value: &str) -> Result<Format, String> {
        match value {
            "table" => Ok(Format::Table),
            "json" => Ok(Format::Json),
            "json-compact" => Ok(Format::JsonCompact),
            other => Err(format!(
                "invalid format: '{other}' (want table|json|json-compact)"
            )),
        }
    }
}
