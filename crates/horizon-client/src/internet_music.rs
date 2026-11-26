//! Internet music search, offline library scanning, song downloads, playlist persistence,
//! and shared launcher/in-game playback bridge.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub const DOWNLOADED_PLAYLIST_NAME: &str = "Downloaded";
pub const EXPORTED_PLAYLIST_NAME: &str = "Exported Music's";
pub const PAGE_SIZE: usize = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MusicSort {
    #[default]
    Popular,
    Relevance,
    MostPlayed,
    Letters,
}

impl MusicSort {
    pub const ALL: [Self; 4] = [Self::Popular, Self::Relevance, Self::MostPlayed, Self::Letters];

    pub fn label(self) -> &'static str {
        match self {
            Self::Popular => "Popular",
            Self::Relevance => "Relevance",
            Self::MostPlayed => "Most Played",
            Self::Letters => "Letters",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration_secs: u64,
    pub stream_url: String,
    #[serde(default)]
    pub fallback_urls: Vec<String>,
    pub icon_url: Option<String>,
    pub source: String,
    #[serde(default)]
    pub play_count: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicPlaylist {
    pub name: String,
    pub tracks: Vec<MusicTrack>,
}

#[derive(Clone, Debug, Default)]
pub struct MusicSearchPage {
    pub tracks: Vec<MusicTrack>,
    pub has_next: bool,
}

#[derive(Clone, Debug, Default)]
pub struct DownloadProgress {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct InGameSearchSnapshot {
    pub query: String,
    pub genre: Option<String>,
    pub sort: MusicSort,
    pub page: usize,
    pub has_next: bool,
    pub searching: bool,
    pub fetched_initial: bool,
    pub results: Vec<MusicTrack>,
    pub status: String,
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

pub type IconCacheLoader = fn(&Path, &str) -> Option<(u32, u32, Vec<u8>)>;
pub type IconNetLoader = fn(&Path, &str, &str) -> Result<(u32, u32, Vec<u8>), String>;

struct SharedState {
    home: Option<PathBuf>,
    library: MusicLibrary,
    loaded: bool,
    backend: Option<MusicBackend>,
    icon_cache_loader: Option<IconCacheLoader>,
    icon_net_loader: Option<IconNetLoader>,
    icons: HashMap<String, (u32, u32, Vec<u8>)>,
    icon_in_flight: HashSet<String>,
    cached_discord_poll: Option<Instant>,
    cached_discord_now_playing: Option<String>,
    cached_offline_poll: Option<Instant>,
    cached_downloaded: Vec<MusicTrack>,
    cached_exported: Vec<MusicTrack>,
    downloads: Vec<DownloadProgress>,
    in_game_search: InGameSearchSnapshot,
    in_game_search_gen: u64,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            home: None,
            library: MusicLibrary::default(),
            loaded: false,
            backend: None,
            icon_cache_loader: None,
            icon_net_loader: None,
            icons: HashMap::new(),
            icon_in_flight: HashSet::new(),
            cached_discord_poll: None,
            cached_discord_now_playing: None,
            cached_offline_poll: None,
            cached_downloaded: Vec::new(),
            cached_exported: Vec::new(),
            downloads: Vec::new(),
            in_game_search: InGameSearchSnapshot::default(),
            in_game_search_gen: 0,
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

fn downloaded_meta_path(home: &Path) -> PathBuf {
    home.join("music").join("downloaded.json")
}

pub fn musics_dir(home: &Path) -> PathBuf {
    home.join("musics")
}

pub fn downloaded_dir(home: &Path) -> PathBuf {
    musics_dir(home).join("downloaded")
}

pub fn is_virtual_playlist(name: &str) -> bool {
    name.eq_ignore_ascii_case(DOWNLOADED_PLAYLIST_NAME)
        || name.eq_ignore_ascii_case(EXPORTED_PLAYLIST_NAME)
}

fn is_audio_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "mp3" | "wav" | "ogg" | "opus" | "flac" | "aac" | "m4a" | "aiff"
    )
}

fn nice_file_title(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Audio Track");
    let cleaned = stem.replace('_', " ").trim().to_owned();
    if cleaned.is_empty() {
        "Audio Track".to_owned()
    } else {
        cleaned
    }
}

fn collect_audio_files(dir: &Path, skip_dir_name: Option<&str>, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(skip) = skip_dir_name
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(skip))
            {
                continue;
            }
            collect_audio_files(&path, None, depth + 1, out);
        } else if path.is_file() && is_audio_file(&path) {
            out.push(path);
        }
    }
}

fn load_downloaded_meta(home: &Path) -> Vec<MusicTrack> {
    let path = downloaded_meta_path(home);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<MusicTrack>>(&raw).unwrap_or_default()
}

fn save_downloaded_meta(home: &Path, tracks: &[MusicTrack]) {
    let path = downloaded_meta_path(home);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(tracks) {
        let tmp = path.with_extension("json.part");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

fn rescan_offline_playlists(home: &Path) -> (Vec<MusicTrack>, Vec<MusicTrack>) {
    let root = musics_dir(home);
    let dl_dir = downloaded_dir(home);
    let _ = std::fs::create_dir_all(&root);
    let _ = std::fs::create_dir_all(&dl_dir);

    let mut meta = load_downloaded_meta(home);
    meta.retain(|t| Path::new(&t.stream_url).is_file());
    let mut known_paths: HashSet<PathBuf> = meta
        .iter()
        .map(|t| PathBuf::from(&t.stream_url))
        .collect();

    let mut dl_files = Vec::new();
    collect_audio_files(&dl_dir, None, 0, &mut dl_files);
    dl_files.sort();
    for p in dl_files {
        if known_paths.insert(p.clone()) {
            let path_str = p.to_string_lossy().into_owned();
            meta.push(MusicTrack {
                id: format!("local:dl:{path_str}"),
                title: nice_file_title(&p),
                artist: "Downloaded".to_owned(),
                duration_secs: 0,
                stream_url: path_str,
                icon_url: None,
                source: "Offline".to_owned(),
                play_count: 0,
                fallback_urls: Vec::new(),
            });
        }
    }

    let mut exp_files = Vec::new();
    collect_audio_files(&root, Some("downloaded"), 0, &mut exp_files);
    exp_files.sort();
    let exported = exp_files
        .into_iter()
        .map(|p| {
            let path_str = p.to_string_lossy().into_owned();
            MusicTrack {
                id: format!("local:exp:{path_str}"),
                title: nice_file_title(&p),
                artist: "Local Audio".to_owned(),
                duration_secs: 0,
                stream_url: path_str,
                icon_url: None,
                source: EXPORTED_PLAYLIST_NAME.to_owned(),
                play_count: 0,
                fallback_urls: Vec::new(),
            }
        })
        .collect();

    (meta, exported)
}

fn ensure_offline_fresh_locked(st: &mut SharedState) {
    let Some(home) = st.home.clone() else {
        return;
    };
    if let Some(last) = st.cached_offline_poll
        && last.elapsed() < Duration::from_secs(3)
    {
        return;
    }
    let (dl, exp) = rescan_offline_playlists(&home);
    st.cached_downloaded = dl;
    st.cached_exported = exp;
    st.cached_offline_poll = Some(Instant::now());
}

pub fn invalidate_offline_cache() {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.cached_offline_poll = None;
}

fn ensure_loaded_locked(st: &mut SharedState) {
    if st.loaded {
        return;
    }
    if let Some(home) = &st.home {
        let _ = std::fs::create_dir_all(musics_dir(home));
        let _ = std::fs::create_dir_all(downloaded_dir(home));
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
        st.cached_offline_poll = None;
    }
    ensure_loaded_locked(&mut st);
    ensure_offline_fresh_locked(&mut st);
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

pub fn install_icon_backend(cache_loader: IconCacheLoader, net_loader: IconNetLoader) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.icon_cache_loader = Some(cache_loader);
    st.icon_net_loader = Some(net_loader);
}

pub fn get_or_request_icon(track_id: &str, icon_url: Option<&str>) -> Option<(u32, u32, Vec<u8>)> {
    if track_id.is_empty() {
        return None;
    }
    let (home, cache_loader, net_loader) = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(img) = st.icons.get(track_id) {
            return Some(img.clone());
        }
        if st.icon_in_flight.contains(track_id) {
            return None;
        }
        (st.home.clone(), st.icon_cache_loader, st.icon_net_loader)
    };
    let home = home?;
    if let Some(loader) = cache_loader
        && let Some((w, h, rgba)) = loader(&home, track_id)
    {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.icons.insert(track_id.to_owned(), (w, h, rgba.clone()));
        return Some((w, h, rgba));
    }
    let url = icon_url?.trim();
    if !url.starts_with("https://") {
        return None;
    }
    let loader = net_loader?;
    {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        if !st.icon_in_flight.insert(track_id.to_owned()) {
            return None;
        }
    }
    let id_owned = track_id.to_owned();
    let url_owned = url.to_owned();
    std::thread::spawn(move || {
        let res = loader(&home, &id_owned, &url_owned);
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.icon_in_flight.remove(&id_owned);
        if let Ok((w, h, rgba)) = res {
            st.icons.insert(id_owned, (w, h, rgba));
        }
    });
    None
}

pub fn find_track_icon_by_url(stream_url: &str) -> Option<(String, u32, u32, Vec<u8>)> {
    if stream_url.is_empty() {
        return None;
    }
    let st = state().lock().unwrap_or_else(|e| e.into_inner());
    let candidate = st
        .in_game_search
        .results
        .iter()
        .chain(st.cached_downloaded.iter())
        .chain(st.library.playlists.iter().flat_map(|p| p.tracks.iter()))
        .find(|t| t.stream_url == stream_url)
        .map(|t| (t.id.clone(), t.icon_url.clone()));
    if let Some((id, icon_url)) = candidate {
        if let Some((w, h, rgba)) = st.icons.get(&id) {
            return Some((id, *w, *h, rgba.clone()));
        }
        drop(st);
        if let Some((w, h, rgba)) = get_or_request_icon(&id, icon_url.as_deref()) {
            return Some((id, w, h, rgba));
        }
    }
    None
}

pub fn playlists() -> Vec<MusicPlaylist> {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_loaded_locked(&mut st);
    ensure_offline_fresh_locked(&mut st);
    let mut out = st.library.playlists.clone();
    out.push(MusicPlaylist {
        name: DOWNLOADED_PLAYLIST_NAME.to_owned(),
        tracks: st.cached_downloaded.clone(),
    });
    out.push(MusicPlaylist {
        name: EXPORTED_PLAYLIST_NAME.to_owned(),
        tracks: st.cached_exported.clone(),
    });
    out
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
    if is_virtual_playlist(clean) {
        return Err(format!("\"{clean}\" is a built-in offline folder playlist"));
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
    if is_virtual_playlist(name) {
        return;
    }
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
    if is_virtual_playlist(clean) {
        return Err(format!("Cannot manually add songs to \"{clean}\""));
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
    if playlist_name.eq_ignore_ascii_case(DOWNLOADED_PLAYLIST_NAME) {
        if let Some(home) = st.home.clone() {
            if let Some(track) = st.cached_downloaded.iter().find(|t| t.id == track_id) {
                let _ = std::fs::remove_file(&track.stream_url);
            }
            let mut meta = load_downloaded_meta(&home);
            meta.retain(|t| t.id != track_id);
            save_downloaded_meta(&home, &meta);
            st.cached_offline_poll = None;
        }
        return;
    }
    if playlist_name.eq_ignore_ascii_case(EXPORTED_PLAYLIST_NAME) {
        if let Some(track) = st.cached_exported.iter().find(|t| t.id == track_id) {
            let _ = std::fs::remove_file(&track.stream_url);
            st.cached_offline_poll = None;
        }
        return;
    }
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

pub fn is_track_downloaded(track: &MusicTrack) -> bool {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    ensure_offline_fresh_locked(&mut st);
    st.cached_downloaded
        .iter()
        .any(|t| t.id == track.id || t.stream_url == track.stream_url)
}

pub fn active_downloads() -> Vec<DownloadProgress> {
    let st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.downloads.clone()
}

pub fn dismiss_finished_downloads() {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.downloads.retain(|d| !d.done);
}

fn sanitize_filename(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(64));
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
        if out.len() >= 48 {
            break;
        }
    }
    let trimmed = out.trim().trim_matches('_');
    if trimmed.is_empty() {
        "track".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn short_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn start_download_track(track: MusicTrack) {
    let home = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        let Some(home) = st.home.clone() else {
            return;
        };
        if st.downloads.iter().any(|d| d.id == track.id && !d.done) {
            return;
        }
        st.downloads.retain(|d| d.id != track.id);
        st.downloads.push(DownloadProgress {
            id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist.clone(),
            downloaded_bytes: 0,
            total_bytes: None,
            done: false,
            error: None,
        });
        home
    };

    std::thread::spawn(move || {
        let result = download_track_blocking(&home, &track);
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(item) = st.downloads.iter_mut().find(|d| d.id == track.id) {
            item.done = true;
            if let Err(err) = result {
                item.error = Some(err);
            }
        }
        st.cached_offline_poll = None;
    });
}

fn download_track_blocking(home: &Path, track: &MusicTrack) -> Result<(), String> {
    if !track.stream_url.starts_with("https://") && !track.stream_url.starts_with("http://") {
        return Err("Track is already local".to_owned());
    }
    let dir = downloaded_dir(home);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create download folder: {e}"))?;

    let (resp, _final_url) = open_track_audio_stream(track)?;
    let total_bytes = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok());
    let ext = match resp
        .header("Content-Type")
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        ct if ct.contains("ogg") => "ogg",
        ct if ct.contains("mp4") || ct.contains("m4a") || ct.contains("aac") => "m4a",
        ct if ct.contains("flac") => "flac",
        ct if ct.contains("wav") => "wav",
        _ => "mp3",
    };

    {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(item) = st.downloads.iter_mut().find(|d| d.id == track.id) {
            item.total_bytes = total_bytes;
        }
    }

    let base = sanitize_filename(&format!("{} - {}", track.title, track.artist));
    let hash = short_hash(&track.id);
    let final_path = dir.join(format!("{base}_{hash:08x}.{ext}"));
    let part_path = dir.join(format!("{base}_{hash:08x}.{ext}.part"));

    let mut file = std::fs::File::create(&part_path)
        .map_err(|e| format!("Cannot create file: {e}"))?;
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 32 * 1024];
    let mut written: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Network read error: {e}"))?;
        if n == 0 {
            break;
        }
        use std::io::Write as _;
        file.write_all(&buf[..n])
            .map_err(|e| format!("Disk write error: {e}"))?;
        written += n as u64;
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(item) = st.downloads.iter_mut().find(|d| d.id == track.id) {
            item.downloaded_bytes = written;
        }
    }
    drop(file);
    if written == 0 {
        let _ = std::fs::remove_file(&part_path);
        return Err("Empty audio stream received".to_owned());
    }
    std::fs::rename(&part_path, &final_path)
        .map_err(|e| format!("Cannot finalize download: {e}"))?;

    let mut offline_track = track.clone();
    offline_track.stream_url = final_path.to_string_lossy().into_owned();
    offline_track.fallback_urls.clear();
    offline_track.source = DOWNLOADED_PLAYLIST_NAME.to_owned();

    let mut meta = load_downloaded_meta(home);
    meta.retain(|t| t.id != offline_track.id && t.stream_url != offline_track.stream_url);
    meta.push(offline_track);
    save_downloaded_meta(home, &meta);
    Ok(())
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
        ensure_offline_fresh_locked(&mut st);
        let found = if playlist_name.eq_ignore_ascii_case(DOWNLOADED_PLAYLIST_NAME) {
            st.cached_downloaded.clone()
        } else if playlist_name.eq_ignore_ascii_case(EXPORTED_PLAYLIST_NAME) {
            st.cached_exported.clone()
        } else {
            st.library
                .playlists
                .iter()
                .find(|p| p.name.eq_ignore_ascii_case(playlist_name))
                .map(|p| p.tracks.clone())
                .unwrap_or_default()
        };
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
    st.cached_discord_poll = Some(Instant::now());
    st.cached_discord_now_playing = next.clone();
    next
}

pub fn in_game_search_snapshot() -> InGameSearchSnapshot {
    let st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.in_game_search.clone()
}

pub fn set_in_game_query(query: String) {
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.in_game_search.query = query;
}

pub fn trigger_in_game_search(
    query: String,
    genre: Option<String>,
    sort: MusicSort,
    page: usize,
) {
    let gen_id = {
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.in_game_search_gen += 1;
        st.in_game_search.query = query.clone();
        st.in_game_search.genre = genre.clone();
        st.in_game_search.sort = sort;
        st.in_game_search.page = page;
        st.in_game_search.searching = true;
        st.in_game_search.fetched_initial = true;
        st.in_game_search.status = if query.trim().is_empty() {
            "Loading songs...".to_owned()
        } else {
            format!("Searching \"{}\"...", query.trim())
        };
        st.in_game_search_gen
    };

    std::thread::spawn(move || {
        let res = search_online_tracks_paged(&query, genre.as_deref(), sort, page);
        let tracks_to_warm: Vec<(String, Option<String>)> = match &res {
            Ok(p) => p
                .tracks
                .iter()
                .map(|t| (t.id.clone(), t.icon_url.clone()))
                .collect(),
            Err(_) => Vec::new(),
        };
        {
            let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
            if st.in_game_search_gen != gen_id {
                return;
            }
            st.in_game_search.searching = false;
            match res {
                Ok(page_res) => {
                    st.in_game_search.has_next = page_res.has_next;
                    st.in_game_search.status = if page_res.tracks.is_empty() {
                        "No songs found.".to_owned()
                    } else {
                        format!("{} songs (Page {})", page_res.tracks.len(), page + 1)
                    };
                    st.in_game_search.results = page_res.tracks;
                }
                Err(err) => {
                    st.in_game_search.has_next = false;
                    st.in_game_search.status = err;
                }
            }
        }
        for (id, icon_url) in tracks_to_warm {
            let _ = get_or_request_icon(&id, icon_url.as_deref());
        }
    });
}

pub fn format_duration(secs: u64) -> String {
    let mins = secs / 60;
    let rem = secs % 60;
    format!("{mins}:{rem:02}")
}

pub fn format_ms(ms: u64) -> String {
    format_duration(ms / 1000)
}

pub fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    if bytes as f64 >= MIB {
        format!("{:.1} MB", bytes as f64 / MIB)
    } else if bytes as f64 >= KIB {
        format!("{:.0} KB", bytes as f64 / KIB)
    } else {
        format!("{bytes} B")
    }
}

const STREAM_USER_AGENT: &str =
    "Mozilla/5.0 (Linux; Android 14; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";

static AUDIUS_HOST_CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>)>>> = OnceLock::new();

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .resolver(horizon_port_pack::net::PreferIpv4Resolver)
        .user_agent(STREAM_USER_AGENT)
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(25))
        .redirects(10)
        .build()
}

fn no_redirect_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .resolver(horizon_port_pack::net::PreferIpv4Resolver)
        .user_agent(STREAM_USER_AGENT)
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .redirects(0)
        .build()
}

fn audius_hosts(agent: &ureq::Agent) -> Vec<String> {
    let cache = AUDIUS_HOST_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock()
        && let Some((at, cached)) = guard.as_ref()
        && at.elapsed() < Duration::from_secs(300)
        && !cached.is_empty()
    {
        return cached.clone();
    }

    let mut hosts = Vec::new();
    if let Ok(resp) = agent.get("https://api.audius.co").call()
        && let Ok(val) = resp.into_json::<serde_json::Value>()
        && let Some(arr) = val.get("data").and_then(|d| d.as_array())
    {
        for item in arr.iter().take(6) {
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
    if !hosts.iter().any(|h| h == "https://api.audius.co") {
        hosts.push("https://api.audius.co".to_owned());
    }
    if let Ok(mut guard) = cache.lock() {
        *guard = Some((Instant::now(), hosts.clone()));
    }
    hosts
}

fn push_unique_url(out: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if (trimmed.starts_with("https://") || trimmed.starts_with("http://"))
        && !out.iter().any(|existing| existing == trimmed)
    {
        out.push(trimmed.to_owned());
    }
}

fn extract_audius_track_id(track: &MusicTrack) -> Option<String> {
    if let Some(rest) = track.id.strip_prefix("audius:")
        && !rest.is_empty()
    {
        return Some(rest.to_owned());
    }
    if let Some(idx) = track.stream_url.find("/v1/tracks/") {
        let rest = &track.stream_url[idx + "/v1/tracks/".len()..];
        let tid = rest.split(['/', '?']).next().unwrap_or("").trim();
        if !tid.is_empty() {
            return Some(tid.to_owned());
        }
    }
    None
}

fn concise_ureq_error(err: &ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, _) => format!("HTTP {code}"),
        ureq::Error::Transport(t) => format!("{:?}", t.kind()),
    }
}

fn is_valid_audio_content_type(ct: &str) -> bool {
    let low = ct.trim().to_ascii_lowercase();
    if low.is_empty() {
        return true;
    }
    !low.contains("text/html")
        && !low.contains("application/json")
        && !low.contains("application/xml")
        && !low.contains("text/plain")
        && !low.contains("mpegurl")
}

fn open_single_url_with_redirects(
    agent: &ureq::Agent,
    start_url: &str,
) -> Result<(ureq::Response, String), String> {
    let mut current = start_url.to_owned();
    for _hop in 0..10 {
        let resp = agent
            .get(&current)
            .set("Accept", "audio/mpeg, audio/*;q=0.9, */*;q=0.8")
            .call();
        match resp {
            Ok(r) => {
                let status = r.status();
                if (300..400).contains(&status) {
                    if let Some(loc) = r.header("Location") {
                        let base = url::Url::parse(&current).map_err(|e| e.to_string())?;
                        let mut next = base
                            .join(loc.trim())
                            .map_err(|e| e.to_string())?
                            .to_string();
                        if next.starts_with("http://") {
                            next = format!("https://{}", &next["http://".len()..]);
                        }
                        current = next;
                        continue;
                    }
                    return Err(format!("Redirect HTTP {status} without Location"));
                }
                if !(200..300).contains(&status) {
                    return Err(format!("HTTP {status}"));
                }
                let ct = r.header("Content-Type").unwrap_or("");
                if !is_valid_audio_content_type(ct) {
                    return Err(format!("Unexpected content-type ({ct})"));
                }
                return Ok((r, current));
            }
            Err(ureq::Error::Status(status, r)) => {
                if (300..400).contains(&status)
                    && let Some(loc) = r.header("Location")
                {
                    let base = url::Url::parse(&current).map_err(|e| e.to_string())?;
                    let mut next = base
                        .join(loc.trim())
                        .map_err(|e| e.to_string())?
                        .to_string();
                    if next.starts_with("http://") {
                        next = format!("https://{}", &next["http://".len()..]);
                    }
                    current = next;
                    continue;
                }
                return Err(format!("HTTP {status}"));
            }
            Err(err) => {
                return Err(concise_ureq_error(&err));
            }
        }
    }
    Err("Too many redirects".to_owned())
}

fn open_track_audio_stream(track: &MusicTrack) -> Result<(ureq::Response, String), String> {
    let api_agent = http_agent();
    let stream_agent = no_redirect_agent();
    let mut candidates = Vec::new();
    push_unique_url(&mut candidates, &track.stream_url);
    for u in &track.fallback_urls {
        push_unique_url(&mut candidates, u);
    }

    let mut last_err = String::new();
    for url in &candidates {
        match open_single_url_with_redirects(&stream_agent, url) {
            Ok(hit) => return Ok(hit),
            Err(err) => last_err = err,
        }
    }

    if let Some(tid) = extract_audius_track_id(track) {
        let mut dynamic = Vec::new();
        for host in audius_hosts(&api_agent).into_iter().take(5) {
            // 1. Ask discovery node for the signed content-node URL via no_redirect=true
            let nr_url = format!("{host}/v1/tracks/{tid}/stream?no_redirect=true&app_name=VoltPlus");
            if let Ok(resp) = api_agent.get(&nr_url).call()
                && let Ok(val) = resp.into_json::<serde_json::Value>()
            {
                if let Some(u) = val.get("data").and_then(|d| d.as_str()) {
                    push_unique_url(&mut dynamic, u);
                }
            }
            // 2. Ask track metadata for stream.url + stream.mirrors
            let meta_url = format!("{host}/v1/tracks/{tid}?app_name=VoltPlus");
            if let Ok(resp) = api_agent.get(&meta_url).call()
                && let Ok(val) = resp.into_json::<serde_json::Value>()
            {
                if let Some(u) = val.pointer("/data/stream/url").and_then(|v| v.as_str()) {
                    push_unique_url(&mut dynamic, u);
                }
                if let Some(arr) = val.pointer("/data/stream/mirrors").and_then(|v| v.as_array()) {
                    for m in arr {
                        if let Some(u) = m.as_str() {
                            push_unique_url(&mut dynamic, u);
                        }
                    }
                }
            }
            // 3. Direct /stream endpoint on this discovery host
            push_unique_url(
                &mut dynamic,
                &format!("{host}/v1/tracks/{tid}/stream?app_name=VoltPlus"),
            );
        }
        for url in dynamic {
            if candidates.contains(&url) {
                continue;
            }
            match open_single_url_with_redirects(&stream_agent, &url) {
                Ok(hit) => return Ok(hit),
                Err(err) => last_err = err,
            }
        }
    }

    Err(if last_err.is_empty() {
        "Audio stream unavailable".to_owned()
    } else {
        format!("Audio unavailable ({last_err})")
    })
}

fn parse_audius_tracks(host: &str, json: &serde_json::Value) -> Vec<MusicTrack> {
    let Some(items) = json.get("data").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        if item.get("is_streamable").and_then(|v| v.as_bool()) == Some(false)
            || item.pointer("/access/stream").and_then(|v| v.as_bool()) == Some(false)
        {
            continue;
        }
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
        let play_count = item
            .get("play_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let icon_url = item
            .pointer("/artwork/150x150")
            .or_else(|| item.pointer("/artwork/480x480"))
            .or_else(|| item.pointer("/artwork/1000x1000"))
            .and_then(|v| v.as_str())
            .filter(|u| u.starts_with("https://"))
            .map(ToOwned::to_owned);

        let mut candidates = Vec::new();
        if let Some(direct) = item.pointer("/stream/url").and_then(|v| v.as_str()) {
            push_unique_url(&mut candidates, direct);
        }
        if let Some(mirrors) = item.pointer("/stream/mirrors").and_then(|v| v.as_array()) {
            for m in mirrors {
                if let Some(u) = m.as_str() {
                    push_unique_url(&mut candidates, u);
                }
            }
        }
        push_unique_url(
            &mut candidates,
            &format!("{host}/v1/tracks/{id}/stream?app_name=VoltPlus"),
        );
        push_unique_url(
            &mut candidates,
            &format!("https://api.audius.co/v1/tracks/{id}/stream?app_name=VoltPlus"),
        );
        let stream_url = candidates.remove(0);
        out.push(MusicTrack {
            id: format!("audius:{id}"),
            title: title.to_owned(),
            artist,
            duration_secs,
            stream_url,
            fallback_urls: candidates,
            icon_url,
            source: "Audius".to_owned(),
            play_count,
        });
    }
    out
}

fn apply_sort(tracks: &mut [MusicTrack], sort: MusicSort) {
    match sort {
        MusicSort::Popular => {}
        MusicSort::Relevance => {}
        MusicSort::MostPlayed => {
            tracks.sort_by(|a, b| b.play_count.cmp(&a.play_count));
        }
        MusicSort::Letters => {
            tracks.sort_by(|a, b| {
                a.title
                    .to_ascii_lowercase()
                    .cmp(&b.title.to_ascii_lowercase())
            });
        }
    }
}

fn search_audius_paged(
    query: &str,
    genre: Option<&str>,
    sort: MusicSort,
    page: usize,
) -> Result<MusicSearchPage, String> {
    let agent = http_agent();
    let trimmed = query.trim();
    let genre_clean = genre
        .map(str::trim)
        .filter(|g| !g.is_empty() && !g.eq_ignore_ascii_case("Trending"));
    let offset = page * PAGE_SIZE;
    let limit = PAGE_SIZE + 1;
    let combined = match (trimmed.is_empty(), genre_clean) {
        (true, None) => String::new(),
        (true, Some(g)) => g.to_owned(),
        (false, None) => trimmed.to_owned(),
        (false, Some(g)) => format!("{trimmed} {g}"),
    };
    let encoded_q = url::form_urlencoded::byte_serialize(combined.as_bytes()).collect::<String>();
    let sort_param = match sort {
        MusicSort::Relevance => "relevant",
        MusicSort::Popular | MusicSort::MostPlayed | MusicSort::Letters => "popular",
    };

    let mut last_err = String::new();
    for host in audius_hosts(&agent) {
        let endpoint = if trimmed.is_empty() {
            let time_param = if sort == MusicSort::MostPlayed {
                "&time=allTime"
            } else {
                ""
            };
            if let Some(g) = genre_clean {
                let enc_g =
                    url::form_urlencoded::byte_serialize(g.as_bytes()).collect::<String>();
                format!(
                    "{host}/v1/tracks/trending?genre={enc_g}{time_param}&limit={limit}&offset={offset}&app_name=VoltPlus"
                )
            } else {
                format!(
                    "{host}/v1/tracks/trending?{time_param}limit={limit}&offset={offset}&app_name=VoltPlus"
                )
                .replace("?&", "?")
            }
        } else {
            format!(
                "{host}/v1/tracks/search?query={encoded_q}&sort_method={sort_param}&limit={limit}&offset={offset}&app_name=VoltPlus"
            )
        };
        match agent.get(&endpoint).call() {
            Ok(resp) => {
                if let Ok(json) = resp.into_json::<serde_json::Value>() {
                    let mut tracks = parse_audius_tracks(&host, &json);
                    if tracks.is_empty() && trimmed.is_empty() && genre_clean.is_some() {
                        let fallback_ep = format!(
                            "{host}/v1/tracks/search?query={encoded_q}&sort_method={sort_param}&limit={limit}&offset={offset}&app_name=VoltPlus"
                        );
                        if let Ok(r2) = agent.get(&fallback_ep).call()
                            && let Ok(j2) = r2.into_json::<serde_json::Value>()
                        {
                            tracks = parse_audius_tracks(&host, &j2);
                        }
                    }
                    if !tracks.is_empty() || !combined.is_empty() {
                        let has_next = tracks.len() > PAGE_SIZE;
                        tracks.truncate(PAGE_SIZE);
                        apply_sort(&mut tracks, sort);
                        return Ok(MusicSearchPage { tracks, has_next });
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

fn search_archive_org_paged(
    query: &str,
    genre: Option<&str>,
    sort: MusicSort,
    page: usize,
) -> Result<MusicSearchPage, String> {
    let agent = http_agent();
    let trimmed = query.trim();
    let genre_clean = genre
        .map(str::trim)
        .filter(|g| !g.is_empty() && !g.eq_ignore_ascii_case("Trending"));
    let combined = match (trimmed.is_empty(), genre_clean) {
        (true, None) => String::new(),
        (true, Some(g)) => g.to_owned(),
        (false, None) => trimmed.to_owned(),
        (false, Some(g)) => format!("{trimmed} {g}"),
    };
    let q_expr = if combined.is_empty() {
        "mediatype:audio AND format:\"VBR MP3\" AND (subject:music OR subject:lofi OR subject:electronic)".to_owned()
    } else {
        format!("mediatype:audio AND format:\"VBR MP3\" AND ({combined})")
    };
    let encoded_q = url::form_urlencoded::byte_serialize(q_expr.as_bytes()).collect::<String>();
    let sort_q = match sort {
        MusicSort::Popular | MusicSort::MostPlayed => "&sort[]=downloads+desc",
        MusicSort::Letters => "&sort[]=titleSorter+asc",
        MusicSort::Relevance => "",
    };
    let url = format!(
        "https://archive.org/advancedsearch.php?q={encoded_q}&fl[]=identifier&fl[]=title&fl[]=creator&rows={}&page={}{sort_q}&output=json",
        PAGE_SIZE,
        page + 1
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
    let num_found = json
        .pointer("/response/numFound")
        .and_then(|n| n.as_u64())
        .unwrap_or(0) as usize;
    let has_next = (page + 1) * PAGE_SIZE < num_found && !docs.is_empty();
    let mut tracks = Vec::new();
    for doc in docs.into_iter().take(16) {
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
            play_count: 0,
            fallback_urls: Vec::new(),
        });
    }
    apply_sort(&mut tracks, sort);
    Ok(MusicSearchPage { tracks, has_next })
}

pub fn search_online_tracks_paged(
    query: &str,
    genre: Option<&str>,
    sort: MusicSort,
    page: usize,
) -> Result<MusicSearchPage, String> {
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
        return Ok(MusicSearchPage {
            tracks: vec![MusicTrack {
                id: format!("url:{trimmed}"),
                title,
                artist: parsed.host_str().unwrap_or("Custom Stream").to_owned(),
                duration_secs: 0,
                stream_url: trimmed.to_owned(),
                icon_url: None,
                source: "Direct URL".to_owned(),
                play_count: 0,
                fallback_urls: Vec::new(),
            }],
            has_next: false,
        });
    }
    match search_audius_paged(trimmed, genre, sort, page) {
        Ok(res) if !res.tracks.is_empty() => Ok(res),
        Ok(_) => search_archive_org_paged(trimmed, genre, sort, page),
        Err(audius_err) => match search_archive_org_paged(trimmed, genre, sort, page) {
            Ok(res) if !res.tracks.is_empty() => Ok(res),
            Ok(_) | Err(_) => Err(format!("Could not fetch online music: {audius_err}")),
        },
    }
}

/// Searches online music tracks (Audius primary, Internet Archive fallback) or constructs
/// a direct stream entry for a pasted `https://` audio URL.
pub fn search_online_tracks(query: &str) -> Result<Vec<MusicTrack>, String> {
    search_online_tracks_paged(query, None, MusicSort::Popular, 0).map(|p| p.tracks)
}
