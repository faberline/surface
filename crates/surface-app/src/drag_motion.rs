pub(crate) const SLOP: f32 = 6.;
const VELOCITY_IDLE_MS: f64 = 100.;

pub(crate) struct Velocity {
    position: f32,
    time_ms: Option<f64>,
    value: f32,
}

impl Velocity {
    pub(crate) fn new(position: f32, time_ms: Option<f64>) -> Self {
        Self {
            position,
            time_ms,
            value: 0.,
        }
    }

    pub(crate) fn sample(&mut self, position: f32, now_ms: Option<f64>) -> f32 {
        let elapsed = now_ms.zip(self.time_ms).map(|(now, last)| now - last);
        if elapsed.is_some_and(|elapsed| elapsed > VELOCITY_IDLE_MS) {
            self.value = 0.;
        }
        if position != self.position {
            if let Some(elapsed) =
                elapsed.filter(|elapsed| *elapsed >= 1. && *elapsed <= VELOCITY_IDLE_MS)
            {
                let velocity =
                    ((f64::from(position) - f64::from(self.position)) * 1_000. / elapsed) as f32;
                if velocity.is_finite() {
                    self.value = velocity;
                }
            }
            self.position = position;
            self.time_ms = now_ms;
        }
        self.value
    }
}
