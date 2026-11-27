//! Do the layers of a board line up? The boxes of the layers are compared, and the holes of the drill file must lie inside the copper.

use crate::scene::{Bounds, Scene, Shape};

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub layer: String,
    pub message: String,
}

/// Compares the centre of every layer with the centre of the first one (the sizes are not compared). `tolerance_mm` is how much a centre may differ.
pub fn compare_boxes(layers: &[(String, Bounds)], tolerance_mm: f64) -> Vec<Finding> {
    let mut out = Vec::new();
    let Some((first_at, (first_name, first))) =
        layers.iter().enumerate().find(|(_, (_, b))| !b.is_empty())
    else {
        return out;
    };
    for (at, (name, b)) in layers.iter().enumerate() {
        if b.is_empty() {
            out.push(Finding {
                layer: name.clone(),
                message: "has nothing to draw".into(),
            });
            continue;
        }
        // the reference is skipped by its place in the list: two layers can have the same name
        if at == first_at {
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

/// The layers (copper, mask, outline...) when none of them has anything to draw: `compare_boxes` has no box to compare then and would find nothing wrong with them.
pub fn all_empty(layers: &[(String, Bounds)]) -> Vec<Finding> {
    if layers.iter().any(|(_, b)| !b.is_empty()) {
        return Vec::new();
    }
    layers
        .iter()
        .map(|(name, _)| Finding {
            layer: name.clone(),
            message: "has nothing to draw".into(),
        })
        .collect()
}

/// A drill file with neither a hole nor a slot: there is nothing that could be inside the copper.
pub fn no_holes(holes: &Scene, layer: &str) -> Option<Finding> {
    let any = holes
        .shapes
        .iter()
        .any(|s| matches!(s, Shape::Disc { .. } | Shape::Stroke { .. }));
    (!any).then(|| Finding {
        layer: layer.to_string(),
        message: "has no hole and no slot to check".into(),
    })
}

/// The slots (the strokes of a drill file) that lie outside the box of `copper`, as one finding each.
pub fn slots_outside(copper: &Scene, holes: &Scene, layer: &str) -> Vec<Finding> {
    let b = copper.bounds();
    holes
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Stroke { x0, y0, x1, y1, w } => Some((*x0, *y0, *x1, *y1, *w)),
            _ => None,
        })
        .filter(|(x0, y0, x1, y1, w)| {
            let r = w / 2.0;
            b.is_empty()
                || x0.min(*x1) - r < b.min_x
                || x0.max(*x1) + r > b.max_x
                || y0.min(*y1) - r < b.min_y
                || y0.max(*y1) + r > b.max_y
        })
        .map(|(x0, y0, x1, y1, w)| Finding {
            layer: layer.to_string(),
            message: format!(
                "the slot of {w:.2} mm from ({x0:.3}, {y0:.3}) to ({x1:.3}, {y1:.3}) is not inside the copper"
            ),
        })
        .collect()
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
