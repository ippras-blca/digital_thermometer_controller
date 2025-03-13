// use crate::config::Config;
// use embedded_svc::wifi::{ClientConfiguration, Configuration};
// use esp_idf_hal::modem::Modem;
use anyhow::{Result, anyhow};
use esp_idf_svc::{
    eventloop::{EspEventLoop, System},
    hal::modem::Modem,
    ipv4::{self, DHCPClientSettings},
    netif::{self, EspNetif, NetifConfiguration, NetifStack},
    nvs::EspDefaultNvsPartition,
    timer::{EspTimerService, Task},
    wifi::{AsyncWifi, AuthMethod, ClientConfiguration, Configuration, EspWifi, WifiDriver},
};
use log::{info, warn};
use std::{net::Ipv4Addr, str::FromStr, sync::Arc};
use tokio::{
    sync::RwLock,
    time::{Duration, sleep},
};

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

// pub(super) async fn wifi_connection() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     let mut wifi_connection = WifiConnection::new(
//         peripherals.modem,
//         event_loop,
//         timer,
//         Some(nvs_default_partition),
//     )
//     .await?;
//     Ok(())
// }

// Shared state of the Wi-Fi connection.
pub struct WifiState {
    pub mac_address: String,
    pub ssid: String,
    ip_address: RwLock<Option<Ipv4Addr>>,
}

impl WifiState {
    pub async fn ip_address(&self) -> Option<Ipv4Addr> {
        *self.ip_address.read().await
    }
}

// Wi-Fi connector.
pub struct Connector<'a> {
    pub state: Arc<WifiState>,
    wifi: AsyncWifi<EspWifi<'a>>,
}

impl<'a> Connector<'a> {
    // Initialize the Wi-Fi driver but do not connect yet.
    pub async fn new(
        modem: Modem,
        event_loop: EspEventLoop<System>,
        timer: EspTimerService<Task>,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Self> {
        info!("Initializing...");

        let wifi_driver = WifiDriver::new(modem, event_loop.clone(), nvs)?;
        let net_if = EspNetif::new_with_conf(&NetifConfiguration {
            ip_configuration: Some(ipv4::Configuration::Client(
                ipv4::ClientConfiguration::DHCP(DHCPClientSettings::default()),
            )),
            ..NetifConfiguration::wifi_default_client()
        })?;

        // Store the MAC address in the shared wifi state
        let mac = net_if.get_mac()?;
        let mac_address = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );
        let state = Arc::new(WifiState {
            ip_address: RwLock::new(None),
            mac_address,
            ssid: SSID.to_owned(),
        });

        // Wrap the Wi-Fi driver in the async wrapper
        let esp_wifi = EspWifi::wrap_all(wifi_driver, net_if, EspNetif::new(NetifStack::Ap)?)?;
        let mut wifi = AsyncWifi::wrap(esp_wifi, event_loop, timer.clone())?;

        // Set the Wi-Fi configuration
        info!("Setting credentials...");
        let wifi_configuration = Configuration::Client(ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            bssid: None,
            auth_method: AuthMethod::WPA2Personal,
            password: PASSWORD.try_into().unwrap(),
            channel: None,
            ..Default::default()
        });
        wifi.set_configuration(&wifi_configuration)?;

        info!("Starting...");
        wifi.start().await?;

        info!("Wi-Fi driver started successfully.");
        Ok(Self { state, wifi })
    }

    // Connect to Wi-Fi and stay connected. This function will loop forever.
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        loop {
            info!("Connecting to SSID '{}'...", self.state.ssid);
            if let Err(err) = self.wifi.connect().await {
                warn!("Connection failed: {err:?}");
                self.wifi.disconnect().await?;
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            info!("Acquiring IP address...");
            let timeout = Some(Duration::from_secs(10));
            if let Err(err) = self
                .wifi
                .ip_wait_while(|wifi| wifi.is_up().map(|status| !status), timeout)
                .await
            {
                warn!("IP association failed: {err:?}");
                self.wifi.disconnect().await?;
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            let ip_info = self.wifi.wifi().sta_netif().get_ip_info();
            *self.state.ip_address.write().await = ip_info.ok().map(|info| info.ip);
            info!("Connected to '{}': {ip_info:#?}", self.state.ssid);

            // Wait for Wi-Fi to be down
            self.wifi.wifi_wait(|wifi| wifi.is_up(), None).await?;
            warn!("Wi-Fi disconnected.");
        }
    }
}
