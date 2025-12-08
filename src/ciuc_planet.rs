#![allow(dead_code)]
use crossbeam_channel::{Sender, Receiver};
use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::{Combinator, Generator, BasicResourceType, BasicResource};
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::protocols::messages;
use common_game::protocols::messages::{PlanetToExplorer, PlanetToOrchestrator};

//costanti per la generazione di risorse
mod safe {
    pub const CELLS: u32 = 2;
}

mod statistic {
    pub const FIRST: u32 = 1;
    pub const SECOND: u32 = 2;
}


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

    //funzione per cambiare lo stato
    fn change_state(&mut self)
    {
        //per ora lo fa solo per il safe state solo tenendo conto del numero di asteroidi e sunray
        if matches!(self.state, AIState::SafeState) && self.count_asteroids >= 3 && self.count_sunrays >= 3
        {
            self.state = AIState::StatisticState;
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
    fn deflect_asteroid(&self, planet_state: &mut PlanetState) -> Option<Rocket> {
        planet_state.take_rocket()
    }


    //Funzione per caricare una cella con un sunray
    fn charge_cell_with_sunray(&self, planet_state: &mut PlanetState, sunray:Sunray) -> Result<(), String>
    {
        let result = planet_state.charge_cell(sunray);
        match result {
            None => {
                Ok(())
            },
            Some(_) => {
                Err("Tutte le celle sono cariche".to_string())      //Ho tutte le celle cariche butto il sunray  (SE SI VA IN QUESTO CASO TANTE VOLTE SI PUò ANCHE IMPLEMENTARE UN AI CHE ALLORA GENERA' DI PIù!! (iIDEA PER IL FUTURO))
            }
        }
    }


    //------------------Funzioni per la modalità SAFE------------------------
    fn generate_carbon_if_have_n_safe_cells(&self, planet_state:&mut PlanetState, generator: &Generator, safe_cells:u32) -> Result<common_game::components::resource::Carbon, String> {
        let energy_cell_charged_len = planet_state.cells_iter().filter(|c| c.is_charged()).count() as u32;
        match energy_cell_charged_len {
            0 =>  Err("Non ho energy cell al momento".to_string()),
            6.. => Err("non dovrei essere qui".to_string()),
            charged_cells if charged_cells > safe_cells =>
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
                },
            _ => Err("non dovrei essere qui".to_string())
        }
    }


    //------------------Funzioni per la modalità SAFE------------------------
    fn generate_carbon_safe_state(&self, planet_state:&mut PlanetState, generator: &Generator)-> Result<common_game::components::resource::Carbon, String> {
        self.generate_carbon_if_have_n_safe_cells(planet_state, &generator, safe::CELLS)
    }


    //------------------Funzioni per la modalità STATISTICA------------------------

    fn generate_carbon_statistic_state(&self, planet_state:&mut PlanetState, generator: &Generator) -> Result<common_game::components::resource::Carbon, String> {
        let now = now_ms();
        let time_passed_last_sunray = now - self.last_time_sunray;
        let time_passed_last_asteroid = now - self.last_time_asteroid;

        let mut remove_safe_cell_cause_sunray = 0;
        if (time_passed_last_sunray as f64) > (0.75 * self.estimate_sunray_ms) //se ho un sunray che sta arrivando posso generare ancora più velocemente (la cella safe mi torna subito) (come invertire il polling senza farlo)
        {
            remove_safe_cell_cause_sunray = 1;
        }

        if (time_passed_last_asteroid as f64) < (self.estimate_asteroid_ms / 2.0) //è passato meno della metà della stima
        {
            let safe_cell = statistic::FIRST - remove_safe_cell_cause_sunray; //se c'è un sunray uso una cella in meno tanto mi torna subito

            self.generate_carbon_if_have_n_safe_cells(planet_state, &generator, safe_cell) //genero velocemente tenendomi solo una cella safe
        }
        else
        {
            let safe_cell = statistic::SECOND - remove_safe_cell_cause_sunray; //se c'è un sunray genero con una cella in meno tanto mi torna subito

            self.generate_carbon_if_have_n_safe_cells(planet_state, &generator, safe_cell) //genero meno velocemente tendomi due celle SAFE (cosi po da tornare ad averne una nella prima parte)
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


    //-----------------------FUNZIONI DA CHIAMARE-------------------------
    fn on_sunray(&mut self, planet_state: &mut PlanetState, sunray:Sunray) -> Result<(), String>
    {
        self.update_sunray_esteem(now_ms());
        let res = self.charge_cell_with_sunray(planet_state, sunray);
        let mess_build = self.build_rocket(planet_state);
        self.print_error_message("Creazione rocket dopo sunray".to_string(), mess_build); //metodo per debuggare la risposta del build rocket
        self.change_state();
        res
    }

    fn on_asteroid(&mut self, planet_state: &mut PlanetState) -> Option<Rocket> //se sono stato distrutto true se vivo false
    {
        self.update_asteroid_esteem(now_ms());  //aggiorno la stima
        let rocket = self.deflect_asteroid(planet_state);
        if let Some(_) = rocket {                                               //appena uso il rocket almeno che non sono morto lo ricreo subito (se non ho energy cell lo creerà il prossimo sunray)
            let mess_build = self.build_rocket(planet_state);
            self.print_error_message("Creazione rocket dopo asteroide".to_string(), mess_build); //metodo per debuggare la risposta del build rocket
            self.change_state(); //cambio lo stato se ho una stima utilizzabile e il pianeta non è morto
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
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes()
                })
            },

            messages::ExplorerToPlanet::SupportedCombinationRequest { explorer_id: _ } => {
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes()
                })
            },

            messages::ExplorerToPlanet::GenerateResourceRequest { explorer_id: _, resource: tipo } => {
                match tipo {
                    common_game::components::resource::BasicResourceType::Carbon => {
                        let res = self.generate_carbon(state, generator);  //Cambierei in genera un tipo generale di risorsa tanto restituisce errore se la risorsa non è nella lista delle risorse generabili (implementato in planet.rs)
                        //SEND ACK
                        match res {
                            Ok(carbon) => Some(PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(BasicResource::Carbon(carbon))
                            }),
                            Err(err) =>
                                {
                                    println!("[Ciuc_planet]::GenerateResourceResponse Failed: {}", err);
                                    None
                                },
                        }
                    },
                    _ => {
                        None
                    }
                }
            },

            messages::ExplorerToPlanet::CombineResourceRequest { explorer_id: _, msg: _ } => {
                None //da riguardare penso che bisogna restituire un errore
            },

            messages::ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: _ } => {
                Some(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: state.cells_iter().len() as u32 }) //non dovrebbe mandare il numero di celle che l'explorer può usare??? o quante ne ho totali??
            },

            _ => None
        }

    }



    fn start(&mut self, state: &PlanetState) { /* startup code */ }
    fn stop(&mut self, state: &PlanetState) { /* stop code */ }
}

// This is the group's "export" function. It will be called by
// the orchestrator to spawn your planet.
pub fn create_planet(rx_orchestrator: Receiver<messages::OrchestratorToPlanet>, tx_orchestrator: Sender<messages::PlanetToOrchestrator>, rx_explorer: Receiver<messages::ExplorerToPlanet>, id:u32) -> Planet {
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

// nessun test per:
// internal state -> visto il commento sulla rimossione
// CombineResourceRequest -> capire il messaggio di errore
// test per GenerateResourceRequest in modalità statistica implemento quando decidiamo i parametri di passaggio

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    use common_game::components::asteroid::Asteroid;
    use common_game::components::resource::{BasicResourceType};
    use common_game::components::sunray::Sunray;
    use common_game::protocols::messages::{
        ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator,
    };



    // function to create a temporary planet for testing purposes
    fn create_mock_planet()->(Planet, Sender<OrchestratorToPlanet>, Receiver<PlanetToOrchestrator>, Sender<ExplorerToPlanet>,){
        // channel creation

        // Orchestrator -> Planet
        let (tx_orch_in, rx_orch_in) = crossbeam_channel::unbounded::<OrchestratorToPlanet>();

        // Planet -> Orchestrator
        let (tx_orch_out, rx_orch_out) = crossbeam_channel::unbounded::<PlanetToOrchestrator>();

        // Explorer -> Planet
        let (tx_expl_in, rx_expl_in) = crossbeam_channel::unbounded::<ExplorerToPlanet>();

        // Planet -> Explorer
        let (_tx_expl_out, _rx_expl_out) = crossbeam_channel::unbounded::<PlanetToExplorer>();

        let planet = create_planet(rx_orch_in, tx_orch_out, rx_expl_in, 1);

        (
            planet,
            tx_orch_in,
            rx_orch_out,
            tx_expl_in,
        )
    }

    //################################################
    //#############OrchestratorToPlanet###############
    // ###############################################
    #[test]
    fn test_asteroid_with_no_rocket() {
        let (mut planet, tx_orch, rx_orch, _tx_expl) = create_mock_planet();

        // spawn planet
        let handle = thread::spawn(move || {
            let _ = planet.run();
        });

        tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        thread::sleep(Duration::from_millis(50));

        // Send asteroid
        tx_orch.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();

        // Expect ACK
        match rx_orch.recv_timeout(Duration::from_millis(500)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id: _, rocket }) => {
                assert!(rocket.is_none(), "Planet should have NO rocket.");
            }
            _ => panic!("AsteroidAck was not received within the timeout period."),
        }

        // NO shutdown necessary for this test
    }



    #[test]
    fn test_asteroid_with_rocket() {
        let (mut planet, tx_orch, rx_orch, _tx_expl) = create_mock_planet();

        let handle = thread::spawn(move || {
            let _ = planet.run();
        });

        tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        thread::sleep(Duration::from_millis(50));

        // Send sunray → expect SunrayAck
        tx_orch.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();

        match rx_orch.recv_timeout(Duration::from_millis(500)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id }) => {
                assert_eq!(planet_id, 1, "Planet ID mismatch.");
            }
            _ => panic!("SunrayAck was not received within the timeout period."),
        }

        // Send asteroid → expect AsteroidAck (with rocket)
        tx_orch.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();

        match rx_orch.recv_timeout(Duration::from_millis(500)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id: _, rocket }) => {
                assert!(rocket.is_some(), "Planet should have a rocket and survive.");
            }
            _ => panic!("AsteroidAck was not received within the timeout period."),
        }

        // Cleanup
        tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        drop(tx_orch);
        let _ = handle.join();
    }



    #[test]
    fn test_asteroid_is_the_rocket_rebuild() {
        let (mut planet, tx_orch, rx_orch, _tx_expl) = create_mock_planet();

        let handle = thread::spawn(move || {
            let _ = planet.run();
        });

        tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        thread::sleep(Duration::from_millis(50));

        // 1° sunray → expect ACK (rocket craft)
        tx_orch.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();

        match rx_orch.recv_timeout(Duration::from_millis(500)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id }) => {
                assert_eq!(planet_id, 1);
            }
            _ => panic!("SunrayAck #1 missing."),
        }

        // 2° sunray → expect ACK (energy cell charge)
        tx_orch.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();

        match rx_orch.recv_timeout(Duration::from_millis(500)) {
            Ok(PlanetToOrchestrator::SunrayAck { planet_id }) => {
                assert_eq!(planet_id, 1);
            }
            _ => panic!("SunrayAck #2 missing."),
        }

        // 1° asteroid → must use rocket, survive
        tx_orch.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();

        match rx_orch.recv_timeout(Duration::from_millis(500)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id: _, rocket }) => {
                assert!(rocket.is_some(), "First asteroid should be defended.");
            }
            _ => panic!("AsteroidAck #1 missing."),
        }

        // 2° asteroid → NEW rocket must already be rebuilt
        tx_orch.send(OrchestratorToPlanet::Asteroid(Asteroid::default())).unwrap();

        match rx_orch.recv_timeout(Duration::from_millis(500)) {
            Ok(PlanetToOrchestrator::AsteroidAck { planet_id: _, rocket }) => {
                assert!(rocket.is_some(), "Rocket should be rebuilt for second asteroid.");
            }
            _ => panic!("AsteroidAck #2 missing."),
        }

        // Cleanup
        tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        drop(tx_orch);
        let _ = handle.join();
    }


    //################################################
    //#############ExplorerToPlanet###################
    // ###############################################


    #[test] // test the available energy cell request function
    fn test_available_energy_cell_request() {
        let (mut planet, tx_orch, _rx_orch, tx_expl) = create_mock_planet();

        // Create an explorer
        let explorer_id = 2;
        let (tx_expl_local, rx_expl_local) = crossbeam_channel::unbounded();

        let handle = thread::spawn(move || {
            let _ = planet.run();
        });

        // Start the AI
        tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        tx_orch.send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        }).unwrap();
        thread::sleep(Duration::from_millis(50));

        // send the AvailableEnergyCellRequest
        tx_expl.send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id }).unwrap();

        // check the AvailableEnergyCellResponse
        match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells }) => {
                // La tua CiucAI restituisce hardcoded 5 per questa richiesta
                assert_eq!(available_cells, 5,
                           "The planet returned {} available cells instead of 5", available_cells);
            },
            _ => panic!("No response was received for AvailableEnergyCellRequest within the timeout period."),
        }

        //Destroy the planet
        tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        drop(tx_orch);
        let _ = handle.join();
    }

    #[test] // test the supported resource request request function
    fn test_supported_resource_request() {
        let (mut planet, tx_orch, _rx_orch, tx_expl) = create_mock_planet();

        // Create an explorer
        let explorer_id = 2;
        let (tx_expl_local, rx_expl_local) = crossbeam_channel::unbounded();

        let handle = thread::spawn(move || {
            let _ = planet.run();
        });

        // Start the AI
        tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        tx_orch.send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        }).unwrap();
        thread::sleep(Duration::from_millis(50));

        // send the SupportedResourceRequest
        tx_expl.send(ExplorerToPlanet::SupportedResourceRequest { explorer_id }).unwrap();

        // check the SupportedResourceResponse
        match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToExplorer::SupportedResourceResponse { mut resource_list }) => {
                assert!(resource_list.contains(&BasicResourceType::Carbon), "Expected to have Carbon.");
                // Remove Carbon form the HashSet
                resource_list.remove(&BasicResourceType::Carbon);
                // Check if the Hashset is Empty
                assert!(resource_list.is_empty(), "Expected to be empty.");
            },
            _ => panic!("No response was received for AvailableEnergyCellRequest within the timeout period."),
        }

        //Destroy the planet
        tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        drop(tx_orch);
        let _ = handle.join();
    }

    #[test]  // test the supported combination request function
    fn test_supported_combination_request() {
        let (mut planet, tx_orch, _rx_orch, tx_expl) = create_mock_planet();

        // Create an explorer
        let explorer_id = 2;
        let (tx_expl_local, rx_expl_local) = crossbeam_channel::unbounded();

        let handle = thread::spawn(move || {
            let _ = planet.run();
        });

        // Start the AI
        tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        tx_orch.send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        }).unwrap();
        thread::sleep(Duration::from_millis(50));

        // send the SupportedCombinationRequest
        tx_expl.send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id }).unwrap();

        // check the SupportedResourceResponse
        match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) => {

                assert!(combination_list.is_empty(), "Expected to be empty.");
            },
            _ => panic!("No response was received for SupportedCombinationResponse within the timeout period."),
        }

        //Destroy the planet
        tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        drop(tx_orch);
        let _ = handle.join();
    }



    #[test] // test the generate resource request function in safe state of the AI
    fn test_generate_carbon_safe_state() {
        let (mut planet, tx_orch, _rx_orch, tx_expl) = create_mock_planet();

        // Create an explorer
        let explorer_id = 2;
        let (tx_expl_local, rx_expl_local) = crossbeam_channel::unbounded();

        let handle = thread::spawn(move || {
            let _ = planet.run();
        });

        // Start the AI
        tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
        tx_orch.send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        }).unwrap();
        thread::sleep(Duration::from_millis(50));

        // Send a sunray (0 charged cell)
        tx_orch.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        let _ = _rx_orch.recv_timeout(Duration::from_millis(200));

        // Send GenerateResourceRequest (Carbon)
        tx_expl.send(ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource: BasicResourceType::Carbon }).unwrap();

        // check
        match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {

                println!("no carbon received, as expected");
            },
            Ok(_) => panic!("I should not receive carbon"),
            Err(e) => panic!("Unexpected error: {:?}", e),
        }

        // Send 2 sunray (2 charged cell)
        tx_orch.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        let _ = _rx_orch.recv_timeout(Duration::from_millis(200));
        tx_orch.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        let _ = _rx_orch.recv_timeout(Duration::from_millis(200));
        tx_orch.send(OrchestratorToPlanet::Sunray(Sunray::default())).unwrap();
        // check
        match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {

                println!("No carbon received, as expected");
            },
            Ok(_) => panic!("I should not receive carbon"),
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
        let _ = _rx_orch.recv_timeout(Duration::from_millis(200));

        tx_expl.send(ExplorerToPlanet::GenerateResourceRequest { explorer_id, resource: BasicResourceType::Carbon }).unwrap();


        match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
                assert!(resource.is_some(), "The resource has not been generated.");
                match resource.unwrap() {
                    BasicResource::Carbon(_) => assert!(true),
                    _ => panic!("An incorrect resource has been generated."),
                }
            },
            _ => panic!("No response was received for GenerateResourceRequest within the timeout period."),
        }

        //Destroy the planet
        tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
        drop(tx_orch);
        let _ = handle.join();
    }

    //################################################
    //#############Other test###################
    // ###############################################

    #[test] // check if the ema function returns the correct result
    fn test_ema(){
        let result = update_ema(10.0,20.0,0.3);
        assert_eq!(result, 13.0);
        assert_eq!(update_ema(result,10.0,0.3), 12.1);
    }
}

