use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub schema_version: u32,
    pub first_day: i64,
    pub days: u64,
    pub total_millis: u64,
    pub scale: u16,
    pub opacity: u8,
    pub always_on_top: bool,
    pub position: Option<(i32, i32)>,
}

impl State {
    pub fn new(today: i64) -> Self {
        Self {
            schema_version: 1,
            first_day: today,
            days: 0,
            total_millis: 0,
            scale: 100,
            opacity: 100,
            always_on_top: true,
            position: None,
        }
    }
    pub fn validate(&self) -> bool {
        self.schema_version == 1
            && (25..=400).contains(&self.scale)
            && (30..=100).contains(&self.opacity)
    }
    pub fn update_day(&mut self, today: i64) {
        self.days = self
            .days
            .max(today.saturating_sub(self.first_day).max(0) as u64);
    }
}

#[derive(Default)]
pub struct Timer {
    total: u64,
    started: Option<u64>,
}
impl Timer {
    pub fn restored(total: u64) -> Self {
        Self {
            total,
            started: None,
        }
    }
    pub fn running(&self) -> bool {
        self.started.is_some()
    }
    pub fn elapsed(&self, now: u64) -> u64 {
        self.started.map_or(0, |start| now.saturating_sub(start))
    }
    pub fn total(&self, now: u64) -> u64 {
        self.total.saturating_add(self.elapsed(now))
    }
    pub fn toggle(&mut self, now: u64) {
        if self.running() {
            self.stop(now);
        } else {
            self.started = Some(now);
        }
    }
    pub fn stop(&mut self, now: u64) {
        self.total = self.total(now);
        self.started = None;
    }
}

pub fn wheel_scale(scale: u16, remainder: &mut i32, delta: i16) -> u16 {
    *remainder += i32::from(delta);
    let steps = *remainder / 120;
    *remainder %= 120;
    (i32::from(scale) + steps * 5).clamp(25, 400) as u16
}

pub fn hours(total_millis: u64) -> String {
    format!("{:.1}", total_millis as f64 / 3_600_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sessions_accumulate_and_restart_is_paused() {
        let mut timer = Timer::restored(1000);
        timer.toggle(100);
        assert_eq!(timer.total(1100), 2000);
        timer.toggle(2100);
        assert_eq!(timer.total(9999), 3000);
        timer.toggle(10000);
        timer.stop(11000);
        assert_eq!(timer.total(99000), 4000);
        assert!(!Timer::restored(timer.total(99000)).running());
    }
    #[test]
    fn calendar_and_precision_and_wheel_contract() {
        let mut state = State::new(100);
        state.update_day(104);
        state.update_day(102);
        assert_eq!(state.days, 4);
        assert_eq!(hours(0), "0.0");
        assert_eq!(hours(3_600_000), "1.0");
        let mut rem = 0;
        assert_eq!(wheel_scale(100, &mut rem, 60), 100);
        assert_eq!(wheel_scale(100, &mut rem, 60), 105);
        assert_eq!(wheel_scale(25, &mut rem, -120), 25);
        assert_eq!(wheel_scale(400, &mut rem, 120), 400);
    }
}
