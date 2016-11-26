use sfml::graphics::{Drawable, RenderTarget, RenderStates, CircleShape, Color, Shape, Transformable};
use sfml::system::Vector2f;

/*
const GAP: f32 = 0.175;

const HORIZ_X: f32 = 0.866 * (2.0 + GAP);
const DIAG_X: f32 = 0.5 * HORIZ_X;
const DIAG_Y: f32 = 1.5 + 0.707 * GAP;
*/

static COLORS: [(u8, u8, u8); 5] = [
    (0xff, 0xff, 0xff),
    (0xcf, 0xcf, 0xcf),
    (0xbb, 0xaa, 0x93),
    (0x7b, 0x7b, 0x7b),
    (0xff, 0x9f, 0x41),
];

type Coord = (i32, i32);

pub struct Hexes {
    radius: f32,
    width: f32,
    height: f32,
    angle: f32,
    pressed: Vec<Coord>,
}

impl Hexes {
    pub fn new(width: f32, height: f32) -> Hexes {
        Hexes {
            radius: 40.0,
            width: width,
            height: height,
            angle: -16.102113752,
            pressed: vec![],
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    fn hex_color(&self, c: Coord) -> Color {
        let n = match 5 * c.0 + 3 * c.1 {
            p if p >= 0 => p % 31,
            n => (248 + n) % 31,
        };
        let cn = match n {
            0 | 5 | 10 | 13 | 18 | 23 | 28 => 0,
            2 | 7 | 12 | 15 | 20 | 25 | 30 => 1,
            29 | 3 | 8 | 11 | 16 | 21 | 26 => 2,
            4 | 9 | 17 | 22 | 27 => 3,
            _ => 4,
        };

        let rgb = COLORS[cn];
        if self.pressed.contains(&c) {
            Color::modulate(Color::new_rgb(rgb.0, rgb.1, rgb.2), Color::new_rgb(0xb0, 0xc0, 0xd0))
        } else {
            Color::new_rgb(rgb.0, rgb.1, rgb.2)
        }
    }

    pub fn press(&mut self, x: i32, y: i32) -> i32 {
        let center = Vector2f {
            x: self.width / 2.0,
            y: self.height / 2.0
        };
        let xy = Vector2f { x: x as f32, y: y as f32 } - center;
        let qr = into_qr(xy, self.radius, self.angle);
        self.pressed.push(qr);

        5 * qr.0 + 3 * qr.1
    }

    pub fn release(&mut self) {
        self.pressed.clear()
    }

    pub fn hl_hex(&mut self, x: i32, y: i32) {
        let center = Vector2f {
            x: self.width / 2.0,
            y: self.height / 2.0
        };
        let xy = Vector2f { x: x as f32, y: y as f32 } - center;
        let qr = into_qr(xy, self.radius, self.angle);
        self.pressed = vec![qr];
    }
}

// Combining http://www.redblobgames.com/grids/hexagons/#hex-to-pixel with rotation
fn into_xy(qr: Coord, radius: f32, angle: f32) -> Vector2f {
    let (q, r) = (qr.0 as f32, qr.1 as f32);
    let angle = 3.14159 * (angle / 180.0);
    let (cos, sin) = (angle.cos(), angle.sin());

    let (a11, a12) = (1.732*cos, 0.866*cos - 1.5*sin);
    let (a21, a22) = (1.732*sin, 0.866*sin + 1.5*cos);

    let (x, y) = (a11*q + a12*r, a21*q + a22*r);

    Vector2f { x: radius * x, y: radius * y }
}

fn into_qr(xy: Vector2f, radius: f32, angle: f32) -> (i32, i32) {
    let (x, y) = (xy.x, xy.y);
    let angle = 3.14159 * (angle / 180.0);
    let (cos, sin) = (angle.cos(), angle.sin());
    let (a11, a12) = (0.577 * cos + 0.333 * sin, 0.577 * sin - 0.333 * cos);
    let (a21, a22) = (-0.667 * sin, 0.667 * cos);

    let (q, r) = ((a11*x + a12*y) / radius, (a21*x + a22*y) / radius);
    let s = -q - r;

    let (rq, rr, rs) = (q.round(), r.round(), s.round());
    let (dq, dr, ds) = (rq - q, rr - r, rs - s);
    if dq > dr && dq > ds {
        ((-rr - rs) as i32, rr as i32)
    }
    else if dr > ds {
        (rq as i32, (-rq - rs) as i32)
    }
    else {
        (rq as i32, rr as i32)
    }
}

fn is_outside(hx: &Hexes, xy: Vector2f) -> bool {
    let pos = (
        xy.x + hx.width / 2.0,
        xy.y + hx.height / 2.0
    );

    pos.0 < 0.0 || pos.0 > hx.width ||
    pos.1 < 0.0 || pos.1 > hx.height
}


impl Drawable for Hexes {
    fn draw<RT: RenderTarget>(&self, target: &mut RT, rs: &mut RenderStates) {
        let center = Vector2f {
            x: self.width / 2.0,
            y: self.height / 2.0
        };
        let mut cs = CircleShape::new_init(self.radius, 6).unwrap();
        cs.set_origin(&Vector2f {
            x: self.radius,
            y: self.radius,
        });
        cs.set_fill_color(&self.hex_color((0,0)));
        cs.set_outline_thickness(2.);
        cs.set_outline_color(&Color::new_rgb(0x00, 0x00, 0x00));
        cs.set_position(&center);
        cs.set_rotation(self.angle);
        cs.draw(target, rs);


        for i in 1.. {
            let xy = into_xy((i, 0), self.radius, self.angle);
            if is_outside(&self, xy) { break }

            cs.set_position(&(xy + center));
            cs.set_fill_color(&self.hex_color((i, 0)));
            cs.draw(target, rs);

            let xy = into_xy((-i, 0), self.radius, self.angle);
            cs.set_position(&(xy + center));
            cs.set_fill_color(&self.hex_color((-i, 0)));
            cs.draw(target, rs);

            let xy = into_xy((0, i), self.radius, self.angle);
            cs.set_fill_color(&self.hex_color((0, i)));
            if is_outside(&self, xy) { continue }

            cs.set_position(&(xy + center));
            cs.draw(target, rs);

            let xy = into_xy((0, -i), self.radius, self.angle);
            cs.set_position(&(xy + center));
            cs.set_fill_color(&self.hex_color((0, -i)));
            cs.draw(target, rs);
        }

        'draw: for i in 1.. {
            let xy = into_xy((i, 0), self.radius, self.angle);
            if is_outside(&self, xy) { break }

            for j in 1.. {
                let xys = [
                    ((i, j), into_xy((i, j), self.radius, self.angle)),
                    ((-i, j), into_xy((-i, j), self.radius, self.angle)),
                    ((i, -j), into_xy((i, -j), self.radius, self.angle)),
                    ((-i, -j), into_xy((-i, -j), self.radius, self.angle)),
                ];
                if xys.iter().all(|&xy| is_outside(&self, xy.1)) { break }
                for &xy in &xys {
                    cs.set_position(&(xy.1 + center));
                    cs.set_fill_color(&self.hex_color(xy.0));
                    cs.draw(target, rs);
                }
            }
        }
    }
}
