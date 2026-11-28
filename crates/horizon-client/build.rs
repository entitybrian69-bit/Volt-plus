//! Embeds Horizon's icon (`packaging/icons/horizon.ico`) in the Windows executables, as icon
//! resource 1: Explorer and the task bar show it, and the window takes it as its icon.
//! The optional Segment Heap manifest is embedded in the same resource compilation.
fn main() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let stage = root.join(".horizon-mixins-stage.json");
    if stage.is_file() {
        println!("cargo:rerun-if-changed={}", stage.display());
        if let Ok(text) = std::fs::read_to_string(&stage)
            && let Some((_, after)) = text.split_once("\"mods\"")
            && let Some((_, list)) = after.split_once('[')
            && let Some((list, _)) = list.split_once(']')
        {
            let mods: Vec<&str> = list
                .split(',')
                .filter_map(|item| item.trim().strip_prefix('"')?.strip_suffix('"'))
                .filter(|id| !id.is_empty())
                .collect();
            if !mods.is_empty() {
                println!("cargo:rustc-env=HORIZON_MIXIN_MODS={}", mods.join(","));
            }
        }
    }
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let ico = format!("{dir}/../../packaging/icons/horizon.ico");
    println!("cargo:rerun-if-changed={ico}");
    generate_wallpaper_previews(&root);
    // The game's main thread gets 8 MB of stack on Windows as on macOS and Linux (Windows gives 1
    // MB): splitscreen builds a joining player's `Game`, ~67 KB, deep in the event loop.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let arg = match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
            Ok("msvc") => "/STACK:8388608",
            _ => "-Wl,--stack,8388608",
        };
        println!("cargo:rustc-link-arg-bin=horizon-client={arg}");
    }
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(&ico);
        if std::env::var_os("CARGO_FEATURE_WINDOWS_SEGMENT_HEAP").is_some() {
            let manifest = format!("{dir}/windows-segment-heap.manifest");
            println!("cargo:rerun-if-changed={manifest}");
            resource.set_manifest_file(&manifest);
        }
        if let Err(e) = resource.compile() {
            panic!("embedding Windows resources: {e}");
        }
    }
}

fn generate_wallpaper_previews(root: &std::path::Path) {
    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let preview_out = out_dir.join("wallpaper_previews");
    let _ = std::fs::create_dir_all(&preview_out);
    let wallpapers_dir = root.join("wallpapers");
    println!("cargo:rerun-if-changed={}", wallpapers_dir.display());
    let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&wallpapers_dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("mp4")
                && let Some(fname) = path.file_name().and_then(|n| n.to_str())
            {
                println!("cargo:rerun-if-changed={}", path.display());
                entries.push((fname.to_owned(), path));
            }
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let has_ffmpeg = std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());

    let mut code = String::from("pub fn embedded_preview(filename: &str) -> &'static [u8] {\n    match filename {\n");
    for (fname, mp4_path) in &entries {
        let stem = mp4_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("wallpaper");
        let checked_in = wallpapers_dir.join("previews").join(format!("{stem}.jpg"));
        let generated = preview_out.join(format!("{stem}.jpg"));
        let chosen = if checked_in.is_file() {
            println!("cargo:rerun-if-changed={}", checked_in.display());
            Some(checked_in)
        } else {
            let needs_gen = !generated.is_file()
                || generated
                    .metadata()
                    .and_then(|g| g.modified())
                    .ok()
                    .zip(mp4_path.metadata().and_then(|m| m.modified()).ok())
                    .is_some_and(|(gm, mm)| gm < mm);
            if needs_gen && has_ffmpeg {
                let extract = |seek: &str| {
                    std::process::Command::new("ffmpeg")
                        .args([
                            "-y",
                            "-loglevel",
                            "error",
                            "-ss",
                            seek,
                            "-i",
                        ])
                        .arg(mp4_path)
                        .args([
                            "-frames:v",
                            "1",
                            "-vf",
                            "scale=112:63:force_original_aspect_ratio=increase,crop=112:63",
                            "-q:v",
                            "4",
                        ])
                        .arg(&generated)
                        .status()
                        .is_ok_and(|s| s.success())
                        && generated.metadata().is_ok_and(|m| m.len() > 0)
                };
                if !extract("0.5") {
                    let _ = extract("0");
                }
            }
            if generated.metadata().is_ok_and(|m| m.len() > 0) {
                Some(generated)
            } else {
                None
            }
        };
        if let Some(jpg) = chosen {
            let escaped = jpg.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
            code.push_str(&format!(
                "        \"{fname}\" => include_bytes!(\"{escaped}\"),\n"
            ));
        }
    }
    code.push_str("        _ => &[],\n    }\n}\n");
    let out_rs = out_dir.join("wallpaper_previews.rs");
    let _ = std::fs::write(out_rs, code);
}
