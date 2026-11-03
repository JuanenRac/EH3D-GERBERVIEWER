//! The layers as one SVG picture: a colour and a transparency for each layer, the board seen from above (y up).

use crate::scene::{Bounds, Scene, Shape};

pub struct Layer {
    pub name: String,
    pub scene: Scene,
    /// `#rrggbb`
    pub color: String,
    /// 0 (invisible) to 1 (solid)
    pub opacity: f64,
}

/// A colour for a layer from the extension of its file.
pub fn default_color(file_name: &str) -> &'static str {
    match file_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "gtl" => "#c8a33c",
        "gbl" => "#3c7bc8",
        "gts" | "gbs" => "#2e8b57",
        "gto" | "gbo" => "#e8e8e8",
        "gtp" | "gbp" => "#9aa0a6",
        "gko" | "gm1" | "gml" => "#ffd700",
        "drl" | "ncd" | "xln" | "exc" | "drd" => "#ffffff",
        _ => "#b0b0b0",
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn shape(s: &Shape) -> String {
    match s {
        Shape::Disc { x, y, r } => format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>", num(*x), num(*y), num(*r)),
        Shape::Rect { x, y, w, h } => format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>", num(x - w / 2.0), num(y - h / 2.0), num(*w), num(*h)),
        Shape::Obround { x, y, w, h } => format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\"/>", num(x - w / 2.0), num(y - h / 2.0), num(*w), num(*h), num(w.min(*h) / 2.0)),
        Shape::Stroke { x0, y0, x1, y1, w } => format!("<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" fill=\"none\"/>", num(*x0), num(*y0), num(*x1), num(*y1), num(*w)),
        Shape::Region { pts } => format!("<polygon points=\"{}\"/>", pts.iter().map(|(x, y)| format!("{},{}", num(*x), num(*y))).collect::<Vec<_>>().join(" ")),
    }
}

/// The picture of all the layers, in order (the first is at the bottom). An empty list or layers with nothing in them give a small empty picture, not an error.
pub fn render(layers: &[Layer], margin_mm: f64) -> String {
    let mut b = Bounds::empty();
    for l in layers {
        b = b.union(&l.scene.bounds());
    }
    if b.is_empty() {
        b = Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 10.0,
            max_y: 10.0,
        };
    }
    let m = if margin_mm.is_finite() && margin_mm >= 0.0 {
        margin_mm
    } else {
        0.0
    };
    let (w, h) = (b.width() + 2.0 * m, b.height() + 2.0 * m);
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"100%\" role=\"img\">\n<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#10141a\"/>\n",
        num(b.min_x - m),
        num(-(b.max_y + m)),
        num(w),
        num(h),
        num(b.min_x - m),
        num(-(b.max_y + m)),
        num(w),
        num(h)
    );
    for l in layers {
        let color = if l.color.len() == 7
            && l.color.starts_with('#')
            && l.color[1..].bytes().all(|c| c.is_ascii_hexdigit())
        {
            l.color.as_str()
        } else {
            "#b0b0b0"
        };
        let opacity = if l.opacity.is_finite() {
            l.opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        out.push_str(&format!(
            "<g id=\"{}\" fill=\"{}\" stroke=\"{}\" opacity=\"{}\" transform=\"scale(1,-1)\">\n",
            escape(&l.name),
            color,
            color,
            num(opacity)
        ));
        for s in &l.scene.shapes {
            out.push_str(&shape(s));
            out.push('\n');
        }
        out.push_str("</g>\n");
    }
    out.push_str("</svg>\n");
    out
}
