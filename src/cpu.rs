use crate::Ppu;
use std::error::Error;

/// To use your cpu with the provided PPU library, you need to implement this trait for
/// your CPU.
pub trait Cpu {
    type TickError: Error;

    /// Called every cpu cycle. Note that some instructions take multiple cycles, which
    /// is important for some games to work properly. That means that it *won't* work to
    /// execute an entire instruction every time tick is called. It should take *multiple*
    /// calls to tick to execute one instruction.
    fn tick(&mut self, ppu: &mut Ppu) -> Result<(), Self::TickError>;

    /// This method is called when the PPU (implemented by us) wants to read a byte from memory.
    /// The byte that is actually read, may depend on the current mapper state. Since you implement
    /// the mapper, you should make sure the correct byte is returned here.
    fn ppu_read_chr_rom(&self, offset: u16) -> u8;

    /// Only needed when the specific mapper you implement has character RAM, writable memory
    /// on the cartridge. Most games don't require this. If you just don't implement this
    /// method it will default to ignoring all writes (as if there was only character ROM, not RAM)
    fn ppu_memory_write(&mut self, address: u16, value: u8);

    /// Sometimes the PPU needs to give a non-maskable interrupt to the cpu. If it does, this method
    /// is called by the PPU.
    fn non_maskable_interrupt(&mut self);
}
