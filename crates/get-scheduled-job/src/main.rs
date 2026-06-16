//! get-scheduled-job — scheduled jobs from systemd timers and the user crontab.
//! Sources:
//!   - `systemctl list-timers --all --no-legend` (type "timer").
//!   - current-user `crontab -l` (type "cron"); a missing/empty crontab is
//!     treated as "no cron jobs" and never an error.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

enum Job {
    /// A systemd timer. `schedule` is the raw leading next/left/last text.
    Timer {
        unit: String,
        activates: String,
        schedule: String,
    },
    /// A crontab entry split into its schedule expression and command.
    Cron { schedule: String, command: String },
}

/// Collect systemd timers and crontab entries into a single job list.
///
/// systemd timers are required (a `systemctl` failure is a hard error); the
/// crontab is best-effort and silently contributes nothing on failure.
fn get_jobs() -> Result<Vec<Job>, String> {
    let mut jobs = Vec::new();
    jobs.extend(get_timers()?);
    jobs.extend(get_cron_jobs());
    Ok(jobs)
}

/// Parse `systemctl list-timers`. The last two columns (UNIT, ACTIVATES) are
/// single tokens; everything before them is the schedule (NEXT/LEFT/LAST/PASSED),
/// which contains spaces and varies by systemd version, so it is kept raw.
fn get_timers() -> Result<Vec<Job>, String> {
    let text = proc::run("systemctl", &["list-timers", "--all", "--no-legend"])?;
    let mut jobs = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 2 {
            continue;
        }
        let activates = tokens[tokens.len() - 1].to_string();
        let unit = tokens[tokens.len() - 2].to_string();
        // Schedule is the text preceding the UNIT column.
        let schedule = tokens[..tokens.len() - 2].join(" ");
        jobs.push(Job::Timer {
            unit,
            activates,
            schedule,
        });
    }
    Ok(jobs)
}

/// Parse the current user's crontab. Comments, blank lines, and environment
/// assignments are skipped. `@`-shortcuts use a single-token schedule; standard
/// entries use the leading five fields as the schedule and the rest as command.
fn get_cron_jobs() -> Vec<Job> {
    let text = match proc::run("crontab", &["-l"]) {
        Ok(t) => t,
        Err(_) => return Vec::new(), // no crontab / crontab unavailable
    };
    let mut jobs = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('@') {
            // e.g. "@daily /usr/bin/foo"
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            let schedule = parts.next().unwrap_or("").to_string();
            let command = parts.next().unwrap_or("").trim_start().to_string();
            if command.is_empty() {
                continue;
            }
            jobs.push(Job::Cron { schedule, command });
            continue;
        }
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        // Skip environment assignments like "PATH=/usr/bin" (no schedule).
        if tokens.len() < 6 {
            continue;
        }
        if !looks_like_cron_field(tokens[0]) {
            continue;
        }
        let schedule = tokens[..5].join(" ");
        let command = remainder_after_tokens(trimmed, 5);
        if command.is_empty() {
            continue;
        }
        jobs.push(Job::Cron { schedule, command });
    }
    jobs
}

/// Heuristic: a cron schedule field consists only of digits and the symbols
/// `*,-/`. This filters out stray environment lines (e.g. `SHELL=/bin/sh`).
fn looks_like_cron_field(token: &str) -> bool {
    !token.is_empty()
        && token
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '*' | ',' | '-' | '/'))
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

/// Build the structured JSON value: `{ timestamp, jobs: [...] }`.
fn to_json(jobs: &[Job]) -> Json {
    let items = jobs
        .iter()
        .map(|j| match j {
            Job::Timer {
                unit,
                activates,
                schedule,
            } => Json::Object(vec![
                ("type".into(), "timer".into()),
                ("unit".into(), unit.clone().into()),
                ("activates".into(), activates.clone().into()),
                ("schedule".into(), schedule.clone().into()),
            ]),
            Job::Cron { schedule, command } => Json::Object(vec![
                ("type".into(), "cron".into()),
                ("schedule".into(), schedule.clone().into()),
                ("command".into(), command.clone().into()),
            ]),
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("jobs".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(jobs: &[Job]) -> String {
    let rows: Vec<Vec<String>> = jobs
        .iter()
        .map(|j| match j {
            Job::Timer {
                unit,
                activates,
                schedule,
            } => vec![
                "timer".to_string(),
                unit.clone(),
                schedule.clone(),
                activates.clone(),
            ],
            Job::Cron { schedule, command } => vec![
                "cron".to_string(),
                String::new(),
                schedule.clone(),
                command.clone(),
            ],
        })
        .collect();

    table::render(
        &["Type", "Name", "Schedule", "Detail"],
        &rows,
        &[Align::Left, Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-scheduled-job — systemd timers + user crontab, table or JSON.
Usage: get-scheduled-job [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Sources: systemctl list-timers --all --no-legend; crontab -l.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-scheduled-job: {e}");
            eprintln!("try 'get-scheduled-job --help'");
            std::process::exit(2);
        }
    };

    let jobs = match get_jobs() {
        Ok(j) => j,
        Err(e) => {
            eprintln!("get-scheduled-job: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&jobs), || to_table(&jobs));
}
