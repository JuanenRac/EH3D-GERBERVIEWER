//! Behaviour at the exact edge of each rule: the zoom and pan of the window with seeded values, the list of files, the triangles of a region, the settings, the stack of layers, the
//! gaps, the ruler, the words of the eight languages and the command line. Nothing here uses a clock or the network.

use eh3d_gerberviewer::browse::{list, matches_extension, size_text, start_dir, Browser};
use eh3d_gerberviewer::draw::{triangulate, Drawing, MAX_CORNERS};
use eh3d_gerberviewer::gap::{layer_gap, scene_gap, shape_gap};
use eh3d_gerberviewer::lang::{fill, Lang, Msg};
use eh3d_gerberviewer::layers::{Layer as StackLayer, Stack, Warning};
use eh3d_gerberviewer::looks_like_drill;
use eh3d_gerberviewer::ruler::{measure_points, ruler, snap, SnapKind, Snapped, Unit};
use eh3d_gerberviewer::scene::{Bounds, Scene, Shape};
use eh3d_gerberviewer::settings::{Settings, MAX_CFG_BYTES};
use eh3d_gerberviewer::viewport::{Viewport, MAX_SCALE, MIN_SCALE};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// A small generator with a fixed seed (31 bits of state).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = (self.0 * 1_103_515_245 + 12_345) & 0x7FFF_FFFF;
        self.0 >> 8
    }

    /// A number from `low` to `high`, in steps of a thousandth.
    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * (self.next() % 1001) as f64 / 1000.0
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("eh3d_gv_behaviour_{}_{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// ---------------------------------------------------------------- the zoom and the pan

#[test]
fn a_point_goes_to_the_screen_and_back_for_any_seeded_view() {
    let mut rng = Lcg(5);
    for _ in 0..200 {
        let v = Viewport {
            centre_x: rng.between(-100.0, 100.0),
            centre_y: rng.between(-100.0, 100.0),
            px_per_mm: rng.between(0.5, 500.0),
        };
        let (w, h) = (rng.between(100.0, 2000.0), rng.between(100.0, 2000.0));
        let (x, y) = (rng.between(-200.0, 200.0), rng.between(-200.0, 200.0));
        let (sx, sy) = v.to_screen(x, y, w, h);
        let (bx, by) = v.to_world(sx, sy, w, h);
        assert!(
            near(x, bx) && near(y, by),
            "({x}, {y}) came back as ({bx}, {by})"
        );
        let (cx, cy) = v.to_screen(v.centre_x, v.centre_y, w, h);
        assert!(near(cx, w / 2.0) && near(cy, h / 2.0));
    }
}

#[test]
fn the_whole_box_is_inside_the_picture_less_its_margin_for_any_seeded_box() {
    let mut rng = Lcg(6);
    for _ in 0..200 {
        let (x0, y0) = (rng.between(-300.0, 300.0), rng.between(-300.0, 300.0));
        let (x1, y1) = (x0 + rng.between(1.0, 500.0), y0 + rng.between(1.0, 500.0));
        let (w, h) = (rng.between(200.0, 1600.0), rng.between(200.0, 1600.0));
        let margin = rng.between(0.0, 50.0);
        let v = Viewport::fit((x0, y0), (x1, y1), w, h, margin);
        assert!((MIN_SCALE..=MAX_SCALE).contains(&v.px_per_mm));
        for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
            let (sx, sy) = v.to_screen(x, y, w, h);
            assert!(sx >= margin - 1e-6 && sx <= w - margin + 1e-6, "{sx}");
            assert!(sy >= margin - 1e-6 && sy <= h - margin + 1e-6, "{sy}");
        }
    }
}

#[test]
fn zooming_in_and_out_by_the_same_factor_comes_back_and_the_pointer_never_moves_from_its_point() {
    let mut rng = Lcg(7);
    for _ in 0..200 {
        let mut v = Viewport::fit((0.0, 0.0), (100.0, 80.0), 800.0, 600.0, 10.0);
        let original = v;
        let (sx, sy) = (rng.between(0.0, 800.0), rng.between(0.0, 600.0));
        let factor = rng.between(0.5, 2.0);
        let under = v.to_world(sx, sy, 800.0, 600.0);
        v.zoom_at(sx, sy, 800.0, 600.0, factor);
        let still = v.to_world(sx, sy, 800.0, 600.0);
        assert!(near(under.0, still.0) && near(under.1, still.1));
        v.zoom_at(sx, sy, 800.0, 600.0, 1.0 / factor);
        assert!(near(v.px_per_mm, original.px_per_mm));
        assert!(near(v.centre_x, original.centre_x) && near(v.centre_y, original.centre_y));
    }
}

#[test]
fn a_drag_and_the_drag_back_leave_the_view_where_it_was() {
    let mut rng = Lcg(8);
    for _ in 0..100 {
        let mut v = Viewport {
            centre_x: rng.between(-10.0, 10.0),
            centre_y: rng.between(-10.0, 10.0),
            px_per_mm: rng.between(1.0, 100.0),
        };
        let original = v;
        let (dx, dy) = (rng.between(-500.0, 500.0), rng.between(-500.0, 500.0));
        v.pan_px(dx, dy);
        v.pan_px(-dx, -dy);
        assert!(near(v.centre_x, original.centre_x) && near(v.centre_y, original.centre_y));
        assert_eq!(v.px_per_mm, original.px_per_mm);
    }
}

#[test]
fn a_box_of_one_point_zooms_as_far_as_allowed_and_a_huge_one_as_little_as_allowed() {
    let point = Viewport::fit((5.0, 5.0), (5.0, 5.0), 100.0, 100.0, 5.0);
    assert_eq!(point.px_per_mm, MAX_SCALE);
    assert_eq!((point.centre_x, point.centre_y), (5.0, 5.0));
    let huge = Viewport::fit((0.0, 0.0), (1.0e9, 1.0e9), 100.0, 100.0, 0.0);
    assert_eq!(huge.px_per_mm, MIN_SCALE);
    let tight = Viewport::fit((0.0, 0.0), (10.0, 10.0), 10.0, 10.0, 100.0);
    assert!(tight.px_per_mm.is_finite() && tight.px_per_mm > 0.0);
}

// ---------------------------------------------------------------- the list of files

#[test]
fn a_name_matches_an_extension_without_regard_to_case_and_a_name_with_only_a_dot_does_not() {
    assert!(matches_extension("a.b.GTL", &["gtl"]));
    assert!(matches_extension("anything", &[]));
    assert!(!matches_extension(".gtl", &["gtl"]));
    assert!(!matches_extension("gtl", &["gtl"]));
    assert!(!matches_extension("a.", &["gtl"]));
    assert!(!matches_extension("a.gtl.bak", &["gtl"]));
    assert!(matches_extension("a.drl", &["gtl", "DRL"]));
}

#[test]
fn a_size_is_told_in_bytes_kib_or_mib_at_the_exact_edges() {
    for (bytes, text) in [
        (0, "0 B"),
        (1023, "1023 B"),
        (1024, "1.0 KiB"),
        (1536, "1.5 KiB"),
        (1_048_575, "1024.0 KiB"),
        (1_048_576, "1.0 MiB"),
        (5_242_880, "5.0 MiB"),
    ] {
        assert_eq!(size_text(bytes), text, "{bytes}");
    }
}

fn make_folder(name: &str) -> PathBuf {
    let dir = scratch(name);
    fs::create_dir(dir.join("b_dir")).unwrap();
    fs::create_dir(dir.join("A_dir")).unwrap();
    fs::write(dir.join("z.gtl"), "12345").unwrap();
    fs::write(dir.join("Y.GTL"), "").unwrap();
    fs::write(dir.join("c.txt"), "x").unwrap();
    fs::write(dir.join("noext"), "x").unwrap();
    fs::write(dir.join(".hidden.gtl"), "x").unwrap();
    dir
}

#[test]
fn a_folder_lists_its_folders_first_then_the_files_that_match_and_hides_the_dot_names() {
    let dir = make_folder("list");
    let names = |ext: &[&str]| -> Vec<String> {
        list(&dir, ext)
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect()
    };
    assert_eq!(names(&["gtl"]), vec!["A_dir", "b_dir", "Y.GTL", "z.gtl"]);
    assert_eq!(
        names(&[]),
        vec!["A_dir", "b_dir", "c.txt", "noext", "Y.GTL", "z.gtl"]
    );
    let entries = list(&dir, &["gtl"]).unwrap();
    assert!(entries[0].is_dir && entries[0].size == 0);
    assert_eq!(entries[3].size, 5);
    assert!(list(&dir.join("not_there"), &[])
        .unwrap_err()
        .contains("not_there"));
    assert!(start_dir().is_dir());
}

#[test]
fn the_browser_enters_a_folder_returns_a_file_goes_up_and_tells_a_path_that_is_not_there() {
    let dir = make_folder("browser");
    let mut b = Browser::new(dir.clone(), &["gtl"]);
    assert_eq!(b.entries.len(), 4);
    assert!(b.error.is_none());
    b.show_all = true;
    b.refresh();
    assert_eq!(b.entries.len(), 6);
    assert_eq!(b.enter("A_dir"), None);
    assert_eq!(b.dir, dir.join("A_dir"));
    assert!(b.entries.is_empty());
    b.up();
    assert_eq!(b.dir, dir);
    assert_eq!(b.enter("z.gtl"), Some(dir.join("z.gtl")));
    b.path_text = format!("  {}  ", dir.join("b_dir").display());
    assert_eq!(b.go_typed(), None);
    assert_eq!(b.dir, dir.join("b_dir"));
    b.path_text = dir.join("Y.GTL").display().to_string();
    assert_eq!(b.go_typed(), Some(dir.join("Y.GTL")));
    b.path_text = dir.join("nowhere").display().to_string();
    assert_eq!(b.go_typed(), None);
    assert!(b.error.as_deref().unwrap_or("").contains("not found"));
    b.go(dir.join("nowhere"));
    assert!(b.error.is_some() && b.entries.is_empty());
}

// ---------------------------------------------------------------- the triangles of a region

fn shoelace(pts: &[(f64, f64)]) -> f64 {
    let mut sum = 0.0;
    for (i, p) in pts.iter().enumerate() {
        let q = pts[(i + 1) % pts.len()];
        sum += p.0 * q.1 - q.0 * p.1;
    }
    sum / 2.0
}

fn tri_area(pts: &[(f64, f64)], tris: &[[u32; 3]]) -> f64 {
    tris.iter()
        .map(|t| {
            let (a, b, c) = (pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
            ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs() / 2.0
        })
        .sum()
}

fn regular(n: usize, radius: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            (radius * a.cos(), radius * a.sin())
        })
        .collect()
}

#[test]
fn a_convex_polygon_of_n_corners_is_n_minus_two_triangles_with_its_own_area_either_way_round() {
    for n in [3, 4, 5, 8, 17, 60] {
        let pts = regular(n, 10.0);
        let tris = triangulate(&pts);
        assert_eq!(tris.len(), n - 2, "{n}");
        assert!(near(tri_area(&pts, &tris), shoelace(&pts).abs()), "{n}");
        let mut turned = pts.clone();
        turned.reverse();
        let back = triangulate(&turned);
        assert_eq!(back.len(), n - 2, "{n}");
        assert!(near(tri_area(&turned, &back), shoelace(&pts).abs()), "{n}");
        for t in &tris {
            assert!(t.iter().all(|&i| (i as usize) < n));
        }
    }
}

#[test]
fn a_closed_ring_gives_the_same_triangles_as_the_same_ring_left_open() {
    let open = [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 3.0)];
    let mut closed = open.to_vec();
    closed.push(open[0]);
    assert_eq!(triangulate(&open), triangulate(&closed));
    assert_eq!(triangulate(&open).len(), 2);
}

#[test]
fn a_region_with_no_area_few_corners_or_a_number_that_is_not_finite_has_no_triangles() {
    assert!(triangulate(&[]).is_empty());
    assert!(triangulate(&[(0.0, 0.0), (1.0, 1.0)]).is_empty());
    assert!(triangulate(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]).is_empty());
    assert!(triangulate(&[(0.0, 0.0), (1.0, 0.0), (f64::NAN, 1.0)]).is_empty());
    assert!(triangulate(&[(0.0, 0.0), (f64::INFINITY, 0.0), (0.0, 1.0)]).is_empty());
}

#[test]
fn a_region_of_the_most_corners_is_split_and_one_corner_more_is_drawn_as_an_outline_only() {
    let edge = regular(MAX_CORNERS, 50.0);
    assert_eq!(triangulate(&edge).len(), MAX_CORNERS - 2);
    let over = regular(MAX_CORNERS + 1, 50.0);
    assert!(triangulate(&over).is_empty());
}

#[test]
fn a_drawing_has_one_list_of_triangles_for_each_shape_and_only_a_region_has_any() {
    let shapes = vec![
        disc(0.0, 0.0, 1.0),
        Shape::Region {
            pts: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        },
        Shape::Stroke {
            x0: 0.0,
            y0: 0.0,
            x1: 1.0,
            y1: 1.0,
            w: 0.2,
        },
    ];
    let drawing = Drawing::new(&shapes);
    assert_eq!(drawing.tris.len(), shapes.len());
    assert!(drawing.tris[0].is_empty() && drawing.tris[2].is_empty());
    assert_eq!(drawing.tris[1].len(), 2);
    assert!(Drawing::new(&[]).extent().is_none());
}

#[test]
fn the_extent_of_a_drawing_grows_every_shape_by_its_own_radius_and_ignores_what_is_not_finite() {
    let drawing = Drawing::new(&[disc(1.0, 1.0, 2.0)]);
    assert_eq!(drawing.extent(), Some(((-1.0, -1.0), (3.0, 3.0))));
    let track = Drawing::new(&[Shape::Stroke {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 0.0,
        w: 2.0,
    }]);
    assert_eq!(track.extent(), Some(((-1.0, -1.0), (11.0, 1.0))));
    let region = Drawing::new(&[Shape::Region {
        pts: vec![(2.0, 3.0), (5.0, 3.0), (5.0, 9.0)],
    }]);
    assert_eq!(region.extent(), Some(((2.0, 3.0), (5.0, 9.0))));
    assert!(Drawing::new(&[disc(f64::NAN, 0.0, 1.0)]).extent().is_none());
}

// ---------------------------------------------------------------- the settings

#[test]
fn each_setting_accepts_its_limits_and_refuses_the_value_next_to_them() {
    for (text, ok) in [
        ("snap_tolerance_mm = 0", true),
        ("snap_tolerance_mm = 10", true),
        ("snap_tolerance_mm = 10.0001", false),
        ("snap_tolerance_mm = -0.1", false),
        ("snap_tolerance_mm = nan", false),
        ("max_layers = 1", true),
        ("max_layers = 0", false),
        ("max_layers = 1024", true),
        ("max_layers = 1025", false),
        ("max_layers = -1", false),
        ("max_gap_pairs = 1", true),
        ("max_gap_pairs = 100000000", true),
        ("max_gap_pairs = 100000001", false),
        ("min_visible_opacity = 1", true),
        ("min_visible_opacity = 1.01", false),
        ("default_opacity = 0", true),
        ("unit = mil", true),
        ("unit = cm", false),
        ("snap_vertices = off", true),
        ("snap_vertices = Yes", false),
    ] {
        assert_eq!(Settings::parse(text).is_ok(), ok, "{text}");
    }
}

#[test]
fn a_mistake_in_the_settings_names_its_line_and_a_comment_or_a_mark_is_not_a_mistake() {
    let why = Settings::parse("max_layers = 5\nmax_layers = 6\n").unwrap_err();
    assert!(
        why.contains("line 2") && why.contains("written twice"),
        "{why}"
    );
    let why = Settings::parse("# a\n\noops\n").unwrap_err();
    assert!(
        why.contains("line 3") && why.contains("not key = value"),
        "{why}"
    );
    let why = Settings::parse("flavour = 1\n").unwrap_err();
    assert!(why.contains("unknown setting flavour"), "{why}");
    let s = Settings::parse("\u{feff}max_layers = 7 # seven\n").unwrap();
    assert_eq!(s.max_layers, 7);
    assert_eq!(Settings::parse("").unwrap(), Settings::default());
    assert_eq!(
        Settings::parse("# nothing\n\n   \n").unwrap(),
        Settings::default()
    );
}

#[test]
fn a_configuration_file_of_exactly_the_most_bytes_is_read_and_one_more_is_refused() {
    let dir = scratch("cfg");
    let file = dir.join("gerberview.cfg");
    fs::write(&file, "#".repeat(MAX_CFG_BYTES)).unwrap();
    assert_eq!(Settings::load(&file).unwrap(), Settings::default());
    fs::write(&file, "#".repeat(MAX_CFG_BYTES + 1)).unwrap();
    assert!(Settings::load(&file)
        .unwrap_err()
        .contains("above 65536 bytes"));
    assert!(Settings::load(&dir.join("none.cfg")).is_err());
    fs::write(&file, "max_layers = 3\n").unwrap();
    assert_eq!(Settings::load(&file).unwrap().max_layers, 3);
}

// ---------------------------------------------------------------- the stack of layers

fn stack_layer(name: &str, opacity: f64) -> StackLayer {
    StackLayer {
        name: name.to_string(),
        rgb: [200, 100, 50],
        opacity,
        visible: true,
    }
}

fn names(stack: &Stack) -> Vec<&str> {
    stack.layers().iter().map(|l| l.name.as_str()).collect()
}

#[test]
fn a_stack_refuses_a_layer_that_has_no_name_a_wrong_opacity_or_one_more_than_its_limit() {
    let settings = Settings {
        max_layers: 2,
        ..Settings::default()
    };
    let mut stack = Stack::new(&settings);
    assert_eq!(stack.push(stack_layer("a", 0.0)), Ok(0));
    assert!(stack
        .push(stack_layer("  ", 0.5))
        .unwrap_err()
        .contains("needs a name"));
    for bad in [-0.1, 1.0001, f64::NAN, f64::INFINITY] {
        assert!(stack
            .push(stack_layer("b", bad))
            .unwrap_err()
            .contains("from 0 to 1"));
    }
    assert_eq!(stack.push(stack_layer("b", 1.0)), Ok(1));
    assert!(stack
        .push(stack_layer("c", 0.5))
        .unwrap_err()
        .contains("the most there can be"));
    assert_eq!(names(&stack), vec!["a", "b"]);
}

#[test]
fn a_change_that_is_refused_leaves_the_stack_as_it_was_and_a_move_keeps_the_others_in_order() {
    let mut stack = Stack::new(&Settings::default());
    for name in ["a", "b", "c", "d"] {
        stack.push(stack_layer(name, 0.5)).unwrap();
    }
    let before = stack.clone();
    assert!(stack.set_opacity(0, 2.0).is_err());
    assert!(stack
        .set_opacity(9, 0.5)
        .unwrap_err()
        .contains("there is no layer 9 (the stack has 4)"));
    assert!(stack.set_visible(4, false).is_err());
    assert!(stack
        .move_to(0, 4)
        .unwrap_err()
        .contains("there is no layer 4"));
    assert!(stack.move_to(7, 0).is_err());
    assert_eq!(stack, before);
    stack.move_to(0, 2).unwrap();
    assert_eq!(names(&stack), vec!["b", "c", "a", "d"]);
    stack.move_to(3, 0).unwrap();
    assert_eq!(names(&stack), vec!["d", "b", "c", "a"]);
    stack.move_to(1, 1).unwrap();
    assert_eq!(names(&stack), vec!["d", "b", "c", "a"]);
}

fn boxed(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
    Bounds {
        min_x: x0,
        min_y: y0,
        max_x: x1,
        max_y: y1,
    }
}

#[test]
fn a_layer_is_almost_invisible_below_the_minimum_and_not_at_it_and_a_hidden_one_is_not_told() {
    let settings = Settings::default();
    let mut stack = Stack::new(&settings);
    stack.push(stack_layer("low", 0.049)).unwrap();
    stack
        .push(stack_layer("edge", settings.min_visible_opacity))
        .unwrap();
    stack.push(stack_layer("off", 0.0)).unwrap();
    stack.set_visible(2, false).unwrap();
    let boxes = [
        boxed(0.0, 0.0, 1.0, 1.0),
        boxed(5.0, 5.0, 6.0, 6.0),
        boxed(9.0, 9.0, 10.0, 10.0),
    ];
    let warnings = stack.warnings(&boxes, &settings).unwrap();
    assert_eq!(
        warnings,
        vec![Warning::AlmostInvisible {
            layer: 0,
            opacity: 0.049
        }]
    );
    let few = stack.warnings(&boxes[..2], &settings).unwrap_err();
    assert!(few.contains("2 boxes for 3 layers"), "{few}");
}

#[test]
fn a_layer_may_be_hidden_only_by_a_solid_visible_layer_above_whose_box_contains_it() {
    let settings = Settings::default();
    let inner = boxed(2.0, 2.0, 3.0, 3.0);
    let outer = boxed(0.0, 0.0, 10.0, 10.0);
    let build = |opacity: f64, visible: bool| {
        let mut stack = Stack::new(&settings);
        stack.push(stack_layer("under", 0.8)).unwrap();
        stack.push(stack_layer("over", opacity)).unwrap();
        stack.set_visible(1, visible).unwrap();
        stack.warnings(&[inner, outer], &settings).unwrap()
    };
    assert_eq!(
        build(1.0, true),
        vec![Warning::MayBeHidden { layer: 0, by: 1 }]
    );
    assert!(build(0.99, true).is_empty());
    assert!(build(1.0, false).is_empty());
    let mut reversed = Stack::new(&settings);
    reversed.push(stack_layer("under", 0.8)).unwrap();
    reversed.push(stack_layer("over", 1.0)).unwrap();
    assert!(reversed
        .warnings(&[outer, inner], &settings)
        .unwrap()
        .is_empty());
}

#[test]
fn the_picture_of_a_stack_has_one_pixel_for_each_place_and_a_hidden_layer_leaves_the_background() {
    let mut stack = Stack::new(&Settings::default());
    stack.push(stack_layer("copper", 1.0)).unwrap();
    let scenes = [scene(vec![Shape::Rect {
        x: 5.0,
        y: 5.0,
        w: 10.0,
        h: 10.0,
    }])];
    let area = [0.0, 0.0, 10.0, 10.0];
    let background = [0.0, 0.0, 0.0];
    let picture = stack.render(&scenes, area, 4, 2, background).unwrap();
    assert_eq!(picture.len(), 8);
    assert!(picture.iter().all(|p| *p == picture[0] && *p != background));
    stack.set_visible(0, false).unwrap();
    let hidden = stack.render(&scenes, area, 4, 2, background).unwrap();
    assert!(hidden.iter().all(|p| *p == background));
    assert!(stack
        .render(&scenes, area, 0, 2, background)
        .unwrap_err()
        .contains("from 1 to 2048"));
    assert!(stack.render(&scenes, area, 2, 2049, background).is_err());
    assert!(stack
        .render(&scenes, [0.0, 0.0, 0.0, 10.0], 2, 2, background)
        .is_err());
    assert!(stack
        .render(&scenes, [0.0, 0.0, f64::NAN, 1.0], 2, 2, background)
        .is_err());
}

// ---------------------------------------------------------------- the gaps

#[test]
fn the_gap_between_two_layers_is_the_closest_pair_and_zero_when_they_touch_or_overlap() {
    let a = scene(vec![disc(0.0, 0.0, 1.0)]);
    let far = scene(vec![disc(5.0, 0.0, 1.0)]);
    let report = layer_gap(&a, &far, 10).unwrap();
    let best = report.best.unwrap();
    assert!(near(best.distance, 3.0));
    assert_eq!((best.a_index, best.b_index), (0, 0));
    assert_eq!((best.a_kind, best.b_kind), ("round pad", "round pad"));
    let touching = scene(vec![disc(2.0, 0.0, 1.0)]);
    assert!(near(
        layer_gap(&a, &touching, 10).unwrap().best.unwrap().distance,
        0.0
    ));
    let over = scene(vec![disc(0.5, 0.0, 1.0)]);
    assert_eq!(
        layer_gap(&a, &over, 10).unwrap().best.unwrap().distance,
        0.0
    );
    assert!(near(
        shape_gap(&disc(0.0, 0.0, 1.0), &disc(5.0, 0.0, 1.0)).unwrap(),
        3.0
    ));
}

#[test]
fn the_number_of_pairs_is_counted_before_the_work_and_the_limit_itself_is_allowed() {
    let one = scene(vec![disc(0.0, 0.0, 1.0)]);
    let why = layer_gap(&one, &one, 0).unwrap_err();
    assert!(why.contains("1 pairs of shapes: more than the 0"), "{why}");
    assert!(layer_gap(&one, &one, 1).is_ok());
    let three = scene(vec![
        disc(0.0, 0.0, 1.0),
        disc(10.0, 0.0, 1.0),
        disc(20.0, 0.0, 1.0),
    ]);
    assert!(scene_gap(&three, 2).unwrap_err().contains("3 pairs"));
    let report = scene_gap(&three, 3).unwrap();
    assert!(near(report.best.unwrap().distance, 8.0));
}

#[test]
fn a_shape_that_cannot_be_measured_is_counted_as_skipped_and_never_hidden() {
    let empty = Scene::default();
    let report = layer_gap(&empty, &empty, 10).unwrap();
    assert!(report.best.is_none() && report.skipped == 0);
    let broken = scene(vec![disc(0.0, 0.0, f64::NAN), disc(1.0, 0.0, -1.0)]);
    let ok = scene(vec![disc(0.0, 0.0, 1.0)]);
    let report = layer_gap(&broken, &ok, 10).unwrap();
    assert!(report.best.is_none());
    assert_eq!(report.skipped, 2);
    assert!(shape_gap(&disc(0.0, 0.0, f64::NAN), &disc(0.0, 0.0, 1.0))
        .unwrap_err()
        .contains("round pad"));
    assert!(scene_gap(&ok, 10).unwrap().best.is_none());
}

// ---------------------------------------------------------------- the ruler

#[test]
fn a_unit_is_read_in_any_case_and_a_length_in_millimetres_is_written_in_it() {
    for (text, unit) in [
        ("mm", Some(Unit::Mm)),
        (" MM ", Some(Unit::Mm)),
        ("in", Some(Unit::Inch)),
        ("Inch", Some(Unit::Inch)),
        ("mil", Some(Unit::Mil)),
        ("THOU", Some(Unit::Mil)),
        ("cm", None),
        ("", None),
    ] {
        assert_eq!(Unit::parse(text), unit, "{text:?}");
    }
    assert_eq!(
        (Unit::Mm.label(), Unit::Inch.label(), Unit::Mil.label()),
        ("mm", "in", "mil")
    );
    assert_eq!(Unit::Mm.from_mm(7.5), 7.5);
    assert_eq!(Unit::Inch.from_mm(25.4), 1.0);
    assert_eq!(Unit::Mil.from_mm(0.0254), 1.0);
}

#[test]
fn a_click_is_caught_at_the_exact_tolerance_and_not_beyond_it_and_a_tie_goes_to_the_pad_centre() {
    let settings = Settings::default();
    let layer = scene(vec![disc(0.0, 0.0, 1.0)]);
    let caught = snap(&layer, (0.15, 0.0), &settings).unwrap();
    assert_eq!(
        (caught.kind, caught.shape, caught.what),
        (SnapKind::PadCentre, Some(0), Some("round pad"))
    );
    let free = snap(&layer, (0.1501, 0.0), &settings).unwrap();
    assert_eq!(
        (free.kind, free.point, free.shape),
        (SnapKind::Free, (0.1501, 0.0), None)
    );
    assert!(snap(&layer, (f64::NAN, 0.0), &settings).is_err());
    assert!(snap(&layer, (0.0, f64::INFINITY), &settings).is_err());
    let rect = scene(vec![Shape::Rect {
        x: 0.0,
        y: 0.0,
        w: 0.2,
        h: 0.2,
    }]);
    let tie = snap(&rect, (0.05, 0.05), &settings).unwrap();
    assert_eq!(tie.kind, SnapKind::PadCentre);
    let no_centres = Settings {
        snap_pad_centres: false,
        ..Settings::default()
    };
    assert_eq!(
        snap(&rect, (0.05, 0.05), &no_centres).unwrap().kind,
        SnapKind::Vertex
    );
    let nothing = Settings {
        snap_pad_centres: false,
        snap_vertices: false,
        ..Settings::default()
    };
    assert_eq!(
        snap(&rect, (0.05, 0.05), &nothing).unwrap().kind,
        SnapKind::Free
    );
    let twins = scene(vec![disc(1.0, 1.0, 0.5), disc(1.0, 1.0, 0.5)]);
    assert_eq!(snap(&twins, (1.0, 1.0), &settings).unwrap().shape, Some(0));
}

fn free(x: f64, y: f64) -> Snapped {
    Snapped {
        point: (x, y),
        kind: SnapKind::Free,
        shape: None,
        what: None,
    }
}

#[test]
fn the_angle_of_a_measure_goes_anticlockwise_from_zero_up_to_below_three_hundred_sixty() {
    for (to, angle) in [
        ((1.0, 0.0), 0.0),
        ((1.0, 1.0), 45.0),
        ((0.0, 1.0), 90.0),
        ((-1.0, 0.0), 180.0),
        ((-1.0, -1.0), 225.0),
        ((0.0, -1.0), 270.0),
        ((0.0, 0.0), 0.0),
    ] {
        let m = measure_points(free(0.0, 0.0), free(to.0, to.1), Unit::Mm);
        assert!(near(m.angle_deg, angle), "{to:?}: {}", m.angle_deg);
        assert!((0.0..360.0).contains(&m.angle_deg));
    }
    let mut rng = Lcg(33);
    for _ in 0..200 {
        let (x, y) = (rng.between(-50.0, 50.0), rng.between(-50.0, 50.0));
        let m = measure_points(free(0.0, 0.0), free(x, y), Unit::Mm);
        assert!((0.0..360.0).contains(&m.angle_deg));
        assert!(near(m.distance, x.hypot(y)));
    }
}

#[test]
fn a_measure_is_told_in_three_lines_with_the_decimals_of_its_unit() {
    let empty = Scene::default();
    let m = ruler(&empty, (0.0, 0.0), (3.0, 4.0), &Settings::default()).unwrap();
    assert_eq!(
        m.lines(),
        vec![
            "from (0.000, 0.000) free".to_string(),
            "to (3.000, 4.000) free".to_string(),
            "distance 5.000 mm  dx +3.000 mm  dy +4.000 mm  angle 53.13 deg".to_string(),
        ]
    );
    let inches = Settings {
        unit: Unit::Inch,
        ..Settings::default()
    };
    let m = ruler(&empty, (0.0, 0.0), (25.4, 0.0), &inches).unwrap();
    assert!(m.lines()[2].starts_with("distance 1.00000 in  dx +1.00000 in  dy +0.00000 in"));
    let mils = Settings {
        unit: Unit::Mil,
        ..Settings::default()
    };
    let m = ruler(&empty, (0.0, 0.0), (0.0254, 0.0), &mils).unwrap();
    assert!(m.lines()[2].starts_with("distance 1.00 mil"));
    assert!(ruler(&empty, (f64::NAN, 0.0), (1.0, 1.0), &Settings::default()).is_err());
}

// ---------------------------------------------------------------- the words and the files

#[test]
fn a_file_is_a_drill_file_by_its_header_and_not_by_its_name() {
    for (text, drill) in [
        ("M48\n", true),
        ("m48\r\n", true),
        ("; made by a program\n\n  M48  \n", true),
        ("\u{feff}M48,METRIC\n", true),
        ("M48;made by\n", true),
        ("M48 METRIC\n", true),
        ("M481\n", false),
        ("M4\n", false),
        ("M48X\n", false),
        ("G04 not a drill*\n%FSLAX24Y24*%\n", false),
        ("", false),
        ("; only a comment\n", false),
        ("T01C0.8\nM48\n", false),
    ] {
        assert_eq!(looks_like_drill(text), drill, "{text:?}");
    }
}

#[test]
fn the_eight_languages_have_their_own_code_and_name_and_come_back_from_their_code() {
    let codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
    assert_eq!(codes, vec!["en", "es", "de", "fr", "it", "ja", "zh", "pt"]);
    for (i, a) in Lang::ALL.iter().enumerate() {
        assert_eq!(Lang::from_code(a.code()), Some(*a));
        assert_eq!(Lang::from_code(&a.code().to_uppercase()), Some(*a));
        for b in &Lang::ALL[i + 1..] {
            assert_ne!(a.name(), b.name());
        }
    }
    assert_eq!(Lang::from_code("zh_CN.UTF-8"), Some(Lang::Zh));
    assert_eq!(Lang::from_code("e"), None);
    assert_eq!(Lang::from_code("\u{65E5}\u{672C}"), None);
}

#[test]
fn no_sentence_starts_or_ends_with_a_blank_and_the_japanese_and_the_chinese_use_their_script() {
    let total = Msg::ALL.len();
    assert!(total >= 40);
    for m in Msg::ALL {
        for l in Lang::ALL {
            let text = m.tr(l);
            assert_eq!(text, text.trim(), "{m:?} in {}", l.code());
        }
    }
    for lang in [Lang::Ja, Lang::Zh] {
        let own = Msg::ALL.iter().filter(|m| !m.tr(lang).is_ascii()).count();
        assert!(own * 10 >= total * 7, "{} {own} of {total}", lang.code());
    }
}

#[test]
fn filling_a_text_leaves_a_place_with_no_value_and_ignores_a_value_with_no_place() {
    assert_eq!(fill("{0}-{1}", &["a"]), "a-{1}");
    assert_eq!(fill("{0}{0}", &["ab"]), "abab");
    assert_eq!(fill("", &["a"]), "");
    assert_eq!(fill("no place", &[]), "no place");
}

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_eh3d-gerberview"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the program runs");
    (
        out.status.code().expect("the program ends by itself"),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_picture_written_twice_from_the_same_layers_is_the_same_file() {
    let dir = scratch("render");
    let (one, two) = (dir.join("one.svg"), dir.join("two.svg"));
    for out in [&one, &two] {
        let (code, said, why) = run(&[
            "render",
            &out.display().to_string(),
            "examples/sample.gtl",
            "examples/sample.ncd",
        ]);
        assert_eq!(code, 0, "{why}");
        assert!(said.starts_with("2 layers drawn in "), "{said}");
    }
    let (a, b) = (
        fs::read_to_string(&one).unwrap(),
        fs::read_to_string(&two).unwrap(),
    );
    assert_eq!(a, b);
    assert!(a.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(a.trim_end().ends_with("</svg>"));
    assert_eq!(a.matches("<g id=\"").count(), 2);
}

#[test]
fn a_picture_that_would_be_written_over_a_layer_or_with_a_wrong_opacity_is_refused_and_nothing_is_written(
) {
    let (code, _, why) = run(&["render", "examples/sample.gtl", "examples/sample.gtl"]);
    assert_ne!(code, 0);
    assert!(why.contains("looks like a layer"), "{why}");
    let dir = scratch("refused");
    let out = dir.join("never.svg").display().to_string();
    let (code, _, why) = run(&["render", &out, "examples/sample.gtl", "--opacity", "1.5"]);
    assert_ne!(code, 0);
    assert!(why.contains("must be from 0 to 1"), "{why}");
    assert!(!dir.join("never.svg").exists());
}

#[test]
fn the_same_layer_checked_against_itself_lines_up_and_the_information_names_it() {
    let (code, said, why) = run(&["check", "examples/sample.gtl", "examples/sample.gtl"]);
    assert_eq!(code, 0, "{why}");
    assert!(said.contains("the layers line up"), "{said}");
    let (code, said, _) = run(&["info", "examples/sample.gtl"]);
    assert_eq!(code, 0);
    assert!(said.starts_with("sample.gtl: "), "{said}");
    assert!(said.contains(" shapes, "), "{said}");
}

#[test]
fn the_ruler_on_two_free_points_prints_its_three_lines_and_a_far_snap_tolerance_is_refused() {
    let (code, said, why) = run(&[
        "ruler",
        "examples/sample.gtl",
        "1000",
        "1000",
        "1003",
        "1004",
    ]);
    assert_eq!(code, 0, "{why}");
    assert!(
        said.contains("distance 5.000 mm  dx +3.000 mm  dy +4.000 mm  angle 53.13 deg"),
        "{said}"
    );
    assert_eq!(said.lines().count(), 3, "{said}");
    let (code, _, why) = run(&[
        "ruler",
        "examples/sample.gtl",
        "0",
        "0",
        "1",
        "1",
        "--snap-tolerance",
        "10.5",
    ]);
    assert_ne!(code, 0);
    assert!(why.contains("from 0 to 10"), "{why}");
}
