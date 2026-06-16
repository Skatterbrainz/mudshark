//! get-cinnamon-applet — enabled Cinnamon panel applets.
//! Source: `gsettings get org.cinnamon enabled-applets`, whose value is a
//! GVariant string-list literal, e.g.
//!   ['panel1:left:0:menu@cinnamon.org:0', 'panel1:left:1:separator@...:1']
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

/// One enabled applet, decomposed from its `panel:position:order:uuid:instance`
/// definition. Fields beyond the raw string are best-effort.
struct Applet {
    raw: String,
    panel: String,
    position: String,
    order: String,
    uuid: String,
    instance: String,
}

/// Extract the contents of single-quoted entries from a GVariant list literal.
/// `@as []` (the empty typed array) yields no entries.
fn parse_quoted_entries(value: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut chars = value.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c == '\'' {
            let mut s = String::new();
            for (_, c2) in chars.by_ref() {
                if c2 == '\'' {
                    break;
                }
                s.push(c2);
            }
            entries.push(s);
        }
    }
    entries
}

/// Run gsettings and parse the enabled-applets list.
fn get_applets() -> Result<Vec<Applet>, String> {
    let out = proc::run("gsettings", &["get", "org.cinnamon", "enabled-applets"])?;
    let applets = parse_quoted_entries(out.trim())
        .into_iter()
        .map(|raw| {
            let f: Vec<&str> = raw.splitn(5, ':').collect();
            let get = |i: usize| f.get(i).map(|s| s.to_string()).unwrap_or_default();
            Applet {
                panel: get(0),
                position: get(1),
                order: get(2),
                uuid: get(3),
                instance: get(4),
                raw,
            }
        })
        .collect();
    Ok(applets)
}

/// Build the structured JSON value: `{ timestamp, applets: [...] }`.
fn to_json(applets: &[Applet]) -> Json {
    let items = applets
        .iter()
        .map(|a| {
            Json::Object(vec![
                ("panel".into(), a.panel.clone().into()),
                ("position".into(), a.position.clone().into()),
                ("order".into(), a.order.clone().into()),
                ("uuid".into(), a.uuid.clone().into()),
                ("instance".into(), a.instance.clone().into()),
                ("definition".into(), a.raw.clone().into()),
            ])
        })
        .collect();
    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("applets".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table.
fn to_table(applets: &[Applet]) -> String {
    let rows: Vec<Vec<String>> = applets
        .iter()
        .map(|a| {
            vec![
                a.panel.clone(),
                a.position.clone(),
                a.order.clone(),
                a.uuid.clone(),
                a.instance.clone(),
            ]
        })
        .collect();
    table::render(
        &["Panel", "Position", "Order", "UUID", "Instance"],
        &rows,
        &[
            Align::Left,
            Align::Left,
            Align::Right,
            Align::Left,
            Align::Right,
        ],
    )
}

const HELP: &str = "\
get-cinnamon-applet — enabled Cinnamon panel applets, table or JSON.
Usage: get-cinnamon-applet [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: gsettings get org.cinnamon enabled-applets.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-cinnamon-applet: {e}");
            eprintln!("try 'get-cinnamon-applet --help'");
            std::process::exit(2);
        }
    };

    let applets = match get_applets() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("get-cinnamon-applet: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&applets), || to_table(&applets));
}
