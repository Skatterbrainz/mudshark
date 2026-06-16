//! Shared command-line handling for mudshark commands.
//!
//! Every command parses the same output flags and emits output the same way,
//! so behaviour stays identical across the suite.

use crate::json::Json;
use crate::Format;

/// Parsed common options.
pub struct Options {
    pub format: Format,
}

/// Parse the standard mudshark output flags from `std::env::args()`.
///
/// Recognises `--json`, `-c`/`--compact`, `-o`/`--output <table|json|json-compact>`,
/// and `-h`/`--help` (prints `help` and exits 0). Unknown flags are an error.
pub fn parse(help: &str) -> Result<Options, String> {
    let mut format = Format::Table;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => format = Format::Json,
            "-c" | "--compact" => format = Format::JsonCompact,
            "-o" | "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --output".to_string())?;
                format = Format::parse(&value)?;
            }
            "-h" | "--help" => {
                println!("{help}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Options { format })
}

/// Emit a value in the selected format. The JSON value and table string are
/// built lazily so only the representation actually needed is produced.
pub fn emit(format: Format, json: impl FnOnce() -> Json, table: impl FnOnce() -> String) {
    match format {
        Format::Table => print!("{}", table()),
        Format::Json => println!("{}", json().to_pretty_string()),
        Format::JsonCompact => println!("{}", json().to_compact_string()),
    }
}
