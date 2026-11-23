//! Thin native view over the shared account and wardrobe APIs.
use super::{
    login::Login,
    pages::{Action, button},
};
use crate::{
    account::{Accounts, Provider},
    skins::{
        self, Model, cloud,
        library::{Kind, Library},
        preview,
    },
};
use horizon_client::{
    hud::rgba,
    skin_head::{self, HeadSkin},
    ui_theme::{Face, Ui},
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

pub(super) enum CloudAction {
    Upload,
    Cape(usize),
    DeactivateCape,
}
pub(super) struct Wardrobe {
    pub root: PathBuf,
    default_skin: PathBuf,
    library: Library,
    key: String,
    provider: Option<Provider>,
    pub capes: bool,
    pub camera: preview::Camera,
    cache: cloud::Cache,
    pub view: [f32; 3],
    worker: Option<mpsc::Receiver<(String, Result<(cloud::Cache, bool), String>)>>,
    bitmap: image::RgbaImage,
    bitmap_revision: u64,
    preview_skin: Option<std::sync::Arc<image::RgbaImage>>,
    preview_cape: Option<std::sync::Arc<image::RgbaImage>>,
    preview_model: Model,
    appearance_generation: u64,
    render_worker: Option<mpsc::Receiver<(u64, image::RgbaImage)>>,
    render_pending: bool,
    head_bitmap: Vec<([f32; 4], u32)>,
    rect: [f32; 4],
    fingers: std::collections::BTreeMap<u64, [f32; 2]>,
    note: String,
    damaged: bool,
    /// The account whose appearance is shown, used to key the head cache.
    name: String,
    /// Decoded head of the selected appearance, drawn by the launcher header.
    head: Option<HeadSkin>,
    pub tab_changed_at: Option<std::time::Instant>,
    pub selection_changed_at: Option<std::time::Instant>,
}
impl Wardrobe {
    pub fn new(home: &Path) -> Self {
        let root = home.join("wardrobe");
        let loaded = Library::load(&root);
        let damaged = loaded.is_err();
        let note = loaded.as_ref().err().cloned().unwrap_or_default();
        let mut value = Self {
            root,
            default_skin: home
                .join(".local/horizon-client")
                .join(horizon_core::VANILLA_VERSION)
                .join("assets/minecraft/textures/entity/player/wide/steve.png"),
            library: loaded.unwrap_or_default(),
            key: String::new(),
            provider: None,
            capes: false,
            camera: Default::default(),
            cache: Default::default(),
            worker: None,
            bitmap: image::RgbaImage::new(384, 480),
            bitmap_revision: 0,
            preview_skin: None,
            preview_cape: None,
            preview_model: Model::Classic,
            appearance_generation: 0,
            render_worker: None,
            render_pending: false,
            head_bitmap: vec![],
            rect: [0.0; 4],
            view: [0.0; 3],
            fingers: Default::default(),
            note,
            damaged,
            name: String::new(),
            head: None,
            tab_changed_at: None,
            selection_changed_at: None,
        };
        value.preview();
        value
    }
    pub fn open(&mut self, offline: &str, client_id: &str) {
        let account = Accounts::load().selected().cloned();
        let name = account
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| offline.to_owned());
        let key = account
            .as_ref()
            .map(|a| a.key())
            .unwrap_or_else(|| format!("offline:{}", offline.to_ascii_lowercase()));
        if self.key != key {
            // A previous provider's in-flight result must not delay this profile's cloud load.
            self.worker = None;
            self.fingers.clear();
            self.key = key.clone();
            self.provider = account.map(|a| a.provider);
            self.cache = Default::default();
            // The last head is on disk: show it on the first frame, then let
            // the local selection or cloud sync replace it.
            self.head = self.read_head(&key);
        }
        self.name = name;
        if let Some(head) = self.head {
            skin_head::remember(&self.name, head);
        }
        self.reload();
        if self.provider.is_some() && self.worker.is_none() {
            self.cloud(client_id, None);
        }
        self.preview();
    }
    pub fn reload(&mut self) {
        match Library::load(&self.root) {
            Ok(value) => {
                self.library = value;
                self.damaged = false;
                self.preview();
            }
            Err(error) => {
                self.damaged = true;
                self.note = error;
            }
        }
    }
    pub fn poll(&mut self) {
        if let Some((key, result)) = self.worker.as_ref().and_then(|r| r.try_recv().ok()) {
            self.worker = None;
            if key != self.key {
                return;
            }
            match result {
                Ok((cache, activated_cape)) => {
                    let selection = self.library.selections.entry(self.key.clone()).or_default();
                    if selection.skin.is_none() {
                        selection.model = if cache.slim {
                            Model::Slim
                        } else {
                            Model::Classic
                        };
                    }
                    if activated_cape {
                        selection.cape = None;
                        selection.hide_cape = false;
                        if let Err(error) = self.library.save(&self.root) {
                            self.note = error;
                            self.preview();
                            return;
                        }
                    }
                    self.cache = cache;
                    self.note = if activated_cape {
                        "Cape equipped on client and Microsoft profile (visible to all players)."
                            .into()
                    } else {
                        "Cloud profile synchronized (visible to all players).".into()
                    };
                    self.preview();
                }
                Err(error) => self.note = error,
            }
        }
    }
    fn kind(&self) -> Kind {
        if self.capes { Kind::Cape } else { Kind::Skin }
    }
    fn save(&mut self) {
        if let Err(error) = self.library.save(&self.root) {
            self.note = error;
        }
        self.preview();
    }
    pub fn select_and_sync(&mut self, index: usize, client_id: &str) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        let Some(entry) = self.library.entries.get(index).cloned() else {
            return;
        };
        match self
            .library
            .select(&self.root, &self.key, &entry.id, entry.kind)
        {
            Ok(()) => {
                self.selection_changed_at = Some(std::time::Instant::now());
                self.preview();
                if entry.kind == Kind::Skin && self.provider == Some(Provider::Microsoft) {
                    self.cloud(client_id, Some(CloudAction::Upload));
                } else if entry.kind == Kind::Skin && self.provider == Some(Provider::Ely) {
                    self.note =
                        "Equipped locally. Ely.by server skins are managed at ely.by/skins."
                            .into();
                } else if entry.kind == Kind::Cape && self.provider == Some(Provider::Microsoft) {
                    self.note =
                        "Custom cape applied locally (official Microsoft capes above sync to servers)."
                            .into();
                } else {
                    self.note = "Applied locally.".into();
                }
            }
            Err(error) => self.note = error,
        }
    }
    pub fn import(&mut self) {
        if self.damaged {
            return;
        }
        self.note = match crate::documents::begin_appearance(self.root.clone(), self.kind()) {
            Ok(()) => "Choose a PNG image in Android Files.".into(),
            Err(error) => error,
        };
    }
    pub fn use_cloud(&mut self) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        let selection = self.library.selections.entry(self.key.clone()).or_default();
        if self.capes {
            selection.cape = None;
            selection.hide_cape = false;
        } else {
            selection.skin = None;
        }
        self.note = if self.provider.is_some() {
            "Using the account's cloud appearance. No cloud changes were made."
        } else {
            "Local override cleared. Using the default appearance."
        }
        .into();
        self.save();
    }
    pub fn model(&mut self) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        let selection = self.library.selections.entry(self.key.clone()).or_default();
        selection.model = if selection.model == Model::Classic {
            Model::Slim
        } else {
            Model::Classic
        };
        self.save();
    }
    pub fn model_and_sync(&mut self, client_id: &str) {
        self.model();
        let has_local_skin = self
            .library
            .selections
            .get(&self.key)
            .is_some_and(|s| s.skin.is_some());
        if has_local_skin && self.provider == Some(Provider::Microsoft) {
            self.cloud(client_id, Some(CloudAction::Upload));
        }
    }
    pub fn hide_cape(&mut self) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        let selection = self.library.selections.entry(self.key.clone()).or_default();
        selection.hide_cape = !selection.hide_cape;
        self.save();
    }
    pub fn hide_cape_and_sync(&mut self, client_id: &str) {
        if self.damaged || crate::documents::busy() {
            return;
        }
        self.hide_cape();
        let hidden = self
            .library
            .selections
            .get(&self.key)
            .is_some_and(|s| s.hide_cape);
        if self.provider == Some(Provider::Microsoft) {
            if hidden {
                self.cloud(client_id, Some(CloudAction::DeactivateCape));
            } else if let Some(idx) = self
                .cache
                .snapshot
                .capes
                .iter()
                .position(|c| c.active)
                .or_else(|| (!self.cache.snapshot.capes.is_empty()).then_some(0))
            {
                self.cloud(client_id, Some(CloudAction::Cape(idx)));
            }
        }
    }
    pub fn cloud(&mut self, client_id: &str, action: Option<CloudAction>) {
        if self.worker.is_some() {
            if action.is_none() {
                return;
            }
            self.worker = None;
        }
        let Some(provider) = self.provider else {
            self.note = "Offline appearance is local to this installation.".into();
            return;
        };
        if provider == Provider::Ely && action.is_some() {
            self.note =
                "Ely.by cloud skins are managed at ely.by. Custom capes can be applied locally."
                    .into();
            if let Err(error) = crate::platform::open_login_url("https://ely.by/skins") {
                self.note = error;
            }
            return;
        }
        let key = self.key.clone();
        let root = self.root.clone();
        let id = client_id.to_owned();
        let selection = self
            .library
            .selections
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let cape = match &action {
            Some(CloudAction::Cape(index)) => {
                self.cache.snapshot.capes.get(*index).map(|c| c.id.clone())
            }
            _ => None,
        };
        if matches!(action, Some(CloudAction::Upload)) && selection.skin.is_none() {
            self.note = "Select a local skin before uploading to your Microsoft profile.".into();
            return;
        }
        if matches!(action, Some(CloudAction::Cape(_))) && cape.is_none() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.worker = Some(receiver);
        self.note = match action {
            Some(CloudAction::Upload) => "Equipping skin on client & Microsoft server profile...".into(),
            Some(CloudAction::Cape(_)) => "Equipping cape on client & Microsoft server profile...".into(),
            Some(CloudAction::DeactivateCape) => "Hiding cape on client & Microsoft server profile...".into(),
            None => "Contacting account service...".into(),
        };
        std::thread::spawn(move || {
            let result = (|| {
                let (account, secrets) = Login::session(&id)?
                    .ok_or("Sign in before synchronizing your cloud appearance.")?;
                if account.key() != key {
                    return Err("The selected account changed. Open the gallery again.".into());
                }
                match action {
                    Some(CloudAction::Upload) => {
                        let path = Library::image(
                            &root,
                            selection.skin.as_deref().ok_or("Select a skin first.")?,
                        )?;
                        cloud::upload_skin(
                            &account,
                            &secrets,
                            &skins::read_file(&path)?,
                            selection.model,
                        )?;
                    }
                    Some(CloudAction::Cape(_)) => {
                        cloud::activate_cape(&account, &secrets, cape.as_deref())?
                    }
                    Some(CloudAction::DeactivateCape) => {
                        cloud::activate_cape(&account, &secrets, None)?
                    }
                    None => {}
                }
                cloud::fetch(&account, &secrets, &root)
                    .map(|cache| (cache, matches!(action, Some(CloudAction::Cape(_)))))
            })();
            let _ = sender.send((key, result));
        });
    }
    pub fn prepare_game(&mut self, offline: &str, game: &Path) -> Result<(), String> {
        let key = Accounts::load()
            .selected()
            .map(|a| a.key())
            .unwrap_or_else(|| format!("offline:{}", offline.to_ascii_lowercase()));
        let cloud = if key == self.key {
            self.cache
                .skin
                .as_deref()
                .map(|p| (p, self.cache.slim, self.cache.cape.as_deref()))
        } else {
            None
        };
        Library::load(&self.root)?.prepare_game(&self.root, &key, game, cloud)
    }
    fn head_path(&self, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .take(96)
            .collect();
        self.root.join("heads").join(format!("{safe}.json"))
    }

    fn read_head(&self, key: &str) -> Option<HeadSkin> {
        let bytes = std::fs::read(self.head_path(key)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Keep the header head in memory and on disk, so the next launch paints
    /// the real face on its first frame without opening Skins first.
    fn set_head(&mut self, head: Option<HeadSkin>) {
        if self.head == head {
            return;
        }
        self.head = head;
        let Some(head) = head else {
            return;
        };
        if !self.name.is_empty() {
            skin_head::remember(&self.name, head);
        }
        if self.key.is_empty() {
            return;
        }
        let path = self.head_path(&self.key);
        if let Some(dir) = path.parent()
            && std::fs::create_dir_all(dir).is_err()
        {
            return;
        }
        if let Ok(bytes) = serde_json::to_vec(&head) {
            let _ = std::fs::write(path, bytes);
        }
    }

    pub fn preview(&mut self) {
        let selection = self
            .library
            .selections
            .get(&self.key)
            .cloned()
            .unwrap_or_default();
        let root = self.root.clone();
        let path =
            |id: &Option<String>| id.as_deref().and_then(|id| Library::image(&root, id).ok());
        let chosen = path(&selection.skin)
            .or_else(|| self.cache.skin.clone())
            .and_then(|p| skins::read_file(&p).ok())
            .and_then(|b| skins::read_skin(&b).ok());
        let skin = chosen
            .clone()
            .or_else(|| {
                let bytes = skins::read_file(&self.default_skin).ok()?;
                skins::read_skin(&bytes).ok()
            })
            .unwrap_or_else(preview::default_steve);
        // A real selection owns the head; with none, keep the cached cloud
        // head until the cloud fetch answers, rather than flashing Steve.
        if let Some(image) = &chosen {
            self.set_head(skin_head::from_rgba(
                image.width(),
                image.height(),
                image.as_raw(),
            ));
        } else if self.head.is_none() {
            self.set_head(skin_head::from_rgba(
                skin.width(),
                skin.height(),
                skin.as_raw(),
            ));
        }
        let cape = if selection.hide_cape {
            None
        } else {
            path(&selection.cape)
                .or_else(|| self.cache.cape.clone())
                .and_then(|p| skins::read_file(&p).ok())
                .and_then(|b| skins::read_cape(&b).ok())
        };
        let model = selection.model;
        let head = preview::render_head(&skin, 48);
        self.head_bitmap.clear();
        for y in 0..head.height() {
            let mut x = 0;
            while x < head.width() {
                let [r, g, b, a] = head.get_pixel(x, y).0;
                let start = x;
                x += 1;
                while x < head.width() && head.get_pixel(x, y).0 == [r, g, b, a] { x += 1; }
                if a > 0 { self.head_bitmap.push(([start as f32, y as f32, (x - start) as f32, 1.0], rgba(r, g, b, a))); }
            }
        }
        // Rasterize at more than twice the former dimensions. The skin's own
        // Minecraft texels remain sharp; model edges no longer use large quads.
        // Invalidate any in-flight render from the previous appearance.
        self.appearance_generation = self.appearance_generation.wrapping_add(1);
        self.render_pending = false;
        self.preview_skin = Some(std::sync::Arc::new(skin));
        self.preview_cape = cape.map(std::sync::Arc::new);
        self.preview_model = model;
        self.bitmap = preview::render(self.preview_skin.as_ref().unwrap(), self.preview_cape.as_deref(), model, self.camera, [384, 480]);
        self.bitmap_revision = self.bitmap_revision.wrapping_add(1);
    }
    // One worker at a time; movements are coalesced to the most recent camera.
    // No filesystem reads, skin decode or head reconstruction on pointer events.
    fn queue_camera_render(&mut self) {
        self.render_pending = true;
        self.start_camera_render();
    }
    fn start_camera_render(&mut self) {
        if self.render_worker.is_some() || !self.render_pending { return; }
        let Some(skin) = self.preview_skin.clone() else { return; };
        let cape = self.preview_cape.clone();
        let model = self.preview_model;
        let camera = self.camera;
        let generation = self.appearance_generation;
        self.render_pending = false;
        let (tx, rx) = mpsc::channel();
        self.render_worker = Some(rx);
        std::thread::spawn(move || {
            let image = preview::render(&skin, cape.as_deref(), model, camera, [384, 480]);
            let _ = tx.send((generation, image));
        });
    }
    fn poll_camera_render(&mut self) {
        let result = self.render_worker.as_ref().and_then(|worker| match worker.try_recv() {
            Ok(result) => Some(Some(result)),
            Err(mpsc::TryRecvError::Disconnected) => Some(None),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.render_worker = None;
            if let Some((generation, image)) = result {
                if generation == self.appearance_generation {
                    self.bitmap = image;
                    self.bitmap_revision = self.bitmap_revision.wrapping_add(1);
                }
            }
            self.start_camera_render();
        }
    }
    pub fn is_animating(&self) -> bool {
        let now = std::time::Instant::now();
        self.worker.is_some()
            || self.render_worker.is_some() || self.render_pending
            || crate::documents::busy()
            || self
                .tab_changed_at
                .is_some_and(|t| now.duration_since(t).as_secs_f32() < 0.22)
            || self
                .selection_changed_at
                .is_some_and(|t| now.duration_since(t).as_secs_f32() < 0.22)
    }

    pub fn set_tab(&mut self, capes: bool) {
        if self.capes != capes {
            self.capes = capes;
            self.tab_changed_at = Some(std::time::Instant::now());
            // Orient camera to showcase the back when switching to Capes, or front when switching to Skins
            if capes && (self.camera.yaw < 70.0 || self.camera.yaw > 290.0) {
                self.camera.yaw = 155.0;
                self.camera.pitch = -6.0;
            } else if !capes && (self.camera.yaw >= 90.0 && self.camera.yaw <= 270.0) {
                self.camera.yaw = 335.0;
                self.camera.pitch = -8.0;
            }
            self.preview();
        }
    }

    /// The account head and full-body preview use the identical selected skin.
    pub fn draw_head(&self, ui: &mut Ui, x: f32, y: f32, size: f32) {
        let scale = size / 48.0;
        for ([bx, by, bw, bh], color) in &self.head_bitmap {
            ui.rounded(x + bx * scale, y + by * scale, bw * scale, bh * scale, 0.0, *color);
        }
    }

    /// The same applied-skin preview used by the wardrobe, placed in the shared scene.
    /// Keeping the hit rectangle here makes drag/pinch orbit work on the dashboard too.
    pub fn draw_scene(&mut self, ui: &mut Ui, rect: [f32; 4]) {
        self.poll_camera_render();
        self.rect = rect;
        let [x, y, w, h] = rect;
        let scale = (w / 160.0).min(h / 200.0);
        let ox = x + (w - 160.0 * scale) * 0.5;
        let oy = y + (h - 200.0 * scale) * 0.5;
        ui.rounded(x + w * 0.23, y + h * 0.91, w * 0.54, 6.0, 3.0,
            horizon_client::hud::rgba(8, 10, 20, 100));
        if !ui.scene_avatar(self.bitmap.width(), self.bitmap.height(), self.bitmap.as_raw(), self.bitmap_revision,
            ox, oy, 160.0 * scale, 200.0 * scale) {
            // Small boot atlases have no room for the reserved slot.
            for y in (0..self.bitmap.height()).step_by(3) {
                for x in (0..self.bitmap.width()).step_by(3) {
                    let [r, g, b, a] = self.bitmap.get_pixel(x, y).0;
                    if a > 0 { ui.rounded(ox + x as f32 / 2.4 * scale, oy + y as f32 / 2.4 * scale,
                        1.25 * scale, 1.25 * scale, 0.0, rgba(r, g, b, a)); }
                }
            }
        }
    }

    pub fn gesture(&mut self, touch: &winit::event::Touch, density: f32) -> bool {
        use winit::event::TouchPhase;
        let point = [
            touch.location.x as f32 / density,
            touch.location.y as f32 / density,
        ];
        if touch.phase == TouchPhase::Started {
            let [x, y, w, h] = self.rect;
            if point[0] >= x && point[0] < x + w && point[1] >= y && point[1] < y + h {
                self.fingers.insert(touch.id, point);
                return true;
            }
            return false;
        }
        let Some(old) = self.fingers.get(&touch.id).copied() else {
            return false;
        };
        if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            self.fingers.remove(&touch.id);
            return true;
        }
        if let Some(other) = self
            .fingers
            .iter()
            .find_map(|(id, p)| (*id != touch.id).then_some(*p))
        {
            let distance =
                |p: [f32; 2]| ((p[0] - other[0]).powi(2) + (p[1] - other[1]).powi(2)).sqrt();
            let before = distance(old);
            if before > 1.0 {
                self.camera.zoom(distance(point) / before);
            }
        } else {
            // Horizontal & vertical drag matches user finger direction in both front and back views
            self.camera.drag(point[0] - old[0], point[1] - old[1]);
        }
        self.fingers.insert(touch.id, point);
        self.queue_camera_render();
        true
    }

    pub fn draw(
        &mut self,
        ui: &mut Ui,
        targets: &mut Vec<([f32; 4], Action)>,
        scroll: f32,
        x: f32,
        w: f32,
        top: f32,
    ) {
        use horizon_client::ui_theme::{Icon, active_theme, motion};
        let theme = active_theme();
        let now = std::time::Instant::now();
        let tab_t = motion::spring_expressive_default(motion::progress(self.tab_changed_at, now, 0.24));
        let slide_y = (1.0 - tab_t) * 8.0;

        let h = ui.h;

        // 1. Header & Segmented Skins / Capes Switcher
        ui.text(
            Face::SemiBold,
            19.0,
            x,
            top,
            theme.text_primary,
            "Skins & Capes Studio",
        );
        let subtitle = ui.fit_text(
            Face::Regular,
            11.5,
            if self.capes {
                "Preview 3D capes, manage local cape overrides, or activate cloud account capes."
            } else {
                "Inspect your 3D character model, switch arm styles, and manage local or cloud skins."
            },
            w,
        );
        ui.text(
            Face::Regular,
            11.5,
            x,
            top + 24.0,
            theme.text_muted,
            &subtitle,
        );

        let skin_count = self
            .library
            .entries
            .iter()
            .filter(|e| e.kind == Kind::Skin)
            .count();
        let cape_count = self
            .library
            .entries
            .iter()
            .filter(|e| e.kind == Kind::Cape)
            .count()
            + if self.provider.is_some() {
                self.cache.snapshot.capes.len()
            } else {
                0
            };

        let tabs_y = top + 46.0;
        ui.rounded(x, tabs_y, w, 38.0, 8.0, theme.surface_dim);
        ui.border(x, tabs_y, w, 38.0, 8.0, theme.border);
        let tw = (w - 8.0) * 0.5;
        for (i, (lbl, is_cape, active)) in [
            (format!("Skins ({skin_count})"), false, !self.capes),
            (format!("Capes ({cape_count})"), true, self.capes),
        ]
        .into_iter()
        .enumerate()
        {
            let tx = x + 3.0 + i as f32 * (tw + 2.0);
            let tr = [tx, tabs_y + 3.0, tw, 32.0];
            if active {
                ui.rounded(tr[0], tr[1], tr[2], tr[3], 6.0, theme.accent);
                ui.centered_fit(Face::SemiBold, 12.0, tr, theme.on_accent, &lbl);
            } else {
                ui.centered_fit(Face::Medium, 12.0, tr, theme.text_muted, &lbl);
            }
            targets.push((tr, Action::WardrobeKind(is_cape)));
        }

        let selection = self
            .library
            .selections
            .get(&self.key)
            .cloned()
            .unwrap_or_default();
        let selected_id = if self.capes {
            selection.cape.as_deref()
        } else {
            selection.skin.as_deref()
        };
        let model = selection.model;

        // The portrait is shared with the dashboard, rather than duplicated in
        // this panel. The same camera and selected skin drive both views.
        let scene_x = 12.0;
        let scene_w = (x - 30.0).max(100.0);
        let controls_y = h - 79.0;
        let button_w = 38.0;
        for (i, (label, action)) in [("<", Action::WardrobeRotate(-1)), (">", Action::WardrobeRotate(1))].into_iter().enumerate() {
            button(ui, targets, [scene_x + scene_w * 0.5 - 41.0 + i as f32 * 44.0, controls_y, button_w, 28.0], label, action, false);
        }
        button(ui, targets, [scene_x, h - 45.0, scene_w, 30.0],
            if self.capes {
                if selection.hide_cape { "Cape hidden" } else { "Cape visible" }
            } else if model == Model::Slim { "Arm Model: Slim (3px)" } else { "Arm Model: Classic (4px)" },
            if self.capes { Action::WardrobeHideCape } else { Action::WardrobeModel }, false);
        let lx = x;
        let lw = w;
        let listtop = tabs_y + 48.0 + slide_y;

        // Active Appearance Details & Primary Action Bar
        let active_title = if let Some(id) = selected_id {
            self.library
                .entries
                .iter()
                .find(|e| e.id == id && e.kind == self.kind())
                .map(|e| format!("Active Local: {}", e.title))
                .unwrap_or_else(|| "Active Local Override".into())
        } else if self.capes {
            if selection.hide_cape {
                "Cape Hidden Locally".into()
            } else if let Some(c) = self.cache.snapshot.capes.iter().find(|c| c.active) {
                format!("Active Cloud Cape: {}", c.title)
            } else if self.provider.is_some() {
                "Account Cloud Cape".into()
            } else {
                "No Cape Equipped".into()
            }
        } else if self.provider.is_some() {
            format!("Cloud Profile ({})", self.name)
        } else {
            "Default Appearance (Steve / Classic)".into()
        };

        let detail_h = 46.0;
        ui.rounded(lx, listtop, lw, detail_h, 12.0, theme.surface);
        ui.border(lx, listtop, lw, detail_h, 12.0, theme.border);
        let import_w = (lw * 0.42).clamp(118.0, 168.0);
        let info_w = (lw - import_w - 24.0).max(60.0);
        let fitted_active = ui.fit_text(Face::SemiBold, 11.5, &active_title, info_w);
        ui.text(
            Face::SemiBold,
            11.5,
            lx + 12.0,
            listtop + 6.0,
            theme.text_primary,
            &fitted_active,
        );
        let detail_sub = if self.worker.is_some() {
            "Synchronizing with account service..."
        } else if crate::documents::busy() {
            "Waiting for Android file picker..."
        } else if self.capes {
            "Import a 64×32 PNG cape or select below"
        } else {
            "Import a 64×64 PNG skin or select below"
        };
        let fitted_sub = ui.fit_text(Face::Regular, 9.5, detail_sub, info_w);
        ui.text(
            Face::Regular,
            9.5,
            lx + 12.0,
            listtop + 24.0,
            if self.worker.is_some() {
                theme.accent
            } else {
                theme.text_muted
            },
            &fitted_sub,
        );
        button(
            ui,
            targets,
            [lx + lw - import_w - 8.0, listtop + 7.0, import_w, 32.0],
            if self.capes {
                "+ Import Cape PNG"
            } else {
                "+ Import Skin PNG"
            },
            Action::WardrobeImport,
            true,
        );

        let list_start = listtop + detail_h + 10.0;
        let bottom = h - 42.0;
        let mut row = list_start - scroll;
        let card_h = 42.0;
        let card_step = 48.0;

        let draw_gallery_item = |ui: &mut Ui,
                                 targets: &mut Vec<([f32; 4], Action)>,
                                 ry: f32,
                                 title: &str,
                                 badge: &str,
                                 is_selected: bool,
                                 action: Action| {
            let r = [lx, ry, lw, card_h];
            ui.rounded(
                r[0],
                r[1],
                r[2],
                r[3],
                11.0,
                if is_selected {
                    theme.surface_selected
                } else {
                    theme.surface
                },
            );
            ui.stroke_rounded(
                r[0],
                r[1],
                r[2],
                r[3],
                11.0,
                if is_selected { 1.6 } else { 1.0 },
                if is_selected {
                    theme.accent
                } else {
                    theme.border
                },
            );
            // Leading icon badge
            let ib = [r[0] + 10.0, r[1] + 8.0, 26.0, 26.0];
            ui.rounded(
                ib[0],
                ib[1],
                ib[2],
                ib[3],
                8.0,
                if is_selected {
                    theme.accent
                } else {
                    theme.surface_dim
                },
            );
            if is_selected {
                ui.icon(Icon::Check, ib[0] + 5.0, ib[1] + 5.0, 16.0, theme.on_accent);
            } else {
                ui.icon(
                    Icon::Installed,
                    ib[0] + 5.0,
                    ib[1] + 5.0,
                    16.0,
                    theme.text_muted,
                );
            }
            let max_tw = (lw - 132.0).max(60.0);
            let t_fit = ui.fit_text(Face::SemiBold, 12.0, title, max_tw);
            ui.text(
                Face::SemiBold,
                12.0,
                r[0] + 44.0,
                r[1] + 6.0,
                theme.text_primary,
                &t_fit,
            );
            ui.text(
                Face::Regular,
                9.5,
                r[0] + 44.0,
                r[1] + 23.0,
                if is_selected {
                    theme.accent
                } else {
                    theme.text_muted
                },
                badge,
            );

            let pill_txt = if is_selected { "Active" } else { "Apply" };
            let pill_w = 64.0;
            let pr = [r[0] + lw - pill_w - 10.0, r[1] + 8.0, pill_w, 26.0];
            ui.rounded(
                pr[0],
                pr[1],
                pr[2],
                pr[3],
                8.0,
                if is_selected {
                    theme.accent
                } else {
                    theme.surface_dim
                },
            );
            ui.centered_fit(
                Face::SemiBold,
                10.5,
                pr,
                if is_selected {
                    theme.on_accent
                } else {
                    theme.text_primary
                },
                pill_txt,
            );
            targets.push((r, action));
        };

        let mut shown_items = 0usize;

        // Cloud Capes (when on Capes tab and signed into an account)
        if self.capes && self.provider.is_some() {
            for (index, cape) in self.cache.snapshot.capes.iter().enumerate() {
                shown_items += 1;
                if row >= list_start - 4.0 && row + card_h <= bottom + 4.0 {
                    let is_active = cape.active && selected_id.is_none() && !selection.hide_cape;
                    draw_gallery_item(
                        ui,
                        targets,
                        row,
                        &cape.title,
                        if is_active {
                            "Cloud Cape · Equipped on profile"
                        } else {
                            "Cloud Cape · Tap to activate"
                        },
                        is_active,
                        Action::WardrobeActivateCape(index),
                    );
                }
                row += card_step;
            }
        }

        // Local Gallery Items (Skins or Capes)
        for (index, entry) in self
            .library
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.kind == self.kind())
        {
            shown_items += 1;
            if row >= list_start - 4.0 && row + card_h <= bottom + 4.0 {
                let is_sel = selected_id == Some(entry.id.as_str());
                draw_gallery_item(
                    ui,
                    targets,
                    row,
                    &entry.title,
                    if is_sel {
                        "Local Gallery · Currently applied"
                    } else if self.capes {
                        "Local Cape PNG · Tap to equip"
                    } else {
                        "Local Skin PNG · Tap to equip"
                    },
                    is_sel,
                    Action::WardrobeSelect(index),
                );
            }
            row += card_step;
        }

        // Empty State Card when gallery has no items for this tab
        if shown_items == 0 {
            if row >= list_start - 4.0 && row + 64.0 <= bottom + 8.0 {
                ui.rounded(lx, row, lw, 60.0, 12.0, theme.surface_dim);
                ui.border(lx, row, lw, 60.0, 12.0, theme.border);
                ui.text(
                    Face::SemiBold,
                    12.0,
                    lx + 14.0,
                    row + 10.0,
                    theme.text_primary,
                    if self.capes {
                        "No custom capes in your gallery yet"
                    } else {
                        "No custom skins in your gallery yet"
                    },
                );
                let empty_sub = ui.fit_text(
                    Face::Regular,
                    10.0,
                    if self.capes {
                        "Tap '+ Import Cape PNG' above to add a 64×32 cape texture from your device."
                    } else {
                        "Tap '+ Import Skin PNG' above to add a 64×64 skin texture from your device."
                    },
                    lw - 28.0,
                );
                ui.text(
                    Face::Regular,
                    10.0,
                    lx + 14.0,
                    row + 31.0,
                    theme.text_muted,
                    &empty_sub,
                );
            }
            row += 68.0;
        }

        // Account & Reset Action Buttons
        if self.provider.is_none() {
            if row >= list_start - 4.0 && row + 34.0 <= bottom + 4.0 {
                button(
                    ui,
                    targets,
                    [lx, row, lw, 34.0],
                    if self.capes {
                        "Clear Local Cape Override"
                    } else {
                        "Reset to Default Appearance Locally"
                    },
                    Action::WardrobeUseCloud,
                    false,
                );
            }
            row += 42.0;
        } else {
            let half_w = (lw - 8.0) * 0.5;
            if row >= list_start - 4.0 && row + 34.0 <= bottom + 4.0 {
                button(
                    ui,
                    targets,
                    [lx, row, half_w, 34.0],
                    "Use Cloud Appearance",
                    Action::WardrobeUseCloud,
                    false,
                );
                button(
                    ui,
                    targets,
                    [lx + half_w + 8.0, row, half_w, 34.0],
                    if self.worker.is_some() {
                        "Syncing Cloud..."
                    } else {
                        "Refresh Cloud Profile"
                    },
                    Action::WardrobeCloud,
                    false,
                );
            }
            row += 42.0;
            if !self.capes {
                if row >= list_start - 4.0 && row + 34.0 <= bottom + 4.0 {
                    button(
                        ui,
                        targets,
                        [lx, row, lw, 34.0],
                        if self.provider == Some(Provider::Microsoft) {
                            "Upload Selected Skin to Microsoft Profile"
                        } else {
                            "Manage Ely.by Cloud Skin on Website"
                        },
                        Action::WardrobeUpload,
                        false,
                    );
                }
                row += 42.0;
            }
        }

        self.view = [list_start, bottom, (row + scroll + 38.0 - bottom).max(0.0)];

        // Bottom status / feedback bar
        if !self.note.is_empty() {
            let banner_y = h - 36.0;
            ui.rounded(x, banner_y, w, 28.0, 8.0, theme.surface_dim);
            let note_col = if self.damaged {
                theme.danger
            } else {
                theme.text_muted
            };
            let note = ui.fit_text(Face::Regular, 10.8, &self.note, w - 20.0);
            ui.text(
                Face::Regular,
                10.8,
                x + 10.0,
                banner_y + 6.0,
                note_col,
                &note,
            );
        }
    }
}
