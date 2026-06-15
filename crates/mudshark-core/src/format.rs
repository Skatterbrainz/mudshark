//! Output format selection shared by every command.

/// Human table or machine-readable JSON.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Table,
    Json,
}

impl Format {
    /// Parse an `--output` value (`table` or `json`).
    pub fn parse(value: &str) -> Result<Format, String> {
        match value {
            "table" => Ok(Format::Table),
            "json" => Ok(Format::Json),
            other => Err(format!("invalid format: '{other}' (want json|table)")),
        }
    }
}
