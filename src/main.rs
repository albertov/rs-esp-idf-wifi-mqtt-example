use anyhow;
use embedded_svc::mqtt::client::QoS;
use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration},
    nvs::EspDefaultNvsPartition,
    wifi::EspWifi,
};
use esp_idf_sys as _;
use std::{thread::sleep, time::Duration};

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    println!("Entered Main function!");

    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi_driver = EspWifi::new(peripherals.modem, sys_loop, Some(nvs)).unwrap();

    wifi_driver
        .set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: heapless::String::try_from(std::env!("WIFI_ESSID")).unwrap(),
            password: heapless::String::try_from(std::env!("WIFI_PASSWORD")).unwrap(),
            ..Default::default()
        }))
        .unwrap();

    wifi_driver.start().unwrap();
    wifi_driver.connect().unwrap();
    while !wifi_driver.is_connected().unwrap() {
        let config = wifi_driver.get_configuration().unwrap();
        println!("Waiting for station {:?}", config);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    let mut mqtt_config = MqttClientConfiguration::default();
    mqtt_config.username = Some(std::env!("MQTT_USERNAME"));
    mqtt_config.password = Some(std::env!("MQTT_PASSWORD"));

    // Create Client Instance and Define Behaviour on Event
    let mut client = EspMqttClient::new_cb("mqtt://hal.lan", &mqtt_config, move |message_event| {
        match message_event.payload() {
            EventPayload::Connected(_) => println!("Connected"),
            EventPayload::Subscribed(id) => println!("Subscribed to {} id", id),
            EventPayload::Received { topic, data, .. } => {
                if data != [] {
                    println!(
                        "Recieved {} from {}",
                        String::from_utf8_lossy(data),
                        topic.unwrap_or("N/A")
                    )
                }
            }
            _ => println!("{:?}", message_event.payload()),
        };
    })?;

    // Subscribe to MQTT Topic
    client.subscribe("#", QoS::AtLeastOnce)?;
    println!("Should be connected now");
    loop {
        println!(
            "IP info: {:?}",
            wifi_driver.sta_netif().get_ip_info().unwrap()
        );
        sleep(Duration::new(10, 0));
    }
}
