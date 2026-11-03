//! More tests: the limits of the measures, the comparison of the boxes of the layers, the holes outside the copper and the picture with odd colours, opacities and names.

use eh3d_gerberviewer::align::{compare_boxes, holes_outside};
use eh3d_gerberviewer::measure::{
    centre_distance, distance, min_clearance, min_track_width, pad_centres, MAX_PAIRS_SHAPES,
};
use eh3d_gerberviewer::scene::{Bounds, Scene, Shape};
use eh3d_gerberviewer::svg::{default_color, render, Layer};

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn boxed(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
    Bounds {
        min_x: x0,
        min_y: y0,
        max_x: x1,
        max_y: y1,
    }
}

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn stroke(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Shape {
    Shape::Stroke { x0, y0, x1, y1, w }
}

#[test]
fn the_distance_is_symmetric_zero_for_the_same_point_and_pythagorean() {
    assert_eq!(distance((1.0, 2.0), (1.0, 2.0)), 0.0);
    assert_eq!(distance((0.0, 0.0), (3.0, 4.0)), 5.0);
    assert_eq!(distance((3.0, 4.0), (0.0, 0.0)), 5.0);
    assert_eq!(distance((-1.0, -1.0), (2.0, 3.0)), 5.0);
}

#[test]
fn the_narrowest_track_ignores_pads_and_is_none_without_tracks() {
    assert_eq!(min_track_width(&scene(vec![])), None);
    assert_eq!(min_track_width(&scene(vec![disc(0.0, 0.0, 0.1)])), None);
    let s = scene(vec![
        stroke(0.0, 0.0, 1.0, 0.0, 0.4),
        disc(5.0, 5.0, 0.01),
        stroke(0.0, 1.0, 1.0, 1.0, 0.15),
        stroke(0.0, 2.0, 1.0, 2.0, 0.25),
    ]);
    assert_eq!(min_track_width(&s), Some(0.15));
}

#[test]
fn the_pads_are_the_flashed_shapes_in_the_order_of_the_file_and_tracks_are_not_pads() {
    let s = scene(vec![
        disc(1.0, 1.0, 0.5),
        stroke(0.0, 0.0, 5.0, 0.0, 0.2),
        Shape::Rect {
            x: 2.0,
            y: 3.0,
            w: 1.0,
            h: 1.0,
        },
        Shape::Obround {
            x: 4.0,
            y: 5.0,
            w: 2.0,
            h: 1.0,
        },
        Shape::Region {
            pts: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
        },
    ]);
    assert_eq!(pad_centres(&s), vec![(1.0, 1.0), (2.0, 3.0), (4.0, 5.0)]);
    assert_eq!(centre_distance(&s, 0, 0), Ok(0.0));
    assert!((centre_distance(&s, 0, 2).unwrap() - distance((1.0, 1.0), (4.0, 5.0))).abs() < 1e-12);
    assert_eq!(centre_distance(&s, 2, 0), centre_distance(&s, 0, 2));
}

#[test]
fn a_pad_that_is_not_there_is_an_error_that_says_how_many_there_are() {
    let s = scene(vec![disc(0.0, 0.0, 1.0), disc(5.0, 0.0, 1.0)]);
    let e = centre_distance(&s, 0, 2).unwrap_err();
    assert!(e.contains("pad 2") && e.contains('2'), "{e}");
    let e = centre_distance(&s, 7, 0).unwrap_err();
    assert!(e.contains("pad 7"), "{e}");
    assert!(centre_distance(&scene(vec![]), 0, 0).is_err());
}

#[test]
fn the_clearance_between_two_pads_is_the_gap_and_touching_or_overlapping_is_zero() {
    let apart = scene(vec![disc(0.0, 0.0, 1.0), disc(5.0, 0.0, 1.0)]);
    let (gap, i, j) = min_clearance(&apart).unwrap().unwrap();
    assert!((gap - 3.0).abs() < 1e-9 && (i, j) == (0, 1));
    let touching = scene(vec![disc(0.0, 0.0, 1.0), disc(2.0, 0.0, 1.0)]);
    assert_eq!(min_clearance(&touching).unwrap().unwrap().0, 0.0);
    let overlapping = scene(vec![disc(0.0, 0.0, 1.0), disc(1.0, 0.0, 1.0)]);
    assert_eq!(min_clearance(&overlapping).unwrap().unwrap().0, 0.0);
}

#[test]
fn the_clearance_looks_at_tracks_as_capsules_and_names_the_closest_pair() {
    // a track 0.2 wide at y = 0 and one at y = 1: a gap of 0.8; a pad far away does not win
    let s = scene(vec![
        stroke(0.0, 0.0, 10.0, 0.0, 0.2),
        disc(50.0, 50.0, 0.5),
        stroke(0.0, 1.0, 10.0, 1.0, 0.2),
    ]);
    let (gap, i, j) = min_clearance(&s).unwrap().unwrap();
    assert!((gap - 0.8).abs() < 1e-9, "{gap}");
    assert_eq!((i, j), (0, 2));
    // crossing tracks are 0 apart
    let x = scene(vec![
        stroke(0.0, 0.0, 4.0, 4.0, 0.1),
        stroke(0.0, 4.0, 4.0, 0.0, 0.1),
    ]);
    assert_eq!(min_clearance(&x).unwrap().unwrap().0, 0.0);
}

#[test]
fn a_layer_with_fewer_than_two_round_things_has_no_clearance_and_rectangles_are_not_compared() {
    assert_eq!(min_clearance(&scene(vec![])), Ok(None));
    assert_eq!(min_clearance(&scene(vec![disc(0.0, 0.0, 1.0)])), Ok(None));
    let s = scene(vec![
        Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        Shape::Rect {
            x: 1.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        disc(9.0, 9.0, 1.0),
    ]);
    assert_eq!(min_clearance(&s), Ok(None));
}

#[test]
fn too_many_shapes_to_compare_pair_by_pair_is_an_error_and_not_a_hang() {
    let many: Vec<Shape> = (0..MAX_PAIRS_SHAPES + 1)
        .map(|i| disc(i as f64 * 3.0, 0.0, 1.0))
        .collect();
    let e = min_clearance(&scene(many)).unwrap_err();
    assert!(e.contains(&MAX_PAIRS_SHAPES.to_string()), "{e}");
    // exactly the limit still works
    let just: Vec<Shape> = (0..MAX_PAIRS_SHAPES)
        .map(|i| disc(i as f64 * 3.0, 0.0, 1.0))
        .collect();
    assert!(min_clearance(&scene(just)).is_ok());
}

#[test]
fn the_boxes_of_the_layers_are_compared_with_the_first_one_that_has_something() {
    let layers = vec![
        ("a.gtl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
        ("b.gbl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
        ("c.gko".to_string(), boxed(0.5, 0.0, 10.5, 10.0)),
    ];
    let f = compare_boxes(&layers, 0.1);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].layer, "c.gko");
    assert!(
        f[0].message.contains("+0.500") && f[0].message.contains("a.gtl"),
        "{}",
        f[0].message
    );
    // a bigger tolerance accepts it
    assert!(compare_boxes(&layers, 1.0).is_empty());
    // exactly the tolerance is accepted (only more than that is a finding)
    assert!(compare_boxes(&layers, 0.5).is_empty());
}

#[test]
fn an_empty_layer_is_said_to_have_nothing_and_the_reference_is_the_first_one_with_something() {
    let layers = vec![
        ("empty.gto".to_string(), Bounds::empty()),
        ("a.gtl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
        ("b.gbl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
    ];
    let f = compare_boxes(&layers, 0.1);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].layer, "empty.gto");
    assert!(f[0].message.contains("nothing"));
    // no layers, or only empty ones: nothing to say
    assert!(compare_boxes(&[], 0.1).is_empty());
    assert!(compare_boxes(&[("x".to_string(), Bounds::empty())], 0.1).is_empty());
}

#[test]
fn two_layers_with_the_same_name_are_both_compared() {
    let layers = vec![
        ("same".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
        ("same".to_string(), boxed(5.0, 5.0, 15.0, 15.0)),
    ];
    let f = compare_boxes(&layers, 0.1);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("+5.000"));
}

#[test]
fn a_hole_that_sticks_out_of_the_copper_is_a_finding_and_a_hole_inside_is_not() {
    let copper = scene(vec![
        stroke(0.0, 0.0, 10.0, 10.0, 0.2),
        disc(0.0, 0.0, 0.1),
        disc(10.0, 10.0, 0.1),
    ]);
    let holes = scene(vec![
        disc(5.0, 5.0, 0.4),
        disc(50.0, 5.0, 0.4),
        disc(0.0, 0.0, 0.4),
    ]);
    let f = holes_outside(&copper, &holes, "drill.drl");
    // the one at the corner is outside by its radius, the one far away is outside, the one in the middle is inside
    assert_eq!(f.len(), 2, "{f:?}");
    assert!(f.iter().all(|x| x.layer == "drill.drl"));
    assert!(f.iter().any(|x| x.message.contains("50.000")));
    assert!(f[0].message.contains("0.80 mm"));
}

#[test]
fn every_hole_is_outside_empty_copper_and_things_that_are_not_discs_are_not_holes() {
    let holes = scene(vec![
        disc(1.0, 1.0, 0.3),
        disc(2.0, 2.0, 0.3),
        stroke(0.0, 0.0, 1.0, 1.0, 0.1),
    ]);
    assert_eq!(holes_outside(&scene(vec![]), &holes, "d").len(), 2);
    assert!(holes_outside(&scene(vec![disc(0.0, 0.0, 5.0)]), &scene(vec![]), "d").is_empty());
}

#[test]
fn the_colour_of_a_layer_comes_from_its_extension_in_any_case_and_unknown_ones_are_grey() {
    assert_eq!(default_color("board.GTL"), "#c8a33c");
    assert_eq!(default_color("board.gtl"), default_color("BOARD.GTL"));
    assert_eq!(default_color("x.gbs"), default_color("x.gts"));
    assert_eq!(default_color("x.gbo"), default_color("x.gto"));
    assert_eq!(default_color("x.gm1"), default_color("x.gko"));
    assert_eq!(default_color("x.xln"), "#ffffff");
    assert_eq!(default_color("x.unknown"), "#b0b0b0");
    assert_eq!(default_color("noextension"), "#b0b0b0");
    assert_eq!(default_color(""), "#b0b0b0");
}

fn layer(name: &str, color: &str, opacity: f64, shapes: Vec<Shape>) -> Layer {
    Layer {
        name: name.to_string(),
        scene: scene(shapes),
        color: color.to_string(),
        opacity,
    }
}

#[test]
fn an_empty_list_of_layers_is_a_small_picture_and_not_an_error() {
    let svg = render(&[], 1.0);
    assert!(
        svg.starts_with("<svg") && svg.trim_end().ends_with("</svg>"),
        "{svg}"
    );
    let svg = render(&[layer("a", "#ffffff", 1.0, vec![])], 1.0);
    assert!(svg.contains("viewBox"));
}

#[test]
fn a_bad_colour_is_grey_an_opacity_is_kept_between_zero_and_one_and_a_name_is_escaped() {
    let s = vec![disc(1.0, 1.0, 0.5)];
    let svg = render(
        &[
            layer("bad", "red", 2.5, s.clone()),
            layer("short", "#fff", -3.0, s.clone()),
            layer("nan", "#12345g", f64::NAN, s.clone()),
            layer("a<b&\"c\">", "#AbCdEf", 0.5, s),
        ],
        0.0,
    );
    assert!(svg.contains("fill=\"#b0b0b0\""));
    assert!(svg.contains("opacity=\"1\""));
    assert!(svg.contains("opacity=\"0\""));
    assert!(svg.contains("fill=\"#AbCdEf\""));
    assert!(svg.contains("id=\"a&lt;b&amp;&quot;c&quot;&gt;\""), "{svg}");
    assert!(!svg.contains("a<b&"));
}

#[test]
fn a_negative_or_infinite_margin_is_taken_as_none_and_the_box_has_the_size_of_the_work() {
    let l = layer("a", "#ffffff", 1.0, vec![disc(5.0, 5.0, 1.0)]);
    let plain = render(std::slice::from_ref(&l), 0.0);
    assert_eq!(render(std::slice::from_ref(&l), -4.0), plain);
    assert_eq!(render(std::slice::from_ref(&l), f64::INFINITY), plain);
    assert_eq!(render(std::slice::from_ref(&l), f64::NAN), plain);
    // a disc of radius 1 at (5, 5): the box is 2 x 2 and starts at x = 4, y = -6 (y goes up in the board)
    assert!(plain.contains("viewBox=\"4 -6 2 2\""), "{plain}");
    let margin = render(&[l], 1.0);
    assert!(margin.contains("viewBox=\"3 -7 4 4\""), "{margin}");
}

#[test]
fn every_kind_of_shape_is_drawn_once() {
    let svg = render(
        &[layer(
            "a",
            "#c8a33c",
            1.0,
            vec![
                disc(0.0, 0.0, 1.0),
                Shape::Rect {
                    x: 3.0,
                    y: 0.0,
                    w: 2.0,
                    h: 1.0,
                },
                Shape::Obround {
                    x: 6.0,
                    y: 0.0,
                    w: 2.0,
                    h: 1.0,
                },
                stroke(0.0, 3.0, 5.0, 3.0, 0.3),
                Shape::Region {
                    pts: vec![(0.0, 5.0), (2.0, 5.0), (1.0, 7.0)],
                },
            ],
        )],
        0.5,
    );
    assert_eq!(svg.matches("<circle").count(), 1);
    assert_eq!(svg.matches("<line").count(), 1);
    assert_eq!(svg.matches("<polygon").count(), 1);
    assert_eq!(
        svg.matches("<rect").count(),
        3,
        "the background and two pads: {svg}"
    );
    assert_eq!(svg.matches("rx=").count(), 1);
}
