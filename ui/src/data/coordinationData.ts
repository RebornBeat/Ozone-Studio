/** Live presence + file claims (B10/B11), read straight from .ozone-context/state.json by the host. */
import { getJson } from "./http";

export interface PresenceEntry {
  agent: string;
  role: string;
  current_files: string[];
  task: string;
  last_seen_age_s: number;
}
export interface PresenceResponse {
  live: PresenceEntry[];
  ttl_seconds: number;
}
export interface FileClaim {
  file: string;
  agent: string | null;
  reason: string;
  age_min: number;
}
export interface ClaimsResponse {
  claims: FileClaim[];
}

export const fetchPresence = (): Promise<PresenceResponse> => getJson("/coordination/presence");
export const fetchClaims = (): Promise<ClaimsResponse> => getJson("/coordination/claims");
