use cgmath::prelude::*;
use cgmath::{Vector2, Vector3, Matrix2, Rad, Deg, Rotation2, Basis2};

macro_rules! rgb {
    ($r:expr, $g:expr, $b:expr) => {
        Vector3 {
            x: $r as f32 / 255.0,
            y: $g as f32 / 255.0,
            z: $b as f32 / 255.0
        }
    }
}

static COLORS: [Vector3<f32>; 5] = [
    rgb!(0xff, 0xff, 0xff),
    rgb!(0xcf, 0xcf, 0xcf),
    rgb!(0xbb, 0xaa, 0x93),
    rgb!(0x7b, 0x7b, 0x7b),
    rgb!(0xff, 0x9f, 0x41),
];

type Xy = Vector2<f32>;
type Qr<T> = Vector2<T>;
type Color = Vector3<f32>;

fn into_lrgb(rgb: Vector3<f32>) -> [f32; 4] {
    use palette::pixel::Srgb;
    let rgb: ::palette::Rgb = Srgb::new(rgb.x, rgb.y, rgb.z).into();
    rgb.to_pixel()
}

// Combining http://www.redblobgames.com/grids/hexagons/#hex-to-pixel with rotation

fn into_xy(qr: Qr<f32>, radius: f32, angle: Rad<f32>) -> Xy {
    let rot: Basis2<f32> = Rotation2::from_angle(angle);

    let mat = Matrix2::new(
        1.732, 0.0,
        0.866, -1.5,

    );

    rot.rotate_vector(mat * qr * radius)
}


fn into_qr(xy: Xy, radius: f32, angle: Rad<f32>) -> Qr<f32> {
    let rot: Basis2<f32> = Rotation2::from_angle(-angle);

    let mat = Matrix2::new(
        0.577, 0.0,
        0.333, -0.667,
    );

    mat * rot.rotate_vector(xy) / radius
}

fn round_qr(qr: Qr<f32>) -> Qr<i32> {
    let (q, r, s) = (qr.x, qr.y, -qr.x - qr.y);
    let (rq, rr, rs) = (q.round(), r.round(), s.round());
    let (dq, dr, ds) = (rq - q, rr - r, rs - s);

    if dq > dr && dq > ds {
        Vector2::new((-rr - rs) as i32, rr as i32)
    }
    else if dr > ds {
        Vector2::new(rq as i32, (-rq - rs) as i32)
    }
    else {
        Vector2::new(rq as i32, rr as i32)
    }
}

fn hex_corner(center: Xy, size: f32, angle: Rad<f32>, i: u8) -> Xy {
    let phi = ::std::f32::consts::PI / 3.0;
    let angle = angle + Rad(i as f32 * phi);
    let rot: Basis2<f32> = Rotation2::from_angle(angle);
    center + rot.rotate_vector(Vector2 { x: 0.0, y: size })
}

#[derive(Debug)]
pub struct Hexes {
    pub size: Vector2<f32>,
    pub hex_size: f32,
    pub hex_gap: f32,
    pub angle: Rad<f32>,
    pub pressed: Vec<Qr<i32>>,
}

fn midi_note(qr: Qr<i32>) -> u8 {
    (5*qr.x + 3*qr.y + 64) as u8
}

impl Hexes {
    pub fn new(size: Vector2<f32>, pressed: Vec<Qr<i32>>) -> Self {
        Hexes {
            size,
            hex_size: 80.0,
            hex_gap: 2.0,
            angle: Deg(16.102113752).into(),
            pressed,
        }
    }

    fn hex_color(&self, c: Qr<i32>) -> Color {
        let n = match 5*c.x + 3*c.y {
            p if p >= 0 => p,
            n => 248 + n,
        } % 31;
        let cn = match n {
            0 | 5 | 10 | 13 | 18 | 23 | 28 => 0,
            2 | 7 | 12 | 15 | 20 | 25 | 30 => 1,
            29 | 3 | 8 | 11 | 16 | 21 | 26 => 2,
            4 | 9 | 17 | 22 | 27 => 3,
            _ => 4,
        };

        let rgb = COLORS[cn];
        if self.pressed.contains(&c) {
            0.5 * rgb
        } else {
            rgb
        }
    }

    pub fn press(&mut self, xy: Xy) -> u8 {
        let xy = 2.0 * xy - self.size;
        let xy = Vector2::new(xy.x, -xy.y);

        let qr = round_qr(into_qr(xy, self.hex_size + self.hex_gap, self.angle));
        self.pressed.push(qr);
        midi_note(qr)
    }

    pub fn release_all(&mut self) {
        self.pressed.clear()
    }
}

impl Hexes {
    pub fn draw<R: super::Render>(&self, renderer: &mut R) {
        let width = self.size.x / self.size.y;
        let size = self.hex_size / self.size.y;
        let gap = self.hex_gap / self.size.y;

        let c0 = round_qr(into_qr(Vector2::new(-width, -1.0), size, -self.angle));
        let c1 = round_qr(into_qr(Vector2::new(width, 1.0), size, -self.angle));

        fn rn(a: i32, b: i32) -> ::std::ops::Range<i32> {
            if a < b { a..(b+1) }
            else { b..(a+1) }
        }

        for q in rn(c0.x, c1.x) {
            for r in rn(c0.y, c1.y) {
                let qr = Vector2::new(q, r);
                let xy = into_xy(Vector2::new(qr.x as f32, qr.y as f32), size + gap, self.angle);
                let color = self.hex_color(qr);
                let v_it = (0..6).map(|i| {
                    let c = hex_corner(xy, size, self.angle, i);
                    super::Vertex {
                        pos: [c.x / width, c.y],
                        color: into_lrgb(color),
                    }
                });

                renderer.render_fan(v_it)
            }
        }
    }
}
