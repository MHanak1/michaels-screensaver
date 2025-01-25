use angular_units::{Angle, Turns};
use prisma::{Hsv, Rgb};
use wgpu::Color;

pub fn compare_colors_ignoring_alpha(left: Color, right: Color) -> bool {
    left.r == right.r && left.g == right.g && left.b == right.b
}

pub fn random_color() -> Color {
    let hsv = Hsv::new(angular_units::Turns(rand::random::<f32>()), 1.0, 1.0);
    let rgb = Rgb::from(hsv);
    Color {
        r: rgb.red(),
        g: rgb.green(),
        b: rgb.blue(),
        a: 1.0,
    }
}

pub fn random_distinct_color(other_color: Color) -> Color {
    let old_rgb = Rgb::new(other_color.r, other_color.g, other_color.b);
    let old_hsv: Hsv<f64, Turns<f64>> = Hsv::from(old_rgb);

    let mut new_hsv = old_hsv.clone();

    loop {
        new_hsv = Hsv::new(angular_units::Turns(rand::random::<f64>()), 1.0, 1.0);
        let mut delta = old_hsv.hue().scalar() - new_hsv.hue().scalar();
        if delta > 0.5 {
            delta -= 1.0
        } else if delta < -0.5 {
            delta += 1.0
        }
        if delta > 0.2 {
            break;
        }
    }
    let rgb = Rgb::from(new_hsv);
    Color {
        r: rgb.red(),
        g: rgb.green(),
        b: rgb.blue(),
        a: 1.0,
    }
}

pub fn color_from_hex(color_hex: String) -> Result<Color, anyhow::Error> {
    if color_hex.starts_with("#") {
        let color_hex: String = color_hex[1..7].parse()?;
        Ok(wgpu::Color {
            r: i64::from_str_radix(&color_hex[0..2], 16)? as f64 / 255.0,
            g: i64::from_str_radix(&color_hex[2..4], 16)? as f64 / 255.0,
            b: i64::from_str_radix(&color_hex[4..6], 16)? as f64 / 255.0,
            a: 1.0,
        })
    } else {
        Err(anyhow::anyhow!("Invalid color hex: {}", color_hex))
    }
}
