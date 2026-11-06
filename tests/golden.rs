//! Golden outputs: the commands run on the two files of `examples/` and print what was worked out by hand from those files.
//!
//! `sample.gtl` is a 0.4 mm track round a 40 x 25 mm board (four straight tracks) and four square pads of 1.6 mm at x = 5, 15, 25 and 35, y = 5; the box of the copper is the
//! track of the outline grown by half its width on every side: from -0.2 to 40.2 in x (40.4 mm) and from -0.2 to 25.2 in y (25.4 mm), and the eight shapes are the four tracks and the
//! four pads. `sample.ncd` is four holes of 0.8 mm on the same x and y = 5: from 4.6 to 35.4 in x (30.8 mm) and from 4.6 to 5.4 in y (0.8 mm).

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");
const GTL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");
const NCD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.ncd");

fn run(args: &[&str]) -> (bool, Vec<String>, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn info_gives_the_shapes_and_the_box_of_each_example_file() {
    let (ok, lines, e) = run(&["info", GTL, NCD]);
    assert!(ok, "{e}");
    assert_eq!(
        lines,
        [
            "sample.gtl: 8 shapes, 40.400 x 25.400 mm",
            "sample.ncd: 4 shapes, 30.800 x 0.800 mm"
        ]
    );
}

#[test]
fn measure_gives_the_narrowest_track_and_the_pads_of_the_board() {
    let (ok, lines, e) = run(&["measure", GTL]);
    assert!(ok, "{e}");
    assert_eq!(
        lines.first().map(String::as_str),
        Some("narrowest track: 0.400 mm")
    );
    assert_eq!(lines.last().map(String::as_str), Some("4 pads"));
}

#[test]
fn check_finds_the_four_holes_inside_the_copper_of_the_board() {
    // one Gerber layer is nothing to compare with, and the holes (0.4 mm of radius at y = 5) are well inside the box of the copper
    let (ok, lines, e) = run(&["check", GTL, NCD]);
    assert!(ok, "{e}");
    assert_eq!(
        lines,
        ["there are no layers to compare (fewer than two Gerber layers); the holes are inside the copper"]
    );
}
