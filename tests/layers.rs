//! The stack of layers: an opacity for each, bounded; an order of composition; warnings; and a software picture in which a layer is painted as a whole, so the opacity does not add up
//! where two of its shapes overlap.

use eh3d_gerberviewer::layers::{Layer, Stack, Warning};
use eh3d_gerberviewer::scene::{Bounds, Scene, Shape};
use eh3d_gerberviewer::settings::Settings;

const BLACK: [f64; 3] = [0.0, 0.0, 0.0];

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn layer(name: &str, rgb: [u8; 3], opacity: f64) -> Layer {
    Layer {
        name: name.to_string(),
        rgb,
        opacity,
        visible: true,
    }
}

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

#[test]
fn the_opacity_of_a_layer_does_not_add_up_where_its_shapes_overlap() {
    let mut stack = Stack::new(&Settings::default());
    stack.push(layer("top", [200, 100, 0], 0.5)).unwrap();
    // two discs of one layer that overlap in the middle
    let scenes = [scene(vec![disc(0.0, 0.0, 2.0), disc(1.0, 0.0, 2.0)])];
    let in_one = stack.sample(&scenes, (-1.5, 0.0), BLACK);
    let in_both = stack.sample(&scenes, (0.5, 0.0), BLACK);
    assert!(close(in_one[0], 100.0) && close(in_one[1], 50.0));
    // as a whole the overlap is the same as the rest of the layer
    assert_eq!(in_one, in_both);
    // painted shape by shape it would be 150 against 100: the difference the whole painting avoids
    let each = stack.sample_per_shape(&scenes, (0.5, 0.0), BLACK);
    assert!(close(each[0], 150.0), "{each:?}");
    assert!(close(
        stack.sample_per_shape(&scenes, (-1.5, 0.0), BLACK)[0],
        100.0
    ));
}

#[test]
fn a_layer_above_covers_the_one_below_by_its_opacity_and_the_order_can_change() {
    let mut stack = Stack::new(&Settings::default());
    stack.push(layer("copper", [255, 0, 0], 1.0)).unwrap();
    stack.push(layer("mask", [0, 0, 255], 0.25)).unwrap();
    let scenes = [
        scene(vec![disc(0.0, 0.0, 3.0)]),
        scene(vec![disc(0.0, 0.0, 3.0)]),
    ];
    let c = stack.sample(&scenes, (0.0, 0.0), BLACK);
    // blue a quarter over solid red
    assert!(close(c[0], 191.25) && close(c[2], 63.75), "{c:?}");
    // the mask below the copper is hidden by the solid copper
    stack.move_to(1, 0).unwrap();
    assert_eq!(stack.layers()[0].name, "mask");
    let scenes = [scenes[1].clone(), scenes[0].clone()];
    let c = stack.sample(&scenes, (0.0, 0.0), BLACK);
    assert!(close(c[0], 255.0) && close(c[2], 0.0), "{c:?}");
}

#[test]
fn a_layer_that_is_hidden_or_has_no_opacity_leaves_the_picture_as_it_was() {
    let mut stack = Stack::new(&Settings::default());
    stack.push(layer("a", [255, 255, 255], 1.0)).unwrap();
    stack.push(layer("b", [255, 255, 255], 0.0)).unwrap();
    let scenes = [scene(vec![]), scene(vec![disc(0.0, 0.0, 1.0)])];
    assert_eq!(stack.sample(&scenes, (0.0, 0.0), BLACK), BLACK);
    stack.set_opacity(1, 1.0).unwrap();
    assert_eq!(stack.sample(&scenes, (0.0, 0.0), BLACK), [255.0; 3]);
    stack.set_visible(1, false).unwrap();
    assert_eq!(stack.sample(&scenes, (0.0, 0.0), BLACK), BLACK);
    // a point outside every shape is the background
    stack.set_visible(1, true).unwrap();
    assert_eq!(stack.sample(&scenes, (9.0, 9.0), BLACK), BLACK);
}

#[test]
fn an_opacity_outside_0_to_1_or_not_a_number_is_refused_and_the_layer_stays() {
    let mut stack = Stack::new(&Settings::default());
    for bad in [-0.01, 1.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(stack.push(layer("x", [0, 0, 0], bad)).is_err(), "{bad}");
    }
    assert!(stack.layers().is_empty());
    stack.push(layer("x", [0, 0, 0], 0.5)).unwrap();
    for bad in [-1.0, 2.0, f64::NAN] {
        let e = stack.set_opacity(0, bad).unwrap_err();
        assert!(e.contains("must be from 0 to 1"), "{e}");
        assert_eq!(stack.layers()[0].opacity, 0.5);
    }
    // the ends are valid
    stack.set_opacity(0, 0.0).unwrap();
    stack.set_opacity(0, 1.0).unwrap();
    assert!(stack
        .set_opacity(3, 0.5)
        .unwrap_err()
        .contains("no layer 3"));
    assert!(stack.push(layer("  ", [0, 0, 0], 0.5)).is_err());
}

#[test]
fn the_stack_has_a_limit_of_layers_and_moves_are_checked() {
    let settings = Settings {
        max_layers: 2,
        ..Settings::default()
    };
    let mut stack = Stack::new(&settings);
    stack.push(layer("a", [0, 0, 0], 1.0)).unwrap();
    stack.push(layer("b", [0, 0, 0], 1.0)).unwrap();
    let e = stack.push(layer("c", [0, 0, 0], 1.0)).unwrap_err();
    assert!(e.contains("max_layers"), "{e}");
    assert!(stack.move_to(0, 2).is_err() && stack.move_to(5, 0).is_err());
    stack.move_to(0, 1).unwrap();
    let names: Vec<&str> = stack.layers().iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["b", "a"]);
}

fn bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
    let mut b = Bounds::empty();
    b.include(x0, y0);
    b.include(x1, y1);
    b
}

#[test]
fn a_composition_that_hides_a_layer_or_leaves_it_almost_invisible_is_warned() {
    let settings = Settings::default();
    let mut stack = Stack::new(&settings);
    stack.push(layer("silk", [255, 255, 255], 0.02)).unwrap();
    stack.push(layer("copper", [200, 0, 0], 0.8)).unwrap();
    stack.push(layer("mask", [0, 100, 0], 1.0)).unwrap();
    let boxes = [
        bounds(1.0, 1.0, 3.0, 3.0),
        bounds(0.0, 0.0, 10.0, 10.0),
        bounds(0.0, 0.0, 10.0, 10.0),
    ];
    let w = stack.warnings(&boxes, &settings).unwrap();
    assert_eq!(
        w,
        [
            Warning::AlmostInvisible {
                layer: 0,
                opacity: 0.02
            },
            Warning::MayBeHidden { layer: 0, by: 2 },
            Warning::MayBeHidden { layer: 1, by: 2 },
        ]
    );
    // hide the solid layer and both go away, except the almost invisible one
    stack.set_visible(2, false).unwrap();
    let w = stack.warnings(&boxes, &settings).unwrap();
    assert_eq!(w.len(), 1);
    // a layer with nothing in it is never warned, and the boxes must match the layers
    let empty = [Bounds::empty(), boxes[1], boxes[2]];
    assert!(stack.warnings(&empty, &settings).unwrap().is_empty());
    assert!(stack.warnings(&boxes[..2], &settings).is_err());
}

#[test]
fn a_small_picture_of_the_stack_has_the_colour_of_each_pixel_centre() {
    let mut stack = Stack::new(&Settings::default());
    stack.push(layer("pad", [100, 200, 50], 1.0)).unwrap();
    // a rectangle that covers the left half of a 4 x 2 area
    let scenes = [scene(vec![Shape::Rect {
        x: 1.0,
        y: 1.0,
        w: 2.0,
        h: 2.0,
    }])];
    let img = stack
        .render(&scenes, [0.0, 0.0, 4.0, 2.0], 4, 2, BLACK)
        .unwrap();
    let on = [100.0, 200.0, 50.0];
    assert_eq!(img, [on, on, BLACK, BLACK, on, on, BLACK, BLACK]);
    // sizes and areas are checked
    assert!(stack
        .render(&scenes, [0.0, 0.0, 4.0, 2.0], 0, 2, BLACK)
        .is_err());
    assert!(stack
        .render(&scenes, [0.0, 0.0, 4.0, 2.0], 5000, 2, BLACK)
        .is_err());
    assert!(stack
        .render(&scenes, [0.0, 0.0, 0.0, 2.0], 4, 2, BLACK)
        .is_err());
    assert!(stack
        .render(&scenes, [0.0, f64::NAN, 4.0, 2.0], 4, 2, BLACK)
        .is_err());
}

#[test]
fn the_picture_of_a_seeded_stack_is_the_same_every_time_and_stays_between_the_colours() {
    let mut state = 0x0bad_5eed_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut stack = Stack::new(&Settings::default());
    let mut scenes = Vec::new();
    for i in 0..4 {
        let opacity = (next() % 101) as f64 / 100.0;
        stack
            .push(layer(
                &format!("l{i}"),
                [(next() % 256) as u8, 40, 255],
                opacity,
            ))
            .unwrap();
        scenes.push(scene(
            (0..3)
                .map(|_| {
                    disc(
                        (next() % 10) as f64,
                        (next() % 10) as f64,
                        1.0 + (next() % 30) as f64 / 10.0,
                    )
                })
                .collect(),
        ));
    }
    let a = stack
        .render(&scenes, [0.0, 0.0, 10.0, 10.0], 24, 24, BLACK)
        .unwrap();
    let b = stack
        .render(&scenes, [0.0, 0.0, 10.0, 10.0], 24, 24, BLACK)
        .unwrap();
    assert_eq!(a, b);
    for px in &a {
        assert!(px.iter().all(|v| (0.0..=255.0).contains(v)), "{px:?}");
    }
    // both ways of painting give colours that are numbers
    for (x, y) in [(2.5, 2.5), (5.5, 4.5), (7.5, 7.5)] {
        let whole = stack.sample(&scenes, (x, y), BLACK);
        let each = stack.sample_per_shape(&scenes, (x, y), BLACK);
        assert!(whole
            .iter()
            .zip(each)
            .all(|(w, e)| w.is_finite() && e.is_finite()));
    }
}
