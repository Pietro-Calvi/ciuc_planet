use common_game::logging::Participant;
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::protocols::planet_explorer::PlanetToExplorer;
use common_game::components::planet::DummyPlanetState;
use crate::CiucAI;
use crate::ciuc::esteem::now_ms;
use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::resource::{BasicResource, BasicResourceType, Combinator, ComplexResourceRequest, Generator, GenericResource};
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::logging::{ActorType, Channel, EventType};

impl CiucAI {
    pub(crate) fn on_sunray(
        &mut self,
        planet_state: &mut PlanetState,
        sunray: Sunray,
    ) -> Result<(), String> {
        self.update_sunray_esteem(now_ms(), planet_state.id());
        let res = self.charge_cell_with_sunray(planet_state, sunray)?;
        let mess_build = self.build_rocket(planet_state);

        match mess_build {
            Ok(_) => {
                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, planet_state.id())),
                    None,
                    EventType::InternalPlanetAction,
                    Channel::Info,
                    [("message", "Rocket built")],
                );
            }
            Err(_) => {
                // If the rocket is not built, it's not a real error, it just tried
                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, planet_state.id())),
                    None,
                    EventType::InternalPlanetAction,
                    Channel::Info,
                    [("message", "Didn't build any rocket")],
                );
            }
        }

        self.change_state(planet_state.id());
        Ok(res)
    }

    pub(crate) fn on_asteroid(&mut self, planet_state: &mut PlanetState) -> Option<Rocket> // Returns rocket if deflected, None if destroyed
    {
        self.update_asteroid_esteem(now_ms(), planet_state.id()); // Update the estimate
        let rocket = self.deflect_asteroid(planet_state);
        if let Some(_) = rocket {
            // As soon as the rocket is used (unless the planet is dead) try to recreate it immediately (if no energy cell, it will be created with the next sunray)
            let mess_build = self.build_rocket(planet_state);

            match mess_build {
                Ok(_) => {
                    CiucAI::log_event(
                        Some(Participant::new(ActorType::User, planet_state.id())),
                        None,
                        EventType::InternalPlanetAction,
                        Channel::Info,
                        [("message", "Rocket built")],
                    );
                }
                Err(_) => {
                    // If the rocket is not built, it's not a real error, it just tried
                    CiucAI::log_event(
                        Some(Participant::new(ActorType::User, planet_state.id())),
                        None,
                        EventType::InternalPlanetAction,
                        Channel::Info,
                        [("message", "Didn't build any rocket")],
                    );
                }
            }

            self.change_state(planet_state.id()); // Change the state if an estimate is usable and the planet is not dead
        }
        rocket
    }
}

impl PlanetAI for CiucAI {
    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        match msg {
            ExplorerToPlanet::SupportedResourceRequest { explorer_id: e_id } => {
                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, state.id())),
                    None,
                    EventType::MessageExplorerToPlanet,
                    Channel::Info,
                    [("message", "Supported resource requested")],
                );

                CiucAI::log_event(
                    Some(Participant::new(ActorType::Explorer, state.id())),
                    Some(Participant::new(ActorType::Explorer, e_id)),
                    EventType::MessagePlanetToExplorer,
                    Channel::Info,
                    [("message", "Sending supported resource")],
                );

                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes(),
                })
            }

            ExplorerToPlanet::SupportedCombinationRequest { explorer_id: e_id } => {
                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, state.id())),
                    None,
                    EventType::MessageExplorerToPlanet,
                    Channel::Info,
                    [("message", "Supported combinations requested")],
                );

                CiucAI::log_event(
                    Some(Participant::new(ActorType::Explorer, state.id())),
                    Some(Participant::new(ActorType::Explorer, e_id)),
                    EventType::MessagePlanetToExplorer,
                    Channel::Info,
                    [("message", "Sending supported combinations")],
                );
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes(),
                })
            }

            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id: e_id,
                resource: res_type,
            } => match res_type {
                BasicResourceType::Carbon => {
                    CiucAI::log_event(
                        Some(Participant::new(ActorType::User, state.id())),
                        None,
                        EventType::MessageExplorerToPlanet,
                        Channel::Info,
                        [("message", "Generate carbon request")],
                    );
                    let res = self.generate_carbon(state, generator);
                    match res {
                        Ok(carbon) => {
                            CiucAI::log_event(
                                Some(Participant::new(ActorType::Explorer, state.id())),
                                Some(Participant::new(ActorType::Explorer, e_id)),
                                EventType::MessagePlanetToExplorer,
                                Channel::Info,
                                [("message", "Sending carbon to explorer")],
                            );

                            Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(BasicResource::Carbon(carbon)),
                            })
                        }
                        Err(err) => {
                            CiucAI::log_event(
                                Some(Participant::new(ActorType::User, state.id())),
                                None,
                                EventType::InternalPlanetAction,
                                Channel::Error,
                                [("message", err)],
                            );
                            Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: None,
                            })
                        }
                    }
                }
                _ => {
                    CiucAI::log_event(
                        Some(Participant::new(ActorType::User, state.id())),
                        None,
                        EventType::MessageExplorerToPlanet,
                        Channel::Error,
                        [("message", "This planet can't generate this type of resource")],
                    );
                    None
                }
            },

            ExplorerToPlanet::CombineResourceRequest {
                explorer_id: _,
                msg: mes,
            } => {
                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, state.id())),
                    None,
                    EventType::MessageExplorerToPlanet,
                    Channel::Info,
                    [("message", "Combination request")],
                );

                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, state.id())),
                    None,
                    EventType::InternalPlanetAction,
                    Channel::Error,
                    [("message", "There aren't any combination rules for this planet")],
                );
                match mes {
                    ComplexResourceRequest::Water(a, b) => {Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("There aren't any combination rules for this planet".to_string(), a.to_generic() , b.to_generic())),
                    })}
                    ComplexResourceRequest::Diamond(a, b) => {Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("There aren't any combination rules for this planet".to_string(), a.to_generic() , b.to_generic())),
                    })}
                    ComplexResourceRequest::Life(a, b) => {Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("There aren't any combination rules for this planet".to_string(), a.to_generic() , b.to_generic())),
                    })}
                    ComplexResourceRequest::Robot(a, b) => {Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("There aren't any combination rules for this planet".to_string(), a.to_generic() , b.to_generic())),
                    })}
                    ComplexResourceRequest::Dolphin(a, b) => {Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("There aren't any combination rules for this planet".to_string(), a.to_generic() , b.to_generic())),
                    })}
                    ComplexResourceRequest::AIPartner(a, b) => {Some(PlanetToExplorer::CombineResourceResponse {
                        complex_response: Err(("There aren't any combination rules for this planet".to_string(), a.to_generic() , b.to_generic())),
                    })}
                }

            }

            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: e_id } => {
                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, state.id())),
                    None,
                    EventType::MessageExplorerToPlanet,
                    Channel::Info,
                    [("message", "Available energy cells requested")],
                );

                CiucAI::log_event(
                    Some(Participant::new(ActorType::Explorer, state.id())),
                    Some(Participant::new(ActorType::Explorer, e_id)),
                    EventType::MessagePlanetToExplorer,
                    Channel::Info,
                    [("message", "Sending available energy cells")],
                );
                Some(PlanetToExplorer::AvailableEnergyCellResponse {
                    available_cells: state.cells_iter().len() as u32,
                })
            }
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
    ) -> Option<Rocket> {
        CiucAI::log_event(
            Some(Participant::new(ActorType::User, state.id())),
            None,
            EventType::MessageOrchestratorToPlanet,
            Channel::Info,
            [("message", "Asteroid received")],
        );
        self.on_asteroid(state)
    }

    fn handle_sunray(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
        sunray: Sunray,
    ){
        CiucAI::log_event(
            Some(Participant::new(ActorType::User, state.id())),
            None,
            EventType::MessageOrchestratorToPlanet,
            Channel::Info,
            [("message", "Sunray received")],
        );
        let message = self.on_sunray(state, sunray);
        match message {
            Ok(_) => {
                CiucAI::log_event(
                    Some(Participant::new(ActorType::Orchestrator, state.id())),
                    None,
                    EventType::InternalPlanetAction,
                    Channel::Info,
                    [("message", "Cell charged")],
                );
            }
            Err(e) => {
                let channel = if e == "All cell are full of charge"{
                    Channel::Info
                }else{
                    Channel::Error
                };

                CiucAI::log_event(
                    Some(Participant::new(ActorType::User, state.id())),
                    None,
                    EventType::InternalPlanetAction,
                    channel,
                    [("message", e)],
                );
            }
        }
    }

    fn handle_internal_state_req(
        &mut self,
        state: &mut PlanetState,
        _generator: &Generator,
        _combinator: &Combinator,
    ) -> DummyPlanetState {
        CiucAI::log_event(
            Some(Participant::new(ActorType::User, state.id())),
            None,
            EventType::MessageOrchestratorToPlanet,
            Channel::Info,
            [("message", "Internal state requested")],
        );

        state.to_dummy()
    }

    fn on_start(&mut self, state: &PlanetState, _generator: &Generator, _combinator: &Combinator) {
        CiucAI::log_event(
            Some(Participant::new(ActorType::User, state.id())),
            None,
            EventType::InternalPlanetAction,
            Channel::Info,
            [("message", "Starting planet's AI")],
        );
    }

    fn on_stop(&mut self, state: &PlanetState, _generator: &Generator, _combinator: &Combinator) {
        CiucAI::log_event(
            Some(Participant::new(ActorType::User, state.id())),
            None,
            EventType::InternalPlanetAction,
            Channel::Info,
            [("message", "Stopping planet's AI")],
        );
    }

}
