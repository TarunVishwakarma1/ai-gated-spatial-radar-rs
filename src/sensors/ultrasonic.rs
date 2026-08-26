use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig};

#[derive(Debug)]
pub enum UltrasonicError {
    Timeout,
    OutOfRange,
}

#[derive(Debug, Clone, Copy)]
pub struct RadarMeasurement {
    pub distance_cm: u16,
    pub timestamp: Instant,
    pub valid: bool,
    pub quality: u32,
}

pub struct Ultrasonic<'a> {
    trig: Output<'a>,
    echo: Input<'a>,
}

impl<'a> Ultrasonic<'a> {
    pub fn new(trig_pin: esp_hal::gpio::AnyPin<'a>, echo_pin: esp_hal::gpio::AnyPin<'a>) -> Self {
        Ultrasonic {
            trig: Output::new(trig_pin, Level::Low, OutputConfig::default()),
            echo: Input::new(echo_pin, InputConfig::default()),
        }
    }

    pub async fn measure(&mut self) -> Result<RadarMeasurement, UltrasonicError> {
        let measurement_timeout = Duration::from_millis(50); // Generous timeout for a ping

        let result = with_timeout(measurement_timeout, async {
            // Pulse Trig HIGH for ~10µs
            self.trig.set_high();
            Timer::after_micros(10).await;
            self.trig.set_low();

            // Wait for Echo to go HIGH
            self.echo.wait_for_rising_edge().await;
            let start = Instant::now();

            // Wait for Echo to go LOW
            self.echo.wait_for_falling_edge().await;
            let end = Instant::now();

            (start, end)
        })
        .await;

        match result {
            Ok((start, end)) => {
                let duration = end.duration_since(start);
                let distance_cm = (duration.as_micros() / 58) as u16;

                // HC-SR04 reliable range is ~2cm to ~400-450cm
                if distance_cm > 450 || distance_cm < 2 {
                    return Err(UltrasonicError::OutOfRange);
                }

                Ok(RadarMeasurement {
                    distance_cm,
                    timestamp: end,
                    valid: true,
                    quality: 100, // Placeholder
                })
            }
            Err(_) => Err(UltrasonicError::Timeout),
        }
    }
}
