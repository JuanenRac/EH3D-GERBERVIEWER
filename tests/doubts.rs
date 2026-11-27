//! What must not pass in silence: a file that may be cut or has no units, a check with nothing to look at, a slot outside the copper, and a gap that is only big because
//! something was not drawn.

use eh3d_gerberviewer::lang::Lang;
use eh3d_gerberviewer::window::{Report, Window};
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");
const BOARD: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.4*%\nD10*\nX00000Y00000D02*\nX400000Y00000D01*\nX400000Y250000D01*\nM02*\n";
// the same layer with no end command, as a file that was cut between two lines
const CUT: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.4*%\nD10*\nX00000Y00000D02*\nX400000Y00000D01*\nX400000Y250000D01*\n";
const EMPTY: &str = "%FSLAX24Y24*%\n%MOMM*%\nM02*\n";
const FAR_SLOT: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX90.000Y90.000G85X95.000Y90.000\nM30\n";
const NO_HOLE: &str = "M48\nMETRIC\nT1C0.8\n%\nM30\n";
const PAD: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.4*%\nD10*\nX500000Y500000D03*\nM02*\n";
// a pad far away and a flash of a macro aperture that is not drawn
const MACRO_LOST: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%AMODD*7,0,0,1.0,0.5,0.1,0*%\n%ADD10C,0.4*%\n%ADD11ODD*%\nD10*\nX00000Y00000D03*\nD11*\nX495000Y500000D03*\nM02*\n";

fn file(name: &str, text: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("doubts-{name}"));
    std::fs::write(&path, text).unwrap();
    path
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(BIN).args(args).output().expect("it runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn text(p: &PathBuf) -> &str {
    p.to_str().unwrap()
}

#[test]
fn the_window_says_that_a_file_may_be_cut() {
    let mut w = Window::new(Lang::Es);
    assert!(w.add_layer(&file("cut.gtl", CUT)));
    assert!(
        w.report
            .iter()
            .any(|r| r.is_problem() && r.text(Lang::Es).contains("end-missing")),
        "{:?}",
        w.report
    );
    // measuring and checking keep saying it
    assert!(w.measure());
    assert!(w
        .report
        .iter()
        .any(|r| r.text(Lang::Es).contains("end-missing")));
    assert!(w.check());
    assert!(w
        .report
        .iter()
        .any(|r| r.text(Lang::Es).contains("end-missing")));
}

#[test]
fn render_and_check_with_exact_refuse_a_file_that_may_be_cut() {
    let cut = file("cut-exact.gtl", CUT);
    let svg = file("cut-exact.svg", "");
    let (code, _, e) = run(&["render", text(&svg), text(&cut), "--exact"]);
    assert_eq!(code, Some(1), "{e}");
    assert!(e.contains("may be cut short"), "{e}");
    assert_eq!(run(&["check", text(&cut), "--exact"]).0, Some(1));
    // without --exact they go on and say it
    let (code, _, e) = run(&["render", text(&svg), text(&cut)]);
    assert_eq!(code, Some(0), "{e}");
    assert!(e.contains("may be cut short"), "{e}");
}

#[test]
fn two_empty_layers_do_not_line_up() {
    let (a, b) = (file("empty-a.gtl", EMPTY), file("empty-b.gbl", EMPTY));
    let (code, out, e) = run(&["check", text(&a), text(&b)]);
    assert_eq!(code, Some(1), "{out}{e}");
    assert!(
        e.contains("has nothing to draw") && !out.contains("line up"),
        "{out}{e}"
    );
}

#[test]
fn a_slot_outside_the_copper_and_a_drill_file_with_no_hole_are_findings() {
    let board = file("board.gtl", BOARD);
    let slot = file("slot.drl", FAR_SLOT);
    let (code, out, e) = run(&["check", text(&board), text(&slot)]);
    assert_eq!(code, Some(1), "{out}{e}");
    assert!(e.contains("the slot of 0.80 mm"), "{e}");
    let none = file("none.drl", NO_HOLE);
    let (code, out, e) = run(&["check", text(&board), text(&none)]);
    assert_eq!(code, Some(1), "{out}{e}");
    assert!(e.contains("has no hole and no slot"), "{e}");
    let mut w = Window::new(Lang::En);
    assert!(w.add_layer(&board));
    assert!(w.add_layer(&slot));
    assert!(w.check());
    assert!(!w.report.contains(&Report::HolesInside), "{:?}", w.report);
}

#[test]
fn a_gap_that_is_big_only_because_something_was_left_out_does_not_pass() {
    let (a, b) = (file("lost.gtl", MACRO_LOST), file("pad.gbl", PAD));
    let (code, out, e) = run(&["gap", text(&a), text(&b), "--min", "1"]);
    assert_eq!(code, Some(2), "{out}{e}");
    assert!(e.contains("not conclusive"), "{e}");
    // without a minimum it is a measure, and the note is said
    let (code, _, e) = run(&["gap", text(&a), text(&b)]);
    assert_eq!(code, Some(0), "{e}");
    assert!(e.contains("a macro aperture is not drawn"), "{e}");
}

#[test]
fn measure_and_ruler_say_what_they_could_not_see() {
    let a = file("measure-lost.gtl", MACRO_LOST);
    let (code, _, e) = run(&["measure", text(&a)]);
    assert_eq!(code, Some(0), "{e}");
    assert!(e.contains("a macro aperture is not drawn"), "{e}");
    let (code, _, e) = run(&["ruler", text(&a), "0", "0", "1", "1"]);
    assert_eq!(code, Some(0), "{e}");
    assert!(e.contains("a macro aperture is not drawn"), "{e}");
    // rectangular pads are not in the narrowest track nor the smallest gap, and that is said
    let sample = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");
    let (code, _, e) = run(&["measure", sample]);
    assert_eq!(code, Some(0), "{e}");
    assert!(e.contains("4 rectangular pads"), "{e}");
}
