use crate::ppu::colors::Color;
use crate::WIDTH;
use pixels::Pixels;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use winit::window::Window;

/// A struct containg all the buttons for one controller and whether they are pressed (`true`) or not (`false`)
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Buttons {
    pub a: bool,
    pub b: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub start: bool,
}

impl Buttons {
    pub fn get_by_index(self, idx: u8) -> bool {
        match idx {
            0 => self.a,
            1 => self.b,
            2 => self.select,
            3 => self.start,
            4 => self.up,
            5 => self.down,
            6 => self.left,
            7 => self.right,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ButtonName {
    A,
    B,
    Up,
    Down,
    Left,
    Right,
    Start,
    Select,
}

pub enum ScreenReader {
    Dummy,
    Real {
        pixels: Box<Mutex<Pixels>>,
        window: Window,
    },
}

pub enum Message {
    Button(ButtonName, bool),
    Pause(bool),
}

#[derive(Clone)]
pub struct Screen(Arc<ScreenReader>);

pub enum ScreenWriter {
    Dummy,
    Real {
        screen: Screen,
        pixels: Vec<u8>,
        control_rx: Receiver<Message>,
    },
}

impl ScreenWriter {
    pub fn draw_pixel(&mut self, x: usize, y: usize, color: Color) {
        if let Self::Real { pixels, .. } = self {
            pixels[4 * (y * WIDTH as usize + x)] = color.0;
            pixels[4 * (y * WIDTH as usize + x) + 1] = color.1;
            pixels[4 * (y * WIDTH as usize + x) + 2] = color.2;
            pixels[4 * (y * WIDTH as usize + x) + 3] = 0xff;
        }
    }

    pub fn render_frame(&mut self) {
        if let Self::Real { pixels, screen, .. } = self {
            if let ScreenReader::Real {
                pixels: reader_pixels,
                ..
            } = &*screen.0
            {
                reader_pixels
                    .lock()
                    .expect("failed to lock")
                    .frame_mut()
                    .clone_from_slice(pixels);
            }
        }
    }
}

impl Screen {
    pub fn dummy() -> (Screen, ScreenWriter) {
        (Screen(Arc::new(ScreenReader::Dummy)), ScreenWriter::Dummy)
    }

    pub fn new(pixels: Pixels, window: Window) -> (Self, ScreenWriter, Sender<Message>) {
        let buf = pixels.frame().to_vec();
        let (tx, rx) = channel();

        let screen = Screen(Arc::new(ScreenReader::Real {
            pixels: Box::new(Mutex::new(pixels)),
            window,
        }));

        (
            screen.clone(),
            ScreenWriter::Real {
                screen,
                pixels: buf,
                control_rx: rx,
            },
            tx,
        )
    }

    pub fn redraw(&mut self) {
        if let ScreenReader::Real { pixels, .. } = &*self.0 {
            pixels
                .lock()
                .expect("failed to lock")
                .render()
                .expect("failed to render using pixels library");
        }
    }
}
