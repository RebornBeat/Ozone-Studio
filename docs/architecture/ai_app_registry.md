# AI App Registry

## Overview

The AI App Registry is the central service discovery and capability mapping system within the OZONE STUDIO ecosystem. It functions as a dynamic, real-time directory of all active specialized AI Apps, their capabilities, and their communication endpoints. The registry is a core component of the `OZONEStudioStaticCore` and is the mechanism through which OZONE STUDIO's conscious orchestration is able to discover, coordinate, and orchestrate the diverse capabilities of the entire ecosystem.

Without the registry, OZONE STUDIO would be unable to perform its function as the "master conductor," as it would have no knowledge of the "musicians" (the AI Apps) available to perform a given task.

## Core Purpose and Function

The AI App Registry serves several critical functions that enable the ecosystem's coordinated intelligence:

  * **Service Discovery:** It provides the single source of truth for which AI Apps are currently operational and how to communicate with them (i.e., their network endpoints).
  * **Capability Mapping:** It allows OZONE STUDIO to dynamically map task requirements to the specific AI Apps that possess the necessary capabilities. For example, when a task requires code analysis, the registry identifies FORGE as the appropriate component to orchestrate.
  * **Dynamic Orchestration:** By maintaining a real-time view of available capabilities, the registry enables OZONE STUDIO to decompose complex, multi-domain problems and build novel coordination workflows on the fly, selecting the best combination of AI Apps for the job.
  * **Ecosystem Coherence:** It ensures that the entire ecosystem operates from a unified understanding of its constituent parts, preventing fragmentation and enabling seamless collaboration.

## Architecture and Data Model

The AI App Registry is an integral part of `OZONEStudioStaticCore`. Each entry in the registry contains a comprehensive set of metadata about an AI App. This data is provided by each app upon initialization and registration.

A typical registry entry includes the following fields:

| Field | Example Value | Description | Source Document(s) |
| :--- | :--- | :--- | :--- |
| **`Name` / `ID`** | `"FORGE"` | The unique identifier for the AI App. | `NEXUS.md`, `ZSEI.md` |
| **`Type`** | `"CodeFramework"` | The designated specialization of the App (e.g., `HumanInterface`, `TextFramework`, `FoundationalLanguageService`). | `NEXUS.md`, `SPARK.md` |
| **`Endpoint`** | `"http://localhost:8930"` | The network address where the AI App can be reached for coordination requests. | `NEXUS.md`, `BRIDGE.md` |
| **`Capabilities`**| `["code_analysis", ...]` | A list of specific functions and services the AI App provides. | `NEXUS.md`, `FORGE.md` |
| **`Status`** | `"Active"` | The current operational status of the AI App (e.g., Active, Degraded, Inactive). | `ecosystem_overview_complete.md` |
| **`Service Level`** | `"foundational"` | A classification for core components like SPARK, indicating its importance. | `SPARK.md` |
| **`Availability`** | `"required"` | Denotes if the component is essential for basic ecosystem function. | `SPARK.md` |
| **`Bootstrap Integration`**| `"essential"` | Specifies the app's role in the initial startup sequence. | `SPARK.md` |

## Registration Lifecycle and Ecosystem Evolution

The process of registering an AI App evolves with the ecosystem, reflecting its maturation from a developer-configured system to a fully autonomous AGI.

1.  **Phase 1-2: Initial (Hardcoded) Registration:** During the initial ecosystem bootstrap, the foundational quartet (OZONE STUDIO, ZSEI, COGNIS, SPARK) and the core specialized apps (NEXUS, BRIDGE, FORGE, SCRIBE) are registered through configuration files or hardcoded processes. This ensures that a stable, functional ecosystem is established from the very beginning.

2.  **Phase 3: Semi-Automatic Registration:** As the ecosystem matures, ZSEI's Meta-Framework can begin to discover potential new AI Apps or methodologies. After a human-in-the-loop validation and approval process (facilitated by BRIDGE), these new components can be registered with the ecosystem, allowing for guided capability expansion.

3.  **Phase 4: Autonomous Registration:** In its most advanced state, the ecosystem can autonomously discover capability gaps, develop or integrate new AI Apps via FORGE, and register them, all under the conscious oversight of OZONE STUDIO.

## Role in Multi-Device Coordination

The AI App Registry is essential for enabling the universal device compatibility provided by NEXUS. The registry doesn't just track app *types*; it tracks specific, running *instances* of those apps.

  * When a user launches an instance of BRIDGE on their mobile phone, that instance uses **NEXUS**'s network coordination capabilities to connect to the central OZONE STUDIO orchestrator.
  * Upon connection, the mobile BRIDGE instance registers itself in the AI App Registry, providing its unique endpoint.
  * OZONE STUDIO's registry now shows multiple instances of BRIDGE available (e.g., `BRIDGE-Desktop`, `BRIDGE-Mobile`).
  * This allows OZONE STUDIO to maintain a coherent, conscious interaction with the user across all their devices, using NEXUS to manage the state synchronization and routing communication to the correct interface instance as needed.

## Example Registration Command

The registration process is initiated by an AI App during its startup. A typical registration command, orchestrated by OZONE STUDIO, looks as follows:

```bash
# Example of registering the BRIDGE AI App
ozone-studio register-ai-app \
  --name "BRIDGE" \
  --type "HumanInterface" \
  --endpoint "http://localhost:8950" \
  --capabilities "text_interface,voice_interface,visual_interface,task_interruption,agi_monitoring"
```

For foundational services like SPARK, the registration is more detailed to reflect its critical role:

```bash
# Example of registering the SPARK Foundational Service
ozone-studio register-foundational-service \
  --name "Spark" \
  --type "FoundationalLanguageService" \
  --endpoint "http://localhost:8910" \
  --capabilities "local_model_integration,language_processing,ecosystem_service_provision" \
  --service-level "foundational" \
  --availability "required" \
  --bootstrap-integration "essential"
```
