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
