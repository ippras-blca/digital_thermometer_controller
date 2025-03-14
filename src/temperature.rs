use anyhow::Result;
use esp_idf_svc::hal::{gpio::IOPin, onewire::OWAddress, peripheral::Peripheral, rmt::RmtChannel};
use log::info;
use std::sync::OnceLock;
use thermometer::{
    Ds18b20Driver,
    scratchpad::{ConfigurationRegister, Resolution, Scratchpad},
};
use tokio::time::{Duration, sleep};

static ADDRESSES: OnceLock<Vec<OWAddress>> = OnceLock::new();

pub(super) async fn run(
    pin: impl Peripheral<P = impl IOPin>,
    channel: impl Peripheral<P = impl RmtChannel>,
) -> Result<()> {
    info!("Start temperature");
    let mut thermometer = Ds18b20Driver::new(pin, channel)?;
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
}
