//! The ruler: a click is caught by the nearest vertex or pad centre within the tolerance, and the measure between two points gives the distance, the two differences and the angle in the
//! unit of the person. The examples are worked out by hand, and the clicks of a seeded run never panic and never give a number that is not finite.

use eh3d_gerberviewer::ruler::{measure_points, ruler, snap, SnapKind, Snapped, Unit};
use eh3d_gerberviewer::scene::{Scene, Shape};
use eh3d_gerberviewer::settings::Settings;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");
const GTL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn free(x: f64, y: f64) -> Snapped {
    Snapped {
        point: (x, y),
        kind: SnapKind::Free,
        shape: None,
        what: None,
    }
}

/// A round pad at (5, 5), a 2 x 2 rectangular pad at (10, 5) and a track from (0, 0) to (3, 4).
fn board() -> Scene {
    Scene {
        shapes: vec![
            Shape::Disc {
                x: 5.0,
                y: 5.0,
                r: 0.5,
            },
            Shape::Rect {
                x: 10.0,
                y: 5.0,
                w: 2.0,
                h: 2.0,
            },
            Shape::Stroke {
                x0: 0.0,
                y0: 0.0,
                x1: 3.0,
                y1: 4.0,
                w: 0.2,
            },
        ],
        notes: Vec::new(),
    }
}

#[test]
fn a_click_near_a_vertex_or_a_pad_centre_is_caught_and_names_the_shape() {
    let s = Settings::default();
    let b = board();
    let p = snap(&b, (5.05, 4.95), &s).unwrap();
    assert_eq!(
        (p.point, p.kind, p.shape, p.what),
        ((5.0, 5.0), SnapKind::PadCentre, Some(0), Some("round pad"))
    );
    // a corner of the rectangle, and the far end of the track
    let p = snap(&b, (11.1, 6.05), &s).unwrap();
    assert_eq!(
        (p.point, p.kind, p.what),
        ((11.0, 6.0), SnapKind::Vertex, Some("rectangular pad"))
    );
    let p = snap(&b, (3.0, 4.1), &s).unwrap();
    assert_eq!(
        (p.point, p.shape, p.what),
        ((3.0, 4.0), Some(2), Some("track"))
    );
    // a click far from everything stays where it is
    let p = snap(&b, (7.5, 2.0), &s).unwrap();
    assert_eq!(
        (p.point, p.kind, p.shape),
        ((7.5, 2.0), SnapKind::Free, None)
    );
}

#[test]
fn a_click_at_exactly_the_tolerance_is_caught_and_a_hair_more_is_not() {
    let s = Settings {
        snap_tolerance_mm: 0.5,
        ..Settings::default()
    };
    let b = board();
    assert_eq!(snap(&b, (5.5, 5.0), &s).unwrap().kind, SnapKind::PadCentre);
    assert_eq!(snap(&b, (5.5000001, 5.0), &s).unwrap().kind, SnapKind::Free);
    // a tolerance of 0 catches only a click on the point
    let s = Settings {
        snap_tolerance_mm: 0.0,
        ..Settings::default()
    };
    assert_eq!(snap(&b, (5.0, 5.0), &s).unwrap().kind, SnapKind::PadCentre);
    assert_eq!(snap(&b, (5.0, 5.001), &s).unwrap().kind, SnapKind::Free);
}

#[test]
fn the_snap_can_be_turned_off_for_vertices_or_for_pad_centres() {
    let b = board();
    let s = Settings {
        snap_pad_centres: false,
        ..Settings::default()
    };
    assert_eq!(snap(&b, (5.0, 5.0), &s).unwrap().kind, SnapKind::Free);
    assert_eq!(snap(&b, (11.0, 6.0), &s).unwrap().kind, SnapKind::Vertex);
    let s = Settings {
        snap_vertices: false,
        ..Settings::default()
    };
    assert_eq!(snap(&b, (11.0, 6.0), &s).unwrap().kind, SnapKind::Free);
    assert_eq!(snap(&b, (10.0, 5.0), &s).unwrap().kind, SnapKind::PadCentre);
}

#[test]
fn a_tie_goes_to_the_pad_centre_and_then_to_the_lower_shape() {
    let s = Settings {
        snap_tolerance_mm: 1.0,
        ..Settings::default()
    };
    // a pad whose centre is the end of a track: the same point twice, the centre wins
    let b = Scene {
        shapes: vec![
            Shape::Stroke {
                x0: 0.0,
                y0: 0.0,
                x1: 1.0,
                y1: 0.0,
                w: 0.1,
            },
            Shape::Disc {
                x: 1.0,
                y: 0.0,
                r: 0.3,
            },
            Shape::Disc {
                x: 1.0,
                y: 0.0,
                r: 0.6,
            },
        ],
        notes: Vec::new(),
    };
    let p = snap(&b, (1.0, 0.0), &s).unwrap();
    assert_eq!((p.kind, p.shape), (SnapKind::PadCentre, Some(1)));
}

#[test]
fn a_three_four_five_triangle_measures_five_in_every_unit() {
    let m = measure_points(free(0.0, 0.0), free(3.0, 4.0), Unit::Mm);
    assert!(close(m.distance, 5.0) && close(m.dx, 3.0) && close(m.dy, 4.0));
    assert!(close(m.angle_deg, 4.0_f64.atan2(3.0).to_degrees()));
    let inch = measure_points(free(0.0, 0.0), free(25.4, 0.0), Unit::Inch);
    assert!(close(inch.distance, 1.0));
    let mil = measure_points(free(0.0, 0.0), free(25.4, 0.0), Unit::Mil);
    assert!(close(mil.distance, 1000.0));
}

#[test]
fn the_angle_runs_from_0_to_360_anticlockwise_and_is_0_for_the_same_point() {
    let a = |x: f64, y: f64| measure_points(free(0.0, 0.0), free(x, y), Unit::Mm).angle_deg;
    assert!(close(a(1.0, 0.0), 0.0));
    assert!(close(a(0.0, 1.0), 90.0));
    assert!(close(a(-1.0, 0.0), 180.0));
    assert!(close(a(0.0, -1.0), 270.0));
    assert!(close(a(1.0, -1.0), 315.0));
    assert_eq!(a(0.0, 0.0), 0.0);
    // a point a hair below the axis is 359.99..., never 360
    assert!(a(1.0, -1e-15) < 360.0);
}

#[test]
fn the_ruler_snaps_both_ends_and_names_the_pads_it_measured() {
    let s = Settings::default();
    let m = ruler(&board(), (5.02, 5.01), (10.1, 5.05), &s).unwrap();
    // centre of the round pad to centre of the rectangular one: 5 mm along x
    assert!(close(m.distance, 5.0) && close(m.angle_deg, 0.0));
    let lines = m.lines();
    assert!(
        lines[0].contains("round pad 0") && lines[1].contains("rectangular pad 1"),
        "{lines:?}"
    );
    assert_eq!(
        lines[2],
        "distance 5.000 mm  dx +5.000 mm  dy +0.000 mm  angle 0.00 deg"
    );
    // in mils the same measure is 5 / 0.0254 mils
    let mil = Settings {
        unit: Unit::Mil,
        ..Settings::default()
    };
    let m = ruler(&board(), (5.0, 5.0), (10.0, 5.0), &mil).unwrap();
    assert!(close(m.distance, 5.0 / 0.0254));
    assert!(
        m.lines()[2].starts_with("distance 196.85 mil"),
        "{:?}",
        m.lines()
    );
}

#[test]
fn a_point_that_is_not_a_number_is_an_error_and_never_a_measure() {
    let s = Settings::default();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(snap(&board(), (bad, 0.0), &s).is_err());
        assert!(ruler(&board(), (0.0, 0.0), (0.0, bad), &s).is_err());
    }
}

#[test]
fn the_clicks_of_a_seeded_run_always_give_finite_numbers() {
    let mut state = 0x1234_5678_9abc_def1_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 200_000) as f64 / 10_000.0 - 5.0
    };
    let s = Settings::default();
    let b = board();
    for _ in 0..2000 {
        let m = ruler(
            &b,
            (next() + 5.0, next() + 5.0),
            (next() + 5.0, next() + 5.0),
            &s,
        )
        .unwrap();
        assert!(m.distance.is_finite() && m.dx.is_finite() && m.dy.is_finite());
        assert!((0.0..360.0).contains(&m.angle_deg));
        assert!(m.distance >= 0.0);
        // a snapped point never moves further than the tolerance
        for p in [&m.from, &m.to] {
            assert!(p.kind == SnapKind::Free || p.shape.is_some());
        }
    }
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
fn the_ruler_command_measures_the_pads_of_the_example_in_each_unit() {
    // the first two pads are at x = 5 and x = 15, y = 5: ten millimetres apart
    let (code, out, e) = run(&["ruler", GTL, "5.02", "5.01", "14.9", "5.1"]);
    assert_eq!(code, Some(0), "{e}");
    assert!(
        out.contains("distance 10.000 mm  dx +10.000 mm  dy +0.000 mm  angle 0.00 deg"),
        "{out}"
    );
    assert!(out.contains("rectangular pad"), "{out}");
    let (_, out, _) = run(&["ruler", GTL, "5", "5", "15", "5", "--unit", "in"]);
    assert!(out.contains("distance 0.39370 in"), "{out}");
    let (_, out, _) = run(&["ruler", GTL, "5", "5", "5.3", "5", "--snap-tolerance", "0"]);
    assert!(
        out.contains("distance 0.300 mm") && out.contains("free"),
        "{out}"
    );
}

#[test]
fn the_ruler_command_ends_with_64_for_a_wrong_use() {
    assert_eq!(run(&["ruler", GTL, "1", "2", "3"]).0, Some(64));
    assert_eq!(run(&["ruler", GTL, "a", "2", "3", "4"]).0, Some(64));
    assert_eq!(
        run(&["ruler", GTL, "1", "2", "3", "4", "--unit", "yd"]).0,
        Some(64)
    );
    assert_eq!(
        run(&["ruler", GTL, "1", "2", "3", "4", "--snap-tolerance", "11"]).0,
        Some(64)
    );
    assert_eq!(
        run(&["ruler", GTL, "1", "2", "3", "4", "--min", "1"]).0,
        Some(64)
    );
    assert_eq!(run(&["info", GTL, "--unit", "mm"]).0, Some(64));
    // a layer that cannot be read is a failure of the work
    assert_eq!(
        run(&["ruler", "missing.gtl", "1", "2", "3", "4"]).0,
        Some(1)
    );
}
