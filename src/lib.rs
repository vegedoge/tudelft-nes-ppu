pub const WIDTH: u32 = 256;
pub const HEIGHT: u32 = 240;
// NTSC
pub const CPU_FREQ: f64 = 1.789773 * 1_000_000.0; //hz

mod cpu;
mod ppu;
mod run;
mod screen;

pub use cpu::Cpu;
pub use ppu::mirroring::Mirroring;
pub use ppu::{registers::PpuRegister, Ppu};
pub use run::{run_cpu, run_cpu_headless};
