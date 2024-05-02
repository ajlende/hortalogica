#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    analog::adc::{AdcConfig, Attenuation, ADC},
    clock::ClockControl,
    embassy,
    gpio::IO,
    peripherals::{Peripherals, ADC1},
    prelude::*,
    timer::{TimerGroup, TimerInterrupts},
};
use esp_println::println;

#[main]
async fn main(_spawner: Spawner) {
    println!("Init");
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // let timerInterrupt: Option<TimerInterrupts> = TimerInterrupts.``
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // Set up the moisture sensor power pin
    let mut power_pin = io.pins.gpio4.into_push_pull_output();

    // Create ADC instances for the moisture sensor
    let mut adc1_config = AdcConfig::new();
    let mut moisture_pin =
        adc1_config.enable_pin(io.pins.gpio5.into_analog(), Attenuation::Attenuation11dB);
    let mut adc1 = ADC::<ADC1>::new(peripherals.ADC1, adc1_config);

    // Main loop.
    loop {
        power_pin.set_high();
        Timer::after(Duration::from_millis(10_000)).await;
        let pin_value: u16 = nb::block!(adc1.read_oneshot(&mut moisture_pin)).unwrap();
        println!("Moisture = {}", pin_value);
        power_pin.set_low();
        Timer::after(Duration::from_millis(5_000)).await;
    }
}
