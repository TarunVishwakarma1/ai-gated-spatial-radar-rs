use esp_hal::{clock::CpuClock, peripherals};

pub fn generate_peripherals_and_config() -> (peripherals::Peripherals, esp_hal::Config) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    (peripherals, config)
}
