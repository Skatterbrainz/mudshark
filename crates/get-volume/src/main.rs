//! get-volume — mounted filesystems (volumes): size/used/available in bytes.
//! Source: `df -B1 --output=...` (GNU coreutils), which runs statvfs for us
//! and reports integer byte counts. Output helpers come from `mudshark-core`.

use std::process::Command;

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{bytes, time, Format};

struct Volume {
    source: String,
    fstype: String,
    size: u64,
    used: u64,
    available: u64,
    use_percent: u64,
    mountpoint: String,
}

/// Query mounted filesystems via `df`, parsing its fixed `--output` columns.
fn get_volumes() -> Result<Vec<Volume>, String> {
    let output = Command::new("df")
        .args(["-B1", "--output=source,fstype,size,used,avail,pcent,target"])
        .output()
        .map_err(|e| format!("failed to run df: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("df failed: {}", stderr.trim()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut volumes = Vec::new();
    for line in text.lines().skip(1) {
        // Fields are single tokens except the mountpoint (last), which may
        // contain spaces, so take the first six and re-join the remainder.
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 7 {
            continue;
        }
        volumes.push(Volume {
            source: fields[0].to_string(),
            fstype: fields[1].to_string(),
            size: fields[2].parse().unwrap_or(0),
            used: fields[3].parse().unwrap_or(0),
            available: fields[4].parse().unwrap_or(0),
            use_percent: fields[5].trim_end_matches('%').parse().unwrap_or(0),
            mountpoint: fields[6..].join(" "),
        });
    }
    Ok(volumes)
}

/// Build the structured JSON value: `{ timestamp, unit, volumes: [...] }`.
fn to_json(volumes: &[Volume]) -> Json {
    let items = volumes
        .iter()
        .map(|v| {
            Json::Object(vec![
                ("source".into(), v.source.clone().into()),
                ("fstype".into(), v.fstype.clone().into()),
                ("size".into(), v.size.into()),
                ("used".into(), v.used.into()),
                ("available".into(), v.available.into()),
                ("use_percent".into(), v.use_percent.into()),
                ("mountpoint".into(), v.mountpoint.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("unit".into(), "bytes".into()),
        ("volumes".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(volumes: &[Volume]) -> String {
    let rows: Vec<Vec<String>> = volumes
        .iter()
        .map(|v| {
            vec![
                v.source.clone(),
                v.fstype.clone(),
                bytes::human(v.size),
                bytes::human(v.used),
                bytes::human(v.available),
                format!("{}%", v.use_percent),
                v.mountpoint.clone(),
            ]
        })
        .collect();

    table::render(
        &["Source", "Type", "Size", "Used", "Avail", "Use%", "Mounted on"],
        &rows,
        &[
            Align::Left,
            Align::Left,
            Align::Right,
            Align::Right,
            Align::Right,
            Align::Right,
            Align::Left,
        ],
    )
}

fn print_help() {
    println!("get-volume — mounted filesystems (bytes), table or JSON.");
    println!("Usage: get-volume [--json | -o json|table] [-h|--help]");
    println!("Source: df -B1 --output=... (GNU coreutils).");
}

fn parse_args() -> Result<Format, String> {
    let mut format = Format::Table;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => format = Format::Json,
            "-o" | "--output" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --output".to_string())?;
                format = Format::parse(&value)?;
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(format)
}

fn main() {
    let format = match parse_args() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("get-volume: {e}");
            eprintln!("try 'get-volume --help'");
            std::process::exit(2);
        }
    };

    let volumes = match get_volumes() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("get-volume: {e}");
            std::process::exit(1);
        }
    };

    match format {
        Format::Json => println!("{}", to_json(&volumes).to_pretty_string()),
        Format::Table => print!("{}", to_table(&volumes)),
    }
}
