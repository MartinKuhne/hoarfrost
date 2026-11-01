//! Entry point of the `hoarfrost` showcase application.
//!
//! A thin shell over the [`hoarfrost_demo`] library: the module graph, and
//! every gram of state, view and data, live there; only the iced builder
//! chain lives here (`RUST-054` — an entry point composes, it does not
//! implement).
//!
//! Run it with `cargo run` from `src/hoarfrost-demo`.

use hoarfrost_demo::{app, theme};

/// Opens the demo window.
///
/// The theme is the hoarfrost-token-derived iced theme from
/// [`hoarfrost_demo::theme`], so iced's own derived widget styles resolve against
/// the same palette the framework widgets read.
///
/// The symbol fallback font is registered inside
/// [`DemoApp::boot`](hoarfrost_demo::app::DemoApp::boot), not here: `boot` is
/// the single init path the window and the headless tests share, and iced
/// needs no window and no renderer before a face can be added to its
/// process-wide font system.
fn main() -> iced::Result {
    iced::application(app::DemoApp::boot, app::DemoApp::update, app::DemoApp::view)
        .title(app::strings::WINDOW_TITLE)
        .theme(|_app: &app::DemoApp| theme::demo_theme())
        .run()
}
