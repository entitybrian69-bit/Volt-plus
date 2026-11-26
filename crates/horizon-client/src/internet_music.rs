//! Internet music search, playlist persistence, and shared launcher/in-game playback bridge.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration_secs: u64,
    pub stream_url: String,
    pub icon_url: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicPlaylist {
    pub name: String,
    pub tracks: Vec<MusicTrack>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MusicLibrary {
    pub playlists: Vec<MusicPlaylist>,
    pub active_playlist: Option<String>,
    pub volume: f32,
    pub muted: bool,
    pub disable_vanilla_music: bool,
}

impl Default for MusicLibrary {
    fn default() -> Self {
        Self {
            playlists: vec![MusicPlaylist {
                name: "Favorites".to_owned(),
                tracks: Vec::new(),
            }],
            active_playlist: None,
            volume: 0.85,
            muted: false,
            disable_vanilla_music: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MusicStatusSnapshot {
    pub playing: bool,
    pub buffering: bool,
    pub muted: bool,
    pub volume: f32,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub queue_index: usize,
    pub queue_len: usize,
    pub active_playlist: String,
    pub current_title: String,
    pub current_artist: String,
    pub current_url: String,
    pub status: String,
}

impl MusicStatusSnapshot {
    pub fn has_track(&self) -> bool {
        self.queue_len > 0 && !self.current_title.is_empty()
    }
}

#[derive(Clone, Copy)]
pub struct MusicBackend {
    pub play_queue: fn(&[MusicTrack], usize, &str),
    pub toggle_pause: fn(),
    pub next: fn(),
    pub prev: fn(),
    pub seek_by_ms: fn(i64),
    pub seek_to_ms: fn(u64),
    pub set_volume: fn(f32),
    pub set_muted: fn(bool),
    pub stop: fn(),
    pub snapshot: fn() -> MusicStatusSnapshot,
}

struct SharedState {
    home: Option<PathBuf>,
    library: MusicLibrary,
    loaded: bool,
    backend: Option<MusicBackend>,
    cached_discord_poll: Option<std::time::Instant>,
    cached_discord_now_playing: Option<String>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            home: None,
            library: MusicLibrary::default(),
            loaded: false,
            backend: None,
            cached_discord_poll: None,
            cached_discord_now_playing: None,
        }
    }
}

static STATE: OnceLock<Mutex<SharedState>> = OnceLock::new();

fn state() -> &'static Mutex<SharedState> {
    STATE.get_or_init(|| Mutex::new(SharedState::default()))
}

fn library_path(home: &Path) -> PathBuf {
    home.join("music").join("library.json")
}

fn ensure_loaded_locked(st: &mut SharedState) {
    if st.loaded {
        return;
    }
    if let Some(home) = &st.home {
        let path = library_path(home);
        if let Ok(raw) = std::fs::read_to_string(&path)
            && let Ok(mut lib) = serde_json::from_str::<MusicLibrary>(&raw)
        {
            if lib.playlists.is_empty() {
                lib.playlists.push(MusicPlaylist {
                    name: "Favorites".to_owned(),
                    tracks: Vec::new(),
                });
            }
            lib.volume = lib.volume.clamp(0.0, 1.0);
            st.library = lib;
        }
        st.loaded = true;
    }
}

fn save_locked(st: &SharedState) {
    let Some(home) = &st.home else {
        return;
    };
    let path = library_path(home);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&st.library) {
        let tmp = path.with_extension("json.part");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

pub fn initialize(home: &Path) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    let home_changed = st.home.as_deref() != Some(home);
    if home_changed {
        st.home = Some(home.to_path_buf());
        st.loaded = false;
    }
    ensure_loaded_locked(&mut st);
    if let Some(backend) = st.backend {
        let vol = st.library.volume;
        let muted = st.library.muted;
        drop(st);
        (backend.set_volume)(vol);
        (backend.set_muted)(muted);
    }
}

pub fn install_backend(backend: MusicBackend) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.backend = Some(backend);
    let vol = st.library.volume;
    let muted = st.library.muted;
    drop(st);
    (backend.set_volume)(vol);
    (backend.set_muted)(muted);
}

pub fn playlists() -> Vec<MusicPlaylist> {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    st.library.playlists.clone()
}

pub fn active_playlist_name() -> Option<String> {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    st.library.active_playlist.clone()
}

pub fn create_playlist(name: &str) -> Result<String, String> {
    let clean = name.trim();
    if clean.is_empty() {
        return Err("Enter a playlist name".to_owned());
    }
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    if let Some(existing) = st
        .library
        .playlists
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(clean))
    {
        return Ok(existing.name.clone());
    }
    let owned = clean.to_owned();
    st.library.playlists.push(MusicPlaylist {
        name: owned.clone(),
        tracks: Vec::new(),
    });
    save_locked(&st);
    Ok(owned)
}

pub fn delete_playlist(name: &str) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    st.library
        .playlists
        .retain(|p| !p.name.eq_ignore_ascii_case(name));
    if st
        .library
        .active_playlist
        .as_deref()
        .is_some_and(|a| a.eq_ignore_ascii_case(name))
    {
        st.library.active_playlist = None;
    }
    if st.library.playlists.is_empty() {
        st.library.playlists.push(MusicPlaylist {
            name: "Favorites".to_owned(),
            tracks: Vec::new(),
        });
    }
    save_locked(&st);
}

pub fn add_track_to_playlist(playlist_name: &str, track: MusicTrack) -> Result<bool, String> {
    let clean = playlist_name.trim();
    if clean.is_empty() {
        return Err("Select or create a playlist".to_owned());
    }
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    let idx = if let Some(pos) = st
        .library
        .playlists
        .iter()
        .position(|p| p.name.eq_ignore_ascii_case(clean))
    {
        pos
    } else {
        st.library.playlists.push(MusicPlaylist {
            name: clean.to_owned(),
            tracks: Vec::new(),
        });
        st.library.playlists.len() - 1
    };
    let list = &mut st.library.playlists[idx];
    if list
        .tracks
        .iter()
        .any(|t| t.id == track.id || t.stream_url == track.stream_url)
    {
        return Ok(false);
    }
    list.tracks.push(track);
    save_locked(&st);
    Ok(true)
}

pub fn remove_track_from_playlist(playlist_name: &str, track_id: &str) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    if let Some(list) = st
        .library
        .playlists
        .iter_mut()
        .find(|p| p.name.eq_ignore_ascii_case(playlist_name))
    {
        list.tracks.retain(|t| t.id != track_id);
        save_locked(&st);
    }
}

pub fn play_queue(tracks: &[MusicTrack], start_index: usize, playlist_name: &str) {
    if tracks.is_empty() {
        return;
    }
    let (backend, vol, muted) = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        ensure_loaded_locked(&mut st);
        st.cached_discord_poll = None;
        st.library.active_playlist = if playlist_name.trim().is_empty() {
            None
        } else {
            Some(playlist_name.to_owned())
        };
        save_locked(&st);
        (st.backend, st.library.volume, st.library.muted)
    };
    if let Some(b) = backend {
        (b.set_volume)(vol);
        (b.set_muted)(muted);
        (b.play_queue)(tracks, start_index.min(tracks.len() - 1), playlist_name);
    }
}

pub fn play_playlist(playlist_name: &str, start_index: usize) -> bool {
    let tracks = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        ensure_loaded_locked(&mut st);
        let found = st
            .library
            .playlists
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(playlist_name))
            .map(|p| p.tracks.clone())
            .unwrap_or_default();
        if found.is_empty() {
            return false;
        }
        st.library.active_playlist = Some(playlist_name.to_owned());
        save_locked(&st);
        found
    };
    play_queue(&tracks, start_index, playlist_name);
    true
}

pub fn toggle_pause() {
    let backend = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.cached_discord_poll = None;
        st.backend
    };
    if let Some(b) = backend {
        (b.toggle_pause)();
    }
}

pub fn next_track() {
    let backend = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.cached_discord_poll = None;
        st.backend
    };
    if let Some(b) = backend {
        (b.next)();
    }
}

pub fn prev_track() {
    let backend = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.cached_discord_poll = None;
        st.backend
    };
    if let Some(b) = backend {
        (b.prev)();
    }
}

pub fn seek_by_ms(delta_ms: i64) {
    if let Some(b) = state().lock().unwrap_or_else(|e| e.into_inner()).backend {
        (b.seek_by_ms)(delta_ms);
    }
}

pub fn seek_to_ms(pos_ms: u64) {
    if let Some(b) = state().lock().unwrap_or_else(|e| e.into_inner()).backend {
        (b.seek_to_ms)(pos_ms);
    }
}

pub fn stop() {
    let backend = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.cached_discord_poll = None;
        st.backend
    };
    if let Some(b) = backend {
        (b.stop)();
    }
}

pub fn volume() -> f32 {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    st.library.volume
}

pub fn set_volume(vol: f32) {
    let clamped = vol.clamp(0.0, 1.0);
    let backend = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        ensure_loaded_locked(&mut st);
        st.library.volume = clamped;
        if clamped > 0.0 && st.library.muted {
            st.library.muted = false;
        }
        save_locked(&st);
        st.backend
    };
    if let Some(b) = backend {
        (b.set_volume)(clamped);
        if clamped > 0.0 {
            (b.set_muted)(false);
        }
    }
}

pub fn muted() -> bool {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    st.library.muted
}

pub fn toggle_mute() -> bool {
    let (next_muted, backend) = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        ensure_loaded_locked(&mut st);
        st.library.muted = !st.library.muted;
        save_locked(&st);
        (st.library.muted, st.backend)
    };
    if let Some(b) = backend {
        (b.set_muted)(next_muted);
    }
    next_muted
}

pub fn is_vanilla_music_disabled() -> bool {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    st.library.disable_vanilla_music
}

pub fn set_vanilla_music_disabled(disabled: bool) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    if st.library.disable_vanilla_music != disabled {
        st.library.disable_vanilla_music = disabled;
        save_locked(&st);
    }
}

pub fn toggle_vanilla_music_disabled() -> bool {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    let next = !st.library.disable_vanilla_music;
    st.library.disable_vanilla_music = next;
    save_locked(&st);
    next
}

pub fn snapshot() -> MusicStatusSnapshot {
    let (backend, vol, muted_cfg, active_pl) = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        ensure_loaded_locked(&mut st);
        (
            st.backend,
            st.library.volume,
            st.library.muted,
            st.library.active_playlist.clone().unwrap_or_default(),
        )
    };
    if let Some(b) = backend {
        let mut snap = (b.snapshot)();
        if snap.active_playlist.is_empty() && !active_pl.is_empty() {
            snap.active_playlist = active_pl;
        }
        snap
    } else {
        MusicStatusSnapshot {
            volume: vol,
            muted: muted_cfg,
            active_playlist: active_pl,
            ..Default::default()
        }
    }
}

/// Returns `"Title - Artist"` when online music is actively playing, for Discord Rich Presence.
/// Polls the native audio backend at most every 800 ms (invalidated immediately on transport calls).
pub fn now_playing_for_discord() -> Option<String> {
    {
        let st = state().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(last) = st.cached_discord_poll
            && last.elapsed() < Duration::from_millis(800)
        {
            return st.cached_discord_now_playing.clone();
        }
    }
    let snap = snapshot();
    let next = if !snap.playing || snap.current_title.trim().is_empty() {
        None
    } else {
        let title = snap.current_title.trim();
        let artist = snap.current_artist.trim();
        if artist.is_empty() || artist == "Unknown Artist" {
            Some(title.to_owned())
        } else {
            Some(format!("{title} - {artist}"))
        }
    };
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.cached_discord_poll = Some(std::time::Instant::now());
    st.cached_discord_now_playing = next.clone();
    next
}

pub fn format_duration(secs: u64) -> String {
    let mins = secs / 60;
    let rem = secs % 60;
    format!("{mins}:{rem:02}")
}

pub fn format_ms(ms: u64) -> String {
    format_duration(ms / 1000)
}

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .resolver(horizon_port_pack::net::PreferIpv4Resolver)
        .user_agent("VoltPlus/1.0.1 (https://github.com/entitybrian69-bit/volt-plus)")
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .build()
}

fn audius_hosts(agent: &ureq::Agent) -> Vec<String> {
    let mut hosts = vec!["https://api.audius.co".to_owned()];
    if let Ok(resp) = agent.get("https://api.audius.co").call()
        && let Ok(val) = resp.into_json::<serde_json::Value>()
        && let Some(arr) = val.get("data").and_then(|d| d.as_array())
    {
        for item in arr.iter().take(4) {
            if let Some(url) = item.as_str()
                && url.starts_with("https://")
            {
                let trimmed = url.trim_end_matches('/').to_owned();
                if !hosts.contains(&trimmed) {
                    hosts.push(trimmed);
                }
            }
        }
    }
    hosts
}

fn parse_audius_tracks(host: &str, json: &serde_json::Value) -> Vec<MusicTrack> {
    let Some(items) = json.get("data").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let Some(id) = item.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let Some(title) = item
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let artist = item
            .pointer("/user/name")
            .or_else(|| item.pointer("/user/handle"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Audius Artist")
            .to_owned();
        let duration_secs = item
            .get("duration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let icon_url = item
            .pointer("/artwork/150x150")
            .or_else(|| item.pointer("/artwork/480x480"))
            .or_else(|| item.pointer("/artwork/1000x1000"))
            .and_then(|v| v.as_str())
            .filter(|u| u.starts_with("https://"))
            .map(ToOwned::to_owned);
        let stream_url = format!("{host}/v1/tracks/{id}/stream?app_name=VoltPlus");
        out.push(MusicTrack {
            id: format!("audius:{id}"),
            title: title.to_owned(),
            artist,
            duration_secs,
            stream_url,
            icon_url,
            source: "Audius".to_owned(),
        });
    }
    out
}

fn search_audius(query: &str) -> Result<Vec<MusicTrack>, String> {
    let agent = http_agent();
    let trimmed = query.trim();
    let encoded_q = url::form_urlencoded::byte_serialize(trimmed.as_bytes()).collect::<String>();
    let mut last_err = String::new();
    for host in audius_hosts(&agent) {
        let endpoint = if trimmed.is_empty() {
            format!("{host}/v1/tracks/trending?app_name=VoltPlus&limit=30")
        } else {
            format!("{host}/v1/tracks/search?query={encoded_q}&app_name=VoltPlus&limit=30")
        };
        match agent.get(&endpoint).call() {
            Ok(resp) => {
                if let Ok(json) = resp.into_json::<serde_json::Value>() {
                    let tracks = parse_audius_tracks(&host, &json);
                    if !tracks.is_empty() || !trimmed.is_empty() {
                        return Ok(tracks);
                    }
                }
            }
            Err(e) => {
                last_err = e.to_string();
            }
        }
    }
    Err(if last_err.is_empty() {
        "No music tracks found".to_owned()
    } else {
        last_err
    })
}

fn search_archive_org(query: &str) -> Result<Vec<MusicTrack>, String> {
    let agent = http_agent();
    let q_expr = if query.trim().is_empty() {
        "mediatype:audio AND format:\"VBR MP3\" AND (subject:music OR subject:lofi OR subject:electronic)".to_owned()
    } else {
        format!(
            "mediatype:audio AND format:\"VBR MP3\" AND ({})",
            query.trim()
        )
    };
    let encoded_q = url::form_urlencoded::byte_serialize(q_expr.as_bytes()).collect::<String>();
    let url = format!(
        "https://archive.org/advancedsearch.php?q={encoded_q}&fl[]=identifier&fl[]=title&fl[]=creator&rows=18&output=json"
    );
    let resp = agent
        .get(&url)
        .call()
        .map_err(|e| format!("Archive.org search failed: {e}"))?;
    let json = resp
        .into_json::<serde_json::Value>()
        .map_err(|e| e.to_string())?;
    let docs = json
        .pointer("/response/docs")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let mut tracks = Vec::new();
    for doc in docs.into_iter().take(12) {
        let Some(ident) = doc.get("identifier").and_then(|v| v.as_str()) else {
            continue;
        };
        let meta_url = format!("https://archive.org/metadata/{ident}/files");
        let Ok(meta_resp) = agent.get(&meta_url).call() else {
            continue;
        };
        let Ok(meta_json) = meta_resp.into_json::<serde_json::Value>() else {
            continue;
        };
        let Some(files) = meta_json.get("result").and_then(|r| r.as_array()) else {
            continue;
        };
        let Some(audio_file) = files.iter().find(|f| {
            f.get("name")
                .and_then(|n| n.as_str())
                .is_some_and(|name| {
                    let lower = name.to_ascii_lowercase();
                    lower.ends_with(".mp3") || lower.ends_with(".ogg") || lower.ends_with(".m4a")
                })
        }) else {
            continue;
        };
        let Some(file_name) = audio_file.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        let duration_secs = audio_file
            .get("length")
            .and_then(|l| match l {
                serde_json::Value::String(s) => s.split('.').next()?.parse::<u64>().ok(),
                serde_json::Value::Number(n) => n.as_f64().map(|v| v.max(0.0) as u64),
                _ => None,
            })
            .unwrap_or(0);
        let title = audio_file
            .get("title")
            .or_else(|| doc.get("title"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(ident)
            .to_owned();
        let artist = doc
            .get("creator")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Internet Archive")
            .to_owned();
        let encoded_file =
            url::form_urlencoded::byte_serialize(file_name.as_bytes()).collect::<String>();
        tracks.push(MusicTrack {
            id: format!("archive:{ident}:{file_name}"),
            title,
            artist,
            duration_secs,
            stream_url: format!("https://archive.org/download/{ident}/{encoded_file}"),
            icon_url: Some(format!("https://archive.org/services/img/{ident}")),
            source: "Internet Archive".to_owned(),
        });
    }
    Ok(tracks)
}

/// Searches online music tracks (Audius primary, Internet Archive fallback) or constructs
/// a direct stream entry for a pasted `https://` audio URL.
pub fn search_online_tracks(query: &str) -> Result<Vec<MusicTrack>, String> {
    let trimmed = query.trim();
    if trimmed.starts_with("https://") {
        let parsed = url::Url::parse(trimmed).map_err(|e| format!("Invalid HTTPS URL: {e}"))?;
        let raw_name = parsed
            .path_segments()
            .and_then(|mut segs| segs.next_back())
            .filter(|s| !s.is_empty())
            .unwrap_or("Custom Audio Stream");
        let title = raw_name
            .split('?')
            .next()
            .unwrap_or(raw_name)
            .replace(['_', '-'], " ");
        return Ok(vec![MusicTrack {
            id: format!("url:{trimmed}"),
            title,
            artist: parsed.host_str().unwrap_or("Custom Stream").to_owned(),
            duration_secs: 0,
            stream_url: trimmed.to_owned(),
            icon_url: None,
            source: "Direct URL".to_owned(),
        }]);
    }
    match search_audius(trimmed) {
        Ok(tracks) if !tracks.is_empty() => Ok(tracks),
        Ok(_) => search_archive_org(trimmed),
        Err(audius_err) => match search_archive_org(trimmed) {
            Ok(tracks) if !tracks.is_empty() => Ok(tracks),
            Ok(_) | Err(_) => Err(format!("Could not fetch online music: {audius_err}")),
        },
    }
}
