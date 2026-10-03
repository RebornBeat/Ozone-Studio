// Device identity — the mobile app's own Ed25519 challenge-response auth,
// mirroring the EXACT real protocol ui/electron/main.js's ensureSessionToken
// already proves live (confirmed by reading it directly, not guessed):
// a persisted device keypair, POST /auth/challenge with the raw 32-byte
// public key (hex), sign the returned challenge bytes with plain Ed25519
// (no prehash), POST /auth/authenticate with {public_key, signature}.
//
// React Native has no Node `crypto` module, so this uses @noble/ed25519
// (audited, zero dependencies, pure JS) instead of Node's native crypto —
// the real, load-bearing difference from the desktop/CLI clients, which
// both use Node's crypto directly. The wire protocol is identical either
// way: raw 32-byte public key, raw 64-byte signature, both hex-encoded.
//
// react-native-get-random-values MUST be imported before this module (see
// src/app/_layout.tsx) — @noble/ed25519 needs a real crypto.getRandomValues
// source for key generation, which Hermes doesn't provide natively.
import * as ed from "@noble/ed25519";
import * as SecureStore from "expo-secure-store";

const SECRET_KEY_STORE_KEY = "ozone_device_secret_key_hex";

let cachedSecretKey: Uint8Array | null = null;
let cachedSessionToken: string | null = null;

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return out;
}

/** Load the persisted device secret key, or generate + persist a new real
 * one on first run. One real device identity, reused across app launches —
 * never a throwaway key per session (same discipline as the CLI/desktop
 * clients, both confirmed to persist their own keys). */
async function ensureSecretKey(): Promise<Uint8Array> {
  if (cachedSecretKey) return cachedSecretKey;
  const stored = await SecureStore.getItemAsync(SECRET_KEY_STORE_KEY);
  if (stored) {
    cachedSecretKey = hexToBytes(stored);
    return cachedSecretKey;
  }
  const fresh = ed.utils.randomSecretKey();
  await SecureStore.setItemAsync(SECRET_KEY_STORE_KEY, bytesToHex(fresh));
  cachedSecretKey = fresh;
  return fresh;
}

export interface OzoneHostConfig {
  baseUrl: string; // e.g. "http://192.168.1.42:50051" (LAN) or a real remote URL
}

async function rawRequest(baseUrl: string, path: string, body: unknown): Promise<any> {
  const res = await fetch(`${baseUrl}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return res.json();
}

/** Real challenge-response authentication against the host — returns a
 * real session token, cached in-memory for the process lifetime (same
 * convention as the Electron client's cachedSessionToken). */
export async function ensureSessionToken(host: OzoneHostConfig): Promise<string> {
  if (cachedSessionToken) return cachedSessionToken;
  const secretKey = await ensureSecretKey();
  const publicKey = ed.getPublicKey(secretKey);
  const rawPub = bytesToHex(publicKey);

  const ch = await rawRequest(host.baseUrl, "/auth/challenge", { public_key: rawPub });
  if (!ch?.challenge) throw new Error("no challenge from host — check the host URL and that Ozone-Studio is running");
  const challengeBytes = hexToBytes(ch.challenge);
  const signature = ed.sign(challengeBytes, secretKey);
  const signatureHex = bytesToHex(signature);

  const auth = await rawRequest(host.baseUrl, "/auth/authenticate", {
    public_key: rawPub,
    signature: signatureHex,
  });
  if (!auth?.success || !auth?.session_token) {
    throw new Error(`host auth failed: ${auth?.error ?? "unknown"}`);
  }
  const token: string = auth.session_token;
  cachedSessionToken = token;
  return token;
}

/** Clear the cached session token (forces re-authentication on the next
 * call) — used when a call comes back rejected, same pattern the Electron
 * client uses. Never clears the persisted device key itself. */
export function clearSessionToken(): void {
  cachedSessionToken = null;
}
