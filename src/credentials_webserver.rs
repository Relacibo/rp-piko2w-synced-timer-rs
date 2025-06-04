use cyw43::Control;
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_rp::flash::Flash;
use embassy_rp::peripherals::FLASH;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;

use crate::flash::{MyFlash, save_credentials_to_flash};

const PAYLOAD: &str = concat!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
    include_str!("../resources/wifi_setup.html")
);
const SUCCESS: &str = concat!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
    include_str!("../resources/wifi_setup_success.html")
);
const FLASH_OFFSET: u32 = 2 * 1024 * 1024 - 4096; // Letzte 4KB vom 2MB-Flash
const FLASH_MAGIC: &[u8] = b"WIFICRED";

use core::str::FromStr;

pub async fn run_setup_ap_and_webserver<'a>(
    control: &mut Control<'_>,
    stack: Stack<'static>,
    flash: &mut MyFlash<'a>,
) -> (heapless::String<64>, heapless::String<64>) {
    control.start_ap_open("Pico2W-Setup", 1).await;

    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 1024];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);

        socket.accept(80).await.unwrap();

        // Lese Request in temporären Buffer
        let mut temp_buf = [0u8; 512];
        let n = match socket.read(&mut temp_buf).await {
            Ok(n) => n,
            Err(_) => continue,
        };

        let request = core::str::from_utf8(&temp_buf[..n]).unwrap_or("");

        // Prüfe, ob es ein POST-Request ist und extrahiere die Daten
        if request.starts_with("POST") {
            if let Some(body_start) = request.find("\r\n\r\n") {
                let body = &request[body_start + 4..];
                // Erwartetes Format: ssid=...&pw=...
                if let (Some(ssid), Some(pw)) =
                    (find_form_value(body, "ssid"), find_form_value(body, "pw"))
                {
                    defmt::info!("SSID: {}, Passwort: {}", ssid, pw);
                    save_credentials_to_flash(flash, ssid, pw).await;
                    let _ = socket.write_all(SUCCESS.as_bytes()).await;
                    let _ = socket.close();
                    Timer::after(Duration::from_secs(2)).await;

                    // Hier die Rückgabe als heapless::String<64>
                    let ssid_str = heapless::String::<64>::from_str(ssid).unwrap_or_default();
                    let pw_str = heapless::String::<64>::from_str(pw).unwrap_or_default();
                    return (ssid_str, pw_str);
                }
            }
        } else {
            // Sende das Formular
            let _ = socket.write_all(PAYLOAD.as_bytes()).await;
            let _ = socket.close();
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}

/// Extrahiert einen Wert aus einem x-www-form-urlencoded Body
fn find_form_value<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    body.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        match (parts.next(), parts.next()) {
            (Some(k), Some(v)) if k == key => Some(v),
            _ => None,
        }
    })
}
