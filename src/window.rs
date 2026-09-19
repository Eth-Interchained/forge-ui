use crate::*;
use serde::Deserialize;
use serde_json::{json, Value};
use softbuffer::{Context, Surface};
use std::{
    error::Error,
    io::{BufRead, Read},
    num::NonZeroU32,
    rc::Rc,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Human,
    Agent,
}
/// Implement this trait to build an app. UI construction is side-effect free.
pub trait Application: 'static {
    fn view(&self) -> Node;
    fn update(&mut self, action: Action);
    fn update_from(&mut self, action: Action, _source: Source) {
        self.update(action);
    }
    fn theme(&self) -> Theme {
        Theme::DARK
    }
    fn debug_bounds(&self) -> bool {
        false
    }
    /// Optional domain state in automation responses. Never put secrets here.
    fn inspect(&self) -> Value {
        Value::Null
    }
    /// Advance time-based state and report whether another frame is wanted.
    ///
    /// The event loop is otherwise driven purely by input: it sleeps until the
    /// user does something, which is the right default and costs nothing when
    /// idle. That leaves no way to show work happening on ANOTHER thread — a
    /// scan, a download, a long computation — because the window simply does
    /// not repaint until the mouse moves. Progress freezes while the work is
    /// fine, which reads as a hang.
    ///
    /// Return `true` to be called again on the next frame, `false` to go back
    /// to sleeping on input. The default is `false`, so an application that
    /// does not animate is completely unaffected and the loop keeps its
    /// zero-wakeup idle behaviour.
    ///
    /// `tick` runs BEFORE the view is rebuilt, so state it mutates is visible
    /// in the frame it produced. Keep it cheap: it is called at the frame rate,
    /// not at the rate of the work being reported.
    fn tick(&mut self) -> bool {
        false
    }
}

/// What the loop should do after an application's `tick`.
///
/// Split out as a pure function because the decision is the entire animation
/// policy and `about_to_wait` cannot be exercised without a real event loop and
/// a display. Untested loop policy is how a UI ends up either frozen or
/// spinning at 100% CPU, and both failures are invisible in a headless suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sleep {
    /// Ask the window for a new frame.
    pub redraw: bool,
    /// Wake on a deadline (animating) rather than only on input.
    pub timed: bool,
}

pub(crate) fn decide_sleep(wants_more: bool, was_animating: bool) -> Sleep {
    if wants_more {
        // Animating: repaint and come back on a deadline.
        Sleep {
            redraw: true,
            timed: true,
        }
    } else if was_animating {
        // TRAILING EDGE. The app stopped animating this frame, so its final
        // state has not been painted yet. Draw once more, then go back to
        // sleeping on input. Without this the last frame of a finished
        // animation — the completed progress bar, the final total — is never
        // shown until the user happens to move the mouse.
        Sleep {
            redraw: true,
            timed: false,
        }
    } else {
        // Idle: no wakeups at all.
        Sleep {
            redraw: false,
            timed: false,
        }
    }
}

/// Frame interval used while an application reports that it is animating.
/// 60 Hz is a deliberate ceiling rather than a target: a progress indicator
/// that costs measurable CPU is a bug, and nothing here needs to be smoother
/// than the eye can follow.
const FRAME: std::time::Duration = std::time::Duration::from_millis(16);
#[derive(Clone, Debug)]
pub struct WindowOptions {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub agent: bool,
}
impl Default for WindowOptions {
    fn default() -> Self {
        Self {
            title: "Forge UI".into(),
            width: 1120.0,
            height: 820.0,
            agent: false,
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    #[serde(default)]
    request_id: Option<String>,
    op: String,
    #[serde(default)]
    id: String,
    text: Option<String>,
    value: Option<f32>,
    key: Option<String>,
    #[serde(default)]
    shift: bool,
    #[serde(default)]
    command: bool,
    path: Option<String>,
}
enum UserEvent {
    Agent(Result<Request, String>),
}
struct Host<A: Application> {
    app: A,
    ui: Ui,
    options: WindowOptions,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    mods: ModifiersState,
    error: Option<String>,
    /// True while the application's last `tick` asked for another frame.
    animating: bool,
}
impl<A: Application> Host<A> {
    fn rebuild(&mut self) -> Result<(), String> {
        self.ui.theme = self.app.theme();
        self.ui.debug_bounds = self.app.debug_bounds();
        self.ui.set_root(self.app.view())
    }
    fn apply(&mut self, actions: Vec<Action>, source: Source) -> Result<(), String> {
        for a in actions {
            self.app.update_from(a, source);
        }
        self.rebuild()?;
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        Ok(())
    }
    fn input(&mut self, input: Input) -> Result<(), String> {
        let a = self.ui.event(input);
        self.apply(a, Source::Human)
    }
    fn fail(&mut self, e: impl ToString, ev: &ActiveEventLoop) {
        self.error = Some(e.to_string());
        ev.exit();
    }
    fn draw(&mut self) -> Result<(), String> {
        let Some(w) = &self.window else { return Ok(()) };
        let size = w.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        self.ui
            .resize(size.width, size.height, w.scale_factor() as f32);
        self.ui.paint();
        let surface = self.surface.as_mut().ok_or("surface not initialized")?;
        surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .map_err(|e| e.to_string())?;
        let mut buffer = surface.buffer_mut().map_err(|e| e.to_string())?;
        buffer.copy_from_slice(&self.ui.painter.pixels);
        buffer.present().map_err(|e| e.to_string())
    }
    fn agent(&mut self, r: &Request, ev: &ActiveEventLoop) -> Result<Value, String> {
        match r.op.as_str() {
            "inspect" => {}
            "click" | "set_text" | "set_value" => {
                let actions = self
                    .ui
                    .agent_action(&r.op, &r.id, r.text.as_deref(), r.value)?;
                self.apply(actions, Source::Agent)?;
            }
            "key" => {
                let key = r.key.as_deref().ok_or("key is required")?;
                if ![
                    "Tab",
                    "Enter",
                    "Space",
                    "ArrowLeft",
                    "ArrowRight",
                    "ArrowUp",
                    "ArrowDown",
                    "Home",
                    "End",
                    "Backspace",
                    "Delete",
                    "a",
                    "A",
                ]
                .contains(&key)
                {
                    return Err("unsupported key".into());
                }
                let actions = self.ui.event(Input::Key {
                    key: key.into(),
                    shift: r.shift,
                    command: r.command,
                });
                self.apply(actions, Source::Agent)?;
            }
            "screenshot" => {
                self.draw()?;
                let path = r.path.as_ref().ok_or("path is required")?;
                self.ui
                    .painter
                    .save_ppm(std::path::Path::new(path))
                    .map_err(|e| e.to_string())?;
            }
            "quit" => ev.exit(),
            _ => return Err(format!("unknown operation: {}", r.op)),
        }
        // Complete the real rendering path before acknowledging agent mutations.
        if r.op != "quit" {
            self.draw()?;
        }
        Ok(json!({"tree":self.ui.inspect(),"app":self.app.inspect()}))
    }
}
impl<A: Application> ApplicationHandler<UserEvent> for Host<A> {
    fn resumed(&mut self, ev: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let w = Rc::new(
                ev.create_window(
                    Window::default_attributes()
                        .with_title(&self.options.title)
                        .with_inner_size(LogicalSize::new(self.options.width, self.options.height))
                        .with_min_inner_size(LogicalSize::new(760.0, 580.0)),
                )
                .map_err(|e| e.to_string())?,
            );
            let ctx = Context::new(w.clone()).map_err(|e| e.to_string())?;
            let surface = Surface::new(&ctx, w.clone()).map_err(|e| e.to_string())?;
            self.surface = Some(surface);
            self.window = Some(w);
            self.rebuild()?;
            self.draw()?;
            Ok(())
        })();
        if let Err(e) = result {
            self.fail(e, ev);
        } else if self.options.agent {
            println!("{}", json!({"event":"ready","protocol":"forge-ui/1"}));
        }
    }
    fn suspended(&mut self, _ev: &ActiveEventLoop) {
        self.surface = None;
        self.window = None;
    }
    fn user_event(&mut self, ev: &ActiveEventLoop, event: UserEvent) {
        let UserEvent::Agent(request) = event;
        match request {
            Ok(r) => {
                let result = self.agent(&r, ev);
                let response = match result {
                    Ok(v) => json!({"request_id":r.request_id,"ok":true,"result":v}),
                    Err(e) => json!({"request_id":r.request_id,"ok":false,"error":e}),
                };
                println!("{response}");
            }
            Err(e) => println!("{}", json!({"ok":false,"error":e})),
        }
    }
    fn window_event(&mut self, ev: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let result = match event {
            WindowEvent::CloseRequested => {
                ev.exit();
                Ok(())
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
                Ok(())
            }
            WindowEvent::ModifiersChanged(m) => {
                self.mods = m.state();
                Ok(())
            }
            WindowEvent::Focused(false) => self.input(Input::Cancel),
            WindowEvent::CursorLeft { .. } => self.input(Input::Move(-1.0, -1.0)),
            WindowEvent::CursorMoved { position, .. } => {
                let s = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0);
                self.input(Input::Move(
                    (position.x / s) as f32,
                    (position.y / s) as f32,
                ))
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.input(if state == ElementState::Pressed {
                Input::Down
            } else {
                Input::Up
            }),
            WindowEvent::MouseWheel { delta, .. } => {
                let s = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor())
                    .unwrap_or(1.0);
                self.input(Input::Wheel(match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 36.0,
                    MouseScrollDelta::PixelDelta(p) => -p.y as f32 / s as f32,
                }))
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let command = self.mods.control_key() || self.mods.super_key();
                let key = match &event.logical_key {
                    Key::Named(NamedKey::Space) => "Space".into(),
                    Key::Named(k) => format!("{k:?}"),
                    Key::Character(c) => c.to_string(),
                    _ => String::new(),
                };
                let named = matches!(event.logical_key, Key::Named(_));
                let r = self.input(Input::Key {
                    key,
                    shift: self.mods.shift_key(),
                    command,
                });
                if r.is_ok() && !command {
                    if let Some(text) = event.text {
                        if !named || matches!(event.logical_key, Key::Named(NamedKey::Space)) {
                            return if let Err(e) = self.input(Input::Text(text.to_string())) {
                                self.fail(e, ev);
                            };
                        }
                    }
                }
                r
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            self.fail(e, ev);
        }
    }
    /// Decide how to sleep. This is the whole animation mechanism: an app that
    /// wants another frame gets a deadline, an app that does not gets an
    /// indefinite wait and zero wakeups.
    fn about_to_wait(&mut self, ev: &ActiveEventLoop) {
        // An app in a failed state must not be driven further.
        if self.error.is_some() {
            ev.set_control_flow(ControlFlow::Wait);
            return;
        }
        let wants_more = self.app.tick();
        let sleep = decide_sleep(wants_more, self.animating);
        if wants_more {
            // Rebuild so the tick's state changes reach the screen in the very
            // frame they caused.
            if let Err(e) = self.rebuild() {
                self.fail(e, ev);
                return;
            }
        }
        if sleep.redraw {
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        ev.set_control_flow(if sleep.timed {
            ControlFlow::WaitUntil(std::time::Instant::now() + FRAME)
        } else {
            ControlFlow::Wait
        });
        self.animating = wants_more;
    }
}
/// Run a native window. Set `options.agent` to opt into local JSON-lines stdin/stdout.
/// No server is opened. EOF leaves the window open; send `quit` to exit.
pub fn run<A: Application>(app: A, options: WindowOptions) -> Result<(), Box<dyn Error>> {
    let ui = Ui::new(app.view()).map_err(std::io::Error::other)?;
    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    if options.agent {
        let proxy = event_loop.create_proxy();
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut input = stdin.lock();
            loop {
                let mut line = String::new();
                let result = (&mut input).take(65537).read_line(&mut line);
                let request = match result {
                    Ok(0) => break,
                    Ok(_) if line.len() > 65536 => Err("request exceeds 64 KiB".into()),
                    Ok(_) => serde_json::from_str(&line).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                };
                let stop = line.len() > 65536;
                if proxy.send_event(UserEvent::Agent(request)).is_err() || stop {
                    break;
                }
            }
        });
    }
    let mut host = Host {
        app,
        ui,
        options,
        window: None,
        surface: None,
        mods: ModifiersState::empty(),
        error: None,
        animating: false,
    };
    event_loop.run_app(&mut host)?;
    if let Some(e) = host.error {
        return Err(std::io::Error::other(e).into());
    }
    Ok(())
}

#[cfg(test)]
mod tick_tests {
    use super::*;

    /// An application that does not animate must not change the loop's
    /// behaviour at all: no redraws, no timed wakeups, ever.
    #[test]
    fn idle_app_never_wakes_the_loop() {
        let s = decide_sleep(false, false);
        assert!(!s.redraw, "an idle app must not request frames");
        assert!(!s.timed, "an idle app must sleep on input, not a deadline");
    }

    #[test]
    fn animating_app_gets_frames_on_a_deadline() {
        let s = decide_sleep(true, false);
        assert!(s.redraw);
        assert!(s.timed);
        // and it keeps them while it keeps asking
        let s = decide_sleep(true, true);
        assert!(s.redraw);
        assert!(s.timed);
    }

    /// The bug this exists to prevent: an animation's FINAL frame never
    /// reaching the screen. When tick stops returning true, one more redraw is
    /// owed — the completed progress bar, the final total — but the loop must
    /// then stop waking up.
    #[test]
    fn stopping_animation_paints_one_last_frame_then_sleeps() {
        let s = decide_sleep(false, true);
        assert!(
            s.redraw,
            "the last frame of a finished animation must be painted"
        );
        assert!(!s.timed, "and then the loop must stop waking on a deadline");
        // the frame after that is fully idle again
        let s = decide_sleep(false, false);
        assert!(!s.redraw);
        assert!(!s.timed);
    }

    /// The trait default must be false, so adding `tick` cannot change any
    /// existing application's behaviour.
    #[test]
    fn tick_defaults_to_not_animating() {
        struct Plain;
        impl Application for Plain {
            fn view(&self) -> Node {
                Node::label("l", "x")
            }
            fn update(&mut self, _: Action) {}
        }
        let mut p = Plain;
        assert!(!p.tick(), "default tick must not opt an app into animation");
    }
}
