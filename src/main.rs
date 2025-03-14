#![feature(once_cell_try)]

use self::wifi::Connector;
use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        // delay::Delay,
        onewire::{OWAddress, OWCommand},
        prelude::Peripherals,
        reset::restart,
    },
    io::vfs::MountedEventfs,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::{EspError, link_patches},
    timer::EspTaskTimerService,
};
use log::{error, info};
use std::sync::OnceLock;
use tcp::server;
use thermometer::{
    Ds18b20Driver,
    scratchpad::{ConfigurationRegister, Resolution, Scratchpad},
};
use tokio::{
    runtime::Builder,
    select, spawn,
    time::{Duration, sleep},
    try_join,
};
use wifi::connect;

static ADDRESSES: OnceLock<Vec<OWAddress>> = OnceLock::new();

fn main() -> Result<()> {
    link_patches();
    EspLogger::initialize_default();
    let _mounted_eventfs = MountedEventfs::mount(5)?;
    info!("System initialized");
    if let Err(error) = Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
    {
        error!("{error:?}");
    } else {
        info!("`main()` finished, restarting");
    }
    restart();
}

async fn run() -> Result<()> {
    // spawn(async move {
    //     loop {
    //         log::info!("tokio 2");
    //         sleep(Duration::from_millis(1000)).await;
    //     }
    // });
    let event_loop = EspSystemEventLoop::take()?;
    let timer = EspTaskTimerService::new()?;
    let peripherals = Peripherals::take()?;
    let nvs_default_partition = EspDefaultNvsPartition::take()?;
    // Initialize the network stack, this must be done before starting the server
    // let mut wifi_connection = Connector::new(
    //     peripherals.modem,
    //     event_loop,
    //     timer,
    //     Some(nvs_default_partition),
    // )
    // .await?;
    // wifi_connection.connect().await?;
    let _wifi = connect(
        peripherals.modem,
        event_loop,
        timer,
        Some(nvs_default_partition),
    )
    .await?;
    // tcp_server().await?;
    // run_server(wifi_connection.state.clone()),
    // try_join!(wifi_connection.connect(), modbus::server())?;
    select! {
        _ = tcp::server() => println!("Exiting"),
        _ = modbus::server() => println!("Exiting"),
    }
    Ok(())
}

async fn temp() -> Result<()> {
    loop {
        log::info!("tokio 1");
        sleep(Duration::from_millis(1000)).await;
    }
}

async fn temperature() -> Result<()> {
    info!("Starting async_main.");

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
        sleep(Duration::from_millis(1000)).await;
        // Delay::new_default();
    }

    // let config = Config::load()?;
    // info!("Configuration:\n{config:#?}");
    // let event_loop = EspSystemEventLoop::take()?;
    // let timer = EspTaskTimerService::new()?;
    // let peripherals = Peripherals::take()?;
    // let nvs_default_partition = nvs::EspDefaultNvsPartition::take()?;
    // // Initialize the network stack, this must be done before starting the server
    // let mut wifi_connection = WifiConnection::new(
    //     peripherals.modem,
    //     event_loop,
    //     timer,
    //     Some(nvs_default_partition),
    //     &config,
    // )
    // .await?;
    // // Run the server and the wifi keepalive concurrently until one of them fails
    // tokio::try_join!(
    //     run_server(wifi_connection.state.clone()),
    //     wifi_connection.connect()
    // )?;
    Ok(())
}

mod modbus;
mod tcp;
mod wifi;

// // addresses
// // 0x230000046eafbc28
// // 0: 0x4500000088204e28
// // 1: 0x970000006a14fe28
// fn main() -> Result<()> {
//     link_patches();
//     // Bind the log crate to the ESP Logging facilities
//     EspLogger::initialize_default();
//     info!("Initialize");
//     let peripherals = Peripherals::take()?;
//     // let mut led = Led::new(peripherals.pins.gpio8, peripherals.rmt.channel0)?;
//     let mut thermometer = Ds18b20Driver::new(peripherals.pins.gpio2, peripherals.rmt.channel0)?;
//     info!("Thermometer initialized");
//     let addresses = ADDRESSES.get_or_try_init(|| thermometer.search()?.collect())?;
//     for address in addresses {
//         let scratchpad = thermometer
//             .initialization()?
//             .match_rom(&address)?
//             .read_scratchpad()?;
//         info!("{address:x?}: {scratchpad:?}");
//     }
//     for address in addresses {
//         thermometer
//             .initialization()?
//             .match_rom(&address)?
//             .write_scratchpad(&Scratchpad {
//                 alarm_high_trigger_register: 30,
//                 alarm_low_trigger_register: 10,
//                 configuration_register: ConfigurationRegister {
//                     resolution: Resolution::Twelve,
//                 },
//                 ..Default::default()
//             })?;
//     }
//     for address in addresses {
//         let scratchpad = thermometer
//             .initialization()?
//             .match_rom(&address)?
//             .read_scratchpad()?;
//         info!("{address:x?}: {scratchpad:?}");
//     }
//     loop {
//         for address in addresses {
//             let temperature = thermometer.temperature(&address)?;
//             info!("{address:x?}: {temperature}");
//         }
//         Delay::new_default();
//     }
// }
