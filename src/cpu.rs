use sdl2::{pixels::Color, render::Canvas, video::Window};

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

        let window = video_subsystem.window("CHIP-8 Emulator", 64, 32)
            .position_centered()
            .build()
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();

        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();

        let mut event_pump = sdl_context.event_pump().unwrap();

        loop {
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
                (0x6, _, _, _) => self.set(x, kk),
                (0x7, _, _, _) => self.add(x, kk),
                (0x8, _, _, 0x4) => self.add_xy(x, y),
                (0xA, _, _, _) => self.set_index(nnn),
                _ => todo!("opcode {:04x}", opcode),
            }
        }
    }

    fn set_index(&mut self, nnn: u16) {
        self.index_register = nnn;
    }

    fn add(&mut self, x: u8, kk: u8) {
        let val = self.registers[x as usize];
        
        match val.checked_add(kk) {
            Some(val) => self.registers[x as usize] = val,
            None => self.registers[x as usize] = 255 as u8,
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
