//! `render` writes the picture whole and never over a layer.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");

fn run(args: &[&str]) -> (bool, String) {
    let out = Command::new(BIN).args(args).output().expect("it runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_picture_is_never_written_over_a_layer_given_by_another_path() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let layer = format!("{dir}/atomic_layer.txt");
    std::fs::copy(
        format!("{}/examples/sample.gtl", env!("CARGO_MANIFEST_DIR")),
        &layer,
    )
    .unwrap();
    let before = std::fs::read(&layer).unwrap();
    let other_way = format!("{dir}/../{}/atomic_layer.txt", {
        std::path::Path::new(dir)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    });
    let (ok, err) = run(&["render", &other_way, &layer]);
    assert!(!ok);
    assert!(err.contains("layer"), "{err}");
    assert_eq!(std::fs::read(&layer).unwrap(), before);
}

#[test]
fn a_picture_is_written_whole_and_no_temporary_file_stays() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let svg = format!("{dir}/atomic_picture.svg");
    let sample = format!("{}/examples/sample.gtl", env!("CARGO_MANIFEST_DIR"));
    let (ok, err) = run(&["render", &svg, &sample]);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert!(text.trim_end().ends_with("</svg>"), "{text}");
    assert!(!std::path::Path::new(&format!("{svg}.tmp")).exists());
    std::fs::remove_file(&svg).unwrap();
}
