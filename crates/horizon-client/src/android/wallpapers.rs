//! Original-quality wallpaper catalog, preview thumbnails, theme extraction,
//! and bounded, hash-verified downloads.
//! The original H.264 files are decoded on Android without recompressing them.
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::ToSocketAddrs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

mod previews {
    include!(concat!(env!("OUT_DIR"), "/wallpaper_previews.rs"));
}

pub const PREVIEW_WIDTH: u32 = 112;
pub const PREVIEW_HEIGHT: u32 = 63;

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

pub const WALLPAPERS: [Wallpaper; 39] = [
    Wallpaper { name: "Aurora night cabin", filename: "aurora.mp4", size: 8_272_647, sha256: "1785dbd85c19788dacdec03f0bc2f82656b4e66e90a071e7fbc97fcc39479d5a", resolution: "4K", width: 3840, height: 2160 },
    Wallpaper { name: "Cherry sunrise valley", filename: "cherry-sunrise.mp4", size: 5_930_493, sha256: "e87d9e48adebd0f5624255a9aecb6c78285fa85971b61a617135b48992f88f7e", resolution: "4K", width: 3840, height: 2160 },
    Wallpaper { name: "Minecraft Earth", filename: "earth.mp4", size: 14_365_508, sha256: "4253f906602eccad70b74f7db4c3bfb63e5a2e8c8d6a3082a307aadd308f11ad", resolution: "4K", width: 3840, height: 2160 },
    Wallpaper { name: "Peaceful greenhouse", filename: "greenhouse.mp4", size: 11_862_687, sha256: "95e479d8e9428630a8245e6371bcf810ba15e65ae4340047a893572a7f6a0d15", resolution: "1080p", width: 1920, height: 1080 },
    Wallpaper { name: "Sakura blossom house", filename: "sakura-house.mp4", size: 9_270_425, sha256: "ed7f53d57cb874725b870a8f571d30fb20e1b9047a51fe2bce3b254293bd285e", resolution: "1080p", width: 1920, height: 1080 },
    Wallpaper { name: "Sunset Shader", filename: "sunset-shader.mp4", size: 466_716, sha256: "e9963a05ced0c8961461a4e3632eddae941fd90102c7d649e06740d342706370", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Sakura Forest", filename: "sakura-forest.mp4", size: 1_488_165, sha256: "ed914bc976e92e90474c67bc27283a0e49bcbbdaf2aea0b62693bbd05cb0e3df", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Rainy Evening", filename: "rainy-evening.mp4", size: 995_848, sha256: "3c92c9cfd62565cb55cc551f1e4818e361ac18f32f82d0a0973dd040f6427c47", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Nether Portal", filename: "nether-portal.mp4", size: 2_054_365, sha256: "cb7ff06966bc8d66e7ba2c7416b4101dd575211ce094673f88566f436f31d717", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Nature Horizon", filename: "minecraft-nature.mp4", size: 1_572_582, sha256: "f2f203dd72548a74716cc3043a1ec0f0ccbb72c6e5f1b5078b98b299f337f825", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Golden Sunset", filename: "minecraft-sunset.mp4", size: 1_317_697, sha256: "7232e0b6a7e5633854a6a5b3cf4d90b83d9f936dd65aaf10326e6046b5d513b3", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Snowy Campfire", filename: "snowy-campfire.mp4", size: 970_946, sha256: "dd368e30636c054f04bb381f178a2cffc44073f6388fab2613fc9fa864aa9ad9", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Relaxing Fireplace", filename: "relaxing-fireplace.mp4", size: 1_028_353, sha256: "9a8f8ee0df1a9c1c2884d1c95b85a622fe37084e42d702203d5d04faf47cc459", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Rainy Landscape", filename: "rainy-landscape.mp4", size: 1_553_421, sha256: "d7977007f6f29f7510451155de2180d1bdcf6bf2962455ca230327d5afda3c7b", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Herobrine Shrine", filename: "herobrine-shrine.mp4", size: 1_012_384, sha256: "7bd197f9b10a78cb23ffe743aa4143a7048402e116f4232407a3658901542d44", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Falling Snow", filename: "falling-snow.mp4", size: 942_731, sha256: "82d1cfdaf823ead9d88dc9d198d340af9004c13b0efb60f314ae616412fa0fed", resolution: "540p", width: 960, height: 540 },
    Wallpaper { name: "Aurora Night", filename: "aurora-night.mp4", size: 458_613, sha256: "01618404176ca2689266880d0f2510db5a6911f24db40eb93ed8fa7e646ac8ab", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Crystal Lake", filename: "crystal-lake.mp4", size: 2_421_501, sha256: "94c6669f79eae5e604889f7cfeb9ba94ec88c6a80bf74998de803c8bb113cbe9", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Bliss Meadow", filename: "bliss-xp.mp4", size: 1_847_291, sha256: "b28f4887c97bef3a75cde59f9714faeb70f6b788b9e7f1da4f961f7fdb01dc1a", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "BMW M5 Night", filename: "bmw-m5.mp4", size: 1_539_564, sha256: "7601d4d039e5a9f8551dd7fb324432e3835a91ac830bfd62d230d92ad49b6f6c", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Shader Vista I", filename: "minecraft-vista-1.mp4", size: 452_082, sha256: "1b268a53aafe2d6984cbdd6394e87ea4b17861d59e541010d3d9c04d589bfe9e", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Tranquil Morning Pond", filename: "tranquil-morning-pond.mp4", size: 1_656_322, sha256: "5dadab81d02d05ac567d31a27fb40770555a6ea592c1d2fbe1baa21dd05478e4", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Cozy Campfire Night", filename: "cozy-campfire-night.mp4", size: 1_011_825, sha256: "92ff0d2d3ea3344332054949e8e2844ea555306be3b47b525520d99837d04df8", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Cherry Blossom Garden", filename: "cherry-blossom-garden.mp4", size: 1_805_360, sha256: "a7928508e6b0f62082c5cb47ccac96ed5a45261014a11b5d44697722059a983b", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Overgrown Forest Train", filename: "overgrown-train.mp4", size: 1_512_752, sha256: "7b1c5c1ce4e654105050f9b73deb79cf8a3a41c03430acf8065f43304a25d20a", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Sakura Lake Retreat", filename: "sakura-lake-retreat.mp4", size: 1_834_920, sha256: "f98cb12d8a9a291cb4b5cede5d68f738cc1f88081a0f47a94866216475ca9b3c", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Hidden Valley", filename: "hidden-valley.mp4", size: 1_931_931, sha256: "94d78b1d00126626c4bbbf4ac7a24b5e87d2d622aca731a15308589b8d6ecce8", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Harbor Village", filename: "harbor-village.mp4", size: 1_457_233, sha256: "1e543f65f35f2fa6fcbff861418dd56e9749850cd59eed2048cc24c1dfe7f6c3", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Sakura Spring", filename: "sakura-spring.mp4", size: 2_151_200, sha256: "c01d56a69d495813945634bd2c0a2417744b7f87b262fde8c2c21ad8c5afa8d1", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Forest Sun Rays", filename: "forest-sun-rays.mp4", size: 2_044_853, sha256: "3fa569f0d68ca2d573fc4fa5fbdf9bd6ea10671ceafea874372b2e60163fe582", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Shader Vista II", filename: "minecraft-vista-2.mp4", size: 566_852, sha256: "d7997a4cd092ae6d465fe7d81eb94ada9bbc1ed5826c9ec6963d90e4e97cbcc7", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Porsche Horizon", filename: "porsche-minecraft.mp4", size: 1_080_059, sha256: "15d0d973298a9a27331c7e4e4f773c3da8adf869eb6473e9818d5f014fb95a57", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Shader Vista III", filename: "minecraft-vista-3.mp4", size: 979_829, sha256: "751a6574b2edf75a979c49239debd235e747d161aa611fe36ae853576795aff0", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Lantern Garden Night", filename: "lantern-garden-night.mp4", size: 595_406, sha256: "aaa2cd535aadc38324310492f41ede27c3a5f4e686259aa35fabcae42f6f7a10", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Shader Vista IV", filename: "minecraft-vista-4.mp4", size: 1_008_022, sha256: "6767d2594b6c0868afd6d7a61bfb80c95d1dd02871f734889c301a95d0feef72", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Axolotl Pond", filename: "axolotl-pond.mp4", size: 2_990_571, sha256: "763d938339e192605460cf30fc83745ed34a4dc9d73341550750cd9f88e9b02b", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Dungeons: The Sift", filename: "dungeons-the-sift.mp4", size: 2_888_712, sha256: "40081d2c434193ad71ae09561b041d96219cd758f5822a977337559700dd2630", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Purple Lamborghini", filename: "purple-lamborghini.mp4", size: 1_131_014, sha256: "751e1e818c6f464c4ea26da878dd8a35fe3f29b938ca9925aadad5107b3a6486", resolution: "720p", width: 1280, height: 720 },
    Wallpaper { name: "Enchanted Forest Rays", filename: "enchanted-forest-rays.mp4", size: 1_313_758, sha256: "90e7a40b28699a5f8745750a1aeacd047090d8ba6493395504911c255f37aed4", resolution: "720p", width: 1280, height: 720 },
];

/// The greenhouse ships with the app; `None` in older settings also means default.
pub const DEFAULT_INDEX: usize = 3;
pub fn effective_index(index: Option<usize>) -> usize {
    index.filter(|&i| i < WALLPAPERS.len()).unwrap_or(DEFAULT_INDEX)
}

pub fn preview_path(home: &Path, index: usize) -> Option<PathBuf> {
    let fname = WALLPAPERS.get(index)?.filename;
    let stem = fname.strip_suffix(".mp4").unwrap_or(fname);
    Some(home.join("wallpapers").join("previews").join(format!("{stem}.jpg")))
}

/// Returns `true` when the wallpaper at `index` is already available locally.
pub fn is_downloaded(home: &Path, index: usize) -> bool {
    let Some(item) = WALLPAPERS.get(index) else {
        return false;
    };
    if index == DEFAULT_INDEX {
        return true;
    }
    path(home, index)
        .and_then(|p| std::fs::metadata(p).ok())
        .is_some_and(|m| m.len() == item.size)
}

/// Loads (or generates/fetches off-thread) a crisp `112x63` RGBA preview thumbnail
/// plus its `(base, accent)` theme palette.
pub fn load_or_fetch_preview(
    home: &Path,
    index: usize,
) -> Option<(u32, u32, Vec<u8>, Option<([u8; 3], [u8; 3])>)> {
    let item = WALLPAPERS.get(index)?;
    let embedded = previews::embedded_preview(item.filename);
    let mut bytes: Option<Vec<u8>> = if !embedded.is_empty() {
        Some(embedded.to_vec())
    } else {
        None
    };

    let cache_path = preview_path(home, index);
    if bytes.is_none()
        && let Some(ref cp) = cache_path
        && cp.is_file()
    {
        bytes = std::fs::read(cp).ok().filter(|b| !b.is_empty());
    }

    if bytes.is_none()
        && let Some(video_path) = path(home, index)
        && let Some(ref cp) = cache_path
        && (index == DEFAULT_INDEX || std::fs::metadata(&video_path).is_ok_and(|m| m.len() == item.size))
    {
        if index == DEFAULT_INDEX && !video_path.is_file() {
            let _ = ensure_bundled_default(home);
        }
        if crate::native_ime::wallpaper_extract_thumbnail(&video_path, cp, PREVIEW_WIDTH, PREVIEW_HEIGHT) {
            bytes = std::fs::read(cp).ok().filter(|b| !b.is_empty());
        }
    }

    if bytes.is_none()
        && let Some(ref cp) = cache_path
    {
        let stem = item.filename.strip_suffix(".mp4").unwrap_or(item.filename);
        let urls = [
            format!(
                "https://raw.githubusercontent.com/entitybrian69-bit/volt-plus/arena/16c8aaea-volt-plus/wallpapers/previews/{stem}.jpg"
            ),
            format!(
                "https://raw.githubusercontent.com/entitybrian69-bit/volt-plus/android/wallpapers/previews/{stem}.jpg"
            ),
        ];
        let agent = ureq::AgentBuilder::new()
            .resolver(PreferIpv4Resolver)
            .user_agent("VoltPlus-Android/1.0")
            .timeout_connect(Duration::from_secs(6))
            .timeout_read(Duration::from_secs(10))
            .build();
        for url in &urls {
            if let Ok(resp) = agent.get(url).call() {
                let mut buf = Vec::new();
                if resp.into_reader().take(256 * 1024).read_to_end(&mut buf).is_ok()
                    && !buf.is_empty()
                    && image::load_from_memory(&buf).is_ok()
                {
                    if let Some(parent) = cp.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(cp, &buf);
                    bytes = Some(buf);
                    break;
                }
            }
        }
    }

    let raw_bytes = bytes?;
    let img = image::load_from_memory(&raw_bytes).ok()?;
    let resized = if img.width() == PREVIEW_WIDTH && img.height() == PREVIEW_HEIGHT {
        img.to_rgba8()
    } else {
        img.resize_to_fill(
            PREVIEW_WIDTH,
            PREVIEW_HEIGHT,
            image::imageops::FilterType::Triangle,
        )
        .to_rgba8()
    };
    let (w, h) = (resized.width(), resized.height());
    let rgba = resized.into_raw();
    let palette = sample_theme_colors(&rgba);
    Some((w, h, rgba, palette))
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
            "https://raw.githubusercontent.com/entitybrian69-bit/volt-plus/arena/16c8aaea-volt-plus/wallpapers/{}",
            item.filename
        ),
        format!(
            "https://raw.githubusercontent.com/entitybrian69-bit/volt-plus/android/wallpapers/{}",
            item.filename
        ),
        format!(
            "https://cdn.jsdelivr.net/gh/entitybrian69-bit/volt-plus@arena/16c8aaea-volt-plus/wallpapers/{}",
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

/// Keep 100% of every <=1080p wallpaper's native resolution so nothing is
/// downsampled, while capping 4K sources at 1080p for universal GPU/MediaCodec stability.
pub fn frame_size(source: [u32; 2], viewport: [u32; 2]) -> [u32; 2] {
    if source.contains(&0) {
        return [1280, 720];
    }
    if source[0] <= 1920 && source[1] <= 1080 {
        return [(source[0] & !1).max(2), (source[1] & !1).max(2)];
    }
    if viewport.contains(&0) {
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

/// Extract both `(scene_base, vibrant_accent)` from decoded RGBA pixels so the
/// launcher adapts richly to each wallpaper's true color mood instead of graying out.
pub fn sample_theme_colors(bytes: &[u8]) -> Option<([u8; 3], [u8; 3])> {
    if bytes.len() < 4 || bytes.len() % 4 != 0 {
        return None;
    }
    let count = bytes.len() / 4;
    let step = (count / 1024).max(1);
    let mut base_sum = [0u64; 3];
    let mut base_n = 0u64;
    let mut acc_sum = [0u64; 3];
    let mut acc_w = 0u64;

    for pixel in bytes.chunks_exact(4).step_by(step) {
        let (r, g, b) = (pixel[0], pixel[1], pixel[2]);
        base_sum[0] += u64::from(r);
        base_sum[1] += u64::from(g);
        base_sum[2] += u64::from(b);
        base_n += 1;

        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let chroma = u64::from(max_c - min_c);
        let luma = (u64::from(r) * 3 + u64::from(g) * 6 + u64::from(b)) / 10;
        // Favor saturated, mid-to-bright pixels so hero colors (sakura, aurora,
        // sunset, neon, embers) lead the UI accent without blowing out near black/white.
        let luma_ok = (24..=236).contains(&luma);
        let weight = if luma_ok && chroma >= 16 {
            1 + (chroma * chroma) / 32
        } else {
            1
        };
        acc_sum[0] += u64::from(r) * weight;
        acc_sum[1] += u64::from(g) * weight;
        acc_sum[2] += u64::from(b) * weight;
        acc_w += weight;
    }

    let base = [
        (base_sum[0] / base_n) as u8,
        (base_sum[1] / base_n) as u8,
        (base_sum[2] / base_n) as u8,
    ];
    let mut accent = [
        (acc_sum[0] / acc_w.max(1)) as u8,
        (acc_sum[1] / acc_w.max(1)) as u8,
        (acc_sum[2] / acc_w.max(1)) as u8,
    ];
    // Gently boost chroma around the pixel mean when the scene is colorful.
    let avg_ch = (i32::from(accent[0]) + i32::from(accent[1]) + i32::from(accent[2])) / 3;
    for ch in &mut accent {
        let boosted = avg_ch + ((i32::from(*ch) - avg_ch) * 125) / 100;
        *ch = boosted.clamp(0, 255) as u8;
    }
    Some((base, accent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_dimensions_match_originals() {
        let mut names = std::collections::HashSet::new();
        for item in &WALLPAPERS {
            assert!(matches!(
                (item.width, item.height),
                (3840, 2160) | (1920, 1080) | (1280, 720) | (960, 540)
            ));
            assert_eq!(item.sha256.len(), 64);
            assert!(names.insert(item.filename), "duplicate filename {}", item.filename);
        }
    }

    #[test]
    fn frame_size_preserves_display_detail_and_bounds() {
        let source_4k = [3840, 2160];
        let small = frame_size(source_4k, [1560, 702]);
        assert!(small[0] >= 1560 && small[1] >= 702);
        assert!(small[0] <= 1920 && small[1] <= 1080);
        assert_eq!(frame_size(source_4k, [3840, 2160]), [1920, 1080]);
        assert_eq!(frame_size(source_4k, [0, 0]), [1920, 1080]);

        // <= 1080p wallpapers always decode at 100% of their native resolution.
        assert_eq!(frame_size([1280, 720], [854, 480]), [1280, 720]);
        assert_eq!(frame_size([960, 540], [854, 480]), [960, 540]);
        assert_eq!(frame_size([1920, 1080], [1280, 720]), [1920, 1080]);
    }

    #[test]
    fn sampled_color_tracks_video_pixels_and_rejects_bad_frames() {
        assert_eq!(sample_color(&[]), None);
        assert_eq!(sample_color(&[1, 2, 3]), None);
        assert_eq!(sample_color(&[100, 50, 200, 255, 200, 150, 0, 255]), Some([150, 100, 100]));
    }

    #[test]
    fn sampled_theme_colors_prioritizes_vibrant_accent() {
        assert_eq!(sample_theme_colors(&[]), None);
        // 3 dark gray pixels + 1 vibrant sakura pink pixel -> accent stays strongly pink.
        let pixels = [
            20, 20, 22, 255,
            24, 24, 26, 255,
            22, 22, 24, 255,
            240, 110, 175, 255,
        ];
        let (base, accent) = sample_theme_colors(&pixels).unwrap();
        assert!(base[0] < 100);
        assert!(accent[0] > 200 && accent[2] > 140 && accent[0] > accent[1]);
    }
}
