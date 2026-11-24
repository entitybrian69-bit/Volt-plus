//! The recorder's own swapchain: the frame as it is before the HUD is drawn.
//!
//! A raw recording (`Record With Controls` off) keeps the touch controls on screen but out
//! of the video. Android's screen capture mirrors the display, so it cannot leave the HUD
//! out; instead the game copies each frame into the encoder's own surface *before* the HUD
//! pass, and `MediaCodec` reads that surface (see `EngineRecorder.java`). One scaled copy
//! per frame, through the same post pass the Android client already uses to present.
use super::world_post;

pub(super) struct RecordTarget {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    post: world_post::Post,
    format: wgpu::TextureFormat,
    /// When the encoder last got a frame: a 120 Hz display must not push twice what the
    /// recording asked for.
    last: Option<std::time::Instant>,
    failed: bool,
}

impl RecordTarget {
    /// Configures `surface` (the encoder's `Surface`, as a swapchain) and builds the copy
    /// pass for its format. `None` when the surface offers no usable format or size.
    pub(super) fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        surface: wgpu::Surface<'static>,
        size: [u32; 2],
    ) -> Option<Self> {
        let caps = surface.get_capabilities(adapter);
        if !caps.usages.contains(wgpu::TextureUsages::RENDER_ATTACHMENT) {
            return None;
        }
        let format = [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ]
        .into_iter()
        .find(|f| caps.formats.contains(f))
        .or_else(|| caps.formats.first().copied())?;
        let alpha_mode = if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::Opaque)
        {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            caps.alpha_modes.first().copied()?
        };
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            *caps.present_modes.first()?
        };
        // H.264 wants even sides, and a surface needs two pixels to exist at all.
        let size = [(size[0].max(2)) & !1, (size[1].max(2)) & !1];
        if size[0] < 16 || size[1] < 16 {
            return None;
        }
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size[0],
            height: size[1],
            // A codec surface has no display to pace it: Fifo keeps one frame in flight.
            present_mode,
            desired_maximum_frame_latency: 1,
            alpha_mode,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::default(),
        };
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let oom = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        surface.configure(device, &config);
        let post = world_post::Post::new(device, format);
        let err = pollster::block_on(oom.pop())
            .or_else(|| pollster::block_on(internal.pop()))
            .or_else(|| pollster::block_on(validation.pop()));
        if let Some(err) = err {
            log::warn!("Horizon renderer: could not configure recorder surface ({err})");
            return None;
        }
        Some(Self {
            surface,
            config,
            post,
            format,
            last: None,
            failed: false,
        })
    }

    pub(super) fn size(&self) -> [u32; 2] {
        [self.config.width, self.config.height]
    }

    /// Copies `source` (the frame before its HUD, `source_size` pixels) into the encoder's
    /// frame, letterboxed when the two aspects differ. The returned frame is presented by
    /// the caller once the submission is queued.
    pub(super) fn present(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        source_size: [u32; 2],
    ) -> Option<wgpu::SurfaceTexture> {
        if self.failed {
            return None;
        }
        // At most sixty frames a second reach the encoder, however fast the screen is.
        if let Some(last) = self.last {
            if last.elapsed().as_secs_f32() < 1.0 / 60.0 {
                return None;
            }
        }
        self.last = Some(std::time::Instant::now());
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                // A codec that restarted: configure it again and keep the last frame.
                let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
                let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
                let oom = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
                self.surface.configure(device, &self.config);
                if let Some(err) = pollster::block_on(oom.pop())
                    .or_else(|| pollster::block_on(internal.pop()))
                    .or_else(|| pollster::block_on(validation.pop()))
                {
                    log::warn!("Horizon renderer: recorder surface reconfigure failed ({err})");
                    self.failed = true;
                }
                return None;
            }
            // Busy, occluded or failed: this frame simply is not recorded.
            _ => return None,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.format),
            ..Default::default()
        });
        let group = self.post.bind(device, source);
        let (source_w, source_h) = (
            source_size[0].max(1) as f32,
            source_size[1].max(1) as f32,
        );
        let (target_w, target_h) = (self.config.width as f32, self.config.height as f32);
        let scale = (target_w / source_w).min(target_h / source_h);
        let width = ((source_w * scale) as u32).max(2).min(self.config.width) & !1;
        let height = ((source_h * scale) as u32).max(2).min(self.config.height) & !1;
        let rect = [
            (self.config.width - width) / 2,
            (self.config.height - height) / 2,
            width,
            height,
        ];
        // `finish_in_rect` clears the whole frame first, so the bars are black.
        self.post
            .finish_in_rect(encoder, &view, &group, Some(rect));
        Some(frame)
    }
}
