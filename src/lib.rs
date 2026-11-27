// EH3D-GERBERVIEWER - a light viewer of Gerber layers and drill files.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! Turns Gerber layers and Excellon drill files into shapes in millimetres, draws them as an SVG picture, measures them and checks that the layers line up.

pub mod align;
pub mod browse;
pub mod draw;
pub mod eh3dfile;
pub mod gap;
#[cfg(feature = "gui")]
pub mod gui;
pub mod lang;
pub mod layers;
pub mod measure;
pub mod ruler;
pub mod scene;
pub mod settings;
pub mod shot;
pub mod svg;
pub mod viewport;
pub mod window;

/// Whether a warning of a reader means that the shapes may not be the whole board or may have another size: the file has no end command (it may be cut), declares no units
/// (millimetres are assumed), may be in inches, or has text after its end that was ignored.
pub fn doubtful(diagnostics: &[eh3d_parser_gerber::Diagnostic]) -> bool {
    use eh3d_parser_gerber::codes;
    diagnostics.iter().any(|d| {
        [
            codes::END_MISSING,
            codes::UNITS_MISSING,
            codes::UNITS_SUSPECT,
            codes::TEXT_AFTER_END,
        ]
        .contains(&d.code)
    })
}

/// Whether a note of a scene may make the smallest gap look bigger than it is. The notes that only add copper (clear polarity drawn as dark, a pad or pen drawn as the shape that
/// holds it, a hole of an aperture left filled) cannot; anything else, a new note included, can.
pub fn may_widen_gaps(note: &str) -> bool {
    ![
        "clear polarity is drawn as dark",
        "a polygon aperture is drawn as a circle",
        "the hole of an aperture is not drawn",
        "a track with an obround pen is drawn with the rectangle that holds the pen",
    ]
    .contains(&note)
}

/// Whether a text is an Excellon drill file whatever its name is: its first line that is not blank and not a comment is the header `M48`. A Gerber file never starts like that.
pub fn looks_like_drill(text: &str) -> bool {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with(';'))
        .is_some_and(|l| {
            // the header, alone or followed by a comma, a comment or a blank (`M48;made by`, `M48 METRIC`); `M481` is not it
            let upper = l.to_ascii_uppercase();
            upper.strip_prefix("M48").is_some_and(|rest| {
                rest.is_empty()
                    || rest.starts_with([',', ';'])
                    || rest.starts_with(char::is_whitespace)
            })
        })
}
