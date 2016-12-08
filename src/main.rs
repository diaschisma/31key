extern crate sfml;
extern crate portmidi;

use sfml::window::*;
use sfml::graphics::*;
use portmidi::{PortMidi, MidiMessage, OutputPort, Result as PmResult};


mod ui;

pub static FONT: &'static [u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/PTM55F.ttf"));

struct MusicBox {
    port: OutputPort,
    notes: Vec<u8>,
    sustain: bool,
}

impl MusicBox {
    fn new(port: OutputPort) -> Self {
        MusicBox {
            port: port,
            notes: vec![],
            sustain: false,
        }
    }

    fn note_on(&mut self, note: u8) -> PmResult<()> {
        let msg = MidiMessage {
            status: 0x90,
            data1: note,
            data2: 64,
        };

        self.notes.push(note);
        self.port.write_message(msg)
    }

    fn note_off(&mut self, note: u8) -> PmResult<()> {
        let msg = MidiMessage {
            status: 0x80,
            data1: note,
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

fn proceed() {
    let midi = PortMidi::new().unwrap();
    let mut the_box = MusicBox::new(midi.default_output_port(1024).unwrap());
    let view = (1920.0, 1080.0);//(960.0, 600.0);

    let mut window = RenderWindow::new(
        VideoMode::new_init(view.0 as u32, view.1 as u32, 32),
        "Tricesimoprimal Keyboard",
        WindowStyle::default(),
        &ContextSettings::default().antialiasing(8),
    ).expect("Cannot create a new Render Window.");
    window.set_key_repeat_enabled(false);

    let mut hexes = ui::Hexes::new(view.0, view.1);

    loop {
        loop {
            use sfml::window::MouseButton::*;
            use sfml::window::Key;

            let event = window.poll_event();
            match event {
                Some(Event::Closed) => return,
                Some(Event::Resized {width: w, height: h}) => {
                    window.set_view(&View::from_rect(&Rect::new(0.0, 0.0, w as f32, h as f32)));
                    hexes.resize(w as f32, h as f32);
                },
                Some(Event::MouseButtonPressed {button: Left, x, y}) => {
                    let note = hexes.press(x, y) + 64;
                    if note < 128 && note > 0 {
                        drop(the_box.note_on(note as u8));
                    }
                },
                Some(Event::MouseButtonReleased {button: Left, ..}) => {
                    if !the_box.sustain {
                        hexes.release();
                        the_box.all_notes_off();
                    }
                },
                Some(Event::KeyPressed {code: Key::Space, ..}) => {
                    the_box.sustain = true;
                },
                /*Some(Event::KeyPressed {code, ctrl, ..}) => the_box.press(code, ctrl),
                Some(Event::TextEntered {code}) => the_box.text(code),*/
                Some(Event::KeyReleased {code: Key::Space, ..}) => {
                    the_box.sustain = false;
                    the_box.all_notes_off();
                    hexes.release();
                },
                None => break,
                _ => (),
            }
        }

        window.clear(&Color::new_rgb(0x21, 0x21, 0x21));
        window.draw(&hexes);

        window.display();
        ::std::thread::sleep(::std::time::Duration::from_millis(25));
    }
}


fn main() {
    proceed()
}
