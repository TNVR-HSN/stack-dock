use crate::model::{Entry, Layout, Rect, Saved, Skin, Tile, bounded, ease, mix};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use fontdue::{Font, FontSettings, Metrics};
use image::{ImageReader, RgbaImage};
use std::{
    collections::HashMap,
    io::{Cursor, Read},
    path::Path,
    rc::Rc,
};

pub struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: usize,
    height: usize,
    origin: Rect,
    pub clip: Rect,
}
impl<'a> Canvas<'a> {
    pub fn new(pixels: &'a mut [u32], width: usize, height: usize, origin: Rect) -> Self {
        pixels.fill(0);
        Self {
            pixels,
            width,
            height,
            origin,
            clip: origin,
        }
    }
    fn put(&mut self, x: i32, y: i32, src: u32) {
        let sx = x as f32 + 0.5;
        let sy = y as f32 + 0.5;
        if !self.clip.contains(sx, sy) {
            return;
        }
        let x = x - self.origin.x as i32;
        let y = y - self.origin.y as i32;
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            let i = y as usize * self.width + x as usize;
            self.pixels[i] = over(src, self.pixels[i]);
        }
    }
    pub fn rounded(&mut self, rect: Rect, radius: f32, color: u32) {
        let alpha = color >> 24;
        if alpha == 0 || rect.w <= 0.0 || rect.h <= 0.0 {
            return;
        }
        let x = rect.x - self.origin.x;
        let y = rect.y - self.origin.y;
        let right = x + rect.w;
        let bottom = y + rect.h;
        let radius = radius.min(rect.w * 0.5).min(rect.h * 0.5).max(0.0);
        let x0 = (x.max(self.clip.x - self.origin.x).floor() as i32).clamp(0, self.width as i32);
        let y0 = (y.max(self.clip.y - self.origin.y).floor() as i32).clamp(0, self.height as i32);
        let x1 = (right.min(self.clip.right() - self.origin.x).ceil() as i32)
            .clamp(0, self.width as i32);
        let y1 = (bottom.min(self.clip.bottom() - self.origin.y).ceil() as i32)
            .clamp(0, self.height as i32);
        let red = (color >> 16) & 255;
        let green = (color >> 8) & 255;
        let blue = color & 255;
        let src = pack(
            alpha,
            mul255(red, alpha),
            mul255(green, alpha),
            mul255(blue, alpha),
        );

        for py in y0..y1 {
            let cy = py as f32 + 0.5;
            let dy = ((y + radius) - cy).max(cy - (bottom - radius)).max(0.0);
            let row = py as usize * self.width;
            for px in x0..x1 {
                let cx = px as f32 + 0.5;
                if cx < x || cx >= right || cy < y || cy >= bottom {
                    continue;
                }
                let dx = ((x + radius) - cx).max(cx - (right - radius)).max(0.0);
                let i = row + px as usize;
                if dx > 0.0 && dy > 0.0 && radius > 0.5 {
                    let distance = dx * dx + dy * dy;
                    if distance >= (radius + 0.5) * (radius + 0.5) {
                        continue;
                    }
                    if distance > (radius - 0.5) * (radius - 0.5) {
                        let coverage = (radius + 0.5 - distance.sqrt()).clamp(0.0, 1.0);
                        let a = (alpha as f32 * coverage).round() as u32;
                        let pixel = pack(a, mul255(red, a), mul255(green, a), mul255(blue, a));
                        self.pixels[i] = over(pixel, self.pixels[i]);
                        continue;
                    }
                }
                self.pixels[i] = over(src, self.pixels[i]);
            }
        }
    }

    fn panel(&mut self, rect: Rect, radius: f32, stroke: f32, fill: u32, rim: u32) {
        let clip = rect.intersect(self.clip);
        let r = radius.max(0.).min(rect.w.min(rect.h) / 2.);
        let (hw, hh) = (rect.w / 2. - r, rect.h / 2. - r);
        let shade = |c: u32, k: f32| {
            let a = ((c >> 24) as f32 * k).round() as u32;
            pack(
                a,
                mul255((c >> 16) & 255, a),
                mul255((c >> 8) & 255, a),
                mul255(c & 255, a),
            )
        };
        for y in clip.y.floor() as i32..clip.bottom().ceil() as i32 {
            let dy = (y as f32 + 0.5 - rect.cy()).abs() - hh;
            for x in clip.x.floor() as i32..clip.right().ceil() as i32 {
                let dx = (x as f32 + 0.5 - rect.cx()).abs() - hw;
                let a = dx.max(0.);
                let b = dy.max(0.);
                let distance = if a > 0. && b > 0. {
                    (a * a + b * b).sqrt() - r
                } else {
                    dx.max(dy) - r
                };
                let edge = bounded(0.5 - distance);
                if edge == 0. {
                    continue;
                }
                let ink = if rim >> 24 > 0 && stroke > 0. {
                    shade(rim, edge - bounded(0.5 - distance - stroke))
                } else {
                    0
                };
                self.put(x, y, over(ink, shade(fill, edge)));
            }
        }
    }
    fn caption(
        &mut self,
        assets: &mut Assets,
        name: &str,
        x: f32,
        y: f32,
        font: f32,
        width: f32,
        opacity: f32,
    ) {
        let mut split = 0;
        let mut space = None;
        let mut w = 0.;
        for (i, ch) in name.char_indices().take(128) {
            let advance = assets
                .glyph(ch, font, false)
                .map_or(0., |g| g.0.advance_width);
            if w + advance > width {
                break;
            }
            w += advance;
            if ch.is_whitespace() && i > 0 {
                space = Some(i);
            }
            split = i + ch.len_utf8();
        }
        if split == 0 || split == name.len() {
            self.text(assets, name, x, y, font, width, opacity, false);
        } else {
            let (first, second) =
                name.split_at(space.filter(|offset| *offset > split / 2).unwrap_or(split));
            self.text(assets, first, x, y, font, width, opacity, false);
            self.text(
                assets,
                second.trim_start(),
                x,
                y + font * 1.18,
                font,
                width,
                opacity,
                false,
            );
        }
    }
    fn art(
        &mut self,
        image: &RgbaImage,
        r: Rect,
        corner: f32,
        rotation: f32,
        alpha: f32,
        flip: bool,
    ) {
        if alpha <= 0. || r.w <= 0. || r.h <= 0. {
            return;
        }
        let bound = r.intersect(self.clip);
        let s = r.w.min(r.h);
        let picture = Rect::new(r.cx() - s * 0.5, r.cy() - s * 0.5, s, s);
        let radius = s * corner.clamp(0., 0.5);
        let iw = image.width();
        let ih = image.height();
        let normal = iw.max(ih).max(1) as f32;
        let (wr, hr) = (iw as f32 / normal, ih as f32 / normal);
        let (sin, cos) = (-rotation.to_radians()).sin_cos();
        for y in bound.y.floor() as i32..bound.bottom().ceil() as i32 {
            let py = (y as f32 + 0.5 - picture.cy()) / s;
            for x in bound.x.floor() as i32..bound.right().ceil() as i32 {
                let px = (x as f32 + 0.5 - picture.cx()) / s;
                if radius > 0. {
                    let dx = (px.abs() * s - (s / 2. - radius)).max(0.);
                    let dy = (py.abs() * s - (s / 2. - radius)).max(0.);
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                }
                let u = (px * cos - py * sin) / wr + 0.5;
                let v = if flip {
                    0.5 - py
                } else {
                    px * sin + py * cos + 0.5
                };
                let v = (v - 0.5) / hr + 0.5;
                if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                    continue;
                }
                let p = image.get_pixel((u * iw as f32) as u32, (v * ih as f32) as u32);
                let opacity = if flip {
                    alpha * (1. - bounded((y as f32 - r.y) / (s * 0.24)))
                } else {
                    alpha
                };
                let a = (p[3] as f32 * opacity).round() as u32;
                self.put(
                    x,
                    y,
                    pack(
                        a,
                        mul255(p[0] as u32, a),
                        mul255(p[1] as u32, a),
                        mul255(p[2] as u32, a),
                    ),
                );
            }
        }
    }
    fn text(
        &mut self,
        assets: &mut Assets,
        value: &str,
        dx: f32,
        dy: f32,
        size: f32,
        max: f32,
        alpha: f32,
        strong: bool,
    ) {
        if alpha <= 0.0 || max < size || assets.font.is_none() {
            return;
        }
        let w = assets.measure(value, size, strong, max);
        let mut ox = dx - w / 2.;
        let mut remaining = String::new();
        let mut occupied = 0.;
        for ch in value.chars().filter(|ch| !ch.is_control()).take(128) {
            let advance = assets
                .glyph(ch, size, strong)
                .map_or(0., |g| g.0.advance_width);
            if occupied + advance > max {
                let dots = assets
                    .glyph('…', size, strong)
                    .map_or(0., |g| g.0.advance_width);
                while occupied + dots > max {
                    let Some(last) = remaining.pop() else { break };
                    occupied -= assets
                        .glyph(last, size, strong)
                        .map_or(0., |g| g.0.advance_width);
                }
                remaining.push('…');
                break;
            }
            occupied += advance;
            remaining.push(ch);
        }
        for ch in remaining.chars() {
            let Some(g) = assets.glyph(ch, size, strong) else {
                continue;
            };
            let (m, bits) = (&g.0, &g.1);
            let top = dy as i32 - m.ymin - m.height as i32;
            for y in 0..m.height {
                for x in 0..m.width {
                    let a = (bits[y * m.width + x] as f32 * alpha).round() as u32;
                    if a > 0 {
                        self.put(
                            ox.round() as i32 + m.xmin + x as i32,
                            top + y as i32,
                            pack(a, mul255(242, a), mul255(234, a), mul255(246, a)),
                        );
                    }
                }
            }
            ox += m.advance_width;
        }
    }
}
type Glyph = (Metrics, Vec<u8>);
pub struct Assets {
    font: Option<Font>,
    bold: Option<Font>,
    images: HashMap<String, Option<Rc<RgbaImage>>>,
    glyphs: HashMap<(char, u16, bool), Rc<Glyph>>,
}
static BUILTIN: &[(&str, &[u8])] = include!("../assets/icons.rs");
impl Assets {
    pub fn clear_layout_images(&mut self) {
        self.images.clear();
    }

    pub fn new(font_dir: &Path) -> Self {
        let font = |names: &[&str]| {
            names.iter().find_map(|p| {
                std::fs::read(font_dir.join(p))
                    .ok()
                    .and_then(|b| Font::from_bytes(b, FontSettings::default()).ok())
            })
        };
        Self {
            font: font(&["segoeui.ttf", "DejaVuSans.ttf"]),
            bold: font(&["segoeuib.ttf", "DejaVuSans-Bold.ttf"]),
            images: HashMap::new(),
            glyphs: HashMap::new(),
        }
    }
    fn glyph(&mut self, ch: char, px: f32, strong: bool) -> Option<Rc<Glyph>> {
        let key = (ch, (px * 4.).round() as u16, strong);
        if let Some(value) = self.glyphs.get(&key) {
            return Some(value.clone());
        }
        if self.glyphs.len() >= 768 {
            self.glyphs.clear()
        }
        let font = if strong {
            self.bold.as_ref().or(self.font.as_ref())
        } else {
            self.font.as_ref()
        }?;
        let a = Rc::new(font.rasterize(ch, key.1 as f32 / 4.));
        self.glyphs.insert(key, a.clone());
        Some(a)
    }
    fn measure(&mut self, s: &str, px: f32, strong: bool, max: f32) -> f32 {
        s.chars()
            .filter(|c| !c.is_control())
            .take(128)
            .map(|c| self.glyph(c, px, strong).map_or(0., |m| m.0.advance_width))
            .sum::<f32>()
            .min(max)
    }
    fn decode(bytes: &[u8]) -> Option<RgbaImage> {
        let mut reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(56 * 1024 * 1024);
        limits.max_image_height = Some(4096);
        limits.max_image_width = Some(4096);
        reader.limits(limits);
        let image = reader.decode().ok()?;
        Some(if image.width() > 192 || image.height() > 192 {
            image.thumbnail(192, 192).to_rgba8()
        } else {
            image.to_rgba8()
        })
    }
    fn local_image(path: &str, index: Option<i32>) -> Option<RgbaImage> {
        // Do not follow network paths or URLs just because a layout was opened.
        let bytes = path.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || ![b'\\', b'/'].contains(&bytes[2])
        {
            return None;
        }
        let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
        if let Some(_index) = index {
            #[cfg(windows)]
            if ["exe", "dll", "mun"].contains(&extension.as_str()) {
                return crate::resource_icon::read(Path::new(path), _index, 128);
            }
        }
        if !["png", "jpg", "jpeg", "gif", "bmp", "ico", "webp"].contains(&extension.as_str()) {
            return None;
        }
        let mut v = Vec::new();
        let max = 12 * 1024 * 1024;
        std::fs::File::open(path)
            .ok()?
            .take(max + 1)
            .read_to_end(&mut v)
            .ok()?;
        if v.len() as u64 > max {
            return None;
        }
        Self::decode(&v)
    }
    fn get(&mut self, entry: &Entry, thumbnails: bool) -> Option<Rc<RgbaImage>> {
        let custom = !entry.image.is_empty()
            && !["source", "target", "default"].contains(&entry.icon_mode.as_str());
        let thumbnail = !entry.folder()
            && entry.kind == "local"
            && entry.local_type != "folder"
            && entry.image_thumbnail.unwrap_or(thumbnails);
        let exe_fallback =
            entry.kind == "program" && (entry.icon_mode.is_empty() || entry.icon_mode == "shell");
        let key = if custom || thumbnail || exe_fallback || !entry.icon_path.is_empty() {
            format!("entry:{}", entry.id)
        } else {
            format!("built-in:{}", entry.icon_key())
        };
        if let Some(v) = self.images.get(&key) {
            return v.clone();
        }
        if self.images.len() >= 96 {
            self.images.clear()
        }
        let value = custom
            .then(|| {
                if entry.image.len() > 530_000 {
                    return None;
                }
                let (mime, data) = entry.image.split_once(',')?;
                if !mime.starts_with("data:image/") || !mime.ends_with(";base64") {
                    return None;
                }
                Self::decode(&STANDARD.decode(data).ok()?)
            })
            .flatten()
            .or_else(|| {
                thumbnail
                    .then(|| Self::local_image(&entry.target, None))
                    .flatten()
            })
            .or_else(|| {
                (entry.icon_mode == "source" && !entry.icon_path.is_empty())
                    .then(|| Self::local_image(&entry.icon_path, Some(entry.icon_index)))
                    .flatten()
            })
            .or_else(|| {
                exe_fallback
                    .then(|| Self::local_image(&entry.target, Some(0)))
                    .flatten()
            })
            .or_else(|| {
                let key = entry.icon_key();
                let fallback = if entry.folder() {
                    "folder-azure"
                } else {
                    "file"
                };
                BUILTIN
                    .iter()
                    .find(|(name, _)| *name == key)
                    .or_else(|| BUILTIN.iter().find(|(name, _)| *name == fallback))
                    .and_then(|(_, data)| Self::decode(data))
            })
            .map(Rc::new);
        self.images.insert(key, value.clone());
        value
    }
}
fn mul255(a: u32, b: u32) -> u32 {
    (a * b + 127) / 255
}
fn pack(a: u32, r: u32, g: u32, b: u32) -> u32 {
    a << 24 | r << 16 | g << 8 | b
}
fn over(src: u32, dst: u32) -> u32 {
    let alpha = src >> 24;
    if alpha == 255 || dst == 0 {
        return src;
    }
    if alpha == 0 {
        return dst;
    }
    let color = |n: u32| ((src >> n) & 255) + mul255((dst >> n) & 255, 255 - alpha);
    pack(color(24), color(16), color(8), color(0))
}
fn mix_color(a: u32, b: u32, p: f32) -> u32 {
    let c = |i: u32| mix(((a >> i) & 255) as f32, ((b >> i) & 255) as f32, p).round() as u32;
    pack(c(24), c(16), c(8), c(0))
}
fn corner(e: &Entry, skin: Skin, dock: bool) -> f32 {
    if dock {
        if let Some(n) = e.dock_icon_corner {
            return n.clamp(0., 50.) / 100.;
        }
        if e.dock_icon_shape == "raw" {
            return 0.;
        }
        if e.dock_icon_shape == "rounded" {
            return 0.5;
        }
    }
    skin.corner
}
fn tile(
    c: &mut Canvas,
    a: &mut Assets,
    e: &Entry,
    t: Tile,
    s: Skin,
    scale: f32,
    p: f32,
    dock: bool,
    hover: bool,
    thumb: bool,
    skip_icon: bool,
) {
    if p <= 0. {
        return;
    }
    if hover {
        c.rounded(
            t.icon.inset(-2. * scale),
            t.icon.w * corner(e, s, dock),
            0x25ffffff,
        );
    }
    if !skip_icon {
        if let Some(image) = a.get(e, thumb) {
            let angle = e.icon_rotation.unwrap_or(s.rotation);
            c.art(&image, t.icon, corner(e, s, dock), angle, p, false);
            if s.reflect > 0. {
                let old = c.clip;
                c.clip = old.intersect(Rect::new(t.icon.x, t.icon.bottom(), t.icon.w, s.reflect));
                let reflection = Rect::new(t.icon.x, t.icon.bottom(), t.icon.w, t.icon.h);
                c.art(&image, reflection, 0., 0., p * s.reflect_alpha, true);
                c.clip = old;
            }
        }
    }
    if s.labels {
        let y = t.icon.bottom() + s.reflect + s.font + scale;
        if dock {
            c.text(a, &e.name, t.box_.cx(), y, s.font, t.box_.w, p, false);
        } else {
            c.caption(a, &e.name, t.box_.cx(), y, s.font, t.box_.w, p);
        }
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum Hit {
    Dock(usize),
    Grid(usize),
    Back,
}
pub struct View<'a> {
    pub saved: &'a Saved,
    pub layout: &'a Layout,
    pub path: &'a [usize],
    pub dock_index: Option<usize>,
    pub p: f32,
    pub a: f32,
    pub hover: Option<Hit>,
}
pub fn frame(c: &mut Canvas, a: &mut Assets, v: View) {
    let View {
        saved,
        layout: l,
        path,
        dock_index,
        p,
        a: flight,
        hover,
    } = v;
    let s = l.s;
    let thumb = saved.general.image_thumbnails.unwrap_or(true);
    let root_clip = c.clip;
    let current = saved.folder_at(path);
    let mode = current
        .map(|f| f.entry_motion.as_str())
        .unwrap_or("classic");
    let (x, y) = match mode {
        "rows" => (bounded(p * 1.65), bounded((p - 0.32) / 0.68)),
        "columns" => (bounded((p - 0.32) / 0.68), bounded(p * 1.65)),
        _ => (p, p),
    };
    let panel = l.dock.lerp(l.grid, x, y);
    let r = mix(l.ds.radius, l.gs.radius, p);
    // Keep the saved opacity. The rim is a border, not a second translucent
    // layer painted behind the interior.
    let stroke = mix(l.ds.stroke, l.gs.stroke, p);
    let fill = mix_color(l.ds.fill, l.gs.fill, p);
    let rim = mix_color(l.ds.border, l.gs.border, p);
    c.panel(panel, r, stroke, fill, rim);
    let order = saved.ordered();
    let dock_alpha = bounded(1. - p * 1.65);
    c.clip = root_clip.intersect(panel);
    for &t in &l.dock_tiles {
        tile(
            c,
            a,
            order[t.item],
            t,
            l.ds,
            s,
            dock_alpha,
            true,
            hover == Some(Hit::Dock(t.item)),
            thumb,
            current.is_some() && dock_index == Some(t.item),
        );
    }
    let Some(folder) = current else {
        c.clip = root_clip;
        return;
    };
    let header_alpha = bounded((p - 0.23) * 2.5);
    let font = 13. * s;
    let max = (l.grid.w - 2. * l.gs.px - l.gs.title - 30. * s).max(font);
    let title_w = a.measure(&folder.name, font, true, max);
    let title_group_width = l.gs.title + 8. * s + title_w;
    let title_y = l.grid.y + l.gs.py;
    let icon_rect = Rect::new(
        l.grid.cx() - title_group_width / 2.,
        title_y,
        l.gs.title,
        l.gs.title,
    );
    c.text(
        a,
        &folder.name,
        icon_rect.right() + 8. * s + title_w / 2.,
        title_y + l.gs.title / 2. + font * 0.35,
        font,
        max,
        header_alpha,
        true,
    );
    if path.len() > 1 {
        c.text(
            a,
            "‹",
            l.back.cx(),
            l.back.y + 21. * s,
            19. * s,
            l.back.w,
            header_alpha,
            false,
        );
    }
    c.clip = root_clip.intersect(panel).intersect(l.grid_clip);
    let max_distance = l.grid_clip.w.hypot(l.grid_clip.h) / 2.;
    for &t in &l.grid_tiles {
        let item = &folder.slots[t.item];
        let d = (t.icon.cx() - l.grid_clip.cx()).hypot(t.icon.cy() - l.grid_clip.cy());
        let mut t = t;
        let alpha = match mode {
            "diagonal" | "outer" => {
                let order = if mode == "outer" {
                    1. - bounded(d / max_distance.max(1.))
                } else {
                    (item.r + item.c) as f32 / (folder.rows + folder.cols - 2).max(1) as f32
                };
                let rank = bounded((p - 0.28 - order * 0.28) / 0.4);
                t.icon = t.icon.square(t.icon.w * mix(0.82, 1., rank));
                rank
            }
            "fly-in" => {
                let percent = bounded((p - 0.02) / 0.9);
                let reach = (1. - percent) * 26. * s / d.max(1.);
                let dx = (t.icon.cx() - l.grid_clip.cx()) * reach;
                let dy = (t.icon.cy() - l.grid_clip.cy()) * reach;
                t.icon.x += dx;
                t.icon.y += dy;
                t.box_.x += dx;
                t.box_.y += dy;
                header_alpha
            }
            _ => header_alpha,
        };
        tile(
            c,
            a,
            item,
            t,
            l.gs,
            s,
            alpha,
            false,
            hover == Some(Hit::Grid(t.item)),
            thumb,
            false,
        );
    }
    c.clip = root_clip;
    if let Some(img) = a.get(folder, thumb) {
        let anchor = l
            .dock_tiles
            .iter()
            .find(|t| Some(t.item) == dock_index)
            .map(|t| t.icon);
        let fly = if path.len() == 1 {
            anchor.map_or(icon_rect, |f| f.lerp(icon_rect, flight, flight))
        } else {
            icon_rect
        };
        let opacity = if anchor.is_some() { 1. } else { header_alpha };
        let radius = mix(corner(folder, l.ds, true), l.gs.corner, flight);
        let angle = folder.icon_rotation.unwrap_or(l.ds.rotation);
        c.art(&img, fly, radius, angle, opacity, false);
    }
}
pub fn ease_open(t: f32) -> f32 {
    ease(t, 0.19, 0.84, 0.26, 1.)
}
pub fn ease_close(t: f32) -> f32 {
    ease(t, 0.38, 0.03, 0.32, 1.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::STARTER;
    #[test]
    fn has_real_icon_and_alpha() {
        let data = Saved::from_json(STARTER.as_bytes()).unwrap();
        let mut a = Assets::new(Path::new("."));
        let icon = a.get(&data.folders[0], true).unwrap();
        assert_eq!((icon.width(), icon.height()), (128, 128));
        assert_eq!(icon.get_pixel(0, 0)[3], 0);
        assert_eq!(icon.get_pixel(64, 64)[3], 255);
    }
    #[test]
    fn transparent_dock_renders_at_transition_endpoint() {
        let saved = Saved::from_json(STARTER.as_bytes()).unwrap();
        let l = Layout::new(
            &saved,
            Rect::new(0., 0., 800., 440.),
            1.,
            Some(&saved.folders[0]),
            (0., 0.),
            0.,
        );
        let mut assets = Assets::new(Path::new("."));
        for p in [0., 1.] {
            let stable = l.envelope(p, false);
            let moving = l.envelope(p, true);
            let mut image = |bounds: Rect| {
                let mut buffer = vec![0; bounds.w as usize * bounds.h as usize];
                frame(
                    &mut Canvas::new(&mut buffer, bounds.w as usize, bounds.h as usize, bounds),
                    &mut assets,
                    View {
                        saved: &saved,
                        layout: &l,
                        path: &[0],
                        dock_index: Some(0),
                        p,
                        a: p,
                        hover: None,
                    },
                );
                buffer
            };
            let a = image(stable);
            let b = image(moving);
            assert_eq!(a[0] >> 24, 0);
            let (x, y) = (
                (stable.x - moving.x) as usize,
                (stable.y - moving.y) as usize,
            );
            for row in 0..stable.h as usize {
                let start = (row + y) * moving.w as usize + x;
                let n = stable.w as usize;
                assert_eq!(&a[row * n..(row + 1) * n], &b[start..start + n]);
            }
        }
    }
}
