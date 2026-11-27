//! The state of the window, without any screen: the layers that are open (each with its colour and whether it is shown), the parameters being edited, the results of measuring and
//! checking, what was last said to the person and the zoom of the picture. The screen (`gui`, built with the `gui` feature) only draws this and passes on what the person does,
//! so everything here is tested as plain code.

use crate::align::{all_empty, compare_boxes, holes_outside, no_holes, slots_outside};
use crate::draw::Drawing;
use crate::eh3dfile::{self, JobFile};
use crate::lang::{fill, Lang, Msg};
use crate::layers::{Layer as StackLayer, Stack, Warning};
use crate::looks_like_drill;
use crate::measure::{min_clearance, min_track_width, pad_centres};
use crate::ruler::{measure_points, snap, Measurement, SnapKind, Snapped, Unit};
use crate::scene::{from_excellon, from_gerber_with, Bounds, Scene, SceneOptions, Shape};
use crate::settings::Settings;
use crate::svg::{default_color, render, Layer as SvgLayer};
use crate::viewport::Viewport;
use eh3d_parser_gerber::{
    read_file_limited, Diagnostic, InputError, InputLimits, ReadError, Severity,
};
use std::fs;
use std::path::{Path, PathBuf};

/// The extensions of the files the window offers to open.
pub const LAYER_EXTENSIONS: [&str; 20] = [
    "gtl", "gbl", "gts", "gbs", "gto", "gbo", "gtp", "gbp", "gko", "gm1", "gbr", "ger", "pho",
    "art", "drl", "ncd", "xln", "exc", "drd", "eh3d",
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
    /// The layers of the job of a project were opened.
    ProjectOpened {
        name: String,
        layers: usize,
    },
    /// The result was kept in the branch of the viewer.
    ProjectSaved {
        name: String,
    },
    /// The project was opened and the result in it was made from an older job.
    ProjectStale {
        name: String,
    },
    NoProject,
    SettingsLoaded {
        name: String,
    },
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
            Status::ProjectOpened { name, layers } => fill(
                Msg::ProjectOpened.tr(lang),
                &[name.as_str(), layers.to_string().as_str()],
            ),
            Status::ProjectSaved { name } => fill(Msg::ProjectSaved.tr(lang), &[name.as_str()]),
            Status::ProjectStale { name } => fill(Msg::ProjectStale.tr(lang), &[name.as_str()]),
            Status::NoProject => Msg::NoProject.tr(lang).to_string(),
            Status::SettingsLoaded { name } => fill(Msg::SettingsLoaded.tr(lang), &[name.as_str()]),
        }
    }

    /// Whether the sentence is a problem or a warning (the window shows it in another colour).
    pub fn is_problem(&self) -> bool {
        !matches!(
            self,
            Status::Ready
                | Status::Opened { .. }
                | Status::Exported { .. }
                | Status::ProjectOpened { .. }
                | Status::ProjectSaved { .. }
                | Status::SettingsLoaded { .. }
        )
    }
}

/// One line of the results of measuring or checking.
#[derive(Debug, Clone, PartialEq)]
pub enum Report {
    Narrowest(f64),
    NoTracks,
    Gap {
        gap_mm: f64,
        a: usize,
        b: usize,
    },
    FewShapes,
    Pads(usize),
    /// How many shapes the narrowest track and the smallest gap did not look at.
    NotCompared(usize),
    Lined,
    HolesInside,
    NothingChecked,
    Finding {
        layer: String,
        message: String,
    },
    /// Something the project says after a save (a signature that no longer holds).
    Notice(String),
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
            Report::NotCompared(n) => {
                fill(Msg::ReportNotCompared.tr(lang), &[n.to_string().as_str()])
            }
            Report::Lined => Msg::ReportLined.tr(lang).to_string(),
            Report::HolesInside => Msg::ReportHoles.tr(lang).to_string(),
            Report::NothingChecked => Msg::ReportNothing.tr(lang).to_string(),
            Report::Finding { layer, message } => fill(
                Msg::ReportFinding.tr(lang),
                &[layer.as_str(), message.as_str()],
            ),
            Report::Notice(text) => text.clone(),
        }
    }

    /// Whether the line reports something wrong (the window shows it in another colour).
    pub fn is_problem(&self) -> bool {
        matches!(
            self,
            Report::Finding { .. }
                | Report::NothingChecked
                | Report::NotCompared(_)
                | Report::Notice(_)
        )
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
    /// The opacity of this layer alone; the one of the parameters when there is none.
    pub opacity: Option<f64>,
    /// The file of the job of a project this layer came from.
    pub job: Option<JobFile>,
    /// What the reader warned about: the first warning of each kind and how many there were.
    pub warnings: Vec<(Diagnostic, usize)>,
}

/// The first warning of each code with the number of warnings of that code.
fn grouped(diagnostics: &[Diagnostic]) -> Vec<(Diagnostic, usize)> {
    let mut out: Vec<(Diagnostic, usize)> = Vec::new();
    for d in diagnostics {
        if d.severity == Severity::Info {
            continue;
        }
        match out.iter_mut().find(|(first, _)| first.code == d.code) {
            Some((_, n)) => *n += 1,
            None => out.push((d.clone(), 1)),
        }
    }
    out
}

/// What a person has to know about a layer before trusting it: the warnings of its reader (in the language of the person) and what of the file is not drawn exactly.
pub fn layer_warnings(layer: &LayerView, lang: Lang) -> Vec<Report> {
    let language = eh3d_parser_common::Language::from_tag(lang.code())
        .unwrap_or(eh3d_parser_common::Language::En);
    let warned = layer.warnings.iter().map(|(d, n)| {
        let text = eh3d_parser_common::localize(d, language);
        if *n > 1 {
            format!("{text} x{n}")
        } else {
            text
        }
    });
    let noted = layer
        .scene
        .notes
        .iter()
        .map(|note| fill(Msg::NotExact.tr(lang), &[note.as_str()]));
    warned
        .chain(noted)
        .map(|message| Report::Finding {
            layer: layer.name.clone(),
            message,
        })
        .collect()
}

/// The ruler of the window: two clicks on the selected layer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RulerState {
    pub on: bool,
    pub first: Option<Snapped>,
    pub measured: Option<Measurement>,
    /// The layer the clicks were made on.
    layer: Option<usize>,
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
    let drill = drill_name(&name);
    layer_from_bytes(path, name, &bytes, drill, lang, arc_tolerance_mm)
}

/// The layer drawn from the bytes of a file; `drill` says that the name or the job already tell that it is a drill file.
pub fn layer_from_bytes(
    path: &Path,
    name: String,
    bytes: &[u8],
    drill: bool,
    lang: Lang,
    arc_tolerance_mm: f64,
) -> Result<LayerView, String> {
    let text = String::from_utf8_lossy(bytes);
    let (scene, drill, errors, warnings) = if drill || looks_like_drill(&text) {
        let file = eh3d_parser_excellon::parse(&text);
        let errors = file
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        (
            from_excellon(&file),
            true,
            errors,
            grouped(&file.diagnostics),
        )
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
            grouped(&file.diagnostics),
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
        opacity: None,
        job: None,
        warnings,
    })
}

/// The name a layer of a job is coloured by: its own when its extension says what it is, or one that says what the job says it is.
fn colour_name<'a>(name: &'a str, kind: &str) -> &'a str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if name.contains('.') && !matches!(ext.as_str(), "gbr" | "ger" | "pho" | "art") {
        return name;
    }
    match kind {
        "copper" => "layer.gtl",
        "solder_mask" => "layer.gts",
        "silkscreen" => "layer.gto",
        "paste" => "layer.gtp",
        "outline" => "layer.gko",
        "drill" => "layer.drl",
        _ => name,
    }
}

/// The layer of a file of a job.
pub fn job_layer(file: &JobFile, lang: Lang, arc_tolerance_mm: f64) -> Result<LayerView, String> {
    let name = file.file_name().to_string();
    let drill = file.kind == "drill" || drill_name(&name);
    let rgb = parse_hex(default_color(colour_name(&name, &file.kind)));
    let mut layer = layer_from_bytes(
        Path::new(&file.rel),
        name,
        &file.bytes,
        drill,
        lang,
        arc_tolerance_mm,
    )?;
    layer.rgb = rgb;
    layer.job = Some(file.clone());
    Ok(layer)
}

/// A layer as the SVG picture draws it, with its own opacity or the usual one.
fn picture(l: &LayerView, usual: f64) -> SvgLayer {
    SvgLayer {
        name: l.name.clone(),
        scene: l.scene.clone(),
        color: hex(l.rgb),
        opacity: l.opacity.unwrap_or(usual),
    }
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
    /// The values read from a configuration file (`gerberview.cfg`).
    pub settings: Settings,
    /// The project whose job is open.
    pub project: Option<PathBuf>,
    pub ruler: RulerState,
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
            settings: Settings::default(),
            project: None,
            ruler: RulerState::default(),
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

    /// Adds a layer or a drill file. The layers that are open stay as they are when it cannot be read. A project of the suite (`.eh3d`) opens the layers of its job, and a configuration file
    /// (`.cfg`) gives the settings.
    pub fn add_layer(&mut self, path: &Path) -> bool {
        if eh3dfile::is_project(path) {
            return self.open_project(path);
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cfg"))
        {
            return self.load_settings(path);
        }
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
                let warnings = layer_warnings(&layer, self.lang);
                self.layers.push(layer);
                self.selected = Some(self.layers.len() - 1);
                self.report = warnings;
                self.clear_ruler();
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
            self.clear_ruler();
            let last = self.layers.len().saturating_sub(1);
            self.selected = if self.layers.is_empty() {
                None
            } else {
                self.selected
                    .map(|s| if s > index { s - 1 } else { s.min(last) })
            };
        }
    }

    /// Reads the settings of a configuration file; the opacity of the parameters takes the one of the file.
    pub fn load_settings(&mut self, path: &Path) -> bool {
        match Settings::load(path) {
            Ok(settings) => {
                if let Some(field) = self.fields.iter_mut().find(|f| f.key == "opacity") {
                    field.text = settings.default_opacity.to_string();
                }
                self.settings = settings;
                self.set_unit(self.settings.unit);
                self.status = Status::SettingsLoaded {
                    name: file_name(path),
                };
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Opens the layers of the job of a project (the branch `manufacturing/`) in place of the ones that are open, and says when the result kept in the project is out of date. Nothing
    /// changes when a layer cannot be drawn.
    pub fn open_project(&mut self, path: &Path) -> bool {
        let arc = match self.values() {
            Ok(v) => v.arc_tolerance_mm,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        let lang = self.lang;
        let kinds = [
            "copper",
            "solder_mask",
            "silkscreen",
            "paste",
            "outline",
            "drill",
        ];
        let read = eh3dfile::read_job(path, &kinds).and_then(|job| {
            let layers = job
                .iter()
                .map(|f| job_layer(f, lang, arc))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((layers, eh3dfile::state(&eh3dfile::KIT, path)?))
        });
        let name = file_name(path);
        match read {
            Ok((layers, _)) if layers.is_empty() => {
                self.status = Status::Failed(fill(Msg::NothingToDraw.tr(lang), &[name.as_str()]));
                false
            }
            Ok((layers, state)) => {
                let count = layers.len();
                self.layers = layers;
                self.selected = Some(count - 1);
                self.project = Some(path.to_path_buf());
                self.report = self.all_warnings();
                self.clear_ruler();
                self.status = if state.out_of_date {
                    Status::ProjectStale { name }
                } else {
                    Status::ProjectOpened {
                        name,
                        layers: count,
                    }
                };
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// Keeps the picture of the shown layers of the job in the branch `gerberviewer/` of the open project, with the reference to the job it was made from.
    pub fn save_project(&mut self) -> bool {
        let Some(project) = self.project.clone() else {
            self.status = Status::NoProject;
            return false;
        };
        let values = match self.values() {
            Ok(v) => v,
            Err(st) => {
                self.status = st;
                return false;
            }
        };
        let shown: Vec<&LayerView> = self
            .layers
            .iter()
            .filter(|l| l.visible && l.job.is_some())
            .collect();
        if shown.is_empty() {
            self.status = Status::NoLayer;
            return false;
        }
        let inputs: Vec<JobFile> = shown.iter().filter_map(|l| l.job.clone()).collect();
        let pictures: Vec<SvgLayer> = shown.iter().map(|l| picture(l, values.opacity)).collect();
        let svg = render(&pictures, values.margin_mm).into_bytes();
        match eh3dfile::save_result(
            &eh3dfile::KIT,
            &project,
            &inputs,
            &[("layers.svg".to_string(), svg)],
        ) {
            Ok(notices) => {
                self.report = notices.into_iter().map(Report::Notice).collect();
                let warnings = self.all_warnings();
                self.report.extend(warnings);
                self.status = Status::ProjectSaved {
                    name: file_name(&project),
                };
                true
            }
            Err(why) => {
                self.status = Status::Failed(why);
                false
            }
        }
    }

    /// The warnings of every layer that is open.
    fn all_warnings(&self) -> Vec<Report> {
        self.layers
            .iter()
            .flat_map(|l| layer_warnings(l, self.lang))
            .collect()
    }

    /// The opacity a layer is drawn with: its own, or the one of the parameters.
    pub fn opacity_of(&self, index: usize) -> f64 {
        self.layers
            .get(index)
            .and_then(|l| l.opacity)
            .unwrap_or_else(|| self.opacity())
    }

    /// Gives a layer its own opacity; a value outside 0 to 1 is refused and the layer stays as it was.
    pub fn set_layer_opacity(&mut self, index: usize, opacity: f64) -> bool {
        match self.layers.get_mut(index) {
            Some(layer) if opacity.is_finite() && (0.0..=1.0).contains(&opacity) => {
                layer.opacity = Some(opacity);
                true
            }
            _ => false,
        }
    }

    /// Moves a layer to another place of the stack (0 is drawn first, under the others); the selection follows its layer.
    pub fn move_layer(&mut self, from: usize, to: usize) -> bool {
        let n = self.layers.len();
        if from >= n || to >= n || from == to {
            return false;
        }
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
        self.selected = self.selected.map(|s| {
            if s == from {
                to
            } else if from < s && s <= to {
                s - 1
            } else if to <= s && s < from {
                s + 1
            } else {
                s
            }
        });
        self.report.clear();
        self.clear_ruler();
        true
    }

    /// What is odd about the stack as it is drawn: a shown layer that can hardly be seen, or one that a solid layer above may hide.
    pub fn stack_warnings(&self) -> Vec<String> {
        let mut stack = Stack::new(&self.settings);
        let mut bounds = Vec::new();
        for (i, l) in self.layers.iter().enumerate() {
            let layer = StackLayer {
                name: l.name.clone(),
                rgb: [l.rgb.0, l.rgb.1, l.rgb.2],
                opacity: self.opacity_of(i),
                visible: l.visible,
            };
            if stack.push(layer).is_err() {
                break;
            }
            bounds.push(l.scene.bounds());
        }
        let name = |i: usize| self.layers.get(i).map_or("", |l| l.name.as_str());
        stack
            .warnings(&bounds, &self.settings)
            .unwrap_or_default()
            .into_iter()
            .map(|w| match w {
                Warning::AlmostInvisible { layer, .. } => {
                    fill(Msg::WarnInvisible.tr(self.lang), &[name(layer)])
                }
                Warning::MayBeHidden { layer, by } => {
                    fill(Msg::WarnHidden.tr(self.lang), &[name(layer), name(by)])
                }
            })
            .collect()
    }

    /// Switches the ruler on or off; a measure that was there goes.
    pub fn set_ruler(&mut self, on: bool) {
        self.ruler = RulerState {
            on,
            ..RulerState::default()
        };
    }

    fn clear_ruler(&mut self) {
        self.set_ruler(self.ruler.on);
    }

    /// The unit the ruler writes in; a measure that is there is written again.
    pub fn set_unit(&mut self, unit: Unit) {
        self.settings.unit = unit;
        if let Some(m) = self.ruler.measured.take() {
            self.ruler.measured = Some(measure_points(m.from, m.to, unit));
        }
    }

    /// A click of the ruler at a point of the board in millimetres: the first one starts a measure, the second one ends it. The point is caught by the pad or vertex of the selected layer
    /// that is within the snap tolerance. Says whether the click was taken.
    pub fn ruler_click(&mut self, at: (f64, f64)) -> bool {
        if !self.ruler.on {
            return false;
        }
        let Some((index, layer)) = self
            .selected
            .and_then(|s| self.layers.get(s).map(|l| (s, l)))
        else {
            self.status = Status::NoLayer;
            return false;
        };
        let point = match snap(&layer.scene, at, &self.settings) {
            Ok(p) => p,
            Err(why) => {
                self.status = Status::Failed(why);
                return false;
            }
        };
        let open = self.ruler.measured.is_none() && self.ruler.layer == Some(index);
        match self.ruler.first.clone() {
            Some(from) if open => {
                self.ruler.measured = Some(measure_points(from, point, self.settings.unit));
            }
            _ => {
                self.ruler.first = Some(point);
                self.ruler.measured = None;
                self.ruler.layer = Some(index);
            }
        }
        true
    }

    /// What the ruler says, in the language of the person; nothing while it is off.
    pub fn ruler_lines(&self) -> Vec<String> {
        let lang = self.lang;
        if !self.ruler.on {
            return Vec::new();
        }
        let at = |p: &Snapped| {
            let kind = match p.kind {
                SnapKind::Free => Msg::SnapFree,
                SnapKind::PadCentre => Msg::SnapPad,
                SnapKind::Vertex => Msg::SnapVertex,
            };
            let unit = self.settings.unit;
            fill(
                Msg::RulerAt.tr(lang),
                &[
                    format!("{:.3}", unit.from_mm(p.point.0)).as_str(),
                    format!("{:.3}", unit.from_mm(p.point.1)).as_str(),
                    kind.tr(lang),
                ],
            )
        };
        match (&self.ruler.first, &self.ruler.measured) {
            (_, Some(m)) => vec![
                at(&m.from),
                at(&m.to),
                fill(
                    Msg::RulerResult.tr(lang),
                    &[
                        format!("{:.4}", m.distance).as_str(),
                        m.unit.label(),
                        format!("{:+.4}", m.dx).as_str(),
                        format!("{:+.4}", m.dy).as_str(),
                        format!("{:.2}", m.angle_deg).as_str(),
                    ],
                ),
            ],
            (Some(first), None) => vec![at(first), Msg::RulerSecond.tr(lang).to_string()],
            (None, None) => vec![Msg::RulerFirst.tr(lang).to_string()],
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
        let left = scene
            .shapes
            .iter()
            .filter(|s| !matches!(s, Shape::Disc { .. } | Shape::Stroke { .. }))
            .count();
        if left > 0 {
            out.push(Report::NotCompared(left));
        }
        out.extend(layer_warnings(layer, self.lang));
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
        findings.extend(all_empty(&boxes));
        for holes in self.layers.iter().filter(|l| l.drill) {
            findings.extend(no_holes(&holes.scene, &holes.name));
        }
        let mut holes_checked = false;
        if let Some(copper) = self.layers.iter().find(|l| !l.drill) {
            for holes in self.layers.iter().filter(|l| l.drill) {
                holes_checked = true;
                findings.extend(holes_outside(&copper.scene, &holes.scene, &holes.name));
                findings.extend(slots_outside(&copper.scene, &holes.scene, &holes.name));
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
        out.extend(self.all_warnings());
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
            .map(|l| picture(l, values.opacity))
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
        assert_eq!(w.report, [Report::Lined, Report::HolesInside]);
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
        assert_eq!(w.report, [Report::NothingChecked]);
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
