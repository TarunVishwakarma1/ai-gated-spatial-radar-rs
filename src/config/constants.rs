use esp_hal::gpio::AnyPin;

use crate::config::common::generate_peripherals_and_config;

pub struct SensorPins<'a> {
    pub ultrasonic: UltrasonicPins<'a>,
}

pub struct UltrasonicPins<'a> {
    pub trig_pin: AnyPin<'a>,
    pub echo_pin: AnyPin<'a>,
}

impl<'a> Default for UltrasonicPins<'a> {
    fn default() -> Self {
        let (peripherals, _) = generate_peripherals_and_config();
        UltrasonicPins {
            trig_pin: peripherals.GPIO15.into(),
            echo_pin: peripherals.GPIO16.into(),
        }
    }
}
