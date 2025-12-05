use std::marker::PhantomData;
use std::sync::mpsc;
use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::{Combinator, Generator, BasicResourceType};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages;
use common_game::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};

struct SafeState;
struct StatisticState;

// Group-defined AI struct
pub struct AI<T> { /* your AI state here */
    state: PhantomData<T>,
    number_explorers: usize
}

impl AI<SafeState> {
    fn generate_carbon(){
        println!("Generating carbon safe");
    }
}

impl AI<StatisticState> {
    fn generate_carbon(){
        println!("Generating carbon statistically");
    }
}


impl<T: std::marker::Send> PlanetAI for AI<T> {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: messages::OrchestratorToPlanet
    ) -> Option<messages::PlanetToOrchestrator> {

        match msg {
            messages::OrchestratorToPlanet::Sunray(..) => {
                //Se ho una cella scarica la carico

                // aggiorno il numero di sunray

                //se sono in stato safe e ho abbastanza dati entro in stato statistico
            }
            messages::OrchestratorToPlanet::InternalStateRequest => {
                //Restituisco state
            }
            messages::OrchestratorToPlanet::IncomingExplorerRequest { explorer_id: _, new_mpsc_sender: _ } => {
                //Sostituisco il nuovo sender e gli restituisco il mio
            }
            messages::OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: _ } => {
                //Elimino il mio sender per explorer
            }

            _ => {}
        }

        None
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: messages::ExplorerToPlanet
    ) -> Option<messages::PlanetToExplorer> {

        match msg {
            messages::ExplorerToPlanet::SupportedResourceRequest { explorer_id: _ } => {
                //Restituire Carbonio
            }
            messages::ExplorerToPlanet::SupportedCombinationRequest { explorer_id: _ } => {
                //Restituire nessuna combination rule
            }
            messages::ExplorerToPlanet::GenerateResourceRequest { explorer_id: _, resource: _ } => {
                //Controllare che la risorsa sia corretta

                //LASCIALO
                // self.generate_carbon();
            }
            messages::ExplorerToPlanet::CombineResourceRequest { explorer_id: _, msg: _ } => {
                //Restituire il nulla
            }
            messages::ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: _ } => {
                //Restituire numero di energy cell available
            }
        }

        None
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {

        //Se non ho un rocket muoio

        //se ho il rocket mi difendo e modifico il conteggio di aseteroidi

        //se sono in stato safe e ho abbastanza dati entro in stato statistico

        None
    }

    fn start(&mut self, state: &PlanetState) { /* startup code */ }
    fn stop(&mut self, state: &PlanetState) { /* stop code */ }
}

// This is the group's "export" function. It will be called by
// the orchestrator to spawn your planet.
pub fn create_planet(
    rx_orchestrator: mpsc::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: mpsc::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: mpsc::Receiver<messages::ExplorerToPlanet>,
    tx_explorer: mpsc::Sender<messages::PlanetToExplorer>,
    id: u32
) -> Planet {
    // crea l'AI concreto
    let ai_concrete = AI::<SafeState> {
        state: PhantomData,
        number_explorers: 0,
    };

    // mettilo nello heap come trait object: Box<dyn PlanetAI>
    let ai_box: Box<dyn PlanetAI> = Box::new(ai_concrete);

    let gen_rules = vec![BasicResourceType::Carbon];
    let comb_rules = vec![];

    // PASSA i singoli canali (non tuple) e l'AI boxed
    Planet::new(
        id,
        PlanetType::A,
        ai_box,
        gen_rules,
        comb_rules,
        (rx_orchestrator, tx_orchestrator),
        rx_explorer
    ).expect("Planet creation failed")
}



