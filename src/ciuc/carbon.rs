use common_game::components::planet::PlanetState;
use common_game::components::resource::Generator;
use common_game::logging::{ActorType, Channel, EventType};
use crate::ciuc::esteem::now_ms;
use crate::CiucAI;

mod safe {
    /// Number of energy cells to preserve in safe state
    pub(crate) const SAFE_CELLS: u32 = 3;
}

mod statistic {
    /// Number of energy cells to preserve when asteroid is far (first threshold)
    pub(crate) const SAFE_CELLS_FAR_ASTEROID: u32 = 1;

    /// Number of energy cells to preserve when asteroid is near (second threshold)
    pub(crate) const SAFE_CELLS_NEAR_ASTEROID: u32 = 2;
    /// Fraction of the estimated sunray interval after which a sunray is considered imminent
    pub(crate) const SUNRAY_IMMINENT_THRESHOLD: f64 = 0.75;

    /// Fraction of the estimated asteroid interval under which the asteroid is considered far
    pub(crate) const ASTEROID_FAR_THRESHOLD: f64 = 0.5;
}


impl CiucAI {
    ///Function for generating carbon if there are more than 'safe_cells' cells charged
    pub(crate) fn generate_carbon_if_have_n_safe_cells(&self, planet_state:&mut PlanetState, generator: &Generator, safe_cells:u32) -> Result<common_game::components::resource::Carbon, String> {
        let energy_cell_charged_len = planet_state.cells_iter().filter(|c| c.is_charged()).count() as u32;
        match energy_cell_charged_len {
            0 =>  Err("Didn't find any charged cell".to_string()),
            6.. => Err("Invalid cell length".to_string()),
            charged_cells if charged_cells > safe_cells =>
                {
                    let first_energy_cell_charged = planet_state.full_cell();
                    match first_energy_cell_charged {
                        Some((cell,_)) => {
                            generator.make_carbon(cell)
                        },
                        None => {
                            Err("Should have found a charged cell, but didn't".to_string())
                        }
                    }
                },
            _ => Err("Shouldn't be in that case".to_string())
        }
    }

    ///Function for generating carbon in safe state
    pub(crate) fn generate_carbon_safe_state(&self, planet_state:&mut PlanetState, generator: &Generator)-> Result<common_game::components::resource::Carbon, String> {
        self.generate_carbon_if_have_n_safe_cells(planet_state, &generator, safe::SAFE_CELLS)
    }

    ///Function for generating carbon in statistic state
    pub(crate) fn generate_carbon_statistic_state(&self, planet_state:&mut PlanetState, generator: &Generator) -> Result<common_game::components::resource::Carbon, String> {
        let now = now_ms();
        let time_passed_last_sunray = now - self.last_time_sunray();
        let time_passed_last_asteroid = now - self.last_time_asteroid();

        let mut remove_safe_cell_cause_sunray = 0;

        // If a sunray is expected soon, we can generate faster (the safe cell will return immediately)
        if (time_passed_last_sunray as f64) > (statistic::SUNRAY_IMMINENT_THRESHOLD * self.estimate_sunray_ms())
        {
            self.log("I estimate that a sunray may arrive, so I reduce the cells to be preserved by one.".to_string(), planet_state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Debug );
            remove_safe_cell_cause_sunray = 1;
        }

        // If the asteroid is far away (less than half the estimated time has passed)
        if (time_passed_last_asteroid as f64) < (statistic::ASTEROID_FAR_THRESHOLD * self.estimate_asteroid_ms())
        {
            let safe_cell = statistic::SAFE_CELLS_FAR_ASTEROID - remove_safe_cell_cause_sunray; // If a sunray is expected, use one less cell as it will return immediately
            self.log( format!("the asteroid is far away so I reserve {} cells for my survival.", safe_cell), planet_state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Debug );
            // Generate quickly, keeping only one safe cell (or zero if sunray expected)
            self.generate_carbon_if_have_n_safe_cells(planet_state, &generator, safe_cell)
        }
        else
        {
            let safe_cell = statistic::SAFE_CELLS_NEAR_ASTEROID - remove_safe_cell_cause_sunray; // If a sunray is expected, generate with one less cell as it will return immediately
            // Generate less quickly, keeping two SAFE cells (or one if sunray expected)
            self.generate_carbon_if_have_n_safe_cells(planet_state, &generator, safe_cell)
        }
    }


}