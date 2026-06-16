//! get-processor — CPU information.
//! Source: `lscpu`, whose output is one `Key:   Value` pair per line. Keys are
//! normalised to snake_case (e.g. "CPU(s)" -> "cpus", "Model name" ->
//! "model_name", "CPU max MHz" -> "cpu_max_mhz"). Single object output.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

/// One `Key: Value` field parsed from `lscpu`, in source order.
struct Field {
    key: String,
    value: String,
}

/// Normalise an `lscpu` key to a snake_case JSON key: drop parentheses (so
/// "CPU(s)" -> "cpus", "Thread(s) per core" -> "threads_per_core"), lowercase,
/// and turn any run of non-alphanumeric characters into a single underscore.
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

/// Query CPU information via `lscpu`, parsing each `Key: Value` line.
fn get_processor() -> Result<Vec<Field>, String> {
    let text = proc::run("lscpu", &[])?;
    let mut fields = Vec::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let value = v.trim();
            if value.is_empty() {
                continue;
            }
            fields.push(Field {
                key: normalize_key(k.trim()),
                value: value.to_string(),
            });
        }
    }
    Ok(fields)
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
get-processor — CPU information, table or JSON.
Usage: get-processor [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: lscpu.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-processor: {e}");
            eprintln!("try 'get-processor --help'");
            std::process::exit(2);
        }
    };

    let fields = match get_processor() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("get-processor: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&fields), || to_table(&fields));
}
