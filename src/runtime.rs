use crate::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
pub enum Input {
    Move(f32, f32),
    Down,
    Up,
    Wheel(f32),
    Text(String),
    Key {
        key: String,
        shift: bool,
        command: bool,
    },
    Cancel,
}
#[derive(Clone)]
pub struct LayoutItem {
    pub node: Node,
    pub rect: Rect,
    pub clip: Rect,
    pub parent: Option<usize>,
    pub scroll_max: f32,
}
#[derive(Clone, Default)]
struct Edit {
    cursor: usize,
    anchor: usize,
}
impl Edit {
    fn range(&self) -> std::ops::Range<usize> {
        self.cursor.min(self.anchor)..self.cursor.max(self.anchor)
    }
    fn clamp(&mut self, text: &str) {
        let boundaries = boundaries(text);
        self.cursor = *boundaries
            .iter()
            .rfind(|&&i| i <= self.cursor.min(text.len()))
            .unwrap_or(&0);
        self.anchor = *boundaries
            .iter()
            .rfind(|&&i| i <= self.anchor.min(text.len()))
            .unwrap_or(&0);
    }
}
fn boundaries(s: &str) -> Vec<usize> {
    s.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain(std::iter::once(s.len()))
        .collect()
}
/// Headless-capable layout and interaction runtime. Native and agent input share it.
pub struct Ui {
    root: Node,
    pub items: Vec<LayoutItem>,
    pub theme: Theme,
    pub focus: Option<String>,
    pub hover: Option<String>,
    pressed: Option<String>,
    pointer: (f32, f32),
    edits: HashMap<String, Edit>,
    scroll: HashMap<String, f32>,
    viewport: Rect,
    pub painter: Painter,
    pub debug_bounds: bool,
}
impl Ui {
    pub fn new(root: Node) -> Result<Self, String> {
        validate(&root)?;
        Ok(Self {
            root,
            items: vec![],
            theme: Theme::default(),
            focus: None,
            hover: None,
            pressed: None,
            pointer: (-1.0, -1.0),
            edits: HashMap::new(),
            scroll: HashMap::new(),
            viewport: Rect::default(),
            painter: Painter::new(),
            debug_bounds: false,
        })
    }
    pub fn set_root(&mut self, root: Node) -> Result<(), String> {
        validate(&root)?;
        self.root = root;
        self.layout();
        let ids: HashSet<_> = self.items.iter().map(|i| i.node.id.clone()).collect();
        self.edits.retain(|id, _| ids.contains(id));
        self.scroll.retain(|id, _| ids.contains(id));
        if self.focus.as_ref().is_some_and(|id| {
            !self
                .items
                .iter()
                .any(|i| &i.node.id == id && i.node.focusable())
        }) {
            self.focus = None;
        }
        if self.pressed.as_ref().is_some_and(|id| {
            !self
                .items
                .iter()
                .any(|i| &i.node.id == id && i.node.focusable())
        }) {
            self.pressed = None;
        }
        for item in &self.items {
            if let Kind::TextInput { value, .. } = &item.node.kind {
                if let Some(e) = self.edits.get_mut(&item.node.id) {
                    e.clamp(value);
                }
            }
        }
        Ok(())
    }
    pub fn resize(&mut self, width: u32, height: u32, scale: f32) {
        self.painter.resize(width, height, scale);
        self.viewport = Rect::new(
            0.0,
            0.0,
            width as f32 / self.painter.scale,
            height as f32 / self.painter.scale,
        );
        self.layout();
    }
    fn natural(&self, n: &Node) -> (f32, f32) {
        let p = n.style.padding * 2.0;
        let size = match &n.kind {
            Kind::Container { axis, .. } => {
                let sizes: Vec<_> = n.children.iter().map(|c| self.natural(c)).collect();
                let gap = n.style.gap * sizes.len().saturating_sub(1) as f32;
                if *axis == Axis::Vertical {
                    (
                        sizes.iter().map(|s| s.0).fold(0.0, f32::max) + p,
                        sizes.iter().map(|s| s.1).sum::<f32>() + gap + p,
                    )
                } else {
                    (
                        sizes.iter().map(|s| s.0).sum::<f32>() + gap + p,
                        sizes.iter().map(|s| s.1).fold(0.0, f32::max) + p,
                    )
                }
            }
            Kind::Label => (
                self.painter
                    .measure(&n.label, n.style.text_size, n.style.bold)
                    + p,
                n.style.text_size * 1.5 + p,
            ),
            _ => (
                self.painter
                    .measure(&n.label, n.style.text_size, n.style.bold)
                    + 40.0,
                42.0,
            ),
        };
        (
            if let Length::Px(w) = n.style.width {
                w
            } else {
                size.0
            },
            if let Length::Px(h) = n.style.height {
                h
            } else {
                size.1
            },
        )
    }
    pub fn layout(&mut self) {
        self.items.clear();
        self.place(self.root.clone(), self.viewport, self.viewport, None, true);
    }
    fn place(&mut self, mut n: Node, r: Rect, clip: Rect, parent: Option<usize>, enabled: bool) {
        n.enabled &= enabled;
        let idx = self.items.len();
        let ownclip = r.intersect(clip);
        let inner = r.inset(n.style.padding);
        self.items.push(LayoutItem {
            node: n.clone(),
            rect: r,
            clip: ownclip,
            parent,
            scroll_max: 0.0,
        });
        if let Kind::Container { axis, scroll } = &n.kind {
            let vertical = *axis == Axis::Vertical;
            let available = if vertical { inner.h } else { inner.w };
            let natural: Vec<_> = n.children.iter().map(|c| self.natural(c)).collect();
            let fills = n
                .children
                .iter()
                .filter(|c| {
                    matches!(
                        if vertical {
                            c.style.height
                        } else {
                            c.style.width
                        },
                        Length::Fill
                    )
                })
                .count();
            let fixed: f32 = n
                .children
                .iter()
                .zip(&natural)
                .filter(|(c, _)| {
                    !matches!(
                        if vertical {
                            c.style.height
                        } else {
                            c.style.width
                        },
                        Length::Fill
                    )
                })
                .map(|(_, s)| if vertical { s.1 } else { s.0 })
                .sum();
            let gaps = n.style.gap * n.children.len().saturating_sub(1) as f32;
            let flex = (available - fixed - gaps).max(0.0) / fills.max(1) as f32;
            let total = fixed + gaps + flex * fills as f32;
            let maxscroll = if *scroll {
                (total - available).max(0.0)
            } else {
                0.0
            };
            let offset = if *scroll {
                let o = self.scroll.entry(n.id.clone()).or_default();
                *o = o.clamp(0.0, maxscroll);
                *o
            } else {
                0.0
            };
            self.items[idx].scroll_max = maxscroll;
            let mut pos = if vertical { inner.y } else { inner.x } - offset;
            for (c, s) in n.children.iter().zip(natural) {
                let main = if matches!(
                    if vertical {
                        c.style.height
                    } else {
                        c.style.width
                    },
                    Length::Fill
                ) {
                    flex
                } else if vertical {
                    s.1
                } else {
                    s.0
                };
                let cross = if vertical {
                    match c.style.width {
                        Length::Fill => inner.w,
                        Length::Px(v) => v,
                        Length::Auto => s.0,
                    }
                } else {
                    match c.style.height {
                        Length::Fill => inner.h,
                        Length::Px(v) => v,
                        Length::Auto => s.1,
                    }
                };
                let cr = if vertical {
                    Rect::new(inner.x, pos, cross, main)
                } else {
                    Rect::new(pos, inner.y, main, cross)
                };
                self.place(
                    c.clone(),
                    cr,
                    ownclip.intersect(inner),
                    Some(idx),
                    n.enabled,
                );
                pos += main + n.style.gap;
            }
        }
    }
    fn hit(&self) -> Option<usize> {
        self.items
            .iter()
            .rposition(|i| i.node.focusable() && i.clip.contains(self.pointer.0, self.pointer.1))
    }
    pub fn item(&self, id: &str) -> Option<&LayoutItem> {
        self.items.iter().find(|i| i.node.id == id)
    }
    fn activate(n: &Node) -> Vec<Action> {
        let kind = match &n.kind {
            Kind::Button
            | Kind::Custom {
                interactive: true, ..
            } => ActionKind::Activate,
            Kind::Toggle(v) => ActionKind::Toggle(!v),
            _ => return vec![],
        };
        vec![Action {
            id: n.id.clone(),
            kind,
        }]
    }
    fn slider(n: &Node, v: f32) -> Vec<Action> {
        if let Kind::Slider { min, max, step, .. } = n.kind {
            let value = (min + ((v - min) / step).round() * step).clamp(min, max);
            vec![Action {
                id: n.id.clone(),
                kind: ActionKind::ChangeNumber(value),
            }]
        } else {
            vec![]
        }
    }
    fn drag(&self, i: &LayoutItem) -> Vec<Action> {
        if let Kind::Slider { min, max, .. } = i.node.kind {
            let t =
                ((self.pointer.0 - i.rect.x - 10.0) / (i.rect.w - 20.0).max(1.0)).clamp(0.0, 1.0);
            Self::slider(&i.node, min + t * (max - min))
        } else {
            vec![]
        }
    }
    fn set_focus(&mut self, id: Option<String>) {
        self.focus = id;
        if let Some(id) = self.focus.clone() {
            if let Some(item) = self.item(&id) {
                if let Kind::TextInput { value, .. } = &item.node.kind {
                    let len = value.len();
                    self.edits.entry(id.clone()).or_insert(Edit {
                        cursor: len,
                        anchor: len,
                    });
                }
            }
            if let Some(index) = self.items.iter().position(|i| i.node.id == id) {
                let r = self.items[index].rect;
                let mut parent = self.items[index].parent;
                while let Some(p) = parent {
                    let i = &self.items[p];
                    if matches!(i.node.kind, Kind::Container { scroll: true, .. }) {
                        let inner = i.rect.inset(i.node.style.padding);
                        let delta = if r.y < inner.y {
                            r.y - inner.y
                        } else if r.y + r.h > inner.y + inner.h {
                            r.y + r.h - inner.y - inner.h
                        } else {
                            0.0
                        };
                        *self.scroll.entry(i.node.id.clone()).or_default() += delta;
                    }
                    parent = i.parent;
                }
                self.layout();
            }
        }
    }
    fn pointer_edit(&mut self, item: &LayoutItem, extend: bool) -> bool {
        let Kind::TextInput { value, .. } = &item.node.kind else {
            return false;
        };
        let current = self.edits.get(&item.node.id).cloned().unwrap_or(Edit {
            cursor: value.len(),
            anchor: value.len(),
        });
        let size = item.node.style.text_size;
        let caret = self.painter.measure(&value[..current.cursor], size, false);
        let offset = (caret - (item.rect.w - 20.0) + 2.0).max(0.0);
        let target = self.pointer.0 - item.rect.x - 10.0 + offset;
        let index = boundaries(value)
            .into_iter()
            .min_by(|a, b| {
                let da = (self.painter.measure(&value[..*a], size, false) - target).abs();
                let db = (self.painter.measure(&value[..*b], size, false) - target).abs();
                da.total_cmp(&db)
            })
            .unwrap_or(0);
        let edit = self.edits.entry(item.node.id.clone()).or_default();
        edit.cursor = index;
        if !extend {
            edit.anchor = index;
        }
        true
    }
    pub fn event(&mut self, input: Input) -> Vec<Action> {
        match input {
            Input::Move(x, y) => {
                self.pointer = (x, y);
                self.hover = self.hit().map(|i| self.items[i].node.id.clone());
                if let Some(i) = self.pressed.as_ref().and_then(|id| self.item(id)).cloned() {
                    if !self.pointer_edit(&i, true) {
                        return self.drag(&i);
                    }
                }
            }
            Input::Down => {
                let hit = self.hit();
                self.pressed = hit.map(|i| self.items[i].node.id.clone());
                self.set_focus(self.pressed.clone());
                if let Some(i) = self.pressed.as_ref().and_then(|id| self.item(id)).cloned() {
                    if !self.pointer_edit(&i, false) {
                        return self.drag(&i);
                    }
                }
            }
            Input::Up => {
                let pressed = self.pressed.take();
                if let Some(i) = self.hit() {
                    if pressed.as_ref() == Some(&self.items[i].node.id) {
                        return Self::activate(&self.items[i].node);
                    }
                }
            }
            Input::Cancel => {
                self.pressed = None;
                self.hover = None;
            }
            Input::Wheel(delta) => {
                if !delta.is_finite() {
                    return vec![];
                }
                if let Some(i) = self.items.iter().rposition(|i| {
                    i.clip.contains(self.pointer.0, self.pointer.1)
                        && matches!(i.node.kind, Kind::Container { scroll: true, .. })
                }) {
                    let item = &self.items[i];
                    let o = self.scroll.entry(item.node.id.clone()).or_default();
                    *o = (*o + delta).clamp(0.0, item.scroll_max);
                    self.layout();
                }
            }
            Input::Text(text) => return self.edit_text(Some(text), "", false, false),
            Input::Key {
                key,
                shift,
                command,
            } => {
                if key == "Tab" {
                    let ids: Vec<_> = self
                        .items
                        .iter()
                        .filter(|i| i.node.focusable())
                        .map(|i| i.node.id.clone())
                        .collect();
                    if !ids.is_empty() {
                        let current = self
                            .focus
                            .as_ref()
                            .and_then(|id| ids.iter().position(|v| v == id));
                        let next = match current {
                            None => {
                                if shift {
                                    ids.len() - 1
                                } else {
                                    0
                                }
                            }
                            Some(i) => {
                                if shift {
                                    (i + ids.len() - 1) % ids.len()
                                } else {
                                    (i + 1) % ids.len()
                                }
                            }
                        };
                        self.set_focus(Some(ids[next].clone()));
                    }
                } else if let Some(n) = self
                    .focus
                    .as_ref()
                    .and_then(|id| self.item(id))
                    .map(|i| i.node.clone())
                {
                    match &n.kind {
                        Kind::TextInput { .. } => {
                            return self.edit_text(None, &key, shift, command)
                        }
                        Kind::Slider {
                            value,
                            min,
                            max,
                            step,
                        } => {
                            let v = match key.as_str() {
                                "ArrowLeft" | "ArrowDown" => value - step,
                                "ArrowRight" | "ArrowUp" => value + step,
                                "Home" => *min,
                                "End" => *max,
                                _ => return vec![],
                            };
                            return Self::slider(&n, v);
                        }
                        _ => {
                            if key == "Enter" || key == "Space" {
                                return Self::activate(&n);
                            }
                        }
                    }
                }
            }
        }
        vec![]
    }
    fn edit_text(
        &mut self,
        text: Option<String>,
        key: &str,
        shift: bool,
        command: bool,
    ) -> Vec<Action> {
        let Some(id) = self.focus.clone() else {
            return vec![];
        };
        let Some(item) = self.item(&id) else {
            return vec![];
        };
        let Kind::TextInput { value, .. } = &item.node.kind else {
            return vec![];
        };
        let mut value = value.clone();
        let e = self.edits.entry(id.clone()).or_insert(Edit {
            cursor: value.len(),
            anchor: value.len(),
        });
        e.clamp(&value);
        let original = value.clone();
        let points = boundaries(&value);
        let prev = *points.iter().rfind(|&&p| p < e.cursor).unwrap_or(&0);
        let next = *points
            .iter()
            .find(|&&p| p > e.cursor)
            .unwrap_or(&value.len());
        if let Some(text) = text {
            let text: String = text.chars().filter(|c| !c.is_control()).collect();
            let range = e.range();
            let candidate_len = value.len() - range.len() + text.len();
            if candidate_len <= 16384 {
                let start = range.start;
                value.replace_range(range, &text);
                e.cursor = start + text.len();
                e.anchor = e.cursor;
            }
        } else {
            match key {
                "a" | "A" if command => {
                    e.anchor = 0;
                    e.cursor = value.len();
                }
                "ArrowLeft" => {
                    e.cursor = if !shift && e.cursor != e.anchor {
                        e.range().start
                    } else {
                        prev
                    };
                    if !shift {
                        e.anchor = e.cursor;
                    }
                }
                "ArrowRight" => {
                    e.cursor = if !shift && e.cursor != e.anchor {
                        e.range().end
                    } else {
                        next
                    };
                    if !shift {
                        e.anchor = e.cursor;
                    }
                }
                "Home" => {
                    e.cursor = 0;
                    if !shift {
                        e.anchor = 0;
                    }
                }
                "End" => {
                    e.cursor = value.len();
                    if !shift {
                        e.anchor = e.cursor;
                    }
                }
                "Backspace" | "Delete" => {
                    let range = if e.cursor != e.anchor {
                        e.range()
                    } else if key == "Backspace" {
                        prev..e.cursor
                    } else {
                        e.cursor..next
                    };
                    e.cursor = range.start;
                    e.anchor = e.cursor;
                    value.replace_range(range, "");
                }
                "Enter" => {
                    return vec![Action {
                        id,
                        kind: ActionKind::Submit(value),
                    }]
                }
                _ => {}
            }
        }
        if value != original {
            vec![Action {
                id,
                kind: ActionKind::ChangeText(value),
            }]
        } else {
            vec![]
        }
    }
    /// Validated agent action. `inspect`, screenshots and quit are handled by the window adapter.
    pub fn agent_action(
        &mut self,
        op: &str,
        id: &str,
        text: Option<&str>,
        value: Option<f32>,
    ) -> Result<Vec<Action>, String> {
        let item = self
            .item(id)
            .ok_or_else(|| format!("unknown widget: {id}"))?
            .clone();
        if !item.node.focusable() {
            return Err(format!("widget is disabled or not actionable: {id}"));
        }
        match op {
            "click"
                if matches!(
                    item.node.kind,
                    Kind::Button
                        | Kind::Toggle(_)
                        | Kind::Custom {
                            interactive: true,
                            ..
                        }
                ) =>
            {
                self.set_focus(Some(id.into()));
                Ok(Self::activate(&item.node))
            }
            "set_text" if matches!(item.node.kind, Kind::TextInput { .. }) => {
                let text = text.ok_or("text is required")?;
                if text.len() > 16384 || text.chars().any(char::is_control) {
                    return Err("text must be a single line, at most 16384 bytes".into());
                }
                self.set_focus(Some(id.into()));
                self.edit_text(None, "a", false, true);
                Ok(self.edit_text(Some(text.into()), "", false, false))
            }
            "set_value" => {
                let v = value.ok_or("value is required")?;
                if let Kind::Slider { min, max, .. } = item.node.kind {
                    if !v.is_finite() || v < min || v > max {
                        return Err("value outside slider range".into());
                    }
                    self.set_focus(Some(id.into()));
                    Ok(Self::slider(&item.node, v))
                } else {
                    Err("set_value requires a slider".into())
                }
            }
            _ => Err(format!("{op} is not supported by {}", item.node.role())),
        }
    }
    pub fn inspect(&self) -> Value {
        json!({"protocol":"forge-ui/1","viewport":self.viewport,"scale":self.painter.scale,"focus":self.focus,"nodes":self.items.iter().map(|i|{
            let n=&i.node;let (value,range)=match &n.kind{Kind::Toggle(v)=>(json!(v),Value::Null),Kind::Slider{value,min,max,step}=>(json!(value),json!({"min":min,"max":max,"step":step})),Kind::TextInput{value,..}=>(json!(value),Value::Null),_=>(Value::Null,Value::Null)};
            let actions=if !n.focusable(){vec![]}else{match n.kind{Kind::TextInput{..}=>vec!["set_text","key"],Kind::Slider{..}=>vec!["set_value","key"],_=>vec!["click","key"]}};
            json!({"id":n.id,"parent":i.parent.map(|p|self.items[p].node.id.as_str()),"role":n.role(),"label":n.label,"value":value,"range":range,"bounds":i.rect,"visible_bounds":i.clip,"visible":i.clip.w>0.0&&i.clip.h>0.0,"enabled":n.enabled,"focused":self.focus.as_ref()==Some(&n.id),"actions":actions,"scroll_offset":self.scroll.get(&n.id).copied().unwrap_or(0.0),"scroll_max":i.scroll_max})
        }).collect::<Vec<_>>()})
    }
    pub fn paint(&mut self) {
        self.painter.clear(self.theme.background);
        let t = self.theme;
        for item in &self.items {
            let n = &item.node;
            let r = item.rect;
            if item.clip.w <= 0.0 || item.clip.h <= 0.0 {
                continue;
            }
            self.painter.set_clip(item.clip);
            let p = &mut self.painter;
            let hovered = self.hover.as_ref() == Some(&n.id);
            let focused = self.focus.as_ref() == Some(&n.id);
            let down = self.pressed.as_ref() == Some(&n.id);
            let fg = if n.enabled {
                n.style.foreground.unwrap_or(t.text)
            } else {
                t.muted.mix(t.panel, 0.45)
            };
            if let Some(bg) = n.style.background {
                p.rect(r, bg, n.style.radius);
            }
            if n.style.border {
                p.outline(r, t.border);
            }
            match &n.kind {
                Kind::Container { scroll, .. } => {
                    if *scroll && item.scroll_max > 0.0 {
                        let inner = r.inset(n.style.padding);
                        let h = (inner.h * inner.h / (inner.h + item.scroll_max)).max(18.0);
                        let offset = self.scroll.get(&n.id).copied().unwrap_or(0.0);
                        p.rect(
                            Rect::new(
                                r.x + r.w - 4.0,
                                inner.y + offset / item.scroll_max * (inner.h - h),
                                3.0,
                                h,
                            ),
                            t.border,
                            1.0,
                        );
                    }
                }
                Kind::Label => p.text(
                    &n.label,
                    r.x + n.style.padding,
                    r.y + n.style.padding,
                    n.style.text_size,
                    fg,
                    n.style.bold,
                ),
                Kind::Button => {
                    let bg =
                        n.style
                            .background
                            .unwrap_or(if n.enabled { t.elevated } else { t.panel });
                    p.rect(
                        r,
                        if down {
                            bg.mix(t.accent, 0.35)
                        } else if hovered {
                            bg.mix(t.text, 0.09)
                        } else {
                            bg
                        },
                        n.style.radius,
                    );
                    let w = p.measure(&n.label, n.style.text_size, n.style.bold);
                    p.text(
                        &n.label,
                        r.x + (r.w - w) / 2.0,
                        r.y + (r.h - n.style.text_size * 1.3) / 2.0,
                        n.style.text_size,
                        fg,
                        n.style.bold,
                    );
                }
                Kind::Toggle(v) => {
                    p.text(&n.label, r.x, r.y + 9.0, n.style.text_size, fg, false);
                    let x = r.x + r.w - 46.0;
                    p.rect(
                        Rect::new(x, r.y + 8.0, 44.0, 24.0),
                        if *v { t.accent } else { t.border },
                        12.0,
                    );
                    p.rect(
                        Rect::new(x + if *v { 23.0 } else { 3.0 }, r.y + 11.0, 18.0, 18.0),
                        if *v { t.on_accent } else { t.text },
                        9.0,
                    );
                }
                Kind::Slider {
                    value, min, max, ..
                } => {
                    p.text(&n.label, r.x, r.y, n.style.text_size, fg, false);
                    let s = format!("{value:.0}");
                    let w = p.measure(&s, 13.0, true);
                    p.text(&s, r.x + r.w - w, r.y, 13.0, t.accent, true);
                    let x = r.x + 10.0;
                    let width = (r.w - 20.0).max(0.0);
                    let frac = (value - min) / (max - min);
                    p.rect(Rect::new(x, r.y + 37.0, width, 3.0), t.border, 1.0);
                    p.rect(Rect::new(x, r.y + 37.0, width * frac, 3.0), t.accent, 1.0);
                    p.rect(
                        Rect::new(x + width * frac - 6.0, r.y + 32.0, 12.0, 12.0),
                        t.accent,
                        3.0,
                    );
                }
                Kind::TextInput { value, placeholder } => {
                    p.rect(r, t.background, n.style.radius);
                    p.outline(r, if focused { t.accent } else { t.border });
                    let area = r.inset(10.0);
                    p.set_clip(item.clip.intersect(area));
                    let e = self.edits.get(&n.id).cloned().unwrap_or(Edit {
                        cursor: value.len(),
                        anchor: value.len(),
                    });
                    let caret = p.measure(
                        &value[..e.cursor.min(value.len())],
                        n.style.text_size,
                        false,
                    );
                    let off = if focused {
                        (caret - area.w + 2.0).max(0.0)
                    } else {
                        0.0
                    };
                    if focused && e.cursor != e.anchor {
                        let sel = e.range();
                        let a = p.measure(&value[..sel.start], n.style.text_size, false);
                        let b = p.measure(&value[..sel.end], n.style.text_size, false);
                        p.rect(
                            Rect::new(area.x + a - off, r.y + 9.0, b - a, 24.0),
                            t.accent.mix(t.background, 0.7),
                            0.0,
                        );
                    }
                    p.text(
                        if value.is_empty() { placeholder } else { value },
                        area.x - off,
                        r.y + 10.0,
                        n.style.text_size,
                        if value.is_empty() { t.muted } else { fg },
                        false,
                    );
                    if focused {
                        p.rect(
                            Rect::new(area.x + caret - off, r.y + 10.0, 1.0, 22.0),
                            t.accent,
                            0.0,
                        );
                    }
                    p.set_clip(item.clip);
                }
                Kind::Custom { paint, .. } => paint(p, r, &t),
            }
            p.set_clip(item.clip);
            if focused {
                p.outline(r.inset(1.0), t.accent);
            }
            if self.debug_bounds {
                p.outline(r, t.success);
            }
        }
    }
}
fn validate(root: &Node) -> Result<(), String> {
    fn visit(n: &Node, ids: &mut HashSet<String>) -> Result<(), String> {
        if n.id.is_empty() || !ids.insert(n.id.clone()) {
            return Err(format!("empty or duplicate widget ID: {}", n.id));
        }
        let s = &n.style;
        let mut numbers = vec![s.padding, s.gap, s.radius, s.text_size];
        if let Length::Px(v) = s.width {
            numbers.push(v);
        }
        if let Length::Px(v) = s.height {
            numbers.push(v);
        }
        if numbers.iter().any(|v| !v.is_finite() || *v < 0.0) || s.text_size > 256.0 {
            return Err(format!("invalid style: {}", n.id));
        }
        if let Kind::Slider {
            value,
            min,
            max,
            step,
        } = &n.kind
        {
            if [value, min, max, step].iter().any(|v| !v.is_finite())
                || min >= max
                || *step <= 0.0
                || value < min
                || value > max
            {
                return Err(format!("invalid slider: {}", n.id));
            }
        }
        if let Kind::TextInput { value, .. } = &n.kind {
            if value.len() > 16384 || value.chars().any(char::is_control) {
                return Err(format!("invalid single-line text: {}", n.id));
            }
        }
        for c in &n.children {
            visit(c, ids)?;
        }
        Ok(())
    }
    visit(root, &mut HashSet::new())
}
