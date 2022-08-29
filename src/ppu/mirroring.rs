/// Vram has 4 banks, while there is only memory for 2.
/// That means that two of the banks are copies of the other two
///
/// Mirroring changes which banks are copies of which banks.
///
/// 4-screen mirroring is an exception. In that case there is physically
/// more ram on the cartridge so all addresses are accessible.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Mirroring {
    /// VRAM is set up so the first and third are the same, and the second and fourth are the same
    Horizontal,
    /// VRAM is set up so the first and second are the same, and the third and fourth are the same
    Vertical,
    /// All banks are unique
    FourScreen,
    /// All banks are a copy of the first bank
    ///
    /// TODO: this feature may not be fully supported by our emulation. Notably, mirroring can't change
    ///       while some mappers have dynamically changing mirroring modes
    SingleScreenLower,
    /// All banks are a copy of the second bank
    ///
    /// TODO: this feature may not be fully supported by our emulation. See [`Mirroring::SingleScreenLower`]
    SingleScreenUpper,
}
