//! In-app GitHub release update detection and installer for Android.
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub const CURRENT_VERSION: &str = "1.0.1";
pub const REPO: &str = "entitybrian69-bit/volt-plus";

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

static STATUS: Mutex<Option<UpdateStatus>> = Mutex::new(None);
static LAST_INFO: Mutex<Option<ReleaseInfo>> = Mutex::new(None);
static CHECKING: AtomicBool = AtomicBool::new(false);

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
    set_status(UpdateStatus::Dismissed);
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
    parse_version(remote) > parse_version(current)
}

/// Fetches the latest release from the repository and parses release metadata.
pub fn check_for_update() -> Result<Option<ReleaseInfo>, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = ureq::get(&url)
        .set("User-Agent", &format!("VoltPlus-Android/{CURRENT_VERSION}"))
        .set("Accept", "application/vnd.github.v3+json")
        .timeout(std::time::Duration::from_secs(12))
        .call()
        .map_err(|e| format!("Could not reach GitHub Releases: {e}"))?;

    let val: Value = resp
        .into_json()
        .map_err(|e| format!("Invalid release JSON response: {e}"))?;

    let tag = val["tag_name"].as_str().unwrap_or("").trim().to_owned();
    let name = val["name"].as_str().unwrap_or(&tag).trim().to_owned();
    let changelog = val["body"]
        .as_str()
        .unwrap_or("New release with improvements and fixes.")
        .trim()
        .to_owned();

    let remote_ver = tag.trim_start_matches('v').trim_start_matches('V');
    if !is_newer_version(remote_ver, CURRENT_VERSION) {
        return Ok(None);
    }

    let mut download_url = None;
    let mut asset_name = format!("volt-plus-{tag}.apk");
    let mut size_bytes = 0u64;

    if let Some(assets) = val["assets"].as_array() {
        let preferred_abi = if cfg!(target_arch = "aarch64") {
            "arm64-v8a"
        } else if cfg!(target_arch = "arm") {
            "armeabi-v7a"
        } else {
            "universal"
        };
        let mut best_score = 0u8;
        for asset in assets {
            let a_name = asset["name"].as_str().unwrap_or("");
            if !a_name.ends_with(".apk") {
                continue;
            }
            let lower = a_name.to_ascii_lowercase();
            let score = if lower.contains(preferred_abi) {
                3
            } else if lower.contains("universal") {
                2
            } else {
                1
            };
            if score > best_score {
                best_score = score;
                asset_name = a_name.to_owned();
                download_url = asset["browser_download_url"].as_str().map(str::to_owned);
                size_bytes = asset["size"].as_u64().unwrap_or(0);
            }
        }
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

/// Initiates an update check on a background worker thread.
pub fn check_in_background() {
    if CHECKING.swap(true, Ordering::SeqCst) {
        return;
    }
    set_status(UpdateStatus::Checking);
    std::thread::Builder::new()
        .name("UpdateChecker".into())
        .spawn(|| {
            match check_for_update() {
                Ok(Some(info)) => {
                    log::info!("New update available: {}", info.version);
                    set_status(UpdateStatus::Available(info));
                }
                Ok(None) => {
                    log::info!("Horizon is up to date (version {CURRENT_VERSION})");
                    set_status(UpdateStatus::Idle);
                }
                Err(err) => {
                    log::warn!("Update check failed: {err}");
                    set_status(UpdateStatus::Error(err));
                }
            }
            CHECKING.store(false, Ordering::SeqCst);
        })
        .ok();
}

/// Opens the verified GitHub release download URL in the system browser.
pub fn download_and_install_in_background(info: ReleaseInfo, _home: PathBuf) {
    let url = info
        .download_url
        .filter(|u| u.starts_with(&format!("https://github.com/{REPO}/")))
        .unwrap_or_else(|| format!("https://github.com/{REPO}/releases/latest"));
    open_release_url(&url);
    set_status(UpdateStatus::Dismissed);
}

/// Opens the verified release download URL in Android's browser via JNI.
pub fn trigger_install(_apk_path: &Path) {
    let url = last_release_info()
        .and_then(|info| info.download_url)
        .filter(|u| u.starts_with(&format!("https://github.com/{REPO}/")))
        .unwrap_or_else(|| format!("https://github.com/{REPO}/releases/latest"));
    open_release_url(&url);
}

fn open_release_url(url: &str) {
    #[cfg(target_os = "android")]
    {
        crate::platform::with_context(|env, activity| {
            let j_url = env.new_string(url)?;
            let cls = env.get_object_class(activity)?;
            let _ = env.call_static_method(
                &cls,
                "openUpdateUrl",
                "(Landroid/content/Context;Ljava/lang/String;)V",
                &[
                    jni::objects::JValue::Object(activity),
                    jni::objects::JValue::Object(&j_url.into()),
                ],
            );
            Ok(())
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        log::info!("Opening release URL: {url}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing_and_comparison() {
        assert_eq!(parse_version("1.0.0"), (1, 0, 0, 0));
        assert_eq!(parse_version("v1.0.1"), (1, 0, 1, 0));
        assert_eq!(parse_version("1.2.3-dev.4"), (1, 2, 3, 4));

        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("v1.1.0", "1.0.0"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("0.9.9", "1.0.0"));
    }
}
