//! A stack of layers with an opacity each and an order of composition, and a software picture of it for the tests. A layer is painted as a whole: where two shapes of one layer overlap,
//! the opacity is the one of the layer and does not add up. The first layer of the stack is the lowest.

use crate::gap::Prim;
use crate::scene::{Bounds, Scene};
use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub rgb: [u8; 3],
    /// From 0 (nothing) to 1 (solid).
    pub opacity: f64,
    pub visible: bool,
}

/// What is odd about the composition of the stack.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// A shown layer whose opacity is below the minimum that is visible: it is drawn but cannot be seen.
    AlmostInvisible { layer: usize, opacity: f64 },
    /// A shown layer whose box is inside the box of a solid layer above it: it may be hidden.
    MayBeHidden { layer: usize, by: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    layers: Vec<Layer>,
    max_layers: usize,
}

fn check_opacity(opacity: f64) -> Result<(), String> {
    if opacity.is_finite() && (0.0..=1.0).contains(&opacity) {
        Ok(())
    } else {
        Err(format!("opacity {opacity} must be from 0 to 1"))
    }
}

impl Stack {
    pub fn new(settings: &Settings) -> Stack {
        Stack {
            layers: Vec::new(),
            max_layers: settings.max_layers,
        }
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Adds a layer on top. The opacity must be from 0 to 1, the name cannot be empty and the stack has a limit.
    pub fn push(&mut self, layer: Layer) -> Result<usize, String> {
        check_opacity(layer.opacity)?;
        if layer.name.trim().is_empty() {
            return Err("a layer needs a name".into());
        }
        if self.layers.len() >= self.max_layers {
            return Err(format!(
                "the stack has {} layers, the most there can be (max_layers)",
                self.max_layers
            ));
        }
        self.layers.push(layer);
        Ok(self.layers.len() - 1)
    }

    fn get_mut(&mut self, i: usize) -> Result<&mut Layer, String> {
        let n = self.layers.len();
        self.layers
            .get_mut(i)
            .ok_or_else(|| format!("there is no layer {i} (the stack has {n})"))
    }

    /// Sets the opacity of a layer; a value outside 0 to 1 is refused and the layer stays as it was.
    pub fn set_opacity(&mut self, i: usize, opacity: f64) -> Result<(), String> {
        check_opacity(opacity)?;
        self.get_mut(i)?.opacity = opacity;
        Ok(())
    }

    pub fn set_visible(&mut self, i: usize, visible: bool) -> Result<(), String> {
        self.get_mut(i)?.visible = visible;
        Ok(())
    }

    /// Moves layer `from` to place `to` (0 is the lowest); the others keep their order.
    pub fn move_to(&mut self, from: usize, to: usize) -> Result<(), String> {
        let n = self.layers.len();
        if from >= n || to >= n {
            return Err(format!(
                "there is no layer {} (the stack has {n})",
                from.max(to)
            ));
        }
        let l = self.layers.remove(from);
        self.layers.insert(to, l);
        Ok(())
    }

    /// The warnings of the composition. `bounds` has the box of each layer, in the order of the stack.
    pub fn warnings(&self, bounds: &[Bounds], settings: &Settings) -> Result<Vec<Warning>, String> {
        if bounds.len() != self.layers.len() {
            return Err(format!(
                "{} boxes for {} layers",
                bounds.len(),
                self.layers.len()
            ));
        }
        let mut out = Vec::new();
        for (i, l) in self.layers.iter().enumerate() {
            if !l.visible || bounds[i].is_empty() {
                continue;
            }
            if l.opacity < settings.min_visible_opacity {
                out.push(Warning::AlmostInvisible {
                    layer: i,
                    opacity: l.opacity,
                });
            }
            let hider = (i + 1..self.layers.len()).find(|&j| {
                let (u, b) = (&self.layers[j], &bounds[j]);
                let (a, c) = (&bounds[i], b);
                u.visible
                    && u.opacity >= 1.0
                    && !c.is_empty()
                    && c.min_x <= a.min_x
                    && c.min_y <= a.min_y
                    && c.max_x >= a.max_x
                    && c.max_y >= a.max_y
            });
            if let Some(by) = hider {
                out.push(Warning::MayBeHidden { layer: i, by });
            }
        }
        Ok(out)
    }

    /// The colour of a point of the picture when each layer is painted as a whole, from the lowest layer up, over `background`.
    pub fn sample(&self, scenes: &[Scene], p: (f64, f64), background: [f64; 3]) -> [f64; 3] {
        let mut c = background;
        for (l, scene) in self.layers.iter().zip(scenes) {
            if l.visible && covered(scene, p) {
                c = mix(c, l.rgb, l.opacity);
            }
        }
        c
    }

    /// The same point when every shape is painted on its own, as a picture does that gives each shape its opacity: where two shapes of one layer overlap it is more solid. This is
    /// what the whole-layer painting avoids; it is kept to show the difference.
    pub fn sample_per_shape(
        &self,
        scenes: &[Scene],
        p: (f64, f64),
        background: [f64; 3],
    ) -> [f64; 3] {
        let mut c = background;
        for (l, scene) in self.layers.iter().zip(scenes) {
            if !l.visible {
                continue;
            }
            for s in &scene.shapes {
                if Prim::of(s).is_ok_and(|q| q.contains(p)) {
                    c = mix(c, l.rgb, l.opacity);
                }
            }
        }
        c
    }

    /// A small picture of the stack for the tests: `width` x `height` pixels over the box `[min_x, min_y, max_x, max_y]` (y up), a pixel taken at its centre. The size is limited.
    pub fn render(
        &self,
        scenes: &[Scene],
        area: [f64; 4],
        width: usize,
        height: usize,
        background: [f64; 3],
    ) -> Result<Vec<[f64; 3]>, String> {
        if width == 0 || height == 0 || width > 2048 || height > 2048 {
            return Err(format!(
                "{width} x {height} pixels must be from 1 to 2048 each way"
            ));
        }
        if !area.iter().all(|v| v.is_finite()) || area[2] <= area[0] || area[3] <= area[1] {
            return Err("the area of the picture is empty or not finite".into());
        }
        let (sx, sy) = (
            (area[2] - area[0]) / width as f64,
            (area[3] - area[1]) / height as f64,
        );
        let mut out = Vec::with_capacity(width * height);
        for row in 0..height {
            // the first row is the top of the picture
            let y = area[3] - (row as f64 + 0.5) * sy;
            for col in 0..width {
                let x = area[0] + (col as f64 + 0.5) * sx;
                out.push(self.sample(scenes, (x, y), background));
            }
        }
        Ok(out)
    }
}

fn mix(below: [f64; 3], rgb: [u8; 3], opacity: f64) -> [f64; 3] {
    let mut out = below;
    for (o, (b, c)) in out.iter_mut().zip(below.iter().zip(rgb)) {
        *o = b * (1.0 - opacity) + f64::from(c) * opacity;
    }
    out
}

/// Whether any shape of the scene covers the point.
fn covered(scene: &Scene, p: (f64, f64)) -> bool {
    scene
        .shapes
        .iter()
        .any(|s| Prim::of(s).is_ok_and(|q| q.contains(p)))
}
