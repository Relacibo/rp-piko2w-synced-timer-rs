#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

pub mod alarm;
pub mod credentials_flash;
pub mod credentials_webserver;
pub mod network;
pub mod utils;

use crate::alarm::Alarm;
use crate::credentials_flash::CredentialsFlash;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use static_cell::StaticCell;

use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::unwrap;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::init;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Duration, Timer};

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(debug_assertions)]
    defmt::println!("{}", info); // e.g. using RTT
    loop {}
}

// Firmware und CLM-Daten
const FW: &[u8] = include_bytes!("../cyw43-firmware/43439A0.bin");
const CLM: &[u8] = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_runner_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) {
    runner.run().await;
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) {
    runner.run().await;
}

static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = init(Default::default());

    // CYW43-Power und SPI via PIO initialisieren
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24, // MOSI
        p.PIN_29, // MISO
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());

    // CYW43 initialisieren
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, FW).await;
    unwrap!(spawner.spawn(cyw43_runner_task(runner)));

    control.init(CLM).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // embassy-net Stack initialisieren
    let config = Config::dhcpv4(Default::default());

    // MAC-Adresse als Seed verwenden (liefert ein [u8; 6])
    let mac = control.address().await;
    let seed = u64::from_le_bytes([mac[0], mac[1], mac[2], mac[3], mac[4], mac[5], 0, 0]);

    let (stack, net_runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<2>::new()),
        seed,
    );

    // Starte den Runner als Task!
    unwrap!(spawner.spawn(net_task(net_runner)));

    let mut flash = CredentialsFlash::new(p.FLASH, p.DMA_CH1);

    // Credentials aus Flash laden
    let creds = flash.load_credentials_from_flash().await;

    let (ssid, password) = if let Some(c) = creds {
        c
    } else {
        credentials_webserver::run_setup_ap_and_webserver(&mut control, stack, &mut flash).await
    };

    // Mit Heim-WLAN verbinden
    control
        .join(&ssid, cyw43::JoinOptions::new(password.as_bytes()))
        .await
        .unwrap();

    let mut alarm = Alarm::new();

    // Button-Initialisierung
    let button = Input::new(p.PIN_15, Pull::Up); // Beispiel-Pin

    // Prüfe beim Start, ob der Button 5 Sekunden gedrückt wird
    if button.is_low() {
        Timer::after(Duration::from_secs(5)).await;
        if button.is_low() {
            // Button war 5 Sekunden gedrückt: Credentials löschen!
            flash.reset_credentials_in_flash().await;
            defmt::info!("WLAN-Credentials wurden zurückgesetzt!");
            // Optional: Neustart oder Setup-Modus aktivieren
        }
    }

    #[cfg(feature = "server")]
    crate::network::run_tcp_server(stack, &mut alarm).await;

    #[cfg(feature = "client")]
    crate::network::run_tcp_client(stack, &mut alarm).await;
}
