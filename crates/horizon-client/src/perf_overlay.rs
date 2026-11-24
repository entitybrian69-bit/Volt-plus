//! The performance overlay (Horizon's Overlay setting): frames per second,
//! frame, CPU and GPU times, and a graph of recent frame times.
use crate::{
    graphics::Overlay,
    hud::{HudAtlas, HudBatch, rgba},
    text::Text,
};
use horizon_client_render::volt_boost::PerfHudCorner;
use std::collections::VecDeque;

const HISTORY: usize = 160;

#[derive(Clone, Debug, Default)]
pub struct LivePerfMetrics {
    pub ram_mb: f32,
    pub draw_calls: u32,
    pub drs_scale_pct: u32,
    pub temp_c: Option<f32>,
    pub battery_pct: Option<u8>,
    pub thermal_tier: &'static str,
    pub reflex_active: bool,
    pub replay_recording: bool,
    pub replay_clips: usize,
}

#[derive(Default)]
pub struct FrameGraph {
    /// Frame, CPU and GPU milliseconds of recent frames, oldest first.
    frames: VecDeque<(f32, f32, f32)>,
}
impl FrameGraph {
    pub fn record(&mut self, frame_ms: f32, cpu_ms: f32, gpu_ms: Option<f32>) {
        let gpu = gpu_ms.unwrap_or_else(|| self.frames.back().map_or(0.0, |f| f.2));
        if self.frames.len() == HISTORY {
            self.frames.pop_front();
        }
        self.frames.push_back((frame_ms, cpu_ms, gpu));
    }
    pub fn draw(&self, mode: Overlay, batch: &mut HudBatch, atlas: &HudAtlas) {
        if mode == Overlay::Off || self.frames.is_empty() {
            return;
        }
        let n = self.frames.len() as f32;
        let avg = |f: fn(&(f32, f32, f32)) -> f32| self.frames.iter().map(f).sum::<f32>() / n;
        let (frame, cpu, gpu) = (avg(|f| f.0), avg(|f| f.1), avg(|f| f.2));
        let worst = self.frames.iter().map(|f| f.0).fold(0.0f32, f32::max);
        let lines = [
            format!("{:.0} fps", 1000.0 / frame.max(0.001)),
            format!("frame {frame:.1} ms  max {worst:.1}"),
            format!("cpu {cpu:.1}  gpu {gpu:.1} ms"),
        ];
        let width = 120.0f32.max(
            lines
                .iter()
                .map(|l| atlas.font.width(l))
                .fold(0.0, f32::max)
                + 6.0,
        );
        let x = batch.width - width - 2.0;
        let graph = if mode == Overlay::Graph { 42.0 } else { 0.0 };
        batch.fill(
            atlas,
            x,
            2.0,
            width,
            4.0 + lines.len() as f32 * 10.0 + graph,
            rgba(0, 0, 0, 144),
        );
        for (i, line) in lines.iter().enumerate() {
            let color = if i == 0 {
                rgba(255, 255, 255, 255)
            } else {
                rgba(200, 200, 200, 255)
            };
            batch.text_styled(
                atlas,
                &Text::literal(line.clone()),
                x + 3.0,
                4.0 + i as f32 * 10.0,
                color,
                true,
            );
        }
        if mode != Overlay::Graph {
            return;
        }
        // Bars of frame time on a 0..33 ms scale; lines at 60 and 30 fps.
        let (gy, gh) = (4.0 + lines.len() as f32 * 10.0 + 2.0, 36.0);
        let scale = gh / 33.3;
        let bar = (width - 6.0) / HISTORY as f32;
        for (i, &(ms, cpu, _)) in self.frames.iter().enumerate() {
            let height = (ms * scale).min(gh);
            let bx = x + 3.0 + i as f32 * bar;
            let color = if ms > 33.3 {
                rgba(240, 90, 80, 255)
            } else if ms > 16.7 {
                rgba(240, 200, 90, 255)
            } else {
                rgba(110, 210, 120, 255)
            };
            batch.fill(atlas, bx, gy + gh - height, bar.max(1.0), height, color);
            let cpu_height = (cpu * scale).min(height);
            batch.fill(
                atlas,
                bx,
                gy + gh - cpu_height,
                bar.max(1.0),
                cpu_height,
                rgba(80, 140, 230, 200),
            );
        }
        for target in [16.7, 33.3] {
            let ly = gy + gh - target * scale;
            batch.fill(
                atlas,
                x + 3.0,
                ly,
                width - 6.0,
                1.0,
                rgba(255, 255, 255, 90),
            );
        }
    }

    pub fn avg_frame_ms(&self) -> f32 {
        if self.frames.is_empty() {
            return 0.0;
        }
        self.frames.iter().map(|f| f.0).sum::<f32>() / self.frames.len() as f32
    }

    pub fn avg_fps(&self) -> u32 {
        let ms = self.avg_frame_ms();
        if ms <= 0.001 {
            0
        } else {
            (1000.0 / ms).round() as u32
        }
    }

    pub fn one_percent_low_fps(&self) -> u32 {
        if self.frames.is_empty() {
            return 0;
        }
        let mut sorted: Vec<f32> = self.frames.iter().map(|f| f.0).collect();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let idx = ((sorted.len() as f32 * 0.99).floor() as usize).min(sorted.len() - 1);
        let p99_ms = sorted[idx].max(0.001);
        (1000.0 / p99_ms).round() as u32
    }

    pub fn draw_live_hud(
        &self,
        enabled: bool,
        corner: PerfHudCorner,
        metrics: &LivePerfMetrics,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
    ) {
        if !enabled {
            return;
        }
        let fps = self.avg_fps();
        let low1 = self.one_percent_low_fps();
        let ms = self.avg_frame_ms();
        let cpu = if self.frames.is_empty() {
            0.0
        } else {
            self.frames.iter().map(|f| f.1).sum::<f32>() / self.frames.len() as f32
        };
        let gpu = if self.frames.is_empty() {
            0.0
        } else {
            self.frames.iter().map(|f| f.2).sum::<f32>() / self.frames.len() as f32
        };
        let mut lines = Vec::new();
        lines.push((
            format!("VOLT+ {fps} FPS  1% {low1}  {ms:.1}ms"),
            rgba(224, 48, 68, 255),
        ));
        lines.push((
            format!(
                "CPU {cpu:.1}ms  GPU {gpu:.1}ms  Draws {}",
                metrics.draw_calls
            ),
            rgba(225, 232, 240, 255),
        ));
        let therm = match (metrics.temp_c, metrics.battery_pct) {
            (Some(t), Some(b)) => format!("{t:.0}C {b}% ({})", metrics.thermal_tier),
            (Some(t), None) => format!("{t:.0}C ({})", metrics.thermal_tier),
            (None, Some(b)) => format!("Bat {b}% ({})", metrics.thermal_tier),
            (None, None) => format!("Therm {}", metrics.thermal_tier),
        };
        lines.push((
            format!(
                "RAM {:.0}MB  DRS {}%  {}",
                metrics.ram_mb, metrics.drs_scale_pct, therm
            ),
            rgba(190, 204, 222, 255),
        ));
        let reflex = if metrics.reflex_active { "ON" } else { "OFF" };
        let rec = if metrics.replay_recording {
            format!("REC ({} clips)", metrics.replay_clips)
        } else {
            format!("Idle ({} clips)", metrics.replay_clips)
        };
        lines.push((
            format!("Reflex {reflex}  Replay {rec}"),
            rgba(160, 210, 180, 255),
        ));

        let width = lines
            .iter()
            .map(|(l, _)| atlas.font.width(l))
            .fold(132.0f32, f32::max)
            + 10.0;
        let h = 6.0 + lines.len() as f32 * 10.0;
        let (x, y) = match corner {
            PerfHudCorner::TopLeft => (6.0, 6.0),
            PerfHudCorner::TopRight => ((batch.width - width - 6.0).max(6.0), 6.0),
            PerfHudCorner::BottomLeft => (6.0, (batch.height - h - 6.0).max(6.0)),
            PerfHudCorner::BottomRight => (
                (batch.width - width - 6.0).max(6.0),
                (batch.height - h - 6.0).max(6.0),
            ),
        };
        batch.fill(atlas, x, y, width, h, rgba(11, 14, 20, 184));
        batch.fill(atlas, x, y, 2.0, h, rgba(224, 48, 68, 235));
        for (i, (line, color)) in lines.iter().enumerate() {
            batch.text_styled(
                atlas,
                &Text::literal(line.clone()),
                x + 5.0,
                y + 3.0 + i as f32 * 10.0,
                *color,
                true,
            );
        }
    }
}

#[cfg(test)]
mod volt_boost_perf_tests {
    use super::*;

    #[test]
    fn frame_graph_reports_fps_and_one_percent_low() {
        let mut g = FrameGraph::default();
        for _ in 0..99 {
            g.record(8.0, 4.0, Some(3.0));
        }
        g.record(25.0, 10.0, Some(12.0));
        assert!(g.avg_fps() >= 100);
        assert!(g.one_percent_low_fps() <= 45);
    }
}
