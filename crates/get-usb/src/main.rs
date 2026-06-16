//! get-usb — connected USB devices.
//! Source: `lsusb`, one device per line:
//!   `Bus BBB Device DDD: ID vvvv:pppp Description`
//! Emits an array `devices` of {bus, device, vendor_id, product_id, id,
//! description}. If lsusb lists nothing, the array is empty (exit 0).

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct UsbDevice {
    bus: String,
    device: String,
    vendor_id: String,
    product_id: String,
    id: String,
    description: String,
}

/// Parse a single `lsusb` line into a `UsbDevice`, or `None` if malformed.
fn parse_line(line: &str) -> Option<UsbDevice> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    // Expect: Bus <bus> Device <dev>: ID <vvvv:pppp> <description...>
    if fields.len() < 6 || fields[0] != "Bus" || fields[2] != "Device" || fields[4] != "ID" {
        return None;
    }
    let bus = fields[1].to_string();
    let device = fields[3].trim_end_matches(':').to_string();
    let id = fields[5].to_string();
    let (vendor_id, product_id) = match id.split_once(':') {
        Some((v, p)) => (v.to_string(), p.to_string()),
        None => (String::new(), String::new()),
    };
    let description = fields[6..].join(" ");
    Some(UsbDevice {
        bus,
        device,
        vendor_id,
        product_id,
        id,
        description,
    })
}

/// Query connected USB devices via `lsusb`.
fn get_usb() -> Result<Vec<UsbDevice>, String> {
    let text = proc::run("lsusb", &[])?;
    Ok(text.lines().filter_map(parse_line).collect())
}

/// Build the structured JSON value: `{ timestamp, devices: [...] }`.
fn to_json(devices: &[UsbDevice]) -> Json {
    let items = devices
        .iter()
        .map(|d| {
            Json::Object(vec![
                ("bus".into(), d.bus.clone().into()),
                ("device".into(), d.device.clone().into()),
                ("vendor_id".into(), d.vendor_id.clone().into()),
                ("product_id".into(), d.product_id.clone().into()),
                ("id".into(), d.id.clone().into()),
                ("description".into(), d.description.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("devices".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(devices: &[UsbDevice]) -> String {
    let rows: Vec<Vec<String>> = devices
        .iter()
        .map(|d| {
            vec![
                d.bus.clone(),
                d.device.clone(),
                d.id.clone(),
                d.description.clone(),
            ]
        })
        .collect();

    table::render(
        &["Bus", "Device", "ID", "Description"],
        &rows,
        &[Align::Left, Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-usb — connected USB devices, table or JSON.
Usage: get-usb [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: lsusb.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-usb: {e}");
            eprintln!("try 'get-usb --help'");
            std::process::exit(2);
        }
    };

    let devices = match get_usb() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("get-usb: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&devices), || to_table(&devices));
}
