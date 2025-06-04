pub struct Alarm {
    pub start_time: u32,
    pub duration: u32,
    pub status: AlarmStatus,
}

#[derive(Copy, Clone, PartialEq)]
pub enum AlarmStatus {
    Idle,
    Running,
    Paused,
}

impl Alarm {
    pub fn new() -> Self {
        Self {
            start_time: 0,
            duration: 0,
            status: AlarmStatus::Idle,
        }
    }

    pub fn start(&mut self, now: u32, duration: u32) {
        self.start_time = now;
        self.duration = duration;
        self.status = AlarmStatus::Running;
    }

    pub fn stop(&mut self) {
        self.status = AlarmStatus::Idle;
    }

    pub fn pause(&mut self) {
        if self.status == AlarmStatus::Running {
            self.status = AlarmStatus::Paused;
        }
    }

    pub fn is_expired(&self, now: u32) -> bool {
        self.status == AlarmStatus::Running && (now - self.start_time >= self.duration)
    }
}
