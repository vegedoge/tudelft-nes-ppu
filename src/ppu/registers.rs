/// There are 8 PPU registers, available to the cpu.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PpuRegister {
    /// should be mapped at 0x2000
    Controller = 0,
    /// should be mapped at 0x2001
    Mask = 1,
    /// should be mapped at 0x2002
    Status = 2,
    /// should be mapped at 0x2003
    OamAddress = 3,
    /// should be mapped at 0x2004
    OamData = 4,
    /// should be mapped at 0x2005
    Scroll = 5,
    /// should be mapped at 0x2006
    Address = 6,
    /// should be mapped at 0x2007
    Data = 7,
}

pub(crate) struct ControllerRegister {
    pub(crate) nametable_address: u16,
    pub(crate) vram_increment: u16,
    pub(crate) sprite_pattern_address: u16,
    pub(crate) background_pattern_address: u16,
    pub(crate) sprite_size: (u8, u8),
    pub(crate) master_slave_select: bool,

    /// When the scanline reaches 240, usually an NMI is generated. However,
    /// whether it does or not is controlled by this flag.
    pub(crate) should_generate_vblank_nmi: bool,

    binary_value: u8,
}

impl Default for ControllerRegister {
    fn default() -> Self {
        let mut s = Self {
            nametable_address: 0x2000,
            vram_increment: 0,
            sprite_pattern_address: 0,
            background_pattern_address: 0,
            sprite_size: (0, 0),
            master_slave_select: false,
            should_generate_vblank_nmi: false,
            binary_value: 0,
        };
        s.write(0);
        s
    }
}

impl ControllerRegister {
    pub fn write(&mut self, value: u8) {
        self.vram_increment = if (value & 0b00000100) > 0 { 32 } else { 1 };
        self.sprite_pattern_address = if (value & 0b00001000) > 0 {
            0x1000
        } else {
            0x0000
        };
        self.background_pattern_address = if (value & 0b0001_0000) > 0 {
            0x1000
        } else {
            0x0000
        };
        self.sprite_size = if (value & 0b0010_0000) > 0 {
            (8, 16)
        } else {
            (8, 8)
        };

        self.master_slave_select = (value & 0b0100_0000) > 0;
        self.should_generate_vblank_nmi = (value & 0b1000_0000) > 0;

        self.binary_value = value;
    }
}

pub(crate) struct MaskRegister {
    pub(crate) greyscale: bool,
    pub(crate) show_bg_left: bool,
    pub(crate) show_sprites_left: bool,
    pub(crate) show_background: bool,
    pub(crate) show_sprites: bool,
    pub(crate) emph_red: bool,
    pub(crate) emph_green: bool,
    pub(crate) emph_blue: bool,

    binary_value: u8,
}

impl Default for MaskRegister {
    fn default() -> Self {
        let mut s = Self {
            greyscale: false,
            show_bg_left: false,
            show_sprites_left: false,
            show_background: false,
            show_sprites: false,
            emph_red: false,
            emph_green: false,
            emph_blue: false,
            binary_value: 0,
        };
        s.write(0);
        s
    }
}

impl MaskRegister {
    pub fn write(&mut self, value: u8) {
        self.greyscale = (value & 0b0000_0001) > 0;
        self.show_bg_left = (value & 0b0000_0010) > 0;
        self.show_sprites_left = (value & 0b0000_0100) > 0;
        self.show_background = (value & 0b0000_1000) > 0;
        self.show_sprites = (value & 0b0001_0000) > 0;
        self.emph_red = (value & 0b0010_0000) > 0;
        self.emph_green = (value & 0b0100_0000) > 0;
        self.emph_blue = (value & 0b1000_0000) > 0;

        self.binary_value = value;
    }
}

#[derive(Default, Debug)]
pub(crate) struct StatusRegister {
    pub(crate) sprite_overflow: bool,
    pub(crate) sprite_zero_hit: bool,
    pub(crate) vblank_started: bool,
}

impl StatusRegister {
    pub fn read(&mut self) -> u8 {
        let value = (self
            .sprite_overflow
            .then_some(0b0010_0000)
            .unwrap_or_default())
            | (self
                .sprite_zero_hit
                .then_some(0b0100_0000)
                .unwrap_or_default())
            | (self
                .vblank_started
                .then_some(0b1000_0000)
                .unwrap_or_default());

        // SOMEHOW this is the expected behavior
        self.vblank_started = false;

        value
    }
}

#[derive(Default)]
pub(crate) struct AddrRegister {
    pub(crate) addr: u16,
}

impl AddrRegister {
    pub fn write(&mut self, value: u8, scroll_addr_latch: bool) {
        if scroll_addr_latch {
            self.addr &= 0x00ff;
            self.addr |= u16::from(value) << 8;
        } else {
            self.addr &= 0xff00;
            self.addr |= u16::from(value);
        }

        if self.addr > 0x3fff {
            self.addr &= 0x3fff;
        }
    }
}

#[derive(Default)]
pub(crate) struct OamAddrRegister {
    pub(crate) addr: u8,
}

impl OamAddrRegister {
    pub fn write(&mut self, value: u8) {
        self.addr = value;
    }
}

#[derive(Default)]
pub(crate) struct ScrollRegister {
    pub(crate) x: u8,
    pub(crate) y: u8,
}

impl ScrollRegister {
    pub fn write(&mut self, value: u8, scroll_addr_latch: bool) {
        if scroll_addr_latch {
            self.y = value;
        } else {
            self.x = value;
        }
    }
}
