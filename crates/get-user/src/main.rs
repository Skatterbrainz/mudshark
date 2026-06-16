//! get-user — local user accounts.
//! Source: `getent passwd` (colon-separated: name:x:uid:gid:gecos:home:shell).
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct User {
    name: String,
    uid: u64,
    gid: u64,
    gecos: String,
    home: String,
    shell: String,
}

/// Query user accounts via `getent passwd`, parsing colon-separated fields.
fn get_users() -> Result<Vec<User>, String> {
    let text = proc::run("getent", &["passwd"])?;
    let mut users = Vec::new();
    for line in text.lines() {
        // name:x:uid:gid:gecos:home:shell — gecos may itself contain commas
        // but never a colon, so splitting on ':' is safe.
        let fields: Vec<&str> = line.splitn(7, ':').collect();
        if fields.len() < 7 {
            continue;
        }
        users.push(User {
            name: fields[0].to_string(),
            uid: fields[2].parse().unwrap_or(0),
            gid: fields[3].parse().unwrap_or(0),
            gecos: fields[4].to_string(),
            home: fields[5].to_string(),
            shell: fields[6].to_string(),
        });
    }
    Ok(users)
}

/// Build the structured JSON value: `{ timestamp, users: [...] }`.
fn to_json(users: &[User]) -> Json {
    let items = users
        .iter()
        .map(|u| {
            Json::Object(vec![
                ("name".into(), u.name.clone().into()),
                ("uid".into(), u.uid.into()),
                ("gid".into(), u.gid.into()),
                ("gecos".into(), u.gecos.clone().into()),
                ("home".into(), u.home.clone().into()),
                ("shell".into(), u.shell.clone().into()),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("users".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table via the shared renderer.
fn to_table(users: &[User]) -> String {
    let rows: Vec<Vec<String>> = users
        .iter()
        .map(|u| {
            vec![
                u.name.clone(),
                u.uid.to_string(),
                u.gid.to_string(),
                u.gecos.clone(),
                u.home.clone(),
                u.shell.clone(),
            ]
        })
        .collect();

    table::render(
        &["Name", "UID", "GID", "Gecos", "Home", "Shell"],
        &rows,
        &[
            Align::Left,
            Align::Right,
            Align::Right,
            Align::Left,
            Align::Left,
            Align::Left,
        ],
    )
}

const HELP: &str = "\
get-user — local user accounts, table or JSON.
Usage: get-user [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: getent passwd.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-user: {e}");
            eprintln!("try 'get-user --help'");
            std::process::exit(2);
        }
    };

    let users = match get_users() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("get-user: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&users), || to_table(&users));
}
