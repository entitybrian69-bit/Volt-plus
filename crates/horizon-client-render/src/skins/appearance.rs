//! Frontend-independent appearance handoff, schema 1. Read each game launch.
use std::{
    path::Path,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

pub const APPEARANCE_CHANNEL: &str = "horizon:appearance";
pub const MAX_APPEARANCE_PNG_BYTES: usize = 4 * 1024 * 1024;
const MAX_NET_PAYLOAD_BYTES: usize = 30_000;
const NET_MAGIC: [u8; 4] = *b"HZAP";

static WAVEY_CAPES_FLAG: AtomicBool = AtomicBool::new(true);

pub fn wavey_capes_enabled() -> bool {
    WAVEY_CAPES_FLAG.load(Ordering::Relaxed)
}

pub fn set_wavey_capes(enabled: bool) {
    WAVEY_CAPES_FLAG.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Default)]
pub struct Appearance {
    pub skin: Option<(Vec<u8>, bool)>,
    pub cape: Option<Vec<u8>>,
    pub cape_frames: Vec<Vec<u8>>,
    pub cape_fps: u8,
    pub hide_cape: bool,
    pub wavey_capes: bool,
    pub skin_png: Option<Vec<u8>>,
    pub cape_png: Option<Vec<u8>>,
    pub external_skin_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NetAppearance {
    pub uuid: u128,
    pub slim: bool,
    pub hide_cape: bool,
    pub has_skin: bool,
    pub has_cape: bool,
    pub cape_fps: u8,
    pub skin: Option<(Vec<u8>, bool)>,
    pub cape_frames: Vec<Vec<u8>>,
}

impl NetAppearance {
    pub fn merge(&mut self, incoming: Self) {
        if incoming.has_skin {
            self.slim = incoming.slim;
            if incoming.skin.is_some() {
                self.skin = incoming.skin;
                self.has_skin = true;
            }
        }
        if incoming.hide_cape {
            self.hide_cape = true;
            self.has_cape = false;
            self.cape_frames.clear();
        } else if incoming.has_cape {
            self.hide_cape = false;
            self.cape_fps = incoming.cape_fps;
            if !incoming.cape_frames.is_empty() {
                self.cape_frames = incoming.cape_frames;
                self.has_cape = true;
            }
        }
    }
}

static CURRENT: RwLock<Option<Arc<Appearance>>> = RwLock::new(None);
pub fn current() -> Option<Arc<Appearance>> {
    CURRENT.read().ok()?.clone()
}

fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Best);
    encoder.write_header().ok()?.write_image_data(rgba).ok()?;
    Some(bytes)
}

fn encode_cape_frames_png(frames: &[Vec<u8>]) -> Option<Vec<u8>> {
    if frames.is_empty() {
        return None;
    }
    let mut max_frames = frames.len().min(32);
    loop {
        let mut stacked = Vec::with_capacity(64 * 32 * 4 * max_frames);
        for frame in frames.iter().take(max_frames) {
            if frame.len() < 64 * 32 * 4 {
                return None;
            }
            stacked.extend_from_slice(&frame[..64 * 32 * 4]);
        }
        if let Some(bytes) = encode_rgba_png(64, (32 * max_frames) as u32, &stacked) {
            if bytes.len() <= 22_000 || max_frames <= 1 {
                return Some(bytes);
            }
        } else {
            return None;
        }
        max_frames = (max_frames / 2).max(1);
    }
}

/// Encodes `horizon:appearance` into one combined payload or splits skin & cape across
/// two payloads when combined bytes would exceed `MAX_NET_PAYLOAD_BYTES`, so neither skin
/// nor cape is ever dropped.
pub fn encode_net_payloads(
    uuid: u128,
    slim: bool,
    hide_cape: bool,
    cape_fps: u8,
    skin_png: Option<&[u8]>,
    cape_png: Option<&[u8]>,
) -> Vec<Vec<u8>> {
    let skin_slice = skin_png.filter(|s| !s.is_empty() && s.len() <= MAX_NET_PAYLOAD_BYTES - 34);
    let cape_slice = if hide_cape {
        None
    } else {
        cape_png.filter(|c| !c.is_empty() && c.len() <= MAX_NET_PAYLOAD_BYTES - 34)
    };
    let skin_len = skin_slice.map_or(0, |s| s.len());
    let cape_len = cape_slice.map_or(0, |c| c.len());
    if 31 + skin_len + cape_len <= MAX_NET_PAYLOAD_BYTES {
        if let Some(one) =
            encode_net_payload(uuid, slim, hide_cape, cape_fps, skin_slice, cape_slice)
        {
            return vec![one];
        }
    }
    let mut packets = Vec::with_capacity(2);
    if let Some(s) = skin_slice
        && let Some(pkt) = encode_net_payload(uuid, slim, false, cape_fps, Some(s), None)
    {
        packets.push(pkt);
    }
    if (hide_cape || cape_slice.is_some())
        && let Some(pkt) = encode_net_payload(uuid, slim, hide_cape, cape_fps, None, cape_slice)
    {
        packets.push(pkt);
    }
    packets
}

/// Encodes a compact `horizon:appearance` payload bounded below `ServerboundCustomPayloadPacket::MAX_PAYLOAD_SIZE`.
pub fn encode_net_payload(
    uuid: u128,
    slim: bool,
    hide_cape: bool,
    cape_fps: u8,
    skin_png: Option<&[u8]>,
    cape_png: Option<&[u8]>,
) -> Option<Vec<u8>> {
    let skin_slice = skin_png.filter(|s| !s.is_empty() && s.len() <= MAX_NET_PAYLOAD_BYTES - 34);
    let mut cape_slice = if hide_cape {
        None
    } else {
        cape_png.filter(|c| !c.is_empty() && c.len() <= MAX_NET_PAYLOAD_BYTES - 34)
    };
    let skin_len = skin_slice.map_or(0, |s| s.len());
    if let Some(c) = cape_slice
        && 31 + skin_len + c.len() > MAX_NET_PAYLOAD_BYTES
    {
        cape_slice = None;
    }
    let flags = u8::from(slim)
        | (u8::from(hide_cape) << 1)
        | (u8::from(skin_slice.is_some()) << 2)
        | (u8::from(cape_slice.is_some()) << 3);
    let mut out = Vec::with_capacity(
        30 + skin_len + cape_slice.map_or(0, |c| c.len()),
    );
    out.extend_from_slice(&NET_MAGIC);
    out.push(1);
    out.push(flags);
    out.push(cape_fps.clamp(1, 30));
    out.extend_from_slice(&uuid.to_be_bytes());
    let s_bytes = skin_slice.unwrap_or(&[]);
    let c_bytes = cape_slice.unwrap_or(&[]);
    out.extend_from_slice(&(s_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(s_bytes);
    out.extend_from_slice(&(c_bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(c_bytes);
    if out.len() <= MAX_NET_PAYLOAD_BYTES {
        Some(out)
    } else {
        None
    }
}

/// Returns a copy of `payload` with `uuid` stamped into the header (server broadcast).
pub fn restamp_net_uuid(payload: &[u8], uuid: u128) -> Option<Vec<u8>> {
    if payload.len() < 31 || payload[..4] != NET_MAGIC || payload[4] != 1 {
        return None;
    }
    let mut out = payload.to_vec();
    out[7..23].copy_from_slice(&uuid.to_be_bytes());
    Some(out)
}

/// Decodes and validates a `horizon:appearance` payload.
pub fn decode_net_payload(payload: &[u8]) -> Option<NetAppearance> {
    if payload.len() > MAX_NET_PAYLOAD_BYTES
        || payload.len() < 31
        || payload[..4] != NET_MAGIC
        || payload[4] != 1
    {
        return None;
    }
    let flags = payload[5];
    let slim = flags & 1 != 0;
    let hide_cape = flags & 2 != 0;
    let has_skin = flags & 4 != 0;
    let has_cape = flags & 8 != 0;
    let cape_fps = payload[6].clamp(1, 30);
    let uuid = u128::from_be_bytes(payload[7..23].try_into().ok()?);
    let mut pos = 23usize;
    let skin_len = u32::from_be_bytes(payload.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos += 4;
    if pos + skin_len + 4 > payload.len() {
        return None;
    }
    let skin_bytes = &payload[pos..pos + skin_len];
    pos += skin_len;
    let cape_len = u32::from_be_bytes(payload.get(pos..pos + 4)?.try_into().ok()?) as usize;
    pos += 4;
    if pos + cape_len != payload.len() {
        return None;
    }
    let cape_bytes = &payload[pos..pos + cape_len];

    let skin = if has_skin && !skin_bytes.is_empty() {
        let reader = png::Decoder::new(std::io::Cursor::new(skin_bytes))
            .read_info()
            .ok()?;
        let (w, h) = (reader.info().width, reader.info().height);
        if w != 64 || (h != 32 && h != 64) {
            return None;
        }
        let (dw, dh, rgba) = crate::hud::decode_bytes(skin_bytes).ok()?;
        super::process_legacy_skin(dw, dh, &rgba).map(|s| (s, slim))
    } else {
        None
    };

    let cape_frames = if !hide_cape && has_cape && !cape_bytes.is_empty() {
        let reader = png::Decoder::new(std::io::Cursor::new(cape_bytes))
            .read_info()
            .ok()?;
        let (w, h) = (reader.info().width, reader.info().height);
        if !super::is_valid_cape_dimensions(w, h) {
            return None;
        }
        let (dw, dh, rgba) = crate::hud::decode_bytes(cape_bytes).ok()?;
        super::process_cape_frames(dw, dh, &rgba).unwrap_or_default()
    } else {
        Vec::new()
    };

    Some(NetAppearance {
        uuid,
        slim,
        hide_cape,
        has_skin,
        has_cape,
        cape_fps,
        skin,
        cape_frames,
    })
}

pub fn configure(home: &Path) -> Result<(), String> {
    *CURRENT
        .write()
        .map_err(|_| "Appearance state unavailable")? = None;
    use std::io::Read;
    let bytes = match std::fs::File::open(home.join("horizon-appearance.json")).and_then(|f| {
        let mut bytes = Vec::new();
        f.take(65537).read_to_end(&mut bytes)?;
        Ok(bytes)
    }) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("Could not read the appearance handoff".into()),
    };
    if bytes.len() > 65536 {
        return Err("Appearance handoff is too large".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid appearance JSON")?;
    if value["schema"] != 1 {
        return Err("Unsupported appearance schema".into());
    }
    let read = |key: &str| -> Result<Option<(u32, u32, Vec<u8>)>, String> {
        let Some(path) = value[key].as_str() else {
            return Ok(None);
        };
        let path = Path::new(path);
        if !path.is_absolute() {
            return Err("Appearance PNG paths must be absolute".into());
        }
        let max_bytes = if key == "skin" {
            1024 * 1024
        } else {
            MAX_APPEARANCE_PNG_BYTES
        };
        use std::io::Read;
        let mut data = Vec::new();
        std::fs::File::open(path)
            .and_then(|f| f.take(max_bytes as u64 + 1).read_to_end(&mut data))
            .map_err(|_| "Could not read appearance PNG")?;
        if data.len() > max_bytes {
            return Err("Appearance PNG exceeds size limit".into());
        }
        let reader = png::Decoder::new(std::io::Cursor::new(&data))
            .read_info()
            .map_err(|_| "Invalid appearance PNG")?;
        let (w, h) = (reader.info().width, reader.info().height);
        let valid = if key == "skin" {
            w == 64 && (h == 32 || h == 64)
        } else {
            super::is_valid_cape_dimensions(w, h)
        };
        if !valid {
            return Err("Invalid appearance PNG dimensions".into());
        }
        crate::hud::decode_bytes(&data)
            .map(Some)
            .map_err(|_| "Could not decode appearance PNG".into())
    };
    let slim = value["slim"].as_bool().unwrap_or(false);
    let skin = read("skin")?
        .and_then(|(w, h, rgba)| super::process_legacy_skin(w, h, &rgba))
        .map(|rgba| (rgba, slim));
    let cape_frames = read("cape")?
        .and_then(|(w, h, rgba)| super::process_cape_frames(w, h, &rgba))
        .unwrap_or_default();
    let cape = cape_frames.first().cloned();
    let cape_fps = value["cape_fps"]
        .as_u64()
        .map(|n| n.clamp(1, 30) as u8)
        .unwrap_or(10);
    let hide_cape = value["hide_cape"].as_bool().unwrap_or(false);
    let wavey_capes = value["wavey_capes"].as_bool().unwrap_or(true);
    set_wavey_capes(wavey_capes);
    let skin_png = skin
        .as_ref()
        .and_then(|(rgba, _)| encode_rgba_png(64, 64, rgba));
    let cape_png = if hide_cape {
        None
    } else {
        encode_cape_frames_png(&cape_frames)
    };
    let external_skin_name = value["external_skin_name"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    *CURRENT
        .write()
        .map_err(|_| "Appearance state unavailable")? = Some(Arc::new(Appearance {
        skin,
        cape,
        cape_frames,
        cape_fps,
        hide_cape,
        wavey_capes,
        skin_png,
        cape_png,
        external_skin_name,
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handoff_loads_pixels_and_clears_state_between_games_and_on_errors() {
        let root = std::env::temp_dir().join(format!(
            "horizon-appearance-contract-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let image = root.join("skin.png");
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 64, 64);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&vec![127; 64 * 64 * 4])
                .unwrap();
        }
        std::fs::write(&image, bytes).unwrap();
        let handoff = root.join("horizon-appearance.json");
        std::fs::write(
            &handoff,
            serde_json::json!({"schema":1,"skin":image,"slim":true,"hide_cape":true}).to_string(),
        )
        .unwrap();
        configure(&root).unwrap();
        let appearance = current().unwrap();
        assert_eq!(appearance.skin.as_ref().unwrap().0.len(), 64 * 64 * 4);
        assert!(appearance.skin.as_ref().unwrap().1 && appearance.hide_cape);
        std::fs::remove_file(&handoff).unwrap();
        configure(&root).unwrap();
        assert!(current().is_none());
        for bad in [
            serde_json::json!({"schema":2}),
            serde_json::json!({"schema":1,"skin":"relative.png"}),
        ] {
            std::fs::write(&handoff, bad.to_string()).unwrap();
            assert!(configure(&root).is_err());
            assert!(current().is_none());
        }
        std::fs::write(&handoff, vec![b' '; 65537]).unwrap();
        assert!(configure(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
