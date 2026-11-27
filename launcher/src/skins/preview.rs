//! Textured 3D cuboids with a depth buffer. No frontend or GPU API dependency;
//! the same camera and RGBA output work in native UI, tests and future bridges.
use super::Model;
use image::{Rgba, RgbaImage};

#[derive(Clone, Copy)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -25.0,
            pitch: -8.0,
            zoom: 1.0,
        }
    }
}
impl Camera {
    pub fn rotate(&mut self, x: f32, y: f32) {
        self.yaw = (self.yaw + x).rem_euclid(360.0);
        self.pitch = (self.pitch + y).clamp(-65.0, 65.0);
    }
    /// Applies a screen-space touch/mouse drag delta `(dx, dy)` in dp so that dragging right
    /// (`dx > 0`) moves the visible camera-facing surface of the model right (`+screen_x`),
    /// and dragging down (`dy > 0`) tilts the visible camera-facing surface down (`+screen_y`),
    /// across both front (`yaw = 0°`) and back (`yaw = 180°`) orientations.
    pub fn drag(&mut self, dx: f32, dy: f32) {
        self.rotate(-dx * 0.7, -dy * 0.5);
    }
    pub fn zoom(&mut self, factor: f32) {
        if factor.is_finite() {
            self.zoom = (self.zoom * factor).clamp(0.5, 2.0);
        }
    }
}

/// Original neutral fitting model used only when no skin is selected.
pub fn mannequin() -> RgbaImage {
    RgbaImage::from_fn(64, 64, |x, y| {
        if y >= 32 {
            Rgba([0, 0, 0, 0])
        } else {
            let shade = if (x / 4 + y / 4) % 2 == 0 { 165 } else { 140 };
            Rgba([shade, shade, shade + 10, 255])
        }
    })
}

/// Original Steve-like fallback artwork used before official game assets are installed.
/// The flat UV layout is generated locally rather than bundling Minecraft textures.
pub fn default_steve() -> RgbaImage {
    let mut skin = RgbaImage::new(64, 64);
    let paint = |skin: &mut RgbaImage, x0: u32, y0: u32, w: u32, h: u32, c: [u8; 4]| {
        for y in y0..y0 + h { for x in x0..x0 + w {
            let mut shade = c;
            if (x * 3 + y * 5) % 11 == 0 {
                for channel in &mut shade[..3] { *channel = channel.saturating_sub(7); }
            }
            skin.put_pixel(x, y, Rgba(shade));
        }}
    };
    let face = [184, 129, 91, 255];
    let hair = [61, 42, 29, 255];
    let shirt = [26, 170, 165, 255];
    let trousers = [51, 55, 137, 255];
    paint(&mut skin, 0, 0, 32, 16, face);
    paint(&mut skin, 0, 0, 32, 8, hair);
    paint(&mut skin, 0, 8, 32, 2, hair);
    // Eyes and mouth on the front face of the cube.
    for (x, y, c) in [(9, 12, [239, 239, 236, 255]), (10, 12, [58, 65, 135, 255]),
        (13, 12, [58, 65, 135, 255]), (14, 12, [239, 239, 236, 255]),
        (11, 14, [84, 51, 37, 255]), (12, 14, [84, 51, 37, 255])] {
        skin.put_pixel(x, y, Rgba(c));
    }
    paint(&mut skin, 16, 16, 24, 16, shirt);
    paint(&mut skin, 0, 16, 16, 16, trousers);
    paint(&mut skin, 16, 48, 16, 16, trousers);
    for (x, y) in [(40, 16), (32, 48)] {
        paint(&mut skin, x, y, 16, 16, face);
        paint(&mut skin, x, y, 16, 8, shirt);
    }
    skin
}

/// Render a standalone three-dimensional cube with the same skin as the body.
pub fn render_head(skin: &RgbaImage, size: u32) -> RgbaImage {
    let side = size.clamp(16, 256);
    let mut raster = Raster {
        image: RgbaImage::new(side, side),
        depth: vec![f32::INFINITY; (side * side) as usize],
        camera: Camera { yaw: -25.0, pitch: -12.0, zoom: 3.4 },
    };
    raster.cuboid(skin, [0.0, 16.0, 0.0], [8.0, 8.0, 8.0], [0.0, 0.0], 0.0, false);
    if skin.height() >= 64 {
        raster.cuboid(skin, [0.0, 16.0, 0.0], [8.0, 8.0, 8.0], [32.0, 0.0], 0.5, false);
    }
    raster.image
}

struct Raster {
    image: RgbaImage,
    depth: Vec<f32>,
    camera: Camera,
}
impl Raster {
    fn project(&self, [x, y, z]: [f32; 3]) -> [f32; 3] {
        let (s, c) = self.camera.yaw.to_radians().sin_cos();
        let (x, z) = (x * c + z * s, -x * s + z * c);
        let (s, c) = self.camera.pitch.to_radians().sin_cos();
        let (y, z) = ((y - 16.0) * c - z * s, (y - 16.0) * s + z * c);
        let scale = self.image.height() as f32 / 39.0 * self.camera.zoom;
        let perspective = 100.0 / (100.0 + z);
        [
            self.image.width() as f32 * 0.5 + x * scale * perspective,
            self.image.height() as f32 * 0.5 - y * scale * perspective,
            z,
        ]
    }
    fn triangle(
        &mut self,
        points: [[f32; 3]; 3],
        uv: [[f32; 2]; 3],
        texture: &RgbaImage,
        light: f32,
    ) {
        let edge = |a: [f32; 3], b: [f32; 3], x: f32, y: f32| {
            (x - a[0]) * (b[1] - a[1]) - (y - a[1]) * (b[0] - a[0])
        };
        let area = edge(points[0], points[1], points[2][0], points[2][1]);
        if area.abs() < 0.0001 {
            return;
        }
        let minx = points
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let maxx = points
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, self.image.width() as f32) as u32;
        let miny = points
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.0) as u32;
        let maxy = points
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .clamp(0.0, self.image.height() as f32) as u32;
        for y in miny..maxy {
            for x in minx..maxx {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let weights = [
                    edge(points[1], points[2], px, py) / area,
                    edge(points[2], points[0], px, py) / area,
                    edge(points[0], points[1], px, py) / area,
                ];
                if weights.iter().any(|w| *w < -0.00001) {
                    continue;
                }
                let depth = (0..3).map(|i| points[i][2] * weights[i]).sum::<f32>();
                let index = (y * self.image.width() + x) as usize;
                if depth >= self.depth[index] {
                    continue;
                }
                let u = (0..3)
                    .map(|i| uv[i][0] * weights[i])
                    .sum::<f32>()
                    .floor()
                    .clamp(0.0, texture.width() as f32 - 1.0) as u32;
                let v = (0..3)
                    .map(|i| uv[i][1] * weights[i])
                    .sum::<f32>()
                    .floor()
                    .clamp(0.0, texture.height() as f32 - 1.0) as u32;
                let mut color = *texture.get_pixel(u, v);
                if color[3] < 128 {
                    continue;
                }
                for channel in 0..3 {
                    color[channel] = (color[channel] as f32 * light) as u8;
                }
                color[3] = 255;
                self.image.put_pixel(x, y, color);
                self.depth[index] = depth;
            }
        }
    }
    fn cuboid(
        &mut self,
        texture: &RgbaImage,
        center: [f32; 3],
        dimensions: [f32; 3],
        uv: [f32; 2],
        inflate: f32,
        reverse_depth: bool,
    ) {
        let [w, h, d] = dimensions;
        let [cx, cy, cz] = center;
        let (x0, x1) = (cx - w / 2.0 - inflate, cx + w / 2.0 + inflate);
        let (y0, y1) = (cy - h / 2.0 - inflate, cy + h / 2.0 + inflate);
        let (z0, z1) = (cz - d / 2.0 - inflate, cz + d / 2.0 + inflate);
        let [u, v] = uv;
        let mut faces = [
            (
                [[x0, y1, z0], [x1, y1, z0], [x1, y0, z0], [x0, y0, z0]],
                [u + d, v + d, w, h],
                1.0,
            ),
            (
                [[x1, y1, z0], [x1, y1, z1], [x1, y0, z1], [x1, y0, z0]],
                [u + d + w, v + d, d, h],
                0.78,
            ),
            (
                [[x1, y1, z1], [x0, y1, z1], [x0, y0, z1], [x1, y0, z1]],
                [u + d + w + d, v + d, w, h],
                0.9,
            ),
            (
                [[x0, y1, z1], [x0, y1, z0], [x0, y0, z0], [x0, y0, z1]],
                [u, v + d, d, h],
                0.82,
            ),
            (
                [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
                [u + d, v, w, d],
                1.0,
            ),
            (
                [[x0, y0, z1], [x1, y0, z1], [x1, y0, z0], [x0, y0, z0]],
                [u + d + w, v, w, d],
                0.65,
            ),
        ];
        if reverse_depth {
            for (points, _, _) in &mut faces {
                for point in points {
                    point[0] = 2.0 * cx - point[0];
                    point[2] = 2.0 * cz - point[2];
                }
            }
        }
        for (points, [u, v, w, h], light) in faces {
            let points = points.map(|p| self.project(p));
            let uv = [[u, v], [u + w, v], [u + w, v + h], [u, v + h]];
            for [a, b, c] in [[0, 1, 2], [0, 2, 3]] {
                self.triangle(
                    [points[a], points[b], points[c]],
                    [uv[a], uv[b], uv[c]],
                    texture,
                    light,
                );
            }
        }
    }

    fn wavey_cape(&mut self, texture: &RgbaImage, time_sec: f32) {
        // Matches the 16-slice curved cloth profile used in-game (cape spans y: 24..8 at z: 2.5..3.5).
        const SLICES: usize = 16;
        let (x0, x1) = (-5.0_f32, 5.0_f32);
        let z_front = 2.5_f32;
        let z_back = 3.5_f32;
        let y_top = 24.0_f32;
        let y_bot = 8.0_f32;
        let sample = |t: f32| -> (f32, f32) {
            let wave = (time_sec * 3.1 - t * 3.4).sin() * t * 1.15;
            let curve = t * t * 2.1 + t * 0.45 + wave;
            let y = y_top - t * (y_top - y_bot);
            (y, curve)
        };
        let mut emit_quad = |pts: [[f32; 3]; 4], uvs: [[f32; 2]; 4], light: f32| {
            let proj = pts.map(|p| self.project(p));
            for [a, b, c] in [[0, 1, 2], [0, 2, 3]] {
                self.triangle([proj[a], proj[b], proj[c]], [uvs[a], uvs[b], uvs[c]], texture, light);
            }
        };
        for i in 0..SLICES {
            let t0 = i as f32 / SLICES as f32;
            let t1 = (i + 1) as f32 / SLICES as f32;
            let (y0, dz0) = sample(t0);
            let (y1, dz1) = sample(t1);
            let (zf0, zb0) = (z_front + dz0, z_back + dz0);
            let (zf1, zb1) = (z_front + dz1, z_back + dz1);
            let v0 = 1.0 + t0 * 16.0;
            let v1 = 1.0 + t1 * 16.0;
            let slope_shade = (1.0 - (dz1 - dz0).abs() * 0.08).clamp(0.75, 1.05);
            // Outer back face (reversed X, matching cuboid reverse_depth: u 1..11)
            emit_quad(
                [[x1, y0, zb0], [x0, y0, zb0], [x0, y1, zb1], [x1, y1, zb1]],
                [[1.0, v0], [11.0, v0], [11.0, v1], [1.0, v1]],
                0.96 * slope_shade,
            );
            // Inner front face (u 12..22)
            emit_quad(
                [[x0, y0, zf0], [x1, y0, zf0], [x1, y1, zf1], [x0, y1, zf1]],
                [[12.0, v0], [22.0, v0], [22.0, v1], [12.0, v1]],
                0.78 * slope_shade,
            );
            // Left and right edges
            emit_quad(
                [[x0, y0, zb0], [x0, y0, zf0], [x0, y1, zf1], [x0, y1, zb1]],
                [[0.0, v0], [1.0, v0], [1.0, v1], [0.0, v1]],
                0.82,
            );
            emit_quad(
                [[x1, y0, zf0], [x1, y0, zb0], [x1, y1, zb1], [x1, y1, zf1]],
                [[11.0, v0], [12.0, v0], [12.0, v1], [11.0, v1]],
                0.78,
            );
            if i == 0 {
                emit_quad(
                    [[x1, y0, zf0], [x0, y0, zf0], [x0, y0, zb0], [x1, y0, zb0]],
                    [[1.0, 0.0], [11.0, 0.0], [11.0, 1.0], [1.0, 1.0]],
                    1.0,
                );
            }
            if i + 1 == SLICES {
                emit_quad(
                    [[x1, y1, zb1], [x0, y1, zb1], [x0, y1, zf1], [x1, y1, zf1]],
                    [[11.0, 0.0], [21.0, 0.0], [21.0, 1.0], [11.0, 1.0]],
                    0.65,
                );
            }
        }
    }
}

/// Bounded pixel output for interactive previews. Both skin layers and the cape
/// are actual geometry; rotating shows their backs, sides and top faces.
pub fn render(
    skin: &RgbaImage,
    cape: Option<&RgbaImage>,
    model: Model,
    camera: Camera,
    size: [u32; 2],
) -> RgbaImage {
    render_styled(skin, cape, model, camera, size, false, 0.0, 0)
}

/// Interactive preview with optional wavey cape cloth curve and animated cape frame selection.
pub fn render_styled(
    skin: &RgbaImage,
    cape: Option<&RgbaImage>,
    model: Model,
    camera: Camera,
    size: [u32; 2],
    wavey: bool,
    time_sec: f32,
    cape_frame_index: usize,
) -> RgbaImage {
    let [width, height] = size.map(|v| v.clamp(16, 1024));
    let mut raster = Raster {
        image: RgbaImage::new(width, height),
        depth: vec![f32::INFINITY; (width * height) as usize],
        camera,
    };
    let normalized = horizon_core::player_texture::process_legacy_skin(
        skin.width(),
        skin.height(),
        skin.as_raw(),
    )
    .and_then(|bytes| RgbaImage::from_raw(64, 64, bytes));
    let skin = normalized.as_ref().unwrap_or(skin);
    let cape_image = cape
        .map(|image| super::cape_frame_64x32(image, cape_frame_index))
        .and_then(|norm| {
            horizon_core::player_texture::process_cape(norm.width(), norm.height(), norm.as_raw())
        })
        .and_then(|bytes| RgbaImage::from_raw(64, 64, bytes));
    let cape = cape_image.as_ref();
    let arm = if model == Model::Slim { 3.0 } else { 4.0 };
    let modern = skin.height() == 64;
    for (center, dimensions, uv, outer) in [
        (
            [0.0, 28.0, 0.0],
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            Some([32.0, 0.0]),
        ),
        (
            [0.0, 18.0, 0.0],
            [8.0, 12.0, 4.0],
            [16.0, 16.0],
            modern.then_some([16.0, 32.0]),
        ),
        (
            [-4.0 - arm / 2.0, 18.0, 0.0],
            [arm, 12.0, 4.0],
            [40.0, 16.0],
            modern.then_some([40.0, 32.0]),
        ),
        (
            [4.0 + arm / 2.0, 18.0, 0.0],
            [arm, 12.0, 4.0],
            if modern { [32.0, 48.0] } else { [40.0, 16.0] },
            modern.then_some([48.0, 48.0]),
        ),
        (
            [-2.0, 6.0, 0.0],
            [4.0, 12.0, 4.0],
            [0.0, 16.0],
            modern.then_some([0.0, 32.0]),
        ),
        (
            [2.0, 6.0, 0.0],
            [4.0, 12.0, 4.0],
            if modern { [16.0, 48.0] } else { [0.0, 16.0] },
            modern.then_some([0.0, 48.0]),
        ),
    ] {
        raster.cuboid(skin, center, dimensions, uv, 0.0, false);
        if let Some(uv) = outer {
            raster.cuboid(
                skin,
                center,
                dimensions,
                uv,
                if dimensions[1] == 8.0 { 0.5 } else { 0.25 },
                false,
            );
        }
    }
    if let Some(cape) = cape {
        if wavey {
            raster.wavey_cape(cape, time_sec);
        } else {
            raster.cuboid(
                cape,
                [0.0, 16.0, 3.0],
                [10.0, 16.0, 1.0],
                [0.0, 0.0],
                0.0,
                true,
            );
        }
    }
    raster.image
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_character_and_head_are_textured_and_head_uses_selected_skin() {
        let skin = default_steve();
        assert_eq!(skin.dimensions(), (64, 64));
        let body = render(&skin, None, Model::Classic, Camera::default(), [160, 200]);
        assert!(body.pixels().any(|p| p[3] > 0 && u16::from(p[1]) > u16::from(p[0]) * 2));
        let head = render_head(&skin, 48);
        assert!(head.pixels().filter(|p| p[3] > 0).count() > 100);
        let mut changed = skin.clone();
        for y in 0..16 { for x in 0..32 { changed.put_pixel(x, y, Rgba([255, 0, 0, 255])); } }
        assert_ne!(head, render_head(&changed, 48));
    }

    #[test]
    fn rotation_model_and_cape_change_the_actual_geometry() {
        let skin = mannequin();
        let front = render(&skin, None, Model::Classic, Camera::default(), [160, 200]);
        let mut camera = Camera::default();
        camera.rotate(160.0, 20.0);
        camera.zoom(1.1);
        let back = render(&skin, None, Model::Slim, camera, [160, 200]);
        assert_ne!(front, back);
        assert!(front.pixels().filter(|p| p[3] > 0).count() > 1000);
        let cape = super::super::library::cape_design(0);
        assert_ne!(
            back,
            render(&skin, Some(&cape), Model::Slim, camera, [160, 200])
        );
    }

    #[test]
    fn drag_direction_matches_projected_screen_motion_front_and_back() {
        let make_raster = |camera: Camera| Raster {
            image: RgbaImage::new(160, 200),
            depth: vec![],
            camera,
        };

        // 1. Front orientation (yaw = 0, pitch = 0): front chest face is at [0.0, 18.0, -2.0]
        let front_pt = [0.0, 18.0, -2.0];
        let cam0 = Camera {
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let p0 = make_raster(cam0).project(front_pt);

        let mut cam_right = cam0;
        cam_right.drag(20.0, 0.0);
        let p_right = make_raster(cam_right).project(front_pt);
        assert!(
            p_right[0] > p0[0],
            "dragging right (dx > 0) must move front surface right (+screen_x)"
        );

        let mut cam_down = cam0;
        cam_down.drag(0.0, 20.0);
        let p_down = make_raster(cam_down).project(front_pt);
        assert!(
            p_down[1] > p0[1],
            "dragging down (dy > 0) must move front surface down (+screen_y)"
        );

        // 2. Back orientation (yaw = 180, pitch = 0): back cape face is at [0.0, 18.0, 3.0]
        let back_pt = [0.0, 18.0, 3.0];
        let cam_back0 = Camera {
            yaw: 180.0,
            pitch: 0.0,
            zoom: 1.0,
        };
        let b0 = make_raster(cam_back0).project(back_pt);

        let mut cam_back_right = cam_back0;
        cam_back_right.drag(20.0, 0.0);
        let b_right = make_raster(cam_back_right).project(back_pt);
        assert!(
            b_right[0] > b0[0],
            "dragging right (dx > 0) in back view must move visible back/cape surface right (+screen_x)"
        );

        let mut cam_back_down = cam_back0;
        cam_back_down.drag(0.0, 20.0);
        let b_down = make_raster(cam_back_down).project(back_pt);
        assert!(
            b_down[1] > b0[1],
            "dragging down (dy > 0) in back view must move visible back/cape surface down (+screen_y)"
        );
    }
}
