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

    pub fn overlay(pixels:&'a mut [u32],width:usize,height:usize,origin:Rect)->Self{
        Self{pixels,width,height,origin,clip:origin}
    }
    pub fn ghost(&mut self,assets:&mut Assets,entry:&Entry,x:f32,y:f32,scale:f32){
        if let Some(img)=assets.get(entry,true){
            let size=36.*scale;
            self.art(&img,Rect::new(x-size/2.,y-size/2.,size,size),0.,0.,0.8,false);
        }
    }
    pub fn fade_snapshot(&mut self,src:&[u32],r:Rect,opacity:f32){
        if opacity<=0. || src.is_empty(){return;}
        let fade=(bounded(opacity)*255.).round() as u32;
        let stride=r.w as usize;
        if fade==0 || stride==0{return;}
        let visible=r.intersect(self.clip).intersect(self.origin);
        if visible.w<=0. || visible.h<=0.{return;}
        let left=(visible.x-0.5).ceil() as i32;
        let right=(visible.right()-0.5).ceil() as i32;
        for y in (visible.y-0.5).ceil() as i32..(visible.bottom()-0.5).ceil() as i32{
            let sy=y-r.y as i32;let sx=left-r.x as i32;
            let dx=left-self.origin.x as i32;let dy=y-self.origin.y as i32;
            if sx<0||sy<0||dx<0||dy<0||right<=left{continue;}
            let count=((right-left) as usize).min(stride.saturating_sub(sx as usize)).min(self.width.saturating_sub(dx as usize));
            let start=sy as usize*stride+sx as usize;let out=dy as usize*self.width+dx as usize;
            if let (Some(line),Some(dst))=(src.get(start..start+count),self.pixels.get_mut(out..out+count)){
                for (&v,d) in line.iter().zip(dst){if v!=0{
                    let ch=|n:u32|mul255((v>>n)&255,fade);
                    *d=over(if fade==255{v}else{pack(ch(24),ch(16),ch(8),ch(0))},*d);
                }}
            }
        }
    }
    pub fn launch_card(&mut self,assets:&mut Assets,saved:&Saved,layout:&Layout,item:&Entry,source:Rect,
        from:Rect,card:Rect,enter:f32,exit:f32,sprite:Option<&TileSprite>,source_skin:Skin,previous:&[u32]){
        let style=&saved.dock.launch;let s=layout.s;let alpha=enter*(1.-exit);
        let panel=from.lerp(card,enter,enter).lerp(layout.dock,exit,exit);
        let color=style.fill(layout.ds.fill);
        let dock=layout.ds;let radius=(style.radius()*s).min(card.h/2.);
        self.panel(panel,mix(mix(source_skin.radius,radius,enter),dock.radius,exit),
            mix(mix(source_skin.stroke,dock.stroke,enter),dock.stroke,exit),
            mix_color(mix_color(source_skin.fill,color,enter),dock.fill,exit),
            mix_color(source_skin.border,dock.border,enter),
            mix(mix(source_skin.inset_shadow,0.,enter),dock.inset_shadow,exit),s);
        let old_clip=self.clip;
        self.clip=self.clip.intersect(panel);
        self.fade_snapshot(previous,from,(1.-enter/0.65).max(0.));
        let size=(style.icon()*s).min(card.h-10.*s).max(1.);
        let icon=Rect::new(card.x+style.pad()*s,card.cy()-size/2.,size,size);
        let returning=Rect::new(panel.x+icon.x-card.x,panel.cy()-size/2.,size,size);
        let flying=source.lerp(icon,enter,enter).lerp(returning,exit,exit);
        if let Some(sprite)=sprite{scaled_sprite(self,sprite,flying,sprite.width as f32,sprite.height as f32,1.-exit);}
        let x=icon.right()+style.gap()*s+(panel.x-card.x)*exit;let width=(card.right()-style.pad()*s-x).max(0.);
        let font=style.font()*s;let caption=style.launch_label.unwrap_or(true);
        let baseline=mix(card.cy(),panel.cy(),exit)+font*0.35-if caption{7.*s}else{0.};
        let measured=assets.measure(&item.name,font,true,width);
        self.colored_text(assets,&item.name,x+measured/2.,baseline,font,width,alpha,true,style.ink());
        if caption{let size=11.*s;let measured=assets.measure("Launched",size,false,width);
            self.colored_text(assets,"Launched",x+measured/2.,baseline+14.*s,size,width,alpha*0.76,false,style.ink());}
        if exit>0.{
            let order=saved.ordered();
            for &tile_rect in &layout.dock_tiles{
                let moved=dock_motion(tile_rect,layout.dock,card,exit,&saved.edge);
                tile(self,assets,order[moved.item],moved,dock,s,exit,true,false,0.,
                    saved.general.image_thumbnails.unwrap_or(true),false,moved.icon.bottom());
            }
        }
        self.clip=old_clip;
    }
    pub fn sprite(&mut self,pixels:&[u32],side:usize,x:i32,y:i32){
        let left=x-side as i32/2;let top=y-side as i32/2;
        for (row,line) in pixels.chunks_exact(side).enumerate(){
            for (col,&pixel) in line.iter().enumerate(){if pixel!=0{self.put(left+col as i32,top+row as i32,pixel);}}
        }
    }
    pub fn drop_cell(&mut self,r:Rect,scale:f32){
        let px=scale.round().max(1.) as i32;
        for y in r.y.ceil() as i32..r.bottom() as i32{
            for x in r.x.ceil() as i32..r.right() as i32{
                let dx=x-r.x.ceil() as i32;let dy=y-r.y.ceil() as i32;
                let ex=dx.min(r.right() as i32-1-x);let ey=dy.min(r.bottom() as i32-1-y);let edge=ex.min(ey);
                self.put(x,y,0x312b1e2e);
                if edge<2*px{self.put(x,y,0x8c34193f);}
                if edge<px && (if ex<ey{dy}else{dx})/(3*px)%2==0{self.put(x,y,0xffefb3ff);}
            }
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

    fn panel(&mut self, rect: Rect, radius: f32, stroke: f32, fill: u32, rim: u32, shadow:f32, dpi:f32) {
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
        let flat=shade(fill,1.);
        let margin=(r+stroke.max(0.)+20.*dpi+2.).ceil();
        for y in clip.y.floor() as i32..clip.bottom().ceil() as i32 {
            let dy = (y as f32 + 0.5 - rect.cy()).abs() - hh;
            let x0=clip.x.floor() as i32;
            let x1=clip.right().ceil() as i32;
            let middle=y as f32>=rect.y+margin && (y as f32)<rect.bottom()-margin;
            let left=if middle{((rect.x+margin).ceil() as i32).clamp(x0,x1)}else{x1};
            let right=if middle{((rect.right()-margin).floor() as i32).clamp(left,x1)}else{x1};
            if right>left{
                let row=y-self.origin.y as i32;let col=left-self.origin.x as i32;
                if row>=0 && row<self.height as i32 && col>=0 && col+right-left<=self.width as i32{
                    let start=row as usize*self.width+col as usize;
                    for d in &mut self.pixels[start..start+(right-left) as usize]{*d=over(flat,*d);}
                }
            }
            for x in (x0..left).chain(right..x1) {
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
                let inset = bounded(1. + (distance + 5.*dpi*(y as f32-rect.cy())/(rect.h*0.5).max(1.))/(14.*dpi).max(1.));
                let black = ((255.*shadow*inset*inset*edge).round() as u32)<<24;
                self.put(x, y, over(ink, over(black, shade(fill, edge))));
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
        let bold = true;
        let mut split = 0;
        let mut space = None;
        let mut w = 0.;

        for (i, ch) in name.char_indices().take(128) {
            let advance = assets
                .glyph(ch, font, bold)
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
            self.text(assets, name, x, y, font, width, opacity, bold);
        } else {
            let (first, second) = name.split_at(space.unwrap_or(split));

            self.text(assets, first, x, y, font, width, opacity, bold);
            self.text(
                assets,
                second.trim_start(),
                x,
                y + font * 1.18,
                font,
                width,
                opacity,
                bold,
            );
        }
    }    fn art(
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
        // Sample a 4 × 4 grid across each output pixel.
        const OFFSETS: [f32; 4] = [0.125, 0.375, 0.625, 0.875];
            
        for y in bound.y.floor() as i32..bound.bottom().ceil() as i32 {
            let opacity = if flip {
                alpha * (1. - bounded((y as f32 - r.y) / (s * 0.24)))
            } else {
                alpha
            };
        
            for x in bound.x.floor() as i32..bound.right().ceil() as i32 {
                let mut channels = [0u32; 4];
            
                for oy in OFFSETS {
                    let py = (y as f32 + oy - picture.cy()) / s;
                
                    for ox in OFFSETS {
                        let px = (x as f32 + ox - picture.cx()) / s;
                    
                        if radius > 0. {
                            let dx = (px.abs() * s - (s * 0.5 - radius)).max(0.);
                            let dy = (py.abs() * s - (s * 0.5 - radius)).max(0.);
                            if dx * dx + dy * dy > radius * radius {
                                continue;
                            }
                        }
                    
                        let u = (px * cos - py * sin) / wr + 0.5;
                        let source_y = if flip { -py } else { px * sin + py * cos };
                        let v = source_y / hr + 0.5;
                    
                        if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                            continue;
                        }
                    
                        let pixel = sample_icon(image, u, v, opacity);
                        channels[0] += pixel >> 24;
                        channels[1] += (pixel >> 16) & 255;
                        channels[2] += (pixel >> 8) & 255;
                        channels[3] += pixel & 255;
                    }
                }

        // Samples outside the artwork contribute transparent black.
                let [a, red, green, blue] = channels.map(|sum| (sum + 8) / 16);
                if a > 0 {
                    self.put(x, y, pack(a, red, green, blue));
                }
            } // ends for x
        } // ends for y
    } // ends fn art
    fn text(&mut self,assets:&mut Assets,value:&str,dx:f32,dy:f32,size:f32,max:f32,alpha:f32,strong:bool){
        self.colored_text(assets,value,dx,dy,size,max,alpha,strong,0xf2eaf6);
    }
    fn colored_text(
        &mut self,
        assets: &mut Assets,
        value: &str,
        dx: f32,
        dy: f32,
        size: f32,
        max: f32,
        alpha: f32,
        strong: bool,
        color:u32,
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
                let start_x = ox;
        for shadow in [true, false] {
            ox = start_x;

            for ch in remaining.chars() {
                let Some(g) = assets.glyph(ch, size, strong) else {
                    continue;
                };

                let (m, bits) = (&g.0, &g.1);
                let left = ox.round() as i32 + m.xmin;
                let top = dy as i32 - m.ymin - m.height as i32;

                for y in 0..m.height {
                    for x in 0..m.width {
                        let coverage = bits[y * m.width + x] as f32 * alpha;
                        let px = left + x as i32;
                        let py = top + y as i32;

                        if shadow {
                            let a = (coverage * 0.8).round() as u32;
                            if a > 0 {
                                self.put(px, py + 1, pack(a, 0, 0, 0));
                            }
                        } else {
                            let a = coverage.round() as u32;
                            if a > 0 {
                                self.put(
                                    px,
                                    py,
                                    pack(a,mul255((color>>16)&255,a),mul255((color>>8)&255,a),mul255(color&255,a)),
                                );
                            }
                        }
                    }
                }
                ox += m.advance_width;
            }
        }
    }
}
type Glyph = (Metrics, Vec<u8>);
pub(crate) struct TileSprite {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
}

pub(crate) struct HoverTip{
    pub rect:Rect, lines:Vec<(String,f32,bool,f32)>,background:bool,scale:f32,
}
impl HoverTip{
    pub fn new(assets:&mut Assets,icon:Rect,bottom:f32,work:Rect,scale:f32,font:f32,bg:bool,name:&str,target:&str)->Option<Self>{
        let limit=(540.*scale).min(work.w-12.*scale)-16.*scale;
        if limit<font{return None;}
        let mut lines=Vec::new();let mut width=0_f32;let mut height=6.*scale;
        for (value,size,bold) in [(name,font,true),(target,(font-scale).max(9.*scale),false)]{
            if value.is_empty(){continue;}
            if !lines.is_empty(){height+=4.*scale;}
            let mut line=String::new();let mut used=0.;let mut n=0;let mut block=0.;
            for ch in value.chars().chain(std::iter::once('\n')){
                let advance=assets.glyph(ch,size,bold).map_or(0.,|g|g.0.advance_width);
                if ch=='\n'||used+advance>limit||n>=128{
                    if !line.is_empty(){let baseline=height-3.*scale+size;
                        lines.push((std::mem::take(&mut line),size,bold,baseline));width=width.max(used);
                        height+=size*1.25;block+=size*1.25;used=0.;n=0;}
                    if block+size*1.25>work.h*0.4||height+size*1.25>work.h-8.*scale{break;}
                    if ch=='\n'{continue;}
                }
                if !ch.is_control(){line.push(ch);used+=advance;n+=1;}
            }
        }
        if lines.is_empty(){return None;}
        width=(width+16.*scale).ceil();height=height.ceil();
        let x=(icon.cx()-width/2.).clamp(work.x+6.*scale,(work.right()-width-6.*scale).max(work.x+6.*scale));
        let below=bottom+5.*scale;
        let y=if below+height+4.*scale<=work.bottom(){below}else{(icon.y-5.*scale-height).max(work.y+4.*scale)};
        Some(Self{rect:Rect::new(x.round(),y.round(),width,height),lines,background:bg,scale})
    }
    pub fn paint(&self,c:&mut Canvas,a:&mut Assets,opacity:f32){
        if self.background{c.rounded(self.rect,5.*self.scale,(((237.*opacity).round() as u32)<<24)|0x130f19);}
        for (line,size,bold,offset) in &self.lines{
            c.colored_text(a,line,self.rect.cx(),self.rect.y+offset,*size,self.rect.w,opacity,*bold,if *bold{0xffffff}else{0xeee1f4});
        }
    }
}
pub struct Assets {
    warm:Option<std::sync::mpsc::Receiver<(String,Option<RgbaImage>)>>,
    entrance_tiles: HashMap<(String,usize,bool), Rc<TileSprite>>,
    font: Option<Font>,
    bold: Option<Font>,
    images: HashMap<String, Option<Rc<RgbaImage>>>,
    reflections: HashMap<(usize,[u32;5]),(Rc<RgbaImage>,Vec<u32>)>,
    painted_icons: HashMap<(usize, [u32; 6]), (Rc<RgbaImage>, Vec<u32>)>,
    glyphs: HashMap<(char, u16, bool), Rc<Glyph>>,
}
static BUILTIN: &[(&str, &[u8])] = include!("../assets/icons.rs");
impl Assets {
    pub fn clear_entrance(&mut self) { self.entrance_tiles.clear(); }

    pub fn clear_layout_images(&mut self) {
        self.warm=None;self.images.clear();
        self.entrance_tiles.clear();
        self.painted_icons.clear();
        self.reflections.clear();
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
            warm:None,
            font: font(&["seguisb.ttf", "segoeui.ttf", "DejaVuSans.ttf"]),
            bold: font(&["segoeuib.ttf", "DejaVuSans-Bold.ttf"]),
            entrance_tiles: HashMap::new(),
            images: HashMap::new(),
            painted_icons: HashMap::new(),
            reflections: HashMap::new(),
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
    pub fn prefetch(&mut self,saved:&Saved){
        let thumbnails=saved.general.image_thumbnails.unwrap_or(true);
        let mut jobs=Vec::new();
        fn collect(entries:&[Entry],jobs:&mut Vec<Entry>,thumbnails:bool){
            for e in entries{
                if jobs.len()>=96{return;}
                if !e.folder() && (!e.image.is_empty() || !e.icon_path.is_empty() || e.kind=="program" ||
                    (e.kind=="local" && e.image_thumbnail.unwrap_or(thumbnails))){jobs.push(e.clone());}
                if e.folder(){collect(&e.slots,jobs,thumbnails);}
            }
        }
        collect(&saved.folders,&mut jobs,thumbnails);
        let (tx,rx)=std::sync::mpsc::sync_channel(96);self.warm=Some(rx);
        std::thread::spawn(move||{
            for e in jobs{
                let custom=!e.image.is_empty() && !["source","target","default"].contains(&e.icon_mode.as_str());
                let thumb=e.kind=="local" && e.local_type!="folder" && e.image_thumbnail.unwrap_or(thumbnails);
                let exe=e.kind=="program" && (e.icon_mode.is_empty()||e.icon_mode=="shell");
                if !(custom||thumb||exe||!e.icon_path.is_empty()){continue;}
                let txt=e.kind=="local" && e.local_type!="folder" && e.icon_key()=="file" && Path::new(&e.target).extension().and_then(|x|x.to_str()).is_some_and(|x|x.eq_ignore_ascii_case("txt"));
                let image=Self::load_entry(&e,custom,thumb,exe,txt);
                if tx.send((format!("entry:{}",e.id),image)).is_err(){break;}
            }
        });
    }
    fn load_entry(entry:&Entry,custom:bool,thumbnail:bool,exe_fallback:bool,text_document:bool)->Option<RgbaImage>{
        custom
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
                if text_document {
                    return Self::decode(include_bytes!("../assets/text-document.png"));
                }

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
        let text_document = entry.kind == "local"
            && entry.local_type != "folder"
            && entry.icon_key() == "file"
            && Path::new(&entry.target)
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"));

        let key = if custom || thumbnail || exe_fallback || !entry.icon_path.is_empty() {
            format!("entry:{}", entry.id)
        } else if text_document {
            String::from("built-in:text-document")
        } else {
            format!("built-in:{}", entry.icon_key())
        };
        if let Some(rx)=&self.warm{
            for (k,image) in rx.try_iter(){if self.images.len()<192{self.images.entry(k).or_insert_with(||image.map(Rc::new));}}
        }
        if let Some(v)=self.images.get(&key){return v.clone();}
        if self.images.len()>=192{self.images.clear();}
        let value=Self::load_entry(entry,custom,thumbnail,exe_fallback,text_document).map(Rc::new);
        self.images.insert(key, value.clone());
        value
    }
}
fn sample_icon(image: &RgbaImage, u: f32, v: f32, opacity: f32) -> u32 {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return 0;
    }

    let x = (u * width as f32 - 0.5).clamp(0.0, (width - 1) as f32);
    let y = (v * height as f32 - 0.5).clamp(0.0, (height - 1) as f32);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let samples = [
        (x0, y0, (1.0 - fx) * (1.0 - fy)),
        (x1, y0, fx * (1.0 - fy)),
        (x0, y1, (1.0 - fx) * fy),
        (x1, y1, fx * fy),
    ];

    let mut a = 0.0;
    let mut rgb = [0.0_f32; 3];
    for (sx, sy, weight) in samples {
        let pixel = image.get_pixel(sx, sy);
        let weighted_alpha = pixel[3] as f32 * weight;
        a += weighted_alpha;
        for channel in 0..3 {
            rgb[channel] += pixel[channel] as f32 * weighted_alpha / 255.0;
        }
    }

    let opacity = opacity.clamp(0.0, 1.0);
    let a = (a * opacity).round() as u32;
    let channel = |i: usize| ((rgb[i] * opacity).round() as u32).min(a);
    pack(a, channel(0), channel(1), channel(2))
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
// A reflection stays at the resting icon position during hover motion.
fn paint_reflection(c:&mut Canvas,a:&mut Assets,img:&Rc<RgbaImage>,r:Rect,depth:f32,opacity:f32){
    let strip=Rect::new(r.x,r.y,r.w,depth.min(r.h));
    let visible=c.clip.intersect(strip);
    if opacity<=0. || visible.w<=0. || visible.h<=0.{return;}
    let x=r.x.floor();let y=r.y.floor();
    let width=(strip.right().ceil()-x) as usize;
    let height=(strip.bottom().ceil()-y) as usize;
    let old=c.clip;c.clip=visible;
    if width>256 || height>128{
        c.art(img,r,0.,0.,opacity,true);
    }else{
        let local=Rect::new(r.x-x,r.y-y,r.w,r.h);
        let key=(Rc::as_ptr(img) as usize,[local.x.to_bits(),local.y.to_bits(),r.w.to_bits(),r.h.to_bits(),depth.to_bits()]);
        if a.reflections.len()>=192 && !a.reflections.contains_key(&key){a.reflections.clear();}
        let (_,pixels)=a.reflections.entry(key).or_insert_with(||{
            let mut pixels=vec![0;width*height];
            let mut layer=Canvas::new(&mut pixels,width,height,Rect::new(0.,0.,width as f32,height as f32));
            layer.clip=Rect::new(local.x,local.y,r.w,strip.h);
            layer.art(img,local,0.,0.,1.,true);
            (Rc::clone(img),pixels)
        });
        c.fade_snapshot(pixels,Rect::new(x,y,width as f32,height as f32),opacity);
    }
    c.clip=old;
}
fn paint_stationary_icon(
    canvas: &mut Canvas,
    assets: &mut Assets,
    image: &Rc<RgbaImage>,
    rect: Rect,
    corner: f32,
    angle: f32,
    opacity: f32,
) {
    let left = rect.x.floor() as i32;
    let top = rect.y.floor() as i32;
    let width = (rect.right().ceil() as i32 - left).max(0) as usize;
    let height = (rect.bottom().ceil() as i32 - top).max(0) as usize;

    if width == 0 || height == 0 {
        return;
    }

    // Bound memory use for unusually large icons or display scales.
    if width > 192 || height > 192 {
        canvas.art(image, rect, corner, angle, opacity, false);
        return;
    }

    let local_x = rect.x - left as f32;
    let local_y = rect.y - top as f32;
    let key = (
        Rc::as_ptr(image) as usize,
        [
            local_x.to_bits(),
            local_y.to_bits(),
            rect.w.to_bits(),
            rect.h.to_bits(),
            corner.to_bits(),
            angle.to_bits(),
        ],
    );

    if assets.painted_icons.len() >= 128 && !assets.painted_icons.contains_key(&key) {
        assets.painted_icons.clear();
    }

    let (_, pixels) = assets.painted_icons.entry(key).or_insert_with(|| {
        let mut pixels = vec![0u32; width * height];
        let bounds = Rect::new(0., 0., width as f32, height as f32);
        let local_rect = Rect::new(local_x, local_y, rect.w, rect.h);
        Canvas::new(&mut pixels, width, height, bounds)
            .art(image, local_rect, corner, angle, 1., false);

        // Retain the source image while its address is used as a cache key.
        (Rc::clone(image), pixels)
    });

    let fade = (opacity.clamp(0., 1.) * 255.).round() as u32;

    // Skip pixels outside the currently visible part of the growing panel.
    let x0 = ((canvas.clip.x.floor() as i32 - left).clamp(0, width as i32)) as usize;
    let x1 = ((canvas.clip.right().ceil() as i32 - left).clamp(0, width as i32)) as usize;
    let y0 = ((canvas.clip.y.floor() as i32 - top).clamp(0, height as i32)) as usize;
    let y1 = ((canvas.clip.bottom().ceil() as i32 - top).clamp(0, height as i32)) as usize;

    for y in y0..y1 {
        for x in x0..x1 {
            let pixel = pixels[y * width + x];
            if pixel == 0 {
                continue;
            }

            let faded = if fade == 255 {
                pixel
            } else {
                pack(
                    mul255(pixel >> 24, fade),
                    mul255((pixel >> 16) & 255, fade),
                    mul255((pixel >> 8) & 255, fade),
                    mul255(pixel & 255, fade),
                )
            };

            canvas.put(left + x as i32, top + y as i32, faded);
        }
    }
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
    hover_amount: f32,
    thumb: bool,
    skip_icon: bool,
    caption_bottom: f32,
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
            let mut artwork = t.icon;
            let mut angle = e.icon_rotation.unwrap_or(s.rotation);

            if matches!(s.hover_mode, 1 | 3 | 5) {
                artwork = t.icon.square(t.icon.w * mix(1., s.hover_scale, hover_amount));
            }
            if matches!(s.hover_mode, 2 | 3 | 5) {
                artwork.y -= s.hover_lift * hover_amount;
            }
            if matches!(s.hover_mode, 4 | 5) {
                angle += s.hover_angle * hover_amount;
            }

            let icon_corner = corner(e, s, dock);
            let at_rest = artwork.x == t.icon.x
                && artwork.y == t.icon.y
                && artwork.w == t.icon.w
                && artwork.h == t.icon.h
                && angle == e.icon_rotation.unwrap_or(s.rotation);

            if at_rest {
                paint_stationary_icon(c, a, &image, artwork, icon_corner, angle, p);
            } else {
                c.art(&image, artwork, icon_corner, angle, p, false);
            }
            if s.reflect > 0. {
                let reflection=Rect::new(t.icon.x,t.icon.bottom(),t.icon.w,t.icon.h);
                paint_reflection(c,a,&image,reflection,s.reflect,p*s.reflect_alpha);
            }
        }
    }
    if s.labels {
        let y = caption_bottom + s.reflect + s.font + scale;
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
    SettingsGear,
}
// Rasterize the complete cell once per entrance, then transform the cached
// premultiplied pixels, including captions and reflections, as a single layer.
fn entrance_tile(c: &mut Canvas, a: &mut Assets, item: &Entry, folder: &Entry,
    original: Tile, dest: Rect, skin: Skin, dpi: f32, alpha: f32, thumb: bool, skip: bool) {
    let visible = dest.intersect(c.clip);
    if alpha <= 0. || visible.w <= 0. || visible.h <= 0. { return; }
    let k=dest.w/original.box_.w.max(1.);
    let top=original.icon.y-original.box_.y;
    let artwork=Rect::new(dest.x+(original.icon.x-original.box_.x)*k,dest.y+top*k,original.icon.w*k,(original.icon.h+skin.reflect)*k);
    if skin.labels{
        let baseline=(artwork.bottom()+skin.font+dpi).round();
        c.caption(a,&item.name,dest.cx().round(),baseline,skin.font,original.box_.w,alpha);
    }
    let dest=artwork;
    let visible=dest.intersect(c.clip);
    if skip || visible.w<=0. || visible.h<=0.{return;}
    let key=(folder.id.clone(),original.item,skip);
    let sprite=if let Some(sprite)=a.entrance_tiles.get(&key) {Rc::clone(sprite)} else {
        let width=original.icon.w.ceil().max(1.) as usize;
        let height=(original.icon.h+skin.reflect).ceil().max(1.) as usize;
        let mut pixels=vec![0; width*height];
        let origin=Rect::new(0.,0.,width as f32,height as f32);
        let mut local=original;
        local.icon.x=0.;
        local.icon.y=0.;
        local.box_=origin;
        let mut skin=skin;skin.labels=false;
        tile(&mut Canvas::new(&mut pixels,width,height,origin),a,item,local,skin,dpi,1.,
            false,false,0.,thumb,skip,local.icon.bottom());
        let sprite=Rc::new(TileSprite {pixels,width,height});
        if a.entrance_tiles.len() >= 256 {a.entrance_tiles.clear();}
        a.entrance_tiles.insert(key,Rc::clone(&sprite));
        sprite
    };
    let opacity=bounded(alpha);
    // Fasade has a fixed cell rectangle. Copy its cached premultiplied pixels directly.
    if (k-1.).abs()<0.0001 {
        let fade=(opacity*255.).round() as u32;
        let left=dest.x.round() as i32; let top=dest.y.round() as i32;
        for y in visible.y.floor() as i32..visible.bottom().ceil() as i32 {
            let sy=y-top;
            if sy<0 || sy>=sprite.height as i32 {continue;}
            for x in visible.x.floor() as i32..visible.right().ceil() as i32 {
                let sx=x-left;
                if sx<0 || sx>=sprite.width as i32 {continue;}
                let px=sprite.pixels[sy as usize*sprite.width+sx as usize];
                if px==0 {continue;}
                let ch=|n:u32|mul255((px>>n)&255,fade);
                c.put(x,y,if fade==255 {px} else {pack(ch(24),ch(16),ch(8),ch(0))});
            }
        }
        return;
    }
    scaled_sprite(c,&sprite,dest,original.icon.w,original.icon.h+skin.reflect,opacity);
}
pub(crate) fn launch_sprite(assets:&mut Assets,item:&Entry,thumb:bool,side:f32)->Option<TileSprite>{
    let image=assets.get(item,thumb)?;
    let side=side.ceil().clamp(1.,512.) as usize;
    let rect=Rect::new(0.,0.,side as f32,side as f32);
    let mut pixels=vec![0;side*side];
    Canvas::new(&mut pixels,side,side,rect).art(&image,rect,0.,item.icon_rotation.unwrap_or(0.),1.,false);
    Some(TileSprite{width:side,height:side,pixels})
}
fn scaled_sprite(c:&mut Canvas,sprite:&TileSprite,dest:Rect,width:f32,height:f32,opacity:f32){
    let visible=dest.intersect(c.clip);
    if visible.w<=0. || visible.h<=0. || opacity<=0.{return;}
    for y in visible.y.floor() as i32..visible.bottom().ceil() as i32 {
        let sy=((y as f32+0.5-dest.y)*height/dest.h-0.5).clamp(0.,(sprite.height-1) as f32);
        let row=sy as usize;let next=(row+1).min(sprite.height-1);
        for x in visible.x.floor() as i32..visible.right().ceil() as i32 {
            let sx=((x as f32+0.5-dest.x)*width/dest.w-0.5).clamp(0.,(sprite.width-1) as f32);
            let col=sx as usize;let right=(col+1).min(sprite.width-1);
            let get=|cx:usize,cy:usize|sprite.pixels[cy*sprite.width+cx];
            let (tl,tr,bl,br)=(get(col,row),get(right,row),get(col,next),get(right,next));
            if (tl|tr|bl|br)==0 {continue;}
            let channel=|shift:u32|{
                let byte=|pixel:u32|((pixel>>shift)&255) as f32;
                (opacity*mix(mix(byte(tl),byte(tr),sx.fract()),mix(byte(bl),byte(br),sx.fract()),sy.fract())).round() as u32
            };
            c.put(x,y,pack(channel(24),channel(16),channel(8),channel(0)));
        }
    }
}

// Progress on the animation clock, before panel easing.
#[derive(Clone, Copy)]
pub struct Clock {
    pub t: f32,
    pub opening: bool,
}

fn fade(t: f32, delay: f32, span: f32) -> f32 {
    ease(bounded((t-delay)/span), 0., 0., 0.58, 1.)
}

pub struct Navigation {
    pub old: Layout,
    pub path: Vec<usize>,
    pub down: bool,
    pub slot: usize,
}

fn flight_rect(from:Rect,to:Rect,t:f32,dpi:f32)->Rect {
    let mut r=from.lerp(to,t,t);
    let dx=to.cx()-from.cx(); let dy=to.cy()-from.cy();
    if dx!=0. && dy!=0. {
        let bend=(12.*dpi).min(dx.hypot(dy)*0.04);
        let weight=if t<0.54 {t/0.54} else {(1.-t)/0.46};
        r.x+=dx.signum()*bend*weight;
    }
    r
}

// Style colors have straight RGB. Composite both independently fading glass
// layers in premultiplied space, then return the panel painter's straight RGB.
fn glass_color(base:u32, grid:u32, dock_alpha:f32, grid_alpha:f32)->u32 {
    let bottom=((base>>24) as f32/255.)*dock_alpha;
    let top=((grid>>24) as f32/255.)*grid_alpha;
    let total=top+bottom*(1.-top);
    if total<=0. {return 0;}
    let ch=|n:u32| ((((grid>>n)&255) as f32*top+((base>>n)&255) as f32*bottom*(1.-top))/total).round() as u32;
    pack((total*255.).round() as u32,ch(16),ch(8),ch(0))
}

fn header_rect(a: &mut Assets, folder: &Entry, l: &Layout) -> Rect {
    let max = (l.grid.w - 2.*l.gs.px - l.gs.title - 30.*l.s).max(13.*l.s);
    let width = a.measure(&folder.name, 13.*l.s, true, max);
    Rect::new(l.grid.cx()-(l.gs.title+8.*l.s+width)/2., l.grid.y+l.gs.py, l.gs.title,l.gs.title)
}

pub struct View<'a> {
    pub clock: Clock,
    pub nav: Option<&'a Navigation>,
    pub saved: &'a Saved,
    pub layout: &'a Layout,
    pub path: &'a [usize],
    pub dock_index: Option<usize>,
    pub p: f32,
    pub a: f32,
    pub hover: Option<Hit>,
    pub hover_levels: &'a [(Hit, f32)],
}
pub fn frame(c: &mut Canvas, a: &mut Assets, v: View) {
    scene(c,a,v,false,true);
}

pub fn content_frame(c:&mut Canvas,a:&mut Assets,v:View){scene(c,a,v,false,false);}

// Move the whole tile so its label and reflection travel with the icon.
fn dock_motion(mut tile:Tile,dock:Rect,expanded:Rect,settled:f32,edge:&str)->Tile{
    let rest=1.-bounded(settled);
    let distance=tile.icon.w*0.22;
    let (lift_x,lift_y)=match edge{"left"=>(distance,0.),"right"=>(-distance,0.),"top"=>(0.,distance),_=>(0.,-distance)};
    let dx=((lift_x+expanded.cx()-dock.cx()+(tile.icon.cx()-dock.cx())*0.30)*rest).round();
    let dy=((lift_y+expanded.cy()-dock.cy()+(tile.icon.cy()-dock.cy())*0.30)*rest).round();
    tile.box_.x+=dx;tile.box_.y+=dy;tile.icon.x+=dx;tile.icon.y+=dy;tile
}

fn scene(c: &mut Canvas, a: &mut Assets, v: View, outgoing: bool, hull:bool) {
    let View {
        clock, nav,
        saved,
        layout: l,
        path,
        dock_index,
        p,
        a: flight,
        hover,
        hover_levels,
    } = v;
    let hover_amount = |hit: Hit| {
        hover_levels
            .iter()
            .find(|entry| entry.0 == hit)
            .map_or(0., |entry| entry.1)
    };
    let s = l.s;
    let thumb = saved.general.image_thumbnails.unwrap_or(true);
    let root_clip = c.clip;
    let current = saved.folder_at(path);
    let mode = current
        .map(|f| f.entry_motion.as_str())
        .unwrap_or("classic");
    let t = clock.t;
    let opening = clock.opening;
    let nav_p = ease(t,0.22,0.76,0.26,1.);
    let panel_mode = opening && nav.is_none() && matches!(mode,"rows"|"columns"|"center"|"together");
    let slow = 1. - (1.-t).powi(4);
    let fast = 1. - (1.-bounded(t/0.42)).powi(4);
    let (x,y) = if panel_mode {
        match mode { "rows" => (fast,slow), "columns" => (slow,fast), _ => (slow,slow) }
    } else { (p,p) };
    let panel = if outgoing {root_clip} else {nav.map_or_else(|| l.dock.lerp(l.grid,x,y), |n| n.old.grid.lerp(l.grid,nav_p,nav_p))};
    let base = nav.map_or(l.ds, |n| n.old.gs);
    let blend = if nav.is_some() { nav_p } else { x.max(y) };
    if !outgoing && hull {
        let layer_color=|base,grid| {
            if nav.is_some() {return mix_color(base,grid,blend);}
            let (d,g)=if opening {(1.-fade(t,0.,225./420.),fade(t,0.,265./420.))}
                else {(fade(t,95./325.,200./325.),1.-fade(t,0.,235./325.))};
            glass_color(base,grid,d,g)
        };
        c.panel(panel,mix(base.radius,l.gs.radius,blend),mix(base.stroke,l.gs.stroke,blend),
            layer_color(base.fill,l.gs.fill),layer_color(base.border,l.gs.border),mix(base.inset_shadow,l.gs.inset_shadow,blend),s);
    }
    if let Some(n) = nav {
        if !outgoing {
            c.clip = root_clip.intersect(panel);
            scene(c,a,View {saved, layout:&n.old,path:&n.path,dock_index,p:1.,a:1.,
                clock,nav:Some(n),hover:None,hover_levels:&[]},true,false);
            c.clip = root_clip;
        }
    }
    let order = saved.ordered();
    let dock_alpha = if nav.is_some() {0.} else if opening {
        1.-fade(t,0.,145./420.)
    } else {fade(t,135./325.,145./325.)};
    c.clip = root_clip.intersect(panel);
    for &original in &l.dock_tiles {
        let t=if current.is_some() && dock_index!=Some(original.item){
            dock_motion(original,l.dock,l.grid,dock_alpha,&saved.edge)
        }else{original};
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
            hover_amount(Hit::Dock(t.item)),
            thumb,
            current.is_some() && dock_index == Some(t.item),
            t.icon.bottom(),
        );
    }
    let Some(folder) = current else {
        c.clip = root_clip;
        return;
    };
    // The Tauri grid is an inset child of the animated panel.
    let classic_slide = nav.is_none() && matches!(mode, "classic" | "");
    let slide_x = if classic_slide { panel.x - l.grid.x } else { 0. };
    let slide_y = if classic_slide { panel.y - l.grid.y } else { 0. };
    let fade_ms = if mode == "fasade" { 230. } else { 185. };
    let header_alpha = if outgoing {1.-fade(t,0.,0.55)}
        else if nav.is_some() {fade(t,0.22,0.65)}
        else if opening {fade(t,45./420.,fade_ms/420.)}
        else {1.-fade(t,0.,140./325.)};
    let font = 13. * s;
    let header_controls = if l.settings_gear.is_some() { 64. } else { 30. };
    let max = (l.grid.w - 2. * l.gs.px - l.gs.title - header_controls * s).max(font);
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
        icon_rect.right() + 8. * s + title_w / 2. + if classic_slide {(panel.w-l.grid.w)/2.+slide_x} else {0.},
        title_y + l.gs.title / 2. + font * 0.35 + slide_y,
        font,
        max,
        header_alpha,
        true,
    );
    if path.len() > 1 {
        c.text(
            a,
            "‹",
            l.back.cx() + slide_x,
            l.back.y + 21. * s + slide_y,
            19. * s,
            l.back.w,
            header_alpha,
            false,
        );
    }
    if let Some(mut gear) = l.settings_gear {
        if classic_slide {
            gear.x += panel.right() - l.grid.right();
            gear.y += slide_y;
        }
        let old_clip = c.clip;
        c.clip = root_clip.intersect(panel);
        if hover == Some(Hit::SettingsGear) {
            c.rounded(gear, 6. * s, ((38. * header_alpha).round() as u32) << 24 | 0xffffff);
        }
        let radius = 8. * s;
        let center_x = gear.cx();
        let center_y = gear.cy();
        let alpha = (255. * header_alpha).round() as u32;
        let ink = pack(alpha, mul255(242, alpha), mul255(234, alpha), mul255(246, alpha));
        for y in (center_y - radius).floor() as i32..(center_y + radius).ceil() as i32 {
            for x in (center_x - radius).floor() as i32..(center_x + radius).ceil() as i32 {
                let dx = x as f32 + 0.5 - center_x;
                let dy = y as f32 + 0.5 - center_y;
                let distance = dx.hypot(dy) / radius;
                let tooth = (dx.atan2(dy) * 8.).cos() > 0.2;
                if (0.35..=0.75).contains(&distance) || (tooth && (0.75..=1.).contains(&distance)) {
                    c.put(x, y, ink);
                }
            }
        }
        c.clip = old_clip;
    }
    let panel_clip = root_clip.intersect(panel);
    let viewport = if classic_slide {
        Rect::new(l.grid_clip.x + slide_x, l.grid_clip.y + slide_y,
            (l.grid_clip.w + panel.w - l.grid.w).max(0.),
            (l.grid_clip.h + panel.h - l.grid.h).max(0.))
    } else { l.grid_clip };
    let grid_clip = panel_clip.intersect(viewport);
    let score = |tile: &Tile| {
        let item = &folder.slots[tile.item];
        if mode == "diagonal" { (item.r + item.c) as i32 }
        else { -(tile.box_.cx()-l.grid_clip.cx()).hypot(tile.box_.cy()-l.grid_clip.cy()).round() as i32 }
    };
    let entrance = opening && !outgoing && t < 1.;
    let mut groups = Vec::new();
    if entrance && matches!(mode,"diagonal"|"outer"|"fly-in") {
        groups.extend(l.grid_tiles.iter().map(score));
        groups.sort_unstable();
        groups.dedup();
    }
    let reach = (l.grid_clip.w.min(l.grid_clip.h)*0.27).clamp(22.*s,62.*s);
    let anchor_x = if nav.is_some() {0.} else {panel.x-l.grid.x};
    let anchor_y = if nav.is_some() {0.} else {panel.y-l.grid.y};


    let hovered_grid_item = match hover {
        Some(Hit::Grid(index)) => Some(index),
        _ => None,
    };

    let regular_tiles = l.grid_tiles.iter().filter(|t| {
        Some(t.item) != hovered_grid_item
    });
    let front_tile = l.grid_tiles.iter().find(|t| {
        Some(t.item) == hovered_grid_item
    });

    if mode == "fasade" && opening && nav.is_none() && header_alpha == 0. {
        if let Some(tile)=l.grid_tiles.iter().find(|tile| !a.entrance_tiles.contains_key(&(folder.id.clone(),tile.item,false))) {
            let original_clip=c.clip;
            c.clip=Rect::new(tile.box_.x,tile.box_.y,1.,1.);
            entrance_tile(c,a,&folder.slots[tile.item],folder,*tile,tile.box_,l.gs,s,0.00001,thumb,false);
            c.clip=original_clip;
        }
    }
    for &t in regular_tiles.chain(front_tile) {
        c.clip = if Some(t.item) == hovered_grid_item || hover_amount(Hit::Grid(t.item)) != 0. {
            panel_clip
        } else {
            grid_clip
        };
        if mode == "fasade" && nav.is_none() && t.box_.intersect(c.clip).w * t.box_.intersect(c.clip).h <= 0. { continue; }
        let item = &folder.slots[t.item];
        let order = groups.binary_search(&score(&t)).unwrap_or(0) as f32 / groups.len().saturating_sub(1).max(1) as f32;
        let original=t;
        let mut t = t;
        let mut size = 1.;
        let (mut dx,mut dy) = (slide_x,slide_y);
        let mut alpha = header_alpha;
        if entrance {
            if matches!(mode,"diagonal"|"outer"|"fly-in") {
                let fly = mode == "fly-in";
                let (delay,span) = if fly {(0.28+order*0.22,0.50)} else {(0.36+order*0.23,0.4)};
                let phase = bounded((clock.t-delay)/span);
                if fly {
                    let u = ease(phase,0.18,0.75,0.28,1.);
                    let (offset,k) = if u < 0.82 {
                        (mix(1.,-0.09,u/0.82),mix(0.72,1.045,u/0.82))
                    } else { (mix(-0.09,0.,(u-0.82)/0.18),mix(1.045,1.,(u-0.82)/0.18)) };
                    let vx=t.box_.cx()-l.grid_clip.cx(); let vy=t.box_.cy()-l.grid_clip.cy();
                    let d=vx.hypot(vy);
                    if d>1. {dx=reach*offset*vx/d; dy=reach*offset*vy/d;}
                    size=k;
                } else {
                    let u=ease(phase,0.2,0.7,0.28,1.);
                    size=mix(if mode=="outer" {0.77} else {0.86},1.,u);
                    if mode=="diagonal" {dx=-(9.*s).min(reach*0.24)*(1.-u);dy=dx;}
                    alpha*=u;
                }
            } else if panel_mode {
                let cw=(l.grid_clip.w+panel.w-l.grid.w).max(1.);
                let ch=(l.grid_clip.h+panel.h-l.grid.h).max(1.);
                dx=anchor_x;dy=anchor_y;
                if mode=="center" {dx+=(cw-l.grid_clip.w)/2.;dy+=(ch-l.grid_clip.h)/2.;}
                if mode=="together" {
                    size=(cw/l.grid_clip.w.max(1.)).min(ch/l.grid_clip.h.max(1.)).clamp(0.06,1.);
                    dx+=(cw-l.grid_clip.w*size)/2.+(t.box_.cx()-l.grid_clip.x)*(size-1.);
                    dy+=(ch-l.grid_clip.h*size)/2.+(t.box_.cy()-l.grid_clip.y)*(size-1.);
                }
                c.clip=panel_clip;
            }
        }
        let transform=|r:Rect| Rect::new(t.box_.cx()+(r.x-t.box_.cx())*size+dx,
            t.box_.cy()+(r.y-t.box_.cy())*size+dy,r.w*size,r.h*size);
        let icon=transform(t.icon);
        t.box_=transform(t.box_);
        t.icon=icon;
        let caption_bottom=t.icon.bottom();
        let mut skin=l.gs;
        skin.font*=size; skin.reflect*=size;
        let skip=nav.is_some_and(|n| n.slot==t.item && (outgoing==n.down));
        if (mode == "fasade" && nav.is_none() && clock.t < 1.) || ((entrance || classic_slide) && (size != 1. || dx != 0. || dy != 0.)) {
            entrance_tile(c,a,item,folder,original,t.box_,l.gs,s,alpha,thumb,skip);
            continue;
        }
        tile(
            c,
            a,
            item,
            t,
            skin,
            s,
            alpha,
            false,
            hover == Some(Hit::Grid(t.item)),
            hover_amount(Hit::Grid(t.item)),
            thumb,
            skip,
            caption_bottom,
        );
    }
    c.clip = root_clip;
    if let Some(n)=nav {
        if outgoing {
            if n.down {if let Some(img)=a.get(folder,thumb) {c.art(&img,icon_rect,l.gs.corner,0.,header_alpha,false);}}
            return;
        }
        if !n.down {if let Some(img)=a.get(folder,thumb) {c.art(&img,icon_rect,l.gs.corner,0.,header_alpha,false);}}
        let Some(old_folder)=saved.folder_at(&n.path) else {return};
        let (moving, source, target)=if n.down {
            (folder,n.old.grid_tiles.iter().find(|t|t.item==n.slot).map(|t|t.icon),Some(icon_rect))
        } else {
            (old_folder,Some(header_rect(a,old_folder,&n.old)),l.grid_tiles.iter().find(|t|t.item==n.slot).map(|t|t.icon))
        };
        if let (Some(from),Some(to),Some(img))=(source,target,a.get(moving,thumb)){
            c.art(&img,flight_rect(from,to,nav_p,s),l.gs.corner,moving.icon_rotation.unwrap_or(0.),1.,false);
        }
        return;
    }
    if let Some(img) = a.get(folder, thumb) {
        let anchor = l
            .dock_tiles
            .iter()
            .find(|t| Some(t.item) == dock_index)
            .map(|t| t.icon);
        let fly = if path.len() == 1 {
            anchor.map_or(icon_rect, |f| flight_rect(f,icon_rect,flight,s))
        } else {
            icon_rect
        };
        let opacity = if anchor.is_some() { 1. } else { header_alpha };
        let radius = mix(corner(folder, l.ds, true), l.gs.corner, flight);
        let source_angle = folder.icon_rotation.unwrap_or(l.ds.rotation);
        let target_angle=folder.icon_rotation.unwrap_or(l.gs.rotation);
        let angle=source_angle+((target_angle-source_angle+180.).rem_euclid(360.)-180.)*flight;
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
                        clock: Clock {t:p,opening:true}, nav:None,
                        saved: &saved,
                        layout: &l,
                        path: &[0],
                        dock_index: Some(0),
                        p,
                        a: p,
                        hover: None,
                        hover_levels: &[],
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