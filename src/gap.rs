//! The smallest gap between two objects, exact for every kind of shape. An object is a core (a point, a segment or a filled polygon) grown by a radius: a round pad is a point, a track and
//! an obround pad are a segment, a rectangular pad and a region are polygons with no radius. The gap between two objects is the distance between their cores less the two radii, and 0
//! when they touch or overlap. The comparison of every pair is bounded by the number of pairs.

use crate::measure::segment_distance;
use crate::scene::{Scene, Shape};
use eh3d_geometry::polygon::{region_contains, Containment};

type Pt = (f64, f64);

/// The core of an object and the radius it is grown by.
#[derive(Debug, Clone, PartialEq)]
pub struct Prim {
    core: Vec<Pt>,
    radius: f64,
}

fn finite(values: &[f64]) -> bool {
    values.iter().all(|v| v.is_finite())
}

/// The name a person reads for a kind of shape.
pub fn kind_name(shape: &Shape) -> &'static str {
    match shape {
        Shape::Disc { .. } => "round pad",
        Shape::Rect { .. } => "rectangular pad",
        Shape::Obround { .. } => "obround pad",
        Shape::Stroke { .. } => "track",
        Shape::Region { .. } => "region",
    }
}

impl Prim {
    /// The object of a shape; a shape with a number that is not finite, a negative size or a region of fewer than three points is an error that says so.
    pub fn of(shape: &Shape) -> Result<Prim, String> {
        let bad = |what: &str| Err(format!("a {} with {what}", kind_name(shape)));
        match shape {
            Shape::Disc { x, y, r } => {
                if !finite(&[*x, *y, *r]) || *r < 0.0 {
                    return bad("a number that is not valid");
                }
                Ok(Prim {
                    core: vec![(*x, *y)],
                    radius: *r,
                })
            }
            Shape::Rect { x, y, w, h } => {
                if !finite(&[*x, *y, *w, *h]) || *w < 0.0 || *h < 0.0 {
                    return bad("a number that is not valid");
                }
                let (hw, hh) = (w / 2.0, h / 2.0);
                Ok(Prim {
                    core: vec![
                        (x - hw, y - hh),
                        (x + hw, y - hh),
                        (x + hw, y + hh),
                        (x - hw, y + hh),
                    ],
                    radius: 0.0,
                })
            }
            Shape::Obround { x, y, w, h } => {
                if !finite(&[*x, *y, *w, *h]) || *w < 0.0 || *h < 0.0 {
                    return bad("a number that is not valid");
                }
                // the long side holds the segment, the short side is the diameter
                let (r, half) = if w >= h {
                    (h / 2.0, (w - h) / 2.0)
                } else {
                    (w / 2.0, (h - w) / 2.0)
                };
                let core = if w >= h {
                    vec![(x - half, *y), (x + half, *y)]
                } else {
                    vec![(*x, y - half), (*x, y + half)]
                };
                Ok(Prim { core, radius: r })
            }
            Shape::Stroke { x0, y0, x1, y1, w } => {
                if !finite(&[*x0, *y0, *x1, *y1, *w]) || *w < 0.0 {
                    return bad("a number that is not valid");
                }
                Ok(Prim {
                    core: vec![(*x0, *y0), (*x1, *y1)],
                    radius: w / 2.0,
                })
            }
            Shape::Region { pts } => {
                if pts.len() < 3 {
                    return bad("fewer than three points");
                }
                if !pts.iter().all(|(x, y)| x.is_finite() && y.is_finite()) {
                    return bad("a number that is not valid");
                }
                Ok(Prim {
                    core: pts.clone(),
                    radius: 0.0,
                })
            }
        }
    }

    fn segments(&self) -> Vec<(Pt, Pt)> {
        let n = self.core.len();
        match n {
            1 => vec![(self.core[0], self.core[0])],
            2 => vec![(self.core[0], self.core[1])],
            _ => (0..n)
                .map(|i| (self.core[i], self.core[(i + 1) % n]))
                .collect(),
        }
    }

    /// The box of the object, grown by its radius: `[min_x, min_y, max_x, max_y]`.
    pub fn bbox(&self) -> [f64; 4] {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for (x, y) in &self.core {
            b[0] = b[0].min(x - self.radius);
            b[1] = b[1].min(y - self.radius);
            b[2] = b[2].max(x + self.radius);
            b[3] = b[3].max(y + self.radius);
        }
        b
    }

    fn filled(&self) -> bool {
        self.core.len() >= 3
    }

    fn core_contains(&self, p: Pt) -> bool {
        self.filled()
            && region_contains(std::slice::from_ref(&self.core), p) != Containment::Outside
    }

    /// Whether a point is inside the object or on its edge.
    pub fn contains(&self, p: Pt) -> bool {
        self.core_contains(p)
            || self
                .segments()
                .iter()
                .any(|s| segment_distance((p, p), *s) <= self.radius)
    }

    /// The distance between the cores: 0 when they touch, cross or one holds the other.
    fn core_distance(&self, other: &Prim) -> f64 {
        if other.core.iter().any(|p| self.core_contains(*p))
            || self.core.iter().any(|p| other.core_contains(*p))
        {
            return 0.0;
        }
        let (sa, sb) = (self.segments(), other.segments());
        let mut best = f64::INFINITY;
        for a in &sa {
            for b in &sb {
                let d = segment_distance(*a, *b);
                if d < best {
                    best = d;
                    if best == 0.0 {
                        return 0.0;
                    }
                }
            }
        }
        best
    }

    /// The gap to another object: the distance between the cores less both radii, never below 0.
    pub fn gap(&self, other: &Prim) -> f64 {
        (self.core_distance(other) - self.radius - other.radius).max(0.0)
    }
}

/// The gap between two shapes.
pub fn shape_gap(a: &Shape, b: &Shape) -> Result<f64, String> {
    Ok(Prim::of(a)?.gap(&Prim::of(b)?))
}

/// The closest pair found, with the numbers of the two shapes in their scenes.
#[derive(Debug, Clone, PartialEq)]
pub struct Gap {
    /// 0 when the two touch or overlap.
    pub distance: f64,
    pub a_index: usize,
    pub b_index: usize,
    pub a_kind: &'static str,
    pub b_kind: &'static str,
}

/// What a comparison found.
#[derive(Debug, Clone, PartialEq)]
pub struct GapReport {
    /// `None` when one side has no valid shape.
    pub best: Option<Gap>,
    /// Shapes that were left out because they were not valid (a number that is not finite, a region of two points), counted and not hidden.
    pub skipped: usize,
}

fn box_distance(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let dx = (a[0] - b[2]).max(b[0] - a[2]).max(0.0);
    let dy = (a[1] - b[3]).max(b[1] - a[3]).max(0.0);
    dx.hypot(dy)
}

fn prims(scene: &Scene) -> (Vec<(usize, Prim, [f64; 4])>, usize) {
    let mut skipped = 0;
    let mut out = Vec::new();
    for (i, s) in scene.shapes.iter().enumerate() {
        match Prim::of(s) {
            Ok(p) => {
                let b = p.bbox();
                out.push((i, p, b));
            }
            Err(_) => skipped += 1,
        }
    }
    (out, skipped)
}

fn search(a: &Scene, b: &Scene, same: bool, max_pairs: usize) -> Result<GapReport, String> {
    let pairs = if same {
        a.shapes
            .len()
            .saturating_mul(a.shapes.len().saturating_sub(1))
            / 2
    } else {
        a.shapes.len().saturating_mul(b.shapes.len())
    };
    if pairs > max_pairs {
        return Err(format!(
            "{pairs} pairs of shapes: more than the {max_pairs} that are compared (max_gap_pairs)"
        ));
    }
    let (pa, mut skipped) = prims(a);
    let pb_owned;
    let pb: &[(usize, Prim, [f64; 4])] = if same {
        &pa
    } else {
        let (v, s) = prims(b);
        skipped += s;
        pb_owned = v;
        &pb_owned
    };
    let mut best: Option<Gap> = None;
    'outer: for (n, (ia, x, bx)) in pa.iter().enumerate() {
        let start = if same { n + 1 } else { 0 };
        for (ib, y, by) in &pb[start..] {
            let limit = best.as_ref().map_or(f64::INFINITY, |g| g.distance);
            if box_distance(bx, by) >= limit {
                continue;
            }
            let d = x.gap(y);
            if d < limit {
                best = Some(Gap {
                    distance: d,
                    a_index: *ia,
                    b_index: *ib,
                    a_kind: kind_name(&a.shapes[*ia]),
                    b_kind: kind_name(&b.shapes[*ib]),
                });
                if d == 0.0 {
                    break 'outer;
                }
            }
        }
    }
    Ok(GapReport { best, skipped })
}

/// The smallest gap between a shape of `a` and a shape of `b` (two layers); an error says how many pairs there were when they are more than `max_pairs`.
pub fn layer_gap(a: &Scene, b: &Scene, max_pairs: usize) -> Result<GapReport, String> {
    search(a, b, false, max_pairs)
}

/// The smallest gap between two different shapes of one layer, of every kind.
pub fn scene_gap(scene: &Scene, max_pairs: usize) -> Result<GapReport, String> {
    search(scene, scene, true, max_pairs)
}
