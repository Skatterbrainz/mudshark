//! get-bluetooth — Bluetooth controllers and their rfkill block state.
//! Sources (best-effort): `bluetoothctl list` for controllers and
//! `rfkill list bluetooth` for soft/hard block state. With no adapter (or no
//! Bluetooth stack installed) this emits an empty array and exits 0.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

/// One Bluetooth controller.
struct Controller {
    address: String,
    name: String,
    default: bool,
    /// rfkill soft-block state, if a matching rfkill entry was found.
    soft_blocked: Option<bool>,
    /// rfkill hard-block state, if a matching rfkill entry was found.
    hard_blocked: Option<bool>,
}

/// Parse `bluetoothctl list`, whose lines look like:
///   `Controller AA:BB:CC:DD:EE:FF myhost [default]`
fn parse_controllers(text: &str) -> Vec<(String, String, bool)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let rest = match line.strip_prefix("Controller ") {
            Some(r) => r,
            None => continue,
        };
        let mut parts = rest.split_whitespace();
        let address = parts.next().unwrap_or("").to_string();
        if address.is_empty() {
            continue;
        }
        let remainder: Vec<&str> = parts.collect();
        let default = remainder.iter().any(|t| *t == "[default]");
        let name = remainder
            .iter()
            .filter(|t| **t != "[default]")
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        out.push((address, name, default));
    }
    out
}

/// Parse `rfkill list bluetooth` into ordered (soft, hard) block states.
/// Each device block starts with a header line like `0: hci0: Bluetooth`.
fn parse_rfkill(text: &str) -> Vec<(bool, bool)> {
    let mut states: Vec<(bool, bool)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Soft blocked:") {
            if let Some(last) = states.last_mut() {
                last.0 = rest.trim() == "yes";
            }
        } else if let Some(rest) = line.strip_prefix("Hard blocked:") {
            if let Some(last) = states.last_mut() {
                last.1 = rest.trim() == "yes";
            }
        } else if line
            .split_once(':')
            .map(|(idx, _)| idx.chars().all(|c| c.is_ascii_digit()) && !idx.is_empty())
            .unwrap_or(false)
        {
            // New device header (e.g. "0: hci0: Bluetooth").
            states.push((false, false));
        }
    }
    states
}

/// Collect controllers (best-effort), annotating block state by position.
fn get_controllers() -> Vec<Controller> {
    let listed = match proc::run("bluetoothctl", &["list"]) {
        Ok(t) => parse_controllers(&t),
        // No adapter, daemon down, or tool absent: report none.
        Err(_) => Vec::new(),
    };
    let rfkill = proc::run("rfkill", &["list", "bluetooth"])
        .map(|t| parse_rfkill(&t))
        .unwrap_or_default();

    listed
        .into_iter()
        .enumerate()
        .map(|(i, (address, name, default))| {
            let (soft, hard) = match rfkill.get(i) {
                Some((s, h)) => (Some(*s), Some(*h)),
                None => (None, None),
            };
            Controller {
                address,
                name,
                default,
                soft_blocked: soft,
                hard_blocked: hard,
            }
        })
        .collect()
}

/// Build the structured JSON value: `{ timestamp, controllers: [...] }`.
fn to_json(controllers: &[Controller]) -> Json {
    let items = controllers
        .iter()
        .map(|c| {
            Json::Object(vec![
                ("address".into(), c.address.clone().into()),
                ("name".into(), c.name.clone().into()),
                ("default".into(), c.default.into()),
                (
                    "soft_blocked".into(),
                    match c.soft_blocked {
                        Some(b) => b.into(),
                        None => Json::Null,
                    },
                ),
                (
                    "hard_blocked".into(),
                    match c.hard_blocked {
                        Some(b) => b.into(),
                        None => Json::Null,
                    },
                ),
            ])
        })
        .collect();
    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("controllers".into(), Json::Array(items)),
    ])
}

/// Render an optional bool as a table cell.
fn yesno(b: Option<bool>) -> String {
    match b {
        Some(true) => "yes".to_string(),
        Some(false) => "no".to_string(),
        None => "-".to_string(),
    }
}

/// Build the human-readable table.
fn to_table(controllers: &[Controller]) -> String {
    let rows: Vec<Vec<String>> = controllers
        .iter()
        .map(|c| {
            vec![
                c.address.clone(),
                c.name.clone(),
                if c.default { "yes" } else { "no" }.to_string(),
                yesno(c.soft_blocked),
                yesno(c.hard_blocked),
            ]
        })
        .collect();
    table::render(
        &["Address", "Name", "Default", "Soft Blocked", "Hard Blocked"],
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
get-bluetooth — Bluetooth controllers + rfkill block state, table or JSON.
Usage: get-bluetooth [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Sources: bluetoothctl list, rfkill list bluetooth (best-effort).";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-bluetooth: {e}");
            eprintln!("try 'get-bluetooth --help'");
            std::process::exit(2);
        }
    };

    let controllers = get_controllers();
    cli::emit(
        opts.format,
        || to_json(&controllers),
        || to_table(&controllers),
    );
}
