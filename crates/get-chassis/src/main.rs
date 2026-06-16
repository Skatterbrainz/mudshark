//! get-chassis — chassis / enclosure information.
//! Source: `dmidecode -t chassis`, parsing the indented `Key: Value` lines of
//! the "Chassis Information" block (Manufacturer, Type, Lock, Serial Number,
//! Asset Tag, Boot-up State, ...). dmidecode requires root, so a permission
//! failure surfaces as a clean error (exit 1). Single object output.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

/// One `Key: Value` field parsed from the dmidecode block, in source order.
struct Field {
    key: String,
    value: String,
}

/// Normalise a dmidecode-style key to a snake_case JSON key: drop parentheses,
/// lowercase, and turn any run of non-alphanumeric characters into a single
/// underscore (e.g. "Serial Number" -> "serial_number", "Boot-up State" ->
/// "boot_up_state").
fn normalize_key(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;
    for c in raw.chars() {
        if c == '(' || c == ')' {
            continue;
        }
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}

/// Collect the indented `Key: Value` lines belonging to the named top-level
/// dmidecode section (e.g. "Chassis Information").
fn parse_section(text: &str, section: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let indented = line.starts_with('\t') || line.starts_with(' ');
        if !indented {
            in_section = line.trim() == section;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((k, v)) = line.trim().split_once(':') {
            let value = v.trim();
            if value.is_empty() {
                continue; // skip section sub-headers like "Contained Elements:"
            }
            fields.push(Field {
                key: normalize_key(k.trim()),
                value: value.to_string(),
            });
        }
    }
    fields
}

/// Query chassis information via dmidecode.
fn get_chassis() -> Result<Vec<Field>, String> {
    let text = proc::run("dmidecode", &["-t", "chassis"])?;
    Ok(parse_section(&text, "Chassis Information"))
}

/// Build the structured JSON value: `{ timestamp, <field>: <value>, ... }`.
fn to_json(fields: &[Field]) -> Json {
    let mut obj = vec![("timestamp".into(), time::now_utc_iso8601().into())];
    for f in fields {
        obj.push((f.key.clone(), f.value.clone().into()));
    }
    Json::Object(obj)
}

/// Build the human-readable table (Field / Value).
fn to_table(fields: &[Field]) -> String {
    let rows: Vec<Vec<String>> = fields
        .iter()
        .map(|f| vec![f.key.clone(), f.value.clone()])
        .collect();
    table::render(&["Field", "Value"], &rows, &[Align::Left, Align::Left])
}

const HELP: &str = "\
get-chassis — chassis / enclosure information, table or JSON.
Usage: get-chassis [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: dmidecode -t chassis (requires root).";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-chassis: {e}");
            eprintln!("try 'get-chassis --help'");
            std::process::exit(2);
        }
    };

    let fields = match get_chassis() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("get-chassis: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&fields), || to_table(&fields));
}
