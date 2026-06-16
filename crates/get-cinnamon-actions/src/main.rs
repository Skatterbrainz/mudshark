//! get-cinnamon-actions — installed Nemo file-manager actions.
//! Source: `*.nemo_action` files under the system and per-user action dirs,
//! enumerated directly via `std::fs`. Missing directories are skipped.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use std::fs;
use std::path::Path;

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, time};

/// One discovered Nemo action.
struct Action {
    /// File name (e.g. "open-terminal.nemo_action").
    file: String,
    /// Absolute path to the file.
    path: String,
    /// Display name from the `Name=` line, if present.
    name: Option<String>,
}

/// Read the unlocalised `Name=` value from a .nemo_action file, if any.
fn read_name(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        // Skip localised variants like `Name[de]=...`; take the bare key.
        if let Some(rest) = line.strip_prefix("Name=") {
            let v = rest.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Collect `*.nemo_action` files from a single directory (missing dir -> none).
fn collect_dir(dir: &Path, out: &mut Vec<Action>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // Directory absent or unreadable: skip silently.
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("nemo_action") {
            continue;
        }
        let file = entry.file_name().to_string_lossy().into_owned();
        let name = read_name(&path);
        out.push(Action {
            file,
            path: path.to_string_lossy().into_owned(),
            name,
        });
    }
}

/// Enumerate actions from the system and per-user action directories.
fn get_actions() -> Vec<Action> {
    let mut actions = Vec::new();
    collect_dir(Path::new("/usr/share/nemo/actions"), &mut actions);
    if let Ok(home) = std::env::var("HOME") {
        let user_dir = Path::new(&home).join(".local/share/nemo/actions");
        collect_dir(&user_dir, &mut actions);
    }
    actions.sort_by(|a, b| a.file.cmp(&b.file));
    actions
}

/// Build the structured JSON value: `{ timestamp, actions: [...] }`.
fn to_json(actions: &[Action]) -> Json {
    let items = actions
        .iter()
        .map(|a| {
            Json::Object(vec![
                ("file".into(), a.file.clone().into()),
                ("path".into(), a.path.clone().into()),
                (
                    "name".into(),
                    match &a.name {
                        Some(n) => n.clone().into(),
                        None => Json::Null,
                    },
                ),
            ])
        })
        .collect();
    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("actions".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table.
fn to_table(actions: &[Action]) -> String {
    let rows: Vec<Vec<String>> = actions
        .iter()
        .map(|a| {
            vec![
                a.name.clone().unwrap_or_default(),
                a.file.clone(),
                a.path.clone(),
            ]
        })
        .collect();
    table::render(
        &["Name", "File", "Path"],
        &rows,
        &[Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-cinnamon-actions — installed Nemo actions (*.nemo_action), table or JSON.
Usage: get-cinnamon-actions [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Sources: /usr/share/nemo/actions, ~/.local/share/nemo/actions.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-cinnamon-actions: {e}");
            eprintln!("try 'get-cinnamon-actions --help'");
            std::process::exit(2);
        }
    };

    let actions = get_actions();
    cli::emit(opts.format, || to_json(&actions), || to_table(&actions));
}
