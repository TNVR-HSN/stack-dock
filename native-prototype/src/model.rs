use serde::Deserialize;
use std::{collections::HashSet, fs::File, io::Read, path::Path};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn right(self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(self) -> f32 {
        self.y + self.h
    }
    pub fn cx(self) -> f32 {
        self.x + self.w / 2.
    }
    pub fn cy(self) -> f32 {
        self.y + self.h / 2.
    }
    pub fn inset(self, p: f32) -> Self {
        Self::new(self.x + p, self.y + p, self.w - p * 2., self.h - p * 2.)
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }
    pub fn union(self, b: Self) -> Self {
        let x = self.x.min(b.x);
        let y = self.y.min(b.y);
        Self::new(
            x,
            y,
            self.right().max(b.right()) - x,
            self.bottom().max(b.bottom()) - y,
        )
    }
    pub fn intersect(self, b: Self) -> Self {
        let x = self.x.max(b.x);
        let y = self.y.max(b.y);
        Self::new(
            x,
            y,
            (self.right().min(b.right()) - x).max(0.),
            (self.bottom().min(b.bottom()) - y).max(0.),
        )
    }
    pub fn lerp(self, b: Self, x: f32, y: f32) -> Self {
        Self::new(
            mix(self.x, b.x, x),
            mix(self.y, b.y, y),
            mix(self.w, b.w, x),
            mix(self.h, b.h, y),
        )
    }
    pub fn square(self, size: f32) -> Self {
        Self::new(self.cx() - size / 2., self.cy() - size / 2., size, size)
    }
}
pub fn mix(a: f32, b: f32, p: f32) -> f32 {
    a + (b - a) * p
}
pub fn bounded(p: f32) -> f32 {
    p.clamp(0., 1.)
}
pub fn ease(p: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    if p <= 0. || p >= 1. {
        return bounded(p);
    }
    let bez = |t: f32, u: f32, v: f32| {
        3. * (1. - t) * (1. - t) * t * u + 3. * (1. - t) * t * t * v + t * t * t
    };
    let (mut lo, mut hi) = (0., 1.);
    for _ in 0..15 {
        let t = (lo + hi) / 2.;
        if bez(t, a, c) < p { lo = t } else { hi = t }
    }
    bez((lo + hi) / 2., b, d)
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Style {
    settings_gear: Option<bool>,
    icon: Option<f32>,
    color: Option<String>,
    opacity: Option<f32>,
    border: Option<bool>,
    border_color: Option<String>,
    border_opacity: Option<f32>,
    border_width: Option<f32>,
    radius: Option<f32>,
    gap: Option<f32>,
    pad: Option<f32>,
    gap_split: bool,
    pad_split: bool,
    gap_x: Option<f32>,
    gap_y: Option<f32>,
    pad_x: Option<f32>,
    pad_y: Option<f32>,
    pub labels: Option<bool>,
    label_font: Option<f32>,
    label_width: Option<f32>,
    title_icon: Option<f32>,
    round_icons: Option<bool>,
    icon_corner: Option<f32>,
    icon_rotation: Option<f32>,
    shadow: Option<bool>,
    #[serde(flatten)]
    pub launch: LaunchStyle,
    pub hit_expand:Option<bool>,
    pub hit_padding:Option<f32>,
    pub hover_labels:Option<bool>,
    pub hover_label_background:Option<bool>,
    pub hover_label_size:Option<f32>,
    pub custom_hover_delay:Option<bool>,
    pub hover_details_delay:Option<u64>,
    pub motion: Option<bool>,
    pub motion_speed: Option<f32>,
    hover_mode: Option<u8>,
    hover_scale: Option<f32>,
    hover_lift: Option<f32>,
    hover_angle: Option<f32>,
    hover_duration: Option<f32>,
    hover_curve: Option<u8>,
    highlight_enabled: Option<bool>,
    highlight_soft: Option<bool>,
    highlight_area: Option<f32>,
    highlight_fill: Option<f32>,
    highlight_corner: Option<f32>,
    hover_opacity: Option<f32>,
    ground_shadow: Option<bool>,
    ground_shadow_intensity: Option<f32>,
    ground_shadow_offset: Option<f32>,
    folder_arrow: Option<bool>,
    folder_arrow_x: Option<f32>,
    folder_arrow_y: Option<f32>,
    folder_arrow_rotation: Option<f32>,
    reflections: Option<bool>,
    reflection_area: Option<f32>,
    reflection_intensity: Option<f32>,
}
#[derive(Clone, Copy, Debug)]
pub struct Skin {
    pub icon: f32,
    pub inset_shadow: f32,
    pub fill: u32,
    pub border: u32,
    pub stroke: f32,
    pub radius: f32,
    pub gx: f32,
    pub gy: f32,
    pub px: f32,
    pub py: f32,
    pub labels: bool,
    pub font: f32,
    pub label_w: f32,
    pub title: f32,
    pub corner: f32,
    pub rotation: f32,
    pub hover_mode: u8,
    pub hover_scale: f32,
    pub hover_lift: f32,
    pub hover_angle: f32,
    pub hover_duration: f32,
    pub hover_curve: u8,
    pub reflect: f32,
    pub reflect_alpha: f32,
    pub highlight_soft: bool,
    pub highlight_area: f32,
    pub highlight_fill: f32,
    pub highlight_corner: Option<f32>,
    pub highlight_alpha: f32,
    pub ground_shadow_alpha: f32,
    pub ground_shadow_y: f32,
    pub ground_shadow_height: f32,
    folder_arrow: ArrowOptions,
}
impl Skin {
    pub fn effects_height(self) -> f32 {
        let shadow = if self.ground_shadow_alpha > 0. {
            self.ground_shadow_y + self.ground_shadow_height
        } else {
            0.
        };
        self.reflect.max(shadow)
    }

    pub fn folder_arrow(self, entry: &Entry) -> Option<(f32, f32, f32)> {
        let own = entry.arrow_override.unwrap_or_default();
        if !entry.folder() || !own.enabled.or(self.folder_arrow.enabled).unwrap_or(false) {
            return None;
        }
        let x = own.x.or(self.folder_arrow.x).unwrap_or(84.).clamp(0., 100.) / 100.;
        let y = own.y.or(self.folder_arrow.y).unwrap_or(16.).clamp(0., 100.) / 100.;
        let rotation = own.rotation.or(self.folder_arrow.rotation).unwrap_or(0.);
        Some((x, y, rotation))
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
struct ArrowOptions {
    enabled: Option<bool>,
    x: Option<f32>,
    y: Option<f32>,
    rotation: Option<f32>,
}
impl Skin {
    pub fn dock_entry_dims(self, entry: &Entry, scale: f32) -> (f32, f32) {
        let width = if entry.trigger_size > 0. {
            entry.trigger_size.clamp(24., 74.) * scale
        } else {
            self.icon
        };
        let label = if self.labels { 17. * scale } else { 0. };
        (width, width + label + self.effects_height())
    }
}

fn hex(s: Option<&str>, fallback: u32) -> u32 {
    s.and_then(|v| v.strip_prefix('#'))
        .filter(|v| v.len() == 6)
        .and_then(|v| u32::from_str_radix(v, 16).ok())
        .unwrap_or(fallback)
}
fn rgba(rgb: u32, pct: f32) -> u32 {
    ((pct.clamp(0., 100.) * 2.55).round() as u32) << 24 | rgb
}
#[derive(Clone,Debug,Default,Deserialize)]
#[serde(default,rename_all="camelCase")]
pub struct LaunchStyle {
    launch_width:Option<f32>,
    launch_height:Option<f32>,
    launch_hold:Option<f32>,
    launch_travel:Option<f32>,
    launch_return:Option<f32>,
    launch_icon:Option<f32>,
    launch_font:Option<f32>,
    launch_radius:Option<f32>,
    launch_opacity:Option<f32>,
    launch_gap:Option<f32>,
    launch_pad:Option<f32>,
    pub launch_enabled:Option<bool>,
    pub launch_label:Option<bool>,
    pub launch_follow_dock:Option<bool>,
    pub launch_color:Option<String>,
    pub launch_text:Option<String>,
}
impl LaunchStyle {
    pub fn width(&self)->f32{ self.launch_width.unwrap_or(300.).clamp(140.,680.) }
    pub fn height(&self)->f32{ self.launch_height.unwrap_or(64.).clamp(42.,120.) }
    pub fn hold(&self)->f32{ self.launch_hold.unwrap_or(1250.).clamp(0.,8000.) }
    pub fn travel(&self)->f32{ self.launch_travel.unwrap_or(300.).clamp(0.,1000.) }
    pub fn return_time(&self)->f32{ self.launch_return.unwrap_or(210.).clamp(0.,700.) }
    pub fn icon(&self)->f32{ self.launch_icon.unwrap_or(42.).clamp(16.,84.) }
    pub fn font(&self)->f32{ self.launch_font.unwrap_or(19.).clamp(11.,34.) }
    pub fn radius(&self)->f32{ self.launch_radius.unwrap_or(32.).clamp(0.,60.) }
    pub fn opacity(&self)->f32{ self.launch_opacity.unwrap_or(65.).clamp(0.,100.) }
    pub fn gap(&self)->f32{ self.launch_gap.unwrap_or(12.).clamp(0.,28.) }
    pub fn pad(&self)->f32{ self.launch_pad.unwrap_or(16.).clamp(4.,28.) }
    pub fn fill(&self,dock:u32)->u32{if self.launch_follow_dock.unwrap_or(false){dock}else{rgba(hex(self.launch_color.as_deref(),0x111a2c),self.opacity())}}
    pub fn ink(&self)->u32{hex(self.launch_text.as_deref(),0xffffff)}
}
impl Style {
    pub fn skin(&self, grid: bool, scale: f32) -> Skin {
        let pick = |g, d| if grid { g } else { d };
        let gap = self.gap.unwrap_or(pick(7., 6.)).clamp(0., 120.);
        let pad = self.pad.unwrap_or(pick(15., 8.)).clamp(0., 160.);
        let axis = |split: bool, v: Option<f32>, def: f32, max: f32| {
            if split {
                v.unwrap_or(def).clamp(0., max) * scale
            } else {
                def * scale
            }
        };
        let icon = self.icon.unwrap_or(pick(52., 40.)).clamp(24., 88.) * scale;
        Skin {
            icon,
            hover_mode: self.hover_mode.unwrap_or(0),
            hover_scale: self.hover_scale.unwrap_or(125.).clamp(100., 200.) / 100.,
            hover_lift: self.hover_lift.unwrap_or(10.).clamp(0., 48.) * scale,
            hover_angle: self.hover_angle.unwrap_or(10.).clamp(-30., 30.),
            hover_duration: self.hover_duration.unwrap_or(160.).clamp(0., 900.),
            hover_curve: self.hover_curve.unwrap_or(1),
            inset_shadow: if self.shadow.unwrap_or(true) {0.33*self.opacity.unwrap_or(pick(80.,76.)).clamp(0.,100.)/100.} else {0.},
            fill: rgba(
                hex(
                    self.color.as_deref(),
                    if grid { 0x0b0b12 } else { 0x101019 },
                ),
                self.opacity.unwrap_or(pick(80., 76.)),
            ),
            border: rgba(
                hex(self.border_color.as_deref(), 0xffffff),
                if self.border.unwrap_or(true) {
                    self.border_opacity.unwrap_or(23.)
                } else {
                    0.
                },
            ),
            stroke: self.border_width.unwrap_or(1.).clamp(1., 5.) * scale,
            radius: self.radius.unwrap_or(pick(22., 26.)).clamp(0., 56.) * scale,
            gx: axis(self.gap_split, self.gap_x, gap, 120.),
            gy: axis(self.gap_split, self.gap_y, gap, 120.),
            px: axis(self.pad_split, self.pad_x, pad, 160.),
            py: axis(self.pad_split, self.pad_y, pad, 160.),
            labels: self.labels.unwrap_or(grid),
            font: self.label_font.unwrap_or(pick(11., 10.)).clamp(9., 18.) * scale,
            label_w: self.label_width.unwrap_or(76.).clamp(64., 200.) * scale,
            title: self.title_icon.unwrap_or(25.).clamp(16., 44.) * scale,
            corner: if self.round_icons == Some(false) {
                0.
            } else {
                self.icon_corner.unwrap_or(50.).clamp(0., 50.) / 100.
            },
            rotation: self.icon_rotation.unwrap_or_default(),
            reflect: if self.reflections.unwrap_or(grid)
                && self.reflection_intensity.unwrap_or(55.) > 0.
            {
                self.reflection_area
                    .unwrap_or(10.)
                    .clamp(0., 64.)
                    .mul_add(scale, 0.)
                    .min(icon)
            } else {
                0.
            },
            reflect_alpha: self.reflection_intensity.unwrap_or(55.).clamp(0., 100.) / 100.,
            highlight_alpha: if self.highlight_enabled.unwrap_or(true) {
                self.hover_opacity.unwrap_or(40.).clamp(0., 100.) / 100.
            } else {
                0.
            },
            highlight_area: (self.highlight_area.unwrap_or(0.).clamp(0., 32.) * scale).round(),
            highlight_fill: self.highlight_fill.unwrap_or(30.).clamp(0., 100.) / 100.,
            highlight_soft: self.highlight_soft.unwrap_or(true),
            highlight_corner: self.highlight_corner.map(|value| value.clamp(0., 50.) / 100.),
            ground_shadow_alpha: if self.ground_shadow.unwrap_or(false) {
                self.ground_shadow_intensity.unwrap_or(65.).clamp(0., 100.) / 100.
            } else {
                0.
            },
            ground_shadow_y: (self.ground_shadow_offset.unwrap_or(0.).clamp(-24., 48.) * scale).round(),
            ground_shadow_height: (8. * scale).round().max(3.),
            folder_arrow: ArrowOptions {
                enabled: self.folder_arrow,
                x: self.folder_arrow_x,
                y: self.folder_arrow_y,
                rotation: self.folder_arrow_rotation,
            },
        }
    }
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub image: String,
    pub icon_path: String,
    pub icon_mode: String,
    pub icon_rotation: Option<f32>,
    pub icon_index: i32,
    pub image_thumbnail: Option<bool>,
    #[serde(rename = "type")]
    pub ty: String,
    pub kind: String,
    pub local_type: String,
    pub target: String,
    pub args: String,
    pub cwd: String,
    pub r: usize,
    pub c: usize,
    pub cols: usize,
    pub rows: usize,
    pub slots: Vec<Entry>,
    pub style: Style,
    pub entry_motion: String,
    pub trigger_size: f32,
    pub dock_icon_corner: Option<f32>,
    pub folder_hover_labels:Option<bool>,
    pub dock_icon_shape: String,
    arrow_override: Option<ArrowOptions>,
}
impl Entry {
    pub fn folder(&self) -> bool {
        self.ty != "item"
    }
    pub fn icon_key(&self) -> &str {
        if self.folder() {
            if self.icon.is_empty() {
                "folder-azure"
            } else {
                &self.icon
            }
        } else if !self.icon.is_empty() {
            &self.icon
        } else {
            match self.kind.as_str() {
                "url" | "uri" => "web",
                "program" => "program",
                _ if self.local_type == "folder" => "folder",
                _ => "file",
            }
        }
    }
}
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    pub hover_names:Option<bool>,
    pub hover_targets:Option<bool>,
    pub hover_delay:Option<u64>,
    pub collapse_on_focus_loss: Option<bool>,
    pub image_thumbnails: Option<bool>,
}
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Saved {
    pub edge: String,
    pub align: String,
    pub orientation: String,
    pub general: General,
    pub dock: Style,
    pub folders: Vec<Entry>,
    pub dock_order: Vec<String>,
    pub dock_shortcuts: Vec<Entry>,
}
#[derive(Deserialize)]
struct Export {
    format: String,
    version: u32,
    layout: Saved,
}
pub const STARTER: &str = include_str!("../assets/starter.json");
impl Saved {
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let Export {
            format,
            version,
            mut layout,
        } = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if format != "stack-dock-layout" || version != 1 {
            return Err("Expected a version 1 StackDock layout export.".into());
        }
        if !["top", "bottom", "left", "right"].contains(&layout.edge.as_str())
            || layout.folders.is_empty()
            || layout.folders.len() > 100
            || layout.dock_shortcuts.len() > 160
        {
            return Err("Invalid dock position or number of entries.".into());
        }
        fn check(f: &mut Entry, depth: u8, ids: &mut HashSet<String>) -> bool {
            if depth > 12
                || f.id.is_empty()
                || !ids.insert(f.id.clone())
                || f.name.trim().is_empty()
            {
                return false;
            }
            if !f.folder() {
                return f.slots.is_empty() && !f.target.trim().is_empty();
            }
            if !(1..=12).contains(&f.cols) || !(1..=80).contains(&f.rows) {
                return false;
            }
            f.slots.retain(|v| v.ty != "spacer");
            let mut cells = HashSet::new();
            f.slots.iter_mut().all(|v| {
                v.r < f.rows
                    && v.c < f.cols
                    && cells.insert((v.r, v.c))
                    && (v.ty == "folder" || v.ty == "item")
                    && check(v, depth + 1, ids)
            })
        }
        let mut ids = HashSet::new();
        if !layout
            .folders
            .iter_mut()
            .all(|f| f.folder() && check(f, 1, &mut ids))
            || !layout
                .dock_shortcuts
                .iter_mut()
                .all(|f| !f.folder() && check(f, 1, &mut ids))
        {
            return Err("Invalid folders, shortcut entries, or slot coordinates.".into());
        }
        Ok(layout)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        const MAX: u64 = 8 * 1024 * 1024;
        let mut b = Vec::new();
        File::open(path)
            .and_then(|f| f.take(MAX + 1).read_to_end(&mut b))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if b.len() as u64 > MAX {
            return Err("StackDock export exceeds the 8 MiB limit.".into());
        }
        Self::from_json(&b)
    }
    pub fn ordered(&self) -> Vec<&Entry> {
        let mut left = self
            .folders
            .iter()
            .chain(&self.dock_shortcuts)
            .collect::<Vec<_>>();
        let mut out = Vec::new();
        for id in &self.dock_order {
            if let Some(i) = left.iter().position(|e| &e.id == id) {
                out.push(left.remove(i));
            }
        }
        out.extend(left);
        out
    }
    pub fn folder_at(&self, path: &[usize]) -> Option<&Entry> {
        let (&root, sub) = path.split_first()?;
        let mut folder = self.folders.get(root)?;
        for &i in sub {
            folder = folder.slots.get(i)?.folder().then(|| &folder.slots[i])?;
        }
        Some(folder)
    }
}
#[derive(Clone, Copy)]
pub struct Tile {
    pub item: usize,
    pub box_: Rect,
    pub icon: Rect,
}
#[derive(Clone)]
pub struct Layout {
    pub dock: Rect,
    pub grid: Rect,
    pub ds: Skin,
    pub gs: Skin,
    pub dock_tiles: Vec<Tile>,
    pub grid_tiles: Vec<Tile>,
    pub back: Rect,
    pub settings_gear: Option<Rect>,
    pub grid_clip: Rect,
    pub s: f32,
    pub scroll: (f32, f32),
    pub dock_scroll: f32,
}
impl Layout {
    pub fn panel_rect(saved:&Saved,work:Rect,p:f32,width:f32,height:f32)->Rect{
            let w = width.clamp(1., (work.w - p * 2.).max(1.)).round();
            let h = height.clamp(1., (work.h - p * 2.).max(1.)).round();
            let a = match saved.align.as_str() {
                "start" => 0.,
                "end" => 1.,
                _ => 0.5,
            };
            let (x, y) = match saved.edge.as_str() {
                "top" => (work.x + p + (work.w - p * 2. - w) * a, work.y + p),
                "left" => (work.x + p, work.y + p + (work.h - p * 2. - h) * a),
                "right" => (work.right() - p - w, work.y + p + (work.h - p * 2. - h) * a),
                _ => (
                    work.x + p + (work.w - p * 2. - w) * a,
                    work.bottom() - p - h,
                ),
            };
            Rect::new(x.round(), y.round(), w, h)
            }

    pub fn new(
        saved: &Saved,
        work: Rect,
        s: f32,
        folder: Option<&Entry>,
        scroll: (f32, f32),
        dock_scroll: f32,
    ) -> Self {
        let s = s.clamp(0.5, 4.);
        let order = saved.ordered();
        let ds = saved.dock.skin(false, s);
        let gs = folder
            .map(|f| f.style.skin(true, s))
            .unwrap_or_else(|| Style::default().skin(true, s));
        let vertical = saved.orientation == "vertical"
            || saved.orientation != "horizontal"
                && ["left", "right"].contains(&saved.edge.as_str());
        let dims = order.iter().map(|entry| ds.dock_entry_dims(entry, s)).collect::<Vec<_>>();
        let gaps = (dims.len().saturating_sub(1)) as f32;
        let total_w = if vertical {
            dims.iter().map(|t| t.0).fold(0., f32::max)
        } else {
            dims.iter().map(|t| t.0).sum::<f32>() + gaps * ds.gx
        };
        let total_h = if vertical {
            dims.iter().map(|t| t.1).sum::<f32>() + gaps * ds.gy
        } else {
            dims.iter().map(|t| t.1).fold(0., f32::max)
        };
        let p = 14. * s;
        let place=|width,height| Self::panel_rect(saved,work,p,width,height);
        let dock = place(total_w + ds.px * 2., total_h + ds.py * 2.);
        let excess = if vertical {
            (total_h + ds.py * 2. - dock.h).max(0.)
        } else {
            (total_w + ds.px * 2. - dock.w).max(0.)
        };
        let mut pos = dock_scroll.clamp(0., excess);
        pos = -pos;
        let dock_tiles = dims
            .iter()
            .enumerate()
            .map(|(item, &(w, h))| {
                let (x, y) = if vertical {
                    (dock.cx() - w / 2., dock.y + ds.py + pos)
                } else {
                    (dock.x + ds.px + pos, dock.cy() - h / 2.)
                };
                pos += if vertical { h + ds.gy } else { w + ds.gx };
                Tile {
                    item,
                    box_: Rect::new(x, y, w, h),
                    icon: Rect::new(x, y, w, w),
                }
            })
            .filter(|t| t.box_.intersect(dock).w > 0. && t.box_.intersect(dock).h > 0.)
            .collect();
        let head = (42. * s).max(gs.title + 15. * s);
        let cell_w = if gs.labels {
            gs.icon.max(gs.label_w)
        } else {
            gs.icon
        };
        let cell_h = gs.icon
            + gs.effects_height()
            + if gs.labels {
                gs.font * 2.36 + 4. * s
            } else {
                0.
            };
        let (cols, rows) = folder
            .map(|f| (f.cols as f32, f.rows as f32))
            .unwrap_or((1., 1.));
        let (nw, nh) = (
            cols * cell_w + (cols - 1.) * gs.gx,
            rows * cell_h + (rows - 1.) * gs.gy,
        );
        let grid = place(nw + gs.px * 2., nh + gs.py * 2. + head);
        let grid_clip = Rect::new(
            grid.x + gs.px,
            grid.y + gs.py + head,
            (grid.w - gs.px * 2.).max(0.),
            (grid.h - gs.py * 2. - head).max(0.),
        );
        let ox = scroll.0.clamp(0., (nw - grid_clip.w).max(0.));
        let oy = scroll.1.clamp(0., (nh - grid_clip.h).max(0.));
        let grid_tiles = folder
            .into_iter()
            .flat_map(|f| f.slots.iter().enumerate())
            .map(|(item, entry)| {
                let x = grid_clip.x + entry.c as f32 * (cell_w + gs.gx) - ox;
                let y = grid_clip.y + entry.r as f32 * (cell_h + gs.gy) - oy;
                Tile {
                    item,
                    box_: Rect::new(x, y, cell_w, cell_h),
                    icon: Rect::new(x + (cell_w - gs.icon) / 2., y, gs.icon, gs.icon),
                }
            })
            .filter(|t| t.box_.intersect(grid_clip).w > 0. && t.box_.intersect(grid_clip).h > 0.)
            .collect();
        let back = Rect::new(grid.x + gs.px, grid.y + gs.py, 28. * s, 28. * s);
        Self {
            dock,
            grid,
            ds,
            gs,
            dock_tiles,
            grid_tiles,
            back,
            settings_gear: folder.filter(|f| f.style.settings_gear.unwrap_or(false)).map(|_| {
                Rect::new(grid.right() - gs.px - 28. * s, grid.y + gs.py, 28. * s, 28. * s)
            }),
            grid_clip,
            s,
            scroll: (ox, oy),
            dock_scroll: dock_scroll.clamp(0., excess),
        }
    }
    pub fn envelope(&self, p: f32, moving: bool) -> Rect {
        if moving {
            self.dock.union(self.grid)
        } else if p <= 0. {
            self.dock
        } else {
            self.grid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matches_real_starter_layout() {
        let saved = Saved::from_json(STARTER.as_bytes()).unwrap();
        let f = &saved.folders[0];
        assert_eq!((&f.icon[..], f.cols, f.rows), ("folder-stacked", 5, 3));
        let l = Layout::new(
            &saved,
            Rect::new(0., 0., 1920., 1040.),
            1.,
            Some(f),
            (0., 0.),
            0.,
        );
        assert_eq!((l.dock.w, l.dock.h), (148., 56.));
        assert_eq!((l.grid.w, l.grid.h), (438., 332.));
        assert_eq!(l.envelope(1., true), l.dock.union(l.grid));
        assert!(l.grid_tiles.is_empty());
    }
    #[test]
    fn scale_and_dock_order() {
        let mut saved = Saved::from_json(STARTER.as_bytes()).unwrap();
        saved.edge = "left".into();
        saved.align = "end".into();
        saved.dock_order = vec![saved.folders[2].id.clone(), saved.folders[0].id.clone()];
        assert_eq!(
            saved
                .ordered()
                .iter()
                .map(|f| &f.name[..])
                .collect::<Vec<_>>(),
            ["Office", "Shortcuts", "Games"]
        );
        let l = Layout::new(
            &saved,
            Rect::new(-1920., 0., 1920., 1040.),
            1.5,
            None,
            (0., 0.),
            0.,
        );
        assert_eq!((l.dock.x, l.dock.bottom()), (-1899., 1019.));
        assert!(l.dock_tiles[1].icon.y > l.dock_tiles[0].icon.y);
    }
    #[test]
    fn rejects_duplicate_grid_cells() {
        let mut data: serde_json::Value = serde_json::from_str(STARTER).unwrap();
        data["layout"]["folders"][0]["slots"] = serde_json::json!([
            {"id":"a","name":"First","type":"item","target":"a","r":0,"c":0},
            {"id":"b","name":"Second","type":"item","target":"b","r":0,"c":0}]);
        assert!(Saved::from_json(&serde_json::to_vec(&data).unwrap()).is_err());
    }
}
