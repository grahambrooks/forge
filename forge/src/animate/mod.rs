//! Forge animation — transforms a static SVG into an animated frame-based SVG.
//!
//! Takes a rendered SVG and an Animation definition, then wraps elements
//! in frame groups with CSS transitions for step-by-step reveal.

mod derive;
mod style;
mod transform;
mod util;

#[cfg(test)]
mod tests;

pub use style::playback_script;
pub use transform::animate_svg;
