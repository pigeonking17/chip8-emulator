use sdl2::{pixels::Color, render::Canvas, video::Window, rect::{Rect, Point}};

pub struct CPU {
    pub registers: [u8; 16],
    pub program_counter: usize,
    pub memory: [u8; 0x1000],
    pub stack: [u16; 16],
    pub stack_pointer: usize,
    pub index_register: u16,
}

impl CPU {
    fn read_opcode(&self) -> u16 {
        let p = self.program_counter;
        let op_byte1 = self.memory[p] as u16;
        let op_byte2 = self.memory[p + 1] as u16;

        op_byte1 << 8 | op_byte2
    }

    pub fn run(&mut self) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem.window("CHIP-8 Emulator", 64*10, 32*10)
            .position_centered()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_scale(10.0, 10.0).unwrap();

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        let mut event_pump = sdl_context.event_pump().unwrap();

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    sdl2::event::Event::Quit {..} => break 'running,
                    _ => {},
                }
            }

            let opcode = self.read_opcode();
            self.program_counter += 2;

            let c = ((opcode & 0xF000) >> 12) as u8;
            let x = ((opcode & 0x0F00) >> 8) as u8;
            let y = ((opcode & 0x00F0) >> 4) as u8;
            let d = ((opcode & 0x000F) >> 0) as u8;

            let nnn = opcode & 0x0FFF;
            let kk = (opcode & 0x00FF) as u8;

            match (c, x, y, d) {
                (0, 0, 0, 0) => { return; },
                (0, 0, 0xE, 0) => self.clear(&mut canvas),
                (0, 0, 0xE, 0xE) => self.ret(),
                (0x1, _, _, _) => self.jump(nnn),
                (0x2, _, _, _) => self.call(nnn),
                (0x3, _, _, _) => self.skip_x_equal(x, kk),
                (0x4, _, _, _) => self.skip_x_nequal(x, kk),
                (0x5, _, _, 0) => self.skip_equal(x, y),
                (0x6, _, _, _) => self.set(x, kk),
                (0x7, _, _, _) => self.add(x, kk),
                (0x8, _, _, 0) => self.set_xy(x, y),
                (0x8, _, _, 0x1) => self.bitwise_or(x, y),
                (0x8, _, _, 0x2) => self.bitwise_and(x, y),
                (0x8, _, _, 0x3) => self.bitwise_xor(x, y),
                (0x8, _, _, 0x4) => self.add_xy(x, y),
                (0x8, _, _, 0x5) => self.sub_xy(x, y),
                (0x8, _, _, 0x6) => self.shift_right(x),
                (0x8, _, _, 0x7) => self.sub_yx(x, y),
                (0x8, _, _, 0xE) => self.shift_left(x),
                (0x9, _, _, 0) => self.skip_nequal(x, y),
                (0xA, _, _, _) => self.set_index(nnn),
                (0xD, _, _, _) => self.display(x, y, d, &mut canvas),
                _ => () //todo!("opcode {:04x}", opcode),
            }
        }
    }

    fn shift_left(&mut self, x: u8) {
        if self.registers[x as usize] & 0x80 == 0x80 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[x as usize] <<= 1;
    }

    fn shift_right(&mut self, x: u8) {
        if self.registers[x as usize] & 0x1 == 0x1 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[x as usize] >>= 1;
    }

    fn sub_yx(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];

        let (val, overflow) = arg2.overflowing_sub(arg1);
        self.registers[x as usize] = val;

        if overflow {
            self.registers[0xF] = 0;
        } else {
            self.registers[0xF] = 1;
        }
    }

    fn sub_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];
        
        let (val, overflow) = arg1.overflowing_sub(arg2);
        self.registers[x as usize] = val;

        if overflow {
            self.registers[0xF] = 0;
        } else {
            self.registers[0xF] = 1;
        }
    }

    fn set_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] = self.registers[y as usize];
    }

    fn bitwise_or(&mut self, x: u8, y: u8) {
        self.registers[x as usize] |= self.registers[y as usize];
    }

    fn bitwise_and(&mut self, x: u8, y: u8) {
        self.registers[x as usize] &= self.registers[y as usize];
    }

    fn bitwise_xor(&mut self, x: u8, y: u8) {
        self.registers[x as usize] ^= self.registers[y as usize];
    }

    fn skip_nequal(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] != self.registers[y as usize] {
            self.program_counter += 2;
        }
    }

    fn skip_equal(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] == self.registers[y as usize] {
            self.program_counter += 2;
        }
    }

    fn skip_x_nequal(&mut self, x: u8, kk: u8) {
        if self.registers[x as usize] != kk {
            self.program_counter += 2;
        }
    }

    fn skip_x_equal(&mut self, x: u8, kk: u8) {
        if self.registers[x as usize] == kk {
            self.program_counter += 2;
        }
    }

    fn display(&mut self, x: u8, y: u8, n: u8, canvas: &mut Canvas<Window>) {
        let mut xp = self.registers[x as usize] & 63;
        let mut yp = self.registers[y as usize] & 63;
        self.registers[0xF] = 0;

        let rect = Rect::new(0, 0, 64, 32);
        let mut pixels = canvas.read_pixels(rect, sdl2::pixels::PixelFormatEnum::RGB24).unwrap();

        pixels = pixels.into_iter()
            .map(|pixel| match pixel {
                0 => 0 as u8,
                _ => 1 as u8,
            }).collect::<Vec<u8>>();

        let pixels = pixels.as_slice().chunks(64).collect::<Vec<&[u8]>>();

        'rows: for row in 0..n {
            if yp >= 32 {
                break;
            }
            
            let sprite_row = self.memory[(self.index_register + row as u16) as usize];
            
            for j in 0..8 {
                if xp >= 64 {
                    continue 'rows;
                }
                let mask = 0x80 >> j;
                match sprite_row & mask {
                    1|2|4|8|16|32|64|128 => if pixels[yp as usize][xp as usize] == 1 {
                        canvas.set_draw_color(Color::RGB(0, 0, 0));
                        canvas.draw_point(Point::new(xp as i32, yp as i32)).unwrap();
                        self.registers[0xF] = 1;
                    } else if pixels[yp as usize][xp as usize] == 0 {
                        canvas.set_draw_color(Color::RGB(255, 255, 255));
                        canvas.draw_point(Point::new(xp as i32, yp as i32)).unwrap();
                    },
                    _ => (),
                }
                xp += 1;
            }
            xp -= 8;
            yp += 1;
        }
        canvas.present();
    }

    fn set_index(&mut self, nnn: u16) {
        self.index_register = nnn;
    }

    fn add(&mut self, x: u8, kk: u8) {
        let val = self.registers[x as usize];

        match val.checked_add(kk) {
            Some(value) => self.registers[x as usize] = value,
            None => self.registers[x as usize] -= 1,
        }
    }

    fn set(&mut self, x: u8, kk: u8) {
        self.registers[x as usize] = kk;
    }

    fn call(&mut self, addr: u16) {
        let sp = self.stack_pointer;
        let stack = &mut self.stack;

        if sp >= stack.len() {
            panic!("Stack overflow!")
        }

        stack[sp] = self.program_counter as u16;
        self.stack_pointer += 1;
        self.program_counter = addr as usize;
    }

    fn ret(&mut self) {
        if self.stack_pointer == 0 {
          panic!("Stack underflow");
        }

        self.stack_pointer -= 1;
        let addr = self.stack[self.stack_pointer];
        self.program_counter = addr as usize;
    }

    fn clear(&mut self, canvas: &mut Canvas<Window>) {
        canvas.clear();
    }

    fn jump(&mut self, addr: u16) {
        self.program_counter = addr as usize;
    }

    fn add_xy(&mut self, x: u8, y: u8) {
        let arg1 = self.registers[x as usize];
        let arg2 = self.registers[y as usize];

        let (val, overflow) = arg1.overflowing_add(arg2);
        self.registers[x as usize] = val;

        if overflow {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }
    }
}
