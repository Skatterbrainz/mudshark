//! get-memory — system memory + swap usage (bytes), human table or JSON.
//! Source: /proc/meminfo (locale-independent).
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use std::collections::HashMap;
use std::fs;

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{bytes, time, Format};

struct Memory {
    total: u64,
    used: u64,
    free: u64,
    available: u64,
    buffers: u64,
    cached: u64,
    shared: u64,
}

struct Swap {
    total: u64,
    used: u64,
    free: u64,
}

struct Report {
    timestamp: String,
    memory: Memory,
    swap: Swap,
}

/// Parse /proc/meminfo into field -> bytes (the file reports kB).
fn read_meminfo() -> std::io::Result<HashMap<String, u64>> {
    let mut map = HashMap::new();
    for line in fs::read_to_string("/proc/meminfo")?.lines() {
        // e.g. "MemTotal:       16384000 kB"
        if let Some((key, rest)) = line.split_once(':') {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(v) = kb.parse::<u64>() {
                    map.insert(key.to_string(), v * 1024);
                }
            }
        }
    }
    Ok(map)
}

/// Collect memory + swap, computing `used` the way `free` does:
///   cached = Cached + SReclaimable; used = total - free - buffers - cached
fn get_memory() -> std::io::Result<Report> {
    let m = read_meminfo()?;
    let g = |k: &str| m.get(k).copied().unwrap_or(0);

    let total = g("MemTotal");
    let free = g("MemFree");
    let buffers = g("Buffers");
    let cached = g("Cached") + g("SReclaimable");
    let swap_total = g("SwapTotal");
    let swap_free = g("SwapFree");

    Ok(Report {
        timestamp: time::now_utc_iso8601(),
        memory: Memory {
            total,
            used: total.saturating_sub(free + buffers + cached),
            free,
            available: g("MemAvailable"),
            buffers,
            cached,
            shared: g("Shmem"),
        },
        swap: Swap {
            total: swap_total,
            used: swap_total.saturating_sub(swap_free),
            free: swap_free,
        },
    })
}

/// Build the structured JSON value (serialised + escaped by mudshark-core).
fn to_json(r: &Report) -> Json {
    Json::Object(vec![
        ("timestamp".into(), r.timestamp.clone().into()),
        ("unit".into(), "bytes".into()),
        (
            "memory".into(),
            Json::Object(vec![
                ("total".into(), r.memory.total.into()),
                ("used".into(), r.memory.used.into()),
                ("free".into(), r.memory.free.into()),
                ("available".into(), r.memory.available.into()),
                ("buffers".into(), r.memory.buffers.into()),
                ("cached".into(), r.memory.cached.into()),
                ("shared".into(), r.memory.shared.into()),
            ]),
        ),
        (
            "swap".into(),
            Json::Object(vec![
                ("total".into(), r.swap.total.into()),
                ("used".into(), r.swap.used.into()),
                ("free".into(), r.swap.free.into()),
            ]),
        ),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(r: &Report) -> String {
    let rows = vec![
        vec!["Total".to_string(), bytes::human(r.memory.total)],
        vec!["Used".to_string(), bytes::human(r.memory.used)],
        vec!["Free".to_string(), bytes::human(r.memory.free)],
        vec!["Available".to_string(), bytes::human(r.memory.available)],
        vec!["Buffers".to_string(), bytes::human(r.memory.buffers)],
        vec!["Cached".to_string(), bytes::human(r.memory.cached)],
        vec!["Shared".to_string(), bytes::human(r.memory.shared)],
        vec!["Swap Total".to_string(), bytes::human(r.swap.total)],
        vec!["Swap Used".to_string(), bytes::human(r.swap.used)],
        vec!["Swap Free".to_string(), bytes::human(r.swap.free)],
    ];
    table::render(&["Metric", "Size"], &rows, &[Align::Left, Align::Right])
}

fn print_help() {
    println!("get-memory — system memory + swap usage (bytes), table or JSON.");
    println!("Usage: get-memory [--json | -o json|table] [-h|--help]");
    println!("Source: /proc/meminfo.");
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
            eprintln!("get-memory: {e}");
            eprintln!("try 'get-memory --help'");
            std::process::exit(2);
        }
    };

    let report = match get_memory() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("get-memory: failed to read /proc/meminfo: {e}");
            std::process::exit(1);
        }
    };

    match format {
        Format::Json => println!("{}", to_json(&report).to_pretty_string()),
        Format::Table => print!("{}", to_table(&report)),
    }
}
