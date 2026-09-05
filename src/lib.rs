//! Forge UI: an owned widget/layout/painting engine with a native desktop adapter.
//!
//! Applications describe a tree and reduce semantic actions into their own state.
//! IDs are stable identity: never derive an interactive ID from its current value.
//!
//! ```no_run
//! use forge_ui::*;
//! struct Counter(i32);
//! impl Application for Counter {
//!     fn view(&self) -> Node {
//!         Node::column("root", vec![
//!             Node::label("count", format!("Count: {}", self.0)),
//!             Node::button("add", "Add one"),
//!         ]).padding(24.0).gap(12.0)
//!     }
//!     fn update(&mut self, action: Action) {
//!         if action.id == "add" { self.0 += 1; }
//!     }
//! }
//! run(Counter(0), WindowOptions::default()).unwrap();
//! ```
#![forbid(unsafe_code)]
#[cfg(feature = "nedb")]
pub mod journal;
mod node;
mod paint;
mod runtime;
mod window;
pub use node::*;
pub use paint::*;
pub use runtime::*;
pub use window::*;
