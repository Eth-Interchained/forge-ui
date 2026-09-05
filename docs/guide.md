## Start with a real window

Forge UI is a small native Rust framework with its own tree, layout engine, pixel renderer and interaction runtime. It is not a theme around another GUI toolkit. The same control surface has two entrances: a person using mouse and keyboard, and an agent using a semantic protocol.

The workbench is an executable example, not the framework itself. Its custom signal plot demonstrates extension; its real embedded NEDB journal demonstrates ownership of state.

### Run the workbench

```sh
git clone https://github.com/Eth-Interchained/forge-ui.git
cd forge-ui
cargo run --locked --release --example showcase
```

Use a current stable Rust toolchain. macOS needs Xcode command-line tools. Windows needs the MSVC toolchain plus Visual Studio C++ Build Tools and Windows SDK. Linux needs a desktop display and native headers; the README lists packages. The initial live verification was Linux/X11, not macOS or Windows. A successful build on another platform is not proof of native input behavior there.

The first build downloads and compiles dependencies. There is no JavaScript runtime, cloud account, API key or inference bill. The compiled application uses bundled fonts offline.

## Your five-minute walkthrough

| Step | Try this | Expected result |
| --- | --- | --- |
| 01 | Click Add one | Count increases. The custom plot changes. A saved sequence appears. |
| 02 | Edit Project name | Click positions the caret; drag selects. Ctrl/Cmd+A replaces the full text. |
| 03 | Drag Signal amplitude | Plot height tracks the slider. Keyboard arrows work after Tab focus. |
| 04 | Toggle Porcelain mode | The same tree renders with the light token set. |
| 05 | Enable Layout outlines | Every visible node shows its actual rectangle. |
| 06 | Scroll down | Causal action history shows source, sequence and hash. |
| 07 | Close and reopen | Name, counter, gain and theme restore from the same NEDB store. |
| 08 | Restore previous | A past state returns as a new immutable version. No history is removed. |
| 09 | Verify store | The last-audit count updates; any integrity failure is visible. |
| 10 | Click the disabled button | Nothing happens. The agent route rejects it too. |

For an independent test, select a new data directory. Never delete a store just to reset the UI.

```sh
cargo run --locked --release --example showcase -- --data ./second-store
```

The default `.forge-ui-data` is relative to your current working directory. Changing directories changes the default store. A process-level lock prevents two workbenches from opening the same directory concurrently.

## Build an application

Implement `Application::view` to describe the tree. Implement `Application::update` to reduce semantic actions into your own state. Keep view construction free of I/O. Interaction state—focus, cursor, pressed control and scroll offset—is retained by `Ui` using stable IDs.

```rust
use forge_ui::*;

struct Counter(i32);

impl Application for Counter {
    fn view(&self) -> Node {
        Node::column("root", vec![
            Node::label("title", "Hello, Forge.")
                .font_size(32.0).bold(),
            Node::label("count", format!("Count: {}", self.0)),
            Node::button("add", "Add one").width(180.0),
        ]).padding(32.0).gap(16.0)
    }

    fn update(&mut self, action: Action) {
        if action.id == "add" { self.0 += 1; }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(Counter(0), WindowOptions::default())
}
```

The equivalent complete program is `examples/minimal.rs`; it is compiled by the build gate. Use `cargo run --example minimal --no-default-features` to exercise the GUI without NEDB. For a new project, run `cargo add forge-ui@0.1.0` to include the default NEDB adapter, or `cargo add forge-ui@0.1.0 --no-default-features` for the GUI core. Enable the `nedb` feature when you want the journal adapter. The showcase requires it; the layout/rendering engine does not.

### Start from Forge Desk

For a fuller starting point, use the [Forge Desk application](https://github.com/Eth-Interchained/forge-ui/tree/main/examples/forge-desk) in this same repository:

```sh
cd examples/forge-desk
cargo run --locked --release
```

It demonstrates task creation and completion, filters, themes, NEDB persistence and agent control. The nested manifest depends on `forge-ui = "=0.1.0"` from crates.io, never a local path override. Its own README explains how to customize the state reducer, view and persistence adapter. The framework's `--example showcase` command and this app's nested-manifest command are different entry points.

## Compose the surface

Every node has a unique string ID, a kind, style, enabled flag and children. Duplicate or empty IDs are rejected. IDs should express durable purpose—`project_name`, not a label that changes on every keystroke.

| Component | Constructor | Semantic action |
| --- | --- | --- |
| Label | `Node::label(id, text)` | Read-only |
| Button | `Node::button(id, label)` | `Activate` |
| Switch | `Node::toggle(id, label, checked)` | `Toggle(bool)` |
| Slider | `Node::slider(id, label, value, min, max, step)` | `ChangeNumber(f32)` |
| Single-line input | `Node::text_input(id, label, value, placeholder)` | `ChangeText(String)`, `Submit(String)` |
| Column / row | `Node::column` / `Node::row` | Groups children |
| Scroll viewport | `Node::scroll(id, children)` | Vertical wheel scrolling |
| Custom paint | `Node::custom(id, label, interactive, paint)` | Optional `Activate` |

Slider ranges must be finite, min must be below max, and step must be positive. Values are clamped and quantized. Invalid style dimensions are rejected rather than passed to the renderer. Disabling a container disables its descendants.

### Layout is deliberately small

Coordinates are logical pixels. The native adapter converts physical input through the window scale factor; the painter rasterizes at that factor. The default width is `Fill`; default height is `Auto`. Use `.width(px)` or `.height(px)` for fixed dimensions, `.auto_width()` for intrinsic width, and `.fill_height()` for remaining vertical space. `.padding(px)` is uniform inset; `.gap(px)` separates siblings.

Rows divide leftover width equally among Fill children. Columns divide leftover height equally among Fill children. Fixed children are not shrunk; overflow is clipped. Scroll containers keep children at their measured height and expose the overflow vertically. All nodes inherit parent clipping. There are no weighted flex factors, grid tracks, text wrapping or responsive media queries in v0.1.

## Make it yours

Forge's default visual language is a precision instrument: carbon and porcelain surfaces, vermilion active controls, fine borders and mint signal marks. Styling belongs to reusable tokens, not hardcoded window rendering.

`Theme::DARK` and `Theme::LIGHT` define background, panel, elevated surface, border, text, muted text, accent, on-accent and success colors. Override `Application::theme()` or construct your own `Theme`. Per-node `.background`, `.color`, `.font_size`, `.bold`, `.border` and `.radius` override individual properties.

Custom widgets receive a painter, their logical rectangle and the active theme:

```rust
use forge_ui::*;

fn meter(level: f32) -> Node {
    Node::custom("meter", "Load meter", false, move |p, r, t| {
        p.rect(r, t.elevated, 4.0);
        p.rect(
            Rect::new(r.x, r.y, r.w * level.clamp(0.0, 1.0), r.h),
            t.accent,
            4.0,
        );
    }).height(18.0)
}
```

This pattern is compiled in `examples/custom.rs`. The workbench's custom plot also opts into activation, so pointer, Enter/Space and the agent's `click` produce the same semantic action. Custom painting should stay inside the assigned rectangle and avoid file/network I/O. Painter primitives respect the current clip; direct pixel-buffer access is an advanced escape hatch, not a security boundary.

## Give agents a readable interface

Start the workbench with `--agent`. The protocol lives on local stdin/stdout. No network port is opened. This is a capability granted to the process that launches the app—not an authenticated multi-user service.

```sh
cargo run --locked --release --example showcase -- --agent
```

Wait for the ready event before sending requests. NEDB 2.8.6 prints startup diagnostics before that marker; clients must ignore pre-ready diagnostic lines. After ready, responses are JSON lines. `tools/agent_client.py` demonstrates this handshake without dependencies.

```json
{"event":"ready","protocol":"forge-ui/1"}
```

Send one request and await the matching `request_id`. IDs must be strings. The maximum request line is 64 KiB. Unknown fields and unsupported operations fail explicitly. Keep inspecting outcomes: `ok: true` means the command was accepted and rendered, not that your application's reducer necessarily made the domain change you hoped for. The showcase reports `app.storage_error` separately.

```json
{"request_id":"1","op":"inspect"}
{"request_id":"2","op":"click","id":"increment"}
{"request_id":"3","op":"set_text","id":"project_name","text":"Agent-built interface"}
{"request_id":"4","op":"set_value","id":"gain","value":80}
{"request_id":"5","op":"key","key":"Tab","shift":false}
{"request_id":"6","op":"screenshot","path":"proof.ppm"}
{"request_id":"7","op":"quit"}
```

### Read, act, assert

The inspect result contains `tree.nodes`: ID, parent, role, label, value, range, logical bounds, visible bounds, enabled state, focused state, supported actions and scroll offset/limit. Also inspect `tree.scale` and `tree.viewport`. Bounds are not screen coordinates. Offscreen controls remain in the tree; ID actions reveal them before activation.

Use supported actions, not guesses about roles. `click` is for buttons, switches and interactive custom nodes; sliders use `set_value`; textboxes use `set_text`. Disabled controls are rejected. The key operation targets current focus. Supported keys are Tab, Enter, Space, ArrowLeft/Right/Up/Down, Home, End, Backspace, Delete, a and A; `command: true` with a/A selects all in text inputs.

Screenshots are lossless PPM. The caller supplies the path; existing files are never overwritten. Captures contain the actual software framebuffer presented to the native window. They are not generated mockups. An agent can inspect state and capture sensitive fields, so applications must not expose secrets in this surface. Causal source tags indicate the adapter used; they do not cryptographically authenticate a human or agent identity.

The normative request schema is `protocol.schema.json`. `AGENTS.md` and `llms.txt` give coding agents the shortest route into the repository.

## State with receipts

The optional `journal::Journal` wraps the published `nedb-engine = 2.8.6` Rust crate. It does not imitate NEDB with JSON files or implement its own database. The workbench uses the embedded engine—no daemon setup required.

Each commit writes one NEDB version in `forge_state`, ID `app`, containing schema version, resulting app state, action and source. Its `caused_by` references the preceding state version's hash. This keeps the action and state snapshot in one stored object rather than pretending two separate writes are transactional.

The adapter calls `try_flush_all` and `try_flush_manifest`, then reads back the object by hash before reporting a saved receipt. `history` traverses native causal edges; `state_as_of` uses native historical lookup. Restoration commits the selected old state as a new version. Verification uses the engine's real object-integrity check; it is not a promise of third-party attestation or cryptographic user authentication.

On opening a store, Forge acquires a process lock and verifies objects. It never calls a destructive repair or reset routine. The underlying engine owns its normal manifest/index recovery behavior. Corrupted objects cause failure and remain byte-for-byte unchanged in the tested case. Preserve the directory and investigate rather than deleting it.

There is no automatic retention policy. Inputs and actions are persisted locally in plaintext; avoid secrets. Saves are synchronous in this first adapter: a slow filesystem can stall input. The UI's last-verified count is the last explicit audit, not a fresh audit on each frame. A save failure is visible; do not infer rollback of a potentially partial storage operation.

## Understand the engine

```text
Application::view() → validated Node tree
                          ↓
                  owned row/column layout
                          ↓
              clipped logical rectangles + semantic tree
                          ↓
         pointer / keyboard / JSON agent command
                          ↓
                    Ui event routing
                          ↓
              Application::update_from(action, source)
                          ↓
                 application state + optional NEDB commit
                          ↓
               fresh tree → Painter → softbuffer → window
```

- `node.rs`: node kinds, builder API, design tokens, semantic actions.
- `runtime.rs`: validation, measurement, layout, clipping, hit tests, focus, text editing, scroll and inspection.
- `paint.rs`: owned CPU rasterization, glyph cache, logical-to-physical coordinates, PPM export.
- `window.rs`: winit lifecycle/input adapter, softbuffer presentation, opt-in JSON protocol.
- `journal.rs`: optional real NEDB state journal.
- `examples/showcase.rs`: application-specific state, controls, signal plot and history UI.

No retained DOM or browser exists. The declarative tree is rebuilt when inputs are processed; state is retained by ID. Rendering is event-driven with `ControlFlow::Wait`, not a polling or animation loop.

## Verify instead of assuming

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --locked --no-default-features
cargo build --locked --release --examples
cargo doc --locked --no-deps
```

Core tests cover sizing, clipping, disabled inheritance, focus, pointer release outside a control, semantic capability checks, Unicode grapheme edits and pixel opacity. Persistence tests use the real engine, drop/reopen the store, traverse history, read AS OF state, restore without rewriting existing objects, reject a concurrent opener, and detect deliberate object corruption.

`tools/live_test.py` opens a real native window under X11/Xvfb. It injects pointer and keyboard events with XTest, tests the JSON route, resizes, scrolls, captures frames, kills the process with SIGKILL and reopens the exact same store. Passing fixtures alone is not the acceptance gate.

### Known boundaries

This is v0.1, not a production-complete toolkit. No OS accessibility bridge, IME composition, bidi/complex-script shaping, fallback-font system, clipboard, undo/redo, wrapping text, rich text, menus, dialogs, GPU backend, animation scheduler or signed installers. Grapheme-aware editing does not imply full international text rendering. Custom nodes currently expose a generic custom role plus activation, not arbitrary agent-defined action schemas. Linux/X11 has live evidence; other platform claims must wait for actual tests.

## License and ownership

Forge UI 0.1.0 uses BUSL-1.1. Licensor: Interchained LLC. Additional Use Grant: None. Change Date: September 5, 2030. Change License: GPL-3.0-only. BUSL permits non-production use; third-party production use needs a separate grant before the change date. It is source-available, not currently open source.

Fonts retain their DejaVu/Bitstream terms. NEDB and all dependencies retain their own terms. Forge's change license cannot relicense third-party code. See LICENSE, COPYING-GPL-3.0.txt, THIRD_PARTY.md and the dependency inventory.
