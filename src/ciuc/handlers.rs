use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::{BasicResource, BasicResourceType, Combinator, Generator};
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::logging::{ActorType, Channel, EventType};
use common_game::protocols::messages;
use common_game::protocols::messages::{PlanetToExplorer, PlanetToOrchestrator};
use crate::ciuc::esteem::now_ms;
use crate::CiucAI;

impl CiucAI {
    pub(crate) fn on_sunray(&mut self, planet_state: &mut PlanetState, sunray:Sunray) -> Result<(), String>
    {
        self.update_sunray_esteem(now_ms(), planet_state.id());
        let res = self.charge_cell_with_sunray(planet_state, sunray)?;
        let mess_build = self.build_rocket(planet_state);

        match mess_build {
            Ok(_) => {
                self.log("Rocket built".to_string(), planet_state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Info );
            }
            Err(_) => {
                // If the rocket is not built, it's not a real error, it just tried
                self.log("Didn't build any rocket".to_string(), planet_state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Info );
            }
        }

        self.change_state(planet_state.id());
        Ok(res)
    }

    pub(crate) fn on_asteroid(&mut self, planet_state: &mut PlanetState) -> Option<Rocket> // Returns rocket if deflected, None if destroyed
    {
        self.update_asteroid_esteem(now_ms(),planet_state.id());  // Update the estimate
        let rocket = self.deflect_asteroid(planet_state);
        if let Some(_) = rocket {  // As soon as the rocket is used (unless the planet is dead) try to recreate it immediately (if no energy cell, it will be created with the next sunray)
            let mess_build = self.build_rocket(planet_state);

            match mess_build {
                Ok(_) => {
                    self.log("Rocket built".to_string(), planet_state.id(), ActorType::User, EventType::InternalPlanetAction, "0".to_string(), Channel::Info );
                }
                Err(_) => {
                    // If the rocket is not built, it's not a real error, it just tried
                    self.log("Didn't build any rocket".to_string(), planet_state.id(), ActorType::User, EventType::InternalPlanetAction, "0".to_string(), Channel::Info );
                }
            }

            self.change_state(planet_state.id()); // Change the state if an estimate is usable and the planet is not dead
        }
        rocket
    }
}

impl PlanetAI for CiucAI
{
    //Handler for managing message exchange with the orchestrator
    fn handle_orchestrator_msg(&mut self, state: &mut PlanetState, _generator: &Generator, _combinator: &Combinator, msg: messages::OrchestratorToPlanet) -> Option<PlanetToOrchestrator> {

        match msg {
            messages::OrchestratorToPlanet::Sunray(sun) => {
                self.log("Sunray received".to_string(), state.id(), ActorType::User, EventType::MessageOrchestratorToPlanet, "user".to_string(), Channel::Info);
                let message = self.on_sunray(state, sun);
                match message {
                    Ok(_) => {
                        self.log("Cell charged".to_string(), state.id(), ActorType::Orchestrator, EventType::InternalPlanetAction, "orchestrator".to_string(), Channel::Info);
                    }
                    Err(e) => {
                        self.log(e, state.id(), ActorType::User, EventType::InternalPlanetAction, "orchestrator".to_string(), Channel::Error);
                    }
                }
                self.log("Sending SunrayAck to the orchestrator".to_string(), state.id(), ActorType::Orchestrator, EventType::MessagePlanetToOrchestrator, "orchestrator".to_string(), Channel::Trace);
                Some(PlanetToOrchestrator::SunrayAck { planet_id: state.id() })

            },
            messages::OrchestratorToPlanet::InternalStateRequest => {
                self.log("Internal state requested".to_string(), state.id(), ActorType::User, EventType::MessageOrchestratorToPlanet, "user".to_string(), Channel::Info);
                self.log("Sending internal state to the orchestrator".to_string(), state.id(), ActorType::Orchestrator, EventType::MessagePlanetToOrchestrator, "orchestrator".to_string(), Channel::Info);
                Some(PlanetToOrchestrator::InternalStateResponse {
                    planet_id: state.id(),
                    planet_state: state.to_dummy(),
                })
            },
            messages::OrchestratorToPlanet::KillPlanet =>{
                //manca il log
                self.log("I'm killed".to_string(), state.id(), ActorType::User, EventType::MessageOrchestratorToPlanet, "user".to_string(), Channel::Info);
                Some(PlanetToOrchestrator::KillPlanetResult { planet_id: state.id() })
            },
            _ => None
        }
    }

    fn handle_explorer_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: messages::ExplorerToPlanet) -> Option<PlanetToExplorer> {

        match msg {
            messages::ExplorerToPlanet::SupportedResourceRequest { explorer_id: e_id } => {
                self.log("Supported resource requested".to_string(), state.id(), ActorType::User, EventType::MessageExplorerToPlanet, "user".to_string(), Channel::Info);
                self.log("Sending supported resource".to_string(), state.id(), ActorType::Explorer, EventType::MessagePlanetToExplorer, e_id.to_string(), Channel::Info);
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes()
                })
            },

            messages::ExplorerToPlanet::SupportedCombinationRequest { explorer_id: e_id } => {
                self.log("Supported combinations requested".to_string(), state.id(), ActorType::User, EventType::MessageExplorerToPlanet, "user".to_string(), Channel::Info);
                self.log("Sending supported combinations".to_string(), state.id(), ActorType::Explorer, EventType::MessagePlanetToExplorer, e_id.to_string(), Channel::Info);
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes()
                })
            },

            messages::ExplorerToPlanet::GenerateResourceRequest { explorer_id: e_id, resource: res_type } => {
                match res_type {
                    BasicResourceType::Carbon => {
                        self.log("Generate carbon request".to_string(), state.id(), ActorType::User, EventType::MessageExplorerToPlanet, "user".to_string(), Channel::Info);
                        let res = self.generate_carbon(state, generator);
                        match res {
                            Ok(carbon) => {

                                self.log("Sending carbon to explorer".to_string(), state.id(), ActorType::Explorer, EventType::MessagePlanetToExplorer, e_id.to_string(), Channel::Info);

                                Some(PlanetToExplorer::GenerateResourceResponse {
                                    resource: Some(BasicResource::Carbon(carbon))
                                })
                            },
                            Err(err) =>
                                {
                                    self.log(err, state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Error);
                                    None
                                },
                        }
                    },
                    _ => {
                        self.log("This planet can't generate this type of resource".to_string(), state.id(), ActorType::User, EventType::MessageExplorerToPlanet, "user".to_string(), Channel::Error);
                        None
                    }
                }
            },

            messages::ExplorerToPlanet::CombineResourceRequest { explorer_id: _, msg: _ } => {
                self.log("Combination request".to_string(), state.id(), ActorType::User, EventType::MessageExplorerToPlanet, "user".to_string(), Channel::Info);
                self.log("There aren't any combination rules for this planet".to_string(), state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Error);
                None
            },

            messages::ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: e_id } => {
                self.log("Available energy cells requested".to_string(), state.id(), ActorType::User, EventType::MessageExplorerToPlanet, "user".to_string(), Channel::Info);
                self.log("Sending available energy cells".to_string(), state.id(), ActorType::Explorer, EventType::MessagePlanetToExplorer, e_id.to_string(), Channel::Info);
                Some(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: state.cells_iter().len() as u32 })
            },

            _ => None
        }

    }

    fn handle_asteroid(&mut self, state: &mut PlanetState, _generator: &Generator, _combinator: &Combinator, ) -> Option<Rocket> {
        self.log("Asteroid received".to_string(), state.id(), ActorType::User, EventType::MessageOrchestratorToPlanet, "user".to_string(), Channel::Info);
        self.on_asteroid(state)
    }

    fn start(&mut self, state: &PlanetState) {
        self.log("Starting planet's AI".to_string(), state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Info);
    }
    
    fn stop(&mut self, state: &PlanetState) {
        self.log("Stopping planet's AI".to_string(), state.id(), ActorType::User, EventType::InternalPlanetAction, "user".to_string(), Channel::Info);
    }

}