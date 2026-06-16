//! Shared command-line handling for mudshark commands.
//!
//! Every command parses the same output flags and emits output the same way,
//! so behaviour stays identical across the suite.

use std::io::{self, Write};

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
///
/// Output goes through [`write_stdout`], which tolerates a closed reader (e.g.
/// `... | head`) by exiting cleanly instead of panicking on a broken pipe.
pub fn emit(format: Format, json: impl FnOnce() -> Json, table: impl FnOnce() -> String) {
    let text = match format {
        // table::render already terminates each row with a newline.
        Format::Table => table(),
        Format::Json => format!("{}\n", json().to_pretty_string()),
        Format::JsonCompact => format!("{}\n", json().to_compact_string()),
    };
    write_stdout(&text);
}

/// Write `text` to stdout, exiting 0 on a broken pipe and 1 on other I/O errors.
fn write_stdout(text: &str) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let wrote = handle.write_all(text.as_bytes()).and_then(|_| handle.flush());
    if let Err(e) = wrote {
        if e.kind() == io::ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        eprintln!("mudshark: write error: {e}");
        std::process::exit(1);
    }
}
