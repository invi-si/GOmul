pub struct GuestClock {
    host: f64,
    guest: f64,
    rate: u32,
}
impl GuestClock {
    pub fn new(host: f64) -> Self {
        Self {
            host,
            guest: host,
            rate: 1000,
        }
    }
    pub fn now(&self, host: f64) -> f64 {
        self.guest + (host - self.host) * self.rate as f64 / 1000.0
    }
    pub fn set_speed(&mut self, host: f64, rate: u32) {
        self.guest = self.now(host);
        self.host = host;
        self.rate = rate.clamp(250, 3000);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_speed_preserves_guest_time_and_scales_elapsed_time() {
        let mut c = GuestClock::new(1000.0);
        c.set_speed(1100.0, 2000);
        assert_eq!(c.now(1100.0), 1100.0);
        assert_eq!(c.now(1200.0), 1300.0);
        c.set_speed(1200.0, 250);
        assert_eq!(c.now(1600.0), 1400.0);
    }
}
