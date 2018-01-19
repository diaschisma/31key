#![allow(dead_code)]

#[macro_use] extern crate gfx;
#[macro_use] extern crate serde_derive;

extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;

extern crate serde;
extern crate serde_json;
extern crate ron;

extern crate cgmath;
extern crate palette;
extern crate clap;
extern crate portmidi;

use portmidi::{PortMidi, MidiMessage, OutputPort, Result as PmResult};

use gfx::{Device};
use gfx_window_glutin as gfx_glutin;
use glutin::GlContext;

use cgmath::Vector2;

use renderer::{Render, Vertex};

mod ui;
mod layout;
mod renderer;

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
    fn new(size: Vector2<f32>, layout: layout::Layout) -> Self {
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

fn draw(model: &Model, renderer: &mut renderer::Renderer) {
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

fn main() {
    let matches = clap::App::new("31key")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A mictotonal keyboard")
        .arg(
            clap::Arg::with_name("edo")
            .long("edo")
            .default_value("31")
            .help("Use a predefined layout for a specific EDO")
        )
        .arg(
            clap::Arg::with_name("ron")
            .long("ron")
            .takes_value(true)
            .help("Load the layout from ron")
        )
        .get_matches();

    let layout =
        if let Some(path) = matches.value_of("ron") {
            use std::io::Read;
            let mut file = std::fs::File::open(path).expect("file not found");

            let mut ron = String::new();
            file.read_to_string(&mut ron)
                .expect("something went wrong reading the file");

            let layout: layout::LayoutConfig =
                ron::de::from_str(&ron).unwrap();

            layout.into()
        } else {
            match matches.value_of("edo") {
                Some("31") => layout::edo31_layout(),
                Some("12") => layout::edo12_layout(),
                Some("53") => layout::edo53_layout(),
                _ => {
                    eprintln!("Unsupported EDO");
                    return
                },
            }
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
    let mut renderer = renderer::Renderer::new(factory, encoder, main_color);

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
