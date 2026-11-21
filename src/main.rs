// EH3D-GERBERVIEWER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

mod cli_args;

use eh3d_gerberviewer::align::{compare_boxes, holes_outside};
use eh3d_gerberviewer::gap::layer_gap;
use eh3d_gerberviewer::measure::{min_clearance, min_track_width, pad_centres};
use eh3d_gerberviewer::ruler::{ruler, Unit};
use eh3d_gerberviewer::scene::{from_excellon, from_gerber_with, Scene, SceneOptions};
use eh3d_gerberviewer::settings::Settings;
use eh3d_gerberviewer::svg::{default_color, render, Layer};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

/// What every command takes from the command line besides the layers.
#[derive(Clone, Copy)]
struct Reading {
    /// The largest file read, in bytes.
    max_bytes: usize,
    /// The largest distance, in mm, between an arc and the polygon drawn for it.
    arc_tolerance_mm: f64,
}

/// The language of the person from the variables of the system (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`); English when it is none of them.
fn language() -> &'static str {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = env::var(var) {
            if v.is_empty() {
                continue;
            }
            let code = v.chars().take(2).collect::<String>().to_ascii_lowercase();
            return ["es", "de", "fr", "it", "ja", "zh", "pt"]
                .into_iter()
                .find(|l| *l == code)
                .unwrap_or("en");
        }
    }
    "en"
}

/// The message of a file that is over the size limit, in the language of the person.
fn too_large(size: usize, limit: usize) -> String {
    let human = |n: usize| {
        if n >= 1_048_576 {
            format!("{} MiB", n / 1_048_576)
        } else {
            format!("{n} bytes")
        }
    };
    let (s, l) = (human(size), human(limit));
    match language() {
        "es" => format!("el fichero supera el límite ({s} frente a {l}); use --max-bytes si es correcto"),
        "de" => format!("die Datei überschreitet die Grenze ({s} gegenüber {l}); --max-bytes verwenden, wenn das richtig ist"),
        "fr" => format!("le fichier dépasse la limite ({s} contre {l}) ; utilisez --max-bytes si c'est voulu"),
        "it" => format!("il file supera il limite ({s} contro {l}); usare --max-bytes se è corretto"),
        "ja" => format!("ファイルが上限を超えています（{s}、上限 {l}）。正しい場合は --max-bytes を使ってください"),
        "zh" => format!("文件超过上限（{s}，上限 {l}）；如无误，请使用 --max-bytes"),
        "pt" => format!("o ficheiro excede o limite ({s} contra {l}); use --max-bytes se estiver correto"),
        _ => format!("the file is above the limit ({s} against {l}); use --max-bytes if that is right"),
    }
}

const HELP: &str = "eh3d-gerberview: a light viewer of Gerber layers and drill files

  eh3d-gerberview render OUT.svg LAYER... [--opacity 0.8] [--margin 2] [--exact]   draw the layers in one SVG picture
  eh3d-gerberview info LAYER...                                          size, shapes and what was not drawn
  eh3d-gerberview measure LAYER                                          narrowest track and smallest gap between pads and tracks
  eh3d-gerberview check LAYER... [--tolerance 0.5] [--exact]             do the layers line up? are the holes inside the copper?
  eh3d-gerberview ruler LAYER X0 Y0 X1 Y1 [--unit mm|in|mil] [--snap-tolerance 0.15] [--config FILE]   distance and angle between two points, caught by the nearest vertex or pad centre
  eh3d-gerberview gap LAYER_A LAYER_B [--min MM] [--config FILE]         smallest gap between a shape of one layer and a shape of the other (ends with 2 when it is below --min)
  eh3d-gerberview [gui] [LAYER...]                                       the window (a build with --features gui): with no arguments, or with a single layer, it opens; the commands above never open it
  eh3d-gerberview gui [FILE...] --shot OUT.png [--size WxH] [--theme forge|dark|light] [--wait N] [--lang CODE] [--replace]   a picture of the window (a build with --features gui)

render and check write on the error output what a layer is not drawn exactly (clear polarity, step and repeat, macro apertures); with --exact they refuse to go on when there is any.

Every command also takes --max-bytes N (the largest file read, 268435456 by default; a larger one is refused before it is read) and --arc-tolerance MM (how closely
arcs are followed, 0.005 by default).

A LAYER is a Gerber file (.gtl .gbl .gts .gbs .gto .gbo .gtp .gbp .gko .gm1 .gbr ...) or a drill file (.drl .ncd .xln .exc).";

/// Every option of a command (the options of the reading are taken before).
const ALL_OPTIONS: [&str; 8] = [
    "--opacity",
    "--margin",
    "--tolerance",
    "--exact",
    "--unit",
    "--snap-tolerance",
    "--config",
    "--min",
];

/// The settings of the file that `--config` names, or the defaults.
fn settings_of(args: &mut Vec<String>) -> Result<Settings, String> {
    match cli_args::flag(args, "--config")? {
        Some(path) => Settings::load(Path::new(&path)),
        None => Ok(Settings::default()),
    }
}

fn is_drill(name: &str) -> bool {
    // a name with no dot ("drl") has no extension
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

/// True when a file name ends like a layer or a drill file: a picture is never written there.
fn is_layer_name(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    name.contains('.')
        && (is_drill(name)
            || matches!(
                ext.as_str(),
                "gtl"
                    | "gbl"
                    | "gts"
                    | "gbs"
                    | "gto"
                    | "gbo"
                    | "gtp"
                    | "gbp"
                    | "gko"
                    | "gm1"
                    | "gbr"
            ))
}

/// Shows what the reader said about a file (warnings and errors, one line each, on the error output) and refuses a file the reader found errors in:
/// a layer that is drawn half is not the layer of the board.
fn report(path: &str, diagnostics: &[eh3d_parser_gerber::Diagnostic]) -> Result<(), String> {
    let mut errors = 0;
    for d in diagnostics {
        let kind = match d.severity {
            eh3d_parser_gerber::Severity::Info => continue,
            eh3d_parser_gerber::Severity::Error => {
                errors += 1;
                "error"
            }
            _ => "warning",
        };
        eprintln!(
            "{}: line {}: {kind}: {} [{}]",
            short(path),
            d.line,
            d.message,
            d.code
        );
    }
    if errors > 0 {
        return Err(format!(
            "{}: the reader found {errors} error(s): the layer is not complete",
            short(path)
        ));
    }
    Ok(())
}

fn load(path: &str, opts: &Reading) -> Result<Scene, String> {
    load_kind(path, opts).map(|(scene, _)| scene)
}

/// Reads a layer and says whether it is a drill file: by its extension, or by its header `M48` when the name says nothing (`.txt`, `.nc`, no extension).
fn load_kind(path: &str, opts: &Reading) -> Result<(Scene, bool), String> {
    use eh3d_parser_gerber::{read_file_limited, InputError, InputLimits, ReadError};
    let bytes = read_file_limited(
        Path::new(path),
        &InputLimits {
            max_bytes: opts.max_bytes,
        },
    )
    .map_err(|e| match e {
        ReadError::TooLarge(InputError::TooLarge { size, limit }) => {
            format!("{}: {}", short(path), too_large(size, limit))
        }
        other => format!("{path}: {other}"),
    })?;
    let text = String::from_utf8_lossy(&bytes);
    if is_drill(path) || eh3d_gerberviewer::looks_like_drill(&text) {
        let file = eh3d_parser_excellon::parse(&text);
        report(path, &file.diagnostics)?;
        Ok((from_excellon(&file), true))
    } else {
        let file = eh3d_parser_gerber::parse(&text);
        report(path, &file.diagnostics)?;
        Ok((
            from_gerber_with(
                &file,
                &SceneOptions {
                    arc_tolerance_mm: opts.arc_tolerance_mm,
                },
            ),
            false,
        ))
    }
}

/// Like `load`, for the commands that need something to draw or measure.
fn load_drawable(path: &str, opts: &Reading) -> Result<Scene, String> {
    let s = load(path, opts)?;
    if s.shapes.is_empty() {
        return Err(format!("{}: has nothing to draw", short(path)));
    }
    Ok(s)
}

/// A number of zero or more given as `--name VALUE`, or `default` when the option is not there: the option is taken out of `args` by the shared reader of the arguments.
fn flag(args: &mut Vec<String>, name: &str, default: f64) -> Result<f64, String> {
    match cli_args::flag(args, name)? {
        Some(v) => cli_args::num(&v, name)
            .ok()
            .filter(|n| *n >= 0.0)
            .ok_or_else(|| format!("{name} '{v}' is not a number of zero or more")),
        None => Ok(default),
    }
}

/// Writes on the error output what each scene does not draw exactly (`note: LAYER: ...`) and says whether there was any.
fn say_notes<'a>(scenes: impl Iterator<Item = (&'a str, &'a Scene)>) -> bool {
    let mut any = false;
    for (name, scene) in scenes {
        for n in &scene.notes {
            eprintln!("note: {name}: not drawn exactly: {n}");
            any = true;
        }
    }
    any
}

/// True when two paths name the same file (once resolved; by their text when one does not exist yet).
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// Written beside and renamed, so that a file is never left half written.
fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let tmp = std::path::PathBuf::from(format!("{}.tmp", path.display()));
    fs::write(&tmp, text)
        .and_then(|()| fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("{}: {e}", path.display())
        })
}

fn short(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "help" | "--help" | "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let cmd = args.remove(0);
    if cmd == "gui" {
        // a build with the window never gets here (main opens it): this one is only a command line
        return Err("this build has no window: build it with --features gui".into());
    }
    let max_bytes = flag(
        &mut args,
        "--max-bytes",
        eh3d_parser_gerber::InputLimits::default().max_bytes as f64,
    )?;
    let arc_tolerance_mm = flag(
        &mut args,
        "--arc-tolerance",
        eh3d_gerberviewer::scene::DEFAULT_ARC_TOLERANCE_MM,
    )?;
    if !(0.0001..=1.0).contains(&arc_tolerance_mm) {
        return Err(format!(
            "--arc-tolerance {arc_tolerance_mm} must be from 0.0001 to 1 mm"
        ));
    }
    let opts = Reading {
        max_bytes: max_bytes as usize,
        arc_tolerance_mm,
    };
    if let Some(o) = args
        .iter()
        .find(|a| a.starts_with("--") && !ALL_OPTIONS.contains(&a.as_str()))
    {
        return Err(format!("{o}: unknown option (see help)"));
    }
    // an option of another command is a wrong use, not a layer that cannot be read
    let takes: &[&str] = match cmd.as_str() {
        "render" => &["--opacity", "--margin", "--exact"],
        "check" => &["--tolerance", "--exact"],
        "ruler" => &["--unit", "--snap-tolerance", "--config"],
        "gap" => &["--min", "--config"],
        _ => &[],
    };
    if matches!(
        cmd.as_str(),
        "render" | "info" | "measure" | "check" | "ruler" | "gap"
    ) {
        if let Some(o) = args
            .iter()
            .find(|a| ALL_OPTIONS.contains(&a.as_str()) && !takes.contains(&a.as_str()))
        {
            return Err(format!("{cmd}: {o} is not an option of {cmd} (see help)"));
        }
    }
    let exact = cli_args::switch(&mut args, "--exact");
    match cmd.as_str() {
        "render" => {
            let opacity = flag(&mut args, "--opacity", 0.85)?;
            if opacity > 1.0 {
                return Err(format!("--opacity {opacity} must be from 0 to 1"));
            }
            let margin = flag(&mut args, "--margin", 2.0)?;
            if args.len() < 2 {
                return Err("render needs an output file and at least one layer".into());
            }
            let out = args.remove(0);
            // the arguments written the other way round (LAYER OUT.svg) would overwrite a layer with the picture
            if is_layer_name(&out) || args.iter().any(|p| p == &out) {
                return Err(format!(
                    "render: {out} looks like a layer, not a picture; the output file comes first (render OUT.svg LAYER...)"
                ));
            }
            let layers = args
                .iter()
                .map(|p| {
                    Ok(Layer {
                        name: short(p),
                        scene: load_drawable(p, &opts)?,
                        color: default_color(p).to_string(),
                        opacity,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let lossy = say_notes(layers.iter().map(|l| (l.name.as_str(), &l.scene)));
            if exact && lossy {
                return Err("render: a layer is not drawn exactly (see the notes above); nothing was written".into());
            }
            // never over a layer, whatever the way the path is written
            if args
                .iter()
                .any(|p| same_file(Path::new(p), Path::new(&out)))
            {
                return Err(format!(
                    "render: {out} is one of the layers; the picture would be written over it"
                ));
            }
            write_atomic(Path::new(&out), &render(&layers, margin))?;
            println!("{} layers drawn in {out}", layers.len());
            Ok(())
        }
        "info" => {
            if args.is_empty() {
                return Err("info needs a layer".into());
            }
            for p in &args {
                let s = load_drawable(p, &opts)?;
                let b = s.bounds();
                println!(
                    "{}: {} shapes, {:.3} x {:.3} mm",
                    short(p),
                    s.shapes.len(),
                    b.width(),
                    b.height()
                );
                for n in &s.notes {
                    println!("  not drawn exactly: {n}");
                }
            }
            Ok(())
        }
        "measure" => {
            let p = args.first().ok_or("measure needs a layer")?;
            if args.len() > 1 {
                return Err(format!(
                    "measure takes one layer, not {} (usage: measure LAYER)",
                    args.len()
                ));
            }
            let s = load_drawable(p, &opts)?;
            match min_track_width(&s) {
                Some(w) => println!("narrowest track: {w:.3} mm"),
                None => println!("no tracks"),
            }
            match min_clearance(&s)? {
                Some((g, i, j)) => println!("smallest gap: {g:.3} mm between shapes {i} and {j}"),
                None => println!("fewer than two pads or tracks"),
            }
            println!("{} pads", pad_centres(&s).len());
            Ok(())
        }
        "ruler" => {
            let mut settings = settings_of(&mut args)?;
            if let Some(u) = cli_args::flag(&mut args, "--unit")? {
                settings.unit = Unit::parse(&u)
                    .ok_or_else(|| format!("--unit '{u}' must be from mm, in or mil"))?;
            }
            settings.snap_tolerance_mm =
                flag(&mut args, "--snap-tolerance", settings.snap_tolerance_mm)?;
            if settings.snap_tolerance_mm > 10.0 {
                return Err(format!(
                    "--snap-tolerance {} must be from 0 to 10 mm",
                    settings.snap_tolerance_mm
                ));
            }
            if args.len() != 5 {
                return Err(
                    "ruler needs a layer and four numbers (usage: ruler LAYER X0 Y0 X1 Y1)".into(),
                );
            }
            let coords = args[1..]
                .iter()
                .map(|t| cli_args::num(t, "coordinate"))
                .collect::<Result<Vec<_>, _>>()?;
            let scene = load_drawable(&args[0], &opts)?;
            let m = ruler(
                &scene,
                (coords[0], coords[1]),
                (coords[2], coords[3]),
                &settings,
            )?;
            for line in m.lines() {
                println!("{line}");
            }
            Ok(())
        }
        "gap" => {
            let settings = settings_of(&mut args)?;
            let min = match cli_args::flag(&mut args, "--min")? {
                Some(v) => Some(
                    cli_args::num(&v, "--min")
                        .ok()
                        .filter(|n| *n >= 0.0)
                        .ok_or_else(|| format!("--min '{v}' is not a number of zero or more"))?,
                ),
                None => None,
            };
            if args.len() != 2 {
                return Err("gap needs two layers (usage: gap LAYER_A LAYER_B)".into());
            }
            let a = load_drawable(&args[0], &opts)?;
            let b = load_drawable(&args[1], &opts)?;
            say_notes([(args[0].as_str(), &a), (args[1].as_str(), &b)].into_iter());
            let report = layer_gap(&a, &b, settings.max_gap_pairs)?;
            if report.skipped > 0 {
                eprintln!(
                    "note: {} shapes with a number that is not valid were left out",
                    report.skipped
                );
            }
            let Some(g) = report.best else {
                println!("one of the layers has no shape to compare");
                return Ok(());
            };
            println!(
                "smallest gap: {:.3} mm between {} {} of {} and {} {} of {}{}",
                g.distance,
                g.a_kind,
                g.a_index,
                short(&args[0]),
                g.b_kind,
                g.b_index,
                short(&args[1]),
                if g.distance == 0.0 {
                    " (they touch or overlap)"
                } else {
                    ""
                }
            );
            match min {
                Some(m) if g.distance < m => Err(format!(
                    "verification failed: the gap {:.3} mm is below the minimum {m} mm",
                    g.distance
                )),
                _ => Ok(()),
            }
        }
        "check" => {
            let tol = flag(&mut args, "--tolerance", 0.5)?;
            if args.is_empty() {
                return Err("check needs layers".into());
            }
            let scenes = args
                .iter()
                .map(|p| {
                    let (scene, drill) = load_kind(p, &opts)?;
                    Ok((short(p), drill, scene))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let lossy = say_notes(scenes.iter().map(|(n, _, s)| (n.as_str(), s)));
            if exact && lossy {
                return Err("check: a layer is not drawn exactly (see the notes above), so what it says about the layers cannot be trusted".into());
            }
            let boxes: Vec<_> = scenes
                .iter()
                .filter(|(_, d, _)| !*d)
                .map(|(n, _, s)| (n.clone(), s.bounds()))
                .collect();
            let mut findings = compare_boxes(&boxes, tol);
            let mut holes_checked = false;
            if let Some((_, _, copper)) = scenes.iter().find(|(_, d, _)| !*d) {
                for (name, _, holes) in scenes.iter().filter(|(_, d, _)| *d) {
                    holes_checked = true;
                    findings.extend(holes_outside(copper, holes, name));
                }
            }
            if findings.is_empty() {
                match (boxes.len() >= 2, holes_checked) {
                    (true, _) => println!("the layers line up"),
                    (false, true) => println!(
                        "there are no layers to compare (fewer than two Gerber layers); the holes are inside the copper"
                    ),
                    (false, false) => println!(
                        "there are no layers to compare (fewer than two Gerber layers and no drill file with copper): nothing was checked"
                    ),
                }
                Ok(())
            } else {
                Err(findings
                    .iter()
                    .map(|f| format!("{}: {}", f.layer, f.message))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
        }
        other => Err(format!("{other}: unknown command (see help)")),
    }
}

/// Whether the program opens its window instead of running a command, and the layers to open in it: no arguments, `gui` (with layers or without), or a single layer file given by the
/// system to open (a name with an extension that is a file; the commands have none).
#[cfg(feature = "gui")]
fn window_request(args: &[String]) -> Option<Vec<String>> {
    match args {
        [] => Some(Vec::new()),
        [a, rest @ ..] if a == "gui" => Some(rest.to_vec()),
        [f] if !f.starts_with('-')
            && Path::new(f).extension().is_some()
            && Path::new(f).is_file() =>
        {
            Some(vec![f.clone()])
        }
        _ => None,
    }
}

#[cfg(feature = "gui")]
fn open_window(layers: Vec<String>) -> ExitCode {
    let lang = eh3d_gerberviewer::lang::Lang::from_env();
    let layers = layers.into_iter().map(std::path::PathBuf::from).collect();
    match eh3d_gerberviewer::gui::run(layers, lang) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the window could not be opened: {e}");
            ExitCode::from(1)
        }
    }
}

/// The picture mode: `--shot FILE.png` opens the window, saves what it draws as a PNG file and closes. The exit code is 0 when the file was written, 1 when it was not and 2 when the
/// arguments are wrong.
#[cfg(feature = "gui")]
fn take_picture(args: &[String]) -> ExitCode {
    let parsed = match eh3d_gerberviewer::shot::parse_window_args(args, 64) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let Some(request) = parsed.shot else {
        eprintln!("error: --shot needs a file to write");
        return ExitCode::from(2);
    };
    let lang = parsed
        .lang
        .unwrap_or_else(eh3d_gerberviewer::lang::Lang::from_env);
    match eh3d_gerberviewer::gui::run_shot(parsed.files, lang, request) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: the picture could not be taken: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if eh3d_gerberviewer::shot::wants_picture(&args) {
        #[cfg(feature = "gui")]
        return take_picture(&args);
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("error: this build has no window: build it with --features gui");
            return ExitCode::from(1);
        }
    }
    #[cfg(feature = "gui")]
    if let Some(layers) = window_request(&args) {
        return open_window(layers);
    }
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(exit_code(&e))
        }
    }
}

/// The exit code of the convention of the suite for a run that ended badly: 64 when the command line was used wrongly (an unknown command, option or setting, or something it needs that is missing), 1 for a failure of the work itself.
fn exit_code(message: &str) -> u8 {
    // a check that ran and was not passed
    if message.starts_with("verification failed") {
        return 2;
    }
    const USAGE: [&str; 8] = [
        "unknown command",
        "unknown option",
        "unknown setting",
        " needs ",
        "(usage:",
        "(see help)",
        "is not a number",
        " must be from ",
    ];
    if USAGE.iter().any(|m| message.contains(m)) {
        64
    } else {
        1
    }
}
