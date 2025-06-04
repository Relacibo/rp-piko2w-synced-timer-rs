use core::net::Ipv4Addr;
use embassy_net::{IpEndpoint, Stack};
use embassy_time::{Duration, Instant, Timer};
use heapless::Vec;

use crate::alarm::Alarm;

const MAX_CLIENTS: usize = 4;
const SERVER_PORT: u16 = 23456;
const RECONNECT_TIMEOUT_MS: u64 = 5000;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq)]
pub enum TimerCmd {
    Start = 1,
    Stop = 2,
    Pause = 3,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TimerMsg {
    pub cmd: TimerCmd,
    pub start_time: u32,
    pub duration: u32,
}

impl TimerMsg {
    pub fn to_bytes(&self) -> [u8; 9] {
        let mut buf = [0u8; 9];
        buf[0] = self.cmd as u8;
        buf[1..5].copy_from_slice(&self.start_time.to_le_bytes());
        buf[5..9].copy_from_slice(&self.duration.to_le_bytes());
        buf
    }
    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 9 {
            return None;
        }
        let cmd = match buf[0] {
            1 => TimerCmd::Start,
            2 => TimerCmd::Stop,
            3 => TimerCmd::Pause,
            _ => return None,
        };
        let mut start_time = [0u8; 4];
        let mut duration = [0u8; 4];
        start_time.copy_from_slice(&buf[1..5]);
        duration.copy_from_slice(&buf[5..9]);
        Some(Self {
            cmd,
            start_time: u32::from_le_bytes(start_time),
            duration: u32::from_le_bytes(duration),
        })
    }
}

#[cfg(feature = "client")]
pub async fn run_tcp_client(stack: Stack<'_>, alarm: &mut Alarm) {
    static mut RX_BUF: [u8; 1024] = [0; 1024];
    static mut TX_BUF: [u8; 1024] = [0; 1024];

    loop {
        let mut socket =
            unsafe { embassy_net::tcp::TcpSocket::new(stack, &mut RX_BUF, &mut TX_BUF) };

        let remote = IpEndpoint::new(Ipv4Addr::from(SERVER_ADDR).into(), SERVER_PORT);

        match socket.connect(remote).await {
            Ok(()) => {
                // Beispiel: TimerMsg senden (z.B. Start)
                let msg = TimerMsg {
                    cmd: TimerCmd::Start,
                    start_time: 123456, // aktuelle Zeit einsetzen!
                    duration: 30000,
                };
                let _ = socket.write(&msg.to_bytes()).await;

                // Nachrichten empfangen
                let mut buf = [0u8; 32];
                loop {
                    match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => {
                            if let Some(received) = TimerMsg::from_bytes(&buf[..n]) {
                                match received.cmd {
                                    TimerCmd::Start => {
                                        alarm.start(received.start_time, received.duration);
                                    }
                                    TimerCmd::Stop => {
                                        alarm.stop();
                                    }
                                    TimerCmd::Pause => {
                                        alarm.pause();
                                    }
                                }
                            }
                        }
                        Ok(_) => break,  // Verbindung geschlossen
                        Err(_) => break, // Fehler
                    }
                }
            }
            Err(_) => {
                // Verbindung fehlgeschlagen
            }
        }
        Timer::after(Duration::from_secs(2)).await;
    }
}

#[cfg(feature = "server")]
pub async fn run_tcp_server(stack: Stack<'_>, alarm: &mut Alarm) {
    static mut RX_BUF: [[u8; 1024]; MAX_CLIENTS] = [[0; 1024]; MAX_CLIENTS];
    static mut TX_BUF: [[u8; 1024]; MAX_CLIENTS] = [[0; 1024]; MAX_CLIENTS];

    let mut clients: Vec<embassy_net::tcp::TcpSocket<'_>, MAX_CLIENTS> = Vec::new();
    let mut last_active: [Option<Instant>; MAX_CLIENTS] = [None; MAX_CLIENTS];

    loop {
        // Neue Verbindungen akzeptieren, solange Platz ist
        for i in 0..MAX_CLIENTS {
            if clients.len() < MAX_CLIENTS {
                let mut socket = unsafe {
                    embassy_net::tcp::TcpSocket::new(stack, &mut RX_BUF[i], &mut TX_BUF[i])
                };
                if socket
                    .accept(IpEndpoint::new(Ipv4Addr::UNSPECIFIED.into(), SERVER_PORT))
                    .await
                    .is_ok()
                {
                    last_active[clients.len()] = Some(Instant::now());
                    clients.push(socket).ok();
                }
            }
        }

        // Nachrichten von allen Clients lesen und an alle weiterleiten
        let mut remove_indices = heapless::Vec::<usize, MAX_CLIENTS>::new();
        for i in 0..clients.len() {
            let mut buf = [0u8; 32];
            match clients[i].read(&mut buf).await {
                Ok(n) if n > 0 => {
                    last_active[i] = Some(Instant::now());
                    if let Some(received) = TimerMsg::from_bytes(&buf[..n]) {
                        match received.cmd {
                            TimerCmd::Start => {
                                alarm.start(received.start_time, received.duration);
                            }
                            TimerCmd::Stop => {
                                alarm.stop();
                            }
                            TimerCmd::Pause => {
                                alarm.pause();
                            }
                        }
                    }
                    // Nachricht an alle anderen Clients weiterleiten
                    for (j, client) in clients.iter_mut().enumerate() {
                        if j != i {
                            let _ = client.write(&buf[..n]).await;
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => {
                    remove_indices.push(i).ok();
                }
            }
        }

        // Inaktive/verlorene Clients entfernen (Timeout)
        let now = Instant::now();
        #[allow(clippy::needless_range_loop)]
        for i in 0..clients.len() {
            if let Some(last) = last_active[i] {
                if now.duration_since(last).as_millis() > RECONNECT_TIMEOUT_MS {
                    remove_indices.push(i).ok();
                }
            }
        }

        // Entferne Clients mit Fehler oder Timeout (von hinten nach vorne!)
        while let Some(idx) = remove_indices.pop() {
            clients.swap_remove(idx);
            last_active[idx] = None;
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}
