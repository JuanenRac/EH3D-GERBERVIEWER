// EH3D-GERBERVIEWER - a light viewer of Gerber layers and drill files.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! Turns Gerber layers and Excellon drill files into shapes in millimetres, draws them as an SVG picture, measures them and checks that the layers line up.

pub mod align;
pub mod browse;
pub mod draw;
#[cfg(feature = "gui")]
pub mod gui;
pub mod lang;
pub mod measure;
pub mod scene;
pub mod svg;
pub mod viewport;
pub mod window;

/// Whether a text is an Excellon drill file whatever its name is: its first line that is not blank and not a comment is the header `M48`. A Gerber file never starts like that.
pub fn looks_like_drill(text: &str) -> bool {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with(';'))
        .is_some_and(|l| {
            l.eq_ignore_ascii_case("M48") || l.to_ascii_uppercase().starts_with("M48,")
        })
}
