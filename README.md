# Forge UI

**Native Rust interfaces for humans and machines.**

An independently implemented widget tree, layout engine, software rasterizer and interaction runtime. No egui, iced, Slint, GTK, browser or webview. Low-level platform work is delegated to winit, softbuffer and fontdue. The native workbench dogfoods the real **nedb-engine 2.8.6** for durable application state and causal history.

![Native Forge UI workbench](https://raw.githubusercontent.com/Eth-Interchained/forge-ui/main/docs/assets/dark.png)

## Add the library

```sh
cargo add forge-ui@0.1.0
```

Default features include the optional NEDB adapter. For just the GUI core:

```sh
cargo add forge-ui@0.1.0 --no-default-features
```

This is a library crate, not a `cargo install` command-line package. Run the showcase from the repository below.

## Run it

Install a current stable Rust toolchain and your platform's native build tools, then:

```sh
git clone https://github.com/Eth-Interchained/forge-ui.git
cd forge-ui
cargo run --locked --release --example showcase
```

First compilation downloads dependencies and can take several minutes. Subsequent launches are fast. This is a native desktop program: run it in a desktop session, not an SSH-only shell.

- **macOS:** `xcode-select --install` if Apple's command-line tools are missing. Intel and Apple Silicon build from the same source; neither was interactively tested in the initial Linux verification.
- **Windows:** use stable Rust's MSVC toolchain and Visual Studio Build Tools with Desktop development with C++ plus Windows SDK. Not MinGW. Windows native execution is not yet verified.
- **Ubuntu/Debian:** `sudo apt install build-essential pkg-config libx11-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libxcursor-dev libxrandr-dev libxi-dev libwayland-dev libegl1-mesa-dev`.
- **Linux runtime:** requires an X11 or Wayland desktop and its client libraries. The supplied Linux build is not a portable static binary.

## Five-minute test drive

1. Click **Add one**. The count and custom signal plot change; the status reports a saved NEDB sequence.
2. Edit **Project name**. Click to position the caret; drag or Shift+arrow to select. Ctrl/Cmd+A selects all. Scroll to the history panel to see immutable versions.
3. Drag **Signal amplitude**. Tab focuses it; arrow keys adjust it; Home/End jump to limits.
4. Toggle **Porcelain mode** and **Layout outlines**. The entire app changes theme; outlines expose the actual node rectangles.
5. Close the window. Run the command again from the same directory. Name, count, amplitude, theme and outlines return from `.forge-ui-data`.
6. Click **Restore previous**. A historical state is appended as a NEW version; old objects remain intact. Click **Verify store** to run the real engine's integrity check.
7. The disabled button must never activate, including through the agent API.

Use a new directory for a clean test without deleting prior data:

```sh
cargo run --locked --release --example showcase -- --data ./my-second-forge-store
```

## Start from a complete desktop app

[Forge Desk](examples/forge-desk/README.md) lives in `examples/forge-desk/`: a task board with add/complete controls, filters, themes, NEDB history and agent mode. Its own manifest depends on **the published `forge-ui = "=0.1.0"` crate**, not this checkout.

```sh
cd examples/forge-desk
cargo run --locked --release
```

Keep customizing in `src/state.rs`, `src/view.rs`, and `src/app.rs`. The nested README includes a test drive and agent commands. This application is a repository example, not a separate crate release.

## Build your own app

```sh
cargo run --locked --example minimal --no-default-features
cargo doc --locked --no-deps --open
```

In your application's manifest:

```toml
[dependencies]
forge-ui = "0.1.0"
```

For the core without storage, use `forge-ui = { version = "0.1.0", default-features = false }`. A path dependency remains useful when developing the framework locally.

See [the full guide](docs/guide.md), [agent instructions](AGENTS.md), [protocol schema](docs/protocol.schema.json) and [live test](tools/live_test.py). The styled [docs site](https://eth-interchained.github.io/forge-ui/) is live. The repository and source ZIP include its offline copy at `docs/index.html`; the lean crates.io package includes Markdown and schema instead of generated site assets.

## Agents can read and operate it

```sh
cargo run --locked --release --example showcase -- --agent
```

Wait for `{"event":"ready","protocol":"forge-ui/1"}`. NEDB emits startup diagnostics before this marker. After ready, stdin/stdout use JSON lines; send one request, await its matching response, then send the next.

```json
{"request_id":"1","op":"inspect"}
{"request_id":"2","op":"click","id":"increment"}
{"request_id":"3","op":"set_text","id":"project_name","text":"Built by my agent"}
{"request_id":"4","op":"set_value","id":"gain","value":80}
{"request_id":"5","op":"screenshot","path":"forge-proof.ppm"}
{"request_id":"6","op":"quit"}
```

Each response contains the current semantic tree and app state, including the NEDB receipt. Stable IDs, roles, supported actions, bounds, values, enabled state and focus are exposed. There is no network listener, inference service, API key or coordinate guessing. Programmatic access is opt-in. The attached agent has full access to the app's exposed state; do not expose secrets in labels, fields or `Application::inspect`.

## Verification

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --locked --no-default-features
cargo build --locked --release --examples
```

The Linux integration test uses **XTest to inject real pointer and keyboard events into the native window**, then inspects resulting state. It also kills the process with SIGKILL and checks acknowledged state/hash recovery from the SAME NEDB directory.

```sh
# Ubuntu/Debian test-only dependencies
sudo apt install xvfb libxtst6
xvfb-run -s "-screen 0 1400x1000x24" python3 tools/live_test.py target/release/examples/showcase
```

The Python test retains its temporary output directory, screenshots and store for inspection. It requires no Python packages for assertions; Pillow optionally converts PPM frames to PNG.

## Scope, honestly

This is a testable **v0.1 foundation**, not a mature toolkit replacement. Layout is simple row/column/fill, not CSS flexbox. Text is single-line and clipped, not wrapped. Editing is grapheme-aware; full complex-script shaping, bidi, font fallback and IME composition are not implemented. No clipboard, undo stack, system accessibility bridge, menus, dialogs, rich text, animation scheduler, GPU renderer or signed installers yet. Semantic inspection is not an OS accessibility implementation. NEDB saves are synchronous and can stall input on slow disks; the showcase is for modest state, not a high-frequency telemetry stream.

## License

Forge UI 0.1.0 is **source-available under BUSL-1.1**, not presently open source. Licensor: Interchained LLC. Additional Use Grant: None. Change Date: **2030-09-05**. Change License: **GPL-3.0-only**. See [LICENSE](LICENSE) and [COPYING-GPL-3.0.txt](COPYING-GPL-3.0.txt). Non-production use is allowed under BUSL; third-party production use requires a separate grant before the change date. Bundled fonts and dependencies keep their own licenses; see [THIRD_PARTY.md](THIRD_PARTY.md). This project's change license does not relicense NEDB or other dependencies.
