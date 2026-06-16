//! get-service — systemd service units.
//! Source: `systemctl list-units --type=service --all --plain --no-legend`.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct Service {
    unit: String,
    load: String,
    active: String,
    sub: String,
    description: String,
}

/// Query service units via `systemctl`, parsing fixed columns. The first four
/// columns are single tokens; `description` is the remainder of the line.
fn get_services() -> Result<Vec<Service>, String> {
    let text = proc::run(
        "systemctl",
        &[
            "list-units",
            "--type=service",
            "--all",
            "--plain",
            "--no-legend",
        ],
    )?;
    let mut services = Vec::new();
    for line in text.lines() {
        // UNIT LOAD ACTIVE SUB DESCRIPTION (description may contain spaces).
        // The first four columns are single tokens; description is the rest.
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 4 {
            continue;
        }
        let description = remainder_after_tokens(line, 4);
        services.push(Service {
            unit: tokens[0].to_string(),
            load: tokens[1].to_string(),
            active: tokens[2].to_string(),
            sub: tokens[3].to_string(),
            description,
        });
    }
    Ok(services)
}

/// Return the substring of `line` that follows the first `n` whitespace-
/// separated tokens, with leading whitespace trimmed.
fn remainder_after_tokens(line: &str, n: usize) -> String {
    let mut count = 0;
    let mut in_token = false;
    for (idx, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if in_token {
                in_token = false;
                count += 1;
                if count == n {
                    return line[idx..].trim_start().to_string();
                }
            }
        } else {
            in_token = true;
        }
    }
    String::new()
}

/// Build the structured JSON value: `{ timestamp, services: [...] }`.
fn to_json(services: &[Service]) -> Json {
    let items = services
        .iter()
        .map(|s| {
            Json::Object(vec![
                ("unit".into(), s.unit.clone().into()),
                ("load".into(), s.load.clone().into()),
                ("active".into(), s.active.clone().into()),
                ("sub".into(), s.sub.clone().into()),
                ("description".into(), s.description.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("services".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(services: &[Service]) -> String {
    let rows: Vec<Vec<String>> = services
        .iter()
        .map(|s| {
            vec![
                s.unit.clone(),
                s.load.clone(),
                s.active.clone(),
                s.sub.clone(),
                s.description.clone(),
            ]
        })
        .collect();

    table::render(
        &["Unit", "Load", "Active", "Sub", "Description"],
        &rows,
        &[
            Align::Left,
            Align::Left,
            Align::Left,
            Align::Left,
            Align::Left,
        ],
    )
}

const HELP: &str = "\
get-service — systemd service units, table or JSON.
Usage: get-service [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: systemctl list-units --type=service --all --plain --no-legend.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-service: {e}");
            eprintln!("try 'get-service --help'");
            std::process::exit(2);
        }
    };

    let services = match get_services() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("get-service: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&services), || to_table(&services));
}
