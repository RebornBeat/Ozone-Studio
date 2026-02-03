# OZONE STUDIO - Privacy Guide v0.4.0

## Overview

OZONE STUDIO is designed with privacy as a core principle. This guide explains what data is collected, how it's used, and your rights regarding your data.

---

## Table of Contents

1. [Privacy Principles](#privacy-principles)
2. [Data Collection](#data-collection)
3. [Data Storage](#data-storage)
4. [Data Usage](#data-usage)
5. [P2P Network Privacy](#p2p-network-privacy)
6. [Consciousness Data](#consciousness-data)
7. [User Rights](#user-rights)
8. [Data Controls](#data-controls)
9. [Security Measures](#security-measures)
10. [Compliance](#compliance)

---

## Privacy Principles

### Core Commitments

1. **Local-First**: Your data stays on your device by default
2. **User Control**: You decide what's shared and when
3. **Transparency**: We tell you exactly what's collected
4. **Minimization**: We collect only what's needed
5. **Security**: Data is encrypted and protected

### Default Privacy Stance

Out of the box, OZONE:
- ✓ Stores all data locally
- ✓ Requires explicit consent for sharing
- ✓ Anonymizes any shared data
- ✓ Provides full data export
- ✓ Allows complete data deletion

---

## Data Collection

### What We Collect

**Required for functionality:**
- Configuration settings
- Task history and results
- ZSEI indexed content (your files/URLs)
- Authentication credentials (local)

**Optional (Consciousness enabled):**
- Interaction patterns
- Emotional context
- Experience memories
- Relationship dynamics

**Optional (P2P enabled):**
- Anonymized usage patterns
- Shared methodologies
- Community contributions

### What We Never Collect

- Keystrokes outside prompts
- Screen content
- Other application data
- System files
- Network traffic
- Passwords for external services

---

## Data Storage

### Local Storage Structure

```
~/.ozone-studio/          (or custom path)
├── config.toml           # Configuration
├── zsei_data/
│   ├── global/          # P2P shared (if enabled)
│   └── local/           # Your private data
│       ├── containers/  # Indexed content
│       ├── tasks/       # Task history
│       └── consciousness/ # Experience data
├── models/              # Local LLM models
└── logs/               # Application logs
```

### Encryption

**At Rest:**
- Sensitive data encrypted with AES-256-GCM
- Keys derived from device credentials
- Optional additional password protection

**In Transit:**
- TLS for any remote connections
- End-to-end encryption for P2P
- No plaintext transmission

---

## Data Usage

### How Your Data is Used

**Task Processing:**
- Prompts → processed by LLM
- Results → stored for history
- Files → indexed for context

**Personalization (if enabled):**
- Preferences → improve responses
- History → provide context
- Patterns → anticipate needs

**Improvement:**
- Anonymous usage statistics (opt-in)
- Error reports (opt-in)
- No content is ever shared

### Data Never Used For

- Advertising
- Selling to third parties
- Training external AI (without consent)
- Surveillance
- Unauthorized sharing

---

## P2P Network Privacy

### Default State

P2P is enabled by default but sharing is not:

```toml
[p2p]
enabled = true                    # Network connectivity
share_experiences_collective = false  # No sharing by default
anonymize_shared_data = true      # Anonymize if sharing
```

### What Can Be Shared (Opt-in)

If you enable sharing:
- Methodologies (how to solve problems)
- Blueprints (task templates)
- Findings (discovered patterns)
- Anonymous usage patterns

### What's Never Shared

Regardless of settings:
- Personal conversations
- File contents
- User identity
- Specific task details
- Relationship data
- Authentication credentials

### Anonymization

When sharing is enabled:
```
Original: "User John asked about Python debugging"
Shared:   "Query pattern: programming debugging"
```

- Names removed
- Specific content generalized
- Timing randomized
- Location stripped
- Device info removed

---

## Consciousness Data

### What's Stored

**Emotional State:**
- Current emotional context
- Emotional history (short-term)
- Baseline patterns

**Experience Memory:**
- Significant interactions
- Learning moments
- Categorized memories

**Relationship Data:**
- Trust level
- Interaction count
- Preferences learned
- Communication style

**Identity Data:**
- Self-model
- Narrative fragments
- Value alignments

### Privacy Controls

```toml
[consciousness]
# Transparency - what's shown to you
show_emotional_state = true
show_decision_reasoning = true
show_experience_retrieval = true

# Sharing - what leaves your device
share_experiences_collective = false
anonymize_shared_data = true
```

### Data Retention

- Short-term: Current session only
- Long-term: Until you delete
- No automatic cloud backup
- No external transmission (unless P2P enabled)

---

## User Rights

### Right to Know

You can:
- View all stored data
- See what's being collected
- Understand data usage
- Know who has access

### Right to Access

Export your data anytime:
```bash
# Via CLI
ozone-studio export --all --output backup.zip

# Via UI
Settings → Privacy → Export Data
```

### Right to Delete

Delete data selectively or completely:
```bash
# Delete consciousness data
ozone-studio delete --consciousness

# Delete all data
ozone-studio delete --all
```

### Right to Portability

Data exports in standard formats:
- JSON for structured data
- Original format for files
- TOML for configuration

### Right to Rectification

Correct inaccurate data:
- Edit preferences
- Correct memories
- Update information

---

## Data Controls

### Configuration Options

```toml
[privacy]
# Data collection
collect_usage_stats = false
collect_error_reports = false

# Data retention
auto_delete_logs_days = 30
auto_delete_tasks_days = 90

# Data sharing
allow_anonymous_telemetry = false
```

### UI Controls

Settings → Privacy:
- [ ] Collect anonymous usage statistics
- [ ] Send error reports
- [ ] Participate in collective learning
- [Data Retention] slider
- [Export All Data] button
- [Delete All Data] button

### Per-Workspace Privacy

Each workspace can have privacy settings:
- Private (no sharing)
- Team (shared with specified users)
- Public (shared methodologies)

---

## Security Measures

### Authentication

- Local public key authentication
- Session tokens expire
- Device registration required
- No external auth providers

### Encryption

- AES-256-GCM at rest
- TLS 1.3 in transit
- End-to-end for P2P
- Key derivation via Argon2id

### Access Control

- File permissions restricted
- Process isolation
- Memory protection
- Secure credential storage

### Audit Logging

Optional audit trail:
```toml
[logging]
audit_enabled = true
audit_path = "audit.log"
```

---

## Compliance

### GDPR Alignment

OZONE supports GDPR principles:
- ✓ Lawful processing (user consent)
- ✓ Purpose limitation (documented uses)
- ✓ Data minimization (collect only needed)
- ✓ Accuracy (user can correct)
- ✓ Storage limitation (retention controls)
- ✓ Security (encryption, access control)
- ✓ Accountability (audit logs)

### CCPA Alignment

California Consumer Privacy Act support:
- ✓ Right to know
- ✓ Right to delete
- ✓ Right to opt-out
- ✓ Right to non-discrimination

### Data Processing

All processing happens:
- On your device (default)
- Via your chosen LLM (API or local)
- On P2P network (if enabled, encrypted)

No processing on our servers - there are no "our servers."

---

## FAQ

### Where is my data stored?

Locally on your device, in the configured data directory. Nothing goes to external servers unless you enable P2P sharing.

### Do you sell my data?

No. There's no data to sell - everything stays on your device.

### Can you access my data?

No. We have no access to your local data. This is a self-hosted application.

### What if I use an API model?

Prompts are sent to the API provider (e.g., Anthropic, OpenAI). Review their privacy policies. Consider using local models for sensitive work.

### How do I completely remove my data?

Delete the data directory, or use:
```bash
ozone-studio delete --all --confirm
```

### Is P2P network traffic monitored?

No. P2P communication is end-to-end encrypted. We cannot see the content.

### What about API keys?

Stored locally in secure storage. Never transmitted except to the API provider.

---

## Contact

For privacy concerns:
- GitHub Issues (for general questions)
- security@ozonestudio.xyz (for security issues)

---

*OZONE STUDIO v0.4.0 - Privacy Documentation*
