//! Do the layers of a board line up? The boxes of the layers are compared, and the holes of the drill file must lie inside the copper.

use crate::scene::{Bounds, Scene, Shape};

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub layer: String,
    pub message: String,
}

/// Compares the centre and the size of every layer with the first one. `tolerance_mm` is how much a centre may differ.
pub fn compare_boxes(layers: &[(String, Bounds)], tolerance_mm: f64) -> Vec<Finding> {
    let mut out = Vec::new();
    let Some((first_name, first)) = layers.iter().find(|(_, b)| !b.is_empty()) else {
        return out;
    };
    for (name, b) in layers {
        if b.is_empty() {
            out.push(Finding {
                layer: name.clone(),
                message: "has nothing to draw".into(),
            });
            continue;
        }
        if name == first_name {
            continue;
        }
        let (c0, c1) = (first.center(), b.center());
        let (dx, dy) = (c1.0 - c0.0, c1.1 - c0.1);
        if dx.abs() > tolerance_mm || dy.abs() > tolerance_mm {
            out.push(Finding { layer: name.clone(), message: format!("its centre is {dx:+.3} mm in x and {dy:+.3} mm in y from the centre of {first_name} (more than {tolerance_mm} mm)") });
        }
    }
    out
}

/// The holes (discs) of `holes` that lie outside the box of `copper`, as one finding each; a layer of copper with nothing in it makes every hole a finding.
pub fn holes_outside(copper: &Scene, holes: &Scene, layer: &str) -> Vec<Finding> {
    let b = copper.bounds();
    holes
        .shapes
        .iter()
        .filter_map(|s| {
            if let Shape::Disc { x, y, r } = s {
                Some((*x, *y, *r))
            } else {
                None
            }
        })
        .filter(|(x, y, r)| {
            b.is_empty() || x - r < b.min_x || x + r > b.max_x || y - r < b.min_y || y + r > b.max_y
        })
        .map(|(x, y, r)| Finding {
            layer: layer.to_string(),
            message: format!(
                "the hole of {:.2} mm at ({x:.3}, {y:.3}) is not inside the copper",
                r * 2.0
            ),
        })
        .collect()
}
