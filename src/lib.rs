/// The height of the NES output video signal
pub const WIDTH: u32 = 256;
/// The width of the NES output video signal
pub const HEIGHT: u32 = 240;

/// CPU frequency constant. This assumes NTSC emulation (instead of PAL).
/// That's also what's emulated in the rest of the ppu.
pub const CPU_FREQ: f64 = 1.789_773 * 1_000_000.0; //hz

mod cpu;
mod ppu;
mod run;
mod screen;

pub use cpu::Cpu;
pub use ppu::mirroring::Mirroring;
pub use ppu::{registers::PpuRegister, Ppu};
pub use run::{run_cpu, run_cpu_headless, run_cpu_headless_for};
pub use screen::Buttons;
