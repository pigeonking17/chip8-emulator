// std::fs used to read the program file.
use std::{fs, path::PathBuf};
// clap library used to parse command line arguments.
use clap::Parser;

mod cpu;

/// Allows for programs to be selected from the command line.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    program: PathBuf,
}

/// Parses the cli arguments, reads the program into bytes, assembles the memory with the font,
/// program, and correct spacing, initates the cpu loop.
fn main() {
    // Read the value of the program flag.
    let cli = Cli::parse();
    let program_buf = cli.program;

    // Check that the file provided is a CHIP-8 program.
    if program_buf.extension().unwrap() != "ch8" {
        panic!("Please provide a .ch8 file.");
    }

    // Reads the file into a vector of bytes.
    let program = fs::read(program_buf).unwrap();

    // Contains the font sprites that are used by some programs.
    let font: [u8; 80] = [
		0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
		0x20, 0x60, 0x20, 0x20, 0x70, // 1
		0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
		0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
		0x90, 0x90, 0xF0, 0x10, 0x10, // 4
		0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
		0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
		0xF0, 0x10, 0x20, 0x40, 0x40, // 7
		0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
		0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
		0xF0, 0x90, 0xF0, 0x90, 0x90, // A
		0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
		0xF0, 0x80, 0x80, 0x80, 0xF0, // C
		0xE0, 0x90, 0x90, 0x90, 0xE0, // D
		0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
		0xF0, 0x80, 0xF0, 0x80, 0x80  // F
	];

    // Initialises and empty memory that is 4kiB in length.
    let mut memory = [0 as u8; 4096];

    // Insert the font into memory.
    for (i, byte) in font.iter().enumerate() {
        memory[i] = *byte;
    }

    // Insert the program into memory at 0x200.
    for (i, byte) in program.iter().enumerate() {
        memory[i + 0x200] = *byte;
    }

    // Creates an empty cpu with the program and font loaded into memory.
    let mut cpu = cpu::CPU {
        registers: [0; 16],
        program_counter: 0x200,
        memory,
        stack: [0; 16],
        stack_pointer: 0,
        index_register: 0,
    };

    // Starts the cpu.
    cpu.run();
}
