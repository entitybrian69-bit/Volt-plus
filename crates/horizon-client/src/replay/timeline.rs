//! Replay timelines: markers, position/rotation/roll/FOV keyframes, time
//! keyframes, Catmull-Rom spline interpolation, and 3D gizmo path preview.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayMarker {
    pub name: String,
    pub time_ms: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interpolation {
    #[default]
    CatmullRom,
    Linear,
}

impl Interpolation {
    pub fn label(self) -> &'static str {
        match self {
            Self::CatmullRom => "Smooth (Catmull-Rom)",
            Self::Linear => "Linear",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::CatmullRom => Self::Linear,
            Self::Linear => Self::CatmullRom,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionKeyframe {
    pub timeline_ms: u64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub fov: f32,
    #[serde(default)]
    pub interp: Interpolation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeKeyframe {
    pub timeline_ms: u64,
    pub replay_time_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub fov: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayTimeline {
    pub name: String,
    pub position_keyframes: Vec<PositionKeyframe>,
    pub time_keyframes: Vec<TimeKeyframe>,
    pub interpolation: Interpolation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayTimelineSet {
    pub active_index: usize,
    pub timelines: Vec<ReplayTimeline>,
}

impl Default for ReplayTimelineSet {
    fn default() -> Self {
        Self {
            active_index: 0,
            timelines: vec![ReplayTimeline {
                name: "Main Path".to_owned(),
                position_keyframes: Vec::new(),
                time_keyframes: Vec::new(),
                interpolation: Interpolation::CatmullRom,
            }],
        }
    }
}

impl ReplayTimelineSet {
    pub fn active(&self) -> &ReplayTimeline {
        self.timelines
            .get(self.active_index)
            .or_else(|| self.timelines.first())
            .expect("ReplayTimelineSet always holds at least one timeline")
    }

    pub fn active_mut(&mut self) -> &mut ReplayTimeline {
        if self.timelines.is_empty() {
            self.timelines.push(ReplayTimeline {
                name: "Main Path".to_owned(),
                position_keyframes: Vec::new(),
                time_keyframes: Vec::new(),
                interpolation: Interpolation::CatmullRom,
            });
            self.active_index = 0;
        } else if self.active_index >= self.timelines.len() {
            self.active_index = 0;
        }
        &mut self.timelines[self.active_index]
    }
}

impl ReplayTimeline {
    pub fn add_position_keyframe(&mut self, kf: PositionKeyframe) {
        if let Some(existing) = self
            .position_keyframes
            .iter_mut()
            .find(|p| p.timeline_ms.abs_diff(kf.timeline_ms) <= 25)
        {
            *existing = kf;
        } else {
            self.position_keyframes.push(kf);
            self.position_keyframes.sort_by_key(|k| k.timeline_ms);
        }
    }

    pub fn remove_nearest_position_keyframe(&mut self, timeline_ms: u64, max_dist_ms: u64) -> bool {
        if let Some((idx, _)) = self
            .position_keyframes
            .iter()
            .enumerate()
            .min_by_key(|(_, k)| k.timeline_ms.abs_diff(timeline_ms))
            && self.position_keyframes[idx]
                .timeline_ms
                .abs_diff(timeline_ms)
                <= max_dist_ms
        {
            self.position_keyframes.remove(idx);
            return true;
        }
        false
    }

    pub fn add_time_keyframe(&mut self, kf: TimeKeyframe) {
        if let Some(existing) = self
            .time_keyframes
            .iter_mut()
            .find(|p| p.timeline_ms.abs_diff(kf.timeline_ms) <= 25)
        {
            *existing = kf;
        } else {
            self.time_keyframes.push(kf);
            self.time_keyframes.sort_by_key(|k| k.timeline_ms);
        }
    }

    pub fn remove_nearest_time_keyframe(&mut self, timeline_ms: u64, max_dist_ms: u64) -> bool {
        if let Some((idx, _)) = self
            .time_keyframes
            .iter()
            .enumerate()
            .min_by_key(|(_, k)| k.timeline_ms.abs_diff(timeline_ms))
            && self.time_keyframes[idx]
                .timeline_ms
                .abs_diff(timeline_ms)
                <= max_dist_ms
        {
            self.time_keyframes.remove(idx);
            return true;
        }
        false
    }

    pub fn clear(&mut self) {
        self.position_keyframes.clear();
        self.time_keyframes.clear();
    }

    pub fn duration_ms(&self) -> u64 {
        let pos_end = self
            .position_keyframes
            .last()
            .map_or(0, |k| k.timeline_ms);
        let time_end = self.time_keyframes.last().map_or(0, |k| k.timeline_ms);
        pos_end.max(time_end)
    }

    pub fn can_preview_path(&self) -> bool {
        self.position_keyframes.len() >= 2
    }

    /// Evaluates the replay timestamp corresponding to `timeline_ms`.
    /// Falls back to `timeline_ms` directly if fewer than 2 time keyframes exist.
    pub fn sample_replay_time_ms(&self, timeline_ms: u64, fallback_ms: u64) -> u64 {
        let keys = &self.time_keyframes;
        if keys.is_empty() {
            return fallback_ms;
        }
        if keys.len() == 1 {
            return keys[0].replay_time_ms;
        }
        if timeline_ms <= keys[0].timeline_ms {
            return keys[0].replay_time_ms;
        }
        if timeline_ms >= keys[keys.len() - 1].timeline_ms {
            return keys[keys.len() - 1].replay_time_ms;
        }
        for pair in keys.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if timeline_ms >= a.timeline_ms && timeline_ms <= b.timeline_ms {
                let span = (b.timeline_ms - a.timeline_ms).max(1) as f64;
                let t = ((timeline_ms - a.timeline_ms) as f64 / span).clamp(0.0, 1.0);
                let val = a.replay_time_ms as f64
                    + (b.replay_time_ms as f64 - a.replay_time_ms as f64) * t;
                return val.round().max(0.0) as u64;
            }
        }
        fallback_ms
    }

    /// Evaluates the smooth camera pose along the position keyframes at `timeline_ms`.
    pub fn sample_camera_pose(&self, timeline_ms: u64) -> Option<CameraPose> {
        let keys = &self.position_keyframes;
        if keys.is_empty() {
            return None;
        }
        if keys.len() == 1 || timeline_ms <= keys[0].timeline_ms {
            let k = &keys[0];
            return Some(CameraPose {
                x: k.x,
                y: k.y,
                z: k.z,
                yaw: k.yaw,
                pitch: k.pitch,
                roll: k.roll,
                fov: k.fov,
            });
        }
        let last = keys.len() - 1;
        if timeline_ms >= keys[last].timeline_ms {
            let k = &keys[last];
            return Some(CameraPose {
                x: k.x,
                y: k.y,
                z: k.z,
                yaw: k.yaw,
                pitch: k.pitch,
                roll: k.roll,
                fov: k.fov,
            });
        }

        let mut seg = 0;
        for i in 0..last {
            if timeline_ms >= keys[i].timeline_ms && timeline_ms <= keys[i + 1].timeline_ms {
                seg = i;
                break;
            }
        }

        let k1 = &keys[seg];
        let k2 = &keys[seg + 1];
        let span = (k2.timeline_ms - k1.timeline_ms).max(1) as f64;
        let t = ((timeline_ms - k1.timeline_ms) as f64 / span).clamp(0.0, 1.0);

        // Unwrap yaw & roll sequentially so splines take the shortest angular path.
        let k0 = if seg > 0 { &keys[seg - 1] } else { k1 };
        let k3 = if seg + 2 < keys.len() {
            &keys[seg + 2]
        } else {
            k2
        };

        let yaw1 = k1.yaw;
        let yaw0 = unwrap_near(k0.yaw, yaw1);
        let yaw2 = unwrap_near(k2.yaw, yaw1);
        let yaw3 = unwrap_near(k3.yaw, yaw2);

        let roll1 = k1.roll;
        let roll0 = unwrap_near(k0.roll, roll1);
        let roll2 = unwrap_near(k2.roll, roll1);
        let roll3 = unwrap_near(k3.roll, roll2);

        if self.interpolation == Interpolation::Linear || keys.len() < 3 {
            let tf = t as f32;
            return Some(CameraPose {
                x: k1.x + (k2.x - k1.x) * t,
                y: k1.y + (k2.y - k1.y) * t,
                z: k1.z + (k2.z - k1.z) * t,
                yaw: wrap_degrees(yaw1 + (yaw2 - yaw1) * tf),
                pitch: (k1.pitch + (k2.pitch - k1.pitch) * tf).clamp(-90.0, 90.0),
                roll: wrap_degrees(roll1 + (roll2 - roll1) * tf),
                fov: (k1.fov + (k2.fov - k1.fov) * tf).clamp(20.0, 140.0),
            });
        }

        let tf = t as f32;
        Some(CameraPose {
            x: catmull_rom_f64(k0.x, k1.x, k2.x, k3.x, t),
            y: catmull_rom_f64(k0.y, k1.y, k2.y, k3.y, t),
            z: catmull_rom_f64(k0.z, k1.z, k2.z, k3.z, t),
            yaw: wrap_degrees(catmull_rom_f32(yaw0, yaw1, yaw2, yaw3, tf)),
            pitch: catmull_rom_f32(k0.pitch, k1.pitch, k2.pitch, k3.pitch, tf).clamp(-90.0, 90.0),
            roll: wrap_degrees(catmull_rom_f32(roll0, roll1, roll2, roll3, tf)),
            fov: catmull_rom_f32(k0.fov, k1.fov, k2.fov, k3.fov, tf).clamp(20.0, 140.0),
        })
    }

    /// Generates 3D line segments (`start`, `end`, ARGB color) for visualizing
    /// the spline path and keyframe markers in `Gizmos`.
    pub fn path_preview_lines(&self) -> Vec<([f64; 3], [f64; 3], u32)> {
        let mut lines = Vec::new();
        if self.position_keyframes.is_empty() {
            return lines;
        }
        // Keyframe diamonds + look direction arrows.
        for k in &self.position_keyframes {
            let p = [k.x, k.y, k.z];
            let s = 0.25;
            lines.push(([p[0] - s, p[1], p[2]], [p[0] + s, p[1], p[2]], 0xFFFF_4081));
            lines.push(([p[0], p[1] - s, p[2]], [p[0], p[1] + s, p[2]], 0xFFFF_4081));
            lines.push(([p[0], p[1], p[2] - s], [p[0], p[1], p[2] + s], 0xFFFF_4081));

            let (yr, pr) = (f64::from(k.yaw).to_radians(), f64::from(k.pitch).to_radians());
            let dir = [-yr.sin() * pr.cos(), -pr.sin(), yr.cos() * pr.cos()];
            lines.push((
                p,
                [p[0] + dir[0] * 0.9, p[1] + dir[1] * 0.9, p[2] + dir[2] * 0.9],
                0xFFFF_EA00,
            ));
        }
        let duration = self.duration_ms();
        if self.position_keyframes.len() >= 2 && duration > 0 {
            let start = self.position_keyframes[0].timeline_ms;
            let end = self.position_keyframes[self.position_keyframes.len() - 1].timeline_ms;
            let steps = ((end - start) / 100).clamp(16, 400);
            let mut prev = self.sample_camera_pose(start);
            for i in 1..=steps {
                let t = start + ((end - start) * i) / steps;
                let next = self.sample_camera_pose(t);
                if let (Some(a), Some(b)) = (prev, next) {
                    lines.push(([a.x, a.y, a.z], [b.x, b.y, b.z], 0xFF00_E5FF));
                }
                prev = next;
            }
        }
        lines
    }
}

fn catmull_rom_f64(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn catmull_rom_f32(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

pub fn wrap_degrees(mut deg: f32) -> f32 {
    deg %= 360.0;
    if deg >= 180.0 {
        deg -= 360.0;
    } else if deg < -180.0 {
        deg += 360.0;
    }
    deg
}

fn unwrap_near(angle: f32, reference: f32) -> f32 {
    reference + wrap_degrees(angle - reference)
}
