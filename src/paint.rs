use crate::Color;
use fontdue::{Font, FontSettings, Metrics};
use serde::Serialize;
use std::{collections::HashMap, io::Write, path::Path};

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    pub fn inset(self, n: f32) -> Self {
        Self::new(
            self.x + n,
            self.y + n,
            (self.w - 2.0 * n).max(0.0),
            (self.h - 2.0 * n).max(0.0),
        )
    }
    pub fn intersect(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        Self::new(
            x,
            y,
            (self.x + self.w).min(other.x + other.w).max(x) - x,
            (self.y + self.h).min(other.y + other.h).max(y) - y,
        )
    }
}
/// CPU rasterizer. All public drawing coordinates are logical pixels.
/// Glyphs are cached by character, weight and physical pixel size.
pub struct Painter {
    pub pixels: Vec<u32>,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    fonts: [Font; 2],
    glyphs: HashMap<(char, u32, bool), (Metrics, Vec<u8>)>,
    clip: Rect,
}
impl Default for Painter {
    fn default() -> Self {
        Self::new()
    }
}
impl Painter {
    pub fn new() -> Self {
        Self {
            pixels: vec![],
            width: 0,
            height: 0,
            scale: 1.0,
            fonts: [
                Font::from_bytes(
                    include_bytes!("../assets/DejaVuSans.ttf") as &[u8],
                    FontSettings::default(),
                )
                .unwrap(),
                Font::from_bytes(
                    include_bytes!("../assets/DejaVuSans-Bold.ttf") as &[u8],
                    FontSettings::default(),
                )
                .unwrap(),
            ],
            glyphs: HashMap::new(),
            clip: Rect::default(),
        }
    }
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale.max(0.25);
        self.pixels.resize(width as usize * height as usize, 0);
        self.clip = Rect::new(
            0.0,
            0.0,
            width as f32 / self.scale,
            height as f32 / self.scale,
        );
    }
    pub fn clear(&mut self, c: Color) {
        self.pixels.fill(c.0);
    }
    pub fn set_clip(&mut self, rect: Rect) {
        self.clip = rect.intersect(Rect::new(
            0.0,
            0.0,
            self.width as f32 / self.scale,
            self.height as f32 / self.scale,
        ));
    }
    pub fn clip(&self) -> Rect {
        self.clip
    }
    pub fn measure(&self, text: &str, size: f32, bold: bool) -> f32 {
        let f = &self.fonts[bold as usize];
        let mut prev = None;
        let mut x = 0.0;
        for ch in text.chars() {
            if let Some(p) = prev {
                x += f.horizontal_kern(p, ch, size).unwrap_or(0.0);
            }
            x += f.metrics(ch, size).advance_width;
            prev = Some(ch);
        }
        x
    }
    fn blend(&mut self, x: i32, y: i32, c: Color, alpha: u8) {
        if x < 0
            || y < 0
            || x >= self.width as i32
            || y >= self.height as i32
            || !self
                .clip
                .contains((x as f32 + 0.5) / self.scale, (y as f32 + 0.5) / self.scale)
        {
            return;
        }
        let i = y as usize * self.width as usize + x as usize;
        self.pixels[i] = Color(self.pixels[i]).mix(c, alpha as f32 / 255.0).0;
    }
    pub fn rect(&mut self, r: Rect, c: Color, radius: f32) {
        let clip = r.intersect(self.clip);
        let s = self.scale;
        let rad = radius.max(0.0).min(r.w / 2.0).min(r.h / 2.0);
        for y in (clip.y * s).floor().max(0.0) as i32
            ..((clip.y + clip.h) * s).ceil().min(self.height as f32) as i32
        {
            for x in (clip.x * s).floor().max(0.0) as i32
                ..((clip.x + clip.w) * s).ceil().min(self.width as f32) as i32
            {
                let px = (x as f32 + 0.5) / s;
                let py = (y as f32 + 0.5) / s;
                let cx = px.clamp(r.x + rad, r.x + r.w - rad);
                let cy = py.clamp(r.y + rad, r.y + r.h - rad);
                let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                let a = if rad == 0.0 {
                    1.0
                } else {
                    ((rad - d) * s + 0.5).clamp(0.0, 1.0)
                };
                if a > 0.0 {
                    self.blend(x, y, c, (a * 255.0) as u8);
                }
            }
        }
    }
    pub fn outline(&mut self, r: Rect, c: Color) {
        self.rect(Rect::new(r.x, r.y, r.w, 1.0), c, 0.0);
        self.rect(Rect::new(r.x, r.y + r.h - 1.0, r.w, 1.0), c, 0.0);
        self.rect(Rect::new(r.x, r.y, 1.0, r.h), c, 0.0);
        self.rect(Rect::new(r.x + r.w - 1.0, r.y, 1.0, r.h), c, 0.0);
    }
    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, c: Color, bold: bool) {
        let s = self.scale;
        let size_px = (size * s * 64.0).round() as u32;
        let mut pen = x * s;
        let baseline = (y + size) * s;
        let mut prev = None;
        for ch in text.chars() {
            let f = &self.fonts[bold as usize];
            if let Some(p) = prev {
                pen += f
                    .horizontal_kern(p, ch, size_px as f32 / 64.0)
                    .unwrap_or(0.0);
            }
            let key = (ch, size_px, bold);
            if !self.glyphs.contains_key(&key) {
                // Bound the cache even when an app cycles through arbitrary font sizes.
                if self.glyphs.len() > 8192 {
                    self.glyphs.clear();
                }
                self.glyphs.insert(
                    key,
                    self.fonts[bold as usize].rasterize(ch, size_px as f32 / 64.0),
                );
            }
            let (m, bitmap) = &self.glyphs[&key];
            let m = *m;
            let gx = pen.round() as i32 + m.xmin;
            let gy = baseline.round() as i32 - m.ymin - m.height as i32;
            // Mutate pixels directly so the cached bitmap is never cloned per glyph.
            for yy in 0..m.height {
                for xx in 0..m.width {
                    let px = gx + xx as i32;
                    let py = gy + yy as i32;
                    if px < 0
                        || py < 0
                        || px >= self.width as i32
                        || py >= self.height as i32
                        || !self
                            .clip
                            .contains((px as f32 + 0.5) / s, (py as f32 + 0.5) / s)
                    {
                        continue;
                    }
                    let a = bitmap[yy * m.width + xx];
                    let i = py as usize * self.width as usize + px as usize;
                    if a > 0 {
                        self.pixels[i] = Color(self.pixels[i]).mix(c, a as f32 / 255.0).0;
                    }
                }
            }
            pen += m.advance_width;
            prev = Some(ch);
        }
    }
    /// Portable lossless screenshot. Refuses to overwrite an existing file.
    pub fn save_ppm(&self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        let mut out = std::io::BufWriter::new(file);
        write!(out, "P6\n{} {}\n255\n", self.width, self.height)?;
        for p in &self.pixels {
            out.write_all(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8])?;
        }
        out.flush()
    }
}
