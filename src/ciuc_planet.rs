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


//The state define the AI logic
enum AIState
{
    SafeState, //The initial state, the world wait some data for change in StatisticState
    StatisticState //The final state, the world use more cells when a asteroid is distant (esteem time)
}


// Group-defined AI struct
pub struct CiucAI { /* your AI state here */
    state: AIState,
    number_explorers: usize,
    count_asteroids: u32,
    count_sunrays: u32,
    last_time_sunray: i64,
    last_time_asteroid: i64,
    estimate_sunray_ms: f64,
    estimate_asteroid_ms: f64,
}



//funzioni di supporto per il calcolo della stima
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



impl CiucAI
{

    fn new() -> Self
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

    //--------FUNZIONI PRIVATE INTERNE-----------------

    //funzioni per l'aggiornamento della stima sunray e asteroidi:
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

    //Funzione per cambiare stato per ora cambia solo da safestate a stastisticstate
    fn change_state(&mut self) -> Result<(),String>
    {
        let a = &self.state;
        match a {
            AIState::SafeState => {
                self.state = AIState::StatisticState;
                Ok(())
            },
            AIState::StatisticState => {
                Err("Per ora non è previsto nessun cambiamento di stato".to_string())
            },
            _ => Err("Stato non riconosciuto".to_string())
        }
    }


    //Funzione per costruisce un Rocket se si ha almeno una cella carica
    fn build_rocket(&self, planet_state:&mut PlanetState) -> Result<(), String>
    {
        match planet_state.full_cell() {
            None => {
                Err("Non sono riuscito a creare il razzo".to_string())
            },
            Some((_cell, i)) => {
                planet_state.build_rocket(i)
            }
        }
    }

    //Funzione per distruggere un asteroide (se non si ha il razzo si viene distrutti)
    fn deflect_asteroid(&self, planet_state:&mut PlanetState) -> Option<Rocket>
    {
        if !planet_state.has_rocket() {
            None
        }
        else {
            let rocket = planet_state.take_rocket();
            rocket
        }
    }


    //Funzione per caricare una cella con un sunray
    fn charge_cell_with_sunray(&mut self, planet_state: &mut PlanetState, sunray:Sunray) -> Result<(), String>
    {
        let result = planet_state.charge_cell(sunray);
        match result {
            None => {
                Ok(())
            },
            Some(cell) => {
                Err("Tutte le celle sono cariche".to_string())      //Ho tutte le celle cariche butto il sunray  (SE SI VA IN QUESTO CASO TANTE VOLTE SI PUò ANCHE IMPLEMENTARE UN AI CHE ALLORA GENERA' DI PIù!! (iIDEA PER IL FUTURO))
            }
        }
    }


    //------------------Funzioni per la modalità SAFE------------------------
    fn generate_carbon_safe_state(&self, planet_state:&mut PlanetState, generator: &Generator) -> Result<common_game::components::resource::Carbon, String> {
        let energy_cell_charged_len = planet_state.cells_iter().enumerate().map(|(i, cell)| { if cell.is_charged() {1} else {0} }).sum::<u32>();
        match energy_cell_charged_len {
            0 =>  Err("Non ho energy cell al momento".to_string()),
            1..3 =>  Err(format!("Per il mio comportamento non ho abbastanza energy cell. Numero attuale: {:?}", energy_cell_charged_len)),
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


    fn print_error_message(&self, incipt:String, msg:Result<(), String>)
    {
        match msg {
            Ok(_) => {
                println!("[CiucWorld]: {} andato a buon fine", incipt);
            },
            Err(e) => println!("[CiucWorld]: {} ha avuto un errore: {}",incipt, e),
        }
    }

    //------------------Funzioni per la modalità STATISTICA------------------------

    fn generate_carbon_statistic_state(&self, planet_state:&mut PlanetState, generator: &Generator) -> Result<common_game::components::resource::Carbon, String> {
        Err("Ancora non implementata".to_string())
    }

    //-----------------------FUNZIONI DA CHIAMARE-------------------------
    fn on_sunray(&mut self, planet_state: &mut PlanetState, sunray:Sunray) -> Result<(), String>
    {
        self.update_sunray_esteem(now_ms());
        let res = self.charge_cell_with_sunray(planet_state, sunray);
        let mess_build = self.build_rocket(planet_state);
        self.print_error_message("Creazione rocket dopo sunray".to_string(), mess_build); //metodo per debuggare la risposta del build rocket
        res
    }

    fn on_asteroid(&mut self, planet_state: &mut PlanetState) -> Option<Rocket> //se sono stato distrutto true se vivo false
    {
        self.update_asteroid_esteem(now_ms());  //aggiorno la stima
        let rocket = self.deflect_asteroid(planet_state);
        if !rocket.is_none() {                                                  //appena uso il rocket almeno che non sono morto lo ricreo subito (se non ho energy cell lo creerà il prossimo sunray)
            let mess_build = self.build_rocket(planet_state);
            self.print_error_message("Creazione rocket dopo asteroide".to_string(), mess_build); //metodo per debuggare la risposta del build rocket
        }
        rocket
    }

    fn generate_carbon(&self, planet_state:&mut PlanetState, generator: &Generator) -> Result<common_game::components::resource::Carbon, String>
    {
        match &self.state {
            AIState::SafeState => {
                self.generate_carbon_safe_state(planet_state, generator)
            },
            AIState::StatisticState => {
                self.generate_carbon_statistic_state(planet_state, generator)
            }
        }
    }
}



impl PlanetAI for CiucAI
{

    //Handle per la gestione degli asteroidi
    fn handle_asteroid(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, ) -> Option<Rocket> { //OK
        self.on_asteroid(state)
    }


    //Handel per la gestione di scambio dei messaggi con l'orchestrator
    fn handle_orchestrator_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: messages::OrchestratorToPlanet) -> Option<messages::PlanetToOrchestrator> {

        match msg {
            messages::OrchestratorToPlanet::Sunray(sun) => { //OK
                let message = self.on_sunray(state, sun); //restituisce errore se tutte le celle sono cariche
                self.print_error_message("ricezione del sunray".to_string(), message); //debug per capire cosa è successo
                Some(PlanetToOrchestrator::SunrayAck { planet_id: state.id() }) //restituisco messaggio con l'ack
            },
            messages::OrchestratorToPlanet::InternalStateRequest => { //FORSE VA RIMOSSA (MI SA CHE NON SI USA PIU)
                Some(PlanetToOrchestrator::InternalStateResponse {
                    planet_id: state.id(),
                    planet_state: state.to_dummy(),
                })
            },
            _ => None
        }
    }


    fn handle_explorer_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: messages::ExplorerToPlanet) -> Option<messages::PlanetToExplorer> {

        match msg {
            messages::ExplorerToPlanet::SupportedResourceRequest { explorer_id: _ } => {
                //restituire carbonio
            }
            messages::ExplorerToPlanet::SupportedCombinationRequest { explorer_id: _ } => {
                //Restituire nessuna combination rule
            }
            messages::ExplorerToPlanet::GenerateResourceRequest { explorer_id: _, resource: _ } => {
                //Controllare che la risorsa sia corretta
                let res = self.generate_carbon(state, generator);

                //SEND ACK
            }
            messages::ExplorerToPlanet::CombineResourceRequest { explorer_id: _, msg: _ } => {
                //Restituire il nulla
            }
            messages::ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: _ } => {
                return Some(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: 5 })
            }
        }

        None
    }



    fn start(&mut self, state: &PlanetState) { /* startup code */ }
    fn stop(&mut self, state: &PlanetState) { /* stop code */ }
}

// This is the group's "export" function. It will be called by
// the orchestrator to spawn your planet.
pub fn create_planet(rx_orchestrator: mpsc::Receiver<messages::OrchestratorToPlanet>, tx_orchestrator: mpsc::Sender<messages::PlanetToOrchestrator>, rx_explorer: mpsc::Receiver<messages::ExplorerToPlanet>, id: u32) -> Planet {

    let ai_concrete = CiucAI::new();
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


