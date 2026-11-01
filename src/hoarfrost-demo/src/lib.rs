//! Module graph of the `hoarfrost` showcase, exposed as a library.
//!
//! Deliberately thin at the top: only the module graph lives here; every gram
//! of state, view and data lives in the modules it declares (`RUST-054` — an
//! entry point composes, it does not implement). The binary in `main.rs` is a
//! shell over this library and adds nothing to it, so a host other than the
//! demo window — a gallery, a snapshot tool — can build and render the very
//! same sections the window shows.
//!
//! The modules the window never needs at runtime stay private and
//! `#[cfg(test)]`-gated, exactly as they were while the whole graph lived in
//! the binary: the layout probe the assertions read, the alignment audit, the
//! reserved-id guard, the layout gate and the framebuffer render checks.

#[cfg(test)]
mod alignment_audit;
pub mod app;
pub mod chrome;
pub mod data;
pub mod font_fallback;
#[cfg(test)]
mod glyph_resolution;
#[cfg(test)]
mod id_namespace;
#[cfg(test)]
mod layout_gate;
#[cfg(test)]
mod layout_probe;
pub mod menu_surface;
pub mod pane_atoms;
pub mod pane_composites;
pub mod previews;
#[cfg(test)]
mod render_atoms;
#[cfg(test)]
mod render_composites;
#[cfg(test)]
mod render_section;
#[cfg(test)]
mod render_stepper;
#[cfg(test)]
mod render_surfaces;
pub mod section;
pub mod theme;
