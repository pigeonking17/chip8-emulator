// SDL2 library used to display the pixels and read keyboard events.
use sdl2::{pixels::Color, render::Canvas, video::Window, rect::{Rect, Point}, EventPump, keyboard::Keycode, event::Event, surface::{self, Surface}};
// rand library used to generate a random number for 0xCxkk.
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::{sleep, interval};

/// Data structure that holds the current state of the cpu.
pub struct CPU {
    /// 16 one-byte registers that are available for use by the program.
    pub registers: [u8; 16],
    /// Holds the current location in memory.
    pub program_counter: usize,
    /// 4kiB of memory that holds the proram and the font.
    pub memory: [u8; 0x1000],
    /// 16-address stack, allows for 16 nested subroutines.
    pub stack: [u16; 16],
    /// Holds the location of the most recent address added to the stack.
    pub stack_pointer: usize,
    /// A register that holds an address that often points to a sprite.
    pub index_register: u16,
    pub delay_timer: Arc<Mutex<u8>>,
}

impl CPU {
    /// Initialises the window and containes the main cpu loop.
    pub async fn run(&mut self) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        // Generates a window that is 960 px by 480 px.
        let window = video_subsystem.window("CHIP-8 Emulator", 64*15, 32*15)
           .position_centered()
           .build()
           .unwrap();

        // The canvas, this is where the pixels are drawn.
        let mut canvas = window.into_canvas().build().unwrap();
        
        // Increase the scale so that the pixels are visible.
        canvas.set_scale(15.0, 15.0).unwrap();

        // Sets the colour to black, fills the screen and presents it.
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        // This is used to detect keypresses, button presses, etc.
        let mut event_pump = sdl_context.event_pump().unwrap();

        let should_exit = AtomicBool::new(false);
        let mut decrement_future;

        // Main cpu loop.
        'running: loop {
            // Allows the window to be closed.
            for event in event_pump.poll_iter() {
                match event {
                    sdl2::event::Event::Quit {..} => break 'running,
                    _ => {},
                }
            }

            // Get the current opcode.
            let opcode = self.read_opcode();
            // Increment the PC to the next instruction.
            self.program_counter += 2;

            // Splits the opcode into 6 different parts. 0xcxyd, 0x_nnn, and 0x__kk.
            let c = ((opcode & 0xF000) >> 12) as u8;
            let x = ((opcode & 0x0F00) >> 8) as u8;
            let y = ((opcode & 0x00F0) >> 4) as u8;
            let d = ((opcode & 0x000F) >> 0) as u8;

            let nnn = opcode & 0x0FFF;
            let kk = (opcode & 0x00FF) as u8;

            // Decide what to do based on the opcode.
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
                (0xB, _, _, _) => self.jump_offset(nnn),
                (0xC, _, _, _) => self.random(x, kk),
                (0xD, _, _, _) => self.display(x, y, d, &mut canvas),
                (0xE, _, 0x9, 0xE) => self.skip_key_pressed(x, &mut event_pump),
                (0xE, _, 0xA, 0x1) => self.skip_key_npressed(x, &mut event_pump),
                (0xF, _, 0, 0x7) => decrement_future = &self.set_timer(x),
                (0xF, _, 0x1, 0x5) => self.read_timer(x),
                (0xF, _, 0x1, 0x8) => (),
                (0xF, _, 0x1, 0xE) => self.add_to_index(x),
                (0xF, _, 0, 0xA) => self.get_key(x, &mut event_pump),
                (0xF, _, 0x2, 0x9) => self.font(x),
                (0xF, _, 0x3, 0x3) => self.decimal(x),
                (0xF, _, 0x5, 0x5) => self.store_memory(x),
                (0xF, _, 0x6, 0x5) => self.load_memory(x),
                _ => (), //todo!("opcode {:04x}", opcode)
            }
            sleep(Duration::from_micros(100)).await;
        }
    }

    fn load_memory(&mut self, x: u8) {
        for i in 0..=x {
            self.registers[i as usize] = self.memory[(self.index_register + i as u16) as usize];
        }
    }

    fn store_memory(&mut self, x: u8) {
        for i in 0..=x {
            self.memory[(self.index_register + i as u16) as usize] = self.registers[i as usize];
        }
    }

    fn decimal(&mut self, x: u8) {
        let digits = self.registers[x as usize]
            .to_string()
            .chars()
            .map(|d| d.to_digit(10).unwrap())
            .collect::<Vec<_>>();

        for (i, digit) in digits.iter().enumerate() {
            self.memory[(self.index_register + i as u16) as usize] = *digit as u8;
        }
    }

    fn font(&mut self, x: u8) {
        let font_char = self.registers[x as usize] & 0xF;
        self.index_register = (font_char * 5) as u16;
    }

    fn get_key(&mut self, x: u8, event_pump: &mut EventPump) {
        if let Some(key) = self.get_depressed_key(event_pump) {
            self.registers[x as usize] = key;
        } else {
            self.program_counter -= 2;
        }
    }

    fn add_to_index(&mut self, x: u8) {
        let arg1 = self.registers[x as usize];

        let (val, overflow) = self.index_register.overflowing_add(arg1 as u16);
        self.index_register = val;

        if overflow {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }
    }

    fn read_timer(&mut self, x: u8) {
        self.registers[x as usize] = *self.delay_timer.lock().unwrap();
    }

    async fn set_timer(&mut self, x: u8) {
        let mut interval = interval(Duration::from_secs_f64(1.0 / 60.0));
        *self.delay_timer.lock().unwrap() = self.registers[x as usize];
        loop {
            interval.tick().await;
            let mut timer = self.delay_timer.lock().unwrap();
            if *timer > 0 {
                *timer -= 1;
            }
        }
    }

    /// Reads the current two-byte opcode using the PC and memory.
    fn read_opcode(&self) -> u16 {
        let p = self.program_counter;
        let op_byte1 = self.memory[p] as u16;
        let op_byte2 = self.memory[p + 1] as u16;

        // Small hack to merge the two bytes in memory.
        op_byte1 << 8 | op_byte2
    }

    /// Skips to the next instruction if the key in Vx is not pressed.
    fn skip_key_npressed(&mut self, x: u8, event_pump: &mut EventPump) {
        let key = self.get_depressed_key(event_pump);

        match key {
            Some(value) => {
                if self.registers[x as usize] != value {
                    self.program_counter += 2;
                }
            }
            None => (),
        }
    }

    /// Skips to the next instruction if the key in Vx is pressed.
    fn skip_key_pressed(&mut self, x: u8, event_pump: &mut EventPump) {
        let key = self.get_depressed_key(event_pump);

        match key {
            Some(value) => {
                if self.registers[x as usize] == value {
                    self.program_counter += 2;
                }
            },
            None => (),
        }
    }

    /// Function to get any keys that are currently being pressed. Mimics the old 16-key keyboard
    /// that CHIP-8 programs use.
    fn get_depressed_key(&mut self, event_pump: &mut EventPump) -> Option<u8> {
        for event in event_pump.poll_iter() {
            match event {
                Event::KeyDown{ keycode: Some(Keycode::Num1), repeat: false, .. } => { return Some(0x1); },
                Event::KeyDown{ keycode: Some(Keycode::Num2), repeat: false, .. } => { return Some(0x2); },
                Event::KeyDown{ keycode: Some(Keycode::Num3), repeat: false, .. } => { return Some(0x3); },
                Event::KeyDown{ keycode: Some(Keycode::Num4), repeat: false, .. } => { return Some(0xC); },
                Event::KeyDown{ keycode: Some(Keycode::Q), repeat: false, .. } => { return Some(0x4); },
                Event::KeyDown{ keycode: Some(Keycode::W), repeat: false, .. } => { return Some(0x5); },
                Event::KeyDown{ keycode: Some(Keycode::E), repeat: false, .. } => { return Some(0x6); },
                Event::KeyDown{ keycode: Some(Keycode::R), repeat: false, .. } => { return Some(0xD); },
                Event::KeyDown{ keycode: Some(Keycode::A), repeat: false, .. } => { return Some(0x7); },
                Event::KeyDown{ keycode: Some(Keycode::S), repeat: false, .. } => { return Some(0x8); },
                Event::KeyDown{ keycode: Some(Keycode::D), repeat: false, .. } => { return Some(0x9); },
                Event::KeyDown{ keycode: Some(Keycode::F), repeat: false, .. } => { return Some(0xE); },
                Event::KeyDown{ keycode: Some(Keycode::Z), repeat: false, .. } => { return Some(0xA); },
                Event::KeyDown{ keycode: Some(Keycode::X), repeat: false, .. } => { return Some(0x0); },
                Event::KeyDown{ keycode: Some(Keycode::C), repeat: false, .. } => { return Some(0xB); },
                Event::KeyDown{ keycode: Some(Keycode::V), repeat: false, .. } => { return Some(0xF); },
                _ => { return None; }
            }
        }
        return None;
    }

    /// Generates a random u8, bitwise ands it with kk and then stores it in Vx.
    fn random(&mut self, x: u8, kk: u8) {
        let random = rand::thread_rng().gen_range(0..u8::MAX);
        self.registers[x as usize] = random & kk;
    }

    /// Jumps a to an instruction offset by the value of Vx. This allows for decision tables.
    fn jump_offset(&mut self, nnn: u16) {
        let offset = self.registers[0];
        self.program_counter = (nnn + offset as u16) as usize;
    }

    /// Shifts Vx left once. Sets VF to 1 if there is an overflow.
    fn shift_left(&mut self, x: u8) {
        if self.registers[x as usize] & 0x80 == 0x80 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[x as usize] <<= 1;
    }

    /// Shifts Vx right once. Sets VF to 1 if there is an overflow.
    fn shift_right(&mut self, x: u8) {
        if self.registers[x as usize] & 0x1 == 0x1 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[x as usize] >>= 1;
    }

    /// Subtracts Vx from Vy and puts the result in Vx. 
    /// Sets VF to 0 if there is an overflow, otherwise it is set to 1.
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

    /// Subtracts Vy from Vx and puts the value in Vx.
    /// Sets VF to 0 if there is an overflow, otherwise it is set to 1.
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

    /// Sets to Vx to Vy.
    fn set_xy(&mut self, x: u8, y: u8) {
        self.registers[x as usize] = self.registers[y as usize];
    }

    /// Puts the result of Vx OR Vy into Vx.
    fn bitwise_or(&mut self, x: u8, y: u8) {
        self.registers[x as usize] |= self.registers[y as usize];
    }

    /// Putes the value of Vx AND Vy into Vx.
    fn bitwise_and(&mut self, x: u8, y: u8) {
        self.registers[x as usize] &= self.registers[y as usize];
    }

    /// Puts the value of Vx XOR Vy into Vx.
    fn bitwise_xor(&mut self, x: u8, y: u8) {
        self.registers[x as usize] ^= self.registers[y as usize];
    }

    /// Skips to the next instruction if Vx and Vy are not equal.
    fn skip_nequal(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] != self.registers[y as usize] {
            self.program_counter += 2;
        }
    }

    /// Skips to the next instruction if Vx and Vy are equal.
    fn skip_equal(&mut self, x: u8, y: u8) {
        if self.registers[x as usize] == self.registers[y as usize] {
            self.program_counter += 2;
        }
    }

    /// Skips to the next instruction if Vx is not equal to kk.
    fn skip_x_nequal(&mut self, x: u8, kk: u8) {
        if self.registers[x as usize] != kk {
            self.program_counter += 2;
        }
    }

    /// Skips to the next instruction if Vx is equal to kk.
    fn skip_x_equal(&mut self, x: u8, kk: u8) {
        if self.registers[x as usize] == kk {
            self.program_counter += 2;
        }
    }

    /// Displays a sprite found in memory at the index register.
    /// The sprite is n rows tall and is displayed at (Vx, Vy).
    fn display(&mut self, x: u8, y: u8, n: u8, canvas: &mut Canvas<Window>) {
        // Gets the coordinates to display the sprite.
        let mut xp = self.registers[x as usize];
        let mut yp = self.registers[y as usize];
        self.registers[0xF] = 0;

        // Gets the current pixels on the screen, this is because displaying new pixels requires
        // knowing what is currently at that point.
        let mut pixels = canvas.read_pixels(canvas.viewport(), sdl2::pixels::PixelFormatEnum::RGB24).unwrap();

        // Turns the pixels from complicated RBG numbers into simple on/off.
        pixels = pixels.into_iter()
            .map(|pixel| match pixel {
                0 => 0 as u8,
                _ => 1 as u8,
            }).collect::<Vec<u8>>();

        let pixels = pixels.as_slice().chunks(64).collect::<Vec<&[u8]>>();

        // Progressivley display each row, starting at the top.
        'rows: for row in 0..n {
            // If the bottom of the screen is reached then stop.
            if yp >= 32 {
                break;
            }

            // Get the sprite row to display. Each bit in the byte means to flip the current value
            // of the pixel in its place. For example, if the bit is a 1 and the pixel is currently
            // on, then it gets turned off. If the bit is 0, the pixel is not changed.
            let sprite_row = self.memory[(self.index_register + row as u16) as usize];

            // Iterate over each bit in the byte.
            for j in 0..8 {
                // Stops if the end of the screen is reached.
                if xp >= 64 {
                    continue 'rows;
                }
                // Use a bit mask to grab the bit we want.
                let mask = 0x80 >> j;
                match sprite_row & mask {
                    // Matches if the bit we want is 1.
                    1|2|4|8|16|32|64|128 =>
                    // If it the pixel is on, turn it off.
                    if pixels[yp as usize][xp as usize] == 1 {
                        canvas.set_draw_color(Color::RGB(0, 0, 0));
                        canvas.draw_point(Point::new(xp as i32, yp as i32)).unwrap();
                        self.registers[0xF] = 1;
                    // Else if it is off then turn it on.
                    } else if pixels[yp as usize][xp as usize] == 0 {
                        canvas.set_draw_color(Color::RGB(255, 255, 255));
                        canvas.draw_point(Point::new(xp as i32, yp as i32)).unwrap();
                    },
                    // Do nothing if the bit is 0.
                    _ => (),
                }
                // Move over one.
                xp += 1;
            }
            // Go back to the start of the row and go down one row.
            xp -= 8;
            yp += 1;
        }
        // Displays the canvas.
        canvas.present();
    }

    /// Set the index register to nnn.
    fn set_index(&mut self, nnn: u16) {
        self.index_register = nnn;
    }

    /// Adds kk to Vx. Does not affect VF if thers is an overflow.
    fn add(&mut self, x: u8, kk: u8) {
        let val = self.registers[x as usize];

        match val.checked_add(kk) {
            Some(value) => self.registers[x as usize] = value,
            // If an overflow occurs, then set it to it's previous value minus one.
            None => self.registers[x as usize] -= 1,
        }
    }

    /// Sets Vx to kk.
    fn set(&mut self, x: u8, kk: u8) {
        self.registers[x as usize] = kk;
    }

    /// Changes the PC to nnn and stores the prevoius value on the stack to return to it later.
    /// Panics if the stack is full.
    fn call(&mut self, nnn: u16) {
        let sp = self.stack_pointer;
        let stack = &mut self.stack;

        if sp >= stack.len() {
            panic!("Stack overflow!")
        }

        stack[sp] = self.program_counter as u16;
        self.stack_pointer += 1;
        self.program_counter = nnn as usize;
    }

    /// Pops an instruction from stack and set the PC to it.
    /// Panics if the stack is empty.
    fn ret(&mut self) {
        if self.stack_pointer == 0 {
          panic!("Stack underflow");
        }

        self.stack_pointer -= 1;
        let addr = self.stack[self.stack_pointer];
        self.program_counter = addr as usize;
    }

    /// Clears the screen.
    fn clear(&mut self, canvas: &mut Canvas<Window>) {
        canvas.clear();
    }

    /// Sets the PC to nnn.
    fn jump(&mut self, nnn: u16) {
        self.program_counter = nnn as usize;
    }

    /// Adds Vx and Vy and stores the value in Vx. Sets VF to 1 if overflow occurs.
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
