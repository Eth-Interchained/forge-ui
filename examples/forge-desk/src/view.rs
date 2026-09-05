use crate::{app::Desk, state::Filter};
use forge_ui::*;

fn small(id: impl Into<String>, text: impl Into<String>, color: Color) -> Node {
    Node::label(id, text).font_size(11.0).color(color)
}
fn panel(id: &str, children: Vec<Node>, theme: Theme) -> Node {
    Node::column(id, children)
        .padding(20.0)
        .gap(12.0)
        .background(theme.panel)
        .border()
        .radius(0.0)
}
pub fn build(d: &Desk) -> Node {
    let t = d.theme();
    let done = d.state.tasks.iter().filter(|task| task.done).count();
    let open = d.state.tasks.len() - done;
    let header = Node::row(
        "header",
        vec![
            Node::column(
                "brand",
                vec![
                    Node::label("brand.title", "FORGE / DESK")
                        .font_size(30.0)
                        .bold(),
                    small(
                        "brand.sub",
                        "A SMALL APP. YOUR WHOLE STARTING POINT.",
                        t.muted,
                    ),
                ],
            )
            .gap(4.0),
            Node::column(
                "build",
                vec![
                    small("build.version", "STARTER / 0.1.0", t.accent).bold(),
                    small("build.dep", "crates.io → forge-ui 0.1.0", t.muted),
                ],
            )
            .width(225.0)
            .gap(6.0),
        ],
    )
    .height(76.0);
    let add = Node::button("add", "+ Add task")
        .width(130.0)
        .background(t.accent)
        .color(t.on_accent)
        .bold();
    let composer = panel(
        "composer",
        vec![
            small("composer.kicker", "01 / WHAT'S NEXT?", t.accent).bold(),
            Node::row(
                "composer.controls",
                vec![
                    Node::text_input(
                        "draft",
                        "New task",
                        &d.state.draft,
                        "Write a task, then press Enter",
                    ),
                    if d.state.can_add() {
                        add
                    } else {
                        add.disabled()
                    },
                ],
            )
            .gap(12.0),
            small(
                "composer.help",
                "Enter adds · click a task switch to finish · no tasks are deleted",
                t.muted,
            ),
        ],
        t,
    );
    let filters = Node::row(
        "filters",
        [
            ("filter.all", "All", Filter::All),
            ("filter.open", "Open", Filter::Open),
            ("filter.done", "Done", Filter::Done),
        ]
        .into_iter()
        .map(|(id, label, filter)| {
            let b = Node::button(id, label);
            if d.state.filter == filter {
                b.background(t.elevated).color(t.accent).bold().border()
            } else {
                b.background(t.background).color(t.muted)
            }
        })
        .collect(),
    )
    .gap(8.0)
    .height(40.0);
    let visible: Vec<_> = d
        .state
        .tasks
        .iter()
        .filter(|task| d.state.filter.includes(task))
        .collect();
    let mut rows = vec![];
    if visible.is_empty() {
        rows.push(
            panel(
                "empty",
                vec![
                    Node::label(
                        "empty.title",
                        if d.state.tasks.is_empty() {
                            "A clear desk."
                        } else {
                            "Nothing in this view."
                        },
                    )
                    .font_size(24.0)
                    .bold(),
                    small(
                        "empty.body",
                        if d.state.tasks.is_empty() {
                            "Add your first task above. Everything stays on this machine."
                        } else {
                            "Switch filters to see your other tasks."
                        },
                        t.muted,
                    ),
                ],
                t,
            )
            .height(125.0),
        );
    }
    for task in visible {
        rows.push(
            Node::row(
                format!("task.row.{}", task.id),
                vec![
                    small(
                        format!("task.number.{}", task.id),
                        format!("{:03}", task.id),
                        if task.done { t.success } else { t.accent },
                    )
                    .width(42.0)
                    .height(40.0),
                    Node::column(
                        format!("task.copy.{}", task.id),
                        vec![
                            Node::label(format!("task.title.{}", task.id), &task.title)
                                .bold()
                                .color(if task.done { t.muted } else { t.text }),
                            small(
                                format!("task.state.{}", task.id),
                                if task.done {
                                    "COMPLETED"
                                } else {
                                    "READY TO BUILD"
                                },
                                if task.done { t.success } else { t.muted },
                            ),
                        ],
                    )
                    .gap(3.0),
                    Node::toggle(format!("task.{}", task.id), "Done", task.done).width(105.0),
                ],
            )
            .padding(16.0)
            .gap(12.0)
            .height(80.0)
            .background(t.panel)
            .border()
            .radius(0.0),
        );
    }
    let left = Node::column(
        "board",
        vec![
            composer,
            filters,
            Node::scroll("task.list", rows).gap(10.0).fill_height(),
        ],
    )
    .gap(16.0)
    .fill_height();
    let ratio = if d.state.tasks.is_empty() {
        0.0
    } else {
        done as f32 / d.state.tasks.len() as f32
    };
    let mut history = vec![small("history.kicker", "LATEST RECEIPTS", t.accent).bold()];
    for (idx, event) in d.history.iter().take(4).enumerate() {
        history.push(
            Node::column(
                format!("history.{idx}"),
                vec![
                    small(
                        format!("history.action.{idx}"),
                        format!(
                            "#{} / {}",
                            event["seq"],
                            event["data"]["action"]["id"].as_str().unwrap_or("")
                        ),
                        t.text,
                    ),
                    small(
                        format!("history.hash.{idx}"),
                        event["hash"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(20)
                            .collect::<String>(),
                        t.muted,
                    ),
                ],
            )
            .gap(2.0),
        );
    }
    if d.history.is_empty() {
        history.push(small(
            "history.empty",
            "Your first edit starts the chain.",
            t.muted,
        ));
    }
    let right = Node::scroll(
        "sidebar",
        vec![
            panel(
                "summary",
                vec![
                    small("summary.kicker", "02 / MOMENTUM", t.accent).bold(),
                    Node::label("summary.open", format!("{open:02} open"))
                        .font_size(32.0)
                        .bold(),
                    small(
                        "summary.done",
                        format!("{done} completed / {} total", d.state.tasks.len()),
                        t.muted,
                    ),
                    Node::custom("progress", "Completion progress", false, move |p, r, t| {
                        p.rect(r, t.elevated, 0.0);
                        p.rect(Rect::new(r.x, r.y, r.w * ratio, r.h), t.success, 0.0);
                    })
                    .height(5.0),
                    Node::toggle("theme", "Light mode", d.state.light),
                ],
                t,
            ),
            panel(
                "storage",
                vec![
                    small("storage.kicker", "03 / OWN YOUR STATE", t.accent).bold(),
                    Node::label("storage.name", "NEDB, embedded.")
                        .font_size(17.0)
                        .bold(),
                    small(
                        "storage.audit",
                        format!("Last audit: {} objects", d.verified),
                        t.muted,
                    ),
                    Node::button("verify", "Verify local store").border(),
                    small(
                        "storage.notice",
                        "Plaintext local state. No cloud.",
                        t.muted,
                    ),
                ],
                t,
            ),
            panel("history", history, t),
        ],
    )
    .width(275.0)
    .gap(16.0)
    .fill_height();
    let message = d.error.as_ref().unwrap_or(&d.status);
    let footer = Node::column(
        "footer",
        vec![
            small(
                "status",
                message,
                if d.error.is_some() {
                    t.accent
                } else {
                    t.success
                },
            ),
            small(
                "data.path",
                format!("STORE / {}", d.data_path.display()),
                t.muted,
            ),
        ],
    )
    .gap(2.0)
    .height(40.0);
    let root = Node::column(
        "root",
        vec![
            header,
            Node::row("workspace", vec![left, right])
                .gap(20.0)
                .fill_height(),
            footer,
        ],
    )
    .padding(28.0)
    .gap(18.0);
    if d.storage_blocked {
        root.disabled()
    } else {
        root
    }
}
