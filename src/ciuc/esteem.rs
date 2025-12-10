use crate::CiucAI;
use crate::ciuc::AIState;
use common_game::logging::{ActorType, Channel, EventType};
pub fn update_ema(prev: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * prev
}
pub(crate) fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

impl CiucAI {
    /// Function for updating sunray esteem
    pub(crate) fn update_sunray_esteem(&mut self, now_ms: i64, id: u32) {
        let prev_esteem_for_log = self.estimate_sunray_ms();
        if self.last_time_sunray() > 0 {
            if self.count_sunrays() == 0 {
                self.set_estimate_sunray_ms((now_ms - self.last_time_sunray()) as f64);
                self.increment_count_sunrays()
            } else {
                let delta = (now_ms - self.last_time_sunray()) as f64;
                self.set_estimate_sunray_ms(update_ema(self.estimate_sunray_ms(), delta, 0.3));
                self.increment_count_sunrays()
            }
        }
        self.set_last_time_sunray(now_ms);
        self.log(
            format!(
                "Updated sunray esteem from {} to {}",
                prev_esteem_for_log,
                self.estimate_sunray_ms()
            ),
            id,
            ActorType::User,
            EventType::InternalPlanetAction,
            "user".to_string(),
            Channel::Debug,
        );
    }

    /// Function for updating asteroid esteem
    pub(crate) fn update_asteroid_esteem(&mut self, now_ms: i64, id: u32) {
        let prev_asteroid_esteem_for_log = self.estimate_asteroid_ms();
        if self.last_time_asteroid() > 0 {
            if self.count_asteroids() == 0 {
                self.set_estimate_asteroid_ms((now_ms - self.last_time_asteroid()) as f64);
                self.increment_count_asteroids()
            } else {
                let delta = (now_ms - self.last_time_asteroid()) as f64;
                self.set_estimate_asteroid_ms(update_ema(self.estimate_asteroid_ms(), delta, 0.3));
                self.increment_count_asteroids()
            }
        }
        self.set_last_time_asteroid(now_ms);
        self.log(
            format!(
                "Updated asteroid esteem from {} to {}",
                prev_asteroid_esteem_for_log,
                self.estimate_asteroid_ms()
            ),
            id,
            ActorType::User,
            EventType::InternalPlanetAction,
            "user".to_string(),
            Channel::Debug,
        );
    }

    ///Function for changing state
    pub(crate) fn change_state(&mut self, id: u32) {
        // Return to safe zone if in StatisticState and asteroid threat is greater than sunray opportunity
        if matches!(self.state(), AIState::StatisticState)
            && self.estimate_asteroid_ms() < self.estimate_sunray_ms()
        {
            self.set_state(AIState::SafeState);
            self.log(
                "Changed AI's state into safe".to_string(),
                id,
                ActorType::User,
                EventType::InternalPlanetAction,
                "user".to_string(),
                Channel::Debug,
            );
        }
        // Transition to StatisticState if enough data is collected and asteroid threat is less than sunray opportunity
        else if matches!(self.state(), AIState::SafeState)
            && self.count_asteroids() >= 3
            && self.count_sunrays() >= 3
            && self.estimate_asteroid_ms() >= self.estimate_sunray_ms()
        {
            self.set_state(AIState::StatisticState);
            self.log(
                "Changed AI's state into statistic".to_string(),
                id,
                ActorType::User,
                EventType::InternalPlanetAction,
                "user".to_string(),
                Channel::Debug,
            );
        }
    }
}
