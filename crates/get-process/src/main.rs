//! get-process — running processes.
//! Source: `ps -eo pid,ppid,user,pcpu,pmem,rss,comm,args --no-headers`.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{bytes, cli, proc, time};

struct Process {
    pid: u64,
    ppid: u64,
    user: String,
    cpu_percent: f64,
    mem_percent: f64,
    rss_bytes: u64,
    comm: String,
    args: String,
}

/// Query processes via `ps`, parsing fixed columns. The first seven columns
/// are single whitespace-separated tokens; `args` is the remainder of the line.
fn get_processes() -> Result<Vec<Process>, String> {
    let text = proc::run(
        "ps",
        &[
            "-eo",
            "pid,ppid,user,pcpu,pmem,rss,comm,args",
            "--no-headers",
        ],
    )?;
    let mut processes = Vec::new();
    for line in text.lines() {
        // The first seven columns are single whitespace-separated tokens;
        // `args` is the remainder of the line (and may contain spaces).
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 8 {
            continue;
        }
        // Recover `args` as everything after the 7th token by walking past the
        // 7 leading columns and their surrounding whitespace.
        let args = remainder_after_tokens(line, 7);

        let rss_kib: u64 = fields[5].parse().unwrap_or(0);
        processes.push(Process {
            pid: fields[0].parse().unwrap_or(0),
            ppid: fields[1].parse().unwrap_or(0),
            user: fields[2].to_string(),
            cpu_percent: fields[3].parse().unwrap_or(0.0),
            mem_percent: fields[4].parse().unwrap_or(0.0),
            rss_bytes: rss_kib * 1024,
            comm: fields[6].to_string(),
            args,
        });
    }
    Ok(processes)
}

/// Return the substring of `line` that follows the first `n` whitespace-
/// separated tokens, with leading whitespace trimmed.
fn remainder_after_tokens(line: &str, n: usize) -> String {
    let mut count = 0;
    let mut in_token = false;
    for (idx, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if in_token {
                in_token = false;
                count += 1;
                if count == n {
                    return line[idx..].trim_start().to_string();
                }
            }
        } else {
            in_token = true;
        }
    }
    String::new()
}

/// Build the structured JSON value: `{ timestamp, unit, processes: [...] }`.
fn to_json(processes: &[Process]) -> Json {
    let items = processes
        .iter()
        .map(|p| {
            Json::Object(vec![
                ("pid".into(), p.pid.into()),
                ("ppid".into(), p.ppid.into()),
                ("user".into(), p.user.clone().into()),
                ("cpu_percent".into(), p.cpu_percent.into()),
                ("mem_percent".into(), p.mem_percent.into()),
                ("rss_bytes".into(), p.rss_bytes.into()),
                ("comm".into(), p.comm.clone().into()),
                ("args".into(), p.args.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("unit".into(), "bytes".into()),
        ("processes".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(processes: &[Process]) -> String {
    let rows: Vec<Vec<String>> = processes
        .iter()
        .map(|p| {
            vec![
                p.pid.to_string(),
                p.user.clone(),
                format!("{:.1}", p.cpu_percent),
                format!("{:.1}", p.mem_percent),
                bytes::human(p.rss_bytes),
                p.comm.clone(),
            ]
        })
        .collect();

    table::render(
        &["PID", "User", "CPU%", "Mem%", "RSS", "Comm"],
        &rows,
        &[
            Align::Right,
            Align::Left,
            Align::Right,
            Align::Right,
            Align::Right,
            Align::Left,
        ],
    )
}

const HELP: &str = "\
get-process — running processes, table or JSON.
Usage: get-process [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: ps -eo pid,ppid,user,pcpu,pmem,rss,comm,args.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-process: {e}");
            eprintln!("try 'get-process --help'");
            std::process::exit(2);
        }
    };

    let processes = match get_processes() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("get-process: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&processes), || to_table(&processes));
}
