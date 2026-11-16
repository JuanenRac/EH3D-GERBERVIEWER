//! `render` and `check` say what a layer does not draw exactly, and `--exact` refuses to go on.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");

fn clear_layer(name: &str) -> String {
    let path = format!("{}/{name}.gtl", env!("CARGO_TARGET_TMPDIR"));
    std::fs::write(
        &path,
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.4*%\n%LPC*%\nD10*\nX00000Y00000D02*\nX400000Y00000D01*\nM02*\n",
    )
    .unwrap();
    path
}

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(BIN).args(args).output().expect("it runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn render_says_what_is_not_drawn_exactly_and_exact_refuses() {
    let layer = clear_layer("exact_render");
    let svg = format!("{}/exact_render.svg", env!("CARGO_TARGET_TMPDIR"));
    let _ = std::fs::remove_file(&svg);
    let (ok, err) = run(&["render", &svg, &layer]);
    assert!(ok, "{err}");
    assert!(err.contains("clear polarity is drawn as dark"), "{err}");
    std::fs::remove_file(&svg).unwrap();
    let (ok, err) = run(&["render", &svg, &layer, "--exact"]);
    assert!(!ok);
    assert!(err.contains("not drawn exactly"), "{err}");
    assert!(!std::path::Path::new(&svg).exists());
}

#[test]
fn check_says_what_is_not_drawn_exactly_and_exact_refuses() {
    let layer = clear_layer("exact_check");
    let (ok, err) = run(&["check", &layer, &layer]);
    assert!(ok, "{err}");
    assert!(err.contains("clear polarity is drawn as dark"), "{err}");
    let (ok, err) = run(&["check", &layer, &layer, "--exact"]);
    assert!(!ok);
    assert!(err.contains("cannot be trusted"), "{err}");
}
