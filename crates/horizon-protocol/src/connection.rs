//! A blocking connection with vanilla's pipeline: optional AES/CFB8
//! encryption, VarInt21 framing and threshold compression. After login it
//! splits into a reader (for a receive thread) and a shareable writer. It runs
//! over TCP, or over an in-memory pipe between two threads of one process
//! (vanilla's `LocalChannel`, which an integrated server's player uses).
use crate::{Error, Result, buf::Reader, crypto::Cfb8, frame};
use std::{
    io::{Read, Write},
    net::{IpAddr, Shutdown, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket},
    sync::{
        Arc, Condvar, Mutex, RwLock,
        atomic::{AtomicI64, Ordering},
    },
    time::{Duration, Instant},
};

/// Active in-game VPN / tunnel route for Minecraft server connections and pings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VpnEndpoint {
    /// Low-latency direct connection with custom fast UDP DNS routing (bypasses ISP DNS throttling).
    FastDns { label: String, dns_ip: String },
    /// RFC 1928 SOCKS5 proxy tunnel (performs remote DNS resolution on the VPN exit node).
    Socks5 {
        label: String,
        host: String,
        port: u16,
    },
    /// HTTP CONNECT proxy tunnel.
    HttpConnect {
        label: String,
        host: String,
        port: u16,
    },
}

impl VpnEndpoint {
    pub fn label(&self) -> &str {
        match self {
            Self::FastDns { label, .. }
            | Self::Socks5 { label, .. }
            | Self::HttpConnect { label, .. } => label,
        }
    }
}

static ACTIVE_VPN: RwLock<Option<VpnEndpoint>> = RwLock::new(None);

pub fn set_active_vpn(endpoint: Option<VpnEndpoint>) {
    if let Ok(mut guard) = ACTIVE_VPN.write() {
        *guard = endpoint;
    }
}

pub fn active_vpn() -> Option<VpnEndpoint> {
    ACTIVE_VPN.read().ok().and_then(|g| g.clone())
}

fn is_local_host(host: &str) -> bool {
    let trimmed = host.trim().trim_matches(|c| c == '[' || c == ']');
    if trimmed.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        return is_local_ip(ip);
    }
    false
}

fn is_local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

fn connect_tcp_addrs(host: &str, addrs: Vec<SocketAddr>, timeout: Duration) -> Result<TcpStream> {
    let mut addrs = ipv4_first(addrs);
    if addrs.is_empty() {
        return Err(Error::new(format!("Unknown host {host}")));
    }
    addrs.truncate(8);
    let (send, recv) = std::sync::mpsc::channel();
    let mut errors: Vec<Option<std::io::Error>> = addrs.iter().map(|_| None).collect();
    let (mut started, mut pending) = (0, 0);
    loop {
        if let Some(&addr) = addrs.get(started) {
            let (send, index) = (send.clone(), started);
            let spawned = std::thread::Builder::new()
                .name("horizon-vpn-connect".into())
                .spawn(move || {
                    let _ = send.send((index, TcpStream::connect_timeout(&addr, timeout)));
                });
            match spawned {
                Ok(_) => pending += 1,
                Err(e) => errors[index] = Some(e),
            }
            started += 1;
        }
        if pending == 0 {
            if started < addrs.len() {
                continue;
            }
            break;
        }
        let reply = if started < addrs.len() {
            match recv.recv_timeout(ATTEMPT_DELAY) {
                Ok(reply) => reply,
                Err(_) => continue,
            }
        } else {
            recv.recv().expect("a sender is held here")
        };
        pending -= 1;
        match reply {
            (_, Ok(stream)) => return Ok(stream),
            (index, Err(e)) => errors[index] = Some(e),
        }
    }
    match errors.into_iter().flatten().next() {
        Some(e) => Err(Error::new(format!("Connection refused: {e}"))),
        None => Err(Error::new(format!("Unknown host {host}"))),
    }
}

fn connect_via_socks5(
    proxy_host: &str,
    proxy_port: u16,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<TcpStream> {
    let addrs: Vec<_> = (proxy_host, proxy_port)
        .to_socket_addrs()
        .map_err(|e| Error::new(format!("VPN proxy {proxy_host}: {e}")))?
        .collect();
    let mut stream = connect_tcp_addrs(proxy_host, addrs, timeout)?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let _ = stream.set_nodelay(true);
    // Greeting: SOCKS5, 1 method, NO AUTH (0x00)
    stream
        .write_all(&[0x05, 0x01, 0x00])
        .map_err(|e| Error::new(format!("SOCKS5 greeting failed: {e}")))?;
    let mut greet = [0u8; 2];
    stream
        .read_exact(&mut greet)
        .map_err(|e| Error::new(format!("SOCKS5 handshake failed: {e}")))?;
    if greet != [0x05, 0x00] {
        return Err(Error::new("SOCKS5 proxy rejected no-auth mode"));
    }
    let host_bytes = target_host.as_bytes();
    if host_bytes.len() > 255 {
        return Err(Error::new("Target hostname too long for SOCKS5"));
    }
    let mut req = Vec::with_capacity(7 + host_bytes.len());
    req.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
    req.extend_from_slice(host_bytes);
    req.extend_from_slice(&target_port.to_be_bytes());
    stream
        .write_all(&req)
        .map_err(|e| Error::new(format!("SOCKS5 connect send failed: {e}")))?;
    let mut head = [0u8; 4];
    stream
        .read_exact(&mut head)
        .map_err(|e| Error::new(format!("SOCKS5 connect reply failed: {e}")))?;
    if head[0] != 0x05 || head[1] != 0x00 {
        return Err(Error::new(format!("SOCKS5 tunnel refused (code {})", head[1])));
    }
    let tail_len = match head[3] {
        0x01 => 4 + 2,
        0x03 => {
            let mut len = [0u8; 1];
            stream
                .read_exact(&mut len)
                .map_err(|e| Error::new(format!("SOCKS5 reply addr failed: {e}")))?;
            usize::from(len[0]) + 2
        }
        0x04 => 16 + 2,
        _ => return Err(Error::new("SOCKS5 invalid reply address type")),
    };
    let mut discard = vec![0u8; tail_len];
    stream
        .read_exact(&mut discard)
        .map_err(|e| Error::new(format!("SOCKS5 reply tail failed: {e}")))?;
    Ok(stream)
}

fn connect_via_http(
    proxy_host: &str,
    proxy_port: u16,
    target_host: &str,
    target_port: u16,
    timeout: Duration,
) -> Result<TcpStream> {
    let addrs: Vec<_> = (proxy_host, proxy_port)
        .to_socket_addrs()
        .map_err(|e| Error::new(format!("VPN proxy {proxy_host}: {e}")))?
        .collect();
    let mut stream = connect_tcp_addrs(proxy_host, addrs, timeout)?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let _ = stream.set_nodelay(true);
    let req = format!(
        "CONNECT {target_host}:{target_port} HTTP/1.1\r\nHost: {target_host}:{target_port}\r\nProxy-Connection: keep-alive\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| Error::new(format!("HTTP tunnel write failed: {e}")))?;
    let mut buf = [0u8; 2048];
    let mut total = 0usize;
    let status_ok = loop {
        let n = stream
            .read(&mut buf[total..])
            .map_err(|e| Error::new(format!("HTTP tunnel reply failed: {e}")))?;
        if n == 0 {
            return Err(Error::new("HTTP proxy closed tunnel unexpectedly"));
        }
        total += n;
        if let Some(end) = buf[..total].windows(4).position(|w| w == b"\r\n\r\n") {
            let header = String::from_utf8_lossy(&buf[..end]);
            let first_line = header.lines().next().unwrap_or("");
            break first_line.contains(" 200");
        }
        if total >= buf.len() {
            return Err(Error::new("HTTP proxy header too large"));
        }
    };
    if !status_ok {
        return Err(Error::new("HTTP proxy rejected CONNECT tunnel"));
    }
    Ok(stream)
}

/// Minimal single-packet UDP DNS A-record lookup (fast direct resolver without ISP DNS throttling).
fn resolve_a_via_udp_dns(dns_ip: &str, host: &str, port: u16, timeout: Duration) -> Option<Vec<SocketAddr>> {
    if let Ok(ip) = host.trim_matches(|c| c == '[' || c == ']').parse::<IpAddr>() {
        return Some(vec![SocketAddr::new(ip, port)]);
    }
    let dns_addr: SocketAddr = format!("{dns_ip}:53").parse().ok()?;
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.set_read_timeout(Some(timeout)).ok()?;
    let mut pkt = Vec::with_capacity(64);
    let txid: u16 = 0x564E;
    pkt.extend_from_slice(&txid.to_be_bytes());
    pkt.extend_from_slice(&[0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    for label in host.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 {
            return None;
        }
        pkt.push(label.len() as u8);
        pkt.extend_from_slice(label.as_bytes());
    }
    pkt.push(0);
    pkt.extend_from_slice(&1u16.to_be_bytes()); // QTYPE A
    pkt.extend_from_slice(&1u16.to_be_bytes()); // QCLASS IN
    sock.send_to(&pkt, dns_addr).ok()?;
    let mut resp = [0u8; 512];
    let n = sock.recv(&mut resp).ok()?;
    if n < 12 || u16::from_be_bytes([resp[0], resp[1]]) != txid {
        return None;
    }
    let qdcount = u16::from_be_bytes([resp[4], resp[5]]) as usize;
    let ancount = u16::from_be_bytes([resp[6], resp[7]]) as usize;
    let mut pos = 12usize;
    let skip_name = |buf: &[u8], mut p: usize| -> Option<usize> {
        loop {
            let len = *buf.get(p)?;
            if len == 0 {
                return Some(p + 1);
            }
            if len & 0xC0 == 0xC0 {
                return buf.get(p + 1).map(|_| p + 2);
            }
            p += 1 + usize::from(len);
        }
    };
    for _ in 0..qdcount {
        pos = skip_name(&resp[..n], pos)?;
        pos += 4;
    }
    let mut out = Vec::new();
    for _ in 0..ancount {
        pos = skip_name(&resp[..n], pos)?;
        if pos + 10 > n {
            break;
        }
        let rtype = u16::from_be_bytes([resp[pos], resp[pos + 1]]);
        let rdlen = u16::from_be_bytes([resp[pos + 8], resp[pos + 9]]) as usize;
        pos += 10;
        if pos + rdlen > n {
            break;
        }
        if rtype == 1 && rdlen == 4 {
            let ip = std::net::Ipv4Addr::new(resp[pos], resp[pos + 1], resp[pos + 2], resp[pos + 3]);
            out.push(SocketAddr::new(IpAddr::V4(ip), port));
        }
        pos += rdlen;
    }
    (!out.is_empty()).then_some(out)
}

/// Real end-to-end tunnel probe for a VPN endpoint; returns measured RTT in ms only if
/// the proxy actually opens an outbound tunnel to `1.1.1.1:443`.
pub fn test_vpn_endpoint(
    endpoint: &VpnEndpoint,
    timeout: Duration,
) -> std::result::Result<u128, String> {
    let start = Instant::now();
    match endpoint {
        VpnEndpoint::FastDns { dns_ip, .. } => {
            if resolve_a_via_udp_dns(dns_ip, "minecraft.net", 25565, timeout).is_some() {
                Ok(start.elapsed().as_millis().max(1))
            } else {
                Err("DNS probe timed out".into())
            }
        }
        VpnEndpoint::Socks5 { host, port, .. } => {
            let stream = connect_via_socks5(host, *port, "1.1.1.1", 443, timeout)
                .map_err(|e| e.to_string())?;
            let _ = stream.shutdown(Shutdown::Both);
            Ok(start.elapsed().as_millis().max(1))
        }
        VpnEndpoint::HttpConnect { host, port, .. } => {
            let stream = connect_via_http(host, *port, "1.1.1.1", 443, timeout)
                .map_err(|e| e.to_string())?;
            let _ = stream.shutdown(Shutdown::Both);
            Ok(start.elapsed().as_millis().max(1))
        }
    }
}

/// Connects through `endpoint` to `api.ipify.org:80` and returns the live external exit IP
/// plus end-to-end RTT in milliseconds.
pub fn probe_vpn_exit_ip(
    endpoint: &VpnEndpoint,
    timeout: Duration,
) -> std::result::Result<(String, u128), String> {
    let start = Instant::now();
    let mut stream = match endpoint {
        VpnEndpoint::Socks5 { host, port, .. } => {
            connect_via_socks5(host, *port, "api.ipify.org", 80, timeout)
                .map_err(|e| e.to_string())?
        }
        VpnEndpoint::HttpConnect { host, port, .. } => {
            connect_via_http(host, *port, "api.ipify.org", 80, timeout)
                .map_err(|e| e.to_string())?
        }
        VpnEndpoint::FastDns { dns_ip, .. } => {
            let addrs = resolve_a_via_udp_dns(dns_ip, "api.ipify.org", 80, timeout)
                .ok_or_else(|| "DNS resolution failed".to_owned())?;
            connect_tcp_addrs("api.ipify.org", addrs, timeout).map_err(|e| e.to_string())?
        }
    };
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: api.ipify.org\r\nConnection: close\r\n\r\n")
        .map_err(|e| e.to_string())?;
    let mut resp = Vec::with_capacity(512);
    let _ = stream.read_to_end(&mut resp);
    let text = String::from_utf8_lossy(&resp);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.trim())
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim();
    if body.parse::<IpAddr>().is_ok() {
        Ok((body.to_owned(), start.elapsed().as_millis().max(1)))
    } else {
        Err("Tunnel connected (exit IP check blocked)".into())
    }
}

/// One direction of a [`Pipe`]: the bytes written and not read yet, and
/// whether either end closed it.
#[derive(Default)]
struct Channel {
    state: Mutex<(std::collections::VecDeque<u8>, bool)>,
    ready: Condvar,
}
impl Channel {
    fn close(&self) {
        self.state.lock().unwrap().1 = true;
        self.ready.notify_all();
    }
}
/// The channels of one end; dropping the end's last clone closes them, as
/// a socket closes with its process.
struct End {
    incoming: Arc<Channel>,
    outgoing: Arc<Channel>,
}
impl Drop for End {
    fn drop(&mut self) {
        self.incoming.close();
        self.outgoing.close();
    }
}
/// One end of an in-memory byte pipe: what one end writes the other reads.
pub struct Pipe {
    end: Arc<End>,
    /// The read timeout (`set_read_timeout`).
    timeout: Mutex<Option<Duration>>,
}
impl Pipe {
    /// Two connected ends.
    pub fn pair() -> (Self, Self) {
        let (a, b) = (Arc::new(Channel::default()), Arc::new(Channel::default()));
        let end = |incoming, outgoing| Self {
            end: Arc::new(End { incoming, outgoing }),
            timeout: Mutex::new(None),
        };
        (end(a.clone(), b.clone()), end(b, a))
    }
    fn try_clone(&self) -> Self {
        Self {
            end: self.end.clone(),
            timeout: Mutex::new(*self.timeout.lock().unwrap()),
        }
    }
    /// Close both directions: reads at either end then see the end of the stream.
    pub fn shutdown(&self) {
        self.end.incoming.close();
        self.end.outgoing.close();
    }
}
impl Read for Pipe {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let deadline = self.timeout.lock().unwrap().map(|t| Instant::now() + t);
        let mut state = self.end.incoming.state.lock().unwrap();
        while state.0.is_empty() && !state.1 {
            state = match deadline {
                None => self.end.incoming.ready.wait(state).unwrap(),
                Some(deadline) => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Err(std::io::ErrorKind::TimedOut.into());
                    }
                    self.end.incoming.ready.wait_timeout(state, left).unwrap().0
                }
            };
        }
        let n = out.len().min(state.0.len());
        for (o, b) in out.iter_mut().zip(state.0.drain(..n)) {
            *o = b;
        }
        Ok(n)
    }
}
impl Write for Pipe {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut state = self.end.outgoing.state.lock().unwrap();
        if state.1 {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        state.0.extend(data);
        self.end.outgoing.ready.notify_all();
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// What a connection runs over.
enum Stream {
    Tcp(TcpStream),
    Memory(Pipe),
}
impl Stream {
    fn try_clone(&self) -> std::io::Result<Self> {
        Ok(match self {
            Self::Tcp(s) => Self::Tcp(s.try_clone()?),
            Self::Memory(p) => Self::Memory(p.try_clone()),
        })
    }
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            Self::Tcp(s) => s.set_read_timeout(timeout),
            Self::Memory(p) => {
                *p.timeout.lock().unwrap() = timeout;
                Ok(())
            }
        }
    }
    fn shutdown(&self) {
        match self {
            Self::Tcp(s) => {
                let _ = s.shutdown(Shutdown::Both);
            }
            Self::Memory(p) => p.shutdown(),
        }
    }
}
impl Read for Stream {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(s) => s.read(out),
            Self::Memory(p) => p.read(out),
        }
    }
}
impl Write for Stream {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(s) => s.write(data),
            Self::Memory(p) => p.write(data),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(s) => s.flush(),
            Self::Memory(p) => p.flush(),
        }
    }
}

/// Packets received/sent and bytes in/out, for the debug screen.
#[derive(Debug, Default)]
pub struct Counters {
    pub received: AtomicI64,
    pub sent: AtomicI64,
    pub bytes_in: AtomicI64,
    pub bytes_out: AtomicI64,
}

pub struct ConnReader {
    stream: Stream,
    buffer: Vec<u8>,
    decrypt: Option<Cfb8>,
    pub threshold: Option<usize>,
    pub counters: Arc<Counters>,
}
/// Closes a connection from another thread: a read or a write blocked on it fails.
pub struct Closer(Stream);
impl Closer {
    pub fn close(&self) {
        self.0.shutdown();
    }
}

pub struct ConnWriter {
    stream: Stream,
    encrypt: Option<Cfb8>,
    pub threshold: Option<usize>,
    counters: Arc<Counters>,
}
/// `InetAddress.getByName` under Java's default `java.net.preferIPv6Addresses=false`: IPv4
/// addresses first, whatever order the system resolver gives (macOS lists IPv6 first). Vanilla
/// connects to that first address only; the rest stay here as fallbacks.
fn ipv4_first(mut addrs: Vec<SocketAddr>) -> Vec<SocketAddr> {
    addrs.sort_by_key(SocketAddr::is_ipv6);
    addrs
}
/// RFC 8305's connection attempt delay: how long an address has alone before the next
/// one is tried beside it.
const ATTEMPT_DELAY: Duration = Duration::from_millis(250);
/// Both halves before the split.
pub struct Connection {
    pub reader: ConnReader,
    pub writer: ConnWriter,
}
impl Connection {
    pub fn connect(host: &str, port: u16, timeout: Duration) -> Result<Self> {
        if !is_local_host(host)
            && let Some(vpn) = active_vpn()
        {
            match vpn {
                VpnEndpoint::Socks5 {
                    host: ph,
                    port: pp,
                    ..
                } => {
                    return connect_via_socks5(&ph, pp, host, port, timeout)
                        .and_then(Self::from_stream);
                }
                VpnEndpoint::HttpConnect {
                    host: ph,
                    port: pp,
                    ..
                } => {
                    return connect_via_http(&ph, pp, host, port, timeout)
                        .and_then(Self::from_stream);
                }
                VpnEndpoint::FastDns { dns_ip, .. } => {
                    if let Some(addrs) = resolve_a_via_udp_dns(&dns_ip, host, port, Duration::from_secs(2)) {
                        return Self::connect_addresses(host, addrs, timeout);
                    }
                }
            }
        }
        let addrs: Vec<_> = (host, port)
            .to_socket_addrs()
            .map_err(|e| Error::new(format!("Unknown host {host}: {e}")))?
            .collect();
        Self::connect_addresses(host, addrs, timeout)
    }
    /// Connect through Minecraft's SRV redirect. Status requests still use the
    /// original address in their handshake (`ServerStatusPinger.pingServer`).
    pub fn connect_server(host: &str, port: u16, timeout: Duration) -> Result<Self> {
        Self::connect_server_with_address(host, port, timeout).map(|(connection, _, _)| connection)
    }
    /// `ConnectScreen.startConnecting`: login handshakes use the resolved host
    /// and port, including an SRV redirect, rather than the status request's address.
    pub fn connect_server_with_address(
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<(Self, String, u16)> {
        if !is_local_host(host)
            && let Some(vpn) = active_vpn()
        {
            match vpn {
                VpnEndpoint::Socks5 {
                    host: ph,
                    port: pp,
                    ..
                } => {
                    let resolved = crate::resolver::resolve(host, port).ok();
                    let (target_host, target_port) = resolved
                        .as_ref()
                        .map(|r| (r.host.clone(), r.port))
                        .unwrap_or_else(|| (host.to_owned(), port));
                    if !is_local_host(&target_host) {
                        let stream = connect_via_socks5(
                            &ph,
                            pp,
                            &target_host,
                            target_port,
                            timeout,
                        )?;
                        return Ok((Self::from_stream(stream)?, target_host, target_port));
                    }
                }
                VpnEndpoint::HttpConnect {
                    host: ph,
                    port: pp,
                    ..
                } => {
                    let resolved = crate::resolver::resolve(host, port).ok();
                    let (target_host, target_port) = resolved
                        .as_ref()
                        .map(|r| (r.host.clone(), r.port))
                        .unwrap_or_else(|| (host.to_owned(), port));
                    if !is_local_host(&target_host) {
                        let stream = connect_via_http(
                            &ph,
                            pp,
                            &target_host,
                            target_port,
                            timeout,
                        )?;
                        return Ok((Self::from_stream(stream)?, target_host, target_port));
                    }
                }
                VpnEndpoint::FastDns { dns_ip, .. } => {
                    if let Ok(resolved) = crate::resolver::resolve(host, port) {
                        if !resolved.addresses.iter().any(|a| is_local_ip(a.ip()))
                            && let Some(fast_addrs) = resolve_a_via_udp_dns(
                                &dns_ip,
                                &resolved.host,
                                resolved.port,
                                Duration::from_secs(2),
                            )
                        {
                            let connection = Self::connect_addresses(host, fast_addrs, timeout)?;
                            return Ok((connection, resolved.host, resolved.port));
                        }
                        let connection = Self::connect_addresses(host, resolved.addresses, timeout)?;
                        return Ok((connection, resolved.host, resolved.port));
                    } else if let Some(fast_addrs) =
                        resolve_a_via_udp_dns(&dns_ip, host, port, Duration::from_secs(2))
                    {
                        let connection = Self::connect_addresses(host, fast_addrs, timeout)?;
                        return Ok((connection, host.to_owned(), port));
                    }
                }
            }
        }
        let resolved = crate::resolver::resolve(host, port)?;
        let connection = Self::connect_addresses(host, resolved.addresses, timeout)?;
        Ok((connection, resolved.host, resolved.port))
    }
    /// Each address starts once the earlier ones fail or [`ATTEMPT_DELAY`] after the last
    /// start, alongside those still trying, and the first to connect wins: a dead address
    /// in a round-robin set costs 250 ms instead of the whole timeout.
    fn connect_addresses(host: &str, addrs: Vec<SocketAddr>, timeout: Duration) -> Result<Self> {
        let addrs = ipv4_first(addrs);
        let (send, recv) = std::sync::mpsc::channel();
        let mut errors: Vec<Option<std::io::Error>> = addrs.iter().map(|_| None).collect();
        let (mut started, mut pending) = (0, 0);
        loop {
            if let Some(&addr) = addrs.get(started) {
                let (send, index) = (send.clone(), started);
                let spawned = std::thread::Builder::new()
                    .name("horizon-connect".into())
                    .spawn(move || {
                        let _ = send.send((index, TcpStream::connect_timeout(&addr, timeout)));
                    });
                match spawned {
                    Ok(_) => pending += 1,
                    Err(e) => errors[index] = Some(e),
                }
                started += 1;
            }
            if pending == 0 {
                if started < addrs.len() {
                    continue;
                }
                break;
            }
            let reply = if started < addrs.len() {
                match recv.recv_timeout(ATTEMPT_DELAY) {
                    Ok(reply) => reply,
                    Err(_) => continue,
                }
            } else {
                recv.recv().expect("a sender is held here")
            };
            pending -= 1;
            match reply {
                (_, Ok(stream)) => return Self::from_stream(stream),
                (index, Err(e)) => errors[index] = Some(e),
            }
        }
        // Report the first address's failure: it is the one vanilla tries, and the IPv6
        // fallbacks after it often fail only because the client has no IPv6 route.
        match errors.into_iter().flatten().next() {
            Some(e) => Err(Error::new(format!("Connection refused: {e}"))),
            None => Err(Error::new(format!("Unknown host {host}"))),
        }
    }
    pub fn from_stream(stream: TcpStream) -> Result<Self> {
        stream.set_nodelay(true)?;
        Self::over(Stream::Tcp(stream))
    }
    /// A connection over one end of an in-memory [`Pipe`].
    pub fn local(pipe: Pipe) -> Self {
        Self::over(Stream::Memory(pipe)).expect("a pipe end clones")
    }
    fn over(stream: Stream) -> Result<Self> {
        let counters = Arc::new(Counters::default());
        let write = stream.try_clone()?;
        Ok(Self {
            reader: ConnReader {
                stream,
                buffer: Vec::new(),
                decrypt: None,
                threshold: None,
                counters: counters.clone(),
            },
            writer: ConnWriter {
                stream: write,
                encrypt: None,
                threshold: None,
                counters,
            },
        })
    }
    /// `Connection.isMemoryConnection`: through a [`Pipe`] rather than a socket.
    pub fn is_memory(&self) -> bool {
        matches!(self.reader.stream, Stream::Memory(_))
    }
    /// ClientboundLoginCompressionPacket: negative thresholds disable compression.
    pub fn set_compression(&mut self, threshold: i32) {
        let t = usize::try_from(threshold).ok();
        self.reader.threshold = t;
        self.writer.threshold = t;
    }
    /// Connection.setEncryptionKey: both directions keyed with the shared secret.
    pub fn enable_encryption(&mut self, secret: &[u8; 16]) {
        let mut decrypt = Cfb8::new(secret, secret);
        // Bytes already read past the last packet were sent encrypted.
        decrypt.decrypt(&mut self.reader.buffer);
        self.reader.decrypt = Some(decrypt);
        self.writer.encrypt = Some(Cfb8::new(secret, secret));
    }
    pub fn send(&mut self, body: &[u8]) -> Result<()> {
        self.writer.send(body)
    }
    pub fn recv(&mut self) -> Result<(i32, Vec<u8>)> {
        self.reader.recv()
    }
    pub fn split(self) -> (ConnReader, ConnWriter) {
        (self.reader, self.writer)
    }
}
impl ConnReader {
    /// Read exactly `len` bytes as they are, with no framing.
    pub fn read_raw(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut out = vec![0u8; len];
        self.stream.read_exact(&mut out)?;
        Ok(out)
    }
    /// Block until a whole packet arrives: (id, payload).
    pub fn recv(&mut self) -> Result<(i32, Vec<u8>)> {
        loop {
            if let Some(frame) = frame::split(&mut self.buffer)? {
                let body = frame::decompress(frame, self.threshold)?;
                if body.is_empty() {
                    // A frame with no packet in it. Vanilla's PacketDecoder
                    // only decodes a packet from a readable buffer, so an
                    // empty compressed frame is skipped and the connection
                    // stays up.
                    continue;
                }
                let mut r = Reader::new(&body);
                let id = r.varint()?;
                let start = r.position();
                self.counters.received.fetch_add(1, Ordering::Relaxed);
                return Ok((id, body[start..].to_vec()));
            }
            let mut chunk = [0u8; 16384];
            let n = match self.stream.read(&mut chunk) {
                Ok(0) => return Err(Error::end_of_stream()),
                Ok(n) => n,
                // The read timeout passed: Windows reports TimedOut, Unix
                // WouldBlock.
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err(Error::timed_out());
                }
                Err(e) => return Err(e.into()),
            };
            self.counters
                .bytes_in
                .fetch_add(n as i64, Ordering::Relaxed);
            let data = &mut chunk[..n];
            if let Some(c) = &mut self.decrypt {
                c.decrypt(data);
            }
            self.buffer.extend_from_slice(data);
        }
    }
    /// A [`Closer`] for this connection.
    pub fn closer(&self) -> Result<Closer> {
        Ok(Closer(self.stream.try_clone()?))
    }
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<()> {
        Ok(self.stream.set_read_timeout(timeout)?)
    }
}
impl ConnWriter {
    /// Write bytes as they are, with no framing: the legacy server list ping
    /// (`LegacyServerPinger`) is not a framed protocol.
    pub fn write_raw(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data)?;
        self.stream.flush()?;
        Ok(())
    }
    /// Send one packet body (VarInt id + payload).
    pub fn send(&mut self, body: &[u8]) -> Result<()> {
        self.send_contents(&frame::contents(body, self.threshold)?)
    }
    /// Send a frame whose contents [`frame::contents`] made with this
    /// writer's threshold.
    pub fn send_contents(&mut self, contents: &[u8]) -> Result<()> {
        let mut framed = frame::prefixed(contents);
        if let Some(c) = &mut self.encrypt {
            c.encrypt(&mut framed);
        }
        self.stream.write_all(&framed)?;
        self.counters.sent.fetch_add(1, Ordering::Relaxed);
        self.counters
            .bytes_out
            .fetch_add(framed.len() as i64, Ordering::Relaxed);
        Ok(())
    }
    pub fn shutdown(&self) {
        self.stream.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn ipv4_first_keeps_each_familys_order() {
        let a = |s: &str| s.parse::<SocketAddr>().unwrap();
        let resolved = vec![
            a("[2001:db8::1]:25565"),
            a("192.0.2.1:25565"),
            a("[2001:db8::2]:25565"),
            a("10.0.0.1:25565"),
        ];
        assert_eq!(
            ipv4_first(resolved),
            [
                a("192.0.2.1:25565"),
                a("10.0.0.1:25565"),
                a("[2001:db8::1]:25565"),
                a("[2001:db8::2]:25565"),
            ]
        );
    }

    fn peer(c: &Connection) -> SocketAddr {
        match &c.reader.stream {
            Stream::Tcp(s) => s.peer_addr().unwrap(),
            Stream::Memory(_) => panic!("not a socket"),
        }
    }

    /// Binds `[::1]`, or `None` where the machine has no IPv6 loopback.
    fn ipv6_loopback() -> Option<TcpListener> {
        let listener = TcpListener::bind("[::1]:0");
        if listener.is_err() {
            eprintln!("skipped: no IPv6 loopback");
        }
        listener.ok()
    }

    #[test]
    fn connects_to_ipv4_although_the_resolver_lists_ipv6_first() {
        // Both listen, as with a server whose IPv6 address accepts and then resets.
        let Some(v6_listener) = ipv6_loopback() else {
            return;
        };
        let v4_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let v6 = v6_listener.local_addr().unwrap();
        let v4 = v4_listener.local_addr().unwrap();
        let c =
            Connection::connect_addresses("test", vec![v6, v4], Duration::from_secs(5)).unwrap();
        assert_eq!(peer(&c), v4);
    }

    #[test]
    fn falls_back_to_ipv6_when_ipv4_refuses() {
        let Some(v6_listener) = ipv6_loopback() else {
            return;
        };
        let v6 = v6_listener.local_addr().unwrap();
        let closed = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let c = Connection::connect_addresses("test", vec![v6, closed], Duration::from_secs(5))
            .unwrap();
        assert_eq!(peer(&c), v6);
    }

    #[test]
    fn a_dead_address_does_not_hold_up_the_next() {
        // TEST-NET-1 never answers (or is unreachable at once).
        let dead: SocketAddr = "192.0.2.1:25565".parse().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let live = listener.local_addr().unwrap();
        let start = Instant::now();
        let c = Connection::connect_addresses("test", vec![dead, live], Duration::from_secs(10))
            .unwrap();
        assert_eq!(peer(&c), live);
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "{:?}",
            start.elapsed()
        );
    }

    #[test]
    fn every_failure_reports_the_first_address() {
        let closed = |_| {
            TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
        };
        let error = |host, addrs| {
            Connection::connect_addresses(host, addrs, Duration::from_secs(5))
                .err()
                .unwrap()
                .to_string()
        };
        let refused = error("test", (0..3).map(closed).collect());
        assert!(refused.starts_with("Connection refused"), "{refused}");
        assert_eq!(error("nowhere", Vec::new()), "Unknown host nowhere");
    }

    #[test]
    fn a_silent_peer_times_out_and_a_closed_one_ends_the_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut c = Connection::connect("127.0.0.1", port, Duration::from_secs(5)).unwrap();
        let (server, _) = listener.accept().unwrap();
        c.reader
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let silent = c.recv().unwrap_err();
        assert!(silent.is_timed_out(), "{silent}");
        assert_eq!(silent.to_string(), "Timed out");
        drop(server);
        let closed = c.recv().unwrap_err();
        assert!(closed.is_end_of_stream(), "{closed}");
    }

    #[test]
    fn loopback_with_compression_and_encryption() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut c = Connection::from_stream(stream).unwrap();
            let (id, payload) = c.recv().unwrap();
            assert_eq!((id, payload.as_slice()), (0, &b"plain"[..]));
            c.enable_encryption(&[7; 16]);
            c.set_compression(64);
            let (id, payload) = c.recv().unwrap();
            assert_eq!((id, payload.len()), (5, 1000));
            let mut body = vec![9];
            body.extend_from_slice(b"reply");
            c.send(&body).unwrap();
        });
        let mut c = Connection::connect("127.0.0.1", port, Duration::from_secs(5)).unwrap();
        let mut body = vec![0];
        body.extend_from_slice(b"plain");
        c.send(&body).unwrap();
        c.enable_encryption(&[7; 16]);
        c.set_compression(64);
        let mut big = vec![5];
        big.extend(std::iter::repeat_n(3u8, 1000));
        c.send(&big).unwrap();
        let (id, payload) = c.recv().unwrap();
        assert_eq!((id, payload.as_slice()), (9, &b"reply"[..]));
        server.join().unwrap();
    }

    #[test]
    fn local_pipe_carries_packets_and_closes() {
        let (a, b) = Pipe::pair();
        let server = std::thread::spawn(move || {
            let mut c = Connection::local(b);
            c.set_compression(64);
            let (id, payload) = c.recv().unwrap();
            assert_eq!((id, payload.len()), (5, 1000));
            c.send(&[9, 1, 2]).unwrap();
            let (_, writer) = c.split();
            writer.shutdown();
        });
        let mut c = Connection::local(a);
        c.set_compression(64);
        let mut big = vec![5];
        big.extend(std::iter::repeat_n(3u8, 1000));
        c.send(&big).unwrap();
        assert_eq!(c.recv().unwrap(), (9, vec![1, 2]));
        assert_eq!(c.reader.counters.sent.load(Ordering::Relaxed), 1);
        assert_eq!(c.reader.counters.received.load(Ordering::Relaxed), 1);
        server.join().unwrap();
        assert!(c.recv().is_err());
    }

    #[test]
    fn dropping_a_pipe_end_closes_it() {
        let (a, b) = Pipe::pair();
        let mut c = Connection::local(a);
        drop(b);
        assert!(c.recv().is_err());
        assert!(c.send(&[1]).is_err());
    }

    #[test]
    fn local_pipe_read_times_out() {
        let (a, _b) = Pipe::pair();
        let mut c = Connection::local(a);
        c.reader
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        assert!(c.recv().is_err());
    }

    /// Encrypted bytes that arrive in the same read as the last plain packet
    /// are decrypted once encryption is enabled.
    #[test]
    fn encrypted_bytes_read_before_enabling_encryption() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (sent, both_sent) = std::sync::mpsc::channel();
        let client = std::thread::spawn(move || {
            let mut c = Connection::connect("127.0.0.1", port, Duration::from_secs(5)).unwrap();
            c.send(&[0, 1, 2]).unwrap();
            c.enable_encryption(&[7; 16]);
            c.send(&[5, 4, 3]).unwrap();
            sent.send(c).unwrap();
        });
        let (stream, _) = listener.accept().unwrap();
        let mut c = Connection::from_stream(stream).unwrap();
        let _keep_open = both_sent.recv().unwrap();
        assert_eq!(c.recv().unwrap(), (0, vec![1, 2]));
        c.enable_encryption(&[7; 16]);
        assert_eq!(c.recv().unwrap(), (5, vec![4, 3]));
        client.join().unwrap();
    }

    /// A server may send a compressed frame that carries no packet: the
    /// uncompressed length is 0 and the frame holds nothing else. Vanilla
    /// skips it (its PacketDecoder only reads a packet from a readable
    /// buffer), so the client stays connected.
    #[test]
    fn an_empty_compressed_frame_is_skipped() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut c = Connection::from_stream(stream).unwrap();
            c.set_compression(64);
            // The empty frame: its length prefix, then the uncompressed
            // length 0 and nothing else.
            c.writer.send_contents(&[0]).unwrap();
            let mut body = vec![3];
            body.extend_from_slice(b"after");
            c.writer.send(&body).unwrap();
        });
        let mut c = Connection::connect("127.0.0.1", port, Duration::from_secs(5)).unwrap();
        c.set_compression(64);
        let (id, payload) = c.recv().unwrap();
        assert_eq!((id, payload.as_slice()), (3, &b"after"[..]));
        server.join().unwrap();
    }
}
