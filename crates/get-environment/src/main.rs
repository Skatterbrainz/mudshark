//! get-environment — process environment variables (name/value). Array output.
//! Source: std::env::vars(). Helpers from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, time};

struct EnvVar {
    name: String,
    value: String,
}

/// Collect the current process environment, sorted by name for stable output.
fn get_environment() -> Vec<EnvVar> {
    let mut vars: Vec<EnvVar> = std::env::vars()
        .map(|(name, value)| EnvVar { name, value })
        .collect();
    vars.sort_by(|a, b| a.name.cmp(&b.name));
    vars
}

/// Build the structured JSON value: `{ timestamp, variables: [...] }`.
fn to_json(vars: &[EnvVar]) -> Json {
    let items = vars
        .iter()
        .map(|v| {
            Json::Object(vec![
                ("name".into(), v.name.clone().into()),
                ("value".into(), v.value.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("variables".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(vars: &[EnvVar]) -> String {
    let rows: Vec<Vec<String>> = vars
        .iter()
        .map(|v| vec![v.name.clone(), v.value.clone()])
        .collect();
    table::render(&["Name", "Value"], &rows, &[Align::Left, Align::Left])
}

const HELP: &str = "\
get-environment — process environment variables, table or JSON.
Usage: get-environment [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: std::env::vars().";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-environment: {e}");
            eprintln!("try 'get-environment --help'");
            std::process::exit(2);
        }
    };

    let vars = get_environment();

    cli::emit(opts.format, || to_json(&vars), || to_table(&vars));
}
