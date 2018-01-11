#![allow(dead_code)]

#[macro_use] extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate cgmath;
extern crate palette;
extern crate clap;

extern crate portmidi;

use portmidi::{PortMidi, MidiMessage, OutputPort, Result as PmResult};

use gfx::handle::{RenderTargetView, DepthStencilView};
use gfx::traits::{Factory, FactoryExt};
use gfx::{Device, Encoder, PipelineState};
use gfx_device_gl as gl;
use gfx_window_glutin as gfx_glutin;
use glutin::GlContext;

use cgmath::Vector2;

mod ui;

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;

struct MusicBox {
    port: OutputPort,
    key: u8,
    notes: Vec<i32>,
}

impl MusicBox {
    fn new(port: OutputPort) -> Self {
        MusicBox {
            port: port,
            key: 60,
            notes: vec![],
        }
    }

    fn note_on(&mut self, note: i32) -> PmResult<()> {
        let key = self.key as i32 + note;

        if key >= 0 && key <= 127 {
            let msg = MidiMessage {
                status: 0x90,
                data1: key as u8,
                data2: 64,
            };

            self.notes.push(note);
            self.port.write_message(msg)?
        }
        Ok(())
    }

    fn note_off(&mut self, note: i32) -> PmResult<()> {
        let key = self.key as i32 + note;

        let msg = MidiMessage {
            status: 0x80,
            data1: key as u8,
            data2: 64,
        };

        self.port.write_message(msg)
    }

    fn all_notes_off(&mut self) {
        // Better to send ALL NOTES OFF, but there're some synths that don't understand it
        let mut notes = vec![];
        ::std::mem::swap(&mut notes, &mut self.notes);
        for &n in &notes {
            drop(self.note_off(n))
        }
    }
}

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        color: [f32; 4] = "a_Color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

pub trait Render {
    fn render_fan<V>(&mut self, iter: V)
    where V: std::iter::IntoIterator<Item=Vertex>;
}

struct Renderer {
    factory: gl::Factory,
    encoder: Encoder<gl::Resources, gl::CommandBuffer>,
    out_color: RenderTargetView<gl::Resources, ColorFormat>,
    pso: PipelineState<gl::Resources, pipe::Meta>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Renderer {
    pub fn new(
        mut factory: gl::Factory,
        encoder: Encoder<gl::Resources, gl::CommandBuffer>,
        out_color: RenderTargetView<gl::Resources, ColorFormat>
    ) -> Self {
        use gfx::state::{Rasterizer, MultiSample};

        let vs = factory.create_shader_vertex(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/plain_150.glslv"))
        ).unwrap();
        let ps = factory.create_shader_pixel(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/plain_150.glslf"))
        ).unwrap();

        let pso = factory.create_pipeline_state(
            &gfx::ShaderSet::Simple(vs, ps),
            gfx::Primitive::TriangleList,
            Rasterizer {
                samples: Some(MultiSample),
                ..Rasterizer::new_fill()
            },
            pipe::new(),
        ).expect("Failed to create a PSO");

        Renderer {
            factory, encoder, pso, out_color,
            vertices: vec![],
            indices: vec![],
        }
    }

    pub fn update_views(&mut self, window: &glutin::GlWindow, depth: &mut DepthStencilView<gl::Resources, DepthFormat>) {
        gfx_glutin::update_views(&window, &mut self.out_color, depth)
    }

    pub fn draw(&mut self, device: &mut gl::Device) {
        let (vbuf, sl) =
            self.factory.create_vertex_buffer_with_slice(&self.vertices, &*self.indices);
        let data = pipe::Data {
            vbuf,
            out: self.out_color.clone(),
        };

        self.encoder.clear(&data.out, BLACK);
        self.encoder.draw(&sl, &self.pso, &data);
        self.encoder.flush(device);

        self.vertices.clear();
        self.indices.clear();
    }
}

impl Render for Renderer {
    fn render_fan<V>(&mut self, iter: V)
    where V: std::iter::IntoIterator<Item=Vertex> {
        let i0 = self.vertices.len() as u16;
        let mut vs = iter.into_iter();
        self.vertices.push(vs.next().unwrap());
        self.vertices.push(vs.next().unwrap());
        for (i, v) in vs.enumerate() {
            let i = i as u16 + 1;
            self.vertices.push(v);
            self.indices.extend(&[i0, i0+i, i0+i+1]);
        }
    }
}

#[derive(Debug)]
enum Msg {
    Resized(Vector2<f32>),
    LeftPressed(Vector2<f32>),
    LeftReleased,
    Keyboard(glutin::ElementState, glutin::VirtualKeyCode)
}

#[derive(Debug)]
struct Intent {
    mouse_pos: Vector2<f32>
}

impl Intent {
    fn new() -> Self {
        Intent {
            mouse_pos: Vector2::new(0.0, 0.0)
        }
    }

    fn intent(&mut self, ev: glutin::WindowEvent) -> Option<Msg> {
        use glutin::WindowEvent as E;
        use glutin::ElementState::*;
        use glutin::MouseButton::{Left};

        Some(match ev {
            E::Resized(w, h) => Msg::Resized(Vector2::new(w as f32, h as f32)),
            E::CursorMoved {position: (w, h), ..} => {
                self.mouse_pos = Vector2::new(w as f32, h as f32);
                return None
            },
            E::MouseInput {state: Pressed, button:Left, ..} =>
                Msg::LeftPressed(self.mouse_pos),
            E::MouseInput {state: Released, button:Left, ..} => Msg::LeftReleased,
            E::KeyboardInput {input: glutin::KeyboardInput {
                state: es, virtual_keycode: Some(vk), ..
            }, ..} =>
                Msg::Keyboard(es, vk),
            _ => return None,
        })
    }
}

#[derive(Debug)]
struct Model {
    hexes: ui::Hexes,
    notes: Vec<i32>,
    sustain: bool,
}

impl Model {
    fn new(size: Vector2<f32>, layout: ui::Layout) -> Self {
        Model {
            hexes: ui::Hexes::new(size, layout),
            notes: vec![],
            sustain: false,
        }
    }
}

fn model(model: Model, msg: Msg) -> Model {
    use Msg::*;
    use glutin::ElementState::*;
    use glutin::VirtualKeyCode::*;

    let Model { mut hexes, mut notes, mut sustain } = model;

    match msg {
        Resized(wh) =>
            hexes.size = wh,
        LeftPressed(xy) => {
            let note = hexes.press(xy);
            notes.push(note)
        },
        LeftReleased if !sustain => {
            hexes.release_all();
            notes.clear()
        },
        Keyboard(Pressed, Space) =>
            sustain = true,
        Keyboard(Released, Space) => {
            sustain = false;
            hexes.release_all();
            notes.clear()
        },
        _ => (),
    };

    Model { hexes, notes, sustain }
}

fn draw(model: &Model, renderer: &mut Renderer) {
    model.hexes.draw(renderer)
}

fn update_midi(model: &Model, the_box: &mut MusicBox) {
    let mn = model.notes.len();
    let bn = the_box.notes.len();
    if mn > bn {
        for n in &model.notes[bn..] {
            drop(the_box.note_on(*n))
        }
    }
    else if mn == 0 {
        the_box.all_notes_off()
    }
}

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

fn main() {
    let matches = clap::App::new("31key")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A mictotonal keyboard")
        .arg(
            clap::Arg::with_name("edo")
            .long("edo")
            .default_value("31")
            .help("Sets the EDO")
        )
        .get_matches();

    let layout = match matches.value_of("edo") {
        Some("31") => ui::edo31_layout(),
        Some("12") => ui::edo12_layout(),
        Some("53") => ui::edo53_layout(),
        _ => {
            eprintln!("Unsupported EDO");
            return
        },
    };

    let mut events_loop = glutin::EventsLoop::new();
    let builder = glutin::WindowBuilder::new()
        .with_title("Tricesimoprimal Keyboard".to_string())
        .with_dimensions(960, 600);
    let context = glutin::ContextBuilder::new()
        .with_multisampling(8)
        .with_vsync(true);
    let (window, mut device, mut factory, main_color, mut main_depth) =
        gfx_glutin::init::<ColorFormat, DepthFormat>(builder, context, &events_loop);

    let encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let mut renderer = Renderer::new(factory, encoder, main_color);

    let midi = PortMidi::new().unwrap();
    let port = midi.default_output_port(1024).unwrap();
    let mut the_box = MusicBox::new(port);

    let mut mailbox = vec![];
    let mut intent = Intent::new();
    let mut the_model = Model::new(Vector2::new(960.0, 600.0), layout);

    let mut running = true;
    let mut needs_update = true;
    while running {
        events_loop.poll_events(|ev| {
            use glutin::WindowEvent::*;
            if let glutin::Event::WindowEvent {event, ..} = ev {
                match event {
                    Closed => running = false,
                    Resized(w, h) => {
                        renderer.update_views(&window, &mut main_depth);
                        if let Some(msg) = intent.intent(Resized(w, h)) {
                            mailbox.push(msg);
                        }
                    },
                    ev =>
                        if let Some(msg) = intent.intent(ev) {
                            mailbox.push(msg);
                        },
                }
                needs_update = true
            }
        });

        if needs_update {
            for m in mailbox.drain(0..) {
                the_model = model(the_model, m)
            }

            draw(&the_model, &mut renderer);
            update_midi(&the_model, &mut the_box);
            renderer.draw(&mut device);
            window.swap_buffers().unwrap();
            device.cleanup();

            needs_update = false;
        }

        let dt = ::std::time::Duration::from_millis(10);
        ::std::thread::sleep(dt);
    }
}
