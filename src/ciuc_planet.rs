use std::marker::PhantomData;
use std::sync::mpsc;
use common_game::components::asteroid::Asteroid;
use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::{Combinator, Generator, BasicResourceType};
use common_game::components::resource::BasicResource::Carbon;
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::protocols::messages;
use common_game::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};

struct SafeState;
struct StatisticState;

// Group-defined AI struct
pub struct AI<T> { /* your AI state here */
    state: PhantomData<T>,
    number_explorers: usize,
    count_asteroids: u32,
    count_sunrays: u32,
    last_time_sunray: i64,
    last_time_asteroid: i64,
    estimate_sunray_ms: f64,
    estimate_asteroid_ms: f64,
}




impl CiucAi for AI<SafeState> {
    fn generate_carbon(&self, planet_state:&mut PlanetState, generator: &Generator) -> Result<common_game::components::resource::Carbon, String> {
        let energy_cell_charged = planet_state.cells_iter().enumerate().map(|(i, cell)| { if cell.is_charged() {1} else {0} }).sum::<u32>();
        match energy_cell_charged {
            0 =>  Err("Non ho energy cell al momento".to_string()),
            1..3 =>  Err(format!("Per il mio comportamento non ho abbastanza energy cell. Numero attuale: {:?}", energy_cell_charged)),
            3..6 => //da sostituire con cell leng
                {
                    let first_energy_cell_charged = planet_state.full_cell();
                    match first_energy_cell_charged {
                        Some((cell,_)) => {
                            generator.make_carbon(cell)
                        },
                        None => {
                            Err("non dovrei essere qui".to_string())
                        }
                    }
                }
            _ => Err("non dovrei essere qui".to_string())
        }
    }


}


fn update_ema(prev: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * prev
}
fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}



impl<T> AI<T>
{

    fn change_state(self) -> AI<T>
    {
        let a =  AI::<T> {
            state: PhantomData,
            number_explorers: 0,
            count_asteroids: 0,
            count_sunrays: 0,
            last_time_sunray: 0,
            last_time_asteroid: 0,
            estimate_asteroid_ms: 0.0,
            estimate_sunray_ms: 0.0,
        };
        a
    }
    fn mains(mut self)
    {
        self = self.change_state();
    }
    fn update_sunray_esteem(&mut self, now_ms: i64) {
        if self.last_time_sunray > 0 {
            let delta = (now_ms - self.last_time_sunray) as f64;
            self.estimate_sunray_ms = update_ema(self.estimate_sunray_ms, delta, 0.3);
            self.count_sunrays += 1;
        }
        self.last_time_sunray = now_ms;
    }
    fn update_asteroid_esteem(&mut self, now_ms: i64) {
        if self.last_time_asteroid > 0 {
            let delta = (now_ms - self.last_time_asteroid) as f64;
            self.estimate_asteroid_ms = update_ema(self.estimate_asteroid_ms, delta, 0.3);
            self.count_asteroids += 1;
        }
        self.last_time_asteroid = now_ms;
    }
    fn on_sunray(&mut self, state: &mut PlanetState, sunray:Sunray) -> Result<(), String>
    {
        let now = now_ms();
        self.update_sunray_esteem(now);
        let result = state.charge_cell(sunray);
        match result {
            None => {
                Ok(())
            },
            Some(cell) => {
                Err("Tutte le celle sono cariche".to_string())
                //Ho tutte le celle cariche butto il sunray
            }
        }
    }

    fn on_asteroid(&mut self, state: &mut PlanetState) -> Option<Rocket> //se sono stato distrutto true se vivo false
    {
        let now = now_ms();
        self.update_asteroid_esteem(now);
        if !state.has_rocket() {
            None
        }
        else {
            let rocket = state.take_rocket();
            match state.full_cell() {
                None => {
                    println!("Non sono riuscito a creare il razzo");
                },
                Some((_cell, i)) => {
                    // assert!(cell.is_charged());
                    let _ = state.build_rocket(i);
                }
            }
            rocket
        }
    }
}

pub trait CiucAi
{
    //IL RAZZO C'E'
    fn generate_carbon(&self, planet_state:&mut PlanetState, generator: &Generator) -> Result<common_game::components::resource::Carbon, String>;

}

impl<T: std::marker::Send> PlanetAI for AI<T> where AI<T>: CiucAi
{
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: messages::OrchestratorToPlanet
    ) -> Option<messages::PlanetToOrchestrator> {

        match msg {
            messages::OrchestratorToPlanet::Sunray(sun) => {
                //Se ho una cella scarica la carico
                let message = self.on_sunray(state, sun); //restituisce se tutte le celle sono cariche
                // aggiorno il numero di sunray
                //creo subito il razzo se non lo ho
                if !state.has_rocket() {
                    //ho sicuramente una cella carica
                    let charged_cell_index = state.full_cell().unwrap().1; //volendo si può fare il match ma dovrebbe esserci sicuramente
                    let _ = state.build_rocket(charged_cell_index); //qua restituisce result MA IN TEORIA NON PUOI MAI ESSERE IN ERRORE
                }

                //se sono in stato safe e ho abbastanza dati entro in stato statistico


            }
            messages::OrchestratorToPlanet::InternalStateRequest => {
                //Restituisco state
            }
            messages::OrchestratorToPlanet::IncomingExplorerRequest { explorer_id: _, new_mpsc_sender: _ } => {
                //metto sender se possibile
                if true {
                    self.number_explorers += 1;
                }

            }
            messages::OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id: _ } => {
                //Elimino il mio sender per explorer
                if true {
                    self.number_explorers -= 1;
                }
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
                let a = self.generate_carbon(state, generator);

                //SEND ACK
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

        self.on_asteroid(state);

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
        count_asteroids: 0,
        count_sunrays: 0,
        last_time_sunray: 0,
        last_time_asteroid: 0,
        estimate_asteroid_ms: 0.0,
        estimate_sunray_ms: 0.0,
    };

    // mettilo nello heap come trait object: Box<dyn PlanetAI>
    let ai_box: Box<dyn PlanetAI> = Box::new(ai_concrete);

    let gen_rules = vec![BasicResourceType::Carbon]; //PENSIAMOCI
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



