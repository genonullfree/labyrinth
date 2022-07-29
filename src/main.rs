#![deny(unsafe_code)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use heapless::Vec;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use microbit::hal::prelude::*;

#[cfg(feature = "v1")]
use microbit::{
    display::blocking::Display, hal::twi, hal::Timer, pac::twi0::frequency::FREQUENCY_A,
};

#[cfg(feature = "v2")]
use microbit::{
    display::blocking::Display, hal::twim, hal::Timer, pac::twim0::frequency::FREQUENCY_A,
};

use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr, Measurement};

const AVG_COUNT: i32 = 15;

#[derive(Debug, Default, Copy, Clone, PartialEq)]
struct Accel {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
struct Point {
    x: u8,
    y: u8,
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
struct Wall {
    a: Point,
    b: Point,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Labyrinth {
    walls: Vec<Wall, 40>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
struct World {
    leds: [[u8; 5]; 5],
}

#[derive(Debug, PartialEq)]
enum Direction {
    Stop,
    Up,
    Down,
    Left,
    Right,
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = microbit::Board::take().unwrap();
    let mut display = Display::new(board.display_pins);
    let mut timer = Timer::new(board.TIMER0);

    #[cfg(feature = "v1")]
    let i2c = { twi::Twi::new(board.TWI0, board.i2c.into(), FREQUENCY_A::K100) };
    #[cfg(feature = "v2")]
    let i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };

    let world = World::default();
    let mut dot = Point::default();

    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor.set_accel_odr(AccelOutputDataRate::Hz50).unwrap();
    sensor.set_accel_mode(AccelMode::LowPower).unwrap();

    let mut avg: Accel = Accel::default();
    let mut count = AVG_COUNT;

    // Setup labyrinth
    let mut l = Labyrinth {
        walls: Vec::<Wall, 40>::new(),
    };
    let difficulty = 10;
    let mut count = 0;
    loop {
        if sensor.accel_status().unwrap().xyz_new_data {
            let data = sensor.accel_data().unwrap();
            let wall = Wall::rand(data);
            if l.walls.contains(&wall) {
                continue;
            }
            rprintln!("Generated wall between: ({:?}) and ({:?})", wall.a, wall.b);
            l.walls.push(wall);
            count += 1;
            if count == difficulty {
                break;
            }
        }
    }

    loop {
        // Get average acceleration
        loop {
            if sensor.accel_status().unwrap().xyz_new_data {
                let data = sensor.accel_data().unwrap();
                avg.add(data);
                count -= 1;
                if count == 0 {
                    avg.avg(AVG_COUNT);
                    break;
                }
            }
        }

        // Update display
        let dir = avg.dir();
        dot.shift(&dir, &l);
        let mut l = world;
        l.leds[dot.x as usize][dot.y as usize] ^= 1;

        display.show(&mut timer, l.leds, 50);
        //rprintln!("Accel: x: {:5} y: {:5} z: {:5}", avg.x, avg.y, avg.z);
        //rprintln!("\n{:?}", l);
        rprintln!("({}, {}) Move {:?}", dot.x, dot.y, dir);

        // Reset variables
        count = AVG_COUNT;
        avg = Accel::default();
    }
}

impl Accel {
    pub fn add(&mut self, m: Measurement) {
        self.x += m.x;
        self.y += m.y;
    }

    pub fn avg(&mut self, c: i32) {
        self.x = self.x / c;
        self.y = self.y / c;
    }

    pub fn dir(&self) -> Direction {
        let x = self.x.abs();
        let y = self.y.abs();

        if x < 100 && y < 100 {
            return Direction::Stop;
        }

        if x > y {
            if self.x > 0 {
                Direction::Right
            } else {
                Direction::Left
            }
        } else {
            if self.y > 0 {
                Direction::Up
            } else {
                Direction::Down
            }
        }
    }
}

impl Point {
    pub fn shift(&mut self, d: &Direction, l: &Labyrinth) {
        let mut np = self.clone();
        match d {
            Direction::Right => np.move_right(),
            Direction::Left => np.move_left(),
            Direction::Down => np.move_down(),
            Direction::Up => np.move_up(),
            Direction::Stop => (),
        }
        if *self == np {
            return;
        }

        if self.is_ok(&np, l) {
            *self = np;
        }
    }

    fn move_down(&mut self) {
        if self.x < 4 {
            self.x += 1;
        }
    }

    fn move_up(&mut self) {
        if self.x > 0 {
            self.x -= 1;
        }
    }

    fn move_right(&mut self) {
        if self.y < 4 {
            self.y += 1;
        }
    }

    fn move_left(&mut self) {
        if self.y > 0 {
            self.y -= 1;
        }
    }

    pub fn is_ok(&self, np: &Point, l: &Labyrinth) -> bool {
        for w in &l.walls {
            if w.is_blocking(self, np) {
                return false;
            }
        }
        true
    }

    pub fn rand(m: &Measurement) -> Self {
        Self {
            x: (m.x as u8 % 5),
            y: (m.y as u8 % 5),
        }
    }
}

impl Wall {
    pub fn is_blocking(&self, p: &Point, np: &Point) -> bool {
        (p == &self.a || p == &self.b) && (np == &self.a || np == &self.b)
    }

    pub fn rand(m: Measurement) -> Self {
        let l = Labyrinth::default();
        let a = Point::rand(&m);
        let mut z = m.z;
        loop {
            let d = match z % 4 {
                0 => Direction::Up,
                1 => Direction::Down,
                2 => Direction::Right,
                3 => Direction::Left,
                _ => {
                    z += 1;
                    continue;
                }
            };
            let mut b = a.clone();
            b.shift(&d, &l);
            if b != a {
                return Self { a, b };
            }
            z += 1;
        }
    }
}
