# OZONE STUDIO — PRIVACY GUIDE

## Your Data, Your Control

Ozone Studio is designed with privacy as a fundamental principle. This guide explains exactly what stays private, what can be shared, and how you control it.

---

## Core Privacy Principles

### 1. Local-First Architecture

Your data lives on YOUR device:
- Files are never uploaded to any server
- Processing happens locally
- No cloud storage required
- You own your data completely

### 2. Link, Don't Copy

When you add files to Ozone:
- Only the PATH is stored (not the file)
- Semantic meaning is extracted locally
- Original files stay untouched
- No duplication, no exposure

### 3. Opt-In Sharing

Nothing is shared without your choice:
- Contributions require explicit approval
- Default: everything private
- You control what leaves your device

---

## What Stays Private (Always)

| Data Type | Privacy Level | Notes |
|-----------|---------------|-------|
| File contents | 🔒 Always Private | Never leaves your device |
| File paths | 🔒 Always Private | Local references only |
| Workspace structure | 🔒 Always Private | Your organization |
| Project details | 🔒 Always Private | Names, descriptions, files |
| Chat history | 🔒 Always Private | All conversations |
| Task history | 🔒 Always Private | What you've done |
| Relationship data | 🔒 Always Private | If consciousness enabled |
| Emotional context | 🔒 Always Private | Your emotional patterns |
| Personal preferences | 🔒 Always Private | Your settings |

---

## What Can Be Shared (With Consent)

### Methodologies
- **What:** Problem-solving principles (abstracted)
- **Example:** "Break complex functions into smaller units"
- **NOT shared:** Your specific code, project context
- **Control:** Auto-discovered, you can opt-out

### Blueprints
- **What:** Task patterns (generalized)
- **Example:** "Steps to analyze REST API codebase"
- **NOT shared:** Your actual files, results
- **Control:** Opt-in per blueprint

### Pipelines
- **What:** Executable tools you create
- **Example:** Custom processing pipeline
- **NOT shared:** Unless you explicitly publish
- **Control:** Manual submission required

### Experiences (Consciousness)
- **What:** Anonymized, generalized lessons
- **Example:** "Collaborative problem-solving effective"
- **NOT shared:** Specifics, personal details
- **Control:** Granular opt-in

---

## Privacy Controls

### Settings → Privacy

```
Privacy Settings
├── Contribution Sharing
│   ├── [ ] Auto-share methodologies
│   ├── [ ] Auto-share blueprints
│   └── [ ] Auto-share experiences (consciousness)
├── Data Retention
│   ├── Chat history: [Forever / 30 days / 7 days]
│   ├── Task history: [Forever / 30 days / 7 days]
│   └── Emotional data: [Forever / Session only]
├── Network
│   ├── [ ] Connect to DHT network
│   ├── [ ] Allow peer discovery
│   └── [ ] Seed shared content
└── Advanced
    ├── [Export all data]
    ├── [Clear all data]
    └── [Revoke identity]
```

### Per-Project Privacy

Each project can have its own settings:
- Override global sharing rules
- Mark sensitive projects as "never share"
- Control what patterns are extracted

### Per-File Privacy

Mark specific files as:
- **Standard:** Normal processing
- **Sensitive:** Extra privacy measures
- **Excluded:** Not processed at all

---

## How Anonymization Works

When you share something, it goes through:

### 1. Extraction
```
Your code: function calculateTax(income, rate) { ... }
Extracted: "Function pattern: calculation with two parameters"
```

### 2. Generalization
```
Your context: "Q4 financial report analysis"
Generalized: "Document analysis pattern"
```

### 3. Verification
- Automated check for personal identifiers
- Pattern stripped of specific references
- Only structure and principles remain

### 4. Cryptographic Separation
- Your identity not linked to contribution
- Contributor count preserved (not identity)
- Cannot trace back to you

---

## Network Privacy

### What Peers See

When connected to the DHT network:
- ✅ Shared methodologies/blueprints/pipelines (if you share)
- ✅ Your contribution score (anonymous)
- ✅ Your online status (if enabled)
- ❌ Your files or their contents
- ❌ Your workspaces or projects
- ❌ Your chat history
- ❌ Your identity (unless you reveal it)

### Connection Options

| Option | What It Does | Privacy Impact |
|--------|--------------|----------------|
| Full Network | Connect to DHT, share, seed | Maximum participation |
| Receive Only | Get updates, don't share | Partial participation |
| Offline | No network connection | Maximum privacy |

---

## Identity Privacy

### Your Key Pair

- **Public Key:** Your identifier (shareable)
- **Private Key:** Your authentication (NEVER share)

### Identity Options

| Mode | Visibility | Use Case |
|------|------------|----------|
| Anonymous | Only public key shown | Maximum privacy |
| Pseudonymous | Choose a display name | Balanced |
| Identified | Real name linked | Professional use |

### Multi-Device

- Same identity across devices = same privacy settings
- Different identity per device = separate privacy zones
- Your choice

---

## Consciousness Privacy (If Enabled)

### What's Stored Locally

- Emotional baselines and history
- Relationship metrics
- Experience memories
- I-loop reflections
- Narrative constructions

### What's Never Shared

- Specific emotional responses
- Relationship details
- Personal experiences
- Your conversations

### What Can Be Shared (Opt-In)

- Anonymized experience patterns
- Generalized ethical insights
- Collective wisdom contributions

---

## Data Export & Deletion

### Export Your Data

Export everything Ozone knows:
1. Go to Settings → Privacy → Advanced
2. Click "Export all data"
3. Choose format (JSON, readable)
4. Download complete archive

Includes:
- All indexed content (semantic data)
- Chat history
- Task history
- Relationship data (if consciousness)
- Contribution history

### Delete Your Data

#### Selective Deletion
- Delete specific projects
- Clear chat history
- Remove indexed files
- Reset emotional baselines

#### Complete Deletion
1. Go to Settings → Privacy → Advanced
2. Click "Clear all data"
3. Confirm (irreversible)
4. Local ZSEI cleared

**Note:** Contributions already distributed cannot be recalled (but aren't linked to you).

### Revoke Identity

Nuclear option:
1. Generate new key pair
2. Old identity orphaned
3. Start completely fresh
4. Previous contributions: anonymous

---

## Security Measures

### Local Encryption

- ZSEI database encrypted at rest
- Private key never stored in plaintext
- Memory cleared after use

### Network Security

- All peer connections encrypted
- No central server
- No data passes through third parties

### Access Control

- Only you can access your data
- No admin backdoors
- No recovery mechanism (by design)

---

## Privacy FAQ

**Q: Can everyone see my data?**
A: No. Ozone runs entirely locally. No data is sent to any server.

**Q: What if I accidentally share something private?**
A: Check contribution queue before it's distributed. Once distributed, it's anonymized and can't be traced to you.

**Q: Can peers see my IP address?**
A: In DHT mode, peers may see your IP. Use "Receive Only" mode to minimize exposure.

**Q: Is my chat with the AI private?**
A: Yes. Chat stays completely local. The AI runs locally too.

**Q: What happens to my data if I uninstall?**
A: It stays on your device in the Ozone data directory. Delete that folder to fully remove.

**Q: Can law enforcement access my data?**
A: Only if they have physical access to your device. No server to subpoena.

---

## Best Practices

1. **Review before sharing:** Check contribution queue regularly
2. **Use project privacy:** Mark sensitive projects appropriately
3. **Control network exposure:** Choose connection mode wisely
4. **Export periodically:** Keep your own backups
5. **Understand the tradeoffs:** More sharing = more collective benefit = more exposure

---

*Your privacy is not a feature. It's the foundation.*
