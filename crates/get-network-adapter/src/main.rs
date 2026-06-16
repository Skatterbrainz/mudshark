//! get-network-adapter — hardware network adapters (NICs).
//! Source: `lspci -k`, selecting devices whose class is an Ethernet or
//! Network controller, capturing the PCI slot, vendor+device description,
//! and the kernel driver in use.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct Adapter {
    slot: String,
    class: String,
    description: String,
    driver: String,
}

/// True if a PCI class string denotes a network adapter.
fn is_network_class(class: &str) -> bool {
    class == "Ethernet controller" || class == "Network controller"
}

/// Parse `lspci -k` output into network adapters.
///
/// A device record starts at column 0 as `<slot> <class>: <description>`,
/// followed by indented detail lines such as `Kernel driver in use: <drv>`.
fn parse_lspci(text: &str) -> Vec<Adapter> {
    let mut adapters: Vec<Adapter> = Vec::new();
    let mut current: Option<Adapter> = None;

    for line in text.lines() {
        let indented = line.starts_with(char::is_whitespace);
        if !indented {
            // A new device record begins; flush any in-progress adapter.
            if let Some(a) = current.take() {
                adapters.push(a);
            }
            let trimmed = line.trim_end();
            let Some((slot, rest)) = trimmed.split_once(' ') else {
                continue;
            };
            let Some((class, description)) = rest.split_once(": ") else {
                continue;
            };
            if is_network_class(class.trim()) {
                current = Some(Adapter {
                    slot: slot.to_string(),
                    class: class.trim().to_string(),
                    description: description.trim().to_string(),
                    driver: String::new(),
                });
            }
        } else if let Some(a) = current.as_mut() {
            if let Some((key, value)) = line.trim().split_once(':') {
                if key.trim() == "Kernel driver in use" {
                    a.driver = value.trim().to_string();
                }
            }
        }
    }
    if let Some(a) = current.take() {
        adapters.push(a);
    }
    adapters
}

/// Collect hardware network adapters via `lspci -k`.
fn get_adapters() -> Result<Vec<Adapter>, String> {
    let text = proc::run("lspci", &["-k"])?;
    Ok(parse_lspci(&text))
}

/// Build the structured JSON value: `{ timestamp, adapters: [...] }`.
fn to_json(adapters: &[Adapter]) -> Json {
    let items = adapters
        .iter()
        .map(|a| {
            Json::Object(vec![
                ("slot".into(), a.slot.clone().into()),
                ("class".into(), a.class.clone().into()),
                ("description".into(), a.description.clone().into()),
                ("driver".into(), a.driver.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("adapters".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(adapters: &[Adapter]) -> String {
    let rows: Vec<Vec<String>> = adapters
        .iter()
        .map(|a| {
            vec![
                a.slot.clone(),
                a.class.clone(),
                a.description.clone(),
                a.driver.clone(),
            ]
        })
        .collect();

    table::render(
        &["Slot", "Class", "Description", "Driver"],
        &rows,
        &[Align::Left, Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-network-adapter — hardware network adapters (NICs), table or JSON.
Usage: get-network-adapter [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: lspci -k.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-network-adapter: {e}");
            eprintln!("try 'get-network-adapter --help'");
            std::process::exit(2);
        }
    };

    let adapters = match get_adapters() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("get-network-adapter: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&adapters), || to_table(&adapters));
}
