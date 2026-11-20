//! The exit codes of the convention of the suite on the command line: 0 for a run that went well, 1 for a failure of the work (a file that is not there), 64 for a wrong use.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");

fn code(args: &[&str]) -> (i32, String) {
    let out = Command::new(BIN)
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the program runs");
    (
        out.status.code().expect("the program ends by itself"),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_run_that_goes_well_ends_with_0() {
    assert_eq!(code(&["info", "examples/sample.gtl"]).0, 0);
    assert_eq!(code(&["help"]).0, 0);
}

#[test]
fn an_unknown_command_or_option_ends_with_64() {
    let (c, e) = code(&["frobnicate"]);
    assert_eq!((c, e.contains("unknown command")), (64, true), "{e}");
    let (c, e) = code(&["info", "--oops", "examples/sample.gtl"]);
    assert_eq!((c, e.contains("unknown option")), (64, true), "{e}");
}

#[test]
fn what_a_command_needs_and_is_missing_ends_with_64() {
    for args in [&["info"][..], &["measure"], &["check"], &["render"]] {
        let (c, e) = code(args);
        assert_eq!(c, 64, "{args:?}: {e}");
    }
}

#[test]
fn a_file_that_cannot_be_read_ends_with_1() {
    let (c, e) = code(&["info", "examples/there_is_no_such_file.gtl"]);
    assert_eq!(c, 1, "{e}");
}

#[test]
fn an_option_of_another_command_ends_with_64_and_names_both() {
    for (args, option, command) in [
        (
            vec!["info", "examples/sample.gtl", "--exact"],
            "--exact",
            "info",
        ),
        (
            vec!["measure", "examples/sample.gtl", "--opacity", "0.5"],
            "--opacity",
            "measure",
        ),
        (
            vec!["check", "examples/sample.gtl", "--margin", "1"],
            "--margin",
            "check",
        ),
        (
            vec![
                "render",
                "out.svg",
                "examples/sample.gtl",
                "--tolerance",
                "1",
            ],
            "--tolerance",
            "render",
        ),
    ] {
        let (c, e) = code(&args);
        assert_eq!(c, 64, "{args:?}: {e}");
        assert!(e.contains(option) && e.contains(command), "{args:?}: {e}");
    }
}

#[test]
fn an_opacity_above_one_ends_with_64_and_one_is_accepted() {
    let (c, e) = code(&["render", "out.svg", "examples/sample.gtl", "--opacity", "7"]);
    assert_eq!(c, 64, "{e}");
    assert!(
        e.contains("--opacity") && e.contains("must be from 0 to 1"),
        "{e}"
    );
    let dir = std::env::temp_dir().join(format!("eh3d_gv_opacity_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let svg = dir.join("one.svg");
    let (c, e) = code(&[
        "render",
        svg.to_str().unwrap(),
        "examples/sample.gtl",
        "--opacity",
        "1",
    ]);
    assert_eq!(c, 0, "{e}");
    std::fs::remove_dir_all(&dir).unwrap();
}
