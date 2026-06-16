//! get-package — installed packages (dpkg): name, version, architecture, status.
//! Source: `dpkg-query -W -f=...` (dpkg), which lists the package database.
//! Output helpers come from `mudshark-core`.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct Package {
    name: String,
    version: String,
    architecture: String,
    status: String,
}

/// Query the dpkg database, one tab-separated record per package.
fn get_packages() -> Result<Vec<Package>, String> {
    // Literal tabs/newline in the format are emitted verbatim by dpkg-query.
    let text = proc::run(
        "dpkg-query",
        &[
            "-W",
            "-f",
            "${Package}\t${Version}\t${Architecture}\t${Status}\n",
        ],
    )?;
    let mut packages = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        // splitn keeps any spaces in the multi-word status field intact.
        let fields: Vec<&str> = line.splitn(4, '\t').collect();
        if fields.len() < 4 {
            continue;
        }
        packages.push(Package {
            name: fields[0].to_string(),
            version: fields[1].to_string(),
            architecture: fields[2].to_string(),
            status: fields[3].to_string(),
        });
    }
    Ok(packages)
}

/// Build the structured JSON value: `{ timestamp, packages: [...] }`.
fn to_json(packages: &[Package]) -> Json {
    let items = packages
        .iter()
        .map(|p| {
            Json::Object(vec![
                ("name".into(), p.name.clone().into()),
                ("version".into(), p.version.clone().into()),
                ("architecture".into(), p.architecture.clone().into()),
                ("status".into(), p.status.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("packages".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(packages: &[Package]) -> String {
    let rows: Vec<Vec<String>> = packages
        .iter()
        .map(|p| {
            vec![
                p.name.clone(),
                p.version.clone(),
                p.architecture.clone(),
                p.status.clone(),
            ]
        })
        .collect();

    table::render(
        &["Name", "Version", "Architecture", "Status"],
        &rows,
        &[Align::Left, Align::Left, Align::Left, Align::Left],
    )
}

const HELP: &str = "\
get-package — installed packages (dpkg), table or JSON.
Usage: get-package [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: dpkg-query -W.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-package: {e}");
            eprintln!("try 'get-package --help'");
            std::process::exit(2);
        }
    };

    let packages = match get_packages() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("get-package: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&packages), || to_table(&packages));
}
