//! A tiny, quiet room for muku, drawn entirely in software.
//!
//! Character geometry and palette follow
//! TSUQREA/mukumo/apps/mobile/src/components/Muku.tsx. No downloaded artwork or
//! fonts are used; the small UI lettering comes from the font8x8 crate.

use crate::state::{Action, PetState};
use font8x8::{UnicodeFonts, BASIC_FONTS};
use tiny_skia::{
    Color, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint, Path, PathBuilder,
    Pixmap, Point, Rect, SpreadMode, Stroke, Transform,
};

pub const WIDTH: u32 = 480;
pub const HEIGHT: u32 = 272;

fn paper() -> Color {
    Color::from_rgba8(245, 243, 238, 255)
}
fn ink() -> Color {
    Color::from_rgba8(21, 20, 16, 255)
}
fn muted() -> Color {
    Color::from_rgba8(112, 115, 106, 255)
}
fn rule() -> Color {
    Color::from_rgba8(214, 214, 203, 255)
}
fn green() -> Color {
    Color::from_rgba8(90, 137, 104, 255)
}

/// Render one frame at the handheld's native 480 × 272 resolution.
///
/// `t` is a monotonic animation time in seconds, independent of saved pet age.
pub fn render(state: &PetState, t: f32) -> Pixmap {
    let mut canvas = Pixmap::new(WIDTH, HEIGHT).expect("fixed-size framebuffer");
    let t = if t.is_finite() { t.max(0.0) } else { 0.0 };
    canvas.fill(paper());
    room(&mut canvas, state.asleep, t);

    text(&mut canvas, "muku", 24.0, 20.0, 2.0, ink());
    text(&mut canvas, "Built with Rust", 110.0, 24.0, 1.0, muted());
    let day = (state.total_awake_secs.max(0.0) / 86_400.0).floor() as u64 + 1;
    text_right(
        &mut canvas,
        &format!("day {day}"),
        455.0,
        24.0,
        1.0,
        muted(),
    );

    // A little woven rug grounds the character without enclosing it in UI.
    oval(
        &mut canvas,
        236.0,
        216.0,
        95.0,
        16.0,
        color(222, 225, 211, 255),
    );
    oval(
        &mut canvas,
        236.0,
        214.0,
        85.0,
        12.0,
        color(231, 234, 222, 255),
    );
    for i in 0..5 {
        let x = 155.0 + i as f32 * 4.0;
        line(
            &mut canvas,
            (x, 215.0),
            (x - 6.0, 218.0),
            1.0,
            color(200, 207, 190, 160),
        );
        line(
            &mut canvas,
            (472.0 - x, 215.0),
            (478.0 - x, 218.0),
            1.0,
            color(200, 207, 190, 160),
        );
    }

    let age = state.action_age.max(0.0);
    let active = age < 2.6;
    let petting = active && matches!(state.last_action, Action::Pet);
    let feeding = active && matches!(state.last_action, Action::Feed);
    let playing = active && matches!(state.last_action, Action::Play);
    let breathe = (t * if state.asleep { 1.35 } else { 1.8 }).sin();
    let bounce = if playing {
        -(age * 7.0).sin().abs() * 20.0 * (1.0 - age / 2.6)
    } else if petting {
        -(age * 5.0).sin().abs() * 3.0
    } else {
        0.0
    };
    let lean = if playing {
        (age * 7.0).sin() * 7.0
    } else {
        (t * 0.65).sin() * 1.5
    };
    let width = 1.86
        + breathe * 0.012
        + if petting {
            (age * 8.0).sin() * 0.015
        } else {
            0.0
        };
    let height = if state.asleep { 1.57 } else { 1.78 } + breathe * 0.024;
    let cx = 236.0 + lean;
    let base = 209.0 + bounce;

    // Layered transparent ovals give a soft, cheap software shadow.
    for i in (0..10).rev() {
        let n = i as f32;
        oval(
            &mut canvas,
            cx,
            212.0,
            56.0 + n * 2.0 + bounce * 0.2,
            4.0 + n * 0.62,
            color(83, 118, 87, 3 + (10 - i) as u8),
        );
    }

    let blink = (t % 6.1) > 5.90;
    draw_muku(
        &mut canvas,
        cx,
        base,
        width,
        height,
        state.asleep || blink,
        petting,
    );

    if petting {
        let fade = ((1.0 - age / 2.6) * 230.0) as u8;
        heart(
            &mut canvas,
            315.0,
            90.0 - age * 13.0,
            10.0,
            color(211, 125, 159, fade),
        );
        if age > 0.3 {
            heart(
                &mut canvas,
                160.0,
                79.0 - age * 9.0,
                6.0,
                color(211, 125, 159, fade),
            );
        }
    }
    if feeding {
        food(&mut canvas, 301.0, 202.0, age);
    }
    if playing {
        ball(
            &mut canvas,
            333.0 + (age * 5.0).sin() * 12.0,
            209.0 - (age * 6.5).sin().abs() * 34.0,
            12.0,
        );
        sparkle(&mut canvas, 155.0, 111.0, 4.0, color(154, 177, 114, 230));
    }
    if state.asleep {
        let rise = (t * 0.33) % 1.0;
        text(
            &mut canvas,
            "z",
            314.0 + rise * 7.0,
            107.0 - rise * 17.0,
            1.0,
            color(121, 135, 153, ((1.0 - rise) * 180.0) as u8),
        );
        text(
            &mut canvas,
            "z",
            328.0 + rise * 7.0,
            87.0 - rise * 17.0,
            1.5,
            color(121, 135, 153, ((1.0 - rise) * 130.0) as u8),
        );
    }

    needs(&mut canvas, state);
    let caption = if state.asleep {
        "a little rest."
    } else if petting {
        "right here with you."
    } else if feeding {
        "a little treat."
    } else if playing {
        "one more?"
    } else if state.fullness < 25.0 {
        "a little hungry."
    } else if state.energy < 25.0 {
        "getting sleepy."
    } else if state.joy < 25.0 {
        "stay a while?"
    } else {
        "a good day to be muku."
    };
    text_center(&mut canvas, caption, 236.0, 231.0, 1.0, muted());
    controls(&mut canvas, state.asleep);
    canvas
}

fn room(p: &mut Pixmap, asleep: bool, t: f32) {
    rect(p, 0.0, 207.0, 480.0, 65.0, color(239, 237, 230, 255));
    line(
        p,
        (20.0, 207.0),
        (122.0, 207.0),
        1.0,
        color(220, 219, 209, 255),
    );
    line(
        p,
        (350.0, 207.0),
        (460.0, 207.0),
        1.0,
        color(220, 219, 209, 255),
    );

    // The window changes gently with muku's sleeping state; the whole display
    // keeps its paper color so text remains readable on a small LCD.
    rounded_rect(p, 37.0, 63.0, 78.0, 84.0, 14.0, color(222, 222, 211, 255));
    let sky = rounded_path(41.0, 67.0, 70.0, 76.0, 11.0);
    let stops = if asleep {
        vec![
            GradientStop::new(0.0, color(123, 143, 169, 255)),
            GradientStop::new(1.0, color(197, 209, 216, 255)),
        ]
    } else {
        vec![
            GradientStop::new(0.0, color(200, 224, 224, 255)),
            GradientStop::new(1.0, color(231, 237, 222, 255)),
        ]
    };
    let paint = Paint {
        shader: LinearGradient::new(
            Point::from_xy(41.0, 67.0),
            Point::from_xy(111.0, 143.0),
            stops,
            SpreadMode::Pad,
            Transform::identity(),
        )
        .expect("nonzero window gradient"),
        anti_alias: true,
        ..Paint::default()
    };
    p.fill_path(&sky, &paint, FillRule::Winding, Transform::identity(), None);
    if asleep {
        oval(p, 88.0, 87.0, 8.0, 8.0, color(243, 239, 206, 255));
        oval(p, 91.5, 83.5, 7.3, 7.3, color(145, 162, 184, 255));
        sparkle(p, 57.0, 93.0, 2.0, color(242, 241, 221, 230));
        oval(p, 94.0, 120.0, 1.0, 1.0, color(242, 241, 221, 220));
    } else {
        oval(p, 88.0, 87.0, 8.0, 8.0, color(238, 218, 155, 255));
        let drift = (t * 0.12).sin() * 3.0;
        oval(p, 58.0 + drift, 112.0, 12.0, 4.0, color(249, 249, 239, 150));
        oval(p, 65.0 + drift, 109.0, 7.0, 5.0, color(249, 249, 239, 150));
    }
    line(
        p,
        (76.0, 67.0),
        (76.0, 143.0),
        3.0,
        color(231, 230, 217, 255),
    );
    line(
        p,
        (42.0, 112.0),
        (110.0, 112.0),
        3.0,
        color(231, 230, 217, 255),
    );
    rounded_rect(p, 32.0, 145.0, 88.0, 5.0, 2.0, color(207, 210, 195, 255));

    // A plant is the only room ornament, drawn with the same rounded rhythm
    // as muku. It makes this feel inhabited without competing with the pet.
    line(
        p,
        (74.0, 195.0),
        (73.0, 170.0),
        2.0,
        color(113, 144, 104, 255),
    );
    leaf(
        p,
        (73.0, 181.0),
        (60.0, 168.0),
        (57.0, 181.0),
        color(147, 170, 125, 255),
    );
    leaf(
        p,
        (74.0, 177.0),
        (88.0, 161.0),
        (92.0, 175.0),
        color(126, 156, 112, 255),
    );
    leaf(
        p,
        (73.0, 173.0),
        (66.0, 158.0),
        (78.0, 157.0),
        color(166, 186, 140, 255),
    );
    let mut pot = PathBuilder::new();
    pot.move_to(60.0, 190.0);
    pot.line_to(87.0, 190.0);
    pot.line_to(83.0, 209.0);
    pot.quad_to(73.0, 213.0, 64.0, 209.0);
    pot.close();
    fill(p, &pot.finish().unwrap(), color(201, 193, 173, 255));
    line(
        p,
        (60.0, 191.0),
        (87.0, 191.0),
        3.0,
        color(187, 180, 159, 255),
    );
}

fn draw_muku(p: &mut Pixmap, cx: f32, base: f32, sx: f32, sy: f32, closed: bool, happy: bool) {
    let x = cx - 50.0 * sx;
    let y = base - 93.0 * sy;
    let mut b = PathBuilder::new();
    b.move_to(25.0, 93.0);
    b.cubic_to(13.0, 93.0, 6.0, 84.0, 8.0, 73.0);
    b.cubic_to(3.0, 65.0, 6.0, 53.0, 16.0, 49.0);
    b.cubic_to(14.0, 35.0, 26.0, 24.0, 39.0, 28.0);
    b.cubic_to(44.0, 17.0, 60.0, 14.0, 68.0, 24.0);
    b.cubic_to(79.0, 20.0, 92.0, 30.0, 89.0, 44.0);
    b.cubic_to(97.0, 50.0, 97.0, 64.0, 90.0, 71.0);
    b.cubic_to(94.0, 83.0, 86.0, 93.0, 75.0, 93.0);
    b.close();
    let body = b
        .finish()
        .unwrap()
        .transform(Transform::from_row(sx, 0.0, 0.0, sy, x, y))
        .unwrap();
    let body_paint = Paint {
        shader: LinearGradient::new(
            Point::from_xy(x + 10.0 * sx, y),
            Point::from_xy(x + 90.0 * sx, y + 100.0 * sy),
            vec![
                GradientStop::new(0.05, color(243, 166, 255, 255)),
                GradientStop::new(0.55, color(0, 196, 255, 255)),
                GradientStop::new(1.0, color(53, 200, 56, 255)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        )
        .expect("nonzero character gradient"),
        anti_alias: true,
        ..Paint::default()
    };
    p.fill_path(
        &body,
        &body_paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    for eye in [39.0, 61.0] {
        let ex = x + eye * sx;
        let ey = y + 64.0 * sy;
        if closed || happy {
            let mut e = PathBuilder::new();
            e.move_to(ex - 4.0 * sx, ey);
            e.quad_to(
                ex,
                ey + if happy { -3.4 } else { 2.8 } * sy,
                ex + 4.0 * sx,
                ey,
            );
            stroke(p, &e.finish().unwrap(), 2.2 * sx, color(35, 38, 46, 255));
        } else {
            oval(p, ex, ey, 3.9 * sx, 5.6 * sy, color(35, 38, 46, 255));
            oval(
                p,
                x + (eye + 1.3) * sx,
                y + 61.7 * sy,
                1.3 * sx,
                1.3 * sy,
                Color::WHITE,
            );
        }
    }
}

fn needs(p: &mut Pixmap, state: &PetState) {
    for (label, value, y) in [
        ("food", state.fullness, 83.0),
        ("happy", state.joy, 121.0),
        ("rest", state.energy, 159.0),
    ] {
        text(p, label, 382.0, y, 1.0, muted());
        let v = if value.is_finite() {
            value.clamp(0.0, 100.0)
        } else {
            0.0
        };
        for i in 0..5 {
            let lit = v >= (i as f32 + 0.5) * 20.0;
            rounded_rect(
                p,
                382.0 + i as f32 * 12.0,
                y + 13.0,
                8.0,
                4.0,
                2.0,
                if lit {
                    if v < 25.0 {
                        color(189, 142, 87, 255)
                    } else {
                        green()
                    }
                } else {
                    rule()
                },
            );
        }
    }
}

fn controls(p: &mut Pixmap, asleep: bool) {
    line(
        p,
        (24.0, 247.0),
        (456.0, 247.0),
        1.0,
        color(221, 219, 209, 255),
    );
    for (key, label, x) in [
        ("A", "pet", 33.0),
        ("B", "feed", 138.0),
        ("X", "play", 243.0),
        ("Y", if asleep { "wake" } else { "sleep" }, 348.0),
    ] {
        oval(p, x + 6.0, 260.0, 7.0, 7.0, color(222, 224, 214, 255));
        text(p, key, x + 2.0, 256.0, 1.0, color(79, 89, 72, 255));
        text(p, label, x + 19.0, 256.0, 1.0, color(93, 98, 88, 255));
    }
}

fn food(p: &mut Pixmap, x: f32, y: f32, age: f32) {
    oval(p, x, y + 10.0, 22.0, 5.0, color(190, 195, 174, 180));
    let mut b = PathBuilder::new();
    b.move_to(x - 20.0, y);
    b.quad_to(x - 16.0, y + 15.0, x, y + 13.0);
    b.quad_to(x + 16.0, y + 15.0, x + 20.0, y);
    b.close();
    fill(p, &b.finish().unwrap(), color(220, 213, 191, 255));
    oval(p, x, y, 20.0, 5.0, color(245, 241, 222, 255));
    oval(p, x, y, 15.0, 3.0, color(191, 167, 124, 255));
    for (dx, dy) in [(-8.0, -1.0), (0.0, -3.0), (8.0, 0.0)] {
        oval(p, x + dx, y + dy, 4.0, 3.0, color(220, 183, 120, 255));
    }
    if age < 1.4 {
        sparkle(
            p,
            x + 5.0,
            y - 15.0 - age * 8.0,
            3.0,
            color(191, 158, 99, 200),
        );
    }
}

fn ball(p: &mut Pixmap, x: f32, y: f32, r: f32) {
    oval(p, x, 217.0, r * 0.8, 3.0, color(112, 136, 109, 28));
    oval(p, x, y, r, r, color(224, 177, 148, 255));
    let mut seam = PathBuilder::new();
    seam.move_to(x - r * 0.7, y - r * 0.7);
    seam.cubic_to(
        x + r * 0.45,
        y - r * 0.3,
        x - r * 0.4,
        y + r * 0.4,
        x + r * 0.7,
        y + r * 0.7,
    );
    stroke(p, &seam.finish().unwrap(), 2.0, color(247, 232, 204, 255));
    oval(p, x - 4.0, y - 5.0, 2.5, 2.5, color(255, 247, 225, 100));
}

fn leaf(p: &mut Pixmap, base: (f32, f32), tip: (f32, f32), curve: (f32, f32), c: Color) {
    let mut b = PathBuilder::new();
    b.move_to(base.0, base.1);
    b.quad_to(curve.0, curve.1, tip.0, tip.1);
    b.quad_to(tip.0 + 7.0, base.1 - 9.0, base.0, base.1);
    b.close();
    fill(p, &b.finish().unwrap(), c);
}

fn heart(p: &mut Pixmap, x: f32, y: f32, r: f32, c: Color) {
    let mut b = PathBuilder::new();
    b.move_to(x, y + r);
    b.cubic_to(x - r * 1.7, y, x - r, y - r * 1.2, x, y - r * 0.4);
    b.cubic_to(x + r, y - r * 1.2, x + r * 1.7, y, x, y + r);
    b.close();
    fill(p, &b.finish().unwrap(), c);
}

fn sparkle(p: &mut Pixmap, x: f32, y: f32, r: f32, c: Color) {
    let mut b = PathBuilder::new();
    b.move_to(x, y - r);
    b.quad_to(x + r * 0.18, y - r * 0.18, x + r, y);
    b.quad_to(x + r * 0.18, y + r * 0.18, x, y + r);
    b.quad_to(x - r * 0.18, y + r * 0.18, x - r, y);
    b.quad_to(x - r * 0.18, y - r * 0.18, x, y - r);
    b.close();
    fill(p, &b.finish().unwrap(), c);
}

fn color(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba8(r, g, b, a)
}

fn fill(p: &mut Pixmap, path: &Path, c: Color) {
    let mut paint = Paint::default();
    paint.set_color(c);
    paint.anti_alias = true;
    p.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
}

fn stroke(p: &mut Pixmap, path: &Path, width: f32, c: Color) {
    let mut paint = Paint::default();
    paint.set_color(c);
    paint.anti_alias = true;
    let style = Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    p.stroke_path(path, &paint, &style, Transform::identity(), None);
}

fn rect(p: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Color) {
    let mut paint = Paint::default();
    paint.set_color(c);
    p.fill_rect(
        Rect::from_xywh(x, y, w, h).unwrap(),
        &paint,
        Transform::identity(),
        None,
    );
}

fn oval(p: &mut Pixmap, x: f32, y: f32, rx: f32, ry: f32, c: Color) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let path = PathBuilder::from_oval(Rect::from_xywh(x - rx, y - ry, rx * 2.0, ry * 2.0).unwrap())
        .unwrap();
    fill(p, &path, c);
}

fn line(p: &mut Pixmap, from: (f32, f32), to: (f32, f32), w: f32, c: Color) {
    let mut b = PathBuilder::new();
    b.move_to(from.0, from.1);
    b.line_to(to.0, to.1);
    stroke(p, &b.finish().unwrap(), w, c);
}

fn rounded_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Path {
    let r = r.min(w * 0.5).min(h * 0.5);
    let mut b = PathBuilder::new();
    b.move_to(x + r, y);
    b.line_to(x + w - r, y);
    b.quad_to(x + w, y, x + w, y + r);
    b.line_to(x + w, y + h - r);
    b.quad_to(x + w, y + h, x + w - r, y + h);
    b.line_to(x + r, y + h);
    b.quad_to(x, y + h, x, y + h - r);
    b.line_to(x, y + r);
    b.quad_to(x, y, x + r, y);
    b.close();
    b.finish().unwrap()
}

fn rounded_rect(p: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    fill(p, &rounded_path(x, y, w, h, r), c);
}

fn text_width(value: &str, size: f32) -> f32 {
    value.chars().count() as f32 * 8.0 * size
}

fn text_center(p: &mut Pixmap, value: &str, cx: f32, y: f32, size: f32, c: Color) {
    text(
        p,
        value,
        (cx - text_width(value, size) * 0.5).round(),
        y,
        size,
        c,
    );
}

fn text_right(p: &mut Pixmap, value: &str, right: f32, y: f32, size: f32, c: Color) {
    text(p, value, right - text_width(value, size), y, size, c);
}

fn text(p: &mut Pixmap, value: &str, x: f32, y: f32, size: f32, c: Color) {
    let mut b = PathBuilder::new();
    for (i, ch) in value.chars().enumerate() {
        if let Some(glyph) = BASIC_FONTS.get(ch) {
            for (row, bits) in glyph.into_iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        b.push_rect(
                            Rect::from_xywh(
                                x + (i as f32 * 8.0 + col as f32) * size,
                                y + row as f32 * size,
                                size,
                                size,
                            )
                            .unwrap(),
                        );
                    }
                }
            }
        }
    }
    if let Some(path) = b.finish() {
        let mut paint = Paint::default();
        paint.set_color(c);
        paint.anti_alias = false;
        p.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}
