//! Procedural particle animation for the visualization panel.
//! Renders a pulsating core with orbiting particles.
//! Animation advances only when playing, freezes when paused.

use std::time::Duration;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Number of orbiting particles
const PARTICLE_COUNT: usize = 12;
/// Core pulse amplitude
const PULSE_AMP: f64 = 1.5;
/// Base core radius (in character cells)
const BASE_RADIUS: f64 = 1.2;

pub struct VizWidget {
    pub elapsed: Duration,
}

impl Widget for VizWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let cx = area.width as f64 / 2.0;
        let cy = area.height as f64 / 2.0;
        let t = self.elapsed.as_secs_f64();

        // Core pulsation
        let pulse = (t * 3.0).sin() * PULSE_AMP + BASE_RADIUS + PULSE_AMP;
        let core_char = match ((t * 5.0).sin() * 2.0 + 2.0) as usize % 4 {
            0 => '◉',
            1 => '◎',
            2 => '●',
            _ => '◌',
        };

        // Draw core
        Self::put_char(buf, area, cx as u16, cy as u16, core_char, Color::Rgb(255, 180, 50));

        // Draw inner glow ring
        for a in 0..360 {
            let rad = (a as f64).to_radians();
            let r = pulse;
            let x = (cx + rad.cos() * r) as u16;
            let y = (cy + rad.sin() * r * 0.45) as u16;
            let bright = ((t * 2.0 + a as f64 * 0.1).sin() * 0.4 + 0.6) as u8;
            let color = Color::Rgb(
                (255.0 * bright as f64) as u8,
                (180.0 * bright as f64) as u8,
                (50.0 * bright as f64) as u8,
            );
            Self::put_char(buf, area, x, y, '░', color);
        }

        // Orbiting particles
        for i in 0..PARTICLE_COUNT {
            let speed = 0.6 + i as f64 * 0.17;
            let radius = 2.5 + i as f64 * 0.55;
            let phase = i as f64 * 0.9;
            let angle = t * speed + phase;
            let x = (cx + angle.cos() * radius) as u16;
            let y = (cy + angle.sin() * radius * 0.45) as u16;
            let particle_char = match i % 4 {
                0 => '✦',
                1 => '✧',
                2 => '⋆',
                _ => '·',
            };
            let hue = (i as f64 * 30.0 + t * 40.0) % 360.0;
            let color = hsl_to_rgb(hue, 0.9, 0.7);
            Self::put_char(buf, area, x, y, particle_char, color);
        }

        // Second ring of smaller particles (outer)
        for i in 0..8 {
            let speed = -0.4 - i as f64 * 0.12;
            let radius = 5.0 + i as f64 * 0.7;
            let angle = t * speed + i as f64 * 0.5;
            let x = (cx + angle.cos() * radius) as u16;
            let y = (cy + angle.sin() * radius * 0.45) as u16;
            let color = hsl_to_rgb((i as f64 * 45.0 + t * 20.0) % 360.0, 0.6, 0.5);
            Self::put_char(buf, area, x, y, '·', color);
        }
    }
}

impl VizWidget {
    fn put_char(buf: &mut Buffer, area: Rect, rx: u16, ry: u16, ch: char, color: Color) {
        if rx >= area.width || ry >= area.height {
            return;
        }
        let sx = area.x + rx;
        let sy = area.y + ry;
        if let Some(cell) = buf.cell_mut((sx, sy)) {
            cell.set_char(ch);
            cell.set_style(Style::new().fg(color));
        }
    }
}

/// Simple HSL→RGB conversion for terminal color.
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    Color::Rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
