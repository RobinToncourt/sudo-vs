use std::time::{
    Duration,
    Instant, //
};

const TIME_BUDGET: Duration = Duration::from_secs(10);
pub const TIMEOUT_GRACE: Duration = Duration::from_secs(3);

pub struct Clock {
    pub mine: Duration,
    pub theirs: Duration,
    pub turn_started: Instant,
}

impl Clock {
    pub fn new(now: Instant) -> Self {
        Self {
            mine: TIME_BUDGET,
            theirs: TIME_BUDGET,
            turn_started: now,
        }
    }
    pub fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.turn_started)
    }
    pub fn mine(&self, my_turn: bool, now: Instant) -> Duration {
        if my_turn {
            self.mine.saturating_sub(self.elapsed(now))
        } else {
            self.mine
        }
    }
    pub fn theirs(&self, my_turn: bool, now: Instant) -> Duration {
        if my_turn {
            self.theirs
        } else {
            self.theirs.saturating_sub(self.elapsed(now))
        }
    }

    pub fn i_moved(&mut self, now: Instant) -> Duration {
        self.mine = self.mine(true, now);
        self.turn_started = now;
        self.mine
    }
    pub fn opponent_moved(&mut self, reported: Duration, now: Instant) {
        self.theirs = reported.min(self.theirs);
        self.turn_started = now;
    }
}
