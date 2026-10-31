//! Measures on a layer: the distance between two points, the narrowest track and the smallest gap between tracks and round pads.

use crate::scene::{Scene, Shape};

/// The most shapes `min_clearance` compares (it looks at every pair).
pub const MAX_PAIRS_SHAPES: usize = 3000;

pub fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The narrowest track of the layer, or `None` without tracks.
pub fn min_track_width(scene: &Scene) -> Option<f64> {
    scene
        .shapes
        .iter()
        .filter_map(|s| {
            if let Shape::Stroke { w, .. } = s {
                Some(*w)
            } else {
                None
            }
        })
        .fold(None, |m: Option<f64>, w| Some(m.map_or(w, |x| x.min(w))))
}

/// The centres of the flashed pads, in the order of the file.
pub fn pad_centres(scene: &Scene) -> Vec<(f64, f64)> {
    scene
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Disc { x, y, .. } | Shape::Rect { x, y, .. } | Shape::Obround { x, y, .. } => {
                Some((*x, *y))
            }
            _ => None,
        })
        .collect()
}

/// The distance between the centres of pad `i` and pad `j` (a number of the list of `pad_centres`).
pub fn centre_distance(scene: &Scene, i: usize, j: usize) -> Result<f64, String> {
    let c = pad_centres(scene);
    let get = |n: usize| {
        c.get(n)
            .copied()
            .ok_or_else(|| format!("there is no pad {n} (the layer has {})", c.len()))
    };
    Ok(distance(get(i)?, get(j)?))
}

/// Two end points and a radius.
type Capsule = ((f64, f64), (f64, f64), f64);

fn capsule(s: &Shape) -> Option<Capsule> {
    match s {
        Shape::Disc { x, y, r } => Some(((*x, *y), (*x, *y), *r)),
        Shape::Stroke { x0, y0, x1, y1, w } => Some(((*x0, *y0), (*x1, *y1), w / 2.0)),
        _ => None,
    }
}

fn segment_distance(a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64))) -> f64 {
    fn point_segment(p: (f64, f64), s: ((f64, f64), (f64, f64))) -> f64 {
        let (dx, dy) = (s.1 .0 - s.0 .0, s.1 .1 - s.0 .1);
        let len2 = dx * dx + dy * dy;
        if len2 == 0.0 {
            return distance(p, s.0);
        }
        let t = (((p.0 - s.0 .0) * dx + (p.1 - s.0 .1) * dy) / len2).clamp(0.0, 1.0);
        distance(p, (s.0 .0 + t * dx, s.0 .1 + t * dy))
    }
    fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    }
    let crossing = cross(a.0, a.1, b.0) * cross(a.0, a.1, b.1) < 0.0
        && cross(b.0, b.1, a.0) * cross(b.0, b.1, a.1) < 0.0;
    if crossing {
        return 0.0;
    }
    point_segment(a.0, b)
        .min(point_segment(a.1, b))
        .min(point_segment(b.0, a))
        .min(point_segment(b.1, a))
}

/// The smallest gap between two different round pads or tracks (a pad or a track is a capsule), with the numbers of the two shapes. Rectangles, obrounds and regions are not
/// compared. Shapes that touch or overlap give 0 (they are the same copper or a short circuit: the picture tells which).
pub fn min_clearance(scene: &Scene) -> Result<Option<(f64, usize, usize)>, String> {
    let shapes: Vec<(usize, Capsule)> = scene
        .shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| capsule(s).map(|c| (i, c)))
        .collect();
    if shapes.len() > MAX_PAIRS_SHAPES {
        return Err(format!(
            "{} pads and tracks: more than the {MAX_PAIRS_SHAPES} that are compared pair by pair",
            shapes.len()
        ));
    }
    let mut best: Option<(f64, usize, usize)> = None;
    for (n, (i, a)) in shapes.iter().enumerate() {
        for (j, b) in shapes.iter().skip(n + 1) {
            let gap = (segment_distance((a.0, a.1), (b.0, b.1)) - a.2 - b.2).max(0.0);
            if best.is_none_or(|(g, _, _)| gap < g) {
                best = Some((gap, *i, *j));
            }
        }
    }
    Ok(best)
}
