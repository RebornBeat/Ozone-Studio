// Connect screen — real host URL entry + a real /health check before
// saving, so a bad URL never gets silently persisted. LAN (e.g.
// http://192.168.1.42:50051) or a real remote URL both work — the same
// host, no separate remote-specific code path, per the operator's own
// "both" answer earlier this session.
import { useEffect, useState, useCallback } from "react";
import { View, Text, TextInput, Pressable, StyleSheet, ActivityIndicator } from "react-native";
import { getSavedHostUrl, saveHostUrl } from "../../lib/hostConfig";
import { checkHealth } from "../../lib/ozoneMobileClient";

export default function ConnectScreen() {
  const [url, setUrl] = useState("");
  const [status, setStatus] = useState<"idle" | "checking" | "ok" | "error">("idle");
  const [statusDetail, setStatusDetail] = useState("");

  useEffect(() => {
    getSavedHostUrl().then((saved) => {
      if (saved) setUrl(saved);
    });
  }, []);

  const testAndSave = useCallback(async () => {
    if (!url.trim()) return;
    setStatus("checking");
    setStatusDetail("");
    try {
      const health = await checkHealth({ baseUrl: url.trim().replace(/\/+$/, "") });
      if (health.healthy) {
        await saveHostUrl(url);
        setStatus("ok");
        setStatusDetail(`Ozone-Studio ${health.version ?? "?"} — connected`);
      } else {
        setStatus("error");
        setStatusDetail("Host responded but reported unhealthy");
      }
    } catch (e) {
      setStatus("error");
      setStatusDetail(e instanceof Error ? e.message : String(e));
    }
  }, [url]);

  return (
    <View style={styles.container}>
      <Text style={styles.label}>Ozone-Studio host</Text>
      <Text style={styles.hint}>LAN: http://&lt;desktop-ip&gt;:50051 — Remote: a real reachable URL</Text>
      <TextInput
        style={styles.input}
        value={url}
        onChangeText={setUrl}
        placeholder="http://192.168.1.42:50051"
        autoCapitalize="none"
        autoCorrect={false}
        keyboardType="url"
      />
      <Pressable style={styles.button} onPress={testAndSave} disabled={status === "checking"}>
        {status === "checking" ? (
          <ActivityIndicator color="#fff" />
        ) : (
          <Text style={styles.buttonText}>Test & Save</Text>
        )}
      </Pressable>
      {status === "ok" && <Text style={styles.ok}>{statusDetail}</Text>}
      {status === "error" && <Text style={styles.error}>{statusDetail}</Text>}
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, padding: 20, gap: 12 },
  label: { fontSize: 16, fontWeight: "600" },
  hint: { fontSize: 12, color: "#666", marginBottom: 8 },
  input: { borderWidth: 1, borderColor: "#ccc", borderRadius: 8, padding: 12, fontSize: 15 },
  button: { backgroundColor: "#2563eb", borderRadius: 8, padding: 14, alignItems: "center" },
  buttonText: { color: "#fff", fontWeight: "600" },
  ok: { color: "#16a34a" },
  error: { color: "#dc2626" },
});
