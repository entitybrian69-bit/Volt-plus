use crate::hud::{HudAtlas, HudBatch, rgba};
use crate::ui::{Caret, Scroll, list_background, scrollbar, separators};
use horizon_protocol::connection::{
    VpnEndpoint, active_vpn, probe_vpn_exit_ip, set_active_vpn, test_vpn_endpoint,
};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const HEADER: f32 = 46.0;
const FOOTER: f32 = 46.0;
const ROW_WIDTH: f32 = 320.0;
const ROW_HEIGHT: f32 = 24.0;
const VPN_CONFIG_FILE: &str = "horizon-vpn.json";
const VPN_CUSTOM_FIELD: crate::clipboard::FieldId = ("vpn.custom", 0);

const PRIORITY_COUNTRIES: &[&str] = &[
    "IN", "SG", "US", "AE", "GB", "DE", "JP", "CA", "NL", "FR", "AU", "BR", "KR", "ID", "HK",
    "TR", "PL", "FI", "SA", "TH", "VN", "MY", "PH",
];

const EXTRA_COUNTRIES: &[(&str, &str)] = &[
    ("", "More..."),
    ("GB", "UK (GB)"),
    ("DE", "Germany"),
    ("JP", "Japan"),
    ("CA", "Canada"),
    ("NL", "Netherlands"),
    ("FR", "France"),
    ("AU", "Australia"),
    ("BR", "Brazil"),
    ("KR", "S. Korea"),
    ("ID", "Indonesia"),
    ("HK", "Hong Kong"),
    ("TR", "Turkey"),
    ("PL", "Poland"),
    ("FI", "Finland"),
    ("SA", "Saudi Arabia"),
    ("LOCAL", "Local App"),
];

fn country_display(code: &str) -> &'static str {
    match code {
        "IN" => "India",
        "SG" => "Singapore",
        "US" => "America (USA)",
        "AE" => "Dubai / UAE",
        "GB" | "UK" => "United Kingdom",
        "DE" => "Germany",
        "JP" => "Japan",
        "CA" => "Canada",
        "NL" => "Netherlands",
        "FR" => "France",
        "AU" => "Australia",
        "BR" => "Brazil",
        "KR" => "South Korea",
        "ID" => "Indonesia",
        "HK" => "Hong Kong",
        "TR" => "Turkey",
        "PL" => "Poland",
        "FI" => "Finland",
        "SA" => "Saudi Arabia",
        "TH" => "Thailand",
        "VN" => "Vietnam",
        "MY" => "Malaysia",
        "PH" => "Philippines",
        "SE" => "Sweden",
        "CH" => "Switzerland",
        "NO" => "Norway",
        "IT" => "Italy",
        "ES" => "Spain",
        "MX" => "Mexico",
        "RU" => "Russia",
        "UA" => "Ukraine",
        "TW" => "Taiwan",
        "ZA" => "South Africa",
        "LOCAL" => "Local Device VPN",
        "CUSTOM" => "Custom Tunnel",
        _ => "Global",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VpnProto {
    Socks5,
    HttpConnect,
    FastDns,
}

impl VpnProto {
    fn tag(self) -> &'static str {
        match self {
            Self::Socks5 => "SOCKS5",
            Self::HttpConnect => "HTTP",
            Self::FastDns => "DNS",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VpnNode {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub country_code: String,
    pub region: String,
    pub proto: VpnProto,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub local: bool,
    #[serde(skip)]
    pub ping_ms: Option<u128>,
    #[serde(skip)]
    pub testing: bool,
    #[serde(skip)]
    pub working: Option<bool>,
}

impl VpnNode {
    pub fn to_endpoint(&self) -> VpnEndpoint {
        match self.proto {
            VpnProto::Socks5 => VpnEndpoint::Socks5 {
                label: self.name.clone(),
                host: self.host.clone(),
                port: self.port,
            },
            VpnProto::HttpConnect => VpnEndpoint::HttpConnect {
                label: self.name.clone(),
                host: self.host.clone(),
                port: self.port,
            },
            VpnProto::FastDns => VpnEndpoint::FastDns {
                label: self.name.clone(),
                dns_ip: self.host.clone(),
            },
        }
    }

    fn matches_endpoint(&self, ep: &VpnEndpoint) -> bool {
        match (self.proto, ep) {
            (VpnProto::Socks5, VpnEndpoint::Socks5 { host, port, .. }) => {
                self.host == *host && self.port == *port
            }
            (VpnProto::HttpConnect, VpnEndpoint::HttpConnect { host, port, .. }) => {
                self.host == *host && self.port == *port
            }
            (VpnProto::FastDns, VpnEndpoint::FastDns { dns_ip, .. }) => self.host == *dns_ip,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct VpnDiskConfig {
    active: Option<VpnNode>,
    custom_nodes: Vec<VpnNode>,
}

fn load_disk_config() -> VpnDiskConfig {
    std::fs::read_to_string(VPN_CONFIG_FILE)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_disk_config(active: Option<&VpnNode>, nodes: &[VpnNode]) {
    let custom_nodes: Vec<VpnNode> = nodes.iter().filter(|n| n.custom).cloned().collect();
    let cfg = VpnDiskConfig {
        active: active.cloned(),
        custom_nodes,
    };
    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        let _ = std::fs::write(VPN_CONFIG_FILE, json);
    }
}

/// Restores any previously enabled VPN route from `horizon-vpn.json` on startup.
pub fn restore_saved_vpn() {
    let cfg = load_disk_config();
    set_active_vpn(cfg.active.map(|n| n.to_endpoint()));
}

fn local_daemon_candidates() -> Vec<VpnNode> {
    let local_tunnels = [
        ("local-warp", "Cloudflare WARP Tunnel", "127.0.0.1", 40000, VpnProto::Socks5),
        ("local-clash-s", "Clash / Mihomo SOCKS5", "127.0.0.1", 7890, VpnProto::Socks5),
        ("local-clash-h", "Clash / Mihomo HTTP", "127.0.0.1", 7891, VpnProto::HttpConnect),
        ("local-v2ray-s", "V2Ray / NekoBox SOCKS5", "127.0.0.1", 10808, VpnProto::Socks5),
        ("local-v2ray-h", "V2Ray / NekoBox HTTP", "127.0.0.1", 10809, VpnProto::HttpConnect),
        ("local-tor", "Tor / Orbot Tunnel", "127.0.0.1", 9050, VpnProto::Socks5),
        ("local-ss", "Shadowsocks / Sing-box", "127.0.0.1", 1080, VpnProto::Socks5),
    ];
    local_tunnels
        .into_iter()
        .map(|(id, name, host, port, proto)| VpnNode {
            id: id.into(),
            name: name.into(),
            country_code: "LOCAL".into(),
            region: format!("Local {host}:{port}"),
            proto,
            host: host.into(),
            port,
            custom: false,
            local: true,
            ping_ms: None,
            testing: true,
            working: None,
        })
        .collect()
}

pub fn parse_custom_node(raw: &str) -> Option<VpnNode> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let (proto, rest) = if let Some(r) = s
        .strip_prefix("socks5://")
        .or_else(|| s.strip_prefix("socks://"))
    {
        (VpnProto::Socks5, r)
    } else if let Some(r) = s
        .strip_prefix("http://")
        .or_else(|| s.strip_prefix("https://"))
    {
        (VpnProto::HttpConnect, r)
    } else if let Some(r) = s.strip_prefix("dns://") {
        (VpnProto::FastDns, r)
    } else {
        (VpnProto::Socks5, s)
    };
    let rest = rest.split('/').next().unwrap_or(rest).trim();
    if proto == VpnProto::FastDns {
        let host = rest.split(':').next().unwrap_or(rest).trim();
        if host.is_empty() {
            return None;
        }
        return Some(VpnNode {
            id: format!("custom-dns-{host}"),
            name: format!("Custom DNS ({host})"),
            country_code: "CUSTOM".into(),
            region: "Custom Fast DNS".into(),
            proto,
            host: host.into(),
            port: 53,
            custom: true,
            local: false,
            ping_ms: None,
            testing: true,
            working: None,
        });
    }
    let (host, port_str) = rest.rsplit_once(':')?;
    let host = host.trim().trim_matches(|c| c == '[' || c == ']');
    let port: u16 = port_str.trim().parse().ok()?;
    if host.is_empty() || port == 0 {
        return None;
    }
    Some(VpnNode {
        id: format!("custom-{}-{host}:{port}", proto.tag().to_lowercase()),
        name: format!("Custom {} ({host})", proto.tag()),
        country_code: "CUSTOM".into(),
        region: format!("Custom {host}:{port}"),
        proto,
        host: host.into(),
        port,
        custom: true,
        local: host == "127.0.0.1" || host == "localhost",
        ping_ms: None,
        testing: true,
        working: None,
    })
}

#[derive(Clone)]
struct RawCandidate {
    proto: VpnProto,
    host: String,
    port: u16,
    country_code: String,
    city: String,
}

fn make_node(c: RawCandidate) -> VpnNode {
    let cc = c.country_code.to_ascii_uppercase();
    let country_name = country_display(&cc);
    let label = if c.city.is_empty() {
        format!("[{cc}] {country_name}")
    } else {
        format!("[{cc}] {country_name} · {}", c.city)
    };
    VpnNode {
        id: format!("vpn-{}-{}-{}:{}", cc, c.proto.tag(), c.host, c.port),
        name: label,
        country_code: cc,
        region: format!("{} · {}:{}", c.proto.tag(), c.host, c.port),
        proto: c.proto,
        host: c.host,
        port: c.port,
        custom: false,
        local: false,
        ping_ms: None,
        testing: true,
        working: None,
    }
}

fn fetch_live_country_tunnels(focus_country: &str) -> Vec<VpnNode> {
    let agent: ureq::Agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout(Duration::from_secs(6))
        .build();

    let mut urls: Vec<(usize, String, &'static str)> = Vec::new();
    // 1. Proxifly geo-tagged JSON (fast jsDelivr CDN + GitHub raw, full country & city metadata)
    urls.push((
        0,
        "https://cdn.jsdelivr.net/gh/proxifly/free-proxy-list@main/proxies/protocols/socks5/data.json".into(),
        "proxifly_socks5",
    ));
    urls.push((
        1,
        "https://cdn.jsdelivr.net/gh/proxifly/free-proxy-list@main/proxies/protocols/http/data.json".into(),
        "proxifly_http",
    ));
    // 2. ProxyScrape v4 rich JSON (countryCode + city + uptime + socks5/http)
    urls.push((
        2,
        "https://api.proxyscrape.com/v4/free-proxy-list/get?request=display_proxies&proxy_format=protocolipport&format=json&protocol=socks5,http&timeout=4000".into(),
        "proxyscrape_v4",
    ));
    // 3. GeoNode rich JSON filtered for popular countries first
    urls.push((
        3,
        "https://proxylist.geonode.com/api/proxy-list?limit=150&page=1&sort_by=lastChecked&sort_type=desc&protocols=socks5%2Chttp".into(),
        "geonode",
    ));

    // 4. Dedicated per-country feeds fetched concurrently so IN, SG, US, AE (Dubai), GB, DE, JP, etc.
    // always get rich country-specific candidates!
    let mut country_list: Vec<&str> = PRIORITY_COUNTRIES.to_vec();
    if !focus_country.is_empty() && focus_country != "LOCAL" && !country_list.contains(&focus_country) {
        country_list.insert(0, focus_country);
    }
    for (idx, cc) in country_list.iter().take(16).enumerate() {
        urls.push((
            10 + idx * 2,
            format!("https://api.proxyscrape.com/v2/?request=displayproxies&protocol=socks5&timeout=4000&country={cc}&ssl=all&anonymity=all"),
            "ps_v2_socks5",
        ));
        urls.push((
            11 + idx * 2,
            format!("https://api.proxyscrape.com/v2/?request=displayproxies&protocol=http&timeout=4000&country={cc}&ssl=all&anonymity=all"),
            "ps_v2_http",
        ));
    }

    let collected = Arc::new(Mutex::new(Vec::<RawCandidate>::new()));
    let country_tags: Vec<String> = urls
        .iter()
        .map(|(_, u, _)| {
            u.split("country=")
                .nth(1)
                .and_then(|s| s.split('&').next())
                .unwrap_or("")
                .to_owned()
        })
        .collect();

    std::thread::scope(|s| {
        for ((_, url, kind), cc_hint) in urls.into_iter().zip(country_tags.into_iter()) {
            let agent = agent.clone();
            let collected = Arc::clone(&collected);
            s.spawn(move || {
                let Ok(resp) = agent.get(&url).call() else {
                    return;
                };
                let Ok(text) = resp.into_string() else {
                    return;
                };
                let mut local = Vec::new();
                match kind {
                    "proxifly_socks5" | "proxifly_http" => {
                        let proto = if kind == "proxifly_socks5" {
                            VpnProto::Socks5
                        } else {
                            VpnProto::HttpConnect
                        };
                        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                            for item in arr.into_iter().take(240) {
                                let Some(ip) = item.get("ip").and_then(|v| v.as_str()) else {
                                    continue;
                                };
                                let Some(port) = item.get("port").and_then(|v| v.as_u64()).and_then(|n| u16::try_from(n).ok()) else {
                                    continue;
                                };
                                let cc = item
                                    .get("geolocation")
                                    .and_then(|g| g.get("country"))
                                    .and_then(|v| v.as_str())
                                    .or_else(|| item.get("country").and_then(|v| v.as_str()))
                                    .unwrap_or("GL");
                                let city = item
                                    .get("geolocation")
                                    .and_then(|g| g.get("city"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                local.push(RawCandidate {
                                    proto,
                                    host: ip.to_owned(),
                                    port,
                                    country_code: cc.to_ascii_uppercase(),
                                    city: city.to_owned(),
                                });
                            }
                        }
                    }
                    "proxyscrape_v4" => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
                            && let Some(arr) = json.get("proxies").and_then(|v| v.as_array())
                        {
                            for item in arr.iter().take(240) {
                                let Some(ip) = item.get("ip").and_then(|v| v.as_str()) else {
                                    continue;
                                };
                                let Some(port) = item.get("port").and_then(|v| v.as_u64()).and_then(|n| u16::try_from(n).ok()) else {
                                    continue;
                                };
                                let proto_str = item.get("protocol").and_then(|v| v.as_str()).unwrap_or("http");
                                let proto = if proto_str.eq_ignore_ascii_case("socks5") {
                                    VpnProto::Socks5
                                } else {
                                    VpnProto::HttpConnect
                                };
                                let cc = item
                                    .get("ip_data")
                                    .and_then(|d| d.get("countryCode"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("GL");
                                let city = item
                                    .get("ip_data")
                                    .and_then(|d| d.get("city"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                local.push(RawCandidate {
                                    proto,
                                    host: ip.to_owned(),
                                    port,
                                    country_code: cc.to_ascii_uppercase(),
                                    city: city.to_owned(),
                                });
                            }
                        }
                    }
                    "geonode" => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
                            && let Some(arr) = json.get("data").and_then(|v| v.as_array())
                        {
                            for item in arr {
                                let Some(ip) = item.get("ip").and_then(|v| v.as_str()) else {
                                    continue;
                                };
                                let Some(port) = item.get("port").and_then(|v| {
                                    v.as_str()
                                        .and_then(|s| s.parse::<u16>().ok())
                                        .or_else(|| v.as_u64().and_then(|n| u16::try_from(n).ok()))
                                }) else {
                                    continue;
                                };
                                let cc = item.get("country").and_then(|v| v.as_str()).unwrap_or("GL");
                                let city = item.get("city").and_then(|v| v.as_str()).unwrap_or("");
                                let is_socks5 = item
                                    .get("protocols")
                                    .and_then(|v| v.as_array())
                                    .is_some_and(|ps| ps.iter().any(|p| p.as_str() == Some("socks5")));
                                local.push(RawCandidate {
                                    proto: if is_socks5 { VpnProto::Socks5 } else { VpnProto::HttpConnect },
                                    host: ip.to_owned(),
                                    port,
                                    country_code: cc.to_ascii_uppercase(),
                                    city: city.to_owned(),
                                });
                            }
                        }
                    }
                    _ => {
                        let proto = if kind == "ps_v2_socks5" {
                            VpnProto::Socks5
                        } else {
                            VpnProto::HttpConnect
                        };
                        for line in text.lines().take(25) {
                            let line = line.trim();
                            if let Some((h, p)) = line.rsplit_once(':')
                                && let Ok(port) = p.trim().parse::<u16>()
                                && !h.trim().is_empty()
                                && port > 0
                            {
                                local.push(RawCandidate {
                                    proto,
                                    host: h.trim().to_owned(),
                                    port,
                                    country_code: cc_hint.clone(),
                                    city: String::new(),
                                });
                            }
                        }
                    }
                }
                if !local.is_empty() {
                    collected.lock().unwrap().extend(local);
                }
            });
        }
    });

    // Deduplicate and round-robin interleave across countries so India, Singapore, America,
    // Dubai, UK, Germany, Japan, etc. are all probed early and evenly!
    let raw = std::mem::take(&mut *collected.lock().unwrap());
    let mut seen = HashSet::new();
    let mut by_country: HashMap<String, Vec<VpnNode>> = HashMap::new();
    for c in raw {
        if c.country_code.is_empty() || c.country_code == "GL" {
            continue;
        }
        if !seen.insert((c.proto.tag(), c.host.clone(), c.port)) {
            continue;
        }
        let cc = c.country_code.clone();
        by_country.entry(cc).or_default().push(make_node(c));
    }

    let mut ordered_ccs: Vec<String> = Vec::new();
    if !focus_country.is_empty() && by_country.contains_key(focus_country) {
        ordered_ccs.push(focus_country.to_owned());
    }
    for &cc in PRIORITY_COUNTRIES {
        if cc != focus_country && by_country.contains_key(cc) {
            ordered_ccs.push(cc.to_owned());
        }
    }
    let mut rest_ccs: Vec<String> = by_country
        .keys()
        .filter(|cc| !ordered_ccs.contains(cc))
        .cloned()
        .collect();
    rest_ccs.sort();
    ordered_ccs.extend(rest_ccs);

    let mut interleaved = Vec::new();
    let mut depth = 0usize;
    while interleaved.len() < 320 {
        let mut added = false;
        for cc in &ordered_ccs {
            if let Some(list) = by_country.get(cc)
                && let Some(node) = list.get(depth)
            {
                interleaved.push(node.clone());
                added = true;
                if interleaved.len() >= 320 {
                    break;
                }
            }
        }
        if !added {
            break;
        }
        depth += 1;
    }
    interleaved
}

#[derive(Clone)]
struct SharedState {
    nodes: Vec<VpnNode>,
    status: String,
    exit_info: Option<String>,
    scanning: bool,
    generation: u64,
}

#[derive(Clone)]
pub struct Screen {
    shared: Arc<Mutex<SharedState>>,
    pub country_filter: String,
    pub extra_idx: usize,
    pub selected: Option<String>,
    pub scroll: Cell<Scroll>,
    pub adding_custom: bool,
    pub custom_input: String,
    pub focused_at: Instant,
    last_click: Option<(Instant, String)>,
    drag_last_y: Option<f32>,
    drag_moved: f32,
}

impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VpnScreen")
            .field("country_filter", &self.country_filter)
            .field("selected", &self.selected)
            .field("adding_custom", &self.adding_custom)
            .finish()
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        let disk = load_disk_config();
        let mut nodes = disk.custom_nodes;
        if let Some(active) = &disk.active
            && !nodes
                .iter()
                .any(|n| n.host == active.host && n.port == active.port && n.proto == active.proto)
        {
            nodes.insert(0, active.clone());
        }
        let shared = Arc::new(Mutex::new(SharedState {
            nodes,
            status: "Fetching & verifying live country VPN servers...".into(),
            exit_info: None,
            scanning: false,
            generation: 0,
        }));
        let mut screen = Self {
            shared,
            country_filter: String::new(),
            extra_idx: 0,
            selected: None,
            scroll: Cell::new(Scroll::default()),
            adding_custom: false,
            custom_input: String::new(),
            focused_at: Instant::now(),
            last_click: None,
            drag_last_y: None,
            drag_moved: 0.0,
        };
        if active_vpn().is_some() {
            screen.verify_active_exit_ip();
        }
        screen.refresh_and_test();
        screen
    }

    pub fn verify_active_exit_ip(&self) {
        let Some(ep) = active_vpn() else {
            self.shared.lock().unwrap().exit_info = None;
            return;
        };
        let shared = Arc::clone(&self.shared);
        {
            let mut g = shared.lock().unwrap();
            g.exit_info = Some(format!("Verifying tunnel IP ({})...", ep.label()));
        }
        std::thread::Builder::new()
            .name("horizon-vpn-ip".into())
            .spawn(move || {
                let msg = match probe_vpn_exit_ip(&ep, Duration::from_secs(4)) {
                    Ok((ip, ms)) => format!("Exit IP: {ip} · {ms}ms ({})", ep.label()),
                    Err(e) => format!("{} ({e})", ep.label()),
                };
                if active_vpn().as_ref() == Some(&ep) {
                    shared.lock().unwrap().exit_info = Some(msg);
                }
            })
            .ok();
    }

    pub fn refresh_and_test(&mut self) {
        let focus_country = self.country_filter.clone();
        let mut guard = self.shared.lock().unwrap();
        guard.generation += 1;
        let generation = guard.generation;
        guard.scanning = true;
        guard.status = "Fetching live VPNs (India, Singapore, USA, Dubai & more)...".into();
        let preserved: Vec<VpnNode> = guard
            .nodes
            .iter()
            .filter(|n| n.custom)
            .cloned()
            .collect();
        guard.nodes = preserved.clone();
        drop(guard);

        let shared = Arc::clone(&self.shared);
        std::thread::Builder::new()
            .name("horizon-vpn-scan".into())
            .spawn(move || {
                // Step 1: Quickly probe any installed local VPN daemons + custom nodes (300ms)
                let mut quick_nodes = preserved;
                quick_nodes.extend(local_daemon_candidates());
                if !quick_nodes.is_empty() {
                    std::thread::scope(|s| {
                        for node in &mut quick_nodes {
                            s.spawn(|| {
                                let ep = node.to_endpoint();
                                let res = test_vpn_endpoint(&ep, Duration::from_millis(450));
                                node.testing = false;
                                match res {
                                    Ok(ms) => {
                                        node.working = Some(true);
                                        node.ping_ms = Some(ms);
                                    }
                                    Err(_) => {
                                        node.working = Some(false);
                                    }
                                }
                            });
                        }
                    });
                    let working_quick: Vec<VpnNode> = quick_nodes
                        .into_iter()
                        .filter(|n| n.custom || n.working == Some(true))
                        .collect();
                    let mut g = shared.lock().unwrap();
                    if g.generation != generation {
                        return;
                    }
                    g.nodes = working_quick;
                    sort_nodes(&mut g.nodes);
                }

                // Step 2: Fetch multi-country live SOCKS5 & HTTP tunnel nodes
                let candidates = fetch_live_country_tunnels(&focus_country);
                let total_candidates = candidates.len();
                if total_candidates == 0 {
                    let mut g = shared.lock().unwrap();
                    if g.generation == generation {
                        g.scanning = false;
                        g.status = "No tunnel feeds reachable; use Add Custom or check internet".into();
                    }
                    return;
                }
                {
                    let mut g = shared.lock().unwrap();
                    if g.generation != generation {
                        return;
                    }
                    g.status = format!("Verifying {total_candidates} country VPN servers...");
                }

                // Step 3: Stream verified working nodes progressively with 32 threads!
                let idx = std::sync::atomic::AtomicUsize::new(0);
                let tested = std::sync::atomic::AtomicUsize::new(0);
                let threads = total_candidates.min(32).max(1);
                std::thread::scope(|s| {
                    for _ in 0..threads {
                        s.spawn(|| {
                            loop {
                                if shared.lock().unwrap().generation != generation {
                                    break;
                                }
                                let i = idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                if i >= total_candidates {
                                    break;
                                }
                                let mut node = candidates[i].clone();
                                let ep = node.to_endpoint();
                                if let Ok(ms) = test_vpn_endpoint(&ep, Duration::from_millis(1800)) {
                                    node.testing = false;
                                    node.working = Some(true);
                                    node.ping_ms = Some(ms);
                                    let mut g = shared.lock().unwrap();
                                    if g.generation != generation {
                                        break;
                                    }
                                    if !g.nodes.iter().any(|existing| {
                                        existing.host == node.host
                                            && existing.port == node.port
                                            && existing.proto == node.proto
                                    }) {
                                        g.nodes.push(node);
                                        sort_nodes(&mut g.nodes);
                                    }
                                }
                                let done =
                                    tested.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                                if done % 12 == 0 {
                                    let mut g = shared.lock().unwrap();
                                    if g.generation == generation {
                                        let ok = g.nodes.iter().filter(|n| n.working == Some(true)).count();
                                        g.status = format!(
                                            "Verified {ok} working VPNs ({done}/{total_candidates} tested)..."
                                        );
                                    }
                                }
                            }
                        });
                    }
                });

                let mut g = shared.lock().unwrap();
                if g.generation == generation {
                    g.scanning = false;
                    sort_nodes(&mut g.nodes);
                    let ok = g.nodes.iter().filter(|n| n.working == Some(true)).count();
                    let countries: HashSet<&str> = g
                        .nodes
                        .iter()
                        .filter(|n| n.working == Some(true) && !n.country_code.is_empty())
                        .map(|n| n.country_code.as_str())
                        .collect();
                    g.status = format!(
                        "{ok} verified working VPNs across {} countries",
                        countries.len().max(1)
                    );
                }
            })
            .ok();
    }

    fn visible_nodes(&self) -> Vec<VpnNode> {
        let g = self.shared.lock().unwrap();
        g.nodes
            .iter()
            .filter(|n| {
                if self.country_filter.is_empty() {
                    true
                } else if self.country_filter == "GB" {
                    n.country_code == "GB" || n.country_code == "UK"
                } else {
                    n.country_code == self.country_filter
                }
            })
            .cloned()
            .collect()
    }

    pub fn list_area(w: f32, h: f32) -> (f32, f32, f32) {
        let top = HEADER;
        let bottom = (h - FOOTER).max(top + 40.0);
        let left = (w / 2.0 - ROW_WIDTH / 2.0).floor();
        (top, bottom, left)
    }

    pub fn buttons(&self, w: f32, h: f32) -> Vec<super::Button> {
        let (w, h) = (w.ceil(), h.ceil());
        let cx = (w / 2.0).floor();
        let b = |x: f32, y: f32, bw: f32, label: &str, enabled: bool, id: u8| super::Button {
            x,
            y,
            w: bw,
            label: label.to_owned(),
            enabled,
            id,
            icon: None,
            trailing: None,
        };
        if self.adding_custom {
            let valid = parse_custom_node(&self.custom_input).is_some();
            return vec![
                b(cx - 100.0, 146.0, 200.0, "Add & Connect VPN", valid, 110),
                b(cx - 100.0, 170.0, 200.0, "Cancel", true, 111),
            ];
        }
        let active = active_vpn();
        let visible = self.visible_nodes();
        let sel_node = self
            .selected
            .as_ref()
            .and_then(|id| visible.iter().find(|n| n.id == *id));
        let is_selected_active =
            sel_node.is_some_and(|n| active.as_ref().is_some_and(|a| n.matches_endpoint(a)));
        let toggle_label = if is_selected_active {
            "Disconnect"
        } else if sel_node.is_some() {
            "Connect"
        } else if active.is_some() {
            "Disconnect"
        } else {
            "Connect"
        };
        let can_toggle = sel_node.is_some() || active.is_some();
        let footer_y = h - FOOTER + 14.0;
        let extra_label = EXTRA_COUNTRIES[self.extra_idx].1;

        vec![
            // Country selector bar (row at y = 21)
            b(cx - 160.0, 21.0, 34.0, "All", !self.country_filter.is_empty(), 105),
            b(cx - 123.0, 21.0, 48.0, "India", self.country_filter != "IN", 106),
            b(cx - 72.0, 21.0, 62.0, "Singapore", self.country_filter != "SG", 107),
            b(cx - 7.0, 21.0, 40.0, "USA", self.country_filter != "US", 108),
            b(cx + 36.0, 21.0, 48.0, "Dubai", self.country_filter != "AE", 109),
            b(cx + 87.0, 21.0, 73.0, extra_label, true, 112),
            // Footer controls
            b(cx - 160.0, footer_y, 78.0, toggle_label, can_toggle, 100),
            b(cx - 79.0, footer_y, 64.0, "Test IP", active.is_some(), 104),
            b(cx - 12.0, footer_y, 58.0, "Refresh", true, 101),
            b(cx + 49.0, footer_y, 62.0, "Custom", true, 102),
            b(cx + 114.0, footer_y, 46.0, "Done", true, 103),
        ]
    }

    pub fn draw(
        &self,
        atlas: &HudAtlas,
        batch: &mut HudBatch,
        caret: &mut Caret,
        started: Instant,
        cursor: (f32, f32),
    ) {
        let (w, h) = (batch.width.ceil(), batch.height.ceil());
        let cx = (w / 2.0).floor();

        if self.adding_custom {
            let title = "Add Custom VPN Server";
            let tw = HudBatch::text_width(atlas, title);
            batch.text(atlas, title, cx - (tw.ceil() / 2.0).floor(), 24.0, u32::MAX, true);

            let label = "socks5://ip:port or http://ip:port";
            batch.text(
                atlas,
                label,
                cx - 100.0,
                84.0,
                rgba(160, 160, 160, 255),
                true,
            );
            caret.draw(
                batch,
                atlas,
                (VPN_CUSTOM_FIELD, &self.custom_input, 128),
                [cx - 100.0, 98.0, 200.0, 20.0],
                true,
                self.focused_at,
            );
            let hint = "Examples: socks5://1.2.3.4:1080 or http://5.6.7.8:8080";
            let hw = HudBatch::text_width(atlas, hint);
            batch.text(
                atlas,
                hint,
                cx - (hw.ceil() / 2.0).floor(),
                125.0,
                rgba(140, 140, 140, 255),
                true,
            );
            return;
        }

        let active = active_vpn();
        let (status, exit_info) = {
            let g = self.shared.lock().unwrap();
            (g.status.clone(), g.exit_info.clone())
        };
        let header_text = if let Some(ep) = &active {
            exit_info.unwrap_or_else(|| format!("VPN: ON · {}", ep.label()))
        } else {
            "VPN Networks (Select a Country Server)".to_owned()
        };
        let header_color = if active.is_some() {
            rgba(85, 255, 85, 255)
        } else {
            u32::MAX
        };
        let tw = HudBatch::text_width(atlas, &header_text);
        batch.text(
            atlas,
            &header_text,
            cx - (tw.ceil() / 2.0).floor(),
            7.0,
            header_color,
            true,
        );

        let (top, bottom, left) = Self::list_area(w, h);
        let visible = self.visible_nodes();
        let content = visible.len() as f32 * ROW_HEIGHT + 4.0;
        let mut scroll = self.scroll.get();
        scroll.clamp(top, bottom, content);
        self.scroll.set(scroll);

        list_background(batch, atlas, top, bottom, false);
        let first = batch.vertices.len();

        let millis = started.elapsed().as_millis();
        let hovered_row = self.row_at(cursor, (w, h), &visible);

        if visible.is_empty() {
            let empty_msg = if self.country_filter.is_empty() {
                "Scanning & verifying working country VPNs..."
            } else {
                "No verified servers in this country yet — scanning or click Refresh"
            };
            let ew = HudBatch::text_width(atlas, empty_msg);
            batch.text(
                atlas,
                empty_msg,
                cx - (ew.ceil() / 2.0).floor(),
                top + 24.0,
                rgba(170, 170, 170, 255),
                true,
            );
        }

        for (row_idx, node) in visible.iter().enumerate() {
            let y = top + 2.0 - scroll.amount.trunc() + row_idx as f32 * ROW_HEIGHT;
            if y + ROW_HEIGHT < top || y > bottom {
                continue;
            }
            let is_selected = self.selected.as_deref() == Some(node.id.as_str());
            let is_active = active.as_ref().is_some_and(|a| node.matches_endpoint(a));
            let is_hovered = hovered_row == Some(row_idx);

            if is_selected {
                batch.fill(atlas, left, y, ROW_WIDTH, ROW_HEIGHT, u32::MAX);
                batch.fill(
                    atlas,
                    left + 1.0,
                    y + 1.0,
                    ROW_WIDTH - 2.0,
                    ROW_HEIGHT - 2.0,
                    rgba(0, 0, 0, 255),
                );
            } else if is_active {
                batch.fill(
                    atlas,
                    left + 1.0,
                    y + 1.0,
                    ROW_WIDTH - 2.0,
                    ROW_HEIGHT - 2.0,
                    rgba(22, 54, 24, 210),
                );
            } else if is_hovered {
                batch.fill(
                    atlas,
                    left + 1.0,
                    y + 1.0,
                    ROW_WIDTH - 2.0,
                    ROW_HEIGHT - 2.0,
                    rgba(255, 255, 255, 26),
                );
            }

            let title_color = if is_active {
                rgba(85, 255, 85, 255)
            } else {
                u32::MAX
            };
            let title_text = if is_active {
                format!("{} (CONNECTED)", node.name)
            } else {
                node.name.clone()
            };
            batch.text(atlas, &title_text, left + 5.0, y + 3.0, title_color, false);
            batch.text(
                atlas,
                &node.region,
                left + 5.0,
                y + 13.0,
                rgba(160, 160, 160, 255),
                false,
            );

            let icon_x = left + ROW_WIDTH - 16.0;
            let (sprite, ping_label, ping_color) = if node.testing {
                let frame = ((millis / 150) % 5) + 1;
                (
                    format!("gui/sprites/icon/pinging_{frame}"),
                    "...".to_owned(),
                    rgba(170, 170, 170, 255),
                )
            } else if let Some(ms) = node.ping_ms {
                let bars = if ms < 90 {
                    5
                } else if ms < 180 {
                    4
                } else if ms < 320 {
                    3
                } else if ms < 600 {
                    2
                } else {
                    1
                };
                let color = if ms < 180 {
                    rgba(85, 255, 85, 255)
                } else if ms < 400 {
                    rgba(255, 255, 85, 255)
                } else {
                    rgba(255, 170, 85, 255)
                };
                (
                    format!("gui/sprites/icon/ping_{bars}"),
                    format!("{ms}ms"),
                    color,
                )
            } else {
                (
                    "gui/sprites/icon/unreachable".to_owned(),
                    "Check".into(),
                    rgba(150, 150, 150, 255),
                )
            };
            batch.sprite_scaled(atlas, &sprite, icon_x, y + 3.0, 10.0, 8.0, u32::MAX);
            let pw = HudBatch::text_width(atlas, &ping_label).ceil();
            batch.text(
                atlas,
                &ping_label,
                icon_x - pw - 4.0,
                y + 3.0,
                ping_color,
                false,
            );
        }

        batch.clip_since(first, 0.0, top, w, bottom);
        separators(batch, atlas, top, bottom, false);
        scrollbar(
            batch,
            atlas,
            left + ROW_WIDTH + 8.0,
            top,
            bottom,
            content,
            scroll.amount,
        );

        let sw = HudBatch::text_width(atlas, &status);
        batch.text(
            atlas,
            &status,
            cx - (sw.ceil() / 2.0).floor(),
            bottom + 2.0,
            rgba(180, 180, 180, 255),
            true,
        );
    }

    fn row_at(
        &self,
        cursor: (f32, f32),
        (w, h): (f32, f32),
        visible: &[VpnNode],
    ) -> Option<usize> {
        let (top, bottom, left) = Self::list_area(w, h);
        let (x, y) = cursor;
        if x < left || x >= left + ROW_WIDTH || y < top || y >= bottom {
            return None;
        }
        let rel = y - (top + 2.0 - self.scroll.get().amount.trunc());
        if rel < 0.0 {
            return None;
        }
        let idx = (rel / ROW_HEIGHT) as usize;
        (idx < visible.len()).then_some(idx)
    }

    pub fn press(&mut self, cursor: (f32, f32), size: (f32, f32)) -> bool {
        if self.adding_custom {
            return false;
        }
        let (w, h) = (size.0.ceil(), size.1.ceil());
        let (top, bottom, left) = Self::list_area(w, h);
        let visible = self.visible_nodes();
        let content = visible.len() as f32 * ROW_HEIGHT + 4.0;
        let mut scroll = self.scroll.get();
        if scroll.press(cursor, left + ROW_WIDTH + 8.0, top, bottom, content) {
            self.scroll.set(scroll);
            self.drag_last_y = None;
            self.drag_moved = 0.0;
            return true;
        }
        if cursor.0 >= left && cursor.0 < left + ROW_WIDTH && cursor.1 >= top && cursor.1 < bottom {
            self.drag_last_y = Some(cursor.1);
            self.drag_moved = 0.0;
        }
        false
    }

    pub fn drag(&mut self, y: f32, size: (f32, f32)) {
        if self.adding_custom {
            return;
        }
        let (w, h) = (size.0.ceil(), size.1.ceil());
        let (top, bottom, _) = Self::list_area(w, h);
        let content = self.visible_nodes().len() as f32 * ROW_HEIGHT + 4.0;
        let mut scroll = self.scroll.get();
        if scroll.held() {
            scroll.drag(y, top, bottom, content);
            self.scroll.set(scroll);
            return;
        }
        if let Some(last_y) = self.drag_last_y {
            let dy = last_y - y;
            self.drag_moved += dy.abs();
            self.drag_last_y = Some(y);
            scroll.amount += dy;
            scroll.clamp(top, bottom, content);
            self.scroll.set(scroll);
        }
    }

    pub fn wheel(&mut self, delta: f32, cursor: (f32, f32), size: (f32, f32)) {
        if self.adding_custom {
            return;
        }
        let (w, h) = (size.0.ceil(), size.1.ceil());
        let (top, bottom, _) = Self::list_area(w, h);
        if cursor.1 < top || cursor.1 >= bottom {
            return;
        }
        let content = self.visible_nodes().len() as f32 * ROW_HEIGHT + 4.0;
        let mut scroll = self.scroll.get();
        scroll.wheel(delta, ROW_HEIGHT * 1.5, top, bottom, content);
        self.scroll.set(scroll);
    }

    pub fn release(&mut self) {
        let mut scroll = self.scroll.get();
        scroll.release();
        self.scroll.set(scroll);
        self.drag_last_y = None;
    }

    fn connect_or_disconnect_node(&mut self, node: &VpnNode) {
        let active = active_vpn();
        let already_active = active.as_ref().is_some_and(|a| node.matches_endpoint(a));
        if already_active {
            set_active_vpn(None);
            let mut g = self.shared.lock().unwrap();
            g.exit_info = None;
            save_disk_config(None, &g.nodes);
        } else {
            set_active_vpn(Some(node.to_endpoint()));
            {
                let g = self.shared.lock().unwrap();
                save_disk_config(Some(node), &g.nodes);
            }
            self.verify_active_exit_ip();
        }
    }

    /// Handles a click on the VPN screen. Returns `true` to return to the Title screen.
    pub fn click(&mut self, hit: Option<u8>, x: f32, y: f32, size: (f32, f32)) -> bool {
        let (w, h) = (size.0.ceil(), size.1.ceil());
        if self.adding_custom {
            match hit {
                Some(110) => {
                    if let Some(node) = parse_custom_node(&self.custom_input) {
                        set_active_vpn(Some(node.to_endpoint()));
                        {
                            let mut g = self.shared.lock().unwrap();
                            g.nodes.retain(|n| {
                                !(n.host == node.host && n.port == node.port && n.proto == node.proto)
                            });
                            g.nodes.insert(0, node.clone());
                            save_disk_config(Some(&node), &g.nodes);
                        }
                        self.adding_custom = false;
                        self.custom_input.clear();
                        self.verify_active_exit_ip();
                        self.refresh_and_test();
                    }
                    return false;
                }
                Some(111) => {
                    self.adding_custom = false;
                    return false;
                }
                _ => return false,
            }
        }

        match hit {
            Some(100) => {
                let visible = self.visible_nodes();
                let sel_node = self
                    .selected
                    .as_ref()
                    .and_then(|id| visible.iter().find(|n| n.id == *id))
                    .cloned();
                if let Some(node) = sel_node {
                    self.connect_or_disconnect_node(&node);
                } else if active_vpn().is_some() {
                    set_active_vpn(None);
                    let mut g = self.shared.lock().unwrap();
                    g.exit_info = None;
                    save_disk_config(None, &g.nodes);
                }
                false
            }
            Some(104) => {
                self.verify_active_exit_ip();
                false
            }
            Some(101) => {
                self.refresh_and_test();
                false
            }
            Some(102) => {
                self.adding_custom = true;
                self.focused_at = Instant::now();
                false
            }
            Some(103) => true,
            Some(105) => {
                self.country_filter.clear();
                self.extra_idx = 0;
                self.scroll.set(Scroll::default());
                false
            }
            Some(106) => {
                self.country_filter = "IN".into();
                self.extra_idx = 0;
                self.scroll.set(Scroll::default());
                false
            }
            Some(107) => {
                self.country_filter = "SG".into();
                self.extra_idx = 0;
                self.scroll.set(Scroll::default());
                false
            }
            Some(108) => {
                self.country_filter = "US".into();
                self.extra_idx = 0;
                self.scroll.set(Scroll::default());
                false
            }
            Some(109) => {
                self.country_filter = "AE".into();
                self.extra_idx = 0;
                self.scroll.set(Scroll::default());
                false
            }
            Some(112) => {
                self.extra_idx = (self.extra_idx + 1) % EXTRA_COUNTRIES.len();
                self.country_filter = EXTRA_COUNTRIES[self.extra_idx].0.to_owned();
                self.scroll.set(Scroll::default());
                false
            }
            _ => {
                let visible = self.visible_nodes();
                if let Some(row_idx) = self.row_at((x, y), (w, h), &visible) {
                    let node = visible[row_idx].clone();
                    let now = Instant::now();
                    let double = self.last_click.as_ref().is_some_and(|(at, prev)| {
                        *prev == node.id && now.duration_since(*at) < Duration::from_millis(350)
                    });
                    self.selected = Some(node.id.clone());
                    self.last_click = Some((now, node.id.clone()));
                    if double {
                        self.connect_or_disconnect_node(&node);
                    }
                }
                false
            }
        }
    }

    pub fn focused_text(&mut self) -> Option<(crate::clipboard::FieldId, &mut String, usize)> {
        self.adding_custom
            .then_some((VPN_CUSTOM_FIELD, &mut self.custom_input, 128))
    }

    pub fn focused_box(&self, (w, h): (f32, f32)) -> Option<[f32; 4]> {
        let _ = h;
        self.adding_custom
            .then(|| [(w.ceil() / 2.0).floor() - 100.0, 98.0, 200.0, 20.0])
    }
}

fn sort_nodes(nodes: &mut [VpnNode]) {
    let active = active_vpn();
    nodes.sort_by(|a, b| {
        let a_active = active.as_ref().is_some_and(|ep| a.matches_endpoint(ep));
        let b_active = active.as_ref().is_some_and(|ep| b.matches_endpoint(ep));
        b_active
            .cmp(&a_active)
            .then_with(|| b.custom.cmp(&a.custom))
            .then_with(|| (b.working == Some(true)).cmp(&(a.working == Some(true))))
            .then_with(|| {
                let a_socks = matches!(a.proto, VpnProto::Socks5);
                let b_socks = matches!(b.proto, VpnProto::Socks5);
                let a_ms = a.ping_ms.unwrap_or(u128::MAX);
                let b_ms = b.ping_ms.unwrap_or(u128::MAX);
                // Prefer SOCKS5 slightly over HTTP when pings are similar, sorted fastest first.
                let a_score = a_ms + if a_socks { 0 } else { 35 };
                let b_score = b_ms + if b_socks { 0 } else { 35 };
                a_score.cmp(&b_score)
            })
    });
}
