//! The configuration file: every value has a default and a range, the sample file says the defaults, and a file that is damaged is an error that names its line, never a panic.

use eh3d_gerberviewer::ruler::Unit;
use eh3d_gerberviewer::settings::{Settings, MAX_CFG_BYTES};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");
const CFG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/gerberview.cfg");
const GTL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");

fn sample() -> String {
    std::fs::read_to_string(CFG).unwrap().replace("\r\n", "\n")
}

#[test]
fn the_sample_file_says_every_default_and_the_empty_text_gives_them() {
    assert_eq!(Settings::parse(&sample()).unwrap(), Settings::default());
    assert_eq!(Settings::parse("").unwrap(), Settings::default());
    assert_eq!(
        Settings::parse("# only a comment\n\n   \n").unwrap(),
        Settings::default()
    );
    // every key of the settings is in the sample file
    for key in [
        "snap_tolerance_mm",
        "snap_vertices",
        "snap_pad_centres",
        "unit",
        "min_visible_opacity",
        "default_opacity",
        "max_gap_pairs",
        "max_layers",
    ] {
        assert!(sample().contains(&format!("\n{key} = ")), "{key}");
    }
}

#[test]
fn a_value_is_read_with_spaces_comments_and_a_byte_order_mark() {
    let s = Settings::parse("\u{feff}  unit =  MIL  # thousandths\nsnap_tolerance_mm=0.5\nsnap_vertices = no\nmax_layers = 8").unwrap();
    assert_eq!(s.unit, Unit::Mil);
    assert_eq!(s.snap_tolerance_mm, 0.5);
    assert!(!s.snap_vertices && s.snap_pad_centres);
    assert_eq!(s.max_layers, 8);
    // a text with Windows line ends reads the same
    let crlf = "unit = in\r\nmax_layers = 3\r\n";
    assert_eq!(Settings::parse(crlf).unwrap().unit, Unit::Inch);
}

#[test]
fn a_value_out_of_its_range_an_unknown_key_or_a_twice_written_key_names_its_line() {
    let cases = [
        (
            "\n\nsnap_tolerance_mm = 11",
            "line 3",
            "must be from 0 to 10",
        ),
        ("snap_tolerance_mm = -1", "line 1", "must be from"),
        ("default_opacity = 1.5", "line 1", "must be from 0 to 1"),
        ("min_visible_opacity = nan", "line 1", "is not a number"),
        ("min_visible_opacity = inf", "line 1", "is not a number"),
        ("max_layers = 0", "line 1", "must be from 1 to 1024"),
        ("max_layers = 2.5", "line 1", "is not a whole number"),
        (
            "max_gap_pairs = 100000001",
            "line 1",
            "must be from 1 to 100000000",
        ),
        ("unit = yd", "line 1", "must be mm, in or mil"),
        ("snap_vertices = maybe", "line 1", "must be yes or no"),
        ("colour = red", "line 1", "unknown setting colour"),
        ("just words", "line 1", "is not key = value"),
        ("unit = mm\nunit = in", "line 2", "written twice"),
    ];
    for (text, line, what) in cases {
        let e = Settings::parse(text).unwrap_err();
        assert!(e.contains(line) && e.contains(what), "{text:?}: {e}");
    }
}

#[test]
fn no_limit_of_the_settings_is_above_what_a_32_bit_machine_counts() {
    // the most that a count may be is far below the range of a 32-bit usize
    let s = Settings::parse("max_gap_pairs = 100000000\nmax_layers = 1024").unwrap();
    assert!(s.max_gap_pairs <= 100_000_000 && s.max_layers <= 1024);
}

#[test]
fn a_configuration_file_is_read_with_a_size_limit() {
    let dir = std::env::temp_dir().join(format!("gbv_cfg_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ok = dir.join("ok.cfg");
    std::fs::write(&ok, "unit = in\n").unwrap();
    assert_eq!(Settings::load(&ok).unwrap().unit, Unit::Inch);
    let big = dir.join("big.cfg");
    std::fs::write(&big, "#".repeat(MAX_CFG_BYTES + 1)).unwrap();
    let e = Settings::load(&big).unwrap_err();
    assert!(e.contains("above") && e.contains("big.cfg"), "{e}");
    let e = Settings::load(&dir.join("missing.cfg")).unwrap_err();
    assert!(e.contains("missing.cfg"), "{e}");
    let bad = dir.join("bad.cfg");
    std::fs::write(&bad, "unit = yd\n").unwrap();
    assert!(Settings::load(&bad)
        .unwrap_err()
        .contains("bad.cfg: line 1"));
    let _ = std::fs::remove_dir_all(&dir);
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn damaged_copies_of_the_sample_file_are_read_without_a_panic() {
    let base = sample().into_bytes();
    let mut rng = Rng(0x5eed_cafe_1234_5678);
    let mut accepted = 0;
    for round in 0..600 {
        let mut b = base.clone();
        for _ in 0..(1 + rng.next() % 6) {
            let at = (rng.next() as usize) % b.len().max(1);
            match rng.next() % 4 {
                0 => b[at] = (rng.next() % 256) as u8,
                1 => b.insert(at, b"=# \n.-9eE"[(rng.next() % 9) as usize]),
                2 => b.truncate(at),
                _ => {
                    b.remove(at);
                }
            }
            if b.is_empty() {
                break;
            }
        }
        let text = String::from_utf8_lossy(&b);
        if let Ok(s) = Settings::parse(&text) {
            accepted += 1;
            // whatever is accepted is inside its ranges
            assert!((0.0..=10.0).contains(&s.snap_tolerance_mm), "{round}");
            assert!((0.0..=1.0).contains(&s.default_opacity));
            assert!((0.0..=1.0).contains(&s.min_visible_opacity));
            assert!((1..=1024).contains(&s.max_layers));
            assert!((1..=100_000_000).contains(&s.max_gap_pairs));
        }
    }
    assert!(accepted > 0);
    // every prefix of the file, and bytes that are not text
    for n in 0..base.len() {
        let _ = Settings::parse(&String::from_utf8_lossy(&base[..n]));
    }
    let noise: Vec<u8> = (0..4096).map(|_| (rng.next() % 256) as u8).collect();
    let _ = Settings::parse(&String::from_utf8_lossy(&noise));
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(BIN)
        .args(args)
        .output()
        .expect("the program runs");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn the_config_option_of_the_commands_reads_the_file_and_refuses_a_bad_one() {
    let dir = std::env::temp_dir().join(format!("gbv_cfgcli_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mils = dir.join("mil.cfg");
    std::fs::write(&mils, "unit = mil\n").unwrap();
    let mils = mils.to_string_lossy().into_owned();
    let (code, out, e) = run(&["ruler", GTL, "5", "5", "15", "5", "--config", &mils]);
    assert_eq!(code, Some(0), "{e}");
    assert!(out.contains("distance 393.70 mil"), "{out}");
    // the option of the command line wins over the file
    let (_, out, _) = run(&[
        "ruler", GTL, "5", "5", "15", "5", "--config", &mils, "--unit", "mm",
    ]);
    assert!(out.contains("distance 10.000 mm"), "{out}");
    let bad = dir.join("bad.cfg");
    std::fs::write(&bad, "colour = red\n").unwrap();
    let bad = bad.to_string_lossy().into_owned();
    let (code, _, e) = run(&["ruler", GTL, "5", "5", "15", "5", "--config", &bad]);
    assert_eq!(code, Some(64), "{e}");
    assert!(e.contains("unknown setting"), "{e}");
    // the sample file of the repository is accepted by the commands
    assert_eq!(run(&["gap", GTL, GTL, "--config", CFG]).0, Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}
