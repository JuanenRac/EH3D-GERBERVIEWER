// EH3D-GERBERVIEWER - a light viewer of Gerber layers and drill files.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! Turns Gerber layers and Excellon drill files into shapes in millimetres, draws them as an SVG picture, measures them and checks that the layers line up.

pub mod align;
pub mod measure;
pub mod scene;
pub mod svg;
