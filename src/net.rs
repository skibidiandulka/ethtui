use anyhow::{Context, Result};
use if_addrs::IfAddr;
use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct EthernetDevice {
    pub name: String,
    pub operstate: String,
    pub carrier: Option<bool>,
    pub mac: Option<String>,
    pub speed_mbps: Option<u32>,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    pub gateway_v4: Option<String>,
    pub dns: Vec<String>,
}

fn is_physical_iface(name: &str) -> bool {
    Path::new("/sys/class/net")
        .join(name)
        .join("device")
        .exists()
}

fn is_wifi_iface(name: &str) -> bool {
    let p = Path::new("/sys/class/net").join(name);
    p.join("wireless").is_dir() || p.join("phy80211").exists()
}

fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_bool(path: impl AsRef<Path>) -> Option<bool> {
    read_to_string(path).and_then(|s| match s.as_str() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    })
}

fn read_u32(path: impl AsRef<Path>) -> Option<u32> {
    read_to_string(path).and_then(|s| s.parse::<u32>().ok())
}

fn list_dns_servers() -> Vec<String> {
    let resolv = fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
    resolv
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("nameserver ") {
                line.split_whitespace().nth(1).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn parse_default_gateway_v4_for_iface(iface: &str) -> Option<Ipv4Addr> {
    // /proc/net/route is stable, and avoids shelling out to `ip route`.
    let content = fs::read_to_string("/proc/net/route").ok()?;
    for (i, line) in content.lines().enumerate() {
        // Skip header
        if i == 0 {
            continue;
        }

        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }

        let ifname = cols[0];
        let destination = cols[1];
        let gateway_hex = cols[2];

        if ifname != iface {
            continue;
        }

        // Destination 00000000 means default route.
        if destination != "00000000" {
            continue;
        }

        // Little-endian hex.
        let gw = u32::from_str_radix(gateway_hex, 16).ok()?;
        let b = gw.to_le_bytes();
        return Some(Ipv4Addr::new(b[0], b[1], b[2], b[3]));
    }

    None
}

fn list_ip_addrs_for_iface(iface: &str) -> Result<(Vec<String>, Vec<String>)> {
    let ifas = if_addrs::get_if_addrs().context("get_if_addrs failed")?;
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();

    for ifa in ifas {
        if ifa.name != iface {
            continue;
        }
        match ifa.addr {
            IfAddr::V4(a) => {
                let prefix = v4_netmask_to_prefix(a.netmask);
                v4.push(format!("{}/{}", a.ip, prefix));
            }
            IfAddr::V6(a) => {
                let prefix = v6_netmask_to_prefix(a.netmask);
                v6.push(format!("{}/{}", a.ip, prefix));
            }
        }
    }

    Ok((v4, v6))
}

fn v4_netmask_to_prefix(mask: Ipv4Addr) -> u8 {
    let bits = u32::from_be_bytes(mask.octets());
    bits.count_ones() as u8
}

fn v6_netmask_to_prefix(mask: std::net::Ipv6Addr) -> u8 {
    mask.octets()
        .into_iter()
        .map(|b| b.count_ones() as u16)
        .sum::<u16>() as u8
}

pub fn list_ethernet_devices() -> Result<Vec<EthernetDevice>> {
    let mut devices = Vec::new();

    for entry in fs::read_dir("/sys/class/net").context("read_dir /sys/class/net failed")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }

        if !is_physical_iface(&name) {
            continue;
        }

        if is_wifi_iface(&name) {
            continue;
        }

        let base = Path::new("/sys/class/net").join(&name);

        let operstate = read_to_string(base.join("operstate")).unwrap_or_else(|| "?".into());
        let carrier = read_bool(base.join("carrier"));
        let mac = read_to_string(base.join("address"));
        let speed_mbps = read_u32(base.join("speed"));

        let (ipv4, ipv6) = list_ip_addrs_for_iface(&name).unwrap_or_default();
        let gateway_v4 = parse_default_gateway_v4_for_iface(&name).map(|g| g.to_string());
        let dns = list_dns_servers();

        devices.push(EthernetDevice {
            name,
            operstate,
            carrier,
            mac,
            speed_mbps,
            ipv4,
            ipv6,
            gateway_v4,
            dns,
        });
    }

    devices.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(devices)
}
