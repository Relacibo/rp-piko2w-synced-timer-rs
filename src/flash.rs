use core::str::FromStr;
use embassy_rp::flash::{Async, Flash};
use embedded_storage_async::nor_flash::NorFlash;

pub type MyFlash<'a> = Flash<'a, embassy_rp::peripherals::FLASH, Async, { 2 * 1024 * 1024 }>;

// Lies die ersten 8 Bytes an FLASH_OFFSET
pub const FLASH_OFFSET: u32 = 2 * 1024 * 1024 - 4096; // wie in credentials_webserver.rs
pub const FLASH_MAGIC: &[u8] = b"WIFICRED";

pub async fn save_credentials_to_flash(flash: &mut MyFlash<'_>, ssid: &str, pw: &str) {
    let mut buf = [0u8; 256];
    buf[..8].copy_from_slice(FLASH_MAGIC);
    let ssid_bytes = ssid.as_bytes();
    let pw_bytes = pw.as_bytes();
    let ssid_len = ssid_bytes.len().min(63);
    let pw_len = pw_bytes.len().min(63);
    buf[8] = ssid_len as u8;
    buf[9..9 + ssid_len].copy_from_slice(&ssid_bytes[..ssid_len]);
    buf[72] = pw_len as u8;
    buf[73..73 + pw_len].copy_from_slice(&pw_bytes[..pw_len]);
    // Flash-Sektor löschen und schreiben
    flash.erase(FLASH_OFFSET, 4096).await.unwrap();
    flash.write(FLASH_OFFSET, &buf).await.unwrap();
}

pub async fn load_credentials_from_flash(
    flash: &mut MyFlash<'_>,
) -> Option<(heapless::String<64>, heapless::String<64>)> {
    let mut buf = [0u8; 256];
    flash.read(FLASH_OFFSET, &mut buf).await.ok()?;
    if &buf[..8] != FLASH_MAGIC {
        return None;
    }
    let ssid_len = buf[8] as usize;
    let pw_len = buf[72] as usize;
    let ssid_str = core::str::from_utf8(&buf[9..9 + ssid_len]).ok()?;
    let pw_str = core::str::from_utf8(&buf[73..73 + pw_len]).ok()?;
    let ssid = heapless::String::<64>::from_str(ssid_str).ok()?;
    let pw = heapless::String::<64>::from_str(pw_str).ok()?;
    Some((ssid, pw))
}

pub async fn reset_credentials_in_flash(flash: &mut MyFlash<'_>) {
    // Lösche den gesamten Sektor, in dem die Credentials liegen
    flash.erase(FLASH_OFFSET, 4096).await.ok();
}
