# Forge UI — agent entry point

Read README.md for build requirements, then docs/guide.md for the real API. Machine request schema: docs/protocol.schema.json. API reference: cargo doc --no-deps. Do not invent an API from the crate name or use another GUI toolkit as a substitute.

## Build apps

- Implement Application::view() -> Node and update(Action). Use update_from(Action, Source) when auditing origin.
- Unique stable IDs are mandatory for every node, including containers and labels. Never derive IDs from mutable labels, text contents, list index when order can change, or current values.
- The app owns domain state; Ui retains focus/cursor/scroll by ID. Do not perform database or network reads inside view or custom painting.
- Layout is row/column/fill with uniform padding and gap. Fixed children do not shrink. Text is single-line and clipped. Do not assume CSS.
- Enums and bounded numeric values beat free-form DSL fields. Use the inspected widget action list.
- Cargo default feature nedb enables the real journal adapter; default-features=false yields the GUI core. Showcase requires nedb; minimal does not.
- Never reset, delete, truncate or automatically repair user stores. Restore state by appending a new version. Surface uncertain save failures instead of claiming rollback.
- No secrets in Application::inspect, text fields or labels. The local agent interface exposes them; the showcase persists edits in plaintext.

## Operate apps

Launch showcase --agent --data <explicit-directory>. Wait for ready (forge-ui/1); ignore NEDB diagnostics before ready. Then send one JSON line at a time with a string request_id. Read its matching response before the next mutation.

1. inspect -> locate stable ID and supported actions.
2. click / set_text / set_value / key -> use supported typed operation.
3. Assert resulting tree/app state, not just ok:true. Check app.storage_error and receipt.
4. screenshot -> explicit new PPM filename; overwrite is refused.
5. quit -> close. EOF alone leaves the window running.

Protocol is local stdin/stdout, not HTTP, MCP or an LLM tool server. There is no network listener. It has no approval gate; enabling it grants the launcher access to the app's exposed control surface. Human/agent source labels are provenance annotations, not authenticated identity.

## Gates before proposing changes

cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --locked --no-default-features
cargo build --locked --release --examples
cargo doc --locked --no-deps

Linux live test: xvfb-run -s "-screen 0 1400x1000x24" python3 tools/live_test.py target/release/examples/showcase

Keep docs examples in sync with compiled examples. Do not call Linux tests macOS/Windows validation. Never claim an unrun build passed. Preserve BUSL-1.1 -> GPL-3.0-only parameters and third-party notices. Use cargo add forge-ui@0.1.0 for the crates.io library; no cargo-install binary is supplied. Publishing is a deliberate maintainer action, not a side effect of CI.
