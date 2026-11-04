//! The state of the window, without any screen: the layers that are open (each with its colour and whether it is shown), the parameters being edited, the results of measuring and
//! checking, what was last said to the person and the zoom of the picture. The screen (`gui`, built with the `gui` feature) only draws this and passes on what the person does,
//! so everything here is tested as plain code.

use crate::align::{compare_boxes, holes_outside};
use crate::draw::Drawing;
use crate::lang::{fill, Lang, Msg};
use crate::looks_like_drill;
use crate::measure::{min_clearance, min_track_width, pad_centres};
use crate::scene::{from_excellon, from_gerber_with, Bounds, Scene, SceneOptions};
use crate::svg::{default_color, render, Layer as SvgLayer};
use crate::viewport::Viewport;
use eh3d_parser_gerber::{read_file_limited, InputError, InputLimits, ReadError, Severity};
use std::fs;
use std::path::{Path, PathBuf};

/// The extensions of the files the window offers to open.
pub const LAYER_EXTENSIONS: [&str; 19] = [
    "gtl", "gbl", "gts", "gbs", "gto", "gbo", "gtp", "gbp", "gko", "gm1", "gbr", "ger", "pho",
    "art", "drl", "ncd", "xln", "exc", "drd",
];

/// A parameter of the window: the sentence that names it and the text being typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub key: &'static str,
    pub label: Msg,
    pub text: String,
}

/// The parameters the window offers, in the order they are shown.
pub fn default_fields() -> Vec<Field> {
    let f = |key: &'static str, label: Msg, text: &str| Field {
        key,
        label,
        text: text.to_string(),
    };
    vec![
        f("opacity", Msg::Opacity, "0.85"),
        f("margin", Msg::Margin, "2"),
        f("tolerance", Msg::Tolerance, "0.5"),
        f("arc_tolerance", Msg::ArcTolerance, "0.005"),
    ]
}

/// The values of the parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Values {
    pub opacity: f64,
    pub margin_mm: f64,
    pub tolerance_mm: f64,
    pub arc_tolerance_mm: f64,
}

/// What the window last said to the person.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Ready,
    Opened {
        name: String,
        shapes: usize,
        width_mm: f64,
        height_mm: f64,
    },
    Exported {
        file: String,
    },
    NoLayer,
    Failed(String),
    BadValue {
        label: Msg,
        why: String,
    },
    SameFile,
    NotSvg,
}

impl Status {
    /// The sentence, in the language of the person.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Status::Ready => Msg::Ready.tr(lang).to_string(),
            Status::Opened {
                name,
                shapes,
                width_mm,
                height_mm,
            } => {
                let (shapes, width, height) = (
                    shapes.to_string(),
                    format!("{width_mm:.2}"),
                    format!("{height_mm:.2}"),
                );
                fill(
                    Msg::Opened.tr(lang),
                    &[
                        name.as_str(),
                        shapes.as_str(),
                        width.as_str(),
                        height.as_str(),
                    ],
                )
            }
            Status::Exported { file } => fill(Msg::Exported.tr(lang), &[file.as_str()]),
            Status::NoLayer => Msg::NoLayer.tr(lang).to_string(),
            Status::Failed(why) => fill(Msg::Failed.tr(lang), &[why.as_str()]),
            Status::BadValue { label, why } => {
                fill(Msg::BadValue.tr(lang), &[label.tr(lang), why.as_str()])
            }
            Status::SameFile => Msg::SameFile.tr(lang).to_string(),
            Status::NotSvg => Msg::NotSvg.tr(lang).to_string(),
        }
    }

    /// Whether the sentence is a problem or a warning (the window shows it in another colour).
    pub fn is_problem(&self) -> bool {
        !matches!(
            self,
            Status::Ready | Status::Opened { .. } | Status::Exported { .. }
        )
    }
}

/// One line of the results of measuring or checking.
#[derive(Debug, Clone, PartialEq)]
pub enum Report {
    Narrowest(f64),
    NoTracks,
    Gap { gap_mm: f64, a: usize, b: usize },
    FewShapes,
    Pads(usize),
    Lined,
    HolesInside,
    NothingChecked,
    Finding { layer: String, message: String },
}

impl Report {
    /// The sentence, in the language of the person.
    pub fn text(&self, lang: Lang) -> String {
        match self {
            Report::Narrowest(w) => {
                fill(Msg::ReportNarrowest.tr(lang), &[format!("{w:.3}").as_str()])
            }
            Report::NoTracks => Msg::ReportNoTracks.tr(lang).to_string(),
            Report::Gap { gap_mm, a, b } => fill(
                Msg::ReportGap.tr(lang),
                &[
                    format!("{gap_mm:.3}").as_str(),
                    a.to_string().as_str(),
                    b.to_string().as_str(),
                ],
            ),
            Report::FewShapes => Msg::ReportFew.tr(lang).to_string(),
            Report::Pads(n) => fill(Msg::ReportPads.tr(lang), &[n.to_string().as_str()]),
            Report::Lined => Msg::ReportLined.tr(lang).to_string(),
            Report::HolesInside => Msg::ReportHoles.tr(lang).to_string(),
            Report::NothingChecked => Msg::ReportNothing.tr(lang).to_string(),
            Report::Finding { layer, message } => fill(
                Msg::ReportFinding.tr(lang),
                &[layer.as_str(), message.as_str()],
            ),
        }
    }

    /// Whether the line reports something wrong (the window shows it in another colour).
    pub fn is_problem(&self) -> bool {
        matches!(self, Report::Finding { .. } | Report::NothingChecked)
    }
}

/// A layer that is open.
#[derive(Debug, Clone)]
pub struct LayerView {
    pub path: PathBuf,
    pub name: String,
    /// A drill file (its holes are drawn and checked against the copper).
    pub drill: bool,
    pub scene: Scene,
    pub drawing: Drawing,
    pub rgb: (u8, u8, u8),
    pub visible: bool,
}

/// The colour `#rrggbb` as three numbers; a grey for anything else.
pub fn parse_hex(text: &str) -> (u8, u8, u8) {
    let t = text.trim();
    let digits = t.strip_prefix('#').unwrap_or("");
    if digits.len() == 6 && digits.bytes().all(|c| c.is_ascii_hexdigit()) {
        let part = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).unwrap_or(0xb0);
        (part(0), part(2), part(4))
    } else {
        (0xb0, 0xb0, 0xb0)
    }
}

/// Three numbers as `#rrggbb`.
pub fn hex(rgb: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

/// Whether a file name says it is a drill file (by its extension).
fn drill_name(name: &str) -> bool {
    name.contains('.')
        && matches!(
            name.rsplit('.')
                .next()
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
            "drl" | "ncd" | "xln" | "exc" | "drd"
        )
}

fn human(n: usize) -> String {
    if n >= 1_048_576 {
        format!("{} MiB", n / 1_048_576)
    } else {
        format!("{n} bytes")
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Reads a layer the way the command line does: with the size limit (refused in the language of the person), a drill file told by its name or its header, and a layer refused when the
/// reader found errors in it (a layer drawn half is not the layer of the board).
pub fn read_layer(path: &Path, lang: Lang, arc_tolerance_mm: f64) -> Result<LayerView, String> {
    let name = file_name(path);
    let bytes = read_file_limited(path, &InputLimits::default()).map_err(|e| match e {
        ReadError::TooLarge(InputError::TooLarge { size, limit }) => fill(
            Msg::TooLarge.tr(lang),
            &[name.as_str(), human(size).as_str(), human(limit).as_str()],
        ),
        other => format!("{name}: {other}"),
    })?;
    let text = String::from_utf8_lossy(&bytes);
    let (scene, drill, errors) = if drill_name(&name) || looks_like_drill(&text) {
        let file = eh3d_parser_excellon::parse(&text);
        let errors = file
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        (from_excellon(&file), true, errors)
    } else {
        let file = eh3d_parser_gerber::parse(&text);
        let errors = file
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        (
            from_gerber_with(&file, &SceneOptions { arc_tolerance_mm }),
            false,
            errors,
        )
    };
    if errors > 0 {
        return Err(fill(
            Msg::Incomplete.tr(lang),
            &[name.as_str(), errors.to_string().as_str()],
        ));
    }
    if scene.shapes.is_empty() {
        return Err(fill(Msg::NothingToDraw.tr(lang), &[name.as_str()]));
    }
    let drawing = Drawing::new(&scene.shapes);
    let rgb = parse_hex(default_color(&name));
    Ok(LayerView {
        path: path.to_path_buf(),
        name,
        drill,
        scene,
        drawing,
        rgb,
        visible: true,
    })
}

/// The window.
pub struct Window {
    pub lang: Lang,
    pub fields: Vec<Field>,
    pub layers: Vec<LayerView>,
    /// The layer that measuring works on.
    pub selected: Option<usize>,
    pub report: Vec<Report>,
    pub status: Status,
    pub viewport: Viewport,
}

impl Window {
    pub fn new(lang: Lang) -> Window {
        Window {
            lang,
            fields: default_fields(),
            layers: Vec::new(),
            selected: None,
            report: Vec::new(),
            status: Status::Ready,
            viewport: Viewport::default(),
        }
    }

    /// The values typed in the parameters. A value that is not valid says which parameter it is.
    pub fn values(&self) -> Result<Values, Status> {
        let mut v = Values {
            opacity: 0.85,
            margin_mm: 2.0,
            tolerance_mm: 0.5,
            arc_tolerance_mm: 0.005,
        };
        for f in &self.fields {
            let bad = |why: String| Status::BadValue {
                label: f.label,
                why,
            };
            let n = f
                .text
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .ok_or_else(|| bad(format!("'{}' is not a number", f.text.trim())))?;
            let (lo, hi) = match f.key {
                "opacity" => (0.0, 1.0),
                "margin" | "tolerance" => (0.0, 1.0e6),
                _ => (0.0001, 1.0),
            };
            if !(lo..=hi).contains(&n) {
                return Err(bad(format!("{n} must be from {lo} to {hi}")));
            }
            match f.key {
                "opacity" => v.opacity = n,
                "margin" => v.margin_mm = n,
                "tolerance" => v.tolerance_mm = n,
                _ => v.arc_tolerance_mm = n,
            }
        }
        Ok(v)
    }

    /// The opacity to draw with: the one typed, or the usual one while the text is not a valid number.
    pub fn opacity(&self) -> f64 {
        self.values().map_or(0.85, |v| v.opacity)
    }

    /// Changes the text of a parameter.
    pub fn set_field(&mut self, index: usize, text: String) {
        if let Some(f) = self.fields.get_mut(index) {
            f.text = text;
        }
    }

    /// The box of the picture: every layer that is shown.
    fn extent(&self) -> Option<((f64, f64), (f64, f64))> {
        let mut lo = (f64::INFINITY, f64::INFINITY);
        let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for l in self.layers.iter().filter(|l| l.visible) {
            if let Some((a, b)) = l.drawing.extent() {
                lo = (lo.0.min(a.0), lo.1.min(a.1));
                hi = (hi.0.max(b.0), hi.1.max(b.1));
            }
        }
        (lo.0 <= hi.0 && lo.1 <= hi.1).then_some((lo, hi))
    }

    /// Shows every layer that is shown in a picture of this size.
    pub fn fit(&mut self, width_px: f64, height_px: f64) {
        if let Some((lo, hi)) = self.extent() {
            self.viewport = Viewport::fit(lo, hi, width_px, height_px, 24.0);
        }
    }

    /// Adds a layer or a drill file. The layers that are open stay as they are when it cannot be read.
    pub fn add_layer(&mut self, path: &Path) -> bool {
        let arc = match self.values() {
            Ok(v) => v.arc_tolerance_mm,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        match read_layer(path, self.lang, arc) {
            Ok(layer) => {
                let b = layer.scene.bounds();
                self.status = Status::Opened {
                    name: layer.name.clone(),
                    shapes: layer.scene.shapes.len(),
                    width_mm: b.width(),
                    height_mm: b.height(),
                };
                self.layers.push(layer);
                self.selected = Some(self.layers.len() - 1);
                self.report.clear();
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Takes a layer out.
    pub fn remove_layer(&mut self, index: usize) {
        if index < self.layers.len() {
            self.layers.remove(index);
            self.report.clear();
            let last = self.layers.len().saturating_sub(1);
            self.selected = if self.layers.is_empty() {
                None
            } else {
                self.selected
                    .map(|s| if s > index { s - 1 } else { s.min(last) })
            };
        }
    }

    /// The narrowest track, the smallest gap and the number of pads of the selected layer.
    pub fn measure(&mut self) -> bool {
        let Some(layer) = self.selected.and_then(|s| self.layers.get(s)) else {
            self.status = Status::NoLayer;
            return false;
        };
        let scene = &layer.scene;
        let mut out = vec![match min_track_width(scene) {
            Some(w) => Report::Narrowest(w),
            None => Report::NoTracks,
        }];
        match min_clearance(scene) {
            Ok(Some((gap_mm, a, b))) => out.push(Report::Gap { gap_mm, a, b }),
            Ok(None) => out.push(Report::FewShapes),
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        }
        out.push(Report::Pads(pad_centres(scene).len()));
        self.report = out;
        true
    }

    /// Do the layers line up, and are the holes inside the copper? (as `check` of the command line)
    pub fn check(&mut self) -> bool {
        if self.layers.is_empty() {
            self.status = Status::NoLayer;
            return false;
        }
        let tolerance = match self.values() {
            Ok(v) => v.tolerance_mm,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        let boxes: Vec<(String, Bounds)> = self
            .layers
            .iter()
            .filter(|l| !l.drill)
            .map(|l| (l.name.clone(), l.scene.bounds()))
            .collect();
        let mut findings = compare_boxes(&boxes, tolerance);
        let mut holes_checked = false;
        if let Some(copper) = self.layers.iter().find(|l| !l.drill) {
            for holes in self.layers.iter().filter(|l| l.drill) {
                holes_checked = true;
                findings.extend(holes_outside(&copper.scene, &holes.scene, &holes.name));
            }
        }
        let mut out = Vec::new();
        if findings.is_empty() {
            if boxes.len() >= 2 {
                out.push(Report::Lined);
            }
            if holes_checked {
                out.push(Report::HolesInside);
            }
            if out.is_empty() {
                out.push(Report::NothingChecked);
            }
        } else {
            out.extend(findings.into_iter().map(|f| Report::Finding {
                layer: f.layer,
                message: f.message,
            }));
        }
        self.report = out;
        true
    }

    /// The name suggested for the picture: `layers.svg` in the folder of the first layer.
    pub fn suggested_output(&self) -> Option<PathBuf> {
        self.layers
            .first()
            .map(|l| l.path.with_file_name("layers.svg"))
    }

    /// Writes the layers that are shown as one SVG picture, with the opacity and the margin of the parameters.
    pub fn export(&mut self, out: &Path) -> bool {
        let values = match self.values() {
            Ok(v) => v,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        if !self.layers.iter().any(|l| l.visible) {
            self.status = Status::NoLayer;
            return false;
        }
        if !out
            .extension()
            .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("svg"))
        {
            self.status = Status::NotSvg;
            return false;
        }
        let same =
            self.layers.iter().any(
                |l| match (fs::canonicalize(&l.path), fs::canonicalize(out)) {
                    (Ok(a), Ok(b)) => a == b,
                    _ => l.path == out,
                },
            );
        if same {
            self.status = Status::SameFile;
            return false;
        }
        let pictures: Vec<SvgLayer> = self
            .layers
            .iter()
            .filter(|l| l.visible)
            .map(|l| SvgLayer {
                name: l.name.clone(),
                scene: l.scene.clone(),
                color: hex(l.rgb),
                opacity: values.opacity,
            })
            .collect();
        match fs::write(out, render(&pictures, values.margin_mm)) {
            Ok(()) => {
                self.status = Status::Exported {
                    file: file_name(out),
                };
                true
            }
            Err(e) => {
                self.status = Status::Failed(format!("{}: {e}", out.display()));
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COPPER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\n%ADD11R,2.0X1.0*%\nD10*\nX00000Y00000D02*\nX100000Y00000D01*\nX100000Y100000D01*\nD11*\nX50000Y50000D03*\nM02*\n";
    const SHIFTED: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX300000Y300000D02*\nX400000Y300000D01*\nM02*\n";
    const DRILL: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX009.000Y009.000\nX005.000Y005.000\nM30\n";
    const FAR_DRILL: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX090.000Y090.000\nM30\n";

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "eh3d-gerberviewer-window-{}-{tag}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file_in(dir: &Path, name: &str, text: &str) -> PathBuf {
        let p = dir.join(name);
        fs::write(&p, text).unwrap();
        p
    }

    #[test]
    fn the_parameters_start_with_the_values_of_the_command_line() {
        let w = Window::new(Lang::En);
        let v = w.values().unwrap();
        assert_eq!(
            v,
            Values {
                opacity: 0.85,
                margin_mm: 2.0,
                tolerance_mm: 0.5,
                arc_tolerance_mm: 0.005
            }
        );
        assert_eq!(w.opacity(), 0.85);
    }

    #[test]
    fn a_value_that_is_not_valid_names_its_parameter() {
        let mut w = Window::new(Lang::En);
        w.set_field(0, "2".into());
        match w.values() {
            Err(Status::BadValue { label, .. }) => assert_eq!(label, Msg::Opacity),
            other => panic!("{other:?}"),
        }
        assert_eq!(w.opacity(), 0.85);
        w.set_field(0, "0.5".into());
        w.set_field(3, "abc".into());
        match w.values() {
            Err(Status::BadValue { label, .. }) => assert_eq!(label, Msg::ArcTolerance),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn colours_go_from_text_to_numbers_and_back() {
        assert_eq!(parse_hex("#c8a33c"), (0xc8, 0xa3, 0x3c));
        assert_eq!(hex((0xc8, 0xa3, 0x3c)), "#c8a33c");
        assert_eq!(parse_hex("red"), (0xb0, 0xb0, 0xb0));
        assert_eq!(parse_hex("#12345"), (0xb0, 0xb0, 0xb0));
        assert_eq!(parse_hex("#12345g"), (0xb0, 0xb0, 0xb0));
        assert_eq!(parse_hex(default_color("top.gtl")), (0xc8, 0xa3, 0x3c));
    }

    #[test]
    fn adding_a_layer_draws_it_and_says_what_it_is() {
        let dir = scratch("add");
        let top = file_in(&dir, "top.gtl", COPPER);
        let mut w = Window::new(Lang::En);
        assert!(w.add_layer(&top));
        assert_eq!(w.layers.len(), 1);
        assert_eq!(w.selected, Some(0));
        assert!(!w.layers[0].drill);
        assert!(matches!(w.status, Status::Opened { shapes, .. } if shapes > 0));
        w.fit(800.0, 600.0);
        assert!(w.viewport.px_per_mm > 1.0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_drill_file_is_told_by_its_name_or_by_its_header() {
        let dir = scratch("drill");
        let mut w = Window::new(Lang::En);
        assert!(w.add_layer(&file_in(&dir, "holes.drl", DRILL)));
        assert!(w.add_layer(&file_in(&dir, "holes.txt", DRILL)));
        assert!(w.layers.iter().all(|l| l.drill));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_cannot_be_drawn_is_refused_and_the_layers_stay() {
        let dir = scratch("refuse");
        let mut w = Window::new(Lang::En);
        assert!(w.add_layer(&file_in(&dir, "top.gtl", COPPER)));
        assert!(!w.add_layer(&file_in(
            &dir,
            "empty.gtl",
            "%FSLAX24Y24*%\n%MOMM*%\nM02*\n"
        )));
        assert!(matches!(w.status, Status::Failed(_)));
        assert!(w.status.is_problem());
        assert!(!w.add_layer(Path::new("/this/file/does/not/exist.gtl")));
        assert_eq!(w.layers.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn taking_a_layer_out_keeps_a_sensible_selection() {
        let dir = scratch("remove");
        let mut w = Window::new(Lang::En);
        for n in ["a.gtl", "b.gbl", "c.gts"] {
            assert!(w.add_layer(&file_in(&dir, n, COPPER)));
        }
        assert_eq!(w.selected, Some(2));
        w.remove_layer(0);
        assert_eq!(w.selected, Some(1));
        w.remove_layer(1);
        assert_eq!(w.selected, Some(0));
        w.remove_layer(0);
        assert_eq!(w.selected, None);
        w.remove_layer(5);
        assert!(w.layers.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn measuring_gives_the_narrowest_track_the_gap_and_the_pads() {
        let dir = scratch("measure");
        let mut w = Window::new(Lang::En);
        assert!(!w.measure());
        assert_eq!(w.status, Status::NoLayer);
        assert!(w.add_layer(&file_in(&dir, "top.gtl", COPPER)));
        assert!(w.measure());
        assert_eq!(w.report[0], Report::Narrowest(0.5));
        assert!(matches!(w.report[2], Report::Pads(_)));
        assert!(w.report[0]
            .text(Lang::En)
            .starts_with("Narrowest track: 0.500"));
        assert!(w.report[0].text(Lang::Es).starts_with("Pista más estrecha"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn layers_that_line_up_and_holes_inside_the_copper_are_good_news() {
        let dir = scratch("lined");
        let mut w = Window::new(Lang::En);
        assert!(w.add_layer(&file_in(&dir, "top.gtl", COPPER)));
        assert!(w.add_layer(&file_in(&dir, "bottom.gbl", COPPER)));
        assert!(w.add_layer(&file_in(&dir, "holes.drl", DRILL)));
        assert!(w.check());
        assert_eq!(w.report, vec![Report::Lined, Report::HolesInside]);
        assert!(!w.report[0].is_problem());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_shifted_layer_and_a_hole_outside_the_copper_are_findings() {
        let dir = scratch("findings");
        let mut w = Window::new(Lang::En);
        assert!(w.add_layer(&file_in(&dir, "top.gtl", COPPER)));
        assert!(w.add_layer(&file_in(&dir, "bottom.gbl", SHIFTED)));
        assert!(w.add_layer(&file_in(&dir, "far.drl", FAR_DRILL)));
        assert!(w.check());
        assert!(w.report.len() >= 2);
        assert!(w
            .report
            .iter()
            .all(|r| matches!(r, Report::Finding { .. }) && r.is_problem()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn one_layer_alone_has_nothing_to_compare() {
        let dir = scratch("alone");
        let mut w = Window::new(Lang::En);
        assert!(!w.check());
        assert!(w.add_layer(&file_in(&dir, "top.gtl", COPPER)));
        assert!(w.check());
        assert_eq!(w.report, vec![Report::NothingChecked]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn exporting_writes_the_picture_of_the_layers_that_are_shown() {
        let dir = scratch("export");
        let out = dir.join("layers.svg");
        let mut w = Window::new(Lang::En);
        assert!(w.add_layer(&file_in(&dir, "top.gtl", COPPER)));
        assert!(w.add_layer(&file_in(&dir, "holes.drl", DRILL)));
        w.layers[1].visible = false;
        assert!(w.export(&out));
        let svg = fs::read_to_string(&out).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("id=\"top.gtl\""));
        assert!(!svg.contains("id=\"holes.drl\""));
        assert!(svg.contains("fill=\"#c8a33c\""));
        assert_eq!(
            w.status,
            Status::Exported {
                file: "layers.svg".into()
            }
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_picture_may_not_be_a_layer_nor_a_file_that_is_not_an_svg() {
        let dir = scratch("refuse-out");
        let top = file_in(&dir, "top.svg", COPPER);
        let mut w = Window::new(Lang::En);
        assert!(!w.export(&dir.join("x.svg")));
        assert_eq!(w.status, Status::NoLayer);
        assert!(w.add_layer(&top));
        assert!(!w.export(&dir.join("picture.txt")));
        assert_eq!(w.status, Status::NotSvg);
        assert!(!w.export(&top));
        assert_eq!(w.status, Status::SameFile);
        assert_eq!(fs::read_to_string(&top).unwrap(), COPPER);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sample_files_of_the_repository_open_together() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
        let mut w = Window::new(Lang::En);
        assert!(w.add_layer(&root.join("sample.gtl")));
        assert!(w.add_layer(&root.join("sample.ncd")));
        assert_eq!(w.layers.len(), 2);
        assert!(w.layers[1].drill);
    }

    #[test]
    fn the_sentences_of_the_status_are_in_the_language_chosen() {
        let st = Status::Opened {
            name: "top.gtl".into(),
            shapes: 5,
            width_mm: 10.0,
            height_mm: 2.5,
        };
        assert_eq!(st.text(Lang::En), "top.gtl: 5 shapes, 10.00 x 2.50 mm");
        assert_eq!(st.text(Lang::De), "top.gtl: 5 Formen, 10.00 x 2.50 mm");
        assert_eq!(Status::Ready.text(Lang::Fr), "Prêt");
        assert!(!st.is_problem());
        assert!(Status::NotSvg.is_problem());
    }
}
