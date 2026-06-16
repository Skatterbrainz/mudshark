//! get-volume — mounted filesystems (volumes): size/used/available in bytes.
//! Source: `df -B1 --output=...` (GNU coreutils), which runs statvfs for us
//! and reports integer byte counts. Output helpers come from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{bytes, cli, proc, time};

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
    let text = proc::run(
        "df",
        &["-B1", "--output=source,fstype,size,used,avail,pcent,target"],
    )?;
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

const HELP: &str = "\
get-volume — mounted filesystems (bytes), table or JSON.
Usage: get-volume [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: df -B1 --output=... (GNU coreutils).";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
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

    cli::emit(opts.format, || to_json(&volumes), || to_table(&volumes));
}
