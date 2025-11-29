use std::time::{Duration, Instant};

pub struct DeltaTimer {
    last_frame_time: Instant,
    delta_time: f32,
    frame_count: u64,
    fps_timer: Instant,
    fps: f32,
}

impl DeltaTimer {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_frame_time: now,
            delta_time: 0.0,
            frame_count: 0,
            fps_timer: now,
            fps: 0.0,
        }
    }

    pub fn update(&mut self) -> f32 {
        let current_time = Instant::now();
        let duration = current_time.duration_since(self.last_frame_time);

        self.delta_time = duration.as_secs_f32();
        self.delta_time = self.delta_time.min(1.0 / 30.0);

        self.last_frame_time = current_time;
        self.frame_count += 1;

        if current_time.duration_since(self.fps_timer) >= Duration::from_secs(1) {
            self.fps = self.frame_count as f32;
            self.frame_count = 0;
            self.fps_timer = current_time;
        }

        self.delta_time
    }

    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }
}
