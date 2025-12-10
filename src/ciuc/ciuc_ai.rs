pub enum AIState
{
    SafeState, //Safe state, the planet generates less resources
    StatisticState //Statistic state, the planet is less conservative: it generates resources depending on 'estimate_asteroid_ms' and 'estimate_sunray_ms'
}


pub struct CiucAI {
    state: AIState,
    number_explorers: usize,
    count_asteroids: u32,
    count_sunrays: u32,
    last_time_sunray: i64,
    last_time_asteroid: i64,
    estimate_sunray_ms: f64,
    estimate_asteroid_ms: f64,
}

impl CiucAI
{
    pub(crate) fn new() -> Self
    {
        CiucAI {
            state: AIState::SafeState,
            number_explorers: 0,
            count_asteroids: 0,
            count_sunrays: 0,
            last_time_sunray: 0,
            last_time_asteroid: 0,
            estimate_asteroid_ms: 0.0,
            estimate_sunray_ms: 0.0,
        }
    }

    // ---------------- Getters ----------------
    pub(crate) fn state(&self) -> &AIState {
        &self.state
    }

    pub(crate) fn number_explorers(&self) -> usize {
        self.number_explorers
    }

    pub(crate) fn count_asteroids(&self) -> u32 {
        self.count_asteroids
    }

    pub(crate) fn count_sunrays(&self) -> u32 {
        self.count_sunrays
    }

    pub(crate) fn last_time_sunray(&self) -> i64 {
        self.last_time_sunray
    }

    pub(crate) fn last_time_asteroid(&self) -> i64 {
        self.last_time_asteroid
    }

    pub(crate) fn estimate_sunray_ms(&self) -> f64 {
        self.estimate_sunray_ms
    }

    pub(crate) fn estimate_asteroid_ms(&self) -> f64 {
        self.estimate_asteroid_ms
    }

    // ---------------- Setters ----------------
    pub(crate) fn set_state(&mut self, state: AIState) {
        self.state = state;
    }

    pub(crate) fn set_number_explorers(&mut self, n: usize) {
        self.number_explorers = n;
    }

    pub(crate) fn increment_count_asteroids(&mut self) {
        self.count_asteroids += 1;
    }

    pub(crate) fn increment_count_sunrays(&mut self) {
        self.count_sunrays += 1;
    }

    pub(crate) fn set_last_time_sunray(&mut self, t: i64) {
        self.last_time_sunray = t;
    }

    pub(crate) fn set_last_time_asteroid(&mut self, t: i64) {
        self.last_time_asteroid = t;
    }

    pub(crate) fn set_estimate_sunray_ms(&mut self, e: f64) {
        self.estimate_sunray_ms = e;
    }

    pub(crate) fn set_estimate_asteroid_ms(&mut self, e: f64) {
        self.estimate_asteroid_ms = e;
    }
}