use forge_ui::*;
fn meter(level: f32) -> Node {
    Node::custom("meter", "Load meter", false, move |p, r, t| {
        p.rect(r, t.elevated, 4.0);
        p.rect(
            Rect::new(r.x, r.y, r.w * level.clamp(0.0, 1.0), r.h),
            t.accent,
            4.0,
        );
    })
    .height(18.0)
}
struct Meter(f32);
impl Application for Meter {
    fn view(&self) -> Node {
        Node::column(
            "root",
            vec![
                Node::label("title", "Your own paint, Forge's layout.")
                    .font_size(24.0)
                    .bold(),
                meter(self.0),
                Node::slider("level", "Load", self.0 * 100.0, 0.0, 100.0, 1.0),
            ],
        )
        .padding(32.0)
        .gap(24.0)
    }
    fn update(&mut self, a: Action) {
        if let ActionKind::ChangeNumber(v) = a.kind {
            self.0 = v / 100.0;
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(Meter(0.64), WindowOptions::default())
}
