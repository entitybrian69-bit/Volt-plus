//! Horizon's own shader packs in the game (docs/shaders.md): the pick in `shaderpacks/` (a
//! folder with `horizon-shaders.toml`, or a `.hshader`; `game/shader_packs.rs`) runs once it
//! passes `horizon_shader_api::check`. The pack is looked at twice a second: a saved file (or
//! saved option values) is checked and the pack rebuilt, and a
//! pack that fails is refused with its problems in the chat while the last good one keeps
//! drawing.

use std::{
    hash::{Hash, Hasher},
    path::Path,
    time::{Duration, Instant},
};

use horizon_client::text::Text;
use horizon_shader_api::{Pack, check};

use super::Game;

const LOOK_EVERY: Duration = Duration::from_millis(500);
const RED: u32 = 0xFF5555;

#[derive(Default)]
pub(super) struct NativeShaders {
    /// The pack's name in `shaderpacks/`, of either kind (`game/shader_packs.rs`); empty: none.
    pub picked: String,
    /// The pack's files when it was last read (`fingerprint`).
    stamp: Option<u64>,
    looked: Option<Instant>,
}

impl NativeShaders {
    pub fn picking(name: &str) -> Self {
        Self {
            picked: name.to_owned(),
            ..Self::default()
        }
    }
}

/// Whether `pack` actually references LabPBR block textures in its material/shared stages.
pub(super) fn pack_reads_labpbr(pack: &Pack) -> bool {
    if pack.manifest.material.is_none() {
        return false;
    }
    let stage_reads = |rel: &str| {
        pack.files
            .get(rel)
            .and_then(|b| std::str::from_utf8(b).ok())
            .is_some_and(|src| {
                src.contains("block_normal_map")
                    || src.contains("block_specular_map")
                    || src.contains("block_pbr_maps")
            })
    };
    pack.manifest.material.as_ref().is_some_and(|st| {
        stage_reads(&st.file) || st.files.iter().any(|f| stage_reads(f))
    }) || pack.manifest.shared.iter().any(|s| stage_reads(s))
}

/// Auto-repairs `.hshader` packs that reference shared helpers like `daylight()` or
/// `luminance()` without declaring/including `common.wgsl`, so previously downloaded
/// packs load cleanly without `unresolved value 'daylight'`.
fn repair_pack_if_needed(pack: &mut Pack) {
    let all_src: String = pack
        .files
        .iter()
        .filter(|(k, _)| k.ends_with(".wgsl"))
        .filter_map(|(_, v)| std::str::from_utf8(v).ok())
        .collect::<Vec<_>>()
        .join("\n");
    let needs_daylight = all_src.contains("daylight(") && !all_src.contains("fn daylight(");
    let needs_luminance = all_src.contains("luminance(") && !all_src.contains("fn luminance(");
    if !needs_daylight && !needs_luminance {
        return;
    }
    let mut extra = String::new();
    if needs_luminance {
        extra.push_str(
            "fn luminance(c: vec3<f32>) -> f32 {\n    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));\n}\n",
        );
    }
    if needs_daylight {
        extra.push_str(
            "fn daylight() -> f32 {\n    return select(0.35, smoothstep(-0.15, 0.15, frame.sun_direction.y), frame.dimension == 0);\n}\n",
        );
    }
    let shared_name = pack
        .manifest
        .shared
        .first()
        .cloned()
        .unwrap_or_else(|| "common.wgsl".to_owned());
    if !pack.manifest.shared.contains(&shared_name) {
        pack.manifest.shared.push(shared_name.clone());
    }
    let existing = pack
        .files
        .get(&shared_name)
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("")
        .to_owned();
    pack.files
        .insert(shared_name, format!("{extra}{existing}").into_bytes());
}

/// Builds a tuned in-memory `.hshader` fallback pack for Android when a desktop-only
/// OptiFine/Iris `.zip` shader uses unsupported GLSL/compute stages on mobile GPUs.
pub(super) fn android_fallback_native_shader(name: &str) -> Pack {
    use horizon_shader_api::manifest::MANIFEST;
    use std::collections::BTreeMap;

    let clean_id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned();
    let id = if clean_id.is_empty() {
        "mobile_shader".to_owned()
    } else {
        clean_id
    };
    let lower = name.to_lowercase();
    let is_night = lower.contains("galax")
        || lower.contains("nebula")
        || lower.contains("night")
        || lower.contains("void")
        || lower.contains("star");
    let is_warm = lower.contains("golden")
        || lower.contains("sildur")
        || lower.contains("bsl")
        || lower.contains("chocapic")
        || lower.contains("seus")
        || lower.contains("nostalgia");

    let (vibrance, water, sky_wgsl, material_wgsl) = if is_night {
        (
            1.22_f32,
            0.80_f32,
            r#"fn sky(input: SkyInput) -> vec4<f32> {
    let d = normalize(input.direction);
    let sun_dot = max(dot(d, frame.sun_direction), 0.0);
    let halo = pow(sun_dot, 10.0) * 0.45 * (1.0 - frame.rain * 0.8);
    let nebula = vec3<f32>(0.24, 0.36, 0.78) * (0.25 + 0.55 * smoothstep(-0.1, 0.9, d.y)) * (1.0 - daylight() * 0.75);
    let warm = vec3<f32>(1.0, 0.58, 0.35) * halo;
    let base = input.color.rgb * mix(vec3<f32>(0.92, 0.96, 1.05), vec3<f32>(0.68, 0.78, 1.12), smoothstep(0.0, 1.0, max(d.y, 0.0)) * 0.4);
    return vec4<f32>(clamp(base + warm + nebula, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
            r#"override water_clarity: f32 = 0.80;

fn material(input: MaterialInput) -> Surface {
    let ndotl = max(dot(input.normal, frame.light_direction), 0.0);
    let dir_shade = 0.76 + 0.38 * ndotl * input.light.y;
    let torch = vec3<f32>(1.22, 0.80, 0.44) * pow(input.light.x, 1.55) * 0.48;
    var col = input.albedo.rgb * input.tint.rgb * (input.lightmap * dir_shade + torch + input.colored_light * 0.6);
    if input.fluid == 1u {
        let view_dir = normalize(-input.position);
        let fresnel = clamp(pow(1.0 - max(dot(input.normal, view_dir), 0.0), 3.4), 0.10, 0.90);
        let sky_ref = mix(vec3<f32>(0.10, 0.24, 0.52), vec3<f32>(0.26, 0.54, 0.92), daylight());
        col = mix(col, sky_ref, fresnel * water_clarity);
    }
    return Surface(vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), input.albedo.a * input.tint.a));
}
"#,
        )
    } else if is_warm {
        (
            1.20,
            0.78,
            r#"fn sky(input: SkyInput) -> vec4<f32> {
    let d = normalize(input.direction);
    let sun_dot = max(dot(d, frame.sun_direction), 0.0);
    let near_horizon = 1.0 - smoothstep(-0.05, 0.55, d.y);
    let day = max(daylight(), 0.25) * (1.0 - frame.rain * 0.65);
    let corona = pow(sun_dot, 6.0) * 0.55 + near_horizon * pow(sun_dot, 2.0) * 0.35;
    let warm = vec3<f32>(1.0, 0.62, 0.26) * corona * day;
    let base = mix(input.color.rgb * 1.04, input.color.rgb * vec3<f32>(0.84, 0.90, 1.06), smoothstep(0.1, 0.95, max(d.y, 0.0)) * 0.36);
    return vec4<f32>(clamp(base + warm, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
            r#"override water_clarity: f32 = 0.78;

fn material(input: MaterialInput) -> Surface {
    let ndotl = max(dot(input.normal, frame.light_direction), 0.0);
    let dir_shade = 0.75 + 0.40 * ndotl * input.light.y;
    let torch = vec3<f32>(1.24, 0.76, 0.36) * pow(input.light.x, 1.55) * 0.50;
    var col = input.albedo.rgb * input.tint.rgb * (input.lightmap * dir_shade + torch + input.colored_light * 0.58);
    if input.fluid == 1u {
        let view_dir = normalize(-input.position);
        let fresnel = clamp(pow(1.0 - max(dot(input.normal, view_dir), 0.0), 3.4), 0.10, 0.88);
        let sky_ref = mix(vec3<f32>(0.12, 0.36, 0.60), vec3<f32>(0.36, 0.62, 0.92), daylight());
        col = mix(col, sky_ref, fresnel * water_clarity);
    }
    return Surface(vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), input.albedo.a * input.tint.a));
}
"#,
        )
    } else {
        (
            1.18,
            0.76,
            r#"fn sky(input: SkyInput) -> vec4<f32> {
    let d = normalize(input.direction);
    let sun_dot = max(dot(d, frame.sun_direction), 0.0);
    let halo = pow(sun_dot, 8.0) * 0.48 * (1.0 - frame.rain * 0.75) * max(daylight(), 0.3);
    let warm = vec3<f32>(1.0, 0.66, 0.34) * halo;
    let zenith = smoothstep(0.05, 0.92, max(d.y, 0.0));
    let base = mix(input.color.rgb * 1.05, input.color.rgb * vec3<f32>(0.80, 0.90, 1.08), zenith * 0.36);
    return vec4<f32>(clamp(base + warm, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
            r#"override water_clarity: f32 = 0.76;

fn material(input: MaterialInput) -> Surface {
    let ndotl = max(dot(input.normal, frame.light_direction), 0.0);
    let dir_shade = 0.77 + 0.38 * ndotl * input.light.y;
    let torch = vec3<f32>(1.20, 0.78, 0.40) * pow(input.light.x, 1.55) * 0.46;
    var col = input.albedo.rgb * input.tint.rgb * (input.lightmap * dir_shade + torch + input.colored_light * 0.56);
    if input.fluid == 1u {
        let view_dir = normalize(-input.position);
        let fresnel = clamp(pow(1.0 - max(dot(input.normal, view_dir), 0.0), 3.4), 0.08, 0.88);
        let sky_ref = mix(vec3<f32>(0.10, 0.34, 0.58), vec3<f32>(0.30, 0.58, 0.90), daylight());
        col = mix(col, sky_ref, fresnel * water_clarity);
    }
    return Surface(vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), input.albedo.a * input.tint.a));
}
"#,
        )
    };
    let post_wgsl = format!(
        r#"override vibrance: f32 = {vibrance:.2};

fn post(input: PostInput) -> vec4<f32> {{
    let c = (input.color.rgb - 0.48) * 1.08 + 0.48;
    let lum = luminance(c);
    let sat = mix(vec3<f32>(lum), c, vibrance);
    return vec4<f32>(clamp(sat, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}}
"#
    );
    let title_lit = serde_json::to_string(name).unwrap_or_else(|_| "\"Shader Pack\"".to_owned());
    let manifest = format!(
        r#"schema = 1
id = "{id}"
name = {title_lit}
version = "1.0.0"
api = "0.1"
description = "Horizon mobile shader compatibility pipeline."
authors = ["Horizon"]
license = "MIT"
shared = ["common.wgsl"]

[sky]
file = "sky.wgsl"

[material]
file = "material.wgsl"

[[post]]
file = "post.wgsl"

[[options]]
id = "vibrance"
name = "Color Vibrance"
type = "float"
default = {vibrance:.2}
min = 0.6
max = 1.8
step = 0.02

[[options]]
id = "water_clarity"
name = "Water Reflection Strength"
type = "float"
default = {water:.2}
min = 0.1
max = 1.0
step = 0.05

[budget]
gpu_ms = 0.5
"#
    );
    let common_wgsl = r#"fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}
fn daylight() -> f32 {
    return select(0.35, smoothstep(-0.15, 0.15, frame.sun_direction.y), frame.dimension == 0);
}
"#;
    let mut files = BTreeMap::new();
    files.insert(MANIFEST.to_owned(), manifest.into_bytes());
    files.insert("common.wgsl".to_owned(), common_wgsl.as_bytes().to_vec());
    files.insert("sky.wgsl".to_owned(), sky_wgsl.as_bytes().to_vec());
    files.insert("material.wgsl".to_owned(), material_wgsl.as_bytes().to_vec());
    files.insert("post.wgsl".to_owned(), post_wgsl.into_bytes());
    Pack::from_files(files).expect("fallback native shader manifest is valid")
}

impl Game<'_> {
    pub(super) fn apply_android_shader_fallback(&mut self, name: &str) -> bool {
        let pack = android_fallback_native_shader(name);
        let Ok(report) = check(&pack) else {
            return false;
        };
        let Some(renderer) = &mut self.renderer else {
            return false;
        };
        let options = &pack.manifest.options;
        let constants = options
            .iter()
            .filter_map(|o| {
                let v = match &o.kind {
                    horizon_shader_api::OptionKind::Float { default, .. } => *default,
                    horizon_shader_api::OptionKind::Int { default, .. } => *default as f32,
                    horizon_shader_api::OptionKind::Bool { default } => f32::from(*default),
                };
                Some((o.id.clone(), v))
            })
            .collect();
        let sprites = || {
            let layers = self.pack.render.sprite_layers.iter();
            layers.map(|(n, layer)| (n.as_str(), *layer))
        };
        let tags = pack.manifest.sprite_tags(sprites());
        let materials = pack.manifest.sprite_materials(sprites());
        let lights = horizon_client::gfx::block_light::LightTable::default();
        renderer
            .set_native_shader_pack(Some((report, constants, tags, materials, lights)))
            .is_ok()
    }
    /// Builds the pack at `path` when its files changed since it was last read.
    pub(super) fn tick_native_shader_pack(&mut self, path: &Path) {
        let now = Instant::now();
        let state = &mut self.native_shaders;
        if state.looked.is_some_and(|t| now - t < LOOK_EVERY) {
            return;
        }
        state.looked = Some(now);
        let stamp = fingerprint(path);
        if stamp == state.stamp {
            return;
        }
        let reload = state.stamp.is_some();
        state.stamp = stamp;
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let result = Pack::read(path).map_err(|e| vec![e]).and_then(|mut pack| {
            repair_pack_if_needed(&mut pack);
            let report = check(&pack)
                .map_err(|p| p.iter().map(ToString::to_string).collect::<Vec<String>>())?;
            // The player's values (the pack's Shader Options page), else the defaults.
            let options = &pack.manifest.options;
            let values = horizon_client::shader_pack_list::load_values(path, options);
            let constants = options.iter().map(|o| o.id.clone()).zip(values).collect();
            let sprites = || {
                let layers = self.pack.render.sprite_layers.iter();
                layers.map(|(name, layer)| (name.as_str(), *layer))
            };
            let tags = pack.manifest.sprite_tags(sprites());
            let materials = pack.manifest.sprite_materials(sprites());
            // Each light-giving block state's `[[lights]]` colour, with its level.
            let mut lights = horizon_client::gfx::block_light::LightTable::default();
            if !pack.manifest.lights.is_empty() {
                for (id, state) in self.res.facts().states.iter().enumerate() {
                    if state.emission == 0 {
                        continue;
                    }
                    if let Some(color) = pack.manifest.light_of(&state.name) {
                        lights.insert(id as u32, (color, state.emission));
                    }
                }
            }
            let labels = renderer
                .set_native_shader_pack(Some((report, constants, tags, materials, lights)))
                .map_err(|e| vec![e])?;
            let needs_labpbr = pack_reads_labpbr(&pack);
            Ok((pack.manifest, labels, needs_labpbr))
        });
        // The resources were loaded without a native pack, so without the resource packs'
        // LabPBR maps its material stage reads: load them once, when the pack actually uses them.
        if result.as_ref().is_ok_and(|(_, _, needs)| *needs)
            && !horizon_client::sprites::labpbr_maps()
        {
            horizon_client::sprites::set_labpbr_maps(true);
            log::info!("Horizon: reloading resources for the LabPBR maps a native pack reads");
            self.reload_packs();
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match result {
            Ok((m, labels, _)) => {
                log::info!(
                    "shader pack {} {}: {} in {} ms",
                    m.id,
                    m.version,
                    labels.join(", "),
                    now.elapsed().as_millis()
                );
                if reload {
                    self.system_message(Text::literal(format!("Shader pack {} reloaded", m.name)));
                }
            }
            Err(problems) => {
                log::warn!("shader pack {name} refused: {}", problems.join("; "));
                self.system_message(Text::colored(format!("Shader pack {name} refused:"), RED));
                for p in problems {
                    self.system_message(Text::colored(p, RED));
                }
            }
        }
    }
}

/// A hash of the names, sizes and modification times of the pack's files and of its saved
/// option values (`None` when the pack is unreadable).
fn fingerprint(path: &Path) -> Option<u64> {
    fn walk(path: &Path, hasher: &mut std::hash::DefaultHasher) -> Option<()> {
        let meta = std::fs::metadata(path).ok()?;
        path.hash(hasher);
        if meta.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(path)
                .ok()?
                .flatten()
                .map(|e| e.path())
                .collect();
            entries.sort();
            for entry in entries {
                walk(&entry, hasher)?;
            }
        } else {
            meta.len().hash(hasher);
            meta.modified().ok()?.hash(hasher);
        }
        Some(())
    }
    let mut hasher = std::hash::DefaultHasher::new();
    walk(path, &mut hasher)?;
    // The player's option values, when there are any.
    let values = horizon_client::shader_pack_list::values_path(path);
    let _ = walk(&values, &mut hasher);
    Some(hasher.finish())
}
