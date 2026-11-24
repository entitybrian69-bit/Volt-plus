# Volt + Boost & Replays — Design & Ambiguity Decisions

This file records design choices and ambiguity resolutions made while implementing **Volt + Boost** (in-game settings + engine hooks) and **Replays** (recording, playback viewer, and launcher Replays page).

## 1. Existing Screens Byte-for-Byte Preservation
- **Decision**: Keep `Page::Horizon` (`volt_pages_ui.rs`), `Page::Video` (`volt_video_ui.rs`), `Page::Graphics` / `Page::GraphicsAdvanced` (`graphics_ui.rs`), and `SettingsCategory::Experimental` (`launcher_pages.rs`) completely untouched.
- **Placement**: Add `Volt + Boost` (`Page::VoltPlusBoost`) directly after `Volt+ Settings...` (`Page::Horizon`) in `Page::Main`, and add only `Replays` (`SettingsCategory::Replays`) in Launcher → Settings → Overview.

## 2. Persistence & Safe Defaults
- **Decision**: Persist all Volt + Boost tuning state and per-device floating Replay HUD geometry in `volt-plus-boost.json` inside `game_home`, loaded and saved alongside `options.txt` in `Settings::load` / `Settings::save`.
- **Defaults**: All performance/tuning switches default to safe/off (`BoostProfile::Off`, toggles `false`, `EntityShadows: true`, `ChunkFade: 1.0`, `ChunkBuildDistance: 12`, `FpsCap: 120`) so nothing changes gameplay until enabled.
- **Application timing**: Tuning changes persist immediately and latch onto the world session on next world/server join, while live overlays/Display & Framerate/Replay HUD react immediately where safe.

## 3. Replay Format (`.vreplay`) & Free-Camera Playback
- **Decision**: Implement a single-file portable `.vreplay` format (`VREP0001` magic + JSON metadata + embedded PNG thumbnail + `flate2` zlib-compressed frame/entity/block/HUD delta stream + FNV-1a/CRC integrity hash).
- **Rationale**: Free-camera 3D fly-through, third-person self camera, and frame-accurate timeline scrubbing at `<= ~2%` FPS recording cost require compact scene state snapshots rather than heavy real-time video pixel readback every frame.
- **Corruption handling**: Truncated, missing, or checksum-failing `.vreplay` files return `ReplayError::Damaged` and display an inline `"This clip is damaged"` warning in the Launcher Replays list without crashing.

## 4. Storage Budget & Ring Buffer
- **Decision**: First time `Replay Recording` is enabled (`storage_budget_asked == false`), display an inline storage budget prompt (`256 MB`, `512 MB`, `1 GB`, `2 GB`, `4 GB`).
- **Ring Buffer**: `Recording Length` (`30 s · 1 min · 5 min · 10 min · Session`) bounds an in-memory `VecDeque<ReplayFrame>` evicting oldest frames first; on save, oldest clips are pruned if total `replays/` usage exceeds the storage budget.

## 5. UI Tokens, Motion, and Accessibility
- **Visual Tokens**: `#11141B` glass cards, `1 px` translucent white hairlines, `12.5` card radius, `10` toggle knob radius, `#E03044` active accent and `2 dp` focus ring, `#F2F5F3` / `#C6CCC9` / `#8F969B` text colors, `>= 48 dp` hit/row heights (`56 dp` with inline dependency/status hints), `16 dp` outer margins, `20 dp` category gaps, and safe-area insets.
- **Sticky Headers & Motion**: Currently visible category header pins at the top of the scroll viewport while options pass beneath; all animations use ease-out-cubic (`<= 250 ms`, `220 ms` expand/collapse, `180 ms` toggle, `18 ms`/row staggered master-switch fade, `0.958` press scale, zero bounce, fully interruptible).
- **Text Scale & Non-Color State**: Layout scales cleanly up to `130%` text size; every row renders its state as a word (`On`, `Off`, profile/mode name, numeric value, or action status) alongside the visual control.

## 6. Hardware / Thermal / Threading Fallbacks
- **Auto-Tune Device**: Reads CPU core count (`available_parallelism`), architecture, and system RAM (`/proc/meminfo`) to choose `Low-RAM Rescue`, `Balanced`, or `Ultra FPS` with an inline explanation.
- **big.LITTLE Boost**: Uses `sched_setaffinity` on Android/Linux to prefer upper-half performance cores when permitted, falling back cleanly if restricted by OS policy.
- **Thermal Guard**: Reads `/sys/class/thermal/thermal_zone*/temp` and sustained frame-time pressure to step down render scale / FPS cap / effects cleanly, reporting the active step inline on the row.
