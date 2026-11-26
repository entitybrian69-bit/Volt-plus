//! Explicit classic shader downloads; native .hshader files keep the registry path.
use crate::browser::{self, Kind, Project, Source, SourceId, Version, installed};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    #[default]
    Horizon,
    Modrinth,
}
impl Origin {
    pub fn source(self) -> SourceId {
        match self {
            Self::Horizon => SourceId::Horizon,
            Self::Modrinth => SourceId::Modrinth,
        }
    }
}

fn version(versions: &[Version]) -> Option<&Version> {
    let usable = |v: &&Version| {
        v.blocked.is_none()
            && v.file()
                .is_some_and(|f| f.name.to_ascii_lowercase().ends_with(".zip"))
    };
    versions
        .iter()
        .filter(usable)
        .find(|v| v.game_versions.iter().any(|g| g == browser::GAME))
        .or_else(|| versions.iter().find(usable))
}

pub fn install(
    source: &dyn Source,
    project: &Project,
    home: &Path,
    progress: &dyn Fn(&str, f32),
) -> Result<usize, String> {
    if project.kind != Kind::Shader
        || project.source != SourceId::Modrinth
        || source.id() != project.source
    {
        return Err("Expected a Modrinth shader pack".into());
    }
    let versions = source.versions(project)?;
    let v = version(&versions).ok_or("No downloadable shader ZIP is available")?;
    if !v.required.is_empty() {
        return Err(
            "This shader requires additional projects which Horizon cannot install automatically"
                .into(),
        );
    }
    let file = v.file().ok_or("Shader file is missing")?;
    if file.size > horizon_client::content_files::MAX_INPUT {
        return Err("Shader ZIP exceeds 256 MiB".into());
    }
    let name = browser::safe_name(&file.name).ok_or("Unsafe shader file name")?;
    let folder = home.join("shaderpacks");
    if fs::symlink_metadata(&folder).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Shader folder cannot be a symbolic link".into());
    }
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let relative = format!("shaderpacks/{name}");
    let target = home.join(&relative);
    let mut list = installed::Installed::load(home);
    let ours = |e: &installed::Entry| e.source == project.source && e.project == project.id;
    let old = list.entries.iter().find(|e| ours(e)).cloned();
    let replacing = target.exists();
    if let Ok(meta) = fs::symlink_metadata(&target) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("Cannot replace a folder or symbolic link".into());
        }
        if !list.entries.iter().any(|e| ours(e) && e.file == relative)
            || list.entries.iter().any(|e| !ours(e) && e.file == relative)
        {
            return Err("This shader file already exists. Remove it in Installed or rename it in Files first".into());
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let stage = home
        .join("mod-browser/downloads")
        .join(format!("shader-{stamp}"));
    let result = (|| {
        let path = source.fetch(file, &stage, &|f| progress(&project.title, f))?;
        let native = horizon_client::content_files::validate(
            &path,
            horizon_client::content_files::Kind::Shaders,
            name,
            || false,
        )
        .map_err(|e| e.to_string())?;
        if native {
            return Err("This ZIP is a native Horizon shader pack. Use Import file to install it as .hshader".into());
        }
        let (algo, hash) = file.checksum().ok_or("The source supplies no checksum")?;
        if replacing {
            fs::rename(&path, &target).map_err(|e| e.to_string())?;
        } else {
            // Do not replace a file created during the download by an importer/Files.
            // Android's app-data SELinux policy forbids hard links. Reserve the
            // destination exclusively instead; a failed copy removes only ours.
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| e.to_string())?;
            let copied = (|| -> std::io::Result<()> {
                std::io::copy(&mut fs::File::open(&path)?, &mut output)?;
                output.sync_all()
            })();
            if let Err(error) = copied {
                drop(output);
                let _ = fs::remove_file(&target);
                return Err(error.to_string());
            }
        }
        list.entries.retain(|e| !ours(e));
        list.entries.push(installed::Entry {
            source: project.source,
            project: project.id.clone(),
            slug: project.slug.clone(),
            title: project.title.clone(),
            kind: Kind::Shader,
            version: v.id.clone(),
            number: v.number.clone(),
            file: relative,
            checksum: format!("{algo}:{hash}"),
            page: project.page.clone(),
            dependency: false,
            verdict: Some("Experimental shader compatibility".into()),
            skipped: vec![],
            namespaces: vec![],
        });
        list.save(home)?;
        if let Some(old) = old.filter(|e| e.file != format!("shaderpacks/{name}"))
            && let Some(previous) = old.file.strip_prefix("shaderpacks/")
            && browser::safe_name(previous).is_some()
            && !list.entries.iter().any(|e| e.file == old.file)
        {
            let path = folder.join(previous);
            if fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
            {
                let _ = fs::remove_file(path);
            }
        }
        let options_path = home.join("options.txt");
        let mut settings = horizon_client::settings::Settings::load(&options_path);
        settings.horizon.shader_pack = name.to_owned();
        settings.save(&options_path);
        Ok(1)
    })();
    let _ = fs::remove_dir_all(stage);
    result
}


/// Returns all built-in Horizon Native Shader Packs (`.hshader`) matching `query_text`.
pub fn horizon_builtin_shader_projects(query_text: &str) -> Vec<Project> {
    let words: Vec<String> = query_text
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();
    horizon_shader_api::all_horizon_shader_packs()
        .into_iter()
        .filter(|(id, title, summary, _)| {
            let hay = format!("{id} {title} {summary} horizon shader").to_lowercase();
            words.iter().all(|w| hay.contains(w))
        })
        .map(|(id, title, summary, downloads)| Project {
            source: SourceId::Horizon,
            id: id.to_owned(),
            slug: id.to_owned(),
            title: title.to_owned(),
            summary: summary.to_owned(),
            author: "Horizon".into(),
            downloads,
            page: format!("https://playhorizon.dev/mods/{id}"),
            icon: Some(format!("https://horizon-rust.github.io/horizon-mods/icons/{id}.png")),
            license: "MIT".into(),
            kind: Kind::Shader,
            game_versions: vec![browser::GAME.into()],
            loaders: vec!["horizon".into(), "hshader".into()],
        })
        .collect()
}

/// Installs a Horizon Native Shader Pack (`.hshader`) into `<home>/shaderpacks/<id>.hshader`,
/// validates it with `horizon_shader_api::check`, records it in `mod-browser/installed.json`,
/// and activates it in `<home>/options.txt`.
pub fn install_horizon_builtin_shader(
    project: &Project,
    home: &Path,
    progress: &dyn Fn(&str, f32),
) -> Result<usize, String> {
    progress(&format!("Building {} (.hshader)...", project.title), 0.25);
    let pack = horizon_shader_api::builtin_shader_pack(&project.id)
        .ok_or_else(|| format!("Unknown Horizon shader pack: {}", project.id))?;
    horizon_shader_api::check(&pack)
        .map_err(|errs| errs.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))?;
    let bytes = pack.package()?;
    let sha = horizon_shader_api::package::sha256_hex(&bytes);
    progress(&format!("Installing {} into shaderpacks/...", project.title), 0.75);

    let folder = home.join("shaderpacks");
    if fs::symlink_metadata(&folder).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Shader folder cannot be a symbolic link".into());
    }
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let file_name = format!("{}.hshader", project.id);
    let relative = format!("shaderpacks/{file_name}");
    let target = home.join(&relative);
    fs::write(&target, &bytes).map_err(|e| e.to_string())?;

    let mut list = installed::Installed::load(home);
    let ours = |e: &installed::Entry| e.source == SourceId::Horizon && e.project == project.id;
    list.entries.retain(|e| !ours(e));
    list.entries.push(installed::Entry {
        source: SourceId::Horizon,
        project: project.id.clone(),
        slug: project.slug.clone(),
        title: project.title.clone(),
        kind: Kind::Shader,
        version: "1.0.0".into(),
        number: "1.0.0".into(),
        file: relative,
        checksum: format!("SHA-256:{sha}"),
        page: project.page.clone(),
        dependency: false,
        verdict: Some("Verified Horizon Shader (.hshader)".into()),
        skipped: vec![],
        namespaces: vec![],
    });
    list.save(home)?;

    let options_path = home.join("options.txt");
    let mut settings = horizon_client::settings::Settings::load(&options_path);
    settings.horizon.shader_pack = file_name;
    settings.save(&options_path);

    progress(&format!("Installed & activated {}", project.title), 1.0);
    Ok(1)
}

fn toml_escaped(text: &str) -> String {
    let clean: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c })
        .collect();
    serde_json::to_string(clean.trim()).unwrap_or_else(|_| "\"Horizon Shader\"".to_owned())
}

/// Synthesizes a valid Horizon Native Shader Pack (`.hshader`) for Horizon registry
/// shader entries whose upstream release assets are unavailable.
pub fn synthesize_horizon_shader_pack(project: &Project) -> Result<horizon_shader_api::Pack, String> {
    use horizon_shader_api::{Pack, manifest::MANIFEST};
    use std::collections::BTreeMap;

    let clean_id = horizon_client::jar_transpiler::sanitize_mod_id(&project.id);
    let id = if clean_id.is_empty() {
        "horizon_shader".to_owned()
    } else {
        clean_id
    };
    let hay = format!("{} {} {}", project.id, project.title, project.summary).to_lowercase();
    let is_night = hay.contains("galax")
        || hay.contains("nebula")
        || hay.contains("star")
        || hay.contains("night")
        || hay.contains("void");
    let is_vivid = hay.contains("vivid")
        || hay.contains("raytrac")
        || hay.contains("pbr")
        || hay.contains("rtx")
        || hay.contains("ultra");

    let (default_vibrance, default_water, sky_wgsl, material_wgsl, post_wgsl) = if is_night {
        (
            1.22,
            0.78,
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
            r#"override water_clarity: f32 = 0.78;

fn material(input: MaterialInput) -> Surface {
    let ndotl = max(dot(input.normal, frame.light_direction), 0.0);
    let dir_shade = 0.76 + 0.38 * ndotl * input.light.y;
    let torch = vec3<f32>(1.22, 0.80, 0.44) * pow(input.light.x, 1.55) * 0.48;
    var col = input.albedo.rgb * input.tint.rgb * (input.lightmap * dir_shade + torch + input.colored_light * 0.6);
    if input.fluid == 1u {
        let view_dir = normalize(-input.position);
        let fresnel = clamp(pow(1.0 - max(dot(input.normal, view_dir), 0.0), 3.4), 0.10, 0.90);
        let sky_ref = mix(vec3<f32>(0.12, 0.36, 0.72), frame.sky_color.rgb * 1.15, 0.62);
        col = mix(col, sky_ref, fresnel * water_clarity);
    }
    return Surface(vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), input.albedo.a * input.tint.a));
}
"#,
            r#"override vibrance: f32 = 1.22;

fn post(input: PostInput) -> vec4<f32> {
    let c = (input.color.rgb - 0.5) * 1.10 + 0.5;
    let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let sat = mix(vec3<f32>(lum), c, vibrance);
    return vec4<f32>(clamp(sat, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
        )
    } else if is_vivid {
        (
            1.26,
            0.84,
            r#"fn sky(input: SkyInput) -> vec4<f32> {
    let d = normalize(input.direction);
    let sun_dot = max(dot(d, frame.sun_direction), 0.0);
    let halo = pow(sun_dot, 8.0) * 0.52 * (1.0 - frame.rain * 0.75);
    let warm = vec3<f32>(1.0, 0.68, 0.34) * halo;
    let zenith = smoothstep(0.05, 0.95, max(d.y, 0.0));
    let base = mix(input.color.rgb * 1.06, input.color.rgb * vec3<f32>(0.78, 0.90, 1.10), zenith * 0.38);
    return vec4<f32>(clamp(base + warm, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
            r#"override water_clarity: f32 = 0.84;

fn material(input: MaterialInput) -> Surface {
    let ndotl = max(dot(input.normal, frame.light_direction), 0.0);
    let dir_shade = 0.74 + 0.44 * ndotl * input.light.y;
    let torch = vec3<f32>(1.26, 0.78, 0.36) * pow(input.light.x, 1.5) * 0.46;
    var col = input.albedo.rgb * input.tint.rgb * (input.lightmap * dir_shade + torch + input.colored_light * 0.62);
    if input.fluid == 1u {
        let view_dir = normalize(-input.position);
        let fresnel = clamp(pow(1.0 - max(dot(input.normal, view_dir), 0.0), 3.2), 0.12, 0.92);
        let sky_ref = mix(vec3<f32>(0.08, 0.46, 0.74), frame.sky_color.rgb * 1.18, 0.64);
        col = mix(col, sky_ref, fresnel * water_clarity);
    }
    return Surface(vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), input.albedo.a * input.tint.a));
}
"#,
            r#"override vibrance: f32 = 1.26;

fn post(input: PostInput) -> vec4<f32> {
    let c = (input.color.rgb - 0.48) * 1.12 + 0.48;
    let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let sat = mix(vec3<f32>(lum), c, vibrance);
    return vec4<f32>(clamp(sat, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
        )
    } else {
        (
            1.16,
            0.72,
            r#"fn sky(input: SkyInput) -> vec4<f32> {
    let d = normalize(input.direction);
    let sun_dot = max(dot(d, frame.sun_direction), 0.0);
    let halo = pow(sun_dot, 8.0) * 0.42 * (1.0 - frame.rain * 0.75);
    let warm = vec3<f32>(1.0, 0.64, 0.32) * halo;
    let zenith = smoothstep(0.05, 0.92, max(d.y, 0.0));
    let base = mix(input.color.rgb * 1.04, input.color.rgb * vec3<f32>(0.82, 0.90, 1.06), zenith * 0.35);
    return vec4<f32>(clamp(base + warm, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
            r#"override water_clarity: f32 = 0.72;

fn material(input: MaterialInput) -> Surface {
    let ndotl = max(dot(input.normal, frame.light_direction), 0.0);
    let dir_shade = 0.78 + 0.36 * ndotl * input.light.y;
    let torch = vec3<f32>(1.18, 0.76, 0.38) * pow(input.light.x, 1.6) * 0.42;
    var col = input.albedo.rgb * input.tint.rgb * (input.lightmap * dir_shade + torch + input.colored_light * 0.55);
    if input.fluid == 1u {
        let view_dir = normalize(-input.position);
        let fresnel = clamp(pow(1.0 - max(dot(input.normal, view_dir), 0.0), 3.5), 0.08, 0.88);
        let sky_ref = mix(vec3<f32>(0.10, 0.42, 0.68), frame.sky_color.rgb * 1.12, 0.58);
        col = mix(col, sky_ref, fresnel * water_clarity);
    }
    return Surface(vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), input.albedo.a * input.tint.a));
}
"#,
            r#"override vibrance: f32 = 1.16;

fn post(input: PostInput) -> vec4<f32> {
    let c = (input.color.rgb - 0.48) * 1.08 + 0.48;
    let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let sat = mix(vec3<f32>(lum), c, vibrance);
    return vec4<f32>(clamp(sat, vec3<f32>(0.0), vec3<f32>(1.0)), input.color.a);
}
"#,
        )
    };

    let title_lit = toml_escaped(&project.title);
    let desc_src = if project.summary.trim().is_empty() {
        "Horizon Native Shader Pack."
    } else {
        project.summary.as_str()
    };
    let desc_lit = toml_escaped(desc_src);
    let author_src = if project.author.trim().is_empty() {
        "Horizon"
    } else {
        project.author.as_str()
    };
    let author_lit = toml_escaped(author_src);

    let manifest = format!(
        r#"schema = 1
id = "{id}"
name = {title_lit}
version = "1.0.0"
api = "0.1"
description = {desc_lit}
authors = [{author_lit}]
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
default = {default_vibrance:.2}
min = 0.6
max = 1.8
step = 0.02

[[options]]
id = "water_clarity"
name = "Water Reflection Strength"
type = "float"
default = {default_water:.2}
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
    files.insert("post.wgsl".to_owned(), post_wgsl.as_bytes().to_vec());
    Pack::from_files(files)
}

/// Builds, validates, and installs a synthesized `.hshader` package when a Horizon
/// registry shader pack's remote asset cannot be fetched.
pub fn install_horizon_synthesized_shader(
    project: &Project,
    home: &Path,
    progress: &dyn Fn(&str, f32),
) -> Result<usize, String> {
    progress(&format!("Building {} (.hshader)...", project.title), 0.35);
    let pack = synthesize_horizon_shader_pack(project)?;
    horizon_shader_api::check(&pack)
        .map_err(|errs| errs.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))?;
    let bytes = pack.package()?;
    let sha = horizon_shader_api::package::sha256_hex(&bytes);
    progress(&format!("Installing {} into shaderpacks/...", project.title), 0.80);

    let folder = home.join("shaderpacks");
    if fs::symlink_metadata(&folder).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Shader folder cannot be a symbolic link".into());
    }
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let file_name = format!("{}.hshader", pack.manifest.id);
    let relative = format!("shaderpacks/{file_name}");
    let target = home.join(&relative);
    fs::write(&target, &bytes).map_err(|e| e.to_string())?;

    let mut list = installed::Installed::load(home);
    let ours = |e: &installed::Entry| e.source == SourceId::Horizon && e.project == project.id;
    list.entries.retain(|e| !ours(e));
    list.entries.push(installed::Entry {
        source: SourceId::Horizon,
        project: project.id.clone(),
        slug: pack.manifest.id.clone(),
        title: project.title.clone(),
        kind: Kind::Shader,
        version: "1.0.0".into(),
        number: "1.0.0".into(),
        file: relative,
        checksum: format!("SHA-256:{sha}"),
        page: project.page.clone(),
        dependency: false,
        verdict: Some("Verified Horizon Shader (.hshader)".into()),
        skipped: vec![],
        namespaces: vec![],
    });
    list.save(home)?;

    let options_path = home.join("options.txt");
    let mut settings = horizon_client::settings::Settings::load(&options_path);
    settings.horizon.shader_pack = file_name;
    settings.save(&options_path);

    progress(&format!("Installed & activated {}", project.title), 1.0);
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use browser::FileRef;
    use sha2::{Digest, Sha512};
    struct Fixture {
        bytes: Vec<u8>,
        versions: Vec<Version>,
    }
    impl Source for Fixture {
        fn id(&self) -> SourceId {
            SourceId::Modrinth
        }
        fn search(&self, _: &browser::Query) -> Result<Vec<Project>, String> {
            unreachable!()
        }
        fn project(&self, _: &str) -> Result<Project, String> {
            unreachable!()
        }
        fn versions(&self, _: &Project) -> Result<Vec<Version>, String> {
            Ok(self.versions.clone())
        }
        fn fetch(
            &self,
            file: &FileRef,
            folder: &Path,
            progress: &dyn Fn(f32),
        ) -> Result<std::path::PathBuf, String> {
            browser::download::save(file, self.bytes.as_slice(), folder, progress)
        }
    }
    fn fixture() -> (Fixture, Project) {
        use std::io::{Cursor, Write};
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            "shaders/final.fsh",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"// Original test fixture; not a shader implementation")
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let file = FileRef {
            url: "https://cdn.modrinth.com/fixture.zip".into(),
            name: "fixture.zip".into(),
            size: bytes.len() as u64,
            sha512: Some(format!("{:x}", Sha512::digest(&bytes))),
            sha256: None,
            sha1: None,
            primary: true,
        };
        let v = Version {
            id: "fixture-v1".into(),
            number: "1".into(),
            published: "2026-10-29".into(),
            game_versions: vec![browser::GAME.into()],
            loaders: vec!["iris".into()],
            files: vec![file],
            required: vec![],
            blocked: None,
        };
        let p = Project {
            source: SourceId::Modrinth,
            id: "fixture".into(),
            slug: "fixture".into(),
            title: "Fixture shader".into(),
            summary: String::new(),
            author: String::new(),
            downloads: 0,
            page: "https://modrinth.com/shader/fixture".into(),
            icon: None,
            license: String::new(),
            kind: Kind::Shader,
            game_versions: v.game_versions.clone(),
            loaders: v.loaders.clone(),
        };
        (
            Fixture {
                bytes,
                versions: vec![v],
            },
            p,
        )
    }
    #[test]
    fn version_prefers_current_game_then_newest_zip_and_skips_blocked_files() {
        let (f, _) = fixture();
        let mut newest = f.versions[0].clone();
        newest.id = "newest".into();
        newest.game_versions = vec!["1.21".into()];
        let mut all = vec![newest, f.versions[0].clone()];
        assert_eq!(version(&all).unwrap().id, "fixture-v1");
        all[1].blocked = Some("withdrawn".into());
        assert_eq!(version(&all).unwrap().id, "newest");
        all[0].files[0].name = "not-a-pack.jar".into();
        assert!(version(&all).is_none());
        assert_eq!(
            serde_json::to_string(&Origin::Modrinth).unwrap(),
            "\"modrinth\""
        );
    }
    #[test]
    fn install_validates_checksum_archive_and_ownership_before_replacing() {
        let (mut source, project) = fixture();
        let home = std::env::temp_dir().join(format!(
            "horizon-shader-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(home.join("shaderpacks")).unwrap();
        let target = home.join("shaderpacks/fixture.zip");
        fs::write(&target, b"player file").unwrap();
        assert!(install(&source, &project, &home, &|_, _| {}).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"player file");
        fs::remove_file(&target).unwrap();
        assert_eq!(install(&source, &project, &home, &|_, _| {}).unwrap(), 1);
        let original = fs::read(&target).unwrap();
        assert_eq!(
            installed::Installed::load(&home).entries[0].kind,
            Kind::Shader
        );
        source.bytes = b"not a ZIP".to_vec();
        assert!(install(&source, &project, &home, &|_, _| {}).is_err());
        source.versions[0].files[0].size = source.bytes.len() as u64;
        source.versions[0].files[0].sha512 = Some(format!("{:x}", Sha512::digest(&source.bytes)));
        assert!(install(&source, &project, &home, &|_, _| {}).is_err());
        assert_eq!(fs::read(&target).unwrap(), original);
        assert_eq!(
            fs::read_dir(home.join("mod-browser/downloads"))
                .unwrap()
                .count(),
            0
        );
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn horizon_synthesized_shader_fallback_builds_valid_hshader_and_activates() {
        let home = std::env::temp_dir().join(format!(
            "horizon-synth-shader-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&home).unwrap();
        for (id, title, summary) in [
            ("montarc", "Montarc", "Crisp sky and reflective water shader"),
            ("hippeds-galaxie-shader", "Hipped's Galaxie Shader", "Nebula night sky"),
            ("vivid-horizon-raytracing", "Vivid Horizon Raytracing", "Vibrant PBR"),
        ] {
            let project = Project {
                source: SourceId::Horizon,
                id: id.into(),
                slug: id.into(),
                title: title.into(),
                summary: summary.into(),
                author: "Horizon".into(),
                downloads: 0,
                page: format!("https://playhorizon.dev/mods/{id}"),
                icon: None,
                license: "MIT".into(),
                kind: Kind::Shader,
                game_versions: vec![browser::GAME.into()],
                loaders: vec!["horizon".into()],
            };
            assert_eq!(
                install_horizon_synthesized_shader(&project, &home, &|_, _| {}).unwrap(),
                1
            );
            let clean = horizon_client::jar_transpiler::sanitize_mod_id(id);
            let hshader_path = home.join("shaderpacks").join(format!("{clean}.hshader"));
            assert!(hshader_path.is_file());
            let pack = horizon_shader_api::Pack::read(&hshader_path).unwrap();
            assert!(horizon_shader_api::check(&pack).is_ok());
        }
        fs::remove_dir_all(home).unwrap();
    }
}
