// EH3D-GERBERVIEWER - command line.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]

use eh3d_gerberviewer::align::{compare_boxes, holes_outside};
use eh3d_gerberviewer::measure::{min_clearance, min_track_width, pad_centres};
use eh3d_gerberviewer::scene::{from_excellon, from_gerber_with, Scene, SceneOptions};
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

  eh3d-gerberview render OUT.svg LAYER... [--opacity 0.8] [--margin 2]   draw the layers in one SVG picture
  eh3d-gerberview info LAYER...                                          size, shapes and what was not drawn
  eh3d-gerberview measure LAYER                                          narrowest track and smallest gap between pads and tracks
  eh3d-gerberview check LAYER... [--tolerance 0.5]                       do the layers line up? are the holes inside the copper?

Every command also takes --max-bytes N (the largest file read, 268435456 by default; a larger one is refused before it is read) and --arc-tolerance MM (how closely
arcs are followed, 0.005 by default).

A LAYER is a Gerber file (.gtl .gbl .gts .gbs .gto .gbo .gtp .gbp .gko .gm1 .gbr ...) or a drill file (.drl .ncd .xln .exc).";

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
    if is_drill(path) {
        let file = eh3d_parser_excellon::parse(&text);
        report(path, &file.diagnostics)?;
        Ok(from_excellon(&file))
    } else {
        let file = eh3d_parser_gerber::parse(&text);
        report(path, &file.diagnostics)?;
        Ok(from_gerber_with(
            &file,
            &SceneOptions {
                arc_tolerance_mm: opts.arc_tolerance_mm,
            },
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

fn flag(args: &mut Vec<String>, name: &str, default: f64) -> Result<f64, String> {
    if let Some(i) = args.iter().position(|a| a == name) {
        let v = args
            .get(i + 1)
            .ok_or_else(|| format!("{name} needs a value"))?
            .clone();
        args.drain(i..=i + 1);
        return v
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .ok_or_else(|| format!("{name} '{v}' is not a number of zero or more"));
    }
    Ok(default)
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
    match cmd.as_str() {
        "render" => {
            let opacity = flag(&mut args, "--opacity", 0.85)?;
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
            fs::write(&out, render(&layers, margin)).map_err(|e| format!("{out}: {e}"))?;
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
        "check" => {
            let tol = flag(&mut args, "--tolerance", 0.5)?;
            if args.is_empty() {
                return Err("check needs layers".into());
            }
            let scenes = args
                .iter()
                .map(|p| Ok((short(p), is_drill(p), load(p, &opts)?)))
                .collect::<Result<Vec<_>, String>>()?;
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

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
