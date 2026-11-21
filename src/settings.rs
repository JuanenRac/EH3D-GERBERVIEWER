//! The settings that a person tunes, read from a small configuration file (`gerberview.cfg`): `key = value` lines, `#` starts a comment. Every value has a default and a range; a value
//! outside its range, an unknown key or a key written twice is an error that names the line, never something to ignore.

use crate::ruler::Unit;

/// The largest configuration file that is read.
pub const MAX_CFG_BYTES: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// How far from a click a vertex or a pad centre still catches it, in millimetres.
    pub snap_tolerance_mm: f64,
    /// Whether the corners of rectangles, the ends of tracks and the points of regions catch a click.
    pub snap_vertices: bool,
    /// Whether the centres of pads catch a click.
    pub snap_pad_centres: bool,
    /// The unit a measure is written in.
    pub unit: Unit,
    /// A layer below this opacity is reported as almost invisible.
    pub min_visible_opacity: f64,
    /// The opacity a layer has when it is added.
    pub default_opacity: f64,
    /// The most pairs of shapes that the gap between two layers looks at.
    pub max_gap_pairs: usize,
    /// The most layers of a stack.
    pub max_layers: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            snap_tolerance_mm: 0.15,
            snap_vertices: true,
            snap_pad_centres: true,
            unit: Unit::Mm,
            min_visible_opacity: 0.05,
            default_opacity: 0.85,
            max_gap_pairs: 4_000_000,
            max_layers: 64,
        }
    }
}

fn number(value: &str, key: &str, lo: f64, hi: f64) -> Result<f64, String> {
    let n: f64 = value
        .parse()
        .ok()
        .filter(|n: &f64| n.is_finite())
        .ok_or_else(|| format!("{key} '{value}' is not a number"))?;
    if !(lo..=hi).contains(&n) {
        return Err(format!("{key} {n} must be from {lo} to {hi}"));
    }
    Ok(n)
}

fn whole(value: &str, key: &str, lo: usize, hi: usize) -> Result<usize, String> {
    let n: usize = value
        .parse()
        .map_err(|_| format!("{key} '{value}' is not a whole number"))?;
    if !(lo..=hi).contains(&n) {
        return Err(format!("{key} {n} must be from {lo} to {hi}"));
    }
    Ok(n)
}

fn switch(value: &str, key: &str) -> Result<bool, String> {
    match value {
        "yes" | "true" | "on" | "1" => Ok(true),
        "no" | "false" | "off" | "0" => Ok(false),
        _ => Err(format!("{key} '{value}' must be yes or no")),
    }
}

impl Settings {
    /// The settings of a configuration text; what the text does not say stays at its default.
    pub fn parse(text: &str) -> Result<Settings, String> {
        let mut s = Settings::default();
        let mut seen: Vec<&str> = Vec::new();
        for (n, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let at = n + 1;
            let (key, value) = line
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .ok_or_else(|| format!("line {at}: '{line}' is not key = value"))?;
            if seen.contains(&key) {
                return Err(format!("line {at}: {key} is written twice"));
            }
            let wrap = |e: String| format!("line {at}: {e}");
            match key {
                "snap_tolerance_mm" => {
                    s.snap_tolerance_mm = number(value, key, 0.0, 10.0).map_err(wrap)?
                }
                "snap_vertices" => s.snap_vertices = switch(value, key).map_err(wrap)?,
                "snap_pad_centres" => s.snap_pad_centres = switch(value, key).map_err(wrap)?,
                "unit" => {
                    s.unit = Unit::parse(value)
                        .ok_or_else(|| format!("line {at}: unit '{value}' must be mm, in or mil"))?
                }
                "min_visible_opacity" => {
                    s.min_visible_opacity = number(value, key, 0.0, 1.0).map_err(wrap)?
                }
                "default_opacity" => {
                    s.default_opacity = number(value, key, 0.0, 1.0).map_err(wrap)?
                }
                "max_gap_pairs" => {
                    s.max_gap_pairs = whole(value, key, 1, 100_000_000).map_err(wrap)?
                }
                "max_layers" => s.max_layers = whole(value, key, 1, 1024).map_err(wrap)?,
                other => return Err(format!("line {at}: unknown setting {other}")),
            }
            seen.push(key);
        }
        Ok(s)
    }

    /// The settings of a file, read with a size limit.
    pub fn load(path: &std::path::Path) -> Result<Settings, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if bytes.len() > MAX_CFG_BYTES {
            return Err(format!(
                "{}: the configuration file is above {MAX_CFG_BYTES} bytes",
                path.display()
            ));
        }
        Settings::parse(&String::from_utf8_lossy(&bytes))
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}
