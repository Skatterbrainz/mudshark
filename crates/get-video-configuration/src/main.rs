//! get-video-configuration — display outputs and their current resolution.
//! Source: `xrandr --query`. A missing X display is treated as "no outputs"
//! (note on stderr, empty list, exit 0) rather than a hard error.
//! Output helpers come from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct Output {
    name: String,
    connected: bool,
    primary: bool,
    /// Current resolution (e.g. `1920x1080`) when the output is active.
    resolution: Option<String>,
}

/// Pull the active resolution from an output header line's geometry token.
///
/// xrandr renders an active output as `<name> connected [primary] WxH+X+Y ...`;
/// the geometry token is the only one that starts with a digit and contains
/// both `x` and `+`. The resolution is the part before the first `+`.
fn current_resolution(tokens: &[&str]) -> Option<String> {
    tokens.iter().find_map(|tok| {
        let starts_with_digit = tok
            .as_bytes()
            .first()
            .map_or(false, |b| b.is_ascii_digit());
        if starts_with_digit && tok.contains('x') && tok.contains('+') {
            Some(tok.split('+').next().unwrap_or(tok).to_string())
        } else {
            None
        }
    })
}

/// Parse `xrandr --query` into one record per output.
///
/// Output header lines are non-indented and not the leading `Screen` summary;
/// mode lines underneath are indented and skipped.
fn get_outputs() -> Result<Vec<Output>, String> {
    let text = proc::run("xrandr", &["--query"])?;
    let mut outputs = Vec::new();

    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') || line.starts_with("Screen") {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let connected = tokens.get(1).map_or(false, |t| *t == "connected");
        let primary = tokens.iter().any(|t| *t == "primary");
        let resolution = if connected {
            current_resolution(&tokens)
        } else {
            None
        };
        outputs.push(Output {
            name: tokens[0].to_string(),
            connected,
            primary,
            resolution,
        });
    }

    Ok(outputs)
}

/// Build the structured JSON value: `{ timestamp, outputs: [...] }`.
fn to_json(outputs: &[Output]) -> Json {
    let items = outputs
        .iter()
        .map(|o| {
            let resolution = match &o.resolution {
                Some(r) => r.clone().into(),
                None => Json::Null,
            };
            Json::Object(vec![
                ("name".into(), o.name.clone().into()),
                ("connected".into(), o.connected.into()),
                ("primary".into(), o.primary.into()),
                ("resolution".into(), resolution),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("outputs".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(outputs: &[Output]) -> String {
    let yes_no = |b: bool| if b { "yes" } else { "no" }.to_string();
    let rows: Vec<Vec<String>> = outputs
        .iter()
        .map(|o| {
            vec![
                o.name.clone(),
                yes_no(o.connected),
                yes_no(o.primary),
                o.resolution.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect();

    table::render(
        &["Output", "Connected", "Primary", "Resolution"],
        &rows,
        &[Align::Left, Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-video-configuration — display outputs and resolution, table or JSON.
Usage: get-video-configuration [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: xrandr --query.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-video-configuration: {e}");
            eprintln!("try 'get-video-configuration --help'");
            std::process::exit(2);
        }
    };

    // A failure here usually means no X display (e.g. headless / Wayland).
    // Treat it as "no outputs" so the command still emits valid output.
    let outputs = match get_outputs() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-video-configuration: {e}");
            eprintln!("get-video-configuration: no X display available; reporting no outputs");
            Vec::new()
        }
    };

    cli::emit(opts.format, || to_json(&outputs), || to_table(&outputs));
}
