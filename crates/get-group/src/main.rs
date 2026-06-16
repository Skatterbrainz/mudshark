//! get-group — local groups and their members.
//! Source: `getent group` (colon-separated: name:x:gid:members).
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct Group {
    name: String,
    gid: u64,
    members: Vec<String>,
}

/// Query groups via `getent group`, parsing colon-separated fields.
fn get_groups() -> Result<Vec<Group>, String> {
    let text = proc::run("getent", &["group"])?;
    let mut groups = Vec::new();
    for line in text.lines() {
        // name:x:gid:members where members is a comma-separated list.
        let fields: Vec<&str> = line.splitn(4, ':').collect();
        if fields.len() < 4 {
            continue;
        }
        let members = fields[3]
            .split(',')
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(String::from)
            .collect();
        groups.push(Group {
            name: fields[0].to_string(),
            gid: fields[2].parse().unwrap_or(0),
            members,
        });
    }
    Ok(groups)
}

/// Build the structured JSON value: `{ timestamp, groups: [...] }`.
fn to_json(groups: &[Group]) -> Json {
    let items = groups
        .iter()
        .map(|g| {
            let members = g
                .members
                .iter()
                .map(|m| Json::from(m.clone()))
                .collect();
            Json::Object(vec![
                ("name".into(), g.name.clone().into()),
                ("gid".into(), g.gid.into()),
                ("members".into(), Json::Array(members)),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("groups".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(groups: &[Group]) -> String {
    let rows: Vec<Vec<String>> = groups
        .iter()
        .map(|g| {
            vec![
                g.name.clone(),
                g.gid.to_string(),
                g.members.join(","),
            ]
        })
        .collect();

    table::render(
        &["Name", "GID", "Members"],
        &rows,
        &[Align::Left, Align::Right, Align::Left],
    )
}

const HELP: &str = "\
get-group — local groups and members, table or JSON.
Usage: get-group [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: getent group.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-group: {e}");
            eprintln!("try 'get-group --help'");
            std::process::exit(2);
        }
    };

    let groups = match get_groups() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("get-group: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&groups), || to_table(&groups));
}
