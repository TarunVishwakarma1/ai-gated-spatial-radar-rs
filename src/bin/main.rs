#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use ai_spatial_radar_intrustion_engine::config::{common, constants};
use ai_spatial_radar_intrustion_engine::sensors::ultrasonic::Ultrasonic;
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::gpio::Level;
use esp_hal::rmt::{PulseCode, Rmt, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const T0H: u16 = 32;
const T0L: u16 = 68;
const T1H: u16 = 68;
const T1L: u16 = 32;

fn encode_bytes(dst: &mut [PulseCode], byte: u8) {
    for (i, slot) in dst.iter_mut().enumerate() {
        let bit_is_one = (byte >> (7 - i)) & 1 == 1;
        *slot = if bit_is_one {
            PulseCode::new(Level::High, T1H, Level::Low, T1L)
        } else {
            PulseCode::new(Level::High, T0H, Level::Low, T0L)
        };
    }
}

fn encode_rgb(r: u8, g: u8, b: u8) -> [PulseCode; 25] {
    let mut buf = [PulseCode::end_marker(); 25];
    encode_bytes(&mut buf[0..8], g);
    encode_bytes(&mut buf[8..16], r);
    encode_bytes(&mut buf[16..24], b);
    buf[24] = PulseCode::end_marker();
    buf
}

fn wheel(pos: u8) -> (u8, u8, u8) {
    let pos = 255 - pos;
    if pos < 85 {
        (255 - pos * 3, 0, pos * 3)
    } else if pos < 170 {
        let pos = pos - 85;
        (0, pos * 3, 255 - pos * 3)
    } else {
        let pos = pos - 170;
        (pos * 3, 255 - pos * 3, 0)
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let (peripherals, _config) = common::generate_peripherals_and_config();

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("failed to initlize RMT");
    let mut channel = rmt
        .channel0
        .configure_tx(&TxChannelConfig::default().with_clk_divider(1))
        .expect("failed to create channel")
        .with_pin(peripherals.GPIO48);

    let sensors_pins = constants::SensorPins {
        ultrasonic: constants::UltrasonicPins::default(),
    };

    let mut ultrasonic = Ultrasonic::new(
        sensors_pins.ultrasonic.trig_pin,
        sensors_pins.ultrasonic.echo_pin,
    );

    let _ = spawner;
    let mut n = 0;
    let mut hue: u8 = 0;

    // Phase 1 starts here.
    loop {
        let (r, g, b) = wheel(hue);
        let data = encode_rgb(r / 20, g / 20, b / 20);
        let transmit = channel.transmit(&data).expect("RMT transmit failed");
        channel = transmit.wait().expect("wait failed");

        match ultrasonic.measure().await {
            Ok(measurement) => log::info!("tick: {n}, distance: {}cm", measurement.distance_cm),
            Err(e) => log::warn!("tick: {n}, error: {:?}", e),
        }

        hue = hue.wrapping_add(1);
        n += 1;
        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    }
}
