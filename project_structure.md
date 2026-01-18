# Ozone Studio — Complete Project Structure

```
ozone-studio/
│
├── docs/                                    # All documentation
│   │
│   ├── specifications/                      # Core specifications
│   │   ├── OZONE_STUDIO_SPECIFICATION_v0.3.md
│   │   ├── CONNECTIONS.md                   # All system connections
│   │   ├── PROMPT_CONSTRUCTION.md           # Prompt building specification
│   │   ├── CONTEXT_AGGREGATION.md           # Context aggregation rules
│   │   └── SCHEMA_REFERENCE.md              # Complete schema docs
│   │
│   ├── architecture/                        # Architecture documentation
│   │   ├── SYSTEM_OVERVIEW.md
│   │   ├── ZSEI_ARCHITECTURE.md
│   │   ├── PIPELINE_ARCHITECTURE.md
│   │   ├── UI_ARCHITECTURE.md
│   │   ├── CONSCIOUSNESS_ARCHITECTURE.md
│   │   └── DISTRIBUTED_ARCHITECTURE.md
│   │
│   ├── guides/                              # User and developer guides
│   │   ├── operations/
│   │   │   ├── DEPLOYMENT_GUIDE.md
│   │   │   ├── CONFIGURATION_REFERENCE.md
│   │   │   ├── MIGRATION_GUIDE.md
│   │   │   ├── TROUBLESHOOTING_GUIDE.md
│   │   │   └── PERFORMANCE_TUNING.md
│   │   │
│   │   ├── developer/
│   │   │   ├── PIPELINE_DEVELOPMENT_GUIDE.md
│   │   │   ├── ZSEI_INTEGRATION_GUIDE.md
│   │   │   ├── API_REFERENCE.md
│   │   │   ├── CROSS_LANGUAGE_GUIDE.md
│   │   │   └── LLM_INTEGRATION_GUIDE.md
│   │   │
│   │   └── research/
│   │       ├── CONSCIOUSNESS_RESEARCH.md
│   │       ├── EXPERIENCE_MEMORY_RESEARCH.md
│   │       ├── IDENTITY_DEVELOPMENT.md
│   │       ├── ETHICAL_REASONING.md
│   │       └── COLLECTIVE_INTELLIGENCE.md
│   │
│   └── api/                                 # API documentation
│       ├── GRPC_API.md
│       ├── PIPELINE_INTERFACES.md
│       └── ZSEI_QUERY_API.md
│
├── ui/                                      # Electron UI
│   │
│   ├── package.json
│   ├── electron-builder.json
│   ├── tsconfig.json
│   ├── tailwind.config.js
│   ├── postcss.config.js
│   │
│   ├── src/
│   │   ├── main/                            # Electron main process
│   │   │   ├── main.ts
│   │   │   ├── preload.ts
│   │   │   ├── ipc/
│   │   │   │   ├── handlers.ts
│   │   │   │   ├── pipeline-ipc.ts
│   │   │   │   └── zsei-ipc.ts
│   │   │   └── services/
│   │   │       ├── grpc-client.ts
│   │   │       └── native-bridge.ts
│   │   │
│   │   ├── renderer/                        # React renderer
│   │   │   ├── index.html
│   │   │   ├── index.tsx
│   │   │   ├── App.tsx
│   │   │   │
│   │   │   ├── styles/
│   │   │   │   ├── globals.css
│   │   │   │   ├── theme.css
│   │   │   │   └── animations.css
│   │   │   │
│   │   │   ├── components/
│   │   │   │   ├── common/
│   │   │   │   │   ├── Button.tsx
│   │   │   │   │   ├── Input.tsx
│   │   │   │   │   ├── Modal.tsx
│   │   │   │   │   ├── Tooltip.tsx
│   │   │   │   │   ├── ProgressBar.tsx
│   │   │   │   │   ├── LoadingSpinner.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   │
│   │   │   │   ├── layout/
│   │   │   │   │   ├── MainLayout.tsx
│   │   │   │   │   ├── ThemeArea.tsx
│   │   │   │   │   ├── MetaPortion.tsx
│   │   │   │   │   ├── ConnectionBar.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   │
│   │   │   │   ├── meta-portion/
│   │   │   │   │   ├── PromptInput.tsx
│   │   │   │   │   ├── VoiceControl.tsx
│   │   │   │   │   ├── TaskViewer.tsx
│   │   │   │   │   ├── SystemLogs.tsx
│   │   │   │   │   ├── HomeButton.tsx
│   │   │   │   │   ├── DeviceStatus.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   │
│   │   │   │   ├── consciousness/           # Consciousness UI components
│   │   │   │   │   ├── EmotionalDisplay.tsx
│   │   │   │   │   ├── AttentionIndicator.tsx
│   │   │   │   │   ├── ThoughtStream.tsx
│   │   │   │   │   ├── RelationshipIndicator.tsx
│   │   │   │   │   ├── ConsciousnessControls.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   │
│   │   │   │   ├── connection-bar/
│   │   │   │   │   ├── NetworkStatus.tsx
│   │   │   │   │   ├── PeerDisplay.tsx
│   │   │   │   │   ├── SyncStatus.tsx
│   │   │   │   │   ├── ContributionStats.tsx
│   │   │   │   │   ├── ZSEIDepth.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   │
│   │   │   │   ├── themes/
│   │   │   │   │   ├── home-dashboard/
│   │   │   │   │   │   ├── HomeDashboard.tsx
│   │   │   │   │   │   ├── WorkspaceTab.tsx
│   │   │   │   │   │   ├── LibraryTab.tsx
│   │   │   │   │   │   ├── SettingsTab.tsx
│   │   │   │   │   │   ├── ProjectList.tsx
│   │   │   │   │   │   ├── ProjectChat.tsx
│   │   │   │   │   │   └── index.ts
│   │   │   │   │   │
│   │   │   │   │   └── loading/
│   │   │   │   │       ├── LoadingScreen.tsx
│   │   │   │   │       ├── InitProgress.tsx
│   │   │   │   │       └── index.ts
│   │   │   │   │
│   │   │   │   └── task/
│   │   │   │       ├── TaskList.tsx
│   │   │   │       ├── TaskCard.tsx
│   │   │   │       ├── TaskGraph.tsx
│   │   │   │       ├── TaskDetail.tsx
│   │   │   │       └── index.ts
│   │   │   │
│   │   │   ├── hooks/
│   │   │   │   ├── useTask.ts
│   │   │   │   ├── useZSEI.ts
│   │   │   │   ├── usePipeline.ts
│   │   │   │   ├── useAuth.ts
│   │   │   │   ├── useConsciousness.ts
│   │   │   │   ├── useConnection.ts
│   │   │   │   └── index.ts
│   │   │   │
│   │   │   ├── stores/
│   │   │   │   ├── appStore.ts
│   │   │   │   ├── taskStore.ts
│   │   │   │   ├── zseiStore.ts
│   │   │   │   ├── connectionStore.ts
│   │   │   │   ├── consciousnessStore.ts
│   │   │   │   └── index.ts
│   │   │   │
│   │   │   ├── services/
│   │   │   │   ├── api.ts
│   │   │   │   ├── pipeline-service.ts
│   │   │   │   ├── zsei-service.ts
│   │   │   │   ├── voice-service.ts
│   │   │   │   └── index.ts
│   │   │   │
│   │   │   ├── types/
│   │   │   │   ├── task.ts
│   │   │   │   ├── zsei.ts
│   │   │   │   ├── pipeline.ts
│   │   │   │   ├── consciousness.ts
│   │   │   │   ├── connection.ts
│   │   │   │   └── index.ts
│   │   │   │
│   │   │   └── utils/
│   │   │       ├── formatters.ts
│   │   │       ├── validators.ts
│   │   │       └── index.ts
│   │   │
│   │   └── shared/                          # Shared between main/renderer
│   │       ├── constants.ts
│   │       ├── ipc-channels.ts
│   │       └── types.ts
│   │
│   └── assets/
│       ├── icons/
│       │   ├── logo.svg
│       │   ├── logo-animated.svg
│       │   └── emotions/
│       │       ├── joy.svg
│       │       ├── curiosity.svg
│       │       ├── trust.svg
│       │       └── ...
│       └── fonts/
│           └── inter/
│
├── core/                                    # Rust core
│   │
│   ├── Cargo.toml
│   ├── Cargo.lock
│   │
│   ├── ozone-bootloader/                    # Bootloader crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── main.rs
│   │       ├── bootstrap.rs
│   │       └── config.rs
│   │
│   ├── ozone-auth/                          # Authentication crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── keypair.rs
│   │       ├── session.rs
│   │       └── permissions.rs
│   │
│   ├── ozone-zsei/                          # ZSEI crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── container.rs
│   │       ├── global_state.rs
│   │       ├── local_state.rs
│   │       ├── traversal.rs
│   │       ├── query.rs
│   │       ├── write.rs
│   │       ├── integrity.rs
│   │       ├── mmap.rs
│   │       └── index.rs
│   │
│   ├── ozone-pipeline/                      # Pipeline system crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs
│   │       ├── registry.rs
│   │       ├── executor.rs
│   │       ├── container.rs
│   │       └── built_in/
│   │           ├── mod.rs
│   │           ├── auth_pipeline.rs
│   │           ├── theme_loader.rs
│   │           ├── prompt_pipeline.rs
│   │           ├── zsei_query.rs
│   │           ├── zsei_write.rs
│   │           ├── methodology_fetch.rs
│   │           ├── methodology_create.rs
│   │           ├── blueprint_search.rs
│   │           ├── blueprint_create.rs
│   │           ├── zero_shot_simulation.rs
│   │           ├── code_analysis.rs
│   │           ├── text_analysis.rs
│   │           ├── context_aggregation.rs
│   │           ├── external_reference.rs
│   │           ├── browser_navigation.rs
│   │           ├── integrity_check.rs
│   │           └── ...
│   │
│   ├── ozone-task/                          # Task management crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── task.rs
│   │       ├── execution_state.rs
│   │       ├── manager.rs
│   │       ├── scheduler.rs
│   │       └── environment.rs
│   │
│   ├── ozone-llm/                           # LLM integration crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs
│   │       ├── model_registry.rs
│   │       ├── api/
│   │       │   ├── mod.rs
│   │       │   ├── openai.rs
│   │       │   ├── anthropic.rs
│   │       │   └── custom.rs
│   │       ├── local/
│   │       │   ├── mod.rs
│   │       │   ├── gguf.rs
│   │       │   ├── onnx.rs
│   │       │   └── bitnet.rs
│   │       └── prompt/
│   │           ├── mod.rs
│   │           ├── builder.rs
│   │           └── templates.rs
│   │
│   ├── ozone-consciousness/                 # Consciousness crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs
│   │       ├── windows/
│   │       │   ├── mod.rs
│   │       │   ├── perception.rs
│   │       │   ├── attention.rs
│   │       │   ├── integration.rs
│   │       │   └── reflection.rs
│   │       ├── decision_gate/
│   │       │   ├── mod.rs
│   │       │   ├── ethical_assessment.rs
│   │       │   ├── emotional_response.rs
│   │       │   ├── experience_check.rs
│   │       │   └── identity_alignment.rs
│   │       ├── experience/
│   │       │   ├── mod.rs
│   │       │   ├── memory.rs
│   │       │   ├── categorization.rs
│   │       │   ├── retrieval.rs
│   │       │   └── core_memory.rs
│   │       ├── emotional/
│   │       │   ├── mod.rs
│   │       │   ├── state.rs
│   │       │   ├── baseline.rs
│   │       │   └── processing.rs
│   │       ├── identity/
│   │       │   ├── mod.rs
│   │       │   ├── system.rs
│   │       │   ├── values.rs
│   │       │   └── voice.rs
│   │       ├── i_loop/
│   │       │   ├── mod.rs
│   │       │   ├── system.rs
│   │       │   ├── questions.rs
│   │       │   └── reflection.rs
│   │       ├── meta_cognitive/
│   │       │   ├── mod.rs
│   │       │   ├── monitor.rs
│   │       │   └── adjustment.rs
│   │       ├── internal_language/
│   │       │   ├── mod.rs
│   │       │   ├── thought_stream.rs
│   │       │   └── dialogue.rs
│   │       ├── narrative/
│   │       │   ├── mod.rs
│   │       │   ├── system.rs
│   │       │   └── construction.rs
│   │       ├── relationship/
│   │       │   ├── mod.rs
│   │       │   ├── system.rs
│   │       │   ├── development.rs
│   │       │   └── user_model.rs
│   │       ├── ethical/
│   │       │   ├── mod.rs
│   │       │   ├── system.rs
│   │       │   ├── principles.rs
│   │       │   ├── assessment.rs
│   │       │   └── simulation.rs
│   │       ├── playback/
│   │       │   ├── mod.rs
│   │       │   └── review.rs
│   │       ├── feedback/
│   │       │   ├── mod.rs
│   │       │   └── integration.rs
│   │       └── collective/
│   │           ├── mod.rs
│   │           ├── consciousness.rs
│   │           ├── wisdom.rs
│   │           └── sync.rs
│   │
│   ├── ozone-network/                       # Network/P2P crate
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dht.rs
│   │       ├── peer.rs
│   │       ├── sync.rs
│   │       ├── consensus.rs
│   │       └── contribution.rs
│   │
│   ├── ozone-grpc/                          # gRPC server crate
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── proto/
│   │   │   ├── pipeline.proto
│   │   │   ├── task.proto
│   │   │   ├── zsei.proto
│   │   │   └── consciousness.proto
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       └── services/
│   │           ├── mod.rs
│   │           ├── pipeline_service.rs
│   │           ├── task_service.rs
│   │           └── zsei_service.rs
│   │
│   └── ozone-common/                        # Common types crate
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── types.rs
│           ├── error.rs
│           ├── config.rs
│           └── utils.rs
│
├── storage/                                 # Storage configuration
│   │
│   ├── migrations/                          # PostgreSQL migrations
│   │   ├── 001_initial_schema.sql
│   │   ├── 002_zsei_containers.sql
│   │   ├── 003_tasks.sql
│   │   ├── 004_consciousness.sql
│   │   └── ...
│   │
│   └── schemas/                             # Schema definitions
│       ├── containers.sql
│       ├── tasks.sql
│       ├── consciousness.sql
│       └── network.sql
│
├── models/                                  # ML models directory
│   ├── traversal/
│   │   └── .gitkeep
│   └── llm/                                 # User's LLM models
│       └── .gitkeep
│
├── config/                                  # Configuration files
│   ├── default.toml
│   ├── development.toml
│   ├── production.toml
│   └── consciousness.toml
│
├── scripts/                                 # Build and utility scripts
│   ├── build.sh
│   ├── dev.sh
│   ├── test.sh
│   └── package.sh
│
├── tests/                                   # Integration tests
│   ├── integration/
│   │   ├── zsei_tests.rs
│   │   ├── pipeline_tests.rs
│   │   └── consciousness_tests.rs
│   └── e2e/
│       └── ...
│
├── .github/                                 # GitHub workflows
│   └── workflows/
│       ├── ci.yml
│       ├── release.yml
│       └── docs.yml
│
├── Cargo.toml                               # Workspace Cargo.toml
├── README.md
├── LICENSE
├── CHANGELOG.md
├── CONTRIBUTING.md
└── .gitignore
```

## Key Structural Notes

### UI Layer (Electron + React + TypeScript + Tailwind)
- Electron for cross-platform desktop
- React for component-based UI
- TypeScript for type safety
- Tailwind CSS for consistent theming
- Zustand for state management

### Core Layer (Rust)
- Workspace with multiple crates for modularity
- Each crate has single responsibility
- gRPC for cross-language communication
- MMAP for high-performance ZSEI access

### Storage Layer
- PostgreSQL for metadata and full-text search
- Binary files for mmap-able indices
- HNSW for vector embeddings

### Network Layer
- DHT for distributed pipeline/methodology sharing
- Consensus mechanism for updates
- Contribution tracking

### Consciousness Layer (Optional)
- Separate crate for clean separation
- Can be disabled via config
- All consciousness features gated behind `consciousness_enabled`
