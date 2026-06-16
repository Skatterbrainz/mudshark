//! get-network-interface — configured OS network interfaces.
//! Source: `ip -o link` (per-interface link state) and `ip -o addr`
//! (per-address records). The `-o` flag keeps each record on one line.
//! Output/formatting helpers are shared via the `mudshark-core` crate.

use mudshark_core::json::Json;
use mudshark_core::table::{self, Align};
use mudshark_core::{cli, proc, time};

struct Address {
    family: String,
    address: String,
    prefixlen: u64,
}

struct Interface {
    name: String,
    state: String,
    mac: String,
    mtu: u64,
    addresses: Vec<Address>,
}

/// Parse one `ip -o link` record into (name, state, mac, mtu).
///
/// Example line:
///   `2: enp0s31f6: <...> mtu 1500 qdisc ... state UP mode ...\    link/ether aa:bb ...`
fn parse_link(line: &str) -> Option<(String, String, String, u64)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 2 {
        return None;
    }
    let name = fields[1].trim_end_matches(':').to_string();
    let mut state = String::new();
    let mut mac = String::new();
    let mut mtu = 0u64;

    let mut i = 0;
    while i < fields.len() {
        match fields[i] {
            "mtu" => {
                if let Some(v) = fields.get(i + 1) {
                    mtu = v.parse().unwrap_or(0);
                }
            }
            "state" => {
                if let Some(v) = fields.get(i + 1) {
                    state = v.to_string();
                }
            }
            f if f.starts_with("link/") => {
                if let Some(v) = fields.get(i + 1) {
                    mac = v.to_string();
                }
            }
            _ => {}
        }
        i += 1;
    }
    Some((name, state, mac, mtu))
}

/// Parse one `ip -o addr` record into (interface name, address).
///
/// Example line:
///   `2: enp0s31f6    inet 10.0.0.2/22 brd ... scope global ...`
fn parse_addr(line: &str) -> Option<(String, Address)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    let name = fields[1].to_string();
    let family = match fields[2] {
        "inet" => "ipv4".to_string(),
        "inet6" => "ipv6".to_string(),
        other => other.to_string(),
    };
    let (address, prefixlen) = match fields[3].split_once('/') {
        Some((a, p)) => (a.to_string(), p.parse().unwrap_or(0)),
        None => (fields[3].to_string(), 0),
    };
    Some((
        name,
        Address {
            family,
            address,
            prefixlen,
        },
    ))
}

/// Collect interfaces from `ip`, attaching addresses to their interface.
fn get_interfaces() -> Result<Vec<Interface>, String> {
    let link_text = proc::run("ip", &["-o", "link"])?;
    let addr_text = proc::run("ip", &["-o", "addr"])?;

    let mut interfaces: Vec<Interface> = Vec::new();
    for line in link_text.lines() {
        if let Some((name, state, mac, mtu)) = parse_link(line) {
            interfaces.push(Interface {
                name,
                state,
                mac,
                mtu,
                addresses: Vec::new(),
            });
        }
    }

    for line in addr_text.lines() {
        if let Some((name, addr)) = parse_addr(line) {
            if let Some(iface) = interfaces.iter_mut().find(|i| i.name == name) {
                iface.addresses.push(addr);
            }
        }
    }

    Ok(interfaces)
}

/// Build the structured JSON value: `{ timestamp, interfaces: [...] }`.
fn to_json(interfaces: &[Interface]) -> Json {
    let items = interfaces
        .iter()
        .map(|i| {
            let addrs = i
                .addresses
                .iter()
                .map(|a| {
                    Json::Object(vec![
                        ("family".into(), a.family.clone().into()),
                        ("address".into(), a.address.clone().into()),
                        ("prefixlen".into(), a.prefixlen.into()),
                    ])
                })
                .collect();
            Json::Object(vec![
                ("name".into(), i.name.clone().into()),
                ("state".into(), i.state.clone().into()),
                ("mac".into(), i.mac.clone().into()),
                ("mtu".into(), i.mtu.into()),
                ("addresses".into(), Json::Array(addrs)),
            ])
        })
        .collect();

    Json::Object(vec![
        ("timestamp".into(), time::now_utc_iso8601().into()),
        ("interfaces".into(), Json::Array(items)),
    ])
}

/// Build the human-readable table; addresses are summarised per row.
fn to_table(interfaces: &[Interface]) -> String {
    let rows: Vec<Vec<String>> = interfaces
        .iter()
        .map(|i| {
            let addrs = i
                .addresses
                .iter()
                .map(|a| format!("{}/{}", a.address, a.prefixlen))
                .collect::<Vec<_>>()
                .join(", ");
            vec![
                i.name.clone(),
                i.state.clone(),
                i.mac.clone(),
                i.mtu.to_string(),
                addrs,
            ]
        })
        .collect();

    table::render(
        &["Name", "State", "MAC", "MTU", "Addresses"],
        &rows,
        &[
            Align::Left,
            Align::Left,
            Align::Left,
            Align::Right,
            Align::Left,
        ],
    )
}

const HELP: &str = "\
get-network-interface — configured OS network interfaces, table or JSON.
Usage: get-network-interface [--json | -c|--compact | -o table|json|json-compact] [-h|--help]
Source: ip -o link and ip -o addr.";

fn main() {
    let opts = match cli::parse(HELP) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("get-network-interface: {e}");
            eprintln!("try 'get-network-interface --help'");
            std::process::exit(2);
        }
    };

    let interfaces = match get_interfaces() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("get-network-interface: {e}");
            std::process::exit(1);
        }
    };

    cli::emit(opts.format, || to_json(&interfaces), || to_table(&interfaces));
}
