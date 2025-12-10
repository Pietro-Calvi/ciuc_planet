# Ciuc AI Library

## Introduction

This Rust library implements the **Ciuc Planet AI** for managing planetary resources, building rockets, and handling energy cells in a simulation environment.

Full documentation is available at [Ciuc Docs](https://cristiancazzanigaunitn.github.io/ciuc_planet_documentation/).

## Usage

To create and use a planet with Ciuc AI:

```
use ciuc::create_planet;

// Assuming you have already defined these channels:
let planet = create_planet(rx_orchestrator, tx_orchestrator, rx_explorer, 1);
```

The AI handles:

- Resource generation (carbon)
- Energy cell charging and management
- Rocket building
- Asteroid deflection
- Logging of AI actions

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
