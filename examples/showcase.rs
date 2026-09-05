use forge_ui::{journal::Journal, *};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
struct State {
    count: i32,
    name: String,
    gain: f32,
    light: bool,
    guides: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            count: 0,
            name: "Interchained".into(),
            gain: 64.0,
            light: false,
            guides: false,
        }
    }
}
struct Showcase {
    state: State,
    journal: Journal,
    receipt: Option<journal::Receipt>,
    status: String,
    history: Vec<Value>,
    verified: usize,
    error: bool,
}
impl Showcase {
    fn section(&self, id: &str, kicker: &str, title: &str, children: Vec<Node>) -> Node {
        let t = self.theme();
        let mut nodes = vec![
            Node::label(format!("{id}.kicker"), kicker)
                .font_size(11.0)
                .bold()
                .color(t.accent),
            Node::label(format!("{id}.title"), title)
                .font_size(21.0)
                .bold(),
        ];
        nodes.extend(children);
        Node::column(id, nodes)
            .padding(20.0)
            .gap(12.0)
            .background(t.panel)
            .border()
            .radius(0.0)
    }
}
impl Application for Showcase {
    fn theme(&self) -> Theme {
        if self.state.light {
            Theme::LIGHT
        } else {
            Theme::DARK
        }
    }
    fn debug_bounds(&self) -> bool {
        self.state.guides
    }
    fn view(&self) -> Node {
        let t = self.theme();
        let gain = self.state.gain;
        let count = self.state.count;
        let header = Node::row(
            "header",
            vec![
                Node::column(
                    "brand",
                    vec![
                        Node::label("wordmark", "FORGE / UI").font_size(30.0).bold(),
                        Node::label("tagline", "NATIVE RUST  /  HUMAN + MACHINE")
                            .font_size(11.0)
                            .color(t.muted),
                    ],
                )
                .gap(2.0),
                Node::column(
                    "version",
                    vec![
                        Node::label("version.number", "0.1 / WORKBENCH")
                            .font_size(12.0)
                            .bold()
                            .color(t.accent),
                        Node::label("version.engine", "Software pixels. Real state.")
                            .font_size(12.0)
                            .color(t.muted),
                    ],
                )
                .width(215.0)
                .gap(6.0),
            ],
        )
        .height(72.0)
        .gap(20.0);
        let headline = Node::column(
            "intro",
            vec![
                Node::label("intro.title", "Interfaces you can understand.")
                    .font_size(27.0)
                    .bold(),
                Node::label(
                    "intro.sub",
                    "Touch every control. Inspect every node. Restart without losing your place.",
                )
                .font_size(13.0)
                .color(t.muted),
            ],
        )
        .gap(5.0)
        .height(73.0);
        let controls = self.section(
            "controls",
            "01 / INTERACTION",
            "The control surface",
            vec![
                Node::label("counter.label", format!("{} operations", self.state.count))
                    .font_size(26.0)
                    .bold(),
                Node::row(
                    "counter.actions",
                    vec![
                        Node::button("decrement", "− Subtract").border(),
                        Node::button("increment", "+ Add one")
                            .background(t.accent)
                            .color(t.on_accent)
                            .bold(),
                    ],
                )
                .gap(10.0),
                Node::label("name.label", "PROJECT NAME")
                    .font_size(11.0)
                    .color(t.muted),
                Node::text_input(
                    "project_name",
                    "Project name",
                    &self.state.name,
                    "Name your project",
                ),
                Node::slider("gain", "Signal amplitude", self.state.gain, 0.0, 100.0, 1.0),
                Node::toggle("theme", "Porcelain mode", self.state.light),
                Node::toggle("guides", "Layout outlines", self.state.guides),
                Node::button("disabled", "Disabled — not agent-actionable").disabled(),
                Node::label(
                    "keyboard.help",
                    "Tab to focus · arrows to adjust · Enter to activate",
                )
                .font_size(11.0)
                .color(t.muted),
            ],
        );
        let chart = self.section(
            "signal",
            "02 / EXTENSION",
            "Draw beyond the defaults",
            vec![
                Node::custom(
                    "signal.plot",
                    "Interactive signal plot; activate to add one",
                    true,
                    move |p, r, t| {
                        p.rect(r, t.background, 0.0);
                        for k in 1..5 {
                            p.rect(
                                Rect::new(r.x, r.y + r.h * k as f32 / 5.0, r.w, 1.0),
                                t.border.mix(t.background, 0.5),
                                0.0,
                            );
                        }
                        let n = 36;
                        let gap = 5.0;
                        let bw = (r.w - 32.0 - gap * (n - 1) as f32) / n as f32;
                        for i in 0..n {
                            let phase = i as f32 * 0.26 + count as f32 * 0.3;
                            let h = (0.25 + phase.sin().abs() * 0.75) * (r.h - 32.0) * gain / 100.0;
                            p.rect(
                                Rect::new(
                                    r.x + 16.0 + i as f32 * (bw + gap),
                                    r.y + r.h - 16.0 - h,
                                    bw,
                                    h.max(2.0),
                                ),
                                if i > 24 { t.accent } else { t.success },
                                1.0,
                            );
                        }
                    },
                )
                .height(160.0),
                Node::label(
                    "signal.caption",
                    "Custom widget. Same focus, clipping and agent contract.",
                )
                .font_size(11.0)
                .color(t.muted),
                Node::row(
                    "signal.stats",
                    vec![
                        Node::label("signal.value", format!("{:.0}%  AMPLITUDE", gain))
                            .bold()
                            .color(t.accent),
                        Node::label("signal.owner", format!("BY {}", self.state.name))
                            .font_size(12.0),
                    ],
                )
                .gap(12.0),
            ],
        );
        let latest = &self.receipt;
        let receipt = latest
            .as_ref()
            .map(|r| format!("seq {}  /  {}", r.seq, &r.hash[..16]))
            .unwrap_or("No commits yet — interact to begin".into());
        let store = self.section(
            "store",
            "03 / NEDB ENGINE",
            "A memory, not a reset button",
            vec![
                Node::label("store.receipt", receipt)
                    .font_size(12.0)
                    .color(t.success),
                Node::label(
                    "store.verified",
                    format!("Last audit: {} objects / NEDB 2.8.6", self.verified),
                )
                .font_size(12.0)
                .color(t.muted),
                Node::row(
                    "store.actions",
                    vec![
                        Node::button("verify", "Verify store").border(),
                        if self.history.len() > 1 {
                            Node::button("restore", "Restore previous").border()
                        } else {
                            Node::button("restore", "Restore previous").disabled()
                        },
                    ],
                )
                .gap(10.0),
                Node::label(
                    "store.note",
                    "Restoration appends a version. Your history stays intact.",
                )
                .font_size(11.0)
                .color(t.muted),
            ],
        );
        let right = Node::column("right", vec![chart, store]).gap(16.0);
        let mut ledger = vec![Node::label("history.heading", "04 / CAUSAL ACTION HISTORY")
            .font_size(11.0)
            .bold()
            .color(t.accent)];
        if self.history.is_empty() {
            ledger.push(
                Node::label(
                    "history.empty",
                    "Make an edit to write the first immutable version.",
                )
                .color(t.muted),
            );
        }
        for (idx, row) in self.history.iter().take(12).enumerate() {
            let d = &row["data"];
            ledger.push(
                Node::row(
                    format!("history.row.{idx}"),
                    vec![
                        Node::label(
                            format!("history.seq.{idx}"),
                            format!("#{:04}", row["seq"].as_u64().unwrap_or(0)),
                        )
                        .width(68.0)
                        .color(t.accent),
                        Node::label(
                            format!("history.action.{idx}"),
                            format!(
                                "{}  /  {}",
                                d["action"]["id"].as_str().unwrap_or(""),
                                d["action"]["kind"]["type"].as_str().unwrap_or("")
                            ),
                        ),
                        Node::label(
                            format!("history.source.{idx}"),
                            d["source"].as_str().unwrap_or(""),
                        )
                        .width(72.0)
                        .color(t.muted),
                        Node::label(
                            format!("history.hash.{idx}"),
                            row["hash"]
                                .as_str()
                                .unwrap_or("")
                                .chars()
                                .take(12)
                                .collect::<String>(),
                        )
                        .width(120.0)
                        .color(t.muted),
                    ],
                )
                .height(30.0)
                .gap(10.0),
            );
        }
        let history = Node::column("history", ledger)
            .padding(20.0)
            .gap(8.0)
            .background(t.panel)
            .border()
            .radius(0.0);
        let content = Node::scroll(
            "workspace",
            vec![
                headline,
                Node::row("panels", vec![controls, right]).gap(16.0),
                history,
            ],
        )
        .gap(16.0)
        .fill_height();
        let footer = Node::row(
            "footer",
            vec![
                Node::label("status", &self.status)
                    .font_size(11.0)
                    .color(if self.error { t.accent } else { t.muted }),
                Node::label("footer.license", "BUSL → GPLv3 / 2030")
                    .width(170.0)
                    .font_size(11.0)
                    .color(t.muted),
            ],
        )
        .height(22.0)
        .gap(12.0);
        Node::column("root", vec![header, content, footer])
            .padding(28.0)
            .gap(16.0)
    }
    fn update(&mut self, a: Action) {
        self.update_from(a, Source::Human);
    }
    fn update_from(&mut self, a: Action, source: Source) {
        let mut candidate = self.state.clone();
        match (a.id.as_str(), &a.kind) {
            ("increment" | "signal.plot", ActionKind::Activate) => {
                candidate.count = candidate.count.saturating_add(1)
            }
            ("decrement", ActionKind::Activate) => {
                candidate.count = candidate.count.saturating_sub(1)
            }
            ("project_name", ActionKind::ChangeText(v)) => candidate.name = v.clone(),
            ("gain", ActionKind::ChangeNumber(v)) => candidate.gain = *v,
            ("theme", ActionKind::Toggle(v)) => candidate.light = *v,
            ("guides", ActionKind::Toggle(v)) => candidate.guides = *v,
            ("verify", ActionKind::Activate) => match self.journal.verify() {
                Ok(n) => self.verified = n,
                Err(e) => {
                    self.error = true;
                    self.status = e;
                    return;
                }
            },
            ("restore", ActionKind::Activate) => {
                if let Some(row) = self.history.get(1) {
                    if let Some(v) = self.journal.state_as_of(row["seq"].as_u64().unwrap_or(0)) {
                        match serde_json::from_value(v) {
                            Ok(s) => candidate = s,
                            Err(e) => {
                                self.error = true;
                                self.status = e.to_string();
                                return;
                            }
                        }
                    }
                }
            }
            _ => return,
        }
        match self
            .journal
            .commit(serde_json::to_value(&candidate).unwrap(), &a, source)
        {
            Ok(receipt) => {
                self.state = candidate;
                self.status = format!("SAVED / {} / seq {} / {:?}", a.id, receipt.seq, source);
                self.receipt = Some(receipt);
                self.error = false;
                self.history = self.journal.history(14);
            }
            Err(e) => {
                self.error = true;
                self.status = format!("SAVE FAILED — {e}");
            }
        }
    }
    fn inspect(&self) -> Value {
        json!({"state":self.state,"receipt":self.receipt,"status":self.status,"storage_error":self.error,"verified_objects":self.verified,"history":self.history})
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let agent = args.iter().any(|a| a == "--agent");
    if args.iter().any(|a| a == "--help") {
        println!("Forge UI workbench\n  --agent          local JSON-lines control on stdin/stdout\n  --data PATH      NEDB store (default .forge-ui-data)\n  --help           this help\nLaunch normally for pointer/keyboard interaction. Docs: docs/index.html");
        return Ok(());
    }
    let data = if let Some(i) = args.iter().position(|a| a == "--data") {
        PathBuf::from(args.get(i + 1).ok_or("--data requires a directory")?)
    } else {
        PathBuf::from(".forge-ui-data")
    };
    let journal = Journal::open(&data).map_err(std::io::Error::other)?;
    let state = match journal.load() {
        Some(v) => serde_json::from_value(v)?,
        None => State::default(),
    };
    let verified = journal.verify().map_err(std::io::Error::other)?;
    let history = journal.history(14);
    let status = format!(
        "LOCAL-FIRST / {} / {} verified objects",
        data.display(),
        verified
    );
    run(
        Showcase {
            state,
            receipt: journal.latest(),
            journal,
            status,
            history,
            verified,
            error: false,
        },
        WindowOptions {
            title: "Forge UI — Native Workbench".into(),
            agent,
            ..Default::default()
        },
    )
}
