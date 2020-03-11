//Chip 8 Emulator by Sean-Thomas Leak
//Based on source code from Laurence Muller (laurence.muller@gmail.com)
use std::{thread, time};
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use rand::prelude::*;


struct Chip8
{
	gfx: [u8; 64*32],
	key: [u8; 16],
	
	pc: u16,
	opcode: u16,
	i: u16,
	sp: u16,
	
	v: [u8;16],
	stack: [u16; 16],
	memory: [u8; 4096],
	
	delay_timer: u8,
	sound_timer: u8,
	draw_flag: bool
}

impl Chip8
{
	fn new() -> Chip8
	{
		let font_set: [u8;80] =
		[
			0xF0, 0x90, 0x90, 0x90, 0xF0, //0
			0x20, 0x60, 0x20, 0x20, 0x70, //1
			0xF0, 0x10, 0xF0, 0x80, 0xF0, //2
			0xF0, 0x10, 0xF0, 0x10, 0xF0, //3
			0x90, 0x90, 0xF0, 0x10, 0x10, //4
			0xF0, 0x80, 0xF0, 0x10, 0xF0, //5
			0xF0, 0x80, 0xF0, 0x90, 0xF0, //6
			0xF0, 0x10, 0x20, 0x40, 0x40, //7
			0xF0, 0x90, 0xF0, 0x90, 0xF0, //8
			0xF0, 0x90, 0xF0, 0x10, 0xF0, //9
			0xF0, 0x90, 0xF0, 0x90, 0x90, //A
			0xE0, 0x90, 0xE0, 0x90, 0xE0, //B
			0xF0, 0x80, 0x80, 0x80, 0xF0, //C
			0xE0, 0x90, 0x90, 0x90, 0xE0, //D
			0xF0, 0x80, 0xF0, 0x80, 0xF0, //E
			0xF0, 0x80, 0xF0, 0x80, 0x80  //F
		];
		
		//First 512 bytes would contain the interpreter for Chip8, so the program counter starts at 0x200 (512)
		let mut c = Chip8 { gfx: [0; 64*32], key: [0;16], pc: 0x200, opcode: 0, i: 0, sp: 0, v: [0;16], stack: [0;16], memory: [0;4096], delay_timer: 0, sound_timer: 0, draw_flag: true};
		
		//Load font set into memory
		for i in 0..80
		{
			c.memory[i as usize] = font_set[i as usize];
		}
		
		c.load_application();
		return c
	}
	
	fn emulate_cycle(&mut self)
	{
		//Setting first 2 bytes to test execution
		//self.memory[self.pc as usize] = 0x00;
		//self.memory[self.pc as usize + 1] = 0xE0;
		
		//Building 2 byte (16 bit) value from memory
		self.opcode = (self.memory[self.pc as usize] as u16) << 8 | (self.memory[self.pc as usize + 1] as u16);
		//println!("{:#018x}",self.opcode);
		
		match self.opcode & 0xF000 //Match according to first 4 bits
		{
			//TODO match opcodes and perform instructions
			0x0000 =>
			{
				match self.opcode & 0x000F
				{
					0x0000 =>
					{
						self.gfx = [0;64*32];
						self.draw_flag = true;
						if DEBUG{println!("{0:x} | Cleared screen",self.opcode);}
						self.pc += 2;
					},
					0x000E =>
					{
						self.sp -= 1;
						self.pc = self.stack[self.sp as usize];
						if DEBUG{println!("{0:x}   | Returned to {1} from subroutine",self.opcode,self.stack[self.sp as usize]);}
						self.pc += 2;
					}
					,
					_ => println!("Unknown instruction: {:0x}",self.opcode)
				}
			},
			0x1000 => //0x1NNN Jumps to address NNN
			{
				let a = (self.opcode & 0x0FFF);
				self.pc = a;
				if DEBUG{println!("{0:x} | Jumped to address {1}",self.opcode, a);}
			}
			,
			0x2000 => //0x2NNN = Calls subroutine at NNN.
			{
				self.stack[self.sp as usize] = self.pc;
				self.sp += 1;
				let nl:u16 = (self.opcode & 0x0FFF);
				if DEBUG{println!("{0:x} | Called a subroutine at {1}",self.opcode,nl)};
				self.pc = nl;
			},
			0x3000 => //0x3XNN: Skips the next instruction if V[X] equals NN
			{
				if(self.v[((self.opcode & 0x0F00) >> 8) as usize] == (self.opcode & 0xFF) as u8) //probably broken
				{
					if DEBUG{println!("{0:x} | Skipped an Instruction because v[{1}] ({3}) == {2}",self.opcode,((self.opcode & 0x0F00) >> 8),(self.opcode & 0x00FF) as u8,self.v[((self.opcode & 0x0F00) >> 8) as usize])};
					self.pc += 4;
				}
				else
				{
					if DEBUG{println!("{0:x} | Didn't skip an Instruction because v[{1}] {3} != {2}",self.opcode,((self.opcode & 0x0F00) >> 8),(self.opcode & 0x00FF) as u8,self.v[((self.opcode & 0x0F00) >> 8) as usize])};
					self.pc += 2;
				}
			},
			0x4000 => //0x3XNN: Skips the next instruction if V[X] does not equal NN
			{
				if(self.v[(self.opcode & 0x0F00 >> 8) as usize] != (self.opcode & 0x00FF) as u8)
				{
					if DEBUG{println!("{0:x} | Skipped an Instruction",self.opcode)};
					self.pc += 4;
				}
				else
				{
					if DEBUG{println!("{0:x} | Didn't skip an Instruction",self.opcode)};
					self.pc += 2;
				}
			},
			0x5000 =>
			{
				if(self.v[((self.opcode & 0xF00) >> 8) as usize] == self.v[((self.opcode & 0x00F0) >> 4) as usize])
				{
					if DEBUG{println!("{0:x} | Skipped an Instruction",self.opcode)};
					self.pc += 4;
				}
				else
				{
					if DEBUG{println!("{0:x} | Didn't skip an Instruction",self.opcode)};
					self.pc += 2;
				}
			},
			0x6000 => //0x6XNN = sets V[X] equal to NN
			{
				let i: usize = ((self.opcode & 0x0F00) >> 8) as usize;
				let v: u8 = (self.opcode & 0x00FF) as u8;
				self.v[i] = v;
				if DEBUG{println!("{0:x} | Set V[{1}] equal to {2}", self.opcode,i, v)};
				self.pc += 2;
			},
			0x7000 => // 0x7XNN = Adds NN to V[X].
			{
				let i: usize = ((self.opcode & 0x0F00) >> 8) as usize;
				let v: u8 = (self.opcode & 0xFF) as u8;

				if(((self.v[i] as u16) + (v as u16)) <= 255)
				{
					self.v[i] += v;
				}
				else
				{
					self.v[i] = 0;
				}
				
				if DEBUG{println!("{0:x} | Added {2} to V[{1}] (V[{1}] is now equal to {3})", self.opcode,i, v,self.v[i])};
				self.pc += 2;
			},
			0x8000 =>
			{
				match (self.opcode & 0x000F)
				{
					0x0000 => //0x8XY0 = Sets V[X] to V[Y]
					{
						self.v[((self.opcode & 0x0F00) >> 8) as usize] = self.v[((self.opcode & 0x00F0) >> 4) as usize];
						if DEBUG{println!("{0:x} | Set V[X] equal to V[Y]", self.opcode)};
						self.pc += 2;
					},
					0x0001 => //0x8XY1 = Sets V[X] to (V[X] OR V[Y])
					{
						self.v[((self.opcode & 0x0F00) >> 8) as usize] |= self.v[((self.opcode & 0x00F0) >> 4) as usize];
						if DEBUG{println!("{0:x} | Set V[X] to (V[X] OR V[Y])", self.opcode)};
						self.pc += 2;
					},
					0x0002 => //0x8XY2 = Sets VX to to (V[X] AND V[Y])
					{
						self.v[((self.opcode & 0x0F00) >> 8) as usize] &= self.v[((self.opcode & 0x00F0) >> 4) as usize];
						if DEBUG{println!("{0:x} | Set V[X] to (V[X] AND V[Y])", self.opcode)};
						self.pc += 2;
					},
					0x0003 => //0x8XY3 = Sets VX to (V[X] XOR V[Y])
					{
						self.v[((self.opcode & 0x0F00) >> 8) as usize] ^= self.v[((self.opcode & 0x00F0) >> 4) as usize];
						if DEBUG{println!("{0:x} | Set V[X] to (V[X] XOR V[Y])", self.opcode)};
						self.pc += 2;
					},
					0x0004 => //0x8XY4 = Adds V[Y] to V[X]. V[15] is set to 1 when there's a carry, and to 0 when not
					{
						if(self.v[((self.opcode & 0x00F0) >> 4) as usize] > (0xFF - self.v[((self.opcode & 0x0F00) >> 8) as usize]))
						{
							self.v[0xF] = 1; //carry
						}
						else
						{
							self.v[0xF] = 0;
						}
						self.v[((self.opcode & 0x0F00) >> 8) as usize] += self.v[((self.opcode & 0x00F0) >> 4) as usize];
						if DEBUG{println!("{0:x} | Added V[Y] to V[X]", self.opcode)};
						self.pc += 2;
					},
					0x0005 => //0x8XY5 = V[Y] is subtracted from V[X]. V[15] is set to 0 if there is a borrow, and to 1 if there isn't
					{
						if(self.v[((self.opcode & 0x00F0) >> 4) as usize] > self.v[((self.opcode & 0x0F00) >> 8) as usize])
						{
							self.v[0xF] = 0; //borrow
						}
						else
						{
							self.v[0xF] = 1;
						}
						self.v[((self.opcode & 0x0F00) >> 8) as usize] -= self.v[((self.opcode & 0x00F0) >> 4) as usize];
						if DEBUG{println!("{0:x} | Subtracted V[Y] to V[X]", self.opcode)};
						self.pc += 2;
					},
					_ => 
					{
						panic!("Unknown instruction: {:0x}",self.opcode);
					}
				}
			},
			//ANNN = Sets I to the address NNN. 
			0xA000 =>
			{
				self.i = (self.opcode & 0x0FFF);
				if DEBUG{println!("{0:x} | Set i register to {1}",self.opcode,self.i);}
				self.pc += 2;
			},
			0xC000 => //CXNN = Sets V[x] to a random number AND NN
			{
				let i: u8 = ((self.opcode & 0x0F00) >> 8)as u8;
				let n: u8 = (self.opcode & 0x00FF) as u8;
				let mut rng = rand::thread_rng(); 
				let r: u8 = rng.gen();
				self.v[i as usize] = (n & r);
				if DEBUG{println!("{0:x} Set V[{1}] to ({2} & {3})",self.opcode,i,n,r);}
				self.pc += 2;
			},
			0xD000 =>
			{
				let x: u16 = self.v[((self.opcode & 0x0F00) >> 8) as usize] as u16;
				let y: u16 = self.v[((self.opcode & 0x00F0)>> 4) as usize] as u16;
				let height: u16 = self.opcode & 0x000F as u16;
				let mut pixel: u16 = 0;

				self.v[0xF] = 0;
				for yline in 0..height
				{
					pixel = self.memory[(self.i + yline) as usize] as u16;
					for xline in 0..8
					{
						if((pixel & (0x80 >> xline)) != 0)
						{
							if(self.gfx[(x + xline + ((y + yline) * 64)) as usize] == 1)
							{
								self.v[0xF] = 1;
							}
							self.gfx[(x + xline + ((y + yline) * 64)) as usize] ^= 1;
						}
					}
				}
				if DEBUG{println!("{0:x} | Drew a sprite",self.opcode)};
				self.draw_flag = true;
				self.pc += 2;
			},
			0xE000 =>
			{
				match (self.opcode & 0x00FF)
				{
					0x009E =>
					{
						if(self.key[self.v[((self.opcode & 0x0F00) >> 8) as usize] as usize] != 0)
						{
							self.pc += 4;
						}
						else
						{
							self.pc += 2;
						}
					},
					0x00A1 =>
					{
						if(self.key[self.v[((self.opcode & 0x0F00) >> 8) as usize] as usize] == 0)
						{
							self.pc += 4;
						}
						else
						{
							self.pc += 2;
						}
					},
					_ => 
					{
						panic!("Unknown instruction: {:0x}",self.opcode);
					}
				}
			},
			0xF000 =>
			{
				match (self.opcode & 0x00FF)
				{
					0x0007 =>
					{
						let i: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
						self.v[i as usize] = self.delay_timer;
						if DEBUG{println!("{0:x} | Set v[{1}] to {2}",self.opcode,i,self.delay_timer)};
						self.pc += 2;
					},
					0x001E => //Probably broken
					{
						if(((self.i as u16) + (self.v[((self.opcode & 0xF00) >> 8) as usize] as u16) as u16) > 0xFFF)
						{
							self.v[0xF] = 1;
							if DEBUG{println!("{0:x} | Set V[15] to 1",self.opcode)};
						}
						else
						{
							self.v[0xF] = 0;
							if DEBUG{println!("{0:x} | Set V[15] to 0",self.opcode)};
						}
						self.i += self.v[(((self.opcode & 0x0F00) >> 8) as usize)] as u16;
						self.pc += 2;
					},
					0x0015 =>
					{
						let i: u8 = ((self.opcode & 0x0F00) >> 8) as u8;
						let v: u8 = self.v[i as usize];
						self.delay_timer = v;
						if DEBUG{println!("{0:x} | Set delay timer to V[{1}] ({2})",self.opcode,i,v)};
						self.pc += 2;
					},
					_ => 
					{
						panic!("Unknown instruction: {:0x}",self.opcode);
					}
				}
			},			
			_ => 
			{
				panic!("Unknown instruction: {:0x}",self.opcode);
			}
		}
		
	}
	
	fn load_application(&mut self)
	{
		//Load ROM
		let mut rom = File::open("TETRIS.c8").expect("Error opening file. Make sure it exists.");
		let mut buffer = Vec::new();
		rom.read_to_end(&mut buffer).expect("Error reading file.");
		if DEBUG{println!("Read {} bytes from rom.",buffer.len());}
		let mut bi: usize = 0;
		for b in &buffer
		{
			self.memory[512 + bi] = b.clone();
			//println!("{:x}",self.memory[512 + bi]);
			bi += 1;
		}
	}
	
	fn debug_render(&mut self)
	{
		if(self.draw_flag)
		{
			for y in 0..32
			{
				for x in 0..64
				{
					if(self.gfx[(y*64) + x] == 0) 
					{
						print!("0");
					}
					else 
					{
						print!("█");
					}				
				}
				print!("\n");
			}
			print!("\n");
			self.draw_flag = false;
		}
	}
}

const DEBUG: bool = true;

fn main()
{
	//Initialize Chip8 struct
    let mut chip8 = Chip8::new();
	let mut t = 0;
    
    while true
    {
		//t += 1;
		chip8.emulate_cycle();
		//print!("{}[2J", 27 as char);
		//chip8.debug_render();
		thread::sleep_ms(1000/30);
	}

}

