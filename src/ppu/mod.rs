use crate::cpu::Cpu;
use crate::ppu::colors::{Color, NES_COLOR_PALLETE};
use crate::ppu::registers::{
    AddrRegister, ControllerRegister, MaskRegister, OamAddrRegister, ScrollRegister, StatusRegister,
};
use crate::screen::{Buttons, ScreenWriter};
use crate::{Mirroring, HEIGHT, WIDTH};
use itertools::Itertools;
use registers::PpuRegister;
use std::default::Default;

pub mod colors;
pub mod mirroring;
pub mod registers;

/// Emulating an NTSC PPU chip
pub struct Ppu {
    /// how many lines we've drawn. After 240, an NMI is given to the cpu
    /// and only at 262 does it reset to 0
    scanline: usize,
    /// how many cycles we've had on this line. Resets after reaching 341.
    /// Note that the screen is only 256 pixels wide. So There's a small h-blank period.
    line_progress: usize,

    controller_register: ControllerRegister,
    mask_register: MaskRegister,
    status_register: StatusRegister,
    addr: AddrRegister,
    scroll: ScrollRegister,

    oam_addr: OamAddrRegister,

    scroll_addr_latch: bool,

    palette_table: [u8; 32],
    /// In normal operation, only the first 2048 bytes are used. That is because the
    /// original PPU had only 2048 bytes of vram and addresses in the higher 2048 bytes would
    /// be a mirror of parts in the lower 2048 bytes. However, if 4-screen [`Mirroring`] is selected
    /// then the upper 2048 bytes are actually used (on real hardware that meant that the cartridge
    /// itself came with more vram)
    vram: [u8; 4096],

    oam: [u8; 256],
    secondary_oam: [u8; 32],

    bus: u8,
    // when reading from the ppu, everything is always lagging behind.
    // new reads go into the data buffer, and when you read you read the old buffer
    data_buffer: u8,

    mirroring: Mirroring,

    pub(crate) buttons: Buttons,
}

impl Ppu {
    /// Creates a new PPU. The mirroring mode needs to be given and is constant
    /// for the lifetime of the emulator. Some real-world memory mappers could howerver
    /// change this in the middle of running a game. This is currently *not* supported
    /// by the emulator.
    pub fn new(mirroring: Mirroring) -> Self {
        Self {
            scanline: 0,
            line_progress: 0,
            controller_register: Default::default(),
            mask_register: Default::default(),
            status_register: Default::default(),
            addr: Default::default(),
            scroll: Default::default(),
            oam_addr: Default::default(),
            scroll_addr_latch: true,
            palette_table: [0; 32],
            vram: [0; 4096],
            oam: [0; 256],
            secondary_oam: [0xff; 32],
            bus: 0,
            data_buffer: 0,
            mirroring,
            buttons: Buttons::default(),
        }
    }

    fn vram_read_mirrored(&self, addr: u16) -> u8 {
        self.vram[(self.mirror_address(addr) - 0x2000) as usize]
    }

    fn mirror_address(&self, addr: u16) -> u16 {
        let addr = if addr > 0x2fff {
            addr - 0x1000
        } else if addr < 0x2000 {
            addr + 0x1000
        } else {
            addr
        };

        match self.mirroring {
            Mirroring::Horizontal => match addr {
                0x2000..=0x023ff => addr,
                0x2400..=0x027ff => addr - 0x400,
                0x2800..=0x02bff => addr - 0x400,
                0x2c00..=0x02fff => addr - 0x800,
                a => panic!("invalid address for vram mirroring 0x{a:x}"),
            },
            Mirroring::Vertical => match addr {
                0x2000..=0x027ff => addr,
                0x2800..=0x02fff => addr - 0x800,
                a => panic!("invalid address for vram mirroring 0x{a:x}"),
            },
            Mirroring::FourScreen => addr,
            Mirroring::SingleScreenLower => match addr {
                0x2000..=0x023ff => addr,
                0x2400..=0x027ff => addr - 0x400,
                0x2800..=0x02bff => addr - 0x800,
                0x2c00..=0x02fff => addr - 0xc00,
                a => panic!("invalid address for vram mirroring 0x{a:x}"),
            },
            Mirroring::SingleScreenUpper => match addr {
                0x2000..=0x023ff => addr + 0x400,
                0x2400..=0x027ff => addr,
                0x2800..=0x02bff => addr - 0x400,
                0x2c00..=0x02fff => addr - 0x800,
                a => panic!("invalid address for vram mirroring: 0x{a:x}"),
            },
        }
    }

    /// Gets what buttons are currently pressed by the user/player.
    pub fn get_joypad_state(&self) -> Buttons {
        self.buttons
    }

    /// Write to a register of the PPU. This is supposed to be called from the CPU when a write occurs
    /// to one of the addresses as defined in the spec (and also mentioned in the docs of [`PpuRegister`])
    pub fn write_ppu_register(&mut self, register: PpuRegister, value: u8) {
        self.bus = value;

        match register {
            PpuRegister::Controller => self.controller_register.write(value),
            PpuRegister::Mask => self.mask_register.write(value),
            PpuRegister::Status => { /* Nothing */ }
            PpuRegister::OamAddress => {
                self.oam_addr.write(value);
            }
            PpuRegister::OamData => {
                self.oam[self.oam_addr.addr as usize] = value;
                self.oam_addr.addr = self.oam_addr.addr.wrapping_add(1);
            }
            PpuRegister::Scroll => {
                self.scroll.write(value, !self.scroll_addr_latch);
                self.scroll_addr_latch = !self.scroll_addr_latch;
            }
            PpuRegister::Address => {
                self.addr.write(value, self.scroll_addr_latch);
                self.scroll_addr_latch = !self.scroll_addr_latch;
            }
            PpuRegister::Data => {
                match self.addr.addr {
                    a @ 0..=0x1fff => log::debug!("write to read-only part of memory (chr rom) through ppu data register: 0x{a:0x}"),
                    a @ 0x2000..=0x2fff => {
                        self.vram[self.mirror_address(a) as usize - 0x2000] = value
                    }
                    a @ 0x3000..=0x3eff => {
                        self.vram[self.mirror_address(a - 0x1000) as usize - 0x2000] = value
                    }
                    a @ (0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c) => {
                        self.palette_table[(a - 0x3f10) as usize] = value;
                    }
                    a @ 0x3f00..=0x3fff => {
                        self.palette_table[(a - 0x3f00) as usize & 31] = value;
                    }
                    x => log::debug!("data written to data register is out of bounds for ppu memory (too big): 0x{x:x}"),
                };

                self.addr.addr += self.controller_register.vram_increment;
                if self.addr.addr > 0x3fff {
                    self.addr.addr &= 0x3fff;
                }
            }
        }
    }

    /// Read from a register of the PPU. This is supposed to be called from the CPU when a read occurs
    /// to one of the addresses as defined in the spec (and also mentioned in the docs of [`PpuRegister`])
    ///
    /// We ask for a reference to the cpu here, since we sometimes need to read from the cartridge.
    pub fn read_ppu_register(&mut self, register: PpuRegister, cpu: &impl Cpu) -> u8 {
        match register {
            PpuRegister::Controller => {}
            PpuRegister::Mask => {}
            PpuRegister::Status => {
                let value = self.status_register.read();
                self.bus &= 0b00011111;
                self.bus |= value;
                self.scroll_addr_latch = true;
            }
            PpuRegister::OamAddress => {}
            PpuRegister::OamData => {}
            PpuRegister::Scroll => {
                self.scroll_addr_latch = true;
            }
            PpuRegister::Address => {
                self.scroll_addr_latch = true;
            }
            PpuRegister::Data => {
                self.bus = match self.addr.addr {
                    a @ 0..=0x1fff => {
                        let result = self.data_buffer;
                        self.data_buffer = cpu.ppu_read_chr_rom(a);
                        result
                    }
                    a @ 0x2000..=0x2fff => {
                        let result = self.data_buffer;
                        self.data_buffer = self.vram[self.mirror_address(a) as usize - 0x2000];
                        result
                    }
                    a @ 0x3000..=0x3eff => {
                        let result = self.data_buffer;
                        self.data_buffer =
                            self.vram[self.mirror_address(a - 0x1000) as usize - 0x2000];
                        result
                    }
                    a @ (0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c) => {
                        self.palette_table[a as usize - 0x3f10]
                    }
                    a @ 0x3f00..=0x3fff => self.palette_table[(a as usize - 0x3f00) & 31],
                    x => panic!("address written to data register out of bounds for ppu memory (too big): 0x{x:x}"),
                };

                self.addr.addr += self.controller_register.vram_increment;
                if self.addr.addr > 0x3fff {
                    self.addr.addr &= 0x3fff;
                }
            }
        }
        self.bus
    }

    /// For writes to 0x4041 (see NES docs at [https://www.nesdev.org/wiki/PPU_registers#OAMDMA](https://www.nesdev.org/wiki/PPU_registers#OAMDMA))
    ///
    /// There is no real DMA. When a value is written to 0x4014, you are supposed to pass the PPU
    /// the right 256 bytes instantly, through this function.
    pub fn write_oam_dma(&mut self, data_to_write: [u8; 256]) {
        self.oam = data_to_write;
    }

    fn update_scanline(&mut self, cpu: &mut impl Cpu, screen: &mut ScreenWriter) {
        self.line_progress += 1;

        if self.line_progress >= 257 && self.line_progress <= 320 {
            self.oam_addr.write(0);
        }

        if self.line_progress > 340 {
            self.line_progress -= 341;

            self.scanline += 1;

            // read oam
            self.secondary_oam = [0xff; 32];
            let mut sprite_index = 0;
            for (index, sprite) in self
                .oam
                .iter()
                .copied()
                .skip(self.oam_addr.addr as usize)
                .tuples::<(_, _, _, _)>()
                .enumerate()
            {
                if self.scanline >= sprite.0 as usize
                    && self.scanline
                        < sprite.0 as usize + self.controller_register.sprite_size.1 as usize
                {
                    if sprite_index == 8 {
                        self.status_register.sprite_overflow = true;
                        break;
                    }

                    let b2 = sprite.2 & 0b1110_0011;

                    self.secondary_oam[sprite_index * 4] = sprite.0;
                    self.secondary_oam[sprite_index * 4 + 1] = sprite.1;
                    // We set bit 2 of byte 2 whenever this sprite is sprite 0 (it has special behavior).
                    // In the actual NES hardware this bit is *unused* so we *abuse* it
                    self.secondary_oam[sprite_index * 4 + 2] =
                        b2 | (index == 0).then_some(0b0000_0100).unwrap_or(0);
                    self.secondary_oam[sprite_index * 4 + 3] = sprite.3;
                    sprite_index += 1;
                }
            }

            // we've just passed the 240th line, vblank begins!
            if self.scanline == 241 {
                self.start_vblank(cpu, screen)
            }

            if self.scanline > 261 {
                self.scanline = 0;
                self.end_vblank()
            }
        }
    }

    fn start_vblank(&mut self, cpu: &mut impl Cpu, screen: &mut ScreenWriter) {
        self.status_register.vblank_started = true;
        self.status_register.sprite_zero_hit = false;
        self.status_register.sprite_overflow = false;

        if self.controller_register.should_generate_vblank_nmi {
            cpu.non_maskable_interrupt();
        }

        screen.render_frame();
    }

    fn end_vblank(&mut self) {
        self.status_register.vblank_started = false;
    }

    fn get_palette(&self, tile_x: usize, tile_y: usize, attr_table: u16) -> [Color; 4] {
        let index = tile_y / 4 * 8 + tile_x / 4;
        let attr = self.vram_read_mirrored(attr_table + index as u16);

        let palette_index = match (tile_x % 4 / 2, tile_y % 4 / 2) {
            (0, 0) => attr & 0b11,
            (1, 0) => (attr >> 2) & 0b11,
            (0, 1) => (attr >> 4) & 0b11,
            (1, 1) => (attr >> 6) & 0b11,
            _ => unreachable!(),
        };

        let start = 1 + (palette_index as usize) * 4;

        let mask = if self.mask_register.greyscale {
            0x30
        } else {
            0xff
        };

        [
            NES_COLOR_PALLETE[(self.palette_table[0] & mask) as usize],
            NES_COLOR_PALLETE[(self.palette_table[start] & mask) as usize],
            NES_COLOR_PALLETE[(self.palette_table[start + 1] & mask) as usize],
            NES_COLOR_PALLETE[(self.palette_table[start + 2] & mask) as usize],
        ]
    }

    fn get_sprite_palette(&self, palette_index: u8) -> [Color; 4] {
        let start = 0x11 + (palette_index * 4) as usize;

        let mask = if self.mask_register.greyscale {
            0x30
        } else {
            0xff
        };

        [
            NES_COLOR_PALLETE[0],
            NES_COLOR_PALLETE[(self.palette_table[start] & mask) as usize],
            NES_COLOR_PALLETE[(self.palette_table[start + 1] & mask) as usize],
            NES_COLOR_PALLETE[(self.palette_table[start + 2] & mask) as usize],
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_pixel(
        &self,
        cpu: &mut impl Cpu,
        screen: &mut ScreenWriter,
        x: usize,
        y: usize,
        scroll_x: u8,
        scroll_y: u8,
        name_table_address: u16,
    ) {
        let scrolled_x = (x as isize + scroll_x as isize).rem_euclid(WIDTH as isize * 2) as usize;
        let scrolled_y = (y as isize + scroll_y as isize).rem_euclid(HEIGHT as isize * 2) as usize;

        let name_table_idx = (scrolled_x / WIDTH as usize) + (scrolled_y / HEIGHT as usize) * 2;
        assert!(name_table_idx < 4);

        let tile_nametable_address = name_table_address + (name_table_idx * 0x400) as u16;
        let attr_table = tile_nametable_address + 0x3c0;

        let tile_x = (scrolled_x / 8) % 32;
        let tile_y = (scrolled_y / 8) % 30;

        let off = tile_x + tile_y * 32;

        let tile_num = self.vram_read_mirrored(tile_nametable_address + off as u16) as usize;

        let palette = self.get_palette(tile_x, tile_y, attr_table);

        let tile_x_off = 7 - (scrolled_x % 8);
        let tile_y_off = scrolled_y % 8;

        let bank = self.controller_register.background_pattern_address;

        let byte_upper = cpu.ppu_read_chr_rom(bank + (tile_num * 16 + tile_y_off) as u16);
        let byte_lower = cpu.ppu_read_chr_rom(bank + (tile_num * 16 + tile_y_off + 8) as u16);

        let bit_upper = (byte_upper & 1 << tile_x_off) != 0;
        let bit_lower = (byte_lower & 1 << tile_x_off) != 0;

        let mut color = match (bit_lower, bit_upper) {
            (false, false) => palette[0],
            (false, true) => palette[1],
            (true, false) => palette[2],
            (true, true) => palette[3],
        };

        if self.mask_register.emph_red {
            color.0 = 0xff;
        }
        if self.mask_register.emph_green {
            color.1 = 0xff;
        }
        if self.mask_register.emph_blue {
            color.2 = 0xff;
        }

        screen.draw_pixel(x, y, color);
    }

    #[inline]
    fn blanking(&self) -> bool {
        !(self.line_progress < 256 && self.scanline < 240)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_sprite_pixel(
        &self,
        cpu: &mut impl Cpu,
        screen: &mut ScreenWriter,
        sprite: [u8; 4],

        x: usize,
        y: usize,
        mut sprite_x_off: u16,
        mut sprite_y_off: u16,
        scroll_x: u8,
        scroll_y: u8,
        name_table: u16,
    ) -> bool {
        let mut sprite_zero_hit = false;

        let tile_num = sprite[1] as u16;

        let flip_y = sprite[2] & 0b1000_0000 > 0;
        let flip_x = sprite[2] & 0b0100_0000 > 0;
        if flip_y {
            sprite_y_off = (self.controller_register.sprite_size.1 as u16 - 1) - sprite_y_off;
        }
        if !flip_x {
            sprite_x_off = 7 - sprite_x_off;
        }

        let (bank, tile_num) = if self.controller_register.sprite_size.1 == 16 {
            let old_tile_num = tile_num;
            let tile_num = if sprite_y_off > 7 {
                sprite_y_off -= 8;
                tile_num | 0x0001
            } else {
                tile_num & 0xfffe
            };

            (
                (old_tile_num & 1 == 1).then_some(0x1000).unwrap_or(0),
                tile_num,
            )
        } else {
            (self.controller_register.sprite_pattern_address, tile_num)
        };

        let palette = self.get_sprite_palette(sprite[2] & 0b0000_0011);

        let byte_upper = cpu.ppu_read_chr_rom(bank + (tile_num * 16 + sprite_y_off) as u16);
        let byte_lower = cpu.ppu_read_chr_rom(bank + (tile_num * 16 + sprite_y_off + 8) as u16);

        let bit_upper = (byte_upper & 1 << sprite_x_off) != 0;
        let bit_lower = (byte_lower & 1 << sprite_x_off) != 0;

        let mut color = match (bit_lower, bit_upper) {
            (false, false) => return sprite_zero_hit,
            (false, true) => palette[1],
            (true, false) => palette[2],
            (true, true) => palette[3],
        };

        // this is our *abused* bit that's unused in the actual NES, but tells
        // us that this is sprite 0 we're drawing
        if sprite[2] & 0b0000_0100 > 0 {
            sprite_zero_hit = true;
        }

        let behind_background = sprite[2] & 0b0010_0000 > 0;

        if behind_background {
            self.draw_pixel(cpu, screen, x, y, scroll_x, scroll_y, name_table);
            return sprite_zero_hit;
        }

        if self.mask_register.emph_red {
            color.0 = 0xff;
        }
        if self.mask_register.emph_green {
            color.1 = 0xff;
        }
        if self.mask_register.emph_blue {
            color.2 = 0xff;
        }

        screen.draw_pixel(x, y, color);

        sprite_zero_hit
    }

    fn draw_sprites(
        &self,
        cpu: &mut impl Cpu,
        screen: &mut ScreenWriter,

        scroll_x: u8,
        scroll_y: u8,
        name_table: u16,
    ) -> bool {
        let mut sprite_zero_hit = false;

        for i in (0..8).rev() {
            let sprite_y = self.secondary_oam[i * 4];
            let sprite_1 = self.secondary_oam[i * 4 + 1];
            let sprite_2 = self.secondary_oam[i * 4 + 2];
            let sprite_x = self.secondary_oam[i * 4 + 3];

            if self.line_progress >= sprite_x as usize
                && self.line_progress < sprite_x as usize + 8
                && sprite_y != 0xff
            {
                sprite_zero_hit |= self.draw_sprite_pixel(
                    cpu,
                    screen,
                    [sprite_y, sprite_1, sprite_2, sprite_x],
                    self.line_progress,
                    self.scanline,
                    (self.line_progress - sprite_x as usize) as u16,
                    (self.scanline - sprite_y as usize) as u16,
                    scroll_x,
                    scroll_y,
                    name_table,
                );
            }
        }

        sprite_zero_hit
    }

    /// the screen is optional, since sometimes there is no screen (headless mode)
    pub(crate) fn update(&mut self, cpu: &mut impl Cpu, screen: &mut ScreenWriter) {
        self.update_scanline(cpu, screen);

        if !self.blanking() {
            let nametable_addr = self.controller_register.nametable_address;

            self.draw_pixel(
                cpu,
                screen,
                self.line_progress,
                self.scanline,
                self.scroll.x,
                self.scroll.y,
                nametable_addr,
            );

            if self.draw_sprites(cpu, screen, self.scroll.x, self.scroll.y, nametable_addr) {
                self.status_register.sprite_zero_hit = true;
            }
        }
    }
}
