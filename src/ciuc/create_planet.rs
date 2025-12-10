use crate::CiucAI;
use common_game::components::planet::{Planet, PlanetAI, PlanetType};
use common_game::components::resource::BasicResourceType;
use common_game::protocols::messages;
use crossbeam_channel::{Receiver, Sender};

pub fn create_planet(
    rx_orchestrator: Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: Sender<messages::PlanetToOrchestrator>,
    rx_explorer: Receiver<messages::ExplorerToPlanet>,
    id: u32,
) -> Planet {
    let ai_concrete = CiucAI::new();
    let ai_box: Box<dyn PlanetAI> = Box::new(ai_concrete);

    let gen_rules = vec![BasicResourceType::Carbon];
    let comb_rules = vec![];

    Planet::new(
        id,
        PlanetType::A,
        ai_box,
        gen_rules,
        comb_rules,
        (rx_orchestrator, tx_orchestrator),
        rx_explorer,
    )
    .expect("Planet creation failed")
}
