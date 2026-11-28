//! In-app GitHub release update detection, direct APK download, and PackageInstaller launcher for Android.
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::ToSocketAddrs,
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const CURRENT_VERSION: &str = "1.0";
pub const REPO: &str = "entitybrian69-bit/volt-plus";
pub const OWNER_GITHUB_URL: &str = "https://github.com/entitybrian69-bit";

#[derive(Clone, Debug, PartialEq)]
pub struct ReleaseInfo {
    pub version: String,
    pub tag_name: String,
    pub title: String,
    pub changelog: String,
    pub download_url: Option<String>,
    pub asset_name: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(ReleaseInfo),
    Downloading {
        progress: f32,
        downloaded: u64,
        total: u64,
    },
    Downloaded {
        path: PathBuf,
        version: String,
    },
    Error(String),
    Dismissed,
}

static HOME_DIR: OnceLock<PathBuf> = OnceLock::new();
static STATUS: Mutex<Option<UpdateStatus>> = Mutex::new(None);
static LAST_INFO: Mutex<Option<ReleaseInfo>> = Mutex::new(None);
static LAST_DOWNLOADED_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
static CHECKING: AtomicBool = AtomicBool::new(false);
static DOWNLOADING: AtomicBool = AtomicBool::new(false);
static AUTO_CHECK_STARTED: AtomicBool = AtomicBool::new(false);
static AUTO_PROMPT_ACTIVE: AtomicBool = AtomicBool::new(false);
static LAST_CHECK_UP_TO_DATE: AtomicBool = AtomicBool::new(false);

struct PreferIpv4Resolver;
impl ureq::Resolver for PreferIpv4Resolver {
    fn resolve(&self, netloc: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
        let mut addrs: Vec<_> = netloc.to_socket_addrs()?.collect();
        // Many mobile networks hang on IPv6 AAAA records; try IPv4 addresses first.
        addrs.sort_by_key(|a| u8::from(!a.is_ipv4()));
        Ok(addrs)
    }
}

pub fn init_home(home: &Path) {
    let _ = HOME_DIR.set(home.to_path_buf());
}

fn seen_version_file() -> Option<PathBuf> {
    HOME_DIR
        .get()
        .map(|h| h.join("updater-seen-version.txt"))
}

fn is_version_seen(version: &str) -> bool {
    let clean = version.trim();
    if clean.is_empty() {
        return false;
    }
    let Some(file) = seen_version_file() else {
        return false;
    };
    std::fs::read_to_string(file)
        .map(|s| s.trim() == clean)
        .unwrap_or(false)
}

fn persist_seen_version(version: &str) {
    let clean = version.trim();
    if clean.is_empty() {
        return;
    }
    if let Some(file) = seen_version_file() {
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(file, clean);
    }
}

pub fn should_auto_prompt() -> bool {
    AUTO_PROMPT_ACTIVE.load(Ordering::Relaxed)
}

pub fn mark_auto_prompt_shown() {
    if let Some(info) = last_release_info() {
        persist_seen_version(&info.version);
    }
}

pub fn last_check_up_to_date() -> bool {
    LAST_CHECK_UP_TO_DATE.load(Ordering::Relaxed)
}

pub fn last_downloaded_path() -> Option<PathBuf> {
    LAST_DOWNLOADED_PATH
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .filter(|p| p.is_file())
}

pub fn get_status() -> UpdateStatus {
    STATUS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or(UpdateStatus::Idle)
}

pub fn last_release_info() -> Option<ReleaseInfo> {
    LAST_INFO
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

pub fn set_status(new_status: UpdateStatus) {
    if let UpdateStatus::Available(ref info) = new_status {
        *LAST_INFO.lock().unwrap_or_else(|e| e.into_inner()) = Some(info.clone());
    }
    *STATUS.lock().unwrap_or_else(|e| e.into_inner()) = Some(new_status);
}

pub fn dismiss() {
    AUTO_PROMPT_ACTIVE.store(false, Ordering::Relaxed);
    if let Some(info) = last_release_info() {
        persist_seen_version(&info.version);
    }
    set_status(UpdateStatus::Dismissed);
}

/// Returns the installed Android `packageName` (`vp.net.voltplus` or `vp.net.voltplus.debug`).
pub fn installed_package_name() -> String {
    #[cfg(target_os = "android")]
    {
        let found: Option<String> = crate::platform::with_context(|env, activity| {
            let obj = env
                .call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])?
                .l()?;
            if obj.is_null() {
                return Ok(String::new());
            }
            let s: jni::objects::JString<'_> = obj.into();
            Ok(String::from(env.get_string(&s)?))
        });
        if let Some(pkg) = found
            && !pkg.trim().is_empty()
        {
            return pkg;
        }
    }
    "vp.net.voltplus".to_owned()
}

/// Returns the installed APK's `versionName` via Android's PackageManager, falling back to `CURRENT_VERSION`.
pub fn installed_version_name() -> String {
    #[cfg(target_os = "android")]
    {
        let found: Option<String> = crate::platform::with_context(|env, activity| {
            let obj = env
                .call_method(activity, "versionName", "()Ljava/lang/String;", &[])?
                .l()?;
            if obj.is_null() {
                return Ok(String::new());
            }
            let s: jni::objects::JString<'_> = obj.into();
            Ok(String::from(env.get_string(&s)?))
        });
        if let Some(v) = found
            && !v.trim().is_empty()
        {
            return v;
        }
    }
    CURRENT_VERSION.to_owned()
}

/// Parses version string into numeric tuple (major, minor, patch, prerelease).
pub fn parse_version(raw: &str) -> (u32, u32, u32, u32) {
    let clean = raw.trim().trim_start_matches('v').trim_start_matches('V');
    let mut parts = clean.split(&['.', '-', '+'][..]);
    let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let build = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch, build)
}

/// Returns true if `remote` is strictly higher than `current`.
pub fn is_newer_version(remote: &str, current: &str) -> bool {
    let current_base = if (current.contains("-debug.") || current.contains("-dev."))
        && !remote.contains("-debug.")
        && !remote.contains("-dev.")
    {
        current.split('-').next().unwrap_or(current)
    } else {
        current
    };
    parse_version(remote) > parse_version(current_base)
}

/// Queries Android's `Build.SUPPORTED_ABIS` via JNI, falling back to compile-time `target_arch`.
fn device_abis() -> Vec<String> {
    #[cfg(target_os = "android")]
    {
        let found: Option<Vec<String>> = crate::platform::with_context(|env, _activity| {
            let build_cls = env.find_class("android/os/Build")?;
            let abis_obj = env
                .get_static_field(&build_cls, "SUPPORTED_ABIS", "[Ljava/lang/String;")?
                .l()?;
            if abis_obj.is_null() {
                return Ok(Vec::new());
            }
            let arr: jni::objects::JObjectArray<'_> = abis_obj.into();
            let len = env.get_array_length(&arr)?;
            let mut out = Vec::with_capacity(len.max(0) as usize);
            for i in 0..len {
                let elem = env.get_object_array_element(&arr, i)?;
                if !elem.is_null() {
                    let s: jni::objects::JString<'_> = elem.into();
                    if let Ok(java_str) = env.get_string(&s) {
                        out.push(String::from(java_str).to_ascii_lowercase());
                    }
                }
            }
            Ok(out)
        });
        if let Some(abis) = found
            && !abis.is_empty()
        {
            return abis;
        }
    }
    if cfg!(target_arch = "aarch64") {
        vec!["arm64-v8a".into(), "armeabi-v7a".into()]
    } else if cfg!(target_arch = "arm") {
        vec!["armeabi-v7a".into()]
    } else if cfg!(target_arch = "x86_64") {
        vec!["x86_64".into(), "x86".into()]
    } else if cfg!(target_arch = "x86") {
        vec!["x86".into()]
    } else {
        vec!["universal".into()]
    }
}

/// Scores a release `.apk` asset filename against the current device's supported ABIs.
/// Returns `0` for any APK targeting an incompatible ABI so wrong-ABI APKs are never offered.
pub fn score_apk_for_abis(asset_name: &str, abis: &[String]) -> u32 {
    score_apk_for_abis_and_pkg(asset_name, abis, false)
}

fn score_apk_for_abis_and_pkg(asset_name: &str, abis: &[String], is_debug_pkg: bool) -> u32 {
    let lower = asset_name.to_ascii_lowercase();
    if !lower.ends_with(".apk") {
        return 0;
    }
    let asset_is_debug = lower.contains("-debug") || lower.contains(".debug") || lower.contains("_debug");
    if is_debug_pkg != asset_is_debug {
        return 0;
    }
    let has_arm64 = lower.contains("arm64-v8a") || lower.contains("arm64") || lower.contains("aarch64");
    let has_armv7 = lower.contains("armeabi-v7a") || lower.contains("armv7") || lower.contains("armeabi");
    let has_x86_64 = lower.contains("x86_64") || lower.contains("amd64");
    let has_x86 = !has_x86_64 && (lower.contains("-x86") || lower.contains("_x86") || lower.contains(".x86"));
    let is_universal = lower.contains("universal");
    let has_any_abi = has_arm64 || has_armv7 || has_x86_64 || has_x86;

    if is_universal {
        // Universal APK merges arm64-v8a + armeabi-v7a; preferred over a secondary ABI fallback,
        // while an exact primary ABI build (`100`) always wins.
        let arm_or_generic = abis.iter().any(|a| a.contains("arm")) || abis.iter().any(|a| a == "universal");
        return if arm_or_generic { 50 } else { 0 };
    }
    if !has_any_abi {
        return 10;
    }
    for (idx, abi) in abis.iter().enumerate() {
        let matched = match abi.as_str() {
            "arm64-v8a" | "aarch64" => has_arm64,
            "armeabi-v7a" | "armeabi" | "armv7" => has_armv7,
            "x86_64" | "amd64" => has_x86_64,
            "x86" | "i686" => has_x86,
            _ => false,
        };
        if matched {
            return if idx == 0 {
                100
            } else {
                30u32.saturating_sub((idx as u32) * 5)
            };
        }
    }
    0
}

/// Fetches the latest published GitHub Release (never drafts or prereleases) and selects
/// the best ABI-compatible APK asset for the currently installed package identity.
pub fn check_for_update() -> Result<Option<ReleaseInfo>, String> {
    let installed_ver = installed_version_name();
    let is_debug_pkg = installed_package_name().ends_with(".debug");
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let agent = ureq::AgentBuilder::new()
        .resolver(PreferIpv4Resolver)
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(12))
        .build();
    let resp = agent
        .get(&url)
        .set("User-Agent", &format!("VoltPlus-Android/{installed_ver}"))
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "No published releases found yet.".to_owned(),
            other => format!("Could not reach GitHub Releases: {other}"),
        })?;

    let val: Value = resp
        .into_json()
        .map_err(|e| format!("Invalid release JSON response: {e}"))?;

    if val["draft"].as_bool() == Some(true) || val["prerelease"].as_bool() == Some(true) {
        return Ok(None);
    }

    let tag = val["tag_name"].as_str().unwrap_or("").trim().to_owned();
    let name = val["name"].as_str().unwrap_or(&tag).trim().to_owned();
    let changelog = val["body"]
        .as_str()
        .unwrap_or("New release with improvements and fixes.")
        .trim()
        .to_owned();

    let remote_ver = tag.trim_start_matches('v').trim_start_matches('V');
    if remote_ver.is_empty() || !is_newer_version(remote_ver, &installed_ver) {
        return Ok(None);
    }

    let abis = device_abis();
    let mut download_url = None;
    let mut asset_name = String::new();
    let mut size_bytes = 0u64;

    if let Some(assets) = val["assets"].as_array() {
        let mut best_score = 0u32;
        for asset in assets {
            let a_name = asset["name"].as_str().unwrap_or("");
            let Some(a_url) = asset["browser_download_url"].as_str() else {
                continue;
            };
            if !a_url.starts_with(&format!("https://github.com/{REPO}/")) {
                continue;
            }
            let score = score_apk_for_abis_and_pkg(a_name, &abis, is_debug_pkg);
            if score > best_score {
                best_score = score;
                asset_name = a_name.to_owned();
                download_url = Some(a_url.to_owned());
                size_bytes = asset["size"].as_u64().unwrap_or(0);
            }
        }
    }

    // Do not show an update prompt if the release has no compatible APK asset for this device.
    if download_url.is_none() {
        return Ok(None);
    }

    Ok(Some(ReleaseInfo {
        version: remote_ver.to_owned(),
        tag_name: tag,
        title: name,
        changelog,
        download_url,
        asset_name,
        size_bytes,
    }))
}

/// Automatic startup/in-game check: runs at most once per process and only raises
/// the auto-popup once per remote release version.
pub fn check_in_background() {
    if AUTO_CHECK_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    run_check(true);
}

/// Manual check from Settings -> About: always queries GitHub and surfaces the
/// result inline without triggering an unexpected popup.
pub fn check_manual_in_background() {
    run_check(false);
}

fn run_check(auto_prompt: bool) {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    if !auto_prompt {
        LAST_CHECK_UP_TO_DATE.store(false, Ordering::Relaxed);
        set_status(UpdateStatus::Checking);
    }
    std::thread::Builder::new()
        .name("UpdateChecker".into())
        .spawn(move || {
            match check_for_update() {
                Ok(Some(info)) => {
                    log::info!("New update available: {}", info.version);
                    LAST_CHECK_UP_TO_DATE.store(false, Ordering::Relaxed);
                    let already_seen = is_version_seen(&info.version);
                    if auto_prompt && !already_seen {
                        AUTO_PROMPT_ACTIVE.store(true, Ordering::Relaxed);
                    }
                    set_status(UpdateStatus::Available(info));
                }
                Ok(None) => {
                    log::info!("Volt Plus is up to date ({})", installed_version_name());
                    *LAST_INFO.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    AUTO_PROMPT_ACTIVE.store(false, Ordering::Relaxed);
                    if !auto_prompt {
                        LAST_CHECK_UP_TO_DATE.store(true, Ordering::Relaxed);
                    }
                    set_status(UpdateStatus::Idle);
                }
                Err(err) => {
                    log::warn!("Update check failed: {err}");
                    if !auto_prompt {
                        set_status(UpdateStatus::Error(err));
                    }
                }
            }
            CHECKING.store(false, Ordering::SeqCst);
        })
        .ok();
}

fn update_cache_dir(fallback_home: &Path) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        let found: Option<String> = crate::platform::with_context(|env, activity| {
            let cls = env.get_object_class(activity)?;
            let obj = env
                .call_static_method(
                    &cls,
                    "getUpdateCacheDir",
                    "(Landroid/content/Context;)Ljava/lang/String;",
                    &[jni::objects::JValue::Object(activity)],
                )?
                .l()?;
            if obj.is_null() {
                return Ok(String::new());
            }
            let s: jni::objects::JString<'_> = obj.into();
            Ok(String::from(env.get_string(&s)?))
        });
        if let Some(dir) = found
            && !dir.trim().is_empty()
        {
            return PathBuf::from(dir);
        }
    }
    fallback_home.join("cache").join("updates")
}

fn sanitize_apk_filename(name: &str, version: &str) -> String {
    let base: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    if base.is_empty() || !base.to_ascii_lowercase().ends_with(".apk") {
        let ver_safe: String = version
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
            .collect();
        format!("VoltPlus-{ver_safe}.apk")
    } else {
        base
    }
}

/// Downloads the verified APK directly to the app's private update cache on a worker thread
/// and automatically opens Android's Package Installer on completion.
pub fn download_and_install_in_background(info: ReleaseInfo, home: PathBuf) {
    init_home(&home);
    AUTO_PROMPT_ACTIVE.store(false, Ordering::Relaxed);
    persist_seen_version(&info.version);

    if let Some(existing) = last_downloaded_path()
        && existing
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == sanitize_apk_filename(&info.asset_name, &info.version))
        && (info.size_bytes == 0
            || std::fs::metadata(&existing).is_ok_and(|m| m.len() == info.size_bytes))
    {
        set_status(UpdateStatus::Downloaded {
            path: existing.clone(),
            version: info.version,
        });
        trigger_install(&existing);
        return;
    }

    if DOWNLOADING.swap(true, Ordering::SeqCst) {
        return;
    }

    let Some(url) = info
        .download_url
        .clone()
        .filter(|u| u.starts_with(&format!("https://github.com/{REPO}/")))
    else {
        DOWNLOADING.store(false, Ordering::SeqCst);
        set_status(UpdateStatus::Error("Release does not provide a valid APK download URL.".into()));
        return;
    };

    let total_expected = info.size_bytes;
    set_status(UpdateStatus::Downloading {
        progress: 0.0,
        downloaded: 0,
        total: total_expected,
    });

    std::thread::Builder::new()
        .name("ApkUpdater".into())
        .spawn(move || {
            let res = download_apk_blocking(&url, &info, &home);
            DOWNLOADING.store(false, Ordering::SeqCst);
            match res {
                Ok(apk_path) => {
                    *LAST_DOWNLOADED_PATH.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(apk_path.clone());
                    set_status(UpdateStatus::Downloaded {
                        path: apk_path.clone(),
                        version: info.version,
                    });
                    trigger_install(&apk_path);
                }
                Err(err) => {
                    log::warn!("Update download failed: {err}");
                    set_status(UpdateStatus::Error(err));
                }
            }
        })
        .ok();
}

fn download_apk_blocking(url: &str, info: &ReleaseInfo, home: &Path) -> Result<PathBuf, String> {
    let dir = update_cache_dir(home);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not prepare update folder: {e}"))?;

    // Remove older cached update APKs before writing the new one.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let _ = std::fs::remove_file(p);
            }
        }
    }

    let filename = sanitize_apk_filename(&info.asset_name, &info.version);
    let final_path = dir.join(&filename);
    let part_path = dir.join(format!("{filename}.partial"));

    let agent = ureq::AgentBuilder::new()
        .resolver(PreferIpv4Resolver)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", &format!("VoltPlus-Android/{}", installed_version_name()))
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let header_len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let total = if info.size_bytes > 0 { info.size_bytes } else { header_len };

    let mut reader = resp.into_reader();
    let mut out = std::fs::File::create(&part_path)
        .map_err(|e| format!("Could not create update file: {e}"))?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut downloaded = 0u64;
    let mut first_four = [0u8; 4];
    let mut magic_len = 0usize;
    let mut last_tick = Instant::now();

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Network error while downloading update: {e}"))?;
        if n == 0 {
            break;
        }
        if magic_len < 4 {
            let take = (4 - magic_len).min(n);
            first_four[magic_len..magic_len + take].copy_from_slice(&buf[..take]);
            magic_len += take;
            if magic_len == 4 && first_four != [b'P', b'K', 0x03, 0x04] {
                let _ = std::fs::remove_file(&part_path);
                return Err("Downloaded file is not a valid APK archive.".into());
            }
        }
        out.write_all(&buf[..n])
            .map_err(|e| format!("Could not write update APK: {e}"))?;
        downloaded += n as u64;
        if downloaded > 512 * 1024 * 1024 {
            let _ = std::fs::remove_file(&part_path);
            return Err("Update APK exceeded maximum allowed size.".into());
        }
        if last_tick.elapsed() >= Duration::from_millis(80) {
            let progress = if total > 0 {
                (downloaded as f32 / total as f32).clamp(0.0, 0.99)
            } else {
                0.5
            };
            set_status(UpdateStatus::Downloading {
                progress,
                downloaded,
                total,
            });
            last_tick = Instant::now();
        }
    }
    out.flush().map_err(|e| format!("Could not finalize update APK: {e}"))?;
    drop(out);

    if downloaded < 1024 || magic_len < 4 {
        let _ = std::fs::remove_file(&part_path);
        return Err("Downloaded APK was incomplete.".into());
    }
    if info.size_bytes > 0 && downloaded != info.size_bytes {
        let _ = std::fs::remove_file(&part_path);
        return Err(format!(
            "Downloaded APK size mismatch ({downloaded} of {} bytes).",
            info.size_bytes
        ));
    }

    std::fs::rename(&part_path, &final_path)
        .map_err(|e| format!("Could not finalize update APK: {e}"))?;
    Ok(final_path)
}

/// Launches Android's system Package Installer for the downloaded APK (`content://...apkupdate/...`).
pub fn trigger_install(apk_path: &Path) {
    if !apk_path.is_file() {
        set_status(UpdateStatus::Error("Downloaded update file is missing. Please retry.".into()));
        return;
    }
    #[cfg(target_os = "android")]
    {
        let path_str = apk_path.to_string_lossy().to_string();
        let result: Option<String> = crate::platform::with_context(|env, activity| {
            let j_path = env.new_string(&path_str)?;
            let cls = env.get_object_class(activity)?;
            let res_obj = env
                .call_static_method(
                    &cls,
                    "installUpdateApk",
                    "(Landroid/content/Context;Ljava/lang/String;)Ljava/lang/String;",
                    &[
                        jni::objects::JValue::Object(activity),
                        jni::objects::JValue::Object(&j_path.into()),
                    ],
                )?
                .l()?;
            if res_obj.is_null() {
                return Ok("Failed to launch package installer.".to_owned());
            }
            let s: jni::objects::JString<'_> = res_obj.into();
            Ok(String::from(env.get_string(&s)?))
        });
        match result.as_deref() {
            Some("OK") => {
                log::info!("Launched Android Package Installer for {}", apk_path.display());
            }
            Some("PERMISSION_REQUIRED") => {
                set_status(UpdateStatus::Error(
                    "Allow 'Install unknown apps' in Android Settings, then tap Install Now.".into(),
                ));
            }
            Some(err) => {
                set_status(UpdateStatus::Error(err.to_owned()));
            }
            None => {
                set_status(UpdateStatus::Error(
                    "Could not connect to Android package installer.".into(),
                ));
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        log::info!("Would launch APK installer for {}", apk_path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing_and_comparison() {
        assert_eq!(parse_version("1.0"), (1, 0, 0, 0));
        assert_eq!(parse_version("1.0.0"), (1, 0, 0, 0));
        assert_eq!(parse_version("v1.0.1"), (1, 0, 1, 0));
        assert_eq!(parse_version("1.2.3-dev.4"), (1, 2, 3, 4));

        assert!(is_newer_version("1.0.1", "1.0"));
        assert!(is_newer_version("v1.1.0", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(is_newer_version("1.0.1", "1.0-debug.66.734d3d5"));
        assert!(!is_newer_version("1.0", "1.0-debug.66.734d3d5"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.9.9", "1.0.0"));
    }

    #[test]
    fn abi_and_package_matching_rejects_mismatched_builds() {
        let abis = vec!["arm64-v8a".to_owned(), "armeabi-v7a".to_owned()];
        assert_eq!(score_apk_for_abis_and_pkg("Volt-Plus.245.abc-arm64-v8a.apk", &abis, false), 100);
        assert_eq!(score_apk_for_abis_and_pkg("Volt-Plus-Debug.66.abc-arm64-v8a.apk", &abis, false), 0);
        assert_eq!(score_apk_for_abis_and_pkg("Volt-Plus-Debug.66.abc-arm64-v8a.apk", &abis, true), 100);
        assert_eq!(score_apk_for_abis_and_pkg("Volt-Plus.245.abc-arm64-v8a.apk", &abis, true), 0);
    }
}
