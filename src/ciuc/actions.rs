use crate::CiucAI;
use crate::ciuc::AIState;
use common_game::components::planet::PlanetState;
use common_game::components::resource::Generator;
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;

impl CiucAI {
    ///Function for building rocket using one charged energy cell
    pub(crate) fn build_rocket(&self, planet_state: &mut PlanetState) -> Result<(), String> {
        match planet_state.full_cell() {
            None => Err("Didn't find any charged cell, impossible to build a rocket".to_string()),
            Some((_cell, i)) => planet_state.build_rocket(i),
        }
    }

    ///Function for trying to destroy an asteroid
    pub(crate) fn deflect_asteroid(&self, planet_state: &mut PlanetState) -> Option<Rocket> {
        planet_state.take_rocket()
    }

    ///Function for charging an energy cell with a sunray
    pub(crate) fn charge_cell_with_sunray(
        &self,
        planet_state: &mut PlanetState,
        sunray: Sunray,
    ) -> Result<(), String> {
        let result = planet_state.charge_cell(sunray);
        match result {
            None => Ok(()),
            Some(_) => {
                // All cells are full of charge, discard the sunray
                Err("All cells are full of charge".to_string())
            }
        }
    }

    pub(crate) fn generate_carbon(
        &self,
        planet_state: &mut PlanetState,
        generator: &Generator,
    ) -> Result<common_game::components::resource::Carbon, String> {
        match &self.state() {
            AIState::SafeState => self.generate_carbon_safe_state(planet_state, generator),
            AIState::StatisticState => {
                self.generate_carbon_statistic_state(planet_state, generator)
            }
        }
    }
}
