use crate::{Painter, Rect};
use serde::Serialize;
use std::sync::Arc;

/// Packed opaque RGB color, independent of the windowing backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Color(pub u32);
impl Color {
    pub const fn rgb(rgb: u32) -> Self {
        Self(rgb & 0xffffff)
    }
    pub fn mix(self, other: Self, amount: f32) -> Self {
        let t = amount.clamp(0.0, 1.0);
        let channel = |shift: u32| {
            (((self.0 >> shift) & 255) as f32 * (1.0 - t) + ((other.0 >> shift) & 255) as f32 * t)
                as u32
        };
        Self(channel(16) << 16 | channel(8) << 8 | channel(0))
    }
}
/// Shared design tokens. All widget colors are derived from these tokens.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Color,
    pub panel: Color,
    pub elevated: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub success: Color,
}
impl Theme {
    pub const DARK: Self = Self {
        background: Color(0x101316),
        panel: Color(0x191d21),
        elevated: Color(0x242a30),
        border: Color(0x343c43),
        text: Color(0xf2f3ef),
        muted: Color(0x9ca8b2),
        accent: Color(0xff6548),
        on_accent: Color(0x17120f),
        success: Color(0x8cdbb6),
    };
    pub const LIGHT: Self = Self {
        background: Color(0xf1f2ef),
        panel: Color(0xffffff),
        elevated: Color(0xe6e9e6),
        border: Color(0xc8ceca),
        text: Color(0x182025),
        muted: Color(0x56636c),
        accent: Color(0xc73e26),
        on_accent: Color(0xffffff),
        success: Color(0x19774e),
    };
}
impl Default for Theme {
    fn default() -> Self {
        Self::DARK
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Fill,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug)]
pub struct Style {
    pub width: Length,
    pub height: Length,
    pub padding: f32,
    pub gap: f32,
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub border: bool,
    pub radius: f32,
    pub text_size: f32,
    pub bold: bool,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            width: Length::Fill,
            height: Length::Auto,
            padding: 0.0,
            gap: 0.0,
            background: None,
            foreground: None,
            border: false,
            radius: 6.0,
            text_size: 14.0,
            bold: false,
        }
    }
}
/// Custom painting is clipped to this node's rectangle. It never receives window access.
pub type PaintFn = Arc<dyn Fn(&mut Painter, Rect, &Theme) + Send + Sync>;
#[derive(Clone)]
pub enum Kind {
    Container {
        axis: Axis,
        scroll: bool,
    },
    Label,
    Button,
    Toggle(bool),
    Slider {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    TextInput {
        value: String,
        placeholder: String,
    },
    Custom {
        paint: PaintFn,
        interactive: bool,
    },
}
/// Declarative widget tree; retained interaction state lives in `Ui`, keyed by `id`.
#[derive(Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub style: Style,
    pub enabled: bool,
    pub children: Vec<Node>,
}
impl Node {
    fn new(id: impl Into<String>, label: impl Into<String>, kind: Kind) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            style: Style::default(),
            enabled: true,
            children: vec![],
        }
    }
    pub fn column(id: impl Into<String>, children: Vec<Self>) -> Self {
        Self::container(id, Axis::Vertical, children)
    }
    pub fn row(id: impl Into<String>, children: Vec<Self>) -> Self {
        Self::container(id, Axis::Horizontal, children)
    }
    fn container(id: impl Into<String>, axis: Axis, children: Vec<Self>) -> Self {
        let mut n = Self::new(
            id,
            "",
            Kind::Container {
                axis,
                scroll: false,
            },
        );
        n.children = children;
        n
    }
    /// Vertical scrolling viewport. Give it a fixed height or `fill_height`.
    pub fn scroll(id: impl Into<String>, children: Vec<Self>) -> Self {
        let mut n = Self::column(id, children);
        n.kind = Kind::Container {
            axis: Axis::Vertical,
            scroll: true,
        };
        n
    }
    pub fn label(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, text, Kind::Label)
    }
    pub fn button(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, text, Kind::Button).height(40.0)
    }
    pub fn toggle(id: impl Into<String>, label: impl Into<String>, checked: bool) -> Self {
        Self::new(id, label, Kind::Toggle(checked)).height(40.0)
    }
    /// Invalid ranges/values are rejected by `Ui::set_root`, not silently accepted.
    pub fn slider(
        id: impl Into<String>,
        label: impl Into<String>,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    ) -> Self {
        Self::new(
            id,
            label,
            Kind::Slider {
                value,
                min,
                max,
                step,
            },
        )
        .height(54.0)
    }
    pub fn text_input(
        id: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> Self {
        Self::new(
            id,
            label,
            Kind::TextInput {
                value: value.into(),
                placeholder: placeholder.into(),
            },
        )
        .height(42.0)
    }
    pub fn custom(
        id: impl Into<String>,
        label: impl Into<String>,
        interactive: bool,
        paint: impl Fn(&mut Painter, Rect, &Theme) + Send + Sync + 'static,
    ) -> Self {
        Self::new(
            id,
            label,
            Kind::Custom {
                paint: Arc::new(paint),
                interactive,
            },
        )
        .height(120.0)
    }
    pub fn width(mut self, px: f32) -> Self {
        self.style.width = Length::Px(px);
        self
    }
    pub fn height(mut self, px: f32) -> Self {
        self.style.height = Length::Px(px);
        self
    }
    pub fn auto_width(mut self) -> Self {
        self.style.width = Length::Auto;
        self
    }
    pub fn fill_height(mut self) -> Self {
        self.style.height = Length::Fill;
        self
    }
    pub fn padding(mut self, px: f32) -> Self {
        self.style.padding = px;
        self
    }
    pub fn gap(mut self, px: f32) -> Self {
        self.style.gap = px;
        self
    }
    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.style.foreground = Some(color);
        self
    }
    pub fn border(mut self) -> Self {
        self.style.border = true;
        self
    }
    pub fn radius(mut self, px: f32) -> Self {
        self.style.radius = px;
        self
    }
    pub fn font_size(mut self, px: f32) -> Self {
        self.style.text_size = px;
        self
    }
    pub fn bold(mut self) -> Self {
        self.style.bold = true;
        self
    }
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    pub fn focusable(&self) -> bool {
        self.enabled
            && matches!(
                self.kind,
                Kind::Button
                    | Kind::Toggle(_)
                    | Kind::Slider { .. }
                    | Kind::TextInput { .. }
                    | Kind::Custom {
                        interactive: true,
                        ..
                    }
            )
    }
    pub fn role(&self) -> &'static str {
        match self.kind {
            Kind::Container { scroll: true, .. } => "scroll",
            Kind::Container { .. } => "group",
            Kind::Label => "label",
            Kind::Button => "button",
            Kind::Toggle(_) => "switch",
            Kind::Slider { .. } => "slider",
            Kind::TextInput { .. } => "textbox",
            Kind::Custom { .. } => "custom",
        }
    }
}
/// Semantic result of a human or agent interaction, reduced by the application.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Action {
    pub id: String,
    pub kind: ActionKind,
}
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ActionKind {
    Activate,
    Toggle(bool),
    ChangeNumber(f32),
    ChangeText(String),
    Submit(String),
}
