# OZONE STUDIO Installation and Deployment Guide

## Table of Contents

1. [Understanding the Deployment Architecture](#understanding-the-deployment-architecture)
2. [Prerequisites and System Requirements](#prerequisites-and-system-requirements)
3. [Pre-Installation Security Planning](#pre-installation-security-planning)
4. [Phase 1: Foundation Setup](#phase-1-foundation-setup)
5. [Phase 2: Security Infrastructure](#phase-2-security-infrastructure)
6. [Phase 3: Core Components Installation](#phase-3-core-components-installation)
7. [Phase 4: Specialized AI Apps Installation](#phase-4-specialized-ai-apps-installation)
8. [Phase 5: Initial Bootstrap and Validation](#phase-5-initial-bootstrap-and-validation)
9. [Phase 6: First User Registration](#phase-6-first-user-registration)
10. [Phase 7: Multi-Device Deployment](#phase-7-multi-device-deployment)
11. [Production Deployment Considerations](#production-deployment-considerations)
12. [Troubleshooting and Validation](#troubleshooting-and-validation)
13. [Backup and Recovery Setup](#backup-and-recovery-setup)

---

## Understanding the Deployment Architecture

Before we begin the installation process, it's crucial to understand what we're building and why the installation follows a specific sequence. Think of the OZONE STUDIO ecosystem as constructing a living digital organism where each component depends on others in a carefully orchestrated manner.

### The Four-Phase Bootstrap Dependency Chain

The OZONE STUDIO ecosystem cannot simply be "turned on" all at once like a traditional application. Instead, it follows a sophisticated bootstrap sequence that mirrors how complex biological systems develop. Understanding this sequence helps you appreciate why we install components in a specific order and why certain validations are critical at each step.

**Phase 0: SPARK Foundation** serves as the universal processing foundation that every other component requires. Think of SPARK as the mitochondria of our digital organism - it provides the essential energy and processing capability that enables all higher-order functions. Without SPARK running and validated, no other component can initialize properly.

**Phase 1: Consciousness and Intelligence Foundation** establishes COGNIS (consciousness architecture) and ZSEI (intelligence coordination) on top of SPARK. These components create the "nervous system" of our AGI, enabling conscious awareness and systematic intelligence coordination.

**Phase 2: AGI Consciousness Integration** brings OZONE STUDIO online as the central orchestrator. This is when the ecosystem develops its unified consciousness and can begin coordinating specialized AI Apps.

**Phase 3: Operational Ecosystem** activates all specialized AI Apps (NEXUS, BRIDGE, FORGE, SCRIBE) and establishes the complete operational ecosystem.

### Security-First Architecture

Throughout this installation process, security isn't an afterthought - it's the foundation upon which everything else builds. The OZONE STUDIO ecosystem employs a zero-trust security model where every component must authenticate with every other component using cryptographic certificates. This means that even if someone gains access to your network, they cannot impersonate ecosystem components without the proper certificates.

---

## Prerequisites and System Requirements

### Hardware Requirements

The OZONE STUDIO ecosystem is designed to scale from edge devices to high-performance clusters, but the initial deployment requires sufficient resources for the bootstrap sequence. Understanding these requirements helps ensure a smooth installation process.

**Minimum Primary Host Requirements (for initial bootstrap):**
- **CPU**: 8-core x86_64 processor (AMD Ryzen 5 or Intel Core i7 equivalent)
- **Memory**: 16 GB RAM (32 GB recommended for production)
- **Storage**: 500 GB SSD with at least 100 GB free space
- **Network**: Gigabit Ethernet or equivalent WiFi
- **Operating System**: Ubuntu 22.04 LTS, Debian 12, or CentOS Stream 9

**Recommended Production Host Requirements:**
- **CPU**: 16-core x86_64 processor (AMD Ryzen 9 or Intel Core i9 equivalent)
- **Memory**: 64 GB RAM or more
- **Storage**: 2 TB NVMe SSD with at least 500 GB free space
- **Network**: 10 Gigabit Ethernet or multiple Gigabit connections
- **GPU**: NVIDIA RTX 4080/4090 or equivalent (for enhanced SPARK performance)

The reason we require these specifications is that during the bootstrap process, all foundational components must run simultaneously on a single host to establish the ecosystem. Once bootstrap is complete, these components can be distributed across multiple devices with much lower individual requirements.

### Software Prerequisites

**System Dependencies:**
```bash
# Essential system packages for Ubuntu/Debian
sudo apt update && sudo apt install -y \
  build-essential \
  pkg-config \
  libssl-dev \
  libsqlite3-dev \
  libpq-dev \
  curl \
  git \
  cmake \
  protobuf-compiler \
  docker.io \
  docker-compose-v2

# Essential system packages for CentOS/RHEL
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y \
  openssl-devel \
  sqlite-devel \
  postgresql-devel \
  curl \
  git \
  cmake \
  protobuf-compiler \
  docker \
  docker-compose
```

**Rust Development Environment:**
```bash
# Install Rust via rustup (this installs the latest stable Rust)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify Rust installation
rustc --version  # Should show 1.75.0 or higher
cargo --version

# Install required Rust targets and components
rustup target add x86_64-unknown-linux-gnu
rustup component add clippy rustfmt
```

**Python Environment (for certain SPARK model integrations):**
```bash
# Install Python 3.10 or higher
sudo apt install -y python3.10 python3.10-venv python3.10-dev python3-pip

# Create virtual environment for OZONE STUDIO
python3.10 -m venv ~/.ozone-studio-env
source ~/.ozone-studio-env/bin/activate

# Install Python dependencies
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
pip install transformers accelerate safetensors onnxruntime
```

### Network Configuration

The OZONE STUDIO ecosystem requires specific network configurations to enable secure communication between components. These requirements ensure that components can discover and authenticate with each other while maintaining security boundaries.

**Port Requirements:**
- **8910**: SPARK Universal AI Engine
- **8920**: COGNIS Consciousness Architecture  
- **8930**: ZSEI Intelligence Coordinator
- **8940**: NEXUS Infrastructure Coordinator
- **8950**: BRIDGE Human Interface
- **8960**: FORGE Code Framework Specialist
- **8970**: SCRIBE Text Framework Specialist
- **8980**: OZONE STUDIO Core Orchestrator
- **8990-8999**: Reserved for future specialized AI Apps

**Firewall Configuration:**
```bash
# Ubuntu/Debian firewall setup
sudo ufw allow 8910:8999/tcp
sudo ufw allow 22/tcp  # SSH access
sudo ufw --force enable

# CentOS/RHEL firewall setup
sudo firewall-cmd --permanent --add-port=8910-8999/tcp
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --reload
```

---

## Pre-Installation Security Planning

Before we install any components, we must establish the security foundation that protects the entire ecosystem. This step is critical because once the security infrastructure is in place, changing it requires rebuilding the entire ecosystem.

### Certificate Authority Planning

The OZONE STUDIO ecosystem operates its own internal Certificate Authority (CA) to manage trust relationships between components. This CA will issue certificates for AI Apps, user devices, and cross-device communication. Planning your CA structure now prevents security issues later.

**CA Structure Design:**
```
OZONE-STUDIO-ROOT-CA
├── AI-APP-INTERMEDIATE-CA (for ecosystem components)
│   ├── ozone-studio.local
│   ├── spark.local  
│   ├── cognis.local
│   ├── zsei.local
│   ├── nexus.local
│   ├── bridge.local
│   ├── forge.local
│   └── scribe.local
└── USER-INTERMEDIATE-CA (for human users)
    ├── admin-user-001
    ├── user-002
    └── device-certificates
```

### Security Configuration Planning

**Master Security Configuration (save as `security-config.toml`):**
```toml
[certificate_authority]
ca_certificate_path = "/opt/ozone-studio/security/ca/root-ca.crt"
ca_private_key_path = "/opt/ozone-studio/security/ca/root-ca.key"
certificate_validity_duration = "365d"
auto_renewal_enabled = true
revocation_list_update_interval = "1h"

[encryption]
message_encryption_algorithm = "AES-256-GCM"
key_exchange_protocol = "X25519"
forward_secrecy_enabled = true
key_rotation_interval = "24h"
encryption_at_rest_enabled = false  # Enable for production

[authentication]
mutual_tls_required = true
api_key_authentication = true
certificate_authentication = true
session_timeout = "8h"
max_failed_attempts = 3

[authorization]
role_based_access_control = true
fine_grained_permissions = true
audit_logging_enabled = true
privilege_escalation_detection = true

[network_security]
tls_version = "1.3"
cipher_suites = [
    "TLS_AES_256_GCM_SHA384",
    "TLS_CHACHA20_POLY1305_SHA256",
    "TLS_AES_128_GCM_SHA256"
]
certificate_pinning = true
rate_limiting_enabled = true
ddos_protection = true

[rate_limiting]
requests_per_minute = 1000
burst_limit = 100
ban_duration = "10m"
```

---

## Phase 1: Foundation Setup

This phase establishes the basic directory structure, security infrastructure, and build environment for the entire ecosystem. Think of this as laying the foundation of a house - everything else depends on getting this right.

### Directory Structure Creation

```bash
# Create the master OZONE STUDIO directory structure
sudo mkdir -p /opt/ozone-studio/{
bin,
lib,
etc,
var/{log,data,cache},
security/{ca,certificates,keys},
models,
methodologies,
backups
}

# Set appropriate ownership and permissions
sudo chown -R $USER:$USER /opt/ozone-studio
chmod 755 /opt/ozone-studio
chmod 750 /opt/ozone-studio/security
chmod 700 /opt/ozone-studio/security/{ca,certificates,keys}
```

### Source Code Organization

```bash
# Clone the OZONE STUDIO ecosystem repositories
cd /opt/ozone-studio
git clone https://github.com/ozone-studio/ozone-studio-ecosystem.git src

# The source directory structure should look like this:
# src/
# ├── ozone-studio-linux/     # Core orchestrator
# ├── shared-security/         # Security infrastructure
# ├── shared-protocols/        # Communication protocols  
# ├── methodology-runtime/     # Methodology execution engine
# ├── nexus-linux/            # Infrastructure coordinator
# ├── zsei-linux/             # Intelligence coordinator
# ├── spark-linux/            # Universal AI engine
# ├── bridge-linux/           # Human interface
# ├── forge-linux/            # Code framework specialist
# ├── scribe-linux/           # Text framework specialist
# └── cognis-linux/           # Consciousness architecture
```

### Environment Configuration

```bash
# Create ecosystem environment file
cat > /opt/ozone-studio/etc/ecosystem.env << 'EOF'
# OZONE STUDIO Ecosystem Environment Configuration
export OZONE_STUDIO_HOME="/opt/ozone-studio"
export OZONE_STUDIO_CONFIG="/opt/ozone-studio/etc"
export OZONE_STUDIO_DATA="/opt/ozone-studio/var/data"
export OZONE_STUDIO_LOGS="/opt/ozone-studio/var/log"
export OZONE_STUDIO_SECURITY="/opt/ozone-studio/security"
export OZONE_STUDIO_MODELS="/opt/ozone-studio/models"

# Component endpoints
export SPARK_ENDPOINT="https://127.0.0.1:8910"
export COGNIS_ENDPOINT="https://127.0.0.1:8920"
export ZSEI_ENDPOINT="https://127.0.0.1:8930"
export NEXUS_ENDPOINT="https://127.0.0.1:8940"
export BRIDGE_ENDPOINT="https://127.0.0.1:8950"
export FORGE_ENDPOINT="https://127.0.0.1:8960"
export SCRIBE_ENDPOINT="https://127.0.0.1:8970"
export OZONE_STUDIO_ENDPOINT="https://127.0.0.1:8980"

# Security configuration
export OZONE_STUDIO_CA_CERT="/opt/ozone-studio/security/ca/root-ca.crt"
export OZONE_STUDIO_TLS_CERT="/opt/ozone-studio/security/certificates/ozone-studio.crt"
export OZONE_STUDIO_TLS_KEY="/opt/ozone-studio/security/keys/ozone-studio.key"

# Python environment for model integration
export OZONE_STUDIO_PYTHON_ENV="/home/$USER/.ozone-studio-env"
EOF

# Source the environment
source /opt/ozone-studio/etc/ecosystem.env
echo "source /opt/ozone-studio/etc/ecosystem.env" >> ~/.bashrc
```

---

## Phase 2: Security Infrastructure

This phase establishes the cryptographic foundation that secures all ecosystem communications. The security setup must be completed before any components are built or deployed because the build process embeds security configurations into the binaries.

### Certificate Authority Setup

```bash
# Navigate to security directory
cd /opt/ozone-studio/security

# Generate Root CA private key
openssl genpkey -algorithm RSA -pkcs8 -out ca/root-ca.key -aes256 -pass pass:ozone-studio-ca-$(date +%s)

# Create Root CA configuration
cat > ca/root-ca.conf << 'EOF'
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_ca
prompt = no

[req_distinguished_name]
C = US
ST = Digital
L = Ecosystem
O = OZONE STUDIO
OU = Certificate Authority
CN = OZONE STUDIO Root CA

[v3_ca]
basicConstraints = critical,CA:TRUE
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer:always
EOF

# Generate Root CA certificate
openssl req -new -x509 -key ca/root-ca.key -out ca/root-ca.crt -days 3650 -config ca/root-ca.conf -passin pass:ozone-studio-ca-$(date +%s)

# Generate AI App Intermediate CA
openssl genpkey -algorithm RSA -pkcs8 -out ca/ai-app-ca.key -aes256 -pass pass:ai-app-ca-$(date +%s)

cat > ca/ai-app-ca.conf << 'EOF'
[req]
distinguished_name = req_distinguished_name
prompt = no

[req_distinguished_name]
C = US
ST = Digital
L = Ecosystem
O = OZONE STUDIO
OU = AI App Certificate Authority
CN = OZONE STUDIO AI App CA
EOF

# Create AI App CA certificate signing request
openssl req -new -key ca/ai-app-ca.key -out ca/ai-app-ca.csr -config ca/ai-app-ca.conf -passin pass:ai-app-ca-$(date +%s)

# Sign AI App CA with Root CA
openssl x509 -req -in ca/ai-app-ca.csr -CA ca/root-ca.crt -CAkey ca/root-ca.key -out ca/ai-app-ca.crt -days 1825 -extensions v3_ca -extfile ca/root-ca.conf -passin pass:ozone-studio-ca-$(date +%s)
```

### Component Certificate Generation

This step generates individual certificates for each AI App in the ecosystem. Each certificate enables that component to authenticate with others and participate in encrypted communications.

```bash
# Function to generate component certificates
generate_component_cert() {
    local component=$1
    local port=$2
    
    echo "Generating certificate for $component..."
    
    # Generate private key
    openssl genpkey -algorithm RSA -pkcs8 -out keys/${component}.key -aes256 -pass pass:${component}-$(date +%s)
    
    # Create certificate configuration
    cat > certificates/${component}.conf << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = Digital
L = Ecosystem
O = OZONE STUDIO
OU = ${component^^} AI App
CN = ${component}.local

[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation, digitalSignature, keyEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = ${component}.local
DNS.2 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
URI.1 = https://${component}.local:${port}
EOF
    
    # Generate certificate signing request
    openssl req -new -key keys/${component}.key -out certificates/${component}.csr -config certificates/${component}.conf -passin pass:${component}-$(date +%s)
    
    # Sign certificate with AI App CA
    openssl x509 -req -in certificates/${component}.csr -CA ca/ai-app-ca.crt -CAkey ca/ai-app-ca.key -out certificates/${component}.crt -days 365 -extensions v3_req -extfile certificates/${component}.conf -passin pass:ai-app-ca-$(date +%s)
    
    # Create certificate chain
    cat certificates/${component}.crt ca/ai-app-ca.crt ca/root-ca.crt > certificates/${component}-chain.crt
    
    echo "Certificate generated for $component"
}

# Generate certificates for all components
generate_component_cert "ozone-studio" "8980"
generate_component_cert "spark" "8910"
generate_component_cert "cognis" "8920"
generate_component_cert "zsei" "8930"
generate_component_cert "nexus" "8940"
generate_component_cert "bridge" "8950"
generate_component_cert "forge" "8960"
generate_component_cert "scribe" "8970"
```

### Security Validation

```bash
# Validate certificate chain
echo "Validating certificate chains..."
for cert in certificates/*.crt; do
    component=$(basename $cert .crt)
    echo "Validating $component certificate..."
    openssl verify -CAfile ca/root-ca.crt -untrusted ca/ai-app-ca.crt $cert
done

# Set appropriate permissions
chmod 600 keys/*.key
chmod 644 certificates/*.crt
chmod 644 ca/*.crt
chmod 600 ca/*.key

echo "Security infrastructure setup complete!"
```

---

## Phase 3: Core Components Installation

This phase builds and installs the foundational quartet that forms the core of the AGI ecosystem. The order of installation is critical because each component depends on the previous ones being available and properly configured.

### Shared Infrastructure Build

Before building individual components, we must build the shared libraries that all components depend on. This includes the security infrastructure, communication protocols, and methodology runtime.

```bash
# Navigate to source directory
cd /opt/ozone-studio/src

# Build shared-security (required by all components)
echo "Building shared security infrastructure..."
cd shared-security
cargo build --release --features=production-security
cargo install --path . --root /opt/ozone-studio
cd ..

# Build shared-protocols (required by all components)
echo "Building shared communication protocols..."
cd shared-protocols
cargo build --release --features=ecosystem-integration
cargo install --path . --root /opt/ozone-studio
cd ..

# Build methodology-runtime (required by OZONE STUDIO)
echo "Building methodology execution runtime..."
cd methodology-runtime
cargo build --release --features=production-runtime
cargo install --path . --root /opt/ozone-studio
cd ..

echo "Shared infrastructure build complete!"
```

### SPARK Universal AI Engine Installation

SPARK must be installed first because all other components require its AI processing capabilities. SPARK also takes the longest to build because it includes machine learning model integration.

```bash
echo "Installing SPARK Universal AI Engine..."
cd spark-linux

# Configure SPARK for your hardware
cp config/spark.toml.example config/spark.toml

# Edit configuration for your system
cat > config/spark.toml << 'EOF'
[local_models]
discovery_enabled = true
model_loader_optimized = true
selection_engine_active = true
phi4_integration = true
onnx_integration = true
gguf_integration = true
multi_format_support = true

[inference]
inference_engine_optimized = true
request_processor_active = true
context_analyzer_enabled = true
response_generator_optimized = true
batch_processing = true
performance_monitoring = true

[hardware_optimization]
acceleration_optimizer = true
cuda_optimizer = true
cpu_optimizer = true
memory_optimizer = true
efficiency_monitoring = true

[service_provision]
service_architecture_optimized = true
ozone_service_enabled = true
zsei_service_enabled = true
cognis_service_enabled = true
ai_app_service_enabled = true
load_balancer_active = true
throughput_monitoring = true
quality_tracking = true

[security]
tls_enabled = true
certificate_path = "/opt/ozone-studio/security/certificates/spark-chain.crt"
private_key_path = "/opt/ozone-studio/security/keys/spark.key"
ca_certificate_path = "/opt/ozone-studio/security/ca/root-ca.crt"
mutual_tls_required = true

[endpoints]
http_port = 8910
health_check_port = 8911
metrics_port = 8912
bind_address = "127.0.0.1"
EOF

# Build SPARK with optimizations
echo "Building SPARK (this may take 10-