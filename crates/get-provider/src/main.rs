//! get-provider — known package providers and whether they're installed.
//! Source: `<tool> --version` for each of apt, dpkg, flatpak, snap.
//! Output helpers come from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

/// Providers probed, in display order.
const PROVIDERS: [&str; 4] = ["apt", "dpkg", "flatpak", "snap"];

struct Provider {
    name: String,
    available: bool,
    /// First line of `--version` output, or `None` when the tool is absent.
    version: Option<String>,
}

/// Probe a single provider by running `<tool> --version`.
///
/// A missing tool (or any failure to run it) is reported as `available=false`
/// rather than a hard error — absence is a normal, expected result here.
fn probe(name: &str) -> Provider {
    match proc::run(name, &["--version"]) {
        Ok(out) => {
            let version = out
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .map(|l| l.to_string());
            Provider {
                name: name.to_string(),
                available: true,
                version,
            }
        }
        Err(_) => Provider {
            name: name.to_string(),
            available: false,
            version: None,
        },
    }
}

/// Probe every known provider.
fn get_providers() -> Vec<Provider> {
    PROVIDERS.iter().map(|name| probe(name)).collect()
}

/// Build the structured JSON value: `{ timestamp, providers: [...] }`.
fn to_json(providers: &[Provider]) -> Json {
    let items = providers
        .iter()
        .map(|p| {
            let version = match &p.version {
                Some(v) => v.clone().into(),
                None => Json::Null,
            };
            Json::Object(vec![
                ("name".into(), p.name.clone().into()),
                ("available".into(), p.available.into()),
                ("version".into(), version),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("providers".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(providers: &[Provider]) -> String {
    let rows: Vec<Vec<String>> = providers
        .iter()
        .map(|p| {
            vec![
                p.name.clone(),
                if p.available { "yes" } else { "no" }.to_string(),
                p.version.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect();

    table::render(
        &["Provider", "Available", "Version"],
        &rows,
        &[Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-provider — known package providers and versions, table or JSON.
Usage: get-provider [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: <tool> --version for apt, dpkg, flatpak, snap.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-provider: {e}");
            eprintln!("try 'get-provider --help'");
            std::process::exit(2);
        }
    };

    let providers = get_providers();

    cli::emit(opts.format, || to_json(&providers), || to_table(&providers));
}
