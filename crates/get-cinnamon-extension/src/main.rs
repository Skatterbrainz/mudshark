//! get-cinnamon-extension — enabled Cinnamon extensions.
//! Source: `gsettings get org.cinnamon enabled-extensions`, whose value is a
//! GVariant string-list literal (often `@as []` when none are enabled).
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

/// Extract the contents of single-quoted entries from a GVariant list literal.
/// `@as []` (the empty typed array) yields no entries.
fn parse_quoted_entries(value: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            let mut s = String::new();
            for c2 in chars.by_ref() {
                if c2 == '\'' {
                    break;
                }
                s.push(c2);
            }
            entries.push(s);
        }
    }
    entries
}

/// Run gsettings and parse the enabled-extensions list (empty if none).
fn get_extensions() -> Result<Vec<String>, String> {
    let out = proc::run("gsettings", &["get", "org.cinnamon", "enabled-extensions"])?;
    Ok(parse_quoted_entries(out.trim()))
}

/// Build the structured JSON value: `{ timestamp, extensions: [...] }`.
fn to_json(extensions: &[String]) -> Json {
    let items = extensions
        .iter()
        .map(|uuid| Json::Object(vec![("uuid".into(), uuid.clone().into())]))
        .collect();
    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("extensions".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table.
fn to_table(extensions: &[String]) -> String {
    let rows: Vec<Vec<String>> = extensions.iter().map(|u| vec![u.clone()]).collect();
    table::render(&["UUID"], &rows, &[Align::Left])
}

const HELP: &str = "\
get-cinnamon-extension — enabled Cinnamon extensions, table or JSON.
Usage: get-cinnamon-extension [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: gsettings get org.cinnamon enabled-extensions.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-cinnamon-extension: {e}");
            eprintln!("try 'get-cinnamon-extension --help'");
            std::process::exit(2);
        }
    };

    let extensions = match get_extensions() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("get-cinnamon-extension: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(
        opts.format,
        || to_json(&extensions),
        || to_table(&extensions),
    );
}
