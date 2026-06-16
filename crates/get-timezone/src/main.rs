//! get-timezone — system timezone and clock settings. Single-object output.
//! Source: `timedatectl show` (systemd), which prints KEY=Value lines.
//! Helpers from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::parse;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct Timezone {
    timezone: String,
    local_rtc: bool,
    can_ntp: bool,
    ntp: bool,
    ntp_synchronized: bool,
}

/// Collect timezone/clock facts via `timedatectl show`.
fn get_timezone() -> Result<Timezone, String> {
    let text = proc::run("timedatectl", &["show"])?;
    let mut timezone = String::new();
    let mut local_rtc = false;
    let mut can_ntp = false;
    let mut ntp = false;
    let mut ntp_synchronized = false;
    // Each line is a single KEY=Value pair (values are unquoted yes/no/strings).
    for line in text.lines() {
        for (k, v) in parse::key_value_pairs(line) {
            let on = v == "yes";
            match k.as_str() {
                "Timezone" => timezone = v,
                "LocalRTC" => local_rtc = on,
                "CanNTP" => can_ntp = on,
                "NTP" => ntp = on,
                "NTPSynchronized" => ntp_synchronized = on,
                _ => {}
            }
        }
    }
    Ok(Timezone {
        timezone,
        local_rtc,
        can_ntp,
        ntp,
        ntp_synchronized,
    })
}

/// Build the structured JSON value (single object of timezone facts).
fn to_json(t: &Timezone) -> Json {
    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("timezone".into(), t.timezone.clone().into()),
        ("local_rtc".into(), t.local_rtc.into()),
        ("can_ntp".into(), t.can_ntp.into()),
        ("ntp".into(), t.ntp.into()),
        ("ntp_synchronized".into(), t.ntp_synchronized.into()),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(t: &Timezone) -> String {
    let yn = |b: bool| if b { "yes" } else { "no" }.to_string();
    let rows = vec![
        vec!["Timezone".to_string(), t.timezone.clone()],
        vec!["Local RTC".to_string(), yn(t.local_rtc)],
        vec!["Can NTP".to_string(), yn(t.can_ntp)],
        vec!["NTP".to_string(), yn(t.ntp)],
        vec!["NTP Synchronized".to_string(), yn(t.ntp_synchronized)],
    ];
    table::render(&["Field", "Value"], &rows, &[Align::Left, Align::Left])
}

const HELP: &str = "\
get-timezone — system timezone and clock settings, table or JSON.
Usage: get-timezone [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: timedatectl show (systemd).";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-timezone: {e}");
            eprintln!("try 'get-timezone --help'");
            std::process::exit(2);
        }
    };

    let tz = match get_timezone() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("get-timezone: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&tz), || to_table(&tz));
}
