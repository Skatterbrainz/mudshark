//! get-disk — block devices (disks): name, type, size (bytes), model, serial,
//! fstype, mountpoint. Source: `lsblk -b -P -o ...` (util-linux), which reports
//! integer byte sizes and quoted KEY="value" pairs. Helpers from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::parse;
use mudshark_core::table::{self, Align};
use mudshark_core::{bytes, cli, proc, time};

struct Disk {
    name: String,
    dtype: String,
    size: u64,
    model: String,
    serial: String,
    fstype: String,
    mountpoint: String,
}

/// Query block devices via `lsblk`, parsing its `-P` KEY="value" lines.
fn get_disks() -> Result<Vec<Disk>, String> {
    let text = proc::run(
        "lsblk",
        &[
            "-b",
            "-P",
            "-o",
            "NAME,TYPE,SIZE,MODEL,SERIAL,FSTYPE,MOUNTPOINT",
        ],
    )?;
    let mut disks = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let pairs = parse::key_value_pairs(line);
        let get = |k: &str| {
            pairs
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        disks.push(Disk {
            name: get("NAME"),
            dtype: get("TYPE"),
            size: get("SIZE").parse().unwrap_or(0),
            model: get("MODEL"),
            serial: get("SERIAL"),
            fstype: get("FSTYPE"),
            mountpoint: get("MOUNTPOINT"),
        });
    }
    Ok(disks)
}

/// Build the structured JSON value: `{ timestamp, unit, disks: [...] }`.
fn to_json(disks: &[Disk]) -> Json {
    let items = disks
        .iter()
        .map(|d| {
            Json::Object(vec![
                ("name".into(), d.name.clone().into()),
                ("type".into(), d.dtype.clone().into()),
                ("size".into(), d.size.into()),
                ("model".into(), d.model.clone().into()),
                ("serial".into(), d.serial.clone().into()),
                ("fstype".into(), d.fstype.clone().into()),
                ("mountpoint".into(), d.mountpoint.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("unit".into(), "bytes".into()),
        ("disks".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(disks: &[Disk]) -> String {
    let rows: Vec<Vec<String>> = disks
        .iter()
        .map(|d| {
            vec![
                d.name.clone(),
                d.dtype.clone(),
                bytes::human(d.size),
                d.model.clone(),
                d.serial.clone(),
                d.fstype.clone(),
                d.mountpoint.clone(),
            ]
        })
        .collect();

    table::render(
        &[
            "Name",
            "Type",
            "Size",
            "Model",
            "Serial",
            "FSType",
            "Mountpoint",
        ],
        &rows,
        &[
            Align::Left,
            Align::Left,
            Align::Right,
            Align::Left,
            Align::Left,
            Align::Left,
            Align::Left,
        ],
    )
}

const HELP: &str = "\
get-disk — block devices (disks) with sizes in bytes, table or JSON.
Usage: get-disk [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: lsblk -b -P -o NAME,TYPE,SIZE,MODEL,SERIAL,FSTYPE,MOUNTPOINT (util-linux).";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-disk: {e}");
            eprintln!("try 'get-disk --help'");
            std::process::exit(2);
        }
    };

    let disks = match get_disks() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("get-disk: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&disks), || to_table(&disks));
}
