#![feature(once_cell_try)]

use esp_idf_svc::{
    hal::{
        delay::Delay,
        onewire::{OWAddress, OWCommand},
        prelude::Peripherals,
    },
    log::EspLogger,
    sys::{EspError, link_patches},
};
use log::{error, info};
use std::{
    cell::LazyCell,
    sync::{LazyLock, OnceLock},
    thread::sleep,
    time::Duration,
};
use thermometer::{
    Ds18b20Driver, Error, Result,
    scratchpad::{ConfigurationRegister, Resolution, Scratchpad},
};

static ADDRESSES: OnceLock<Vec<OWAddress>> = OnceLock::new();

// addresses
// 0x230000046eafbc28
// 0: 0x4500000088204e28
// 1: 0x970000006a14fe28
fn main() -> Result<()> {
    link_patches();
    // Bind the log crate to the ESP Logging facilities
    EspLogger::initialize_default();
    info!("Initialize");

    let peripherals = Peripherals::take()?;

    // let mut led = Led::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
    let mut thermometer = Ds18b20Driver::new(peripherals.pins.gpio2, peripherals.rmt.channel0)?;
    info!("Thermometer initialized");
    let addresses = ADDRESSES.get_or_try_init(|| thermometer.search()?.collect())?;
    for address in addresses {
        let scratchpad = thermometer
            .initialization()?
            .match_rom(&address)?
            .read_scratchpad()?;
        info!("{address:x?}: {scratchpad:?}");
    }
    for address in addresses {
        thermometer
            .initialization()?
            .match_rom(&address)?
            .write_scratchpad(&Scratchpad {
                alarm_high_trigger_register: 30,
                alarm_low_trigger_register: 10,
                configuration_register: ConfigurationRegister {
                    resolution: Resolution::Twelve,
                },
                ..Default::default()
            })?;
    }
    for address in addresses {
        let scratchpad = thermometer
            .initialization()?
            .match_rom(&address)?
            .read_scratchpad()?;
        info!("{address:x?}: {scratchpad:?}");
    }
    loop {
        for address in addresses {
            let temperature = thermometer.temperature(&address)?;
            info!("{address:x?}: {temperature}");
        }
        Delay::new_default();
    }
}
