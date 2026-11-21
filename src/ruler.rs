//! The ruler: the distance, the angle and the two differences between two points of a layer, in the unit of the person. A point that is clicked is caught by the nearest vertex or pad
//! centre within a tolerance (snap) and says which pad, track or region it caught. This is the logic of a click on the picture; the window only has to give it the point.

use crate::gap::kind_name;
use crate::scene::{Scene, Shape};
use crate::settings::Settings;

/// The unit a measure is written in. A layer is always in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Mm,
    Inch,
    /// A thousandth of an inch.
    Mil,
}

impl Unit {
    pub fn parse(text: &str) -> Option<Unit> {
        match text.trim().to_ascii_lowercase().as_str() {
            "mm" => Some(Unit::Mm),
            "in" | "inch" => Some(Unit::Inch),
            "mil" | "thou" => Some(Unit::Mil),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Inch => "in",
            Unit::Mil => "mil",
        }
    }

    /// A length in millimetres written in this unit.
    pub fn from_mm(self, mm: f64) -> f64 {
        match self {
            Unit::Mm => mm,
            Unit::Inch => mm / 25.4,
            Unit::Mil => mm / 0.0254,
        }
    }

    fn decimals(self) -> usize {
        match self {
            Unit::Mm => 3,
            Unit::Inch => 5,
            Unit::Mil => 2,
        }
    }
}

/// What caught a click.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    /// Nothing was near: the point is where it was clicked.
    Free,
    PadCentre,
    Vertex,
}

/// A point after the snap.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapped {
    pub point: (f64, f64),
    pub kind: SnapKind,
    /// The number of the shape that caught the click, in the scene.
    pub shape: Option<usize>,
    /// What that shape is (`round pad`, `track`...).
    pub what: Option<&'static str>,
}

/// The distance, the rank of the kind (a pad centre before a vertex), the shape, the point and its kind.
type Best = (f64, u8, usize, (f64, f64), SnapKind);

fn candidates(shape: &Shape, settings: &Settings) -> Vec<((f64, f64), SnapKind)> {
    let mut out = Vec::new();
    let centre = |x: f64, y: f64, out: &mut Vec<_>| {
        if settings.snap_pad_centres {
            out.push(((x, y), SnapKind::PadCentre));
        }
    };
    let vertex = |x: f64, y: f64, out: &mut Vec<_>| {
        if settings.snap_vertices {
            out.push(((x, y), SnapKind::Vertex));
        }
    };
    match shape {
        Shape::Disc { x, y, .. } | Shape::Obround { x, y, .. } => centre(*x, *y, &mut out),
        Shape::Rect { x, y, w, h } => {
            centre(*x, *y, &mut out);
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                vertex(x + sx * w / 2.0, y + sy * h / 2.0, &mut out);
            }
        }
        Shape::Stroke { x0, y0, x1, y1, .. } => {
            vertex(*x0, *y0, &mut out);
            vertex(*x1, *y1, &mut out);
        }
        Shape::Region { pts } => pts.iter().for_each(|(x, y)| vertex(*x, *y, &mut out)),
    }
    out.retain(|((x, y), _)| x.is_finite() && y.is_finite());
    out
}

/// The point a click at `click` becomes: the nearest vertex or pad centre of the layer that is no further than the tolerance (a distance equal to the tolerance catches), or the click
/// itself. A tie in distance goes to a pad centre and then to the lower shape.
pub fn snap(scene: &Scene, click: (f64, f64), settings: &Settings) -> Result<Snapped, String> {
    if !click.0.is_finite() || !click.1.is_finite() {
        return Err("the point is not a finite number".into());
    }
    let mut best: Option<Best> = None;
    for (i, shape) in scene.shapes.iter().enumerate() {
        for (p, kind) in candidates(shape, settings) {
            let d = (p.0 - click.0).hypot(p.1 - click.1);
            if d > settings.snap_tolerance_mm {
                continue;
            }
            let rank = u8::from(kind != SnapKind::PadCentre);
            if best.is_none_or(|(bd, br, _, _, _)| d < bd || (d == bd && rank < br)) {
                best = Some((d, rank, i, p, kind));
            }
        }
    }
    Ok(match best {
        Some((_, _, i, p, kind)) => Snapped {
            point: p,
            kind,
            shape: Some(i),
            what: Some(kind_name(&scene.shapes[i])),
        },
        None => Snapped {
            point: click,
            kind: SnapKind::Free,
            shape: None,
            what: None,
        },
    })
}

/// A measure between two points.
#[derive(Debug, Clone, PartialEq)]
pub struct Measurement {
    pub from: Snapped,
    pub to: Snapped,
    pub unit: Unit,
    pub dx: f64,
    pub dy: f64,
    pub distance: f64,
    /// From 0 (inclusive) to 360 (exclusive) degrees, anticlockwise from the x axis; 0 when the two points are the same.
    pub angle_deg: f64,
}

/// The measure from `a` to `b` (both are the points after the snap), written in `unit`.
pub fn measure_points(from: Snapped, to: Snapped, unit: Unit) -> Measurement {
    let (dx, dy) = (to.point.0 - from.point.0, to.point.1 - from.point.1);
    let mut angle = dy.atan2(dx).to_degrees();
    if angle < 0.0 {
        angle += 360.0;
    }
    if angle >= 360.0 || dx == 0.0 && dy == 0.0 {
        angle = 0.0;
    }
    Measurement {
        unit,
        dx: unit.from_mm(dx),
        dy: unit.from_mm(dy),
        distance: unit.from_mm(dx.hypot(dy)),
        angle_deg: angle,
        from,
        to,
    }
}

/// Two clicks on a layer: both are snapped with the settings and measured in their unit.
pub fn ruler(
    scene: &Scene,
    a: (f64, f64),
    b: (f64, f64),
    settings: &Settings,
) -> Result<Measurement, String> {
    Ok(measure_points(
        snap(scene, a, settings)?,
        snap(scene, b, settings)?,
        settings.unit,
    ))
}

fn end(p: &Snapped) -> String {
    match (p.what, p.shape) {
        (Some(what), Some(n)) => format!(
            "({:.3}, {:.3}) on {what} {n}{}",
            p.point.0,
            p.point.1,
            if p.kind == SnapKind::PadCentre {
                " (centre)"
            } else {
                " (vertex)"
            }
        ),
        _ => format!("({:.3}, {:.3}) free", p.point.0, p.point.1),
    }
}

impl Measurement {
    /// The lines the command prints.
    pub fn lines(&self) -> Vec<String> {
        let d = self.unit.decimals();
        let u = self.unit.label();
        vec![
            format!("from {}", end(&self.from)),
            format!("to {}", end(&self.to)),
            format!(
                "distance {:.d$} {u}  dx {:+.d$} {u}  dy {:+.d$} {u}  angle {:.2} deg",
                self.distance, self.dx, self.dy, self.angle_deg
            ),
        ]
    }
}
