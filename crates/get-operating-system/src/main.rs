//! get-operating-system — OS identity from /etc/os-release plus kernel info
//! from `uname`. Single-object output. Helpers from `mudshark-core`.

use std::fs;

use mudshark_core::json::Json;
use mudshark_core::parse;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct OperatingSystem {
    name: String,
    version: String,
    id: String,
    version_id: String,
    pretty_name: String,
    kernel_name: String,
    kernel_release: String,
    kernel_machine: String,
}

/// Collect OS identity from /etc/os-release and kernel info from `uname`.
fn get_operating_system() -> Result<OperatingSystem, String> {
    // os-release lines are KEY=VALUE / KEY="VALUE"; parse each independently.
    // The file is informational, so a missing file yields empty fields.
    let mut name = String::new();
    let mut version = String::new();
    let mut id = String::new();
    let mut version_id = String::new();
    let mut pretty_name = String::new();
    if let Ok(text) = fs::read_to_string("/etc/os-release") {
        for line in text.lines() {
            for (k, v) in parse::key_value_pairs(line) {
                match k.as_str() {
                    "NAME" => name = v,
                    "VERSION" => version = v,
                    "ID" => id = v,
                    "VERSION_ID" => version_id = v,
                    "PRETTY_NAME" => pretty_name = v,
                    _ => {}
                }
            }
        }
    }

    // Kernel name, release, and machine on a single line, e.g. "Linux 6.8.0 x86_64".
    let uname = proc::run("uname", &["-s", "-r", "-m"])?;
    let parts: Vec<&str> = uname.split_whitespace().collect();
    let kernel_name = parts.first().copied().unwrap_or_default().to_string();
    let kernel_release = parts.get(1).copied().unwrap_or_default().to_string();
    let kernel_machine = parts.get(2).copied().unwrap_or_default().to_string();

    Ok(OperatingSystem {
        name,
        version,
        id,
        version_id,
        pretty_name,
        kernel_name,
        kernel_release,
        kernel_machine,
    })
}

/// Build the structured JSON value (single object of OS facts).
fn to_json(os: &OperatingSystem) -> Json {
    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("name".into(), os.name.clone().into()),
        ("version".into(), os.version.clone().into()),
        ("id".into(), os.id.clone().into()),
        ("version_id".into(), os.version_id.clone().into()),
        ("pretty_name".into(), os.pretty_name.clone().into()),
        ("kernel_name".into(), os.kernel_name.clone().into()),
        ("kernel_release".into(), os.kernel_release.clone().into()),
        ("kernel_machine".into(), os.kernel_machine.clone().into()),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(os: &OperatingSystem) -> String {
    let rows = vec![
        vec!["Name".to_string(), os.name.clone()],
        vec!["Version".to_string(), os.version.clone()],
        vec!["ID".to_string(), os.id.clone()],
        vec!["Version ID".to_string(), os.version_id.clone()],
        vec!["Pretty Name".to_string(), os.pretty_name.clone()],
        vec!["Kernel Name".to_string(), os.kernel_name.clone()],
        vec!["Kernel Release".to_string(), os.kernel_release.clone()],
        vec!["Kernel Machine".to_string(), os.kernel_machine.clone()],
    ];
    table::render(&["Field", "Value"], &rows, &[Align::Left, Align::Left])
}

const HELP: &str = "\
get-operating-system — OS identity and kernel info, table or JSON.
Usage: get-operating-system [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: /etc/os-release and uname -s -r -m.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-operating-system: {e}");
            eprintln!("try 'get-operating-system --help'");
            std::process::exit(2);
        }
    };

    let os = match get_operating_system() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-operating-system: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&os), || to_table(&os));
}
