//! get-cinnamon-desktop — Cinnamon version + key theme/interface settings.
//! Sources: `cinnamon --version` plus best-effort `gsettings get` queries.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

/// A single resolved setting: a human label and its string value.
struct Setting {
    /// JSON key (snake_case).
    key: &'static str,
    /// Human-readable table label.
    label: &'static str,
    value: String,
}

struct Desktop {
    timestamp: String,
    /// Cinnamon version string, if `cinnamon --version` succeeded.
    version: Option<String>,
    /// Best-effort gsettings values; entries that errored are omitted.
    settings: Vec<Setting>,
}

/// Read `cinnamon --version` and return the version token (e.g. "5.6.8").
fn cinnamon_version() -> Option<String> {
    let out = proc::run("cinnamon", &["--version"]).ok()?;
    // Typical output: "Cinnamon 5.6.8".
    let line = out.lines().next()?.trim();
    let v = line
        .strip_prefix("Cinnamon ")
        .unwrap_or(line)
        .trim()
        .to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

/// Query a single gsettings key, returning its unquoted value on success.
///
/// `gsettings get` prints values GVariant-quoted (e.g. `'Mint-Y'`); we strip
/// surrounding single quotes. Missing schemas/keys return an error and are
/// dropped by the caller.
fn gset(schema: &str, key: &str) -> Option<String> {
    let out = proc::run("gsettings", &["get", schema, key]).ok()?;
    let trimmed = out.trim();
    let unquoted = trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(trimmed)
        .to_string();
    if unquoted.is_empty() {
        None
    } else {
        Some(unquoted)
    }
}

/// Collect the version and the best-effort settings list.
fn get_desktop() -> Desktop {
    // (schema, key, json_key, label)
    let queries: &[(&str, &str, &'static str, &'static str)] = &[
        (
            "org.cinnamon.desktop.interface",
            "gtk-theme",
            "gtk_theme",
            "GTK Theme",
        ),
        (
            "org.cinnamon.desktop.interface",
            "icon-theme",
            "icon_theme",
            "Icon Theme",
        ),
        (
            "org.cinnamon.desktop.interface",
            "cursor-theme",
            "cursor_theme",
            "Cursor Theme",
        ),
        (
            "org.cinnamon.desktop.interface",
            "font-name",
            "font_name",
            "Font",
        ),
        ("org.cinnamon", "theme name", "cinnamon_theme", "Cinnamon Theme"),
    ];

    let mut settings = Vec::new();
    for (schema, key, json_key, label) in queries {
        if let Some(value) = gset(schema, key) {
            settings.push(Setting {
                key: json_key,
                label,
                value,
            });
        }
    }

    Desktop {
        timestamp: time::now_utc_iso8601(),
        version: cinnamon_version(),
        settings,
    }
}

/// Build the structured JSON value: `{ timestamp, version?, <settings...> }`.
fn to_json(d: &Desktop) -> Json {
    let mut obj: Vec<(String, Json)> = vec![("timestamp".into(), d.timestamp.clone().into())];
    if let Some(v) = &d.version {
        obj.push(("version".into(), v.clone().into()));
    }
    for s in &d.settings {
        obj.push((s.key.to_string(), s.value.clone().into()));
    }
    Json::Object(obj)
}

/// Build the human-readable table (Property / Value rows).
fn to_table(d: &Desktop) -> String {
    let mut rows: Vec<Vec<String>> = Vec::new();
    rows.push(vec![
        "Version".to_string(),
        d.version.clone().unwrap_or_else(|| "unknown".to_string()),
    ]);
    for s in &d.settings {
        rows.push(vec![s.label.to_string(), s.value.clone()]);
    }
    table::render(&["Property", "Value"], &rows, &[Align::Left, Align::Left])
}

const HELP: &str = "\
get-cinnamon-desktop — Cinnamon version + theme/interface settings, table or JSON.
Usage: get-cinnamon-desktop [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Sources: cinnamon --version, gsettings get.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-cinnamon-desktop: {e}");
            eprintln!("try 'get-cinnamon-desktop --help'");
            std::process::exit(2);
        }
    };

    let desktop = get_desktop();
    cli::emit(opts.format, || to_json(&desktop), || to_table(&desktop));
}
