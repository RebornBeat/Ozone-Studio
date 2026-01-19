ozone-studio/
├── core/                    # Rust core (single crate)
│   ├── Cargo.toml
│   ├── build.rs            # Proto compilation
│   └── src/
│       ├── main.rs         # Entry point
│       ├── lib.rs          # Library exports
│       ├── bootstrap.rs    # Initialization sequence
│       ├── config.rs       # Configuration loading
│       ├── auth.rs         # Authentication
│       ├── session.rs      # Session management
│       ├── zsei/           # ZSEI implementation
│       ├── pipelines/      # Pipeline system
│       └── grpc/           # gRPC server
│
├── ui/                      # Electron UI
│   ├── package.json
│   ├── electron-builder.json
│   └── src/
│       ├── main/           # Electron main process
│       ├── renderer/       # React renderer
│       └── shared/         # Shared types
│
├── storage/                 # Database schemas
│   ├── migrations/
│   └── schemas/
│
├── config/                  # Configuration files
│   ├── default.toml
│   └── development.toml
│
└── scripts/                 # Build scripts
    ├── build.sh
    ├── dev.sh
    └── package.sh
