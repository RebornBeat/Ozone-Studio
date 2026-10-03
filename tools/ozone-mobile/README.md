# ozone-mobile

A real Expo/React Native app connecting to a running Ozone-Studio host —
built 2026-09-30, per the operator's earlier directive ("both LAN and
remote" access, React Native).

## What's real here

- **Device identity** (`src/lib/deviceIdentity.ts`) — a real, persisted
  Ed25519 keypair (via `expo-secure-store`), authenticating against the
  host's real `/auth/challenge` + `/auth/authenticate` endpoints. Uses
  `@noble/ed25519` (pure JS, audited, zero deps) since React Native has
  no Node `crypto` module — the wire protocol is byte-for-byte identical
  to what the Electron desktop client and the `ozone-cli` already prove
  works (raw 32-byte public key hex, raw 64-byte signature hex, no
  prehash).
- **Connect screen** (`src/app/(tabs)/connect.tsx`) — enter a host URL
  (LAN `http://<ip>:50051` or a real remote URL), tests it against
  `/health` before persisting.
- **Order screen** (`src/app/(tabs)/index.tsx`) — the real Universal
  Order day view: reads `/order/global`, buckets by `due_at` using the
  EXACT same arithmetic `ui/src/components/OrderPanel.tsx` uses on
  desktop (read directly from that file, not re-derived), plus a
  quick-capture form posting through the real `/task/create` path.

## What's NOT verified — stated honestly

This was built and verified via `tsc --noEmit` (clean), `expo lint`
(clean), and `expo-doctor` (21/21 checks passed) — real, structural,
type-level correctness. **It has not been run on a real device or
simulator in this environment** (none available) — the actual
runtime behavior (does the Ed25519 auth flow genuinely round-trip
against a live host from a real phone, does the UI render correctly,
does `expo-secure-store` behave as expected on real iOS/Android) is
unverified. Before relying on this, run it for real:

```bash
cd tools/ozone-mobile
npm install
npx expo start
# scan the QR code with Expo Go, or:
npx expo run:ios     # needs a Mac
npx expo run:android
```

## Not built yet (real, stated scope)

- Chat/orchestrate screen.
- Push notifications for due items (would need a real background task /
  notification permission flow — separate scope).
- The Acting Loop's own findings surfaced on mobile — not yet wired up
  (would reuse the same `/order/global`-adjacent pattern once there's a
  real read endpoint for recent findings).
