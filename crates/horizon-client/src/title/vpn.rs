use crate::font::HudAtlas;
use crate::hud::HudBatch;
use crate::ui::{Caret, Scroll, list_background, scrollbar, separators};
use horizon_protocol::connection::{VpnEndpoint, active_vpn, set_active_vpn, test_vpn_endpoint};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const HEADER: f32 = 44.0;
const FOOTER: f32 = 48.0;
const ROW_WIDTH: f32 = 305.0;
const ROW_HEIGHT: f32 = 24.0;
const VPN_CONFIG_FILE: &str = "horizon-vpn.json";
const VPN_CUSTOM_FIELD: crate::clipboard::FieldId = ("vpn.custom", 0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VpnProto {
    FastDns,
    Socks5,
    HttpConnect,
}

impl VpnProto {
    fn tag(self) -> &'static str {
        match self {
            Self::FastDns => "DNS",
            Self::Socks5 => "SOCKS5",
            Self::HttpConnect => "HTTP",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VpnNode {
    pub id: String,
    pub name: String,
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
            VpnProto::FastDns => VpnEndpoint::FastDns {
                label: self.name.clone(),
                dns_ip: self.host.clone(),
            },
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
        }
    }

    fn matches_endpoint(&self, ep: &VpnEndpoint) -> bool {
        match (self.proto, ep) {
            (VpnProto::FastDns, VpnEndpoint::FastDns { dns_ip, .. }) => self.host == *dns_ip,
            (VpnProto::Socks5, VpnEndpoint::Socks5 { host, port, .. }) => {
                self.host == *host && self.port == *port
            }
            (VpnProto::HttpConnect, VpnEndpoint::HttpConnect { host, port, .. }) => {
                self.host == *host && self.port == *port
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterTab {
    All,
    Tunnels,
    FastDns,
}

impl FilterTab {
    fn matches(self, node: &VpnNode) -> bool {
        match self {
            Self::All => true,
            Self::Tunnels => matches!(node.proto, VpnProto::Socks5 | VpnProto::HttpConnect),
            Self::FastDns => matches!(node.proto, VpnProto::FastDns),
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

fn builtin_nodes() -> Vec<VpnNode> {
    let dns = [
        ("dns-cf-1", "Cloudflare Fast Route", "Global Anycast", "1.1.1.1"),
        ("dns-cf-2", "Cloudflare Backup Route", "Global Anycast", "1.0.0.1"),
        ("dns-gg-1", "Google Global Route", "Global Anycast", "8.8.8.8"),
        ("dns-gg-2", "Google Secondary Route", "Global Anycast", "8.8.4.4"),
        ("dns-q9", "Quad9 Encrypted Route", "Global Anycast", "9.9.9.9"),
        ("dns-adg", "AdGuard Clean Route", "EU / Global", "94.140.14.14"),
        ("dns-cd", "ControlD Low-Latency", "Global Anycast", "76.76.2.0"),
        ("dns-od", "OpenDNS Cisco Route", "US / Global", "208.67.222.222"),
        ("dns-mv", "Mullvad Privacy DNS", "SE / Europe", "194.242.2.2"),
        ("dns-cb", "CleanBrowsing Fast", "US / EU", "185.228.168.9"),
        ("dns-nd", "NextDNS Anycast", "Global Anycast", "45.90.28.0"),
        ("dns-cf-sec", "Cloudflare Security DNS", "Global Anycast", "1.1.1.2"),
    ];
    let local_tunnels = [
        ("local-warp", "Cloudflare WARP Tunnel", "Local 127.0.0.1:40000", VpnProto::Socks5, "127.0.0.1", 40000),
        ("local-clash-s", "Clash / Mihomo SOCKS5", "Local 127.0.0.1:7890", VpnProto::Socks5, "127.0.0.1", 7890),
        ("local-clash-h", "Clash / Mihomo HTTP", "Local 127.0.0.1:7891", VpnProto::HttpConnect, "127.0.0.1", 7891),
        ("local-v2ray-s", "V2Ray / NekoBox SOCKS5", "Local 127.0.0.1:10808", VpnProto::Socks5, "127.0.0.1", 10808),
        ("local-v2ray-h", "V2Ray / NekoBox HTTP", "Local 127.0.0.1:10809", VpnProto::HttpConnect, "127.0.0.1", 10809),
        ("local-tor", "Tor / Orbot Tunnel", "Local 127.0.0.1:9050", VpnProto::Socks5, "127.0.0.1", 9050),
        ("local-ss", "Shadowsocks / Sing-box", "Local 127.0.0.1:1080", VpnProto::Socks5, "127.0.0.1", 1080),
    ];

    let mut out = Vec::with_capacity(dns.len() + local_tunnels.len());
    for (id, name, region, ip) in dns {
        out.push(VpnNode {
            id: id.into(),
            name: name.into(),
            region: region.into(),
            proto: VpnProto::FastDns,
            host: ip.into(),
            port: 53,
            custom: false,
            local: false,
            ping_ms: None,
            testing: true,
            working: None,
        });
    }
    for (id, name, region, proto, host, port) in local_tunnels {
        out.push(VpnNode {
            id: id.into(),
            name: name.into(),
            region: region.into(),
            proto,
            host: host.into(),
            port,
            custom: false,
            local: true,
            ping_ms: None,
            testing: true,
            working: None,
        });
    }
    out
}

pub fn parse_custom_node(raw: &str) -> Option<VpnNode> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let (proto, rest) = if let Some(r) = s.strip_prefix("socks5://").or_else(|| s.strip_prefix("socks://")) {
        (VpnProto::Socks5, r)
    } else if let Some(r) = s.strip_prefix("http://").or_else(|| s.strip_prefix("https://")) {
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

fn fetch_live_proxy_candidates() -> Vec<VpnNode> {
    let agent: ureq::Agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(4))
        .timeout(Duration::from_secs(6))
        .build();

    let mut raw_entries: Vec<(VpnProto, String, u16, String)> = Vec::new();

    // 1) GeoNode rich JSON feed (gives country codes + latency + socks5/http)
    if let Ok(resp) = agent
        .get("https://proxylist.geonode.com/api/proxy-list?limit=80&page=1&sort_by=lastChecked&sort_type=desc&protocols=socks5%2Chttp")
        .call()
        && let Ok(text) = resp.into_string()
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(arr) = json.get("data").and_then(|v| v.as_array())
    {
        for item in arr {
            let Some(ip) = item.get("ip").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(port) = item
                .get("port")
                .and_then(|v| v.as_str().and_then(|s| s.parse::<u16>().ok()).or_else(|| v.as_u64().and_then(|n| u16::try_from(n).ok())))
            else {
                continue;
            };
            let country = item
                .get("country")
                .and_then(|v| v.as_str())
                .unwrap_or("Global");
            let is_socks5 = item
                .get("protocols")
                .and_then(|v| v.as_array())
                .is_some_and(|ps| ps.iter().any(|p| p.as_str() == Some("socks5")));
            let proto = if is_socks5 {
                VpnProto::Socks5
            } else {
                VpnProto::HttpConnect
            };
            raw_entries.push((proto, ip.to_owned(), port, country.to_owned()));
        }
    }

    // 2) ProxyScrape fast live SOCKS5 + HTTP feeds + SpeedX mirror
    let feeds = [
        (
            VpnProto::Socks5,
            "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=socks5&timeout=3000&country=all&ssl=all&anonymity=all",
            "Global Tunnel",
        ),
        (
            VpnProto::HttpConnect,
            "https://api.proxyscrape.com/v2/?request=displayproxies&protocol=http&timeout=3000&country=all&ssl=all&anonymity=all",
            "Global HTTP",
        ),
        (
            VpnProto::Socks5,
            "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/socks5.txt",
            "Community Pool",
        ),
    ];

    for (proto, url, region) in feeds {
        if let Ok(resp) = agent.get(url).call()
            && let Ok(body) = resp.into_string()
        {
            for line in body.lines().take(60) {
                let line = line.trim();
                if let Some((h, p)) = line.rsplit_once(':')
                    && let Ok(port) = p.trim().parse::<u16>()
                    && !h.trim().is_empty()
                    && port > 0
                {
                    raw_entries.push((proto, h.trim().to_owned(), port, region.to_owned()));
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (proto, host, port, region) in raw_entries {
        if !seen.insert((proto.tag(), host.clone(), port)) {
            continue;
        }
        out.push(VpnNode {
            id: format!("live-{}-{host}:{port}", proto.tag().to_lowercase()),
            name: format!("{} · {region}", proto.tag()),
            region: format!("{host}:{port}"),
            proto,
            host,
            port,
            custom: false,
            local: false,
            ping_ms: None,
            testing: true,
            working: None,
        });
        if out.len() >= 120 {
            break;
        }
    }
    out
}

#[derive(Clone)]
struct SharedState {
    nodes: Vec<VpnNode>,
    status: String,
    scanning: bool,
    generation: u64,
}

#[derive(Clone)]
pub struct Screen {
    shared: Arc<Mutex<SharedState>>,
    pub filter: FilterTab,
    pub selected: Option<usize>,
    pub scroll: Scroll,
    pub adding_custom: bool,
    pub custom_input: String,
    pub focused_at: Instant,
    last_click: Option<(Instant, usize)>,
    drag_last_y: Option<f32>,
    drag_moved: f32,
}

impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VpnScreen")
            .field("filter", &self.filter)
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
        nodes.extend(builtin_nodes());
        if let Some(active) = &disk.active
            && !nodes.iter().any(|n| n.host == active.host && n.port == active.port && n.proto == active.proto)
        {
            nodes.insert(0, active.clone());
        }
        let shared = Arc::new(Mutex::new(SharedState {
            nodes,
            status: "Probing VPN routes & fetching fast nodes...".into(),
            scanning: false,
            generation: 0,
        }));
        let mut screen = Self {
            shared,
            filter: FilterTab::All,
            selected: None,
            scroll: Scroll::default(),
            adding_custom: false,
            custom_input: String::new(),
            focused_at: Instant::now(),
            last_click: None,
            drag_last_y: None,
            drag_moved: 0.0,
        };
        screen.refresh_and_test();
        screen
    }

    pub fn refresh_and_test(&mut self) {
        let mut guard = self.shared.lock().unwrap();
        guard.generation += 1;
        let generation = guard.generation;
        guard.scanning = true;
        guard.status = "Testing routes & fetching live fast VPN nodes...".into();
        for n in &mut guard.nodes {
            n.testing = true;
            n.working = None;
            n.ping_ms = None;
        }
        let base_nodes: Vec<VpnNode> = guard
            .nodes
            .iter()
            .filter(|n| n.custom || n.local || n.proto == VpnProto::FastDns)
            .cloned()
            .collect();
        drop(guard);

        let shared = Arc::clone(&self.shared);
        std::thread::Builder::new()
            .name("horizon-vpn-scan".into())
            .spawn(move || {
                // Phase 1: probe built-in DNS + local tunnels + custom nodes right away (<1.2s)
                let probe_batch = |batch: Vec<VpnNode>, timeout_ms: u64| -> Vec<VpnNode> {
                    let len = batch.len();
                    let results = Arc::new(Mutex::new(vec![None; len]));
                    let idx = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                    let threads = len.min(24).max(1);
                    std::thread::scope(|s| {
                        for _ in 0..threads {
                            let idx = Arc::clone(&idx);
                            let results = Arc::clone(&results);
                            let batch_ref = &batch;
                            s.spawn(move || {
                                loop {
                                    let i = idx.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    if i >= len {
                                        break;
                                    }
                                    let ep = batch_ref[i].to_endpoint();
                                    let res = test_vpn_endpoint(&ep, Duration::from_millis(timeout_ms));
                                    results.lock().unwrap()[i] = Some(res);
                                }
                            });
                        }
                    });
                    let mut out = batch;
                    let res_guard = results.lock().unwrap();
                    for (i, n) in out.iter_mut().enumerate() {
                        n.testing = false;
                        match &res_guard[i] {
                            Some(Ok(ms)) => {
                                n.working = Some(true);
                                n.ping_ms = Some(*ms);
                            }
                            _ => {
                                n.working = Some(false);
                                n.ping_ms = None;
                            }
                        }
                    }
                    out
                };

                let probed_base = probe_batch(base_nodes, 1100);
                {
                    let mut g = shared.lock().unwrap();
                    if g.generation != generation {
                        return;
                    }
                    g.nodes = probed_base.clone();
                    sort_nodes(&mut g.nodes);
                    let ok_count = g.nodes.iter().filter(|n| n.working == Some(true)).count();
                    g.status = format!("{ok_count} fast routes ready · scanning global tunnels...");
                }

                // Phase 2: fetch live SOCKS5 & HTTP tunnels and verify them concurrently
                let live_candidates = fetch_live_proxy_candidates();
                let probed_live = probe_batch(live_candidates, 1400);
                let mut g = shared.lock().unwrap();
                if g.generation != generation {
                    return;
                }
                let working_live: Vec<VpnNode> = probed_live
                    .into_iter()
                    .filter(|n| n.working == Some(true))
                    .collect();
                let live_count = working_live.len();
                g.nodes = probed_base;
                g.nodes.extend(working_live);
                sort_nodes(&mut g.nodes);
                let total_ok = g.nodes.iter().filter(|n| n.working == Some(true)).count();
                g.scanning = false;
                g.status = if live_count > 0 {
                    format!("{total_ok} working VPN routes ({live_count} verified tunnels)")
                } else {
                    format!("{total_ok} working fast routes ready")
                };
            })
            .ok();
    }

    fn visible_nodes(&self) -> Vec<(usize, VpnNode)> {
        let g = self.shared.lock().unwrap();
        g.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                if !self.filter.matches(n) {
                    return false;
                }
                // Keep custom, local tunnels, fast DNS routes, and all verified working live tunnels.
                n.custom || n.local || n.proto == VpnProto::FastDns || n.working == Some(true)
            })
            .map(|(i, n)| (i, n.clone()))
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
                b(cx - 100.0, 146.0, 200.0, "Add & Enable VPN", valid, 110),
                b(cx - 100.0, 170.0, 200.0, "Cancel", true, 111),
            ];
        }
        let active = active_vpn();
        let visible = self.visible_nodes();
        let sel_node = self
            .selected
            .and_then(|idx| visible.iter().find(|(i, _)| *i == idx).map(|(_, n)| n));
        let is_selected_active = sel_node.is_some_and(|n| active.as_ref().is_some_and(|a| n.matches_endpoint(a)));
        let toggle_label = if is_selected_active {
            "Disconnect"
        } else if sel_node.is_some() {
            "Connect"
        } else if active.is_some() {
            "Disable VPN"
        } else {
            "Connect"
        };
        let can_toggle = sel_node.is_some() || active.is_some();
        let footer_y = h - FOOTER + 12.0;

        vec![
            // Filter bar (top row under title)
            b(cx - 152.0, 21.0, 72.0, "All", self.filter != FilterTab::All, 105),
            b(cx - 76.0, 21.0, 74.0, "Tunnels", self.filter != FilterTab::Tunnels, 106),
            b(cx + 2.0, 21.0, 74.0, "Fast DNS", self.filter != FilterTab::FastDns, 107),
            b(cx + 80.0, 21.0, 72.0, "Refresh", true, 101),
            // Footer controls
            b(cx - 152.0, footer_y, 98.0, toggle_label, can_toggle, 100),
            b(cx - 50.0, footer_y, 98.0, "Add Custom", true, 102),
            b(cx + 52.0, footer_y, 100.0, "Done", true, 103),
        ]
    }

    pub fn draw(
        &mut self,
        atlas: &HudAtlas,
        batch: &mut HudBatch,
        caret: &mut Caret,
        started: Instant,
        cursor: (f32, f32),
    ) {
        let (w, h) = (batch.width.ceil(), batch.height.ceil());
        let cx = (w / 2.0).floor();

        if self.adding_custom {
            let title = "Add Custom VPN Tunnel";
            let tw = HudBatch::text_width(atlas, title);
            batch.text(atlas, title, cx - (tw.ceil() / 2.0).floor(), 24.0, u32::MAX, true);

            let label = "socks5://ip:port, http://ip:port, or dns://ip";
            batch.text(
                atlas,
                label,
                cx - 100.0,
                84.0,
                super::rgba(160, 160, 160, 255),
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
            let hint = "Examples: 127.0.0.1:40000 or http://1.2.3.4:8080";
            let hw = HudBatch::text_width(atlas, hint);
            batch.text(
                atlas,
                hint,
                cx - (hw.ceil() / 2.0).floor(),
                125.0,
                super::rgba(140, 140, 140, 255),
                true,
            );
            return;
        }

        let active = active_vpn();
        let header_text = match &active {
            Some(ep) => format!("VPN: ON ({})", ep.label()),
            None => "VPN Networks (Direct)".to_owned(),
        };
        let header_color = if active.is_some() {
            super::rgba(85, 255, 85, 255)
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
        self.scroll.clamp(top, bottom, content);

        list_background(batch, atlas, top, bottom, false);
        let first = batch.vertices.len();

        let millis = started.elapsed().as_millis();
        let hovered_row = self.row_at(cursor, (w, h), &visible);

        for (row_idx, (real_idx, node)) in visible.iter().enumerate() {
            let y = top + 2.0 - self.scroll.amount.trunc() + row_idx as f32 * ROW_HEIGHT;
            if y + ROW_HEIGHT < top || y > bottom {
                continue;
            }
            let is_selected = self.selected == Some(*real_idx);
            let is_active = active.as_ref().is_some_and(|a| node.matches_endpoint(a));
            let is_hovered = hovered_row == Some(row_idx);

            if is_selected {
                batch.rect(left, y, ROW_WIDTH, ROW_HEIGHT, u32::MAX);
                batch.rect(
                    left + 1.0,
                    y + 1.0,
                    ROW_WIDTH - 2.0,
                    ROW_HEIGHT - 2.0,
                    super::rgba(0, 0, 0, 255),
                );
            } else if is_active {
                batch.rect(
                    left + 1.0,
                    y + 1.0,
                    ROW_WIDTH - 2.0,
                    ROW_HEIGHT - 2.0,
                    super::rgba(22, 54, 24, 210),
                );
            } else if is_hovered {
                batch.rect(
                    left + 1.0,
                    y + 1.0,
                    ROW_WIDTH - 2.0,
                    ROW_HEIGHT - 2.0,
                    super::rgba(255, 255, 255, 26),
                );
            }

            let tag = format!("[{}]", node.proto.tag());
            let tag_color = match node.proto {
                VpnProto::FastDns => super::rgba(85, 255, 255, 255),
                VpnProto::Socks5 => super::rgba(85, 255, 85, 255),
                VpnProto::HttpConnect => super::rgba(255, 215, 85, 255),
            };
            batch.text(atlas, &tag, left + 5.0, y + 3.0, tag_color, false);
            let tag_w = HudBatch::text_width(atlas, &tag).ceil();
            let title_color = if is_active {
                super::rgba(85, 255, 85, 255)
            } else {
                u32::MAX
            };
            let title_text = if is_active {
                format!("{} (ON)", node.name)
            } else {
                node.name.clone()
            };
            batch.text(
                atlas,
                &title_text,
                left + 9.0 + tag_w,
                y + 3.0,
                title_color,
                false,
            );
            batch.text(
                atlas,
                &node.region,
                left + 5.0,
                y + 13.0,
                super::rgba(160, 160, 160, 255),
                false,
            );

            // Ping sprite + ms readout on the right side of the row
            let icon_x = left + ROW_WIDTH - 16.0;
            let (sprite, ping_label, ping_color) = if node.testing {
                let frame = ((millis / 150) % 5) + 1;
                (
                    format!("gui/sprites/icon/pinging_{frame}"),
                    "...".to_owned(),
                    super::rgba(170, 170, 170, 255),
                )
            } else if let Some(ms) = node.ping_ms {
                let bars = if ms < 60 {
                    5
                } else if ms < 130 {
                    4
                } else if ms < 240 {
                    3
                } else if ms < 450 {
                    2
                } else {
                    1
                };
                let color = if ms < 130 {
                    super::rgba(85, 255, 85, 255)
                } else if ms < 320 {
                    super::rgba(255, 255, 85, 255)
                } else {
                    super::rgba(255, 170, 85, 255)
                };
                (format!("gui/sprites/icon/ping_{bars}"), format!("{ms}ms"), color)
            } else {
                (
                    "gui/sprites/icon/unreachable".to_owned(),
                    if node.local { "Off".into() } else { "Timeout".into() },
                    super::rgba(150, 150, 150, 255),
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
            self.scroll.amount,
        );

        let status = self.shared.lock().unwrap().status.clone();
        let sw = HudBatch::text_width(atlas, &status);
        batch.text(
            atlas,
            &status,
            cx - (sw.ceil() / 2.0).floor(),
            bottom + 2.0,
            super::rgba(170, 170, 170, 255),
            true,
        );
    }

    fn row_at(
        &self,
        cursor: (f32, f32),
        (w, h): (f32, f32),
        visible: &[(usize, VpnNode)],
    ) -> Option<usize> {
        let (top, bottom, left) = Self::list_area(w, h);
        let (x, y) = cursor;
        if x < left || x >= left + ROW_WIDTH || y < top || y >= bottom {
            return None;
        }
        let rel = y - (top + 2.0 - self.scroll.amount.trunc());
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
        if self
            .scroll
            .press(cursor, left + ROW_WIDTH + 8.0, top, bottom, content)
        {
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
        if self.scroll.held() {
            self.scroll.drag(y, top, bottom, content);
            return;
        }
        if let Some(last_y) = self.drag_last_y {
            let dy = last_y - y;
            self.drag_moved += dy.abs();
            self.drag_last_y = Some(y);
            self.scroll.amount += dy;
            self.scroll.clamp(top, bottom, content);
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
        self.scroll
            .wheel(delta, ROW_HEIGHT * 1.5, top, bottom, content);
    }

    pub fn release(&mut self) {
        self.scroll.release();
        self.drag_last_y = None;
    }

    /// Handles a click on the VPN screen. Returns `true` to return to the Title screen.
    pub fn click(&mut self, hit: Option<u8>, x: f32, y: f32, size: (f32, f32)) -> bool {
        let (w, h) = (size.0.ceil(), size.1.ceil());
        if self.adding_custom {
            match hit {
                Some(110) => {
                    if let Some(node) = parse_custom_node(&self.custom_input) {
                        let ep = node.to_endpoint();
                        set_active_vpn(Some(ep));
                        {
                            let mut g = self.shared.lock().unwrap();
                            g.nodes.retain(|n| !(n.host == node.host && n.port == node.port && n.proto == node.proto));
                            g.nodes.insert(0, node.clone());
                            save_disk_config(Some(&node), &g.nodes);
                        }
                        self.adding_custom = false;
                        self.custom_input.clear();
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
                // Connect / Disconnect / Disable
                let visible = self.visible_nodes();
                let active = active_vpn();
                if let Some(sel_idx) = self.selected
                    && let Some((_, node)) = visible.iter().find(|(i, _)| *i == sel_idx)
                {
                    let already_active = active.as_ref().is_some_and(|a| node.matches_endpoint(a));
                    if already_active {
                        set_active_vpn(None);
                        let g = self.shared.lock().unwrap();
                        save_disk_config(None, &g.nodes);
                    } else {
                        set_active_vpn(Some(node.to_endpoint()));
                        let g = self.shared.lock().unwrap();
                        save_disk_config(Some(node), &g.nodes);
                    }
                } else if active.is_some() {
                    set_active_vpn(None);
                    let g = self.shared.lock().unwrap();
                    save_disk_config(None, &g.nodes);
                }
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
                self.filter = FilterTab::All;
                self.scroll.amount = 0.0;
                false
            }
            Some(106) => {
                self.filter = FilterTab::Tunnels;
                self.scroll.amount = 0.0;
                false
            }
            Some(107) => {
                self.filter = FilterTab::FastDns;
                self.scroll.amount = 0.0;
                false
            }
            _ => {
                let visible = self.visible_nodes();
                if let Some(row_idx) = self.row_at((x, y), (w, h), &visible) {
                    let (real_idx, node) = &visible[row_idx];
                    let now = Instant::now();
                    let double = self.last_click.is_some_and(|(at, prev)| {
                        prev == *real_idx && now.duration_since(at) < Duration::from_millis(350)
                    });
                    self.selected = Some(*real_idx);
                    self.last_click = Some((now, *real_idx));
                    if double {
                        let active = active_vpn();
                        let already_active = active.as_ref().is_some_and(|a| node.matches_endpoint(a));
                        if already_active {
                            set_active_vpn(None);
                            let g = self.shared.lock().unwrap();
                            save_disk_config(None, &g.nodes);
                        } else {
                            set_active_vpn(Some(node.to_endpoint()));
                            let g = self.shared.lock().unwrap();
                            save_disk_config(Some(node), &g.nodes);
                        }
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
                // Prefer actual encrypted proxy tunnels over plain FastDNS when both work cleanly,
                // sorted by lowest measured latency.
                let a_ms = a.ping_ms.unwrap_or(u128::MAX);
                let b_ms = b.ping_ms.unwrap_or(u128::MAX);
                a_ms.cmp(&b_ms)
            })
    });
}
