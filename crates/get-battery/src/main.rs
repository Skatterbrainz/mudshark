//! get-battery — battery status and charge.
//! Source: /sys/class/power_supply/*, including only entries whose `type` file
//! reads `Battery`. Fields are pulled from each device's `uevent`. Emits an
//! array `batteries`; if there are none (e.g. a desktop), the array is empty
//! and the command exits 0.
//!
//! NOTE: `energy_full` / `energy_now` are microwatt-hours (µWh), and `capacity`
//! is a percentage — these are NOT byte counts, so they stay raw integers and
//! must never be passed through `bytes::human`.

use std::collections::HashMap;
use std::fs;

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, time};

struct Battery {
    name: String,
    status: String,
    capacity: Option<u64>,
    energy_full: Option<u64>, // µWh
    energy_now: Option<u64>,  // µWh
    manufacturer: String,
    model_name: String,
}

/// Parse a `uevent` file (one `KEY=value` pair per line) into a map.
fn parse_uevent(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

/// Read every `type == Battery` entry under /sys/class/power_supply.
///
/// A missing directory simply means no batteries are present, so an empty
/// list (not an error) is returned.
fn get_batteries() -> Vec<Battery> {
    let mut batteries = Vec::new();
    let entries = match fs::read_dir("/sys/class/power_supply") {
        Ok(e) => e,
        Err(_) => return batteries,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // Only include power supplies that are batteries.
        match fs::read_to_string(path.join("type")) {
            Ok(t) if t.trim() == "Battery" => {}
            _ => continue,
        }

        let uevent = match fs::read_to_string(path.join("uevent")) {
            Ok(text) => parse_uevent(&text),
            Err(_) => HashMap::new(),
        };
        let get = |k: &str| uevent.get(k).cloned().unwrap_or_default();
        let num = |k: &str| uevent.get(k).and_then(|v| v.parse::<u64>().ok());

        let name = match uevent.get("POWER_SUPPLY_NAME") {
            Some(n) => n.clone(),
            None => entry.file_name().to_string_lossy().into_owned(),
        };

        batteries.push(Battery {
            name,
            status: get("POWER_SUPPLY_STATUS"),
            capacity: num("POWER_SUPPLY_CAPACITY"),
            energy_full: num("POWER_SUPPLY_ENERGY_FULL"),
            energy_now: num("POWER_SUPPLY_ENERGY_NOW"),
            manufacturer: get("POWER_SUPPLY_MANUFACTURER"),
            model_name: get("POWER_SUPPLY_MODEL_NAME"),
        });
    }

    batteries.sort_by(|a, b| a.name.cmp(&b.name));
    batteries
}

/// Represent an optional integer as JSON (`Null` when absent).
fn opt_u64(value: Option<u64>) -> Json {
    match value {
        Some(v) => v.into(),
        None => Json::Null,
    }
}

/// Build the structured JSON value: `{ timestamp, batteries: [...] }`.
fn to_json(batteries: &[Battery]) -> Json {
    let items = batteries
        .iter()
        .map(|b| {
            Json::Object(vec![
                ("name".into(), b.name.clone().into()),
                ("status".into(), b.status.clone().into()),
                ("capacity".into(), opt_u64(b.capacity)),
                ("energy_full".into(), opt_u64(b.energy_full)),
                ("energy_now".into(), opt_u64(b.energy_now)),
                ("manufacturer".into(), b.manufacturer.clone().into()),
                ("model_name".into(), b.model_name.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("batteries".into(), Json::Array(items)),
    ])
}

/// Render an optional integer for the table (`-` when absent).
fn cell_u64(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string())
}

/// Build the human-readable table via the shared renderer.
fn to_table(batteries: &[Battery]) -> String {
    let rows: Vec<Vec<String>> = batteries
        .iter()
        .map(|b| {
            vec![
                b.name.clone(),
                b.status.clone(),
                b.capacity
                    .map(|c| format!("{c}%"))
                    .unwrap_or_else(|| "-".to_string()),
                cell_u64(b.energy_full),
                cell_u64(b.energy_now),
                b.manufacturer.clone(),
                b.model_name.clone(),
            ]
        })
        .collect();

    table::render(
        &[
            "Name",
            "Status",
            "Capacity",
            "Energy Full (µWh)",
            "Energy Now (µWh)",
            "Manufacturer",
            "Model",
        ],
        &rows,
        &[
            Align::Left,
            Align::Left,
            Align::Right,
            Align::Right,
            Align::Right,
            Align::Left,
            Align::Left,
        ],
    )
}

const HELP: &str = "\
get-battery — battery status and charge, table or JSON.
Usage: get-battery [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: /sys/class/power_supply/* (type == Battery). energy_* are µWh.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-battery: {e}");
            eprintln!("try 'get-battery --help'");
            std::process::exit(2);
        }
    };

    let batteries = get_batteries();

    cli::emit(opts.format, || to_json(&batteries), || to_table(&batteries));
}
