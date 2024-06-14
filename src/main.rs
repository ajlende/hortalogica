#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::rng::Rng;
use esp_hal::system::SystemControl;
use esp_hal::{
    analog::adc::{AdcConfig, Attenuation, Adc},
    clock::ClockControl,
    gpio::Io,
    peripherals::{Peripherals, ADC1},
    prelude::*,
    timer::timg::TimerGroup,
};
use esp_hal_embassy::*;
use esp_println::println;
use esp_wifi::wifi::{ClientConfiguration, Configuration, WifiStaDevice};
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiState};
use esp_wifi::{initialize, EspWifiInitFor};
use static_cell::make_static;

const SSID: &str = option_env!("SSID").unwrap();
const PASSWORD: &str = option_env!("PASSWORD").unwrap();
const HOST_IP: &str = option_env!("HOST_IP").unwrap();

#[main]
async fn main(spawner: Spawner) {
    println!("Init");

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // let timerInterrupt: Option<TimerInterrupts> = TimerInterrupts.``
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);
    esp_hal_embassy::init(&clocks, timg0);

    let timer = TimerGroup::new(peripherals.TIMG1, &clocks, None).timer0;

    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
        &clocks,
    )
    .unwrap();

    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let stack = &*mk_static!(
        Stack<WifiDevice<'_, WifiStaDevice>>,
        Stack::new(
            wifi_interface,
            config,
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            seed
        )
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(&stack)).ok();

    let socket = TcpSocket::new(&stack, &mut rx_buffer, &mut tx_buffer);

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Set up the moisture sensor power pin
    let mut power_pin = io.pins.gpio4;

    // Create ADC instances for the moisture sensor
    let mut adc1_config = AdcConfig::new();
    let mut moisture_pin =
        adc1_config.enable_pin(io.pins.gpio5, Attenuation::Attenuation11dB);
    let mut adc1 = Adc::<ADC1>::new(peripherals.ADC1, adc1_config);

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

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
