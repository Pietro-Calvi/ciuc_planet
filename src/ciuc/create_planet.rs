use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::protocols::orchestrator_planet::PlanetToOrchestrator;
use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
use crate::CiucAI;
use common_game::components::planet::{Planet, PlanetAI, PlanetType};
use common_game::components::resource::BasicResourceType;
use crossbeam_channel::{Receiver, Sender};

pub fn create_planet(
    rx_orchestrator: Receiver<OrchestratorToPlanet>,
    tx_orchestrator: Sender<PlanetToOrchestrator>,
    rx_explorer: Receiver<ExplorerToPlanet>,
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
