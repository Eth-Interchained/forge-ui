use forge_ui::*;
struct Counter(i32);
impl Application for Counter {
    fn view(&self) -> Node {
        Node::column(
            "root",
            vec![
                Node::label("title", "Hello, Forge.").font_size(32.0).bold(),
                Node::label("count", format!("Count: {}", self.0)),
                Node::button("add", "Add one").width(180.0),
            ],
        )
        .padding(32.0)
        .gap(16.0)
    }
    fn update(&mut self, a: Action) {
        if a.id == "add" {
            self.0 += 1;
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(
        Counter(0),
        WindowOptions {
            title: "My first Forge app".into(),
            ..Default::default()
        },
    )
}
