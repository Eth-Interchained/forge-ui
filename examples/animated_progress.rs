//! Demonstrates `Application::tick` — showing work that happens on ANOTHER
//! thread, which is impossible on an input-driven loop alone.
//!
//! A worker counts in the background. Without `tick` the window would sit
//! frozen on "0" until the mouse moved; with it the count advances on its own
//! and the loop returns to zero-wakeup idle the moment the work finishes.
//!
//!     cargo run --example animated_progress

use forge_ui::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

struct Work {
    done: Arc<AtomicUsize>,
    finished: Arc<AtomicBool>,
    total: usize,
}

impl Application for Work {
    fn view(&self) -> Node {
        let done = self.done.load(Ordering::Relaxed);
        let total = self.total;
        let finished = self.finished.load(Ordering::Relaxed);
        Node::column(
            "root",
            vec![
                Node::label("title", "Background work")
                    .font_size(22.0)
                    .bold(),
                Node::label(
                    "count",
                    format!(
                        "{done} / {total}{}",
                        if finished { "  — complete" } else { "" }
                    ),
                )
                .font_size(15.0),
                Node::custom("bar", "", false, move |p, r, t| {
                    p.rect(Rect::new(r.x, r.y, r.w, 10.0), t.elevated, 5.0);
                    let frac = done as f32 / total.max(1) as f32;
                    p.rect(
                        Rect::new(r.x, r.y, r.w * frac.min(1.0), 10.0),
                        if finished { t.success } else { t.accent },
                        5.0,
                    );
                })
                .height(20.0),
            ],
        )
        .padding(28.0)
        .gap(14.0)
    }

    fn update(&mut self, _: Action) {}

    /// Keep asking for frames only while the worker is still going. Once it
    /// finishes we return false, the loop paints one last frame (the completed
    /// bar) and goes back to sleeping on input.
    fn tick(&mut self) -> bool {
        !self.finished.load(Ordering::Relaxed)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let done = Arc::new(AtomicUsize::new(0));
    let finished = Arc::new(AtomicBool::new(false));
    let total = 240;
    {
        let done = Arc::clone(&done);
        let finished = Arc::clone(&finished);
        std::thread::spawn(move || {
            for i in 1..=total {
                std::thread::sleep(std::time::Duration::from_millis(25));
                done.store(i, Ordering::Relaxed);
            }
            finished.store(true, Ordering::Relaxed);
        });
    }
    run(
        Work {
            done,
            finished,
            total,
        },
        WindowOptions {
            title: "tick".into(),
            width: 520.0,
            height: 220.0,
            ..Default::default()
        },
    )
}
