#![deny(unsafe_code)]
#![no_main]
#![no_std]

use cortex_m_rt::entry;
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

    let leds = [[0u8; 5]; 5];
    let mut dot = Point::default();

    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor.set_accel_odr(AccelOutputDataRate::Hz50).unwrap();
    sensor.set_accel_mode(AccelMode::LowPower).unwrap();

    let mut avg: Accel = Accel::default();
    let mut count = AVG_COUNT;

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
        dot.shift(avg.dir());
        let mut l = leds;
        l[dot.x as usize][dot.y as usize] ^= 1;

        display.show(&mut timer, l, 50);
        rprintln!("Accel: x: {:5} y: {:5} z: {:5}", avg.x, avg.y, avg.z);

        // Reset variables
        count = AVG_COUNT;
        avg = Accel::default();
    }
}

impl Accel {
    pub fn add(&mut self, m: Measurement) {
        self.x += m.x;
        self.y += m.y;
        self.z += m.z;
    }

    pub fn avg(&mut self, c: i32) {
        self.x = self.x / c;
        self.y = self.y / c;
        self.z = self.z / c;
    }

    pub fn dir(&self) -> Direction {
        let x = self.x.abs();
        let y = self.y.abs();

        if x < 100 && y < 100 {
            return Direction::Stop;
        }

        if x > y {
            if self.x > 0 {
                Direction::Down
            } else {
                Direction::Up
            }
        } else {
            if self.y > 0 {
                Direction::Left
            } else {
                Direction::Right
            }
        }
    }
}

impl Point {
    pub fn shift(&mut self, d: Direction) {
        match d {
            Direction::Right => {
                if self.x < 4 {
                    self.x += 1;
                }
            }
            Direction::Left => {
                if self.x > 0 {
                    self.x -= 1;
                }
            }
            Direction::Down => {
                if self.y < 4 {
                    self.y += 1;
                }
            }
            Direction::Up => {
                if self.y > 0 {
                    self.y -= 1;
                }
            }
            Direction::Stop => (),
        }
    }
}
