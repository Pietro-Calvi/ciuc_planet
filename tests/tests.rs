use std::thread;
use std::time::Duration;

use ciuc_planet::ciuc::create_planet;
use ciuc_planet::update_ema;
use common_game::components::asteroid::Asteroid;
use common_game::components::planet::Planet;
use common_game::components::resource::{BasicResource, BasicResourceType};
use common_game::components::sunray::Sunray;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use crossbeam_channel::{Receiver, Sender};

// function to create a temporary planet for testing purposes
fn create_mock_planet() -> (
    Planet,
    Sender<OrchestratorToPlanet>,
    Receiver<PlanetToOrchestrator>,
    Sender<ExplorerToPlanet>,
) {
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

    (planet, tx_orch_in, rx_orch_out, tx_expl_in)
}

//-----------------------Orchestrator to Planet-------------------------

#[test]
fn test_asteroid_with_no_rocket() {
    let (mut planet, tx_orch, rx_orch, _tx_expl) = create_mock_planet();

    // spawn planet
    let _handle = thread::spawn(move || {
        let _ = planet.run();
    });

    tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: _ }) => {}
        _ => panic!("StartPlanetAIResult was not received within the timeout period."),
    }

    thread::sleep(Duration::from_millis(50));

    // Send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();

    // Expect ACK
    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::AsteroidAck {
            planet_id: _,
            rocket,
        }) => {
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

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: _ }) => {}
        _ => panic!("StartPlanetAIResult was not received within the timeout period."),
    }

    thread::sleep(Duration::from_millis(50));

    // Send sunray → expect SunrayAck
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::SunrayAck { planet_id }) => {
            assert_eq!(planet_id, 1, "Planet ID mismatch.");
        }
        _ => panic!("SunrayAck was not received within the timeout period."),
    }

    // Send asteroid → expect AsteroidAck (with rocket)
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::AsteroidAck {
            planet_id: _,
            rocket,
        }) => {
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

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::StartPlanetAIResult { planet_id: _ }) => {}
        _ => panic!("StartPlanetAIResult was not received within the timeout period."),
    }

    thread::sleep(Duration::from_millis(50));

    // 1° sunray → expect ACK (rocket craft)
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::SunrayAck { planet_id }) => {
            assert_eq!(planet_id, 1);
        }
        _ => panic!("SunrayAck #1 missing."),
    }

    // 2° sunray → expect ACK (energy cell charge)
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::SunrayAck { planet_id }) => {
            assert_eq!(planet_id, 1);
        }
        _ => panic!("SunrayAck #2 missing."),
    }

    // 1° asteroid → must use rocket, survive
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::AsteroidAck {
            planet_id: _,
            rocket,
        }) => {
            assert!(rocket.is_some(), "First asteroid should be defended.");
        }
        _ => panic!("AsteroidAck #1 missing."),
    }

    // 2° asteroid → NEW rocket must already be rebuilt
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::AsteroidAck {
            planet_id: _,
            rocket,
        }) => {
            assert!(
                rocket.is_some(),
                "Rocket should be rebuilt for second asteroid."
            );
        }
        _ => panic!("AsteroidAck #2 missing."),
    }

    // Cleanup
    tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();

    match rx_orch.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToOrchestrator::StopPlanetAIResult { planet_id: _ }) => {}
        _ => panic!("StopPlanetAIResult was not received within the timeout period."),
    }

    drop(tx_orch);
    let _ = handle.join();
}

//-----------------------EXPLORER TO PLANET-------------------------

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
    tx_orch
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(50));

    // send the AvailableEnergyCellRequest
    tx_expl
        .send(ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id })
        .unwrap();

    // check the AvailableEnergyCellResponse
    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::AvailableEnergyCellResponse { available_cells }) => {
            assert_eq!(
                available_cells, 5,
                "The planet returned {} available cells instead of 5",
                available_cells
            );
        }
        _ => panic!(
            "No response was received for AvailableEnergyCellRequest within the timeout period."
        ),
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
    tx_orch
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(50));

    // send the SupportedResourceRequest
    tx_expl
        .send(ExplorerToPlanet::SupportedResourceRequest { explorer_id })
        .unwrap();

    // check the SupportedResourceResponse
    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::SupportedResourceResponse { mut resource_list }) => {
            assert!(
                resource_list.contains(&BasicResourceType::Carbon),
                "Expected to have Carbon."
            );
            // Remove Carbon form the HashSet
            resource_list.remove(&BasicResourceType::Carbon);
            // Check if the Hashset is Empty
            assert!(resource_list.is_empty(), "Expected to be empty.");
        }
        _ => panic!(
            "No response was received for AvailableEnergyCellRequest within the timeout period."
        ),
    }

    //Destroy the planet
    tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
    drop(tx_orch);
    let _ = handle.join();
}

#[test] // test the supported combination request function
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
    tx_orch
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(50));

    // send the SupportedCombinationRequest
    tx_expl
        .send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id })
        .unwrap();

    // check the SupportedResourceResponse
    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) => {
            assert!(combination_list.is_empty(), "Expected to be empty.");
        }
        _ => panic!(
            "No response was received for SupportedCombinationResponse within the timeout period."
        ),
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
    tx_orch
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(50));

    // Send a sunray (0 charged cell)
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = _rx_orch.recv_timeout(Duration::from_millis(200));
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = _rx_orch.recv_timeout(Duration::from_millis(200));

    // Send GenerateResourceRequest (Carbon)
    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    // check
    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
            println!("no carbon received, as expected");
        }
        Ok(_) => panic!("I should not receive carbon"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    // Send 2 sunray (2 charged cell)
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = _rx_orch.recv_timeout(Duration::from_millis(200));
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = _rx_orch.recv_timeout(Duration::from_millis(200));
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    // check
    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
            println!("No carbon received, as expected");
        }
        Ok(_) => panic!("I should not receive carbon"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
    let _ = _rx_orch.recv_timeout(Duration::from_millis(200));

    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_some(), "The resource has not been generated.");
            match resource.unwrap() {
                BasicResource::Carbon(_) => assert!(true),
                _ => panic!("An incorrect resource has been generated."),
            }
        }
        _ => panic!(
            "No response was received for GenerateResourceRequest within the timeout period."
        ),
    }

    //Destroy the planet
    tx_orch.send(OrchestratorToPlanet::StopPlanetAI).unwrap();
    drop(tx_orch);
    let _ = handle.join();
}
#[test]
fn test_generate_carbon_statistic_state() {
    let (mut planet, tx_orch, rx_orch, tx_expl) = create_mock_planet();

    // Create an explorer
    let explorer_id = 2;
    let (tx_expl_local, rx_expl_local) = crossbeam_channel::unbounded();

    let _handle = thread::spawn(move || {
        let _ = planet.run();
    });

    // Start the AI
    tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
    tx_orch
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(50));

    // Go in statistic mode

    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));

    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));
    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));

    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));

    // send sunray to create a rocket
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50));
    thread::sleep(Duration::from_millis(100));

    // send sunray to charge an energy cell
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck
    thread::sleep(Duration::from_millis(200));

    // send sunray to charge an energy cell
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200)); // SunrayAck

    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck

    // Send GenerateResourceRequest (Carbon)
    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    // check
    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
            println!("no carbon received, as expected");
        }
        Ok(_) => panic!("I should not receive carbon"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200));
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200));
    thread::sleep(Duration::from_millis(200));

    // ----- Second case and a sunray is coming ----

    // send a GenerateResourceRequest
    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_some(), "The resource has not been generated.");
            match resource.unwrap() {
                BasicResource::Carbon(_) => assert!(true),
                _ => panic!("An incorrect resource has been generated."),
            }
        }
        _ => panic!(
            "No response was received for GenerateResourceRequest within the timeout period."
        ),
    }

    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));

    // ----- First case and a sunray is coming ----

    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_some(), "The resource has not been generated.");
            match resource.unwrap() {
                BasicResource::Carbon(_) => assert!(true),
                _ => panic!("An incorrect resource has been generated."),
            }
        }
        _ => panic!(
            "No response was received for GenerateResourceRequest within the timeout period."
        ),
    }

    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200));

    // ----- First case and a sunray is far away ----

    // send a GenerateResourceRequest
    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_some(), "The resource has not been generated.");
            match resource.unwrap() {
                BasicResource::Carbon(_) => assert!(true),
                _ => panic!("An incorrect resource has been generated."),
            }
        }
        _ => panic!(
            "No response was received for GenerateResourceRequest within the timeout period."
        ),
    }
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200));

    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(700));

    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(200));

    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    // ----- Second case and a sunray is far away ----

    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Ok(PlanetToExplorer::GenerateResourceResponse { resource }) => {
            assert!(resource.is_some(), "The resource has not been generated.");
            match resource.unwrap() {
                BasicResource::Carbon(_) => assert!(true),
                _ => panic!("An incorrect resource has been generated."),
            }
        }
        _ => panic!(
            "No response was received for GenerateResourceRequest within the timeout period."
        ),
    }
}

#[test]
fn test_revert_ai() {
    let (mut planet, tx_orch, rx_orch, tx_expl) = create_mock_planet();

    // Create an explorer
    let explorer_id = 2;
    let (tx_expl_local, rx_expl_local) = crossbeam_channel::unbounded();

    let _handle = thread::spawn(move || {
        let _ = planet.run();
    });

    // Start the AI
    tx_orch.send(OrchestratorToPlanet::StartPlanetAI).unwrap();
    tx_orch
        .send(OrchestratorToPlanet::IncomingExplorerRequest {
            explorer_id,
            new_mpsc_sender: tx_expl_local,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(50));

    // Go in statistic mode

    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));

    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));
    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));

    // send sunray
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck & Rocket built
    thread::sleep(Duration::from_millis(200));

    // send asteroid
    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck
    thread::sleep(Duration::from_millis(200));

    // send sunray to create a rocket
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50));
    thread::sleep(Duration::from_millis(100));

    // send sunray to charge an energy cell
    tx_orch
        .send(OrchestratorToPlanet::Sunray(Sunray::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // SunrayAck
    thread::sleep(Duration::from_millis(200));

    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck

    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck

    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck

    tx_orch
        .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
        .unwrap();
    let _ = rx_orch.recv_timeout(Duration::from_millis(50)); // AsteroidAck

    // Testing the side effect, in safeMode no carbon should be generated
    tx_expl
        .send(ExplorerToPlanet::GenerateResourceRequest {
            explorer_id,
            resource: BasicResourceType::Carbon,
        })
        .unwrap();

    match rx_expl_local.recv_timeout(Duration::from_millis(200)) {
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
            println!("no carbon received, as expected");
        }
        Ok(_) => panic!("I should not receive carbon"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

//-----------------------Other test-------------------------
#[test] // check if the ema function returns the correct result
fn test_ema() {
    let result = update_ema(10.0, 20.0, 0.3);
    assert_eq!(result, 13.0);
    assert_eq!(update_ema(result, 10.0, 0.3), 12.1);
}
