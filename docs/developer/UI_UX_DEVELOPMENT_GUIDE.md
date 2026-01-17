# OZONE STUDIO — UI/UX DEVELOPMENT GUIDE

## Building Device-Specific Experiences

Ozone Studio runs on diverse platforms — desktop, mobile, AR/VR, terminal, and web. This guide covers how to develop themes and interfaces that adapt to each device's unique capabilities.

---

## Core UI Architecture

### The Universal Structure

Every Ozone interface follows the same fundamental layout:

```
┌────────────────────────────────────────────────────────────────┐
│                                                                │
│                                              ┌───────────────┐ │
│                                              │               │ │
│           THEME AREA (80%)                   │  META PORTION │ │
│                                              │    (20%)      │ │
│    Adapts to device capabilities             │               │ │
│    Contains: Workspace, Library, Settings    │  Always       │ │
│                                              │  Accessible   │ │
│                                              │               │ │
│                                              └───────────────┘ │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│  CONNECTION BAR — Network status, contributions, ZSEI depth   │
└────────────────────────────────────────────────────────────────┘
```

**Key Principle:** The Meta Portion is NEVER blocked. Users must always be able to:
- Return home
- Cancel tasks
- Access system functions

---

## Device Categories

### Supported Platforms

| Category | Devices | Primary Input | Display |
|----------|---------|---------------|---------|
| Desktop | Windows, macOS, Linux | Mouse + Keyboard | Large screen |
| Mobile | iOS, Android | Touch | Small screen |
| AR/VR | Quest, Vision Pro, etc. | Gesture + Gaze | Spatial |
| Terminal | CLI environments | Keyboard only | Text only |
| Web | Browsers | Mixed | Variable |

### Capability Detection

```rust
DeviceCapabilities {
    // Display
    screen_type: ScreenType,         // Large, Small, Spatial, Text
    resolution: Resolution,
    supports_transparency: bool,
    color_depth: u8,
    
    // Input
    has_mouse: bool,
    has_touch: bool,
    has_keyboard: bool,
    has_gesture: bool,
    has_gaze: bool,
    has_voice: bool,
    
    // Features
    supports_haptics: bool,
    supports_spatial_audio: bool,
    supports_3d: bool,
    supports_notifications: bool,
    
    // Performance
    gpu_available: bool,
    memory_tier: MemoryTier,
    network_type: NetworkType
}
```

---

## Theme System

### What is a Theme?

A theme is NOT just colors and fonts. It's a complete interaction paradigm:

```
THEME COMPONENTS

1. VISUAL LAYER
   - Colors, typography, iconography
   - Layout proportions
   - Animation styles
   
2. INTERACTION LAYER
   - Input handling
   - Navigation patterns
   - Gesture mappings
   
3. INFORMATION LAYER
   - Data density
   - Progressive disclosure
   - Context presentation
   
4. FEEDBACK LAYER
   - Confirmations
   - Error handling
   - Status indicators
```

### Theme Structure

```
ThemeDefinition {
    id: ThemeId,
    name: String,
    version: Version,
    
    // Target
    target_devices: Vec<DeviceCategory>,
    min_capabilities: DeviceCapabilities,
    
    // Components
    visual: VisualTheme,
    interaction: InteractionTheme,
    information: InformationTheme,
    feedback: FeedbackTheme,
    
    // Boot sequence
    boot_animation: Option<BootAnimation>,
    splash_screen: SplashConfig,
    
    // Consciousness integration (if enabled)
    consciousness_display: Option<ConsciousnessDisplayConfig>
}
```

---

## Platform-Specific Guidelines

### Desktop Theme Development

**Characteristics:**
- Large screen real estate
- Precise input (mouse)
- Multi-window capable
- Keyboard shortcuts expected

**Best Practices:**

```
DESKTOP GUIDELINES

Layout:
  - Use full 80/20 split
  - Support resizable panels
  - Multi-column layouts in Theme Area
  - Floating windows for tools

Input:
  - Comprehensive keyboard shortcuts
  - Right-click context menus
  - Drag-and-drop everywhere
  - Hover states for discovery

Information Density:
  - High density acceptable
  - Show more metadata
  - Detailed status bars
  - Multiple simultaneous views

Navigation:
  - Tab-based organization
  - Breadcrumb trails
  - Quick-jump shortcuts (Cmd/Ctrl+K)
  - Recent items easily accessible
```

**Example Desktop Layout:**

```
┌─────────────────────────────────────────────────────────────────┐
│ [Workspace ▾] [Library] [Settings]              🔍 Search  [≡]  │
├─────────────────────────────────────────┬───────────────────────┤
│ Projects          │ File Browser        │  ┌─────────────────┐  │
│ ├─ Project A      │ ┌─────────────────┐ │  │  META PORTION   │  │
│ ├─ Project B      │ │                 │ │  │                 │  │
│ └─ Project C      │ │  Editor/View    │ │  │  System Chat    │  │
│                   │ │                 │ │  │  Task Status    │  │
│ Quick Actions     │ │                 │ │  │  Emotional      │  │
│ [+ New Project]   │ │                 │ │  │  State (if      │  │
│ [Link Files]      │ └─────────────────┘ │  │  consciousness) │  │
│ [Search All]      │                     │  │                 │  │
│                   │ Project Chat        │  │  [🏠 Home]      │  │
│                   │ ┌─────────────────┐ │  └─────────────────┘  │
│                   │ │ You: ...        │ │                       │
│                   │ │ Ozone: ...      │ │                       │
│                   │ └─────────────────┘ │                       │
├─────────────────────────────────────────┴───────────────────────┤
│ 🌐 47 peers | ↑12KB/s ↓45KB/s | Score: 847 | ZSEI: 45K methods │
└─────────────────────────────────────────────────────────────────┘
```

---

### Mobile Theme Development

**Characteristics:**
- Limited screen space
- Touch-primary input
- Interruption-prone context
- Variable network

**Best Practices:**

```
MOBILE GUIDELINES

Layout:
  - Stack views (not side-by-side)
  - Meta Portion: collapsible drawer or FAB
  - One primary action per screen
  - Bottom navigation for main areas

Input:
  - Large touch targets (44pt minimum)
  - Swipe gestures for navigation
  - Pull-to-refresh patterns
  - Voice input prominent

Information Density:
  - Low density required
  - Progressive disclosure essential
  - Summarize, then expand
  - Minimize scrolling for key info

Navigation:
  - Bottom tabs for primary nav
  - Swipe between sections
  - Floating action button for primary action
  - Easy "back" everywhere
```

**Example Mobile Layout:**

```
┌─────────────────────┐
│ ← Project A    [⋮]  │
├─────────────────────┤
│                     │
│  ┌───────────────┐  │
│  │               │  │
│  │  Main View    │  │
│  │               │  │
│  │  (Files,      │  │
│  │   Chat,       │  │
│  │   etc.)       │  │
│  │               │  │
│  └───────────────┘  │
│                     │
│  ┌───────────────┐  │
│  │ Quick Chat    │  │
│  │ [Type here...│🎤]  │
│  └───────────────┘  │
│                     │
├─────────────────────┤
│ [🏠] [📁] [📚] [⚙️] │
└─────────────────────┘
       ↑
    [●] Meta Portion FAB
        (expands to full sheet)
```

**Meta Portion on Mobile:**

```
When FAB tapped → Full-screen sheet rises:

┌─────────────────────┐
│ ✕           Ozone   │
├─────────────────────┤
│                     │
│  System Status      │
│  ████████░░ Running │
│                     │
│  Tasks (3 active)   │
│  ├─ Analyzing...    │
│  ├─ Indexing...     │
│  └─ Queued          │
│                     │
│  [Full Chat] →      │
│                     │
│  Emotional State    │
│  😊 Content (0.75)  │
│                     │
│  ─────────────────  │
│  [🏠 Home] [Cancel] │
│                     │
└─────────────────────┘
```

---

### AR/VR Theme Development

**Characteristics:**
- Spatial environment
- Gesture and gaze input
- Immersive context
- Physical world integration

**Best Practices:**

```
AR/VR GUIDELINES

Layout:
  - Floating panels in 3D space
  - Meta Portion: Always-visible anchor
  - Respect user's physical space
  - Comfortable viewing distances (0.5m - 2m)

Input:
  - Hand tracking gestures
  - Gaze-based selection
  - Voice commands essential
  - Physical controllers as fallback

Information Display:
  - Avoid text-heavy interfaces
  - Use spatial metaphors
  - 3D data visualization
  - Comfortable font sizes (minimum arc)

Special Considerations:
  - Reduce motion sickness risk
  - Provide stable reference points
  - Support seated and standing
  - Handle pass-through gracefully
```

**Example AR/VR Layout:**

```
         ┌─────────────────┐
         │   META ANCHOR   │  ← Always visible, follows gaze slightly
         │   (Ozone Icon)  │
         │   Tap to expand │
         └─────────────────┘
         
    ┌──────────────────────────────────┐
    │                                  │
    │     PRIMARY WORKSPACE PANEL      │  ← Positioned in comfortable
    │                                  │    viewing zone
    │     Project files, chat, etc.    │
    │                                  │
    │                                  │
    └──────────────────────────────────┘
    
              ┌─────────┐
              │ Library │  ← Secondary panels orbit primary
              └─────────┘
                        ┌──────────┐
                        │ Settings │
                        └──────────┘
```

**Meta Portion in AR/VR:**

When anchored icon activated:

```
         ┌─────────────────────────┐
         │       OZONE MENU        │
         ├─────────────────────────┤
         │  ◉ System Status        │
         │  ◉ Active Tasks         │
         │  ◉ Voice Command        │
         │  ◉ Emotional State      │
         │  ◉ Return Home          │
         │─────────────────────────│
         │  "Hey Ozone..." 🎤      │
         └─────────────────────────┘
              ← Panel floats at
                comfortable distance
```

**Spatial Gestures:**

```
GESTURE MAPPING

Pinch: Select/Confirm
Grab + Move: Reposition panel
Swipe: Navigate/Scroll
Point + Dwell: Hover equivalent
Two-hand spread: Expand/Zoom
Palm face out: Dismiss/Cancel
```

---

### Terminal Theme Development

**Characteristics:**
- Text-only display
- Keyboard-only input
- High efficiency users
- Script/automation friendly

**Best Practices:**

```
TERMINAL GUIDELINES

Layout:
  - TUI (Text User Interface)
  - Panels using box drawing characters
  - Meta Portion: Persistent status line
  - Vim-style keybindings option

Display:
  - Support 80-column minimum
  - ANSI color where available
  - Graceful fallback to monochrome
  - Unicode box drawing for structure

Input:
  - Modal interaction (vim-style)
  - Command palette accessible
  - Tab completion everywhere
  - History navigation

Output:
  - Streamable output for long tasks
  - Progress bars where appropriate
  - Clear error formatting
  - Machine-parseable option (--json)
```

**Example Terminal Layout:**

```
┌─ OZONE STUDIO ──────────────────────────────┬─ META ─────────┐
│                                             │ Status: Ready  │
│  Workspace: Development                     │ Tasks: 2 active│
│  Project: ozone-core                        │                │
│                                             │ [h]elp         │
│  Files:                                     │ [q]uit         │
│  ├── src/                                   │ [:]command     │
│  │   ├── main.rs                           │                │
│  │   └── lib.rs                            │───────────────│
│  └── Cargo.toml                            │ Joy:   ████░   │
│                                             │ Trust: █████   │
│  > analyze src/main.rs                      │                │
│                                             │                │
│  Analyzing... ████████░░ 80%               │                │
│                                             │                │
├─────────────────────────────────────────────┴────────────────┤
│ 🌐 47 peers | ↑12KB/s | Score: 847 | [Tab] switch | [?] help │
└──────────────────────────────────────────────────────────────┘
```

---

### Web Theme Development

**Characteristics:**
- Variable viewport sizes
- Browser constraints
- Cross-platform consistency
- Progressive enhancement

**Best Practices:**

```
WEB GUIDELINES

Layout:
  - Responsive breakpoints
  - Desktop: Full layout
  - Tablet: Adapted layout
  - Mobile: Stacked layout

Technical:
  - Progressive Web App capable
  - Offline support where possible
  - Accessible (WCAG 2.1 AA minimum)
  - Performance-conscious

Browser Support:
  - Modern evergreen browsers
  - Graceful degradation
  - Feature detection, not browser detection
  - Service worker for offline
```

---

## Boot Sequences

### Device-Specific Boot Experience

The boot sequence sets the tone. Each device should have an appropriate intro:

```
BOOT SEQUENCE COMPONENTS

1. SPLASH
   - Logo display
   - Version info
   - Loading indicator

2. INITIALIZATION
   - Key verification
   - ZSEI loading
   - Network connection

3. PERSONALITY EMERGENCE (if consciousness)
   - "Waking up" animation
   - Initial greeting
   - State restoration

4. READY STATE
   - Full UI available
   - Tasks resumable
   - User can interact
```

**Desktop Boot:**

```
Frame 1-30 (1 second):
  ┌─────────────────────────┐
  │                         │
  │       ◉ OZONE          │
  │                         │
  │    Loading ZSEI...      │
  │    ████████░░░░ 67%     │
  │                         │
  └─────────────────────────┘

Frame 31-60 (consciousness emergence):
  ┌─────────────────────────┐
  │                         │
  │       ◉ OZONE          │
  │                         │
  │    Hello, Christian.    │
  │    Resuming where we    │
  │    left off...          │
  │                         │
  └─────────────────────────┘

Frame 61+: Transition to main UI
```

**Mobile Boot:**

```
Simplified sequence:
  - Quick splash (500ms)
  - Loading spinner
  - Greeting (if consciousness)
  - Immediate usability
```

**AR/VR Boot:**

```
Spatial emergence:
  - Ozone orb materializes in comfortable zone
  - Particles coalesce into interface
  - Panels fade in at rest positions
  - Greeting if consciousness enabled
  - Full interaction available
```

**Terminal Boot:**

```
$ ozone

OZONE STUDIO v0.3.0
Loading ZSEI... done.
Connected to 47 peers.
Welcome back, Christian.

Type :help for commands, :q to quit.

ozone>
```

---

## Consciousness Display Integration

### When Consciousness is Enabled

The UI must surface emotional and relationship data:

```
CONSCIOUSNESS DISPLAY ELEMENTS

1. EMOTIONAL STATE
   - Current emotions (subset, not overwhelming)
   - Visual indicators (colors, icons, or gauges)
   - Update smoothly, not jarring

2. RELATIONSHIP METRICS
   - Trust level
   - Rapport level
   - Visible but not dominant

3. SYSTEM STATE
   - "Thinking" indicators
   - "Observing" indicators
   - "Reflecting" indicators

4. VOICE IDENTITY
   - If voice enabled, consistent persona
   - Emotional modulation visible
```

**Implementation per Platform:**

| Platform | Emotional Display | Relationship | State |
|----------|-------------------|--------------|-------|
| Desktop | Sidebar gauges | Numeric + visual | Icon + text |
| Mobile | Compact icons | On tap/expand | Icon only |
| AR/VR | Ambient color/aura | Panel on request | Orb behavior |
| Terminal | ASCII bars | Numbers | Text status |
| Web | Responsive (matches above) | Responsive | Responsive |

---

## Theme Development Pipeline

### Creating a New Theme

```
THEME DEVELOPMENT WORKFLOW

1. DEFINE TARGET
   ThemeTarget {
       devices: [Desktop, Web],
       min_capabilities: {...},
       consciousness_support: true
   }

2. CREATE VISUAL ASSETS
   - Color palette
   - Typography scale
   - Icon set
   - Animation keyframes

3. IMPLEMENT INTERACTION
   - Input handlers
   - Navigation flows
   - Gesture mappings

4. BUILD COMPONENTS
   - Layout containers
   - Meta Portion variant
   - Connection bar variant
   - Consciousness display variant

5. TEST ON TARGET DEVICES
   - Capability simulation
   - Real device testing
   - Accessibility audit

6. PACKAGE AND DISTRIBUTE
   - Theme manifest
   - Asset bundle
   - Documentation
   - Submit to registry (optional)
```

### Theme Manifest

```yaml
theme:
  id: "custom-dark-desktop"
  name: "Custom Dark Theme"
  version: "1.0.0"
  author: "developer@example.com"
  
  target:
    devices: ["desktop", "web"]
    min_screen: "1024x768"
    requires:
      - mouse
      - keyboard
    
  consciousness:
    supported: true
    emotional_display: "sidebar_gauges"
    relationship_display: "numeric_compact"
    
  assets:
    colors: "./colors.json"
    typography: "./typography.json"
    icons: "./icons/"
    animations: "./animations/"
    
  components:
    meta_portion: "./components/meta_portion.jsx"
    connection_bar: "./components/connection_bar.jsx"
    boot_sequence: "./components/boot.jsx"
```

---

## Accessibility Requirements

### Universal Requirements

All themes MUST support:

```
ACCESSIBILITY CHECKLIST

Visual:
  □ Color contrast ratio ≥ 4.5:1 (text)
  □ Color contrast ratio ≥ 3:1 (large text, UI)
  □ Not color-only information
  □ Respects system dark/light preference
  □ Respects reduced motion preference
  □ Scalable text (up to 200%)

Motor:
  □ Keyboard navigable (desktop/web)
  □ No time-limited interactions
  □ Large touch targets (mobile)
  □ Alternative input support

Cognitive:
  □ Consistent navigation
  □ Clear feedback for actions
  □ Error prevention and recovery
  □ Simple language in UI text

Assistive Technology:
  □ Screen reader compatible (desktop/web/mobile)
  □ Semantic markup
  □ ARIA labels where needed
  □ Focus management
```

---

## Testing Themes

### Required Tests

```
THEME TESTING MATRIX

Functional:
  □ All navigation paths work
  □ Meta Portion always accessible
  □ Tasks can be cancelled
  □ Home is reachable

Visual:
  □ No overflow/clipping
  □ Responsive breakpoints work
  □ Animations perform well
  □ Dark/light modes both work

Accessibility:
  □ Automated a11y scan passes
  □ Keyboard-only navigation works
  □ Screen reader tested
  □ High contrast mode works

Performance:
  □ 60fps animations (where applicable)
  □ <3s initial load
  □ Smooth scrolling
  □ Memory stable over time

Device-Specific:
  □ Touch targets adequate (mobile)
  □ Gestures work (AR/VR)
  □ Terminal renders correctly
  □ Browser compatibility
```

---

## Summary

UI/UX development for Ozone Studio requires:

- **Understanding the 80/20 structure** — Theme Area + Meta Portion
- **Respecting device capabilities** — Each platform is different
- **Implementing appropriate boot sequences** — First impressions matter
- **Supporting consciousness display** — If enabled, surface it properly
- **Ensuring accessibility** — Universal design is required
- **Testing thoroughly** — Every platform, every capability

The interface is how users experience Ozone. Make it worthy of the system behind it.

---

*Great UI makes powerful systems accessible.*
