//! get-video-device — video/display PCI controllers and their kernel drivers.
//! Source: `lspci -k`, selecting VGA / 3D / Display controller classes.
//! Output helpers come from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct VideoDevice {
    slot: String,
    description: String,
    /// Value of `Kernel driver in use`, or `None` when no driver is bound.
    driver: Option<String>,
}

/// True for PCI classes that represent a video/display adapter.
fn is_display_class(class: &str) -> bool {
    class.contains("VGA compatible controller")
        || class.contains("3D controller")
        || class.contains("Display controller")
}

/// Parse `lspci -k`, keeping only display devices and their bound driver.
///
/// `lspci -k` emits a non-indented header line per device
/// (`<slot> <class>: <description>`) followed by indented detail lines, one of
/// which may be `Kernel driver in use: <driver>`.
fn get_devices() -> Result<Vec<VideoDevice>, String> {
    let text = proc::run("lspci", &["-k"])?;
    let mut devices: Vec<VideoDevice> = Vec::new();
    let mut current: Option<VideoDevice> = None;

    for line in text.lines() {
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented {
            // New device header: flush the previous display device (if any).
            if let Some(dev) = current.take() {
                devices.push(dev);
            }
            if let Some((slot, rest)) = line.split_once(' ') {
                if let Some((class, description)) = rest.split_once(": ") {
                    if is_display_class(class) {
                        current = Some(VideoDevice {
                            slot: slot.to_string(),
                            description: description.to_string(),
                            driver: None,
                        });
                    }
                }
            }
        } else if let Some(dev) = current.as_mut() {
            if let Some(driver) = line.trim().strip_prefix("Kernel driver in use:") {
                dev.driver = Some(driver.trim().to_string());
            }
        }
    }
    if let Some(dev) = current.take() {
        devices.push(dev);
    }

    Ok(devices)
}

/// Build the structured JSON value: `{ timestamp, devices: [...] }`.
fn to_json(devices: &[VideoDevice]) -> Json {
    let items = devices
        .iter()
        .map(|d| {
            let driver = match &d.driver {
                Some(v) => v.clone().into(),
                None => Json::Null,
            };
            Json::Object(vec![
                ("slot".into(), d.slot.clone().into()),
                ("description".into(), d.description.clone().into()),
                ("driver".into(), driver),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("devices".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(devices: &[VideoDevice]) -> String {
    let rows: Vec<Vec<String>> = devices
        .iter()
        .map(|d| {
            vec![
                d.slot.clone(),
                d.description.clone(),
                d.driver.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect();

    table::render(
        &["Slot", "Description", "Driver"],
        &rows,
        &[Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-video-device — video/display PCI devices, table or JSON.
Usage: get-video-device [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: lspci -k.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-video-device: {e}");
            eprintln!("try 'get-video-device --help'");
            std::process::exit(2);
        }
    };

    let devices = match get_devices() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("get-video-device: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&devices), || to_table(&devices));
}
