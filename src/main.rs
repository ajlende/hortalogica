#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    embassy::{self},
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
};
use esp_println::println;

#[embassy_executor::task]
async fn run() {
    loop {
        Timer::after(Duration::from_millis(3_000)).await;
        println!("Fizz");
    }
}

#[main]
async fn main(spawner: Spawner) {
    println!("Init");
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    spawner.spawn(run()).ok();

    loop {
        Timer::after(Duration::from_millis(5_000)).await;
        println!("Buzz");
    }
}