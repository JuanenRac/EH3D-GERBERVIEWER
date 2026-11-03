use eh3d_gerberviewer::align::{compare_boxes, holes_outside};
use eh3d_gerberviewer::measure::{
    centre_distance, distance, min_clearance, min_track_width, pad_centres,
};
use eh3d_gerberviewer::scene::{arc_points, from_excellon, from_gerber, Bounds, Shape};
use eh3d_gerberviewer::svg::{default_color, render, Layer};
use eh3d_parser_gerber::QuadrantMode;

const COPPER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\n%ADD11R,2.0X1.0*%\n%ADD12O,2.0X1.0*%\nD10*\nX00000Y00000D02*\nX100000Y00000D01*\nX100000Y100000D01*\n\
D11*\nX50000Y50000D03*\nD12*\nX20000Y80000D03*\nD10*\nX00000Y00000D03*\nM02*\n";
const DRILL: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX009.000Y009.000\nX005.000Y005.000\nM30\n";

#[test]
fn a_gerber_layer_becomes_shapes_in_millimetres() {
    let s = from_gerber(&eh3d_parser_gerber::parse(COPPER));
    assert_eq!(
        s.shapes
            .iter()
            .filter(|x| matches!(x, Shape::Stroke { .. }))
            .count(),
        2
    );
    assert!(s
        .shapes
        .iter()
        .any(|x| matches!(x, Shape::Rect { w, h, .. } if *w == 2.0 && *h == 1.0)));
    assert!(s.shapes.iter().any(|x| matches!(x, Shape::Obround { .. })));
    assert!(s
        .shapes
        .iter()
        .any(|x| matches!(x, Shape::Disc { r, .. } if (*r - 0.25).abs() < 1e-9)));
    let b = s.bounds();
    assert!(
        (b.width() - 10.5).abs() < 1e-6 && (b.height() - 10.5).abs() < 1e-6,
        "{b:?}"
    );
}

#[test]
fn inches_are_converted() {
    let text =
        "%FSLAX24Y24*%\n%MOIN*%\n%ADD10C,0.1*%\nD10*\nX00000Y00000D02*\nX10000Y00000D01*\nM02*\n";
    let s = from_gerber(&eh3d_parser_gerber::parse(text));
    match &s.shapes[0] {
        Shape::Stroke { x1, w, .. } => assert!((x1 - 25.4).abs() < 1e-9 && (w - 2.54).abs() < 1e-9),
        other => panic!("{other:?}"),
    }
}

#[test]
fn what_is_not_drawn_is_said_once() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\n%LPC*%\nD10*\nX0Y0D03*\nX10000Y0D03*\n%LPC*%\nM02*\n";
    let s = from_gerber(&eh3d_parser_gerber::parse(text));
    assert_eq!(
        s.notes
            .iter()
            .filter(|n| n.contains("clear polarity"))
            .count(),
        1
    );
    let none = from_gerber(&eh3d_parser_gerber::parse(
        "%FSLAX24Y24*%\n%MOMM*%\nX0Y0D03*\nM02*\n",
    ));
    assert!(none.shapes.is_empty() && !none.notes.is_empty());
}

#[test]
fn a_region_becomes_a_polygon() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG36*\nX00000Y00000D02*\nX100000Y00000D01*\nX100000Y100000D01*\nX00000Y100000D01*\nX00000Y00000D01*\nG37*\nM02*\n";
    let s = from_gerber(&eh3d_parser_gerber::parse(text));
    assert!(
        s.shapes
            .iter()
            .any(|x| matches!(x, Shape::Region { pts } if pts.len() >= 4)),
        "{:?}",
        s.shapes
    );
    assert!(
        s.shapes.iter().all(|x| !matches!(x, Shape::Stroke { .. })),
        "the lines of a region are its outline, not tracks"
    );
}

#[test]
fn an_arc_ends_where_it_should_and_is_as_long_as_the_circle_says() {
    let pts = arc_points(
        (1.0, 0.0),
        (0.0, 1.0),
        (-1.0, 0.0),
        false,
        QuadrantMode::Multi,
    );
    assert!(
        distance(pts[0], (1.0, 0.0)) < 1e-9 && distance(*pts.last().unwrap(), (0.0, 1.0)) < 1e-9
    );
    assert!(pts
        .iter()
        .all(|p| (distance(*p, (0.0, 0.0)) - 1.0).abs() < 1e-9));
    let long = arc_points(
        (1.0, 0.0),
        (0.0, 1.0),
        (-1.0, 0.0),
        true,
        QuadrantMode::Multi,
    );
    let angle = |v: &[(f64, f64)]| v.windows(2).map(|w| distance(w[0], w[1])).sum::<f64>();
    assert!((angle(&pts) - std::f64::consts::FRAC_PI_2).abs() < 0.01);
    assert!(
        (angle(&long) - 3.0 * std::f64::consts::FRAC_PI_2).abs() < 0.02,
        "clockwise from 0 to 90 degrees goes the long way round"
    );
    let single = arc_points(
        (1.0, 0.0),
        (0.0, 1.0),
        (1.0, 0.0),
        false,
        QuadrantMode::Single,
    );
    assert!(
        single
            .iter()
            .all(|p| (distance(*p, (0.0, 0.0)) - 1.0).abs() < 1e-9),
        "single quadrant mode finds the centre by itself"
    );
    assert_eq!(
        arc_points(
            (1.0, 1.0),
            (2.0, 2.0),
            (0.0, 0.0),
            true,
            QuadrantMode::Multi
        )
        .len(),
        2,
        "an arc with no radius is a straight line"
    );
}

#[test]
fn the_holes_of_a_drill_file_are_discs() {
    let s = from_excellon(&eh3d_parser_excellon::parse(DRILL));
    assert_eq!(s.shapes.len(), 2);
    assert!(
        matches!(s.shapes[0], Shape::Disc { x, y, r } if x == 9.0 && y == 9.0 && (r - 0.4).abs() < 1e-9)
    );
}

#[test]
fn garbage_and_empty_layers_do_not_panic() {
    for text in [
        "",
        "\u{0}\u{1}\u{2}",
        "G36*G36*G37*G37*",
        "X999999999999Y1D01*",
        "%ADD10C,*%D10*X0Y0D03*",
        "%MOMM*%%FSLAX24Y24*%%ADD10C,1*%D10*X0Y0D02*X0Y0I0J0D01*G03*",
    ] {
        let s = from_gerber(&eh3d_parser_gerber::parse(text));
        let _ = s.bounds();
        let _ = render(
            &[Layer {
                name: "x".into(),
                scene: s,
                color: "#fff".into(),
                opacity: 1.0,
            }],
            1.0,
        );
    }
    assert!(from_excellon(&eh3d_parser_excellon::parse(""))
        .shapes
        .is_empty());
}

#[test]
fn the_picture_is_valid_looking_svg_with_one_group_per_layer() {
    let s = from_gerber(&eh3d_parser_gerber::parse(COPPER));
    let d = from_excellon(&eh3d_parser_excellon::parse(DRILL));
    let svg = render(
        &[
            Layer {
                name: "top <&> \"copper\"".into(),
                scene: s,
                color: default_color("a.GTL").into(),
                opacity: 0.5,
            },
            Layer {
                name: "holes".into(),
                scene: d,
                color: "not a colour".into(),
                opacity: f64::NAN,
            },
        ],
        2.0,
    );
    assert!(svg.starts_with("<svg ") && svg.trim_end().ends_with("</svg>"));
    assert_eq!(svg.matches("<g ").count(), 2);
    assert!(svg.contains("top &lt;&amp;&gt; &quot;copper&quot;") && !svg.contains("<&>"));
    assert!(
        svg.contains("fill=\"#b0b0b0\"")
            && svg.contains("opacity=\"1\"")
            && svg.contains("opacity=\"0.5\"")
    );
    assert!(svg.contains("scale(1,-1)"));
    assert!(!svg.contains("NaN") && !svg.contains("inf"));
}

#[test]
fn an_empty_picture_is_a_small_empty_picture() {
    let svg = render(&[], 1.0);
    assert!(svg.contains("viewBox") && !svg.contains("NaN") && !svg.contains("inf"));
    assert!(render(&[], f64::NAN).contains("viewBox"));
}

#[test]
fn colours_come_from_the_extension() {
    assert_eq!(default_color("board.GTL"), "#c8a33c");
    assert_eq!(default_color("board.gbl"), "#3c7bc8");
    assert_eq!(default_color("holes.NCD"), "#ffffff");
    assert_eq!(default_color("README"), "#b0b0b0");
}

#[test]
fn the_narrowest_track_and_the_pads_are_found() {
    let s = from_gerber(&eh3d_parser_gerber::parse(COPPER));
    assert_eq!(min_track_width(&s), Some(0.5));
    assert_eq!(pad_centres(&s).len(), 3);
    assert!((centre_distance(&s, 0, 1).unwrap() - distance((5.0, 5.0), (2.0, 8.0))).abs() < 1e-9);
    assert!(centre_distance(&s, 0, 9).unwrap_err().contains("no pad 9"));
    assert_eq!(
        min_track_width(&from_excellon(&eh3d_parser_excellon::parse(DRILL))),
        None
    );
}

#[test]
fn the_smallest_gap_between_two_tracks_is_measured_edge_to_edge() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,1.0*%\nD10*\nX00000Y00000D02*\nX100000Y00000D01*\nX00000Y30000D02*\nX100000Y30000D01*\nM02*\n";
    let s = from_gerber(&eh3d_parser_gerber::parse(text));
    let (gap, i, j) = min_clearance(&s).unwrap().unwrap();
    assert!(
        (gap - 2.0).abs() < 1e-9 && (i, j) == (0, 1),
        "tracks 3 mm apart and 1 mm wide leave 2 mm, got {gap}"
    );
    let crossing = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,1.0*%\nD10*\nX00000Y00000D02*\nX100000Y100000D01*\nX00000Y100000D02*\nX100000Y00000D01*\nM02*\n";
    assert_eq!(
        min_clearance(&from_gerber(&eh3d_parser_gerber::parse(crossing)))
            .unwrap()
            .unwrap()
            .0,
        0.0
    );
    assert_eq!(
        min_clearance(&from_gerber(&eh3d_parser_gerber::parse(
            "%FSLAX24Y24*%\n%MOMM*%\nM02*\n"
        )))
        .unwrap(),
        None
    );
}

#[test]
fn layers_that_do_not_line_up_are_reported() {
    let a = Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 100.0,
        max_y: 80.0,
    };
    let shifted = Bounds {
        min_x: 3.0,
        min_y: 0.0,
        max_x: 103.0,
        max_y: 80.0,
    };
    let f = compare_boxes(
        &[
            ("top".into(), a),
            ("bottom".into(), a),
            ("mask".into(), shifted),
            ("empty".into(), Bounds::empty()),
        ],
        0.5,
    );
    assert_eq!(f.len(), 2, "{f:?}");
    assert!(f[0].layer == "mask" && f[0].message.contains("+3.000"));
    assert!(f[1].layer == "empty");
    assert!(compare_boxes(&[("top".into(), a), ("bottom".into(), shifted)], 5.0).is_empty());
    assert!(compare_boxes(&[], 1.0).is_empty());
}

#[test]
fn a_hole_outside_the_copper_is_reported() {
    let copper = from_gerber(&eh3d_parser_gerber::parse(COPPER));
    let inside = from_excellon(&eh3d_parser_excellon::parse(DRILL));
    assert!(holes_outside(&copper, &inside, "holes").is_empty());
    let out = from_excellon(&eh3d_parser_excellon::parse(
        "M48\nMETRIC\nT1C0.8\n%\nT1\nX050.000Y050.000\nM30\n",
    ));
    let f = holes_outside(&copper, &out, "holes");
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("0.80 mm"));
    assert_eq!(
        holes_outside(
            &from_gerber(&eh3d_parser_gerber::parse("")),
            &inside,
            "holes"
        )
        .len(),
        2
    );
}

#[test]
fn a_track_with_a_rectangular_pen_is_the_swept_rectangle() {
    use eh3d_gerberviewer::scene::swept_rectangle;
    // along an axis the result is one rectangle
    let h = swept_rectangle((0.0, 0.0), (10.0, 0.0), 2.0, 1.0);
    assert_eq!(h.len(), 4, "{h:?}");
    let xs: Vec<f64> = h.iter().map(|p| p.0).collect();
    let ys: Vec<f64> = h.iter().map(|p| p.1).collect();
    assert!(xs.iter().cloned().fold(f64::INFINITY, f64::min) == -1.0);
    assert!(xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) == 11.0);
    assert!(ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max) == 0.5);
    // on the diagonal the corners of the rectangle at both ends make a hexagon, not a round-ended line
    let d = swept_rectangle((0.0, 0.0), (10.0, 10.0), 2.0, 1.0);
    assert_eq!(d.len(), 6, "{d:?}");
    // a segment of no length is the pen itself
    assert_eq!(swept_rectangle((3.0, 3.0), (3.0, 3.0), 2.0, 1.0).len(), 4);

    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD11R,2.0X1.0*%\nD11*\nX00000Y00000D02*\nX100000Y100000D01*\nM02*\n";
    let s = from_gerber(&eh3d_parser_gerber::parse(text));
    assert!(s.shapes.iter().all(|x| !matches!(x, Shape::Stroke { .. })));
    match &s.shapes[0] {
        Shape::Region { pts } => assert_eq!(pts.len(), 6),
        other => panic!("{other:?}"),
    }
    assert!(s.notes.is_empty(), "{:?}", s.notes);
}

// ---- more edges: tolerances, the same name twice, measures that touch, colours and the command line ----

use eh3d_gerberviewer::measure::MAX_PAIRS_SHAPES;
use eh3d_gerberviewer::scene::Scene;
use std::process::Command;

fn boxed(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
    let mut b = Bounds::empty();
    b.include(x0, y0);
    b.include(x1, y1);
    b
}

type Track = ((f64, f64), (f64, f64), f64);

fn strokes(list: &[Track]) -> Scene {
    Scene {
        shapes: list
            .iter()
            .map(|&((x0, y0), (x1, y1), w)| Shape::Stroke { x0, y0, x1, y1, w })
            .collect(),
        notes: vec![],
    }
}

#[test]
fn a_centre_off_by_exactly_the_tolerance_is_fine_and_a_hair_more_is_reported_with_its_signs() {
    let layers = |dx: f64| {
        vec![
            ("a.gtl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
            ("b.gtl".to_string(), boxed(dx, 0.0, 10.0 + dx, 10.0)),
        ]
    };
    assert!(compare_boxes(&layers(0.5), 0.5).is_empty());
    let f = compare_boxes(&layers(-0.5001), 0.5);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].layer, "b.gtl");
    assert!(
        f[0].message.contains("-0.500 mm in x")
            && f[0].message.contains("+0.000 mm in y")
            && f[0].message.contains("a.gtl"),
        "{}",
        f[0].message
    );
}

#[test]
fn an_empty_layer_is_named_and_the_first_layer_with_something_is_the_reference() {
    let layers = vec![
        ("empty.gko".to_string(), Bounds::empty()),
        ("a.gtl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
        ("b.gbl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
    ];
    let f = compare_boxes(&layers, 0.1);
    assert_eq!(f.len(), 1);
    assert_eq!(
        (f[0].layer.as_str(), f[0].message.as_str()),
        ("empty.gko", "has nothing to draw")
    );
    assert!(compare_boxes(&[], 0.1).is_empty());
}

#[test]
fn two_layers_with_the_same_name_are_both_compared() {
    // the same file name in two folders: the second one is a different layer and it does not line up
    let layers = vec![
        ("top.gtl".to_string(), boxed(0.0, 0.0, 10.0, 10.0)),
        ("top.gtl".to_string(), boxed(50.0, 0.0, 60.0, 10.0)),
    ];
    let f = compare_boxes(&layers, 0.5);
    assert_eq!(f.len(), 1, "{f:?}");
}

#[test]
fn a_hole_that_touches_the_edge_of_the_copper_is_inside_and_one_beyond_it_is_not() {
    let copper = Scene {
        shapes: vec![Shape::Rect {
            x: 5.0,
            y: 5.0,
            w: 10.0,
            h: 10.0,
        }],
        notes: vec![],
    };
    let holes = |x: f64, r: f64| Scene {
        shapes: vec![Shape::Disc { x, y: 5.0, r }],
        notes: vec![],
    };
    assert!(
        holes_outside(&copper, &holes(1.0, 1.0), "d.drl").is_empty(),
        "touching the edge is inside"
    );
    let out = holes_outside(&copper, &holes(0.99, 1.0), "d.drl");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].layer, "d.drl");
    assert!(
        out[0].message.contains("2.00 mm") && out[0].message.contains("0.990"),
        "{}",
        out[0].message
    );
    // a drill file has discs only: anything else in it is not a hole
    let odd = Scene {
        shapes: vec![Shape::Rect {
            x: 99.0,
            y: 99.0,
            w: 1.0,
            h: 1.0,
        }],
        notes: vec![],
    };
    assert!(holes_outside(&copper, &odd, "d.drl").is_empty());
    // copper with nothing in it: every hole is outside
    assert_eq!(
        holes_outside(&Scene::default(), &holes(1.0, 0.4), "d.drl").len(),
        1
    );
}

#[test]
fn collinear_overlapping_and_touching_tracks_leave_no_gap_and_parallel_ones_leave_the_exact_gap() {
    let overlap = strokes(&[
        ((0.0, 0.0), (10.0, 0.0), 0.2),
        ((5.0, 0.0), (15.0, 0.0), 0.2),
    ]);
    assert_eq!(min_clearance(&overlap).unwrap().unwrap().0, 0.0);
    let end_to_end = strokes(&[((0.0, 0.0), (5.0, 0.0), 0.2), ((5.0, 0.0), (9.0, 3.0), 0.2)]);
    assert_eq!(min_clearance(&end_to_end).unwrap().unwrap().0, 0.0);
    let parallel = strokes(&[
        ((0.0, 0.0), (10.0, 0.0), 0.2),
        ((2.0, 1.0), (8.0, 1.0), 0.4),
    ]);
    assert!((min_clearance(&parallel).unwrap().unwrap().0 - 0.7).abs() < 1e-9);
    // a track and a round pad at the side of it
    let mut mixed = strokes(&[((0.0, 0.0), (10.0, 0.0), 0.2)]);
    mixed.shapes.push(Shape::Disc {
        x: 5.0,
        y: 2.0,
        r: 0.5,
    });
    assert!((min_clearance(&mixed).unwrap().unwrap().0 - 1.4).abs() < 1e-9);
    // a track of no length is a point with a width
    let dot = strokes(&[((0.0, 0.0), (0.0, 0.0), 1.0), ((3.0, 0.0), (3.0, 0.0), 1.0)]);
    assert!((min_clearance(&dot).unwrap().unwrap().0 - 2.0).abs() < 1e-9);
}

#[test]
fn the_pair_with_the_smallest_gap_is_named_by_the_place_of_its_shapes_in_the_layer() {
    let mut s = Scene {
        shapes: vec![Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        }],
        notes: vec![],
    };
    s.shapes.push(Shape::Disc {
        x: 0.0,
        y: 10.0,
        r: 0.5,
    });
    s.shapes.push(Shape::Disc {
        x: 10.0,
        y: 10.0,
        r: 0.5,
    });
    s.shapes.push(Shape::Disc {
        x: 12.0,
        y: 10.0,
        r: 0.5,
    });
    let (gap, i, j) = min_clearance(&s).unwrap().unwrap();
    assert_eq!(
        (i, j),
        (2, 3),
        "the rectangle is not compared but it still counts as shape 0"
    );
    assert!((gap - 1.0).abs() < 1e-9);
    assert_eq!(
        min_clearance(&Scene {
            shapes: vec![Shape::Disc {
                x: 0.0,
                y: 0.0,
                r: 1.0
            }],
            notes: vec![]
        })
        .unwrap(),
        None
    );
}

#[test]
fn too_many_shapes_to_compare_pair_by_pair_is_an_error_that_says_how_many() {
    let many = Scene {
        shapes: (0..=MAX_PAIRS_SHAPES)
            .map(|i| Shape::Disc {
                x: i as f64,
                y: 0.0,
                r: 0.1,
            })
            .collect(),
        notes: vec![],
    };
    let e = min_clearance(&many).unwrap_err();
    assert!(e.contains(&(MAX_PAIRS_SHAPES + 1).to_string()), "{e}");
}

#[test]
fn the_pad_numbers_ignore_tracks_and_regions_and_count_every_flashed_shape() {
    let s = Scene {
        shapes: vec![
            Shape::Stroke {
                x0: 0.0,
                y0: 0.0,
                x1: 1.0,
                y1: 1.0,
                w: 0.1,
            },
            Shape::Disc {
                x: 1.0,
                y: 1.0,
                r: 0.1,
            },
            Shape::Region {
                pts: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
            },
            Shape::Obround {
                x: 2.0,
                y: 2.0,
                w: 1.0,
                h: 0.5,
            },
            Shape::Rect {
                x: 3.0,
                y: 3.0,
                w: 1.0,
                h: 0.5,
            },
        ],
        notes: vec![],
    };
    assert_eq!(pad_centres(&s), vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)]);
    assert!((centre_distance(&s, 0, 2).unwrap() - 8.0_f64.sqrt()).abs() < 1e-12);
    assert_eq!(centre_distance(&s, 1, 1).unwrap(), 0.0);
    assert!(centre_distance(&s, 3, 0)
        .unwrap_err()
        .contains("the layer has 3"));
    assert_eq!(
        min_track_width(&strokes(&[
            ((0.0, 0.0), (1.0, 0.0), 0.3),
            ((0.0, 1.0), (1.0, 1.0), 0.15),
            ((0.0, 2.0), (1.0, 2.0), 0.2)
        ])),
        Some(0.15)
    );
}

fn picture(color: &str, opacity: f64, name: &str, margin: f64) -> String {
    let layer = Layer {
        name: name.to_string(),
        scene: Scene {
            shapes: vec![Shape::Disc {
                x: 5.0,
                y: 5.0,
                r: 1.0,
            }],
            notes: vec![],
        },
        color: color.to_string(),
        opacity,
    };
    render(&[layer], margin)
}

#[test]
fn a_colour_that_is_not_six_hex_digits_is_the_grey_and_the_opacity_is_kept_in_range() {
    for bad in [
        "red",
        "#12345",
        "#1234567",
        "#12345g",
        "123456",
        "",
        "url(#x)\" onload=\"x",
    ] {
        let svg = picture(bad, 1.0, "l", 1.0);
        assert!(svg.contains("fill=\"#b0b0b0\""), "{bad:?}");
        assert!(!svg.contains("onload"), "{bad:?}");
    }
    assert!(picture("#AbCdEf", 1.0, "l", 1.0).contains("fill=\"#AbCdEf\""));
    for (given, shown) in [
        (0.5, "opacity=\"0.5\""),
        (7.0, "opacity=\"1\""),
        (-3.0, "opacity=\"0\""),
        (f64::NAN, "opacity=\"1\""),
        (f64::INFINITY, "opacity=\"1\""),
    ] {
        assert!(
            picture("#ffffff", given, "l", 1.0).contains(shown),
            "{given}"
        );
    }
}

#[test]
fn the_name_of_a_layer_cannot_break_out_of_the_attribute() {
    let svg = picture("#ffffff", 1.0, "a\"b<c>&d", 1.0);
    assert!(svg.contains("id=\"a&quot;b&lt;c&gt;&amp;d\""), "{svg}");
    assert_eq!(svg.matches("<g ").count(), 1);
}

#[test]
fn the_view_box_has_the_margin_and_the_board_flipped_so_that_y_is_up() {
    let svg = picture("#ffffff", 1.0, "l", 2.0);
    // the disc spans 4..6 in both axes: with a margin of 2 the box is 6 x 6 from x 2, and y from -8 (the flipped top)
    assert!(svg.contains("viewBox=\"2 -8 6 6\""), "{svg}");
    assert!(svg.contains("transform=\"scale(1,-1)\""));
    for bad in [-5.0, f64::NAN] {
        assert!(
            picture("#ffffff", 1.0, "l", bad).contains("viewBox=\"4 -6 2 2\""),
            "a margin that makes no sense is no margin: {bad}"
        );
    }
}

#[test]
fn the_picture_is_the_same_every_time_and_has_balanced_groups() {
    let a = picture("#ffffff", 0.5, "l", 1.0);
    assert_eq!(a, picture("#ffffff", 0.5, "l", 1.0));
    assert_eq!(a.matches("<g ").count(), a.matches("</g>").count());
    assert!(a.starts_with("<svg ") && a.ends_with("</svg>\n"));
}

#[test]
fn the_colour_of_a_layer_comes_from_the_last_extension_in_any_case() {
    assert_eq!(default_color("BOARD.GTL"), default_color("board.gtl"));
    assert_eq!(default_color("a.v2.gbl"), default_color("x.gbl"));
    assert_eq!(default_color("noextension"), "#b0b0b0");
    assert_eq!(
        default_color("gtl"),
        default_color("x.gtl"),
        "a name that is only an extension is that extension"
    );
    assert_eq!(default_color("x.gtl.bak"), "#b0b0b0");
    assert_ne!(default_color("x.gtl"), default_color("x.gbl"));
    assert_eq!(default_color("x.gts"), default_color("x.gbs"));
}

fn viewer(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-gerberview"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_viewer_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_command_line_draws_measures_and_checks_layers() {
    let dir = scratch("cli");
    let (top, bottom, drill) = (
        dir.join("top.gtl"),
        dir.join("bottom.gbl"),
        dir.join("holes.drl"),
    );
    std::fs::write(&top, COPPER).unwrap();
    std::fs::write(&bottom, COPPER).unwrap();
    std::fs::write(&drill, DRILL).unwrap();
    let svg = dir.join("out.svg");
    let (c, o, e) = viewer(&[
        "render",
        svg.to_str().unwrap(),
        top.to_str().unwrap(),
        bottom.to_str().unwrap(),
        drill.to_str().unwrap(),
        "--opacity",
        "0.5",
    ]);
    assert_eq!(c, Some(0), "{e}");
    assert!(o.contains("3 layers drawn"), "{o}");
    let text = std::fs::read_to_string(&svg).unwrap();
    assert_eq!(text.matches("<g ").count(), 3);
    assert!(text.contains("id=\"holes.drl\"") && text.contains("opacity=\"0.5\""));
    let (c, o, _) = viewer(&["measure", top.to_str().unwrap()]);
    assert_eq!(c, Some(0));
    assert!(
        o.contains("narrowest track: 0.500 mm") && o.contains("3 pads"),
        "{o}"
    );
    let (c, o, e) = viewer(&["check", top.to_str().unwrap(), bottom.to_str().unwrap()]);
    assert_eq!(c, Some(0), "{e}");
    assert!(o.contains("the layers line up"));
    let (c, _, e) = viewer(&["info", top.to_str().unwrap(), drill.to_str().unwrap()]);
    assert_eq!(c, Some(0), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_command_line_fails_with_one_line_per_finding_when_a_hole_is_outside_the_copper() {
    let dir = scratch("cli_check");
    let (top, drill) = (dir.join("top.gtl"), dir.join("holes.drl"));
    std::fs::write(&top, COPPER).unwrap();
    std::fs::write(
        &drill,
        "M48\nMETRIC\nT1C0.8\n%\nT1\nX500.000Y500.000\nX005.000Y005.000\nM30\n",
    )
    .unwrap();
    let (c, _, e) = viewer(&["check", top.to_str().unwrap(), drill.to_str().unwrap()]);
    assert_eq!(c, Some(1));
    assert_eq!(
        e.lines()
            .filter(|l| l.contains("is not inside the copper"))
            .count(),
        1,
        "{e}"
    );
    assert!(e.contains("holes.drl:"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_command_line_names_what_is_missing_or_wrong() {
    let dir = scratch("cli_errors");
    let top = dir.join("top.gtl");
    std::fs::write(&top, COPPER).unwrap();
    let t = top.to_str().unwrap();
    let none = dir.join("none.gtl");
    let out = dir.join("o.svg");
    for (args, word) in [
        (vec!["render", out.to_str().unwrap()], "at least one layer"),
        (
            vec!["render", out.to_str().unwrap(), none.to_str().unwrap()],
            "none.gtl",
        ),
        (
            vec!["render", out.to_str().unwrap(), t, "--opacity", "-1"],
            "--opacity",
        ),
        (
            vec!["render", out.to_str().unwrap(), t, "--margin", "wide"],
            "--margin",
        ),
        (
            vec!["render", out.to_str().unwrap(), t, "--margin"],
            "needs a value",
        ),
        (vec!["info"], "needs a layer"),
        (vec!["measure"], "needs a layer"),
        (vec!["check"], "needs layers"),
        (vec!["check", t, "--tolerance", "nan"], "--tolerance"),
        (vec!["frobnicate"], "unknown command"),
    ] {
        let (c, _, e) = viewer(&args);
        assert_eq!(c, Some(1), "{args:?}");
        assert!(e.starts_with("error:") && e.contains(word), "{args:?}: {e}");
    }
    assert!(!out.exists(), "nothing is written after an error");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_says_when_there_are_no_layers_to_compare_and_measure_refuses_extra_arguments() {
    let dir = scratch("cli_nothing_to_compare");
    let (top, drill) = (dir.join("top.gtl"), dir.join("holes.drl"));
    std::fs::write(&top, COPPER).unwrap();
    std::fs::write(
        &drill,
        "M48\nMETRIC\nT1C0.8\n%\nT1\nX005.000Y005.000\nM30\n",
    )
    .unwrap();
    let (t, d) = (top.to_str().unwrap(), drill.to_str().unwrap());
    // only a drill file: nothing was compared and it must not claim that the layers line up
    let (c, o, _) = viewer(&["check", d]);
    assert_eq!(c, Some(0));
    assert!(
        o.contains("no layers to compare") && o.contains("nothing was checked"),
        "{o}"
    );
    assert!(!o.contains("line up"), "{o}");
    // one layer and its holes: the holes were checked, the layers were not compared
    let (c, o, _) = viewer(&["check", t, d]);
    assert_eq!(c, Some(0));
    assert!(
        o.contains("no layers to compare") && o.contains("holes are inside"),
        "{o}"
    );
    // a second argument of measure is a mistake of use
    let (c, _, e) = viewer(&["measure", t, d]);
    assert_eq!(c, Some(1));
    assert!(e.contains("measure takes one layer"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_layer_the_reader_found_errors_in_or_with_nothing_to_draw_fails_every_command() {
    let dir = scratch("cli_bad_layer");
    let (bad, empty, good) = (
        dir.join("bad.gtl"),
        dir.join("empty.gtl"),
        dir.join("good.gtl"),
    );
    // seven digits do not fit the format 2.4: the block is lost
    std::fs::write(
        &bad,
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX1234567Y0D01*\nM02*\n",
    )
    .unwrap();
    std::fs::write(&empty, "%FSLAX24Y24*%\n%MOMM*%\nM02*\n").unwrap();
    std::fs::write(&good, COPPER).unwrap();
    let (b, e, g) = (
        bad.to_str().unwrap(),
        empty.to_str().unwrap(),
        good.to_str().unwrap(),
    );
    let svg = dir.join("o.svg");
    for args in [
        vec!["info", b],
        vec!["measure", b],
        vec!["check", b, g],
        vec!["render", svg.to_str().unwrap(), b],
        vec!["info", e],
        vec!["measure", e],
        vec!["render", svg.to_str().unwrap(), e],
    ] {
        let (c, _, err) = viewer(&args);
        assert_eq!(c, Some(1), "{args:?}: {err}");
    }
    let (_, _, err) = viewer(&["info", b]);
    assert!(
        err.contains("[coordinate-invalid]") || err.contains("error"),
        "{err}"
    );
    assert!(!svg.exists());
    let (c, _, err) = viewer(&["info", g]);
    assert_eq!(c, Some(0), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_layer_over_the_size_limit_is_refused_in_the_language_of_the_system_and_the_arc_tolerance_is_checked(
) {
    let dir = scratch("cli_limit");
    let top = dir.join("top.gtl");
    std::fs::write(&top, COPPER).unwrap();
    let t = top.to_str().unwrap();
    for (lang, word) in [
        ("en", "above the limit"),
        ("es", "supera el límite"),
        ("de", "überschreitet die Grenze"),
        ("fr", "dépasse la limite"),
        ("it", "supera il limite"),
        ("ja", "上限を超えています"),
        ("zh", "文件超过上限"),
        ("pt", "excede o limite"),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_eh3d-gerberview"))
            .args(["info", t, "--max-bytes", "10"])
            .env("LC_ALL", lang)
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(out.status.code(), Some(1), "{lang}: {err}");
        assert!(err.contains(word), "{lang}: {err}");
    }
    let (c, _, e) = viewer(&["info", t, "--max-bytes", "1000000"]);
    assert_eq!(c, Some(0), "{e}");
    for bad in ["0", "5", "x"] {
        let (c, _, e) = viewer(&["info", t, "--arc-tolerance", bad]);
        assert_eq!(c, Some(1), "{bad}");
        assert!(e.contains("--arc-tolerance"), "{bad}: {e}");
    }
    let (c, _, e) = viewer(&["info", t, "--arc-tolerance", "0.001"]);
    assert_eq!(c, Some(0), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_never_overwrites_a_layer_when_the_arguments_are_in_the_wrong_order() {
    let dir = scratch("cli_render_order");
    let top = dir.join("top.gtl");
    std::fs::write(&top, COPPER).unwrap();
    let svg = dir.join("out.svg");
    let (c, _, e) = viewer(&["render", top.to_str().unwrap(), svg.to_str().unwrap()]);
    assert_eq!(c, Some(1), "{e}");
    assert!(e.contains("looks like a layer"), "{e}");
    assert_eq!(std::fs::read_to_string(&top).unwrap(), COPPER);
    let (c, _, e) = viewer(&["render", svg.to_str().unwrap(), svg.to_str().unwrap()]);
    assert_eq!(c, Some(1), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}
