use forge_ui::*;
fn fixture(text: &str) -> Node {
    Node::column(
        "root",
        vec![
            Node::button("one", "One"),
            Node::button("off", "Off").disabled(),
            Node::text_input("text", "Name", text, ""),
            Node::slider("slider", "Level", 50.0, 0.0, 100.0, 5.0),
            Node::toggle("switch", "Switch", false),
        ],
    )
    .padding(10.0)
    .gap(8.0)
}
fn ui() -> Ui {
    let mut ui = Ui::new(fixture("hello")).unwrap();
    ui.resize(400, 400, 1.0);
    ui
}
fn key(ui: &mut Ui, k: &str, shift: bool, command: bool) -> Vec<Action> {
    ui.event(Input::Key {
        key: k.into(),
        shift,
        command,
    })
}
#[test]
fn flex_rows_and_padding() {
    let mut ui = Ui::new(
        Node::row(
            "root",
            vec![
                Node::button("a", "A").width(80.0),
                Node::button("b", "B"),
                Node::button("c", "C"),
            ],
        )
        .padding(10.0)
        .gap(10.0),
    )
    .unwrap();
    ui.resize(400, 100, 1.0);
    assert_eq!(
        ui.item("a").unwrap().rect,
        Rect::new(10.0, 10.0, 80.0, 40.0)
    );
    assert_eq!(ui.item("b").unwrap().rect.w, 140.0);
    assert_eq!(ui.item("c").unwrap().rect.x, 250.0);
}
#[test]
fn disabled_skipped_and_tab_wraps() {
    let mut u = ui();
    key(&mut u, "Tab", false, false);
    assert_eq!(u.focus.as_deref(), Some("one"));
    key(&mut u, "Tab", false, false);
    assert_eq!(u.focus.as_deref(), Some("text"));
    key(&mut u, "Tab", true, false);
    assert_eq!(u.focus.as_deref(), Some("one"));
    key(&mut u, "Tab", true, false);
    assert_eq!(u.focus.as_deref(), Some("switch"));
}
#[test]
fn release_outside_does_not_activate() {
    let mut u = ui();
    u.event(Input::Move(30.0, 30.0));
    u.event(Input::Down);
    u.event(Input::Move(900.0, 900.0));
    assert!(u.event(Input::Up).is_empty());
}
#[test]
fn mouse_and_agent_same_actions() {
    let mut u = ui();
    u.event(Input::Move(30.0, 30.0));
    u.event(Input::Down);
    let mouse = u.event(Input::Up);
    let agent = u.agent_action("click", "one", None, None).unwrap();
    assert_eq!(mouse, agent);
}
#[test]
fn disabled_and_wrong_role_rejected() {
    let mut u = ui();
    assert!(u.agent_action("click", "off", None, None).is_err());
    assert!(u.agent_action("click", "text", None, None).is_err());
    assert!(u.agent_action("click", "missing", None, None).is_err());
    assert!(u
        .agent_action("set_value", "slider", None, Some(f32::NAN))
        .is_err());
    assert!(u
        .agent_action("set_value", "slider", None, Some(101.0))
        .is_err());
}
#[test]
fn duplicate_ids_fail_before_layout() {
    assert!(Ui::new(Node::column(
        "r",
        vec![Node::button("a", "A"), Node::button("a", "B")]
    ))
    .is_err());
}
#[test]
fn invalid_values_rejected() {
    assert!(Ui::new(Node::slider("s", "Bad", 0.0, 0.0, 0.0, 1.0)).is_err());
    assert!(Ui::new(Node::button("b", "Bad").width(f32::NAN)).is_err());
    assert!(Ui::new(Node::text_input("t", "Text", "a\nb", "")).is_err());
}
#[test]
fn inherited_disabled_blocks_input() {
    let mut u = Ui::new(fixture("hi").disabled()).unwrap();
    u.resize(400, 400, 1.0);
    assert!(u.agent_action("click", "one", None, None).is_err());
    key(&mut u, "Tab", false, false);
    assert!(u.focus.is_none());
}
#[test]
fn unicode_grapheme_backspace() {
    let mut u = ui();
    u.set_root(fixture("a👩‍💻e\u{301}")).unwrap();
    key(&mut u, "Tab", false, false);
    key(&mut u, "Tab", false, false);
    let a = key(&mut u, "Backspace", false, false);
    assert_eq!(a[0].kind, ActionKind::ChangeText("a👩‍💻".into()));
    u.set_root(fixture("a👩‍💻")).unwrap();
    let a = key(&mut u, "Backspace", false, false);
    assert_eq!(a[0].kind, ActionKind::ChangeText("a".into()));
}
#[test]
fn selection_replace_and_caret_survives_rebuild() {
    let mut u = ui();
    let a = u
        .agent_action("set_text", "text", Some("abc"), None)
        .unwrap();
    assert_eq!(a[0].kind, ActionKind::ChangeText("abc".into()));
    u.set_root(fixture("abc")).unwrap();
    key(&mut u, "ArrowLeft", true, false);
    let a = u.event(Input::Text("Z".into()));
    assert_eq!(a[0].kind, ActionKind::ChangeText("abZ".into()));
}
#[test]
fn external_text_change_clamps_to_grapheme() {
    let mut u = ui();
    u.agent_action("set_text", "text", Some("hello"), None)
        .unwrap();
    u.set_root(fixture("👩‍💻")).unwrap();
    u.paint();
    key(&mut u, "Backspace", false, false);
}
#[test]
fn slider_clamps_and_quantizes() {
    let mut u = ui();
    let a = u
        .agent_action("set_value", "slider", None, Some(62.0))
        .unwrap();
    assert_eq!(a[0].kind, ActionKind::ChangeNumber(60.0));
    let a = key(&mut u, "End", false, false);
    assert_eq!(a[0].kind, ActionKind::ChangeNumber(100.0));
}
#[test]
fn scroll_clips_hits_and_keyboard_reveals() {
    let root = Node::scroll(
        "scroll",
        (0..10)
            .map(|i| Node::button(format!("b{i}"), "Button"))
            .collect(),
    );
    let mut u = Ui::new(root).unwrap();
    u.resize(200, 100, 1.0);
    assert_eq!(u.item("b9").unwrap().clip.h, 0.0);
    for _ in 0..10 {
        key(&mut u, "Tab", false, false);
    }
    assert_eq!(u.focus.as_deref(), Some("b9"));
    assert!(u.item("b9").unwrap().clip.h > 0.0);
    assert!(u.inspect()["nodes"][0]["scroll_offset"].as_f64().unwrap() > 0.0);
}
#[test]
fn focus_removed_when_widget_disappears() {
    let mut u = ui();
    key(&mut u, "Tab", false, false);
    u.set_root(Node::label("empty", "Empty")).unwrap();
    assert!(u.focus.is_none());
}
#[test]
fn dpi_uses_logical_bounds() {
    let mut u = ui();
    u.resize(800, 800, 2.0);
    assert_eq!(u.item("one").unwrap().rect.w, 380.0);
    u.paint();
    assert_eq!(u.painter.pixels.len(), 640000);
}
#[test]
fn renderer_opaque_and_clipped() {
    let mut p = Painter::new();
    p.resize(20, 20, 1.0);
    p.clear(Color(0));
    p.set_clip(Rect::new(5.0, 5.0, 5.0, 5.0));
    p.rect(Rect::new(0.0, 0.0, 20.0, 20.0), Color(0xffffff), 0.0);
    assert_eq!(p.pixels[6 * 20 + 6], 0xffffff);
    assert_eq!(p.pixels[4 * 20 + 4], 0);
    assert_eq!(p.pixels[10 * 20 + 10], 0);
}
#[test]
fn inspect_has_stable_ids_roles_and_capabilities() {
    let u = ui();
    let data = u.inspect();
    let nodes = data["nodes"].as_array().unwrap();
    let off = nodes.iter().find(|n| n["id"] == "off").unwrap();
    assert_eq!(off["actions"], serde_json::json!([]));
    let text = nodes.iter().find(|n| n["id"] == "text").unwrap();
    assert_eq!(text["role"], "textbox");
    assert_eq!(text["parent"], "root");
    assert_eq!(text["value"], "hello");
}
#[test]
fn mouse_positions_caret_and_drag_selects() {
    let mut u = ui();
    let r = u.item("text").unwrap().rect;
    let first = u.painter.measure("h", 14.0, false);
    u.event(Input::Move(r.x + 10.0 + first, r.y + 20.0));
    u.event(Input::Down);
    u.event(Input::Up);
    let a = u.event(Input::Text("!".into()));
    assert_eq!(a[0].kind, ActionKind::ChangeText("h!ello".into()));
    u.set_root(fixture("h!ello")).unwrap();
    u.event(Input::Move(r.x + 10.0, r.y + 20.0));
    u.event(Input::Down);
    let end = u.painter.measure("h!", 14.0, false);
    u.event(Input::Move(r.x + 10.0 + end, r.y + 20.0));
    u.event(Input::Up);
    let a = u.event(Input::Text("H".into()));
    assert_eq!(a[0].kind, ActionKind::ChangeText("Hello".into()));
}
#[test]
fn screenshot_never_overwrites() {
    let mut p = Painter::new();
    p.resize(1, 1, 1.0);
    let path = std::env::temp_dir().join(format!("forge-ppm-{}", std::process::id()));
    std::fs::write(&path, b"keep me").unwrap();
    assert!(p.save_ppm(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"keep me");
}
