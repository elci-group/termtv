//! Scene rendering trait and built-in demo scene.

use crate::color::{Cell, Rgb};
use crate::framebuffer::Framebuffer;

/// A renderable object that can draw itself into a framebuffer at a given time.
pub trait Scene {
    fn render(&self, frame: &mut Framebuffer, time: f32);
}

/// A compact demo scene used by the binary and useful as a smoke test.
pub struct DemoScene;

impl Scene for DemoScene {
    fn render(&self, frame: &mut Framebuffer, time: f32) {
        let w = frame.width as f32;
        let h = frame.height as f32;
        frame.clear(Rgb(7, 10, 18));
        let center = (w * 0.5 + time.sin() * w * 0.18, h * 0.5);
        let radius = (h.min(w) * 0.28) + time.sin() * 1.5;
        for y in 0..frame.height {
            for x in 0..frame.width {
                let dx = x as f32 - center.0;
                let dy = y as f32 - center.1;
                let d = (dx * dx + dy * dy).sqrt();
                if d < radius {
                    let glow = (1.0 - d / radius).max(0.0);
                    let pulse = ((time * 4.0 + d * 0.12).sin() * 0.5 + 0.5) * 30.0;
                    let c = Rgb(
                        (40.0 + glow * 170.0) as u8,
                        (70.0 + glow * 90.0) as u8,
                        (120.0 + pulse + glow * 100.0).min(255.0) as u8,
                    );
                    frame.set(
                        x,
                        y,
                        Cell {
                            top: c,
                            bottom: Rgb(c.0 / 2, c.1 / 2, c.2),
                        },
                    );
                }
            }
        }
    }
}
