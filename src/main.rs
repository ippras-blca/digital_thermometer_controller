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
        task::block_on,
    },
    io::vfs::MountedEventfs,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::{EspError, link_patches},
    timer::EspTaskTimerService,
    wifi::WifiEvent,
};
use log::{error, info, warn};
use std::sync::OnceLock;
use tcp::server;
use thermometer::{
    Ds18b20Driver,
    scratchpad::{ConfigurationRegister, Resolution, Scratchpad},
};
use tokio::{
    join,
    runtime::Builder,
    select, spawn,
    time::{Duration, sleep},
    try_join,
};
use wifi::connect;

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
    // spawn(async move {
    //     loop {
    //         info!("Spawn temperature");
    //         if let Err(error) =
    //             temperature::run(&mut peripherals.pins.gpio2, &mut peripherals.rmt.channel0).await
    //         {
    //             error!("{error:?}");
    //         }
    //         sleep(Duration::from_millis(1000)).await;
    //     }
    // });
    let event_loop = EspSystemEventLoop::take()?;
    let timer = EspTaskTimerService::new()?;
    let peripherals = Peripherals::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let temperature_sender = temperature::run(peripherals.pins.gpio2, peripherals.rmt.channel0)?;
    // Initialize the network stack, this must be done before starting the server
    let mut wifi = connect(peripherals.modem, event_loop.clone(), timer, Some(nvs)).await?;
    let _subscription = event_loop.subscribe::<WifiEvent, _>(move |event| {
        info!("Got event: {event:?}");
        if let WifiEvent::StaDisconnected(_) = event {
            if let Err(error) = wifi.connect() {
                warn!("Wifi connect failed: {error}");
            }
        }
    })?;

    // tcp_server().await?;
    // run_server(wifi_connection.state.clone()),
    // try_join!(wifi_connection.connect(), modbus::server())?;
    select! {
        // _ = tcp::server() => println!("Exiting"),
        _ = modbus::server(temperature_sender) => println!("Exiting"),
    }
    Ok(())
}

async fn temp() -> Result<()> {
    loop {
        log::info!("tokio 1");
        sleep(Duration::from_millis(1000)).await;
    }
}

mod modbus;
mod tcp;
mod temperature;
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
