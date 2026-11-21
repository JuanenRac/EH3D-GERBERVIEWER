//! The gap between objects of every kind, against distances worked out by hand and by formula, with a fixed seed. A gap is 0 when two objects touch or overlap, and it never depends on
//! the order of the two or on where the pair is on the board.

use eh3d_gerberviewer::gap::{layer_gap, scene_gap, shape_gap, Prim};
use eh3d_gerberviewer::scene::{Scene, Shape};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");
const GTL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.gtl");
const NCD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/sample.ncd");

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// A number from `lo` to `hi`, in steps of a hundredth so that the sums stay exact enough.
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next() % 10_000) as f64 / 10_000.0 * (hi - lo)
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Shape {
    Shape::Rect { x, y, w, h }
}

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn random_shape(rng: &mut Rng) -> Shape {
    let (x, y) = (rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
    match rng.next() % 5 {
        0 => disc(x, y, rng.range(0.1, 2.0)),
        1 => rect(x, y, rng.range(0.1, 4.0), rng.range(0.1, 4.0)),
        2 => Shape::Obround {
            x,
            y,
            w: rng.range(0.1, 4.0),
            h: rng.range(0.1, 4.0),
        },
        3 => Shape::Stroke {
            x0: x,
            y0: y,
            x1: x + rng.range(-5.0, 5.0),
            y1: y + rng.range(-5.0, 5.0),
            w: rng.range(0.1, 1.0),
        },
        _ => Shape::Region {
            pts: vec![
                (x, y),
                (x + rng.range(0.5, 4.0), y),
                (x + rng.range(0.5, 4.0), y + rng.range(0.5, 4.0)),
                (x, y + rng.range(0.5, 4.0)),
            ],
        },
    }
}

fn moved(s: &Shape, dx: f64, dy: f64) -> Shape {
    match s {
        Shape::Disc { x, y, r } => disc(x + dx, y + dy, *r),
        Shape::Rect { x, y, w, h } => rect(x + dx, y + dy, *w, *h),
        Shape::Obround { x, y, w, h } => Shape::Obround {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
        },
        Shape::Stroke { x0, y0, x1, y1, w } => Shape::Stroke {
            x0: x0 + dx,
            y0: y0 + dy,
            x1: x1 + dx,
            y1: y1 + dy,
            w: *w,
        },
        Shape::Region { pts } => Shape::Region {
            pts: pts.iter().map(|(x, y)| (x + dx, y + dy)).collect(),
        },
    }
}

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

#[test]
fn two_rectangles_are_apart_by_the_distance_of_their_sides_or_corners() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..500 {
        let (ax, ay, aw, ah) = (
            rng.range(-5.0, 5.0),
            rng.range(-5.0, 5.0),
            rng.range(0.2, 3.0),
            rng.range(0.2, 3.0),
        );
        let (bx, by, bw, bh) = (
            rng.range(-8.0, 8.0),
            rng.range(-8.0, 8.0),
            rng.range(0.2, 3.0),
            rng.range(0.2, 3.0),
        );
        let dx = ((ax - bx).abs() - (aw + bw) / 2.0).max(0.0);
        let dy = ((ay - by).abs() - (ah + bh) / 2.0).max(0.0);
        let got = shape_gap(&rect(ax, ay, aw, ah), &rect(bx, by, bw, bh)).unwrap();
        assert!(close(got, dx.hypot(dy)), "{got} against {}", dx.hypot(dy));
    }
}

#[test]
fn two_round_pads_are_apart_by_the_distance_of_their_centres_less_the_radii() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for _ in 0..500 {
        let (a, b) = (
            disc(
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
                rng.range(0.1, 2.0),
            ),
            disc(
                rng.range(-5.0, 5.0),
                rng.range(-5.0, 5.0),
                rng.range(0.1, 2.0),
            ),
        );
        let (
            Shape::Disc {
                x: x0,
                y: y0,
                r: r0,
            },
            Shape::Disc {
                x: x1,
                y: y1,
                r: r1,
            },
        ) = (&a, &b)
        else {
            unreachable!()
        };
        let want = ((x0 - x1).hypot(y0 - y1) - r0 - r1).max(0.0);
        assert!(close(shape_gap(&a, &b).unwrap(), want));
    }
}

#[test]
fn a_region_is_measured_like_the_rectangle_it_draws() {
    let mut rng = Rng(77);
    for _ in 0..300 {
        let (x, y, w, h) = (
            rng.range(-5.0, 5.0),
            rng.range(-5.0, 5.0),
            rng.range(0.5, 4.0),
            rng.range(0.5, 4.0),
        );
        let region = Shape::Region {
            pts: vec![
                (x - w / 2.0, y - h / 2.0),
                (x + w / 2.0, y - h / 2.0),
                (x + w / 2.0, y + h / 2.0),
                (x - w / 2.0, y + h / 2.0),
            ],
        };
        let other = random_shape(&mut rng);
        let (a, b) = (
            shape_gap(&region, &other).unwrap(),
            shape_gap(&rect(x, y, w, h), &other).unwrap(),
        );
        assert!(close(a, b), "{a} against {b} for {other:?}");
    }
}

#[test]
fn the_gap_to_a_triangle_is_the_distance_to_its_slanted_side() {
    // the sides (0,0)-(4,0)-(0,3): the slanted side is the line 3x + 4y = 12, and the point (4,3) is 12/5 from it
    let triangle = Shape::Region {
        pts: vec![(0.0, 0.0), (4.0, 0.0), (0.0, 3.0)],
    };
    assert!(close(
        shape_gap(&triangle, &disc(4.0, 3.0, 0.0)).unwrap(),
        2.4
    ));
    assert!(close(
        shape_gap(&triangle, &disc(4.0, 3.0, 0.4)).unwrap(),
        2.0
    ));
    // a point inside, and one on the side, touch
    assert_eq!(shape_gap(&triangle, &disc(1.0, 1.0, 0.0)).unwrap(), 0.0);
    assert_eq!(shape_gap(&triangle, &disc(2.0, 0.0, 0.0)).unwrap(), 0.0);
}

#[test]
fn an_obround_pad_is_a_segment_with_the_radius_of_its_short_side() {
    // 4 x 2 at the origin: the segment from -1 to 1 on x and the radius 1; a disc of 0.5 at x = 5 is 4 - 1 - 0.5 away
    let o = Shape::Obround {
        x: 0.0,
        y: 0.0,
        w: 4.0,
        h: 2.0,
    };
    assert!(close(shape_gap(&o, &disc(5.0, 0.0, 0.5)).unwrap(), 2.5));
    // the same standing up
    let tall = Shape::Obround {
        x: 0.0,
        y: 0.0,
        w: 2.0,
        h: 4.0,
    };
    assert!(close(shape_gap(&tall, &disc(0.0, 5.0, 0.5)).unwrap(), 2.5));
}

#[test]
fn objects_that_touch_cross_or_hold_one_another_have_no_gap() {
    let big = rect(0.0, 0.0, 10.0, 10.0);
    // a pad inside a bigger one, and one that shares a side
    assert_eq!(shape_gap(&big, &rect(1.0, 1.0, 1.0, 1.0)).unwrap(), 0.0);
    assert_eq!(shape_gap(&big, &rect(10.0, 0.0, 10.0, 10.0)).unwrap(), 0.0);
    // two tracks that cross like an X
    let a = Shape::Stroke {
        x0: 0.0,
        y0: 0.0,
        x1: 4.0,
        y1: 4.0,
        w: 0.1,
    };
    let b = Shape::Stroke {
        x0: 0.0,
        y0: 4.0,
        x1: 4.0,
        y1: 0.0,
        w: 0.1,
    };
    assert_eq!(shape_gap(&a, &b).unwrap(), 0.0);
    // a track across a region without an end inside it
    let region = Shape::Region {
        pts: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
    };
    let across = Shape::Stroke {
        x0: -2.0,
        y0: 2.0,
        x1: 6.0,
        y1: 2.0,
        w: 0.2,
    };
    assert_eq!(shape_gap(&region, &across).unwrap(), 0.0);
    // a region that holds another
    let small = Shape::Region {
        pts: vec![(1.0, 1.0), (2.0, 1.0), (2.0, 2.0)],
    };
    assert_eq!(shape_gap(&region, &small).unwrap(), 0.0);
}

#[test]
fn the_gap_does_not_depend_on_the_order_or_on_where_the_pair_is() {
    let mut rng = Rng(0xdead_beef_cafe_f00d);
    for _ in 0..600 {
        let (a, b) = (random_shape(&mut rng), random_shape(&mut rng));
        let (ab, ba) = (shape_gap(&a, &b).unwrap(), shape_gap(&b, &a).unwrap());
        assert!(close(ab, ba), "{a:?} {b:?}: {ab} against {ba}");
        let (dx, dy) = (rng.range(-1000.0, 1000.0), rng.range(-1000.0, 1000.0));
        let far = shape_gap(&moved(&a, dx, dy), &moved(&b, dx, dy)).unwrap();
        assert!((ab - far).abs() < 1e-6, "{a:?} {b:?}: {ab} against {far}");
        assert!(ab >= 0.0 && ab.is_finite());
    }
}

#[test]
fn a_point_inside_an_object_is_found_and_one_outside_is_not() {
    let rounded = Prim::of(&Shape::Stroke {
        x0: 0.0,
        y0: 0.0,
        x1: 4.0,
        y1: 0.0,
        w: 2.0,
    })
    .unwrap();
    assert!(rounded.contains((2.0, 1.0)));
    assert!(rounded.contains((-1.0, 0.0)));
    assert!(!rounded.contains((-0.8, 0.8)));
}

#[test]
fn the_closest_pair_of_two_layers_names_its_shapes_and_their_kinds() {
    let a = scene(vec![rect(0.0, 0.0, 2.0, 2.0), disc(20.0, 0.0, 1.0)]);
    let b = scene(vec![
        Shape::Region {
            pts: vec![(30.0, 0.0), (32.0, 0.0), (32.0, 2.0)],
        },
        Shape::Stroke {
            x0: 5.0,
            y0: -3.0,
            x1: 5.0,
            y1: 3.0,
            w: 1.0,
        },
    ]);
    let report = layer_gap(&a, &b, 100).unwrap();
    let g = report.best.unwrap();
    // the rectangle ends at x = 1 and the track starts at x = 4.5
    assert!(close(g.distance, 3.5));
    assert_eq!(
        (g.a_index, g.b_index, g.a_kind, g.b_kind),
        (0, 1, "rectangular pad", "track")
    );
    assert_eq!(report.skipped, 0);
}

#[test]
fn shapes_that_are_not_valid_are_counted_and_too_many_pairs_are_refused() {
    let a = scene(vec![
        disc(0.0, 0.0, 1.0),
        disc(f64::NAN, 0.0, 1.0),
        Shape::Region {
            pts: vec![(0.0, 0.0), (1.0, 1.0)],
        },
        rect(0.0, 0.0, -1.0, 1.0),
    ]);
    let b = scene(vec![disc(10.0, 0.0, 1.0)]);
    let report = layer_gap(&a, &b, 100).unwrap();
    assert_eq!(report.skipped, 3);
    assert!(close(report.best.unwrap().distance, 8.0));
    let e = layer_gap(&a, &b, 3).unwrap_err();
    assert!(e.contains("4 pairs") && e.contains("max_gap_pairs"), "{e}");
    // a layer with nothing valid has no pair
    assert_eq!(layer_gap(&scene(vec![]), &b, 10).unwrap().best, None);
    // the number of pairs cannot overflow
    assert!(layer_gap(&a, &b, usize::MAX).is_ok());
}

#[test]
fn a_layer_of_the_example_has_a_pad_four_millimetres_from_the_track_round_the_board() {
    // the pads (1.6 square at y = 5) are 5 - 0.8 - 0.2 = 4 from the track below and 5 - 0.8 - 0.2 from the one on the left; the tracks themselves touch at the corners
    let text = std::fs::read_to_string(GTL).unwrap();
    let file = eh3d_parser_gerber::parse(&text);
    let s = eh3d_gerberviewer::scene::from_gerber(&file);
    let pads_only = scene(
        s.shapes
            .iter()
            .filter(|x| matches!(x, Shape::Rect { .. }))
            .cloned()
            .collect(),
    );
    let tracks_only = scene(
        s.shapes
            .iter()
            .filter(|x| matches!(x, Shape::Stroke { .. }))
            .cloned()
            .collect(),
    );
    let g = layer_gap(&pads_only, &tracks_only, 100)
        .unwrap()
        .best
        .unwrap();
    assert!(close(g.distance, 4.0), "{}", g.distance);
    // inside one layer: the tracks touch each other at the corners, so the smallest gap there is 0
    assert_eq!(scene_gap(&s, 100).unwrap().best.unwrap().distance, 0.0);
    // among the pads alone it is the 10 mm pitch less the 1.6 mm of the side
    let g = scene_gap(&pads_only, 100).unwrap().best.unwrap();
    assert!(close(g.distance, 8.4), "{}", g.distance);
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
fn the_gap_command_names_the_pair_and_ends_with_2_below_the_minimum() {
    // the holes lie inside the pads, so the layers touch
    let (code, out, e) = run(&["gap", GTL, NCD]);
    assert_eq!(code, Some(0), "{e}");
    assert!(
        out.contains("smallest gap: 0.000 mm") && out.contains("touch or overlap"),
        "{out}"
    );
    let (code, _, e) = run(&["gap", GTL, NCD, "--min", "0.1"]);
    assert_eq!(code, Some(2), "{e}");
    assert!(e.contains("verification failed"), "{e}");
    // 0 as the minimum is met by any gap
    assert_eq!(run(&["gap", GTL, NCD, "--min", "0"]).0, Some(0));
    // a wrong use is 64
    assert_eq!(run(&["gap", GTL]).0, Some(64));
    assert_eq!(run(&["gap", GTL, NCD, "--unit", "mil"]).0, Some(64));
    assert_eq!(run(&["gap", GTL, NCD, "--min", "x"]).0, Some(64));
}
