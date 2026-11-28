//! Original-quality wallpaper catalog and bounded, verified downloads.
//! The original H.264 files are decoded on Android without recompressing them.
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::ToSocketAddrs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static DOWNLOAD_DONE_BYTES: AtomicU64 = AtomicU64::new(0);
static DOWNLOAD_TOTAL_BYTES: AtomicU64 = AtomicU64::new(0);

/// Returns `(downloaded_bytes, total_bytes)` for an in-flight preset wallpaper download.
pub fn download_progress() -> (u64, u64) {
    (
        DOWNLOAD_DONE_BYTES.load(Ordering::Relaxed),
        DOWNLOAD_TOTAL_BYTES.load(Ordering::Relaxed),
    )
}

struct PreferIpv4Resolver;
impl ureq::Resolver for PreferIpv4Resolver {
    fn resolve(&self, netloc: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
        let mut addrs: Vec<_> = netloc.to_socket_addrs()?.collect();
        // Many mobile networks hang on IPv6 AAAA records; try IPv4 addresses first.
        addrs.sort_by_key(|a| u8::from(!a.is_ipv4()));
        Ok(addrs)
    }
}

pub struct Wallpaper {
    pub name: &'static str,
    pub filename: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    pub resolution: &'static str,
    pub width: u32,
    pub height: u32,
}

pub const WALLPAPERS: [Wallpaper; 5] = [
    Wallpaper { name: "Aurora night cabin", filename: "aurora.mp4", size: 8_272_647, sha256: "1785dbd85c19788dacdec03f0bc2f82656b4e66e90a071e7fbc97fcc39479d5a", resolution: "4K", width: 3840, height: 2160 },
    Wallpaper { name: "Cherry sunrise valley", filename: "cherry-sunrise.mp4", size: 5_930_493, sha256: "e87d9e48adebd0f5624255a9aecb6c78285fa85971b61a617135b48992f88f7e", resolution: "4K", width: 3840, height: 2160 },
    Wallpaper { name: "Minecraft Earth", filename: "earth.mp4", size: 14_365_508, sha256: "4253f906602eccad70b74f7db4c3bfb63e5a2e8c8d6a3082a307aadd308f11ad", resolution: "4K", width: 3840, height: 2160 },
    Wallpaper { name: "Peaceful greenhouse", filename: "greenhouse.mp4", size: 11_862_687, sha256: "95e479d8e9428630a8245e6371bcf810ba15e65ae4340047a893572a7f6a0d15", resolution: "1080p", width: 1920, height: 1080 },
    Wallpaper { name: "Sakura blossom house", filename: "sakura-house.mp4", size: 9_270_425, sha256: "ed7f53d57cb874725b870a8f571d30fb20e1b9047a51fe2bce3b254293bd285e", resolution: "1080p", width: 1920, height: 1080 },
];

/// The greenhouse ships with the app; `None` in older settings also means default.
pub const DEFAULT_INDEX: usize = 3;
pub fn effective_index(index: Option<usize>) -> usize {
    index.filter(|&i| i < WALLPAPERS.len()).unwrap_or(DEFAULT_INDEX)
}

/// Restore the original, bundled video once, atomically, without a network request.
pub fn ensure_bundled_default(home: &Path) -> Result<(), String> {
    static DEFAULT_VIDEO: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../wallpapers/greenhouse.mp4"));
    let item = &WALLPAPERS[DEFAULT_INDEX];
    let target = path(home, DEFAULT_INDEX).ok_or("Invalid default wallpaper")?;
    if std::fs::metadata(&target).is_ok_and(|m| m.len() == item.size) { return Ok(()); }
    let dir = target.parent().ok_or("Invalid wallpaper path")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = target.with_extension("mp4.bundled.part");
    let result = (|| -> Result<(), String> {
        std::fs::write(&tmp, DEFAULT_VIDEO).map_err(|e| e.to_string())?;
        if digest_file(&tmp, item.size)? != item.sha256 { return Err("Bundled video integrity check failed".into()); }
        std::fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() { let _ = std::fs::remove_file(&tmp); }
    result
}

pub fn digest_file(path: &Path, expected_size: u64) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() != expected_size {
        return Err("File size does not match".into());
    }
    let mut hash = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn path(home: &Path, index: usize) -> Option<PathBuf> {
    Some(home.join("wallpapers").join(WALLPAPERS.get(index)?.filename))
}

/// Run off the UI thread. Only a complete, hash-verified file is installed.
pub fn download(home: &Path, index: usize) -> Result<String, String> {
    let item = WALLPAPERS.get(index).ok_or("Unknown wallpaper")?;
    let target = path(home, index).ok_or("Unknown wallpaper")?;
    let dir = target.parent().ok_or("Invalid wallpaper path")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    if digest_file(&target, item.size).is_ok_and(|v| v == item.sha256) {
        DOWNLOAD_DONE_BYTES.store(item.size, Ordering::Relaxed);
        DOWNLOAD_TOTAL_BYTES.store(item.size, Ordering::Relaxed);
        return Ok(format!("{} is already downloaded", item.name));
    }
    DOWNLOAD_DONE_BYTES.store(0, Ordering::Relaxed);
    DOWNLOAD_TOTAL_BYTES.store(item.size, Ordering::Relaxed);
    let tmp = target.with_extension("mp4.part");
    let urls = [
        format!(
            "https://raw.githubusercontent.com/entitybrian69-bit/volt-plus/android/wallpapers/{}",
            item.filename
        ),
        format!(
            "https://raw.githubusercontent.com/entitybrian69-bit/volt-plus/arena/16c8aaea-volt-plus/wallpapers/{}",
            item.filename
        ),
        format!(
            "https://cdn.jsdelivr.net/gh/entitybrian69-bit/volt-plus@android/wallpapers/{}",
            item.filename
        ),
        format!(
            "https://github.com/entitybrian69-bit/volt-plus/raw/android/wallpapers/{}",
            item.filename
        ),
    ];
    let agent = ureq::AgentBuilder::new()
        .resolver(PreferIpv4Resolver)
        .user_agent("VoltPlus-Android/1.0")
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(45))
        .build();

    let mut last_err = String::from("Download failed");
    let mut succeeded = false;
    'mirrors: for url in &urls {
        for attempt in 0..2 {
            let _ = std::fs::remove_file(&tmp);
            DOWNLOAD_DONE_BYTES.store(0, Ordering::Relaxed);
            let resp = match agent.get(url).call() {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("Network error ({e})");
                    if attempt == 0 {
                        std::thread::sleep(Duration::from_millis(350));
                    }
                    continue;
                }
            };
            let copy_res = (|| -> Result<(), String> {
                let mut input = resp.into_reader().take(item.size + 1);
                let mut output = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
                let mut buf = [0u8; 64 * 1024];
                let mut written = 0u64;
                loop {
                    let n = input.read(&mut buf).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    output.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                    written = written.saturating_add(n as u64);
                    DOWNLOAD_DONE_BYTES.store(written.min(item.size), Ordering::Relaxed);
                }
                output.flush().map_err(|e| e.to_string())?;
                drop(output);
                if digest_file(&tmp, item.size)? != item.sha256 {
                    return Err("Downloaded video failed its integrity check".into());
                }
                std::fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
                Ok(())
            })();
            match copy_res {
                Ok(()) => {
                    succeeded = true;
                    break 'mirrors;
                }
                Err(e) => {
                    last_err = e;
                    if attempt == 0 {
                        std::thread::sleep(Duration::from_millis(350));
                    }
                }
            }
        }
    }
    if !succeeded {
        let _ = std::fs::remove_file(&tmp);
        DOWNLOAD_DONE_BYTES.store(0, Ordering::Relaxed);
        DOWNLOAD_TOTAL_BYTES.store(0, Ordering::Relaxed);
        return Err(last_err);
    }
    Ok(format!("{} downloaded and equipped", item.name))
}

/// Match the display's physical resolution including parallax overscan, never
/// resample above the original or above 1080p for universal GPU/MediaCodec stability.
pub fn frame_size(source: [u32; 2], viewport: [u32; 2]) -> [u32; 2] {
    if source.contains(&0) || viewport.contains(&0) {
        return [source[0].min(1920) & !1, source[1].min(1080) & !1];
    }
    let scale = ((viewport[0] as f64 / source[0] as f64)
        .max(viewport[1] as f64 / source[1] as f64)
        * 1.30)
        .clamp(0.01, 1.0);
    let mut w = ((source[0] as f64 * scale).ceil() as u32).min(source[0]);
    let mut h = ((source[1] as f64 * scale).ceil() as u32).min(source[1]);
    if w > 1920 || h > 1080 {
        let cap = (1920.0 / w as f64).min(1080.0 / h as f64);
        w = ((w as f64 * cap).floor() as u32).max(2);
        h = ((h as f64 * cap).floor() as u32).max(2);
    }
    [(w & !1).max(2), (h & !1).max(2)]
}

/// Sample a bounded number of decoded pixels for a scene-matching glass tint.
pub fn sample_color(bytes: &[u8]) -> Option<[u8; 3]> {
    if bytes.len() < 4 || bytes.len() % 4 != 0 { return None; }
    let count = bytes.len() / 4;
    let step = (count / 1024).max(1);
    let mut sum = [0u64; 3];
    let mut n = 0u64;
    for pixel in bytes.chunks_exact(4).step_by(step) {
        for c in 0..3 { sum[c] += u64::from(pixel[c]); }
        n += 1;
    }
    Some([(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_dimensions_match_originals() {
        for item in &WALLPAPERS {
            assert!(matches!((item.width, item.height), (3840, 2160) | (1920, 1080)));
            assert_eq!(item.sha256.len(), 64);
        }
    }

    #[test]
    fn frame_size_preserves_display_detail_and_bounds() {
        let source = [3840, 2160];
        let small = frame_size(source, [1560, 702]);
        assert!(small[0] >= 1560 && small[1] >= 702);
        assert!(small[0] < source[0] && small[1] < source[1]);
        assert_eq!(frame_size(source, [3840, 2160]), source);
        assert_eq!(frame_size(source, [0, 0]), source);
    }

    #[test]
    fn sampled_color_tracks_video_pixels_and_rejects_bad_frames() {
        assert_eq!(sample_color(&[]), None);
        assert_eq!(sample_color(&[1, 2, 3]), None);
        assert_eq!(sample_color(&[100, 50, 200, 255, 200, 150, 0, 255]), Some([150, 100, 100]));
    }
}
