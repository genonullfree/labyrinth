#![deny(unsafe_code)]
#![no_main]
#![no_std]

use core::fmt::Error;
use core::prelude::v1::Ok;
use cortex_m_rt::entry;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use microbit::hal::prelude::*;

#[cfg(feature = "v1")]
use microbit::{hal::twi, pac::twi0::frequency::FREQUENCY_A};

#[cfg(feature = "v2")]
use microbit::{hal::twim, pac::twim0::frequency::FREQUENCY_A};

use lsm303agr::{AccelMode, AccelOutputDataRate, Lsm303agr, Measurement};

const ACCELEROMETER_ADDR: u8 = 0b0011001;
const MAGNETOMETER_ADDR: u8 = 0b0011110;

const ACCELEROMETER_ID_REG: u8 = 0x0f;
const MAGNETOMETER_ID_REG: u8 = 0x4f;

const AVG_COUNT: i32 = 15;

#[derive(Debug, Default, Copy, Clone, PartialEq)]
struct Accel {
    x: i32,
    y: i32,
    z: i32,
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let board = microbit::Board::take().unwrap();

    #[cfg(feature = "v1")]
    let mut i2c = { twi::Twi::new(board.TWI0, board.i2c.into(), FREQUENCY_A::K100) };
    #[cfg(feature = "v2")]
    let mut i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K400) };

    let mut acc = [0];
    let mut mag = [0];

    // First write the address + register onto the bus, then read the chip's responses
    i2c.write_read(ACCELEROMETER_ADDR, &[ACCELEROMETER_ID_REG], &mut acc)
        .unwrap();
    i2c.write_read(MAGNETOMETER_ADDR, &[MAGNETOMETER_ID_REG], &mut mag)
        .unwrap();
    rprintln!("The accelerometer chip's id is: {:#b}", acc[0]);
    rprintln!("The magnetometer chip's id is: {:#b}", mag[0]);

    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor.set_accel_odr(AccelOutputDataRate::Hz50).unwrap();
    sensor.set_accel_mode(AccelMode::LowPower).unwrap();

    let mut avg: Accel = Accel::default();
    let mut count = AVG_COUNT;

    loop {
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
        rprintln!("Accel: x: {:5} y: {:5} z: {:5}", avg.x, avg.y, avg.z);
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
}
