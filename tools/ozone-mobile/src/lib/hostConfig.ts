// Real, persisted host connection config — LAN or remote (both real,
// confirmed-available options per the operator's own choice earlier this
// session: "both LAN + remote"). Not secret (a URL, not a key), so plain
// AsyncStorage-equivalent (SecureStore works fine for small non-sensitive
// strings too — reusing it avoids a second storage dependency).
import * as SecureStore from "expo-secure-store";

const HOST_URL_KEY = "ozone_host_base_url";

export async function getSavedHostUrl(): Promise<string | null> {
  return SecureStore.getItemAsync(HOST_URL_KEY);
}

export async function saveHostUrl(url: string): Promise<void> {
  // Real, minimal normalization — strip a trailing slash so path joins
  // downstream never produce "//health" etc.
  const normalized = url.trim().replace(/\/+$/, "");
  await SecureStore.setItemAsync(HOST_URL_KEY, normalized);
}
