// The Order screen — the real "personal assistant day view" on mobile.
// Bucket arithmetic mirrors ui/src/components/OrderPanel.tsx's own EXACT
// real overdue/today/week/upcoming/someday logic (read directly, not
// re-derived), so the mobile app buckets identically to the desktop app
// reading the SAME real /order/global data — no drift between the two.
import { useCallback, useEffect, useState } from "react";
import { View, Text, StyleSheet, FlatList, RefreshControl, Pressable, TextInput, ActivityIndicator, Alert } from "react-native";
import { useRouter } from "expo-router";
import { getSavedHostUrl } from "../../lib/hostConfig";
import { fetchGlobalOrder, createOrderItem, GlobalOrder, OrderItem, OrderItemKind } from "../../lib/ozoneMobileClient";

type Bucket = "overdue" | "today" | "week" | "upcoming" | "someday";
const BUCKET_ORDER: Bucket[] = ["overdue", "today", "week", "upcoming", "someday"];
const BUCKET_LABELS: Record<Bucket, string> = {
  overdue: "Overdue", today: "Today", week: "This Week", upcoming: "Upcoming", someday: "Someday",
};

function bucketOf(dueAt: number | null): Bucket {
  if (dueAt == null) return "someday";
  const now = Date.now() / 1000;
  const day = 86400;
  if (dueAt < now - day) return "overdue";
  if (dueAt <= now + day) return "today";
  if (dueAt <= now + 7 * day) return "week";
  return "upcoming";
}

function dueLabel(dueAt: number | null): string {
  if (dueAt == null) return "";
  return new Date(dueAt * 1000).toLocaleString([], { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

interface Row {
  bucket: Bucket;
  item?: OrderItem;
  isHeader?: boolean;
}

export default function OrderScreen() {
  const router = useRouter();
  const [hostUrl, setHostUrl] = useState<string | null>(null);
  const [order, setOrder] = useState<GlobalOrder | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [captureText, setCaptureText] = useState("");
  const [capturing, setCapturing] = useState(false);

  const load = useCallback(async (url: string) => {
    setError(null);
    try {
      const result = await fetchGlobalOrder({ baseUrl: url });
      setOrder(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    getSavedHostUrl().then((saved) => {
      if (!saved) {
        setLoading(false);
        return;
      }
      setHostUrl(saved);
      load(saved);
    });
  }, [load]);

  const onRefresh = useCallback(() => {
    if (hostUrl) {
      setLoading(true);
      load(hostUrl);
    }
  }, [hostUrl, load]);

  const onCapture = useCallback(async () => {
    if (!hostUrl || !captureText.trim()) return;
    setCapturing(true);
    try {
      const result = await createOrderItem({ baseUrl: hostUrl }, { name: captureText.trim(), kind: "todo" as OrderItemKind });
      if (result.success) {
        setCaptureText("");
        load(hostUrl);
      } else {
        Alert.alert("Capture failed", result.error ?? "unknown error");
      }
    } catch (e) {
      Alert.alert("Capture failed", e instanceof Error ? e.message : String(e));
    } finally {
      setCapturing(false);
    }
  }, [hostUrl, captureText, load]);

  if (!hostUrl) {
    return (
      <View style={styles.center}>
        <Text style={styles.emptyText}>No host connected yet.</Text>
        <Pressable style={styles.connectButton} onPress={() => router.push("/connect")}>
          <Text style={styles.connectButtonText}>Go to Connect</Text>
        </Pressable>
      </View>
    );
  }

  // Real bucketing, identical to the desktop OrderPanel's own logic —
  // active work (live/paused/queued/interrupted) shown separately from
  // the due-date buckets, matching the desktop app's own precedence rule.
  const buckets: Record<Bucket, OrderItem[]> = { overdue: [], today: [], week: [], upcoming: [], someday: [] };
  const active: OrderItem[] = [];
  if (order) {
    for (const state of ["live", "paused", "queued", "interrupted"] as const) {
      active.push(...(order[state] ?? []));
    }
    const activeIds = new Set(active.map((i) => i.task_id));
    for (const state of ["live", "paused", "queued", "interrupted", "other"] as const) {
      for (const item of order[state] ?? []) {
        if (activeIds.has(item.task_id) && state !== "other") continue;
        buckets[bucketOf(item.due_at)].push(item);
      }
    }
  }

  const rows: Row[] = [];
  for (const bucket of BUCKET_ORDER) {
    if (buckets[bucket].length === 0) continue;
    rows.push({ bucket, isHeader: true });
    for (const item of buckets[bucket]) rows.push({ bucket, item });
  }

  return (
    <View style={styles.container}>
      <View style={styles.captureRow}>
        <TextInput
          style={styles.captureInput}
          value={captureText}
          onChangeText={setCaptureText}
          placeholder="Quick capture — a todo, a note..."
          onSubmitEditing={onCapture}
        />
        <Pressable style={styles.captureButton} onPress={onCapture} disabled={capturing || !captureText.trim()}>
          {capturing ? <ActivityIndicator color="#fff" size="small" /> : <Text style={styles.captureButtonText}>Add</Text>}
        </Pressable>
      </View>

      {error && <Text style={styles.error}>{error}</Text>}

      {active.length > 0 && (
        <View style={styles.activeSection}>
          <Text style={styles.sectionHeader}>Active</Text>
          {active.map((item) => (
            <View key={item.task_id} style={styles.itemRow}>
              <Text style={styles.itemName} numberOfLines={1}>{item.name}</Text>
              <Text style={styles.itemMeta}>{item.status}{item.steps_total > 0 ? ` · ${item.steps_done}/${item.steps_total}` : ""}</Text>
            </View>
          ))}
        </View>
      )}

      <FlatList
        data={rows}
        keyExtractor={(row, i) => (row.isHeader ? `h-${row.bucket}` : `i-${row.item!.task_id}-${i}`)}
        refreshControl={<RefreshControl refreshing={loading} onRefresh={onRefresh} />}
        renderItem={({ item: row }) =>
          row.isHeader ? (
            <Text style={[styles.sectionHeader, row.bucket === "overdue" && styles.overdueHeader]}>
              {BUCKET_LABELS[row.bucket]}
            </Text>
          ) : (
            <View style={styles.itemRow}>
              <Text style={styles.itemName} numberOfLines={1}>{row.item!.name}</Text>
              {row.item!.due_at != null && <Text style={styles.itemMeta}>{dueLabel(row.item!.due_at)}</Text>}
            </View>
          )
        }
        ListEmptyComponent={!loading ? <Text style={styles.emptyText}>Nothing due — clear order.</Text> : null}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, padding: 16 },
  center: { flex: 1, alignItems: "center", justifyContent: "center", gap: 16, padding: 24 },
  captureRow: { flexDirection: "row", gap: 8, marginBottom: 12 },
  captureInput: { flex: 1, borderWidth: 1, borderColor: "#ccc", borderRadius: 8, padding: 10 },
  captureButton: { backgroundColor: "#2563eb", borderRadius: 8, paddingHorizontal: 16, justifyContent: "center" },
  captureButtonText: { color: "#fff", fontWeight: "600" },
  activeSection: { marginBottom: 12 },
  sectionHeader: { fontSize: 12, fontWeight: "700", color: "#666", textTransform: "uppercase", marginTop: 12, marginBottom: 4 },
  overdueHeader: { color: "#dc2626" },
  itemRow: { flexDirection: "row", justifyContent: "space-between", paddingVertical: 8, borderBottomWidth: StyleSheet.hairlineWidth, borderColor: "#eee" },
  itemName: { flex: 1, fontSize: 15 },
  itemMeta: { fontSize: 12, color: "#888", marginLeft: 8 },
  emptyText: { textAlign: "center", color: "#888", marginTop: 24 },
  error: { color: "#dc2626", marginBottom: 8 },
  connectButton: { backgroundColor: "#2563eb", borderRadius: 8, paddingHorizontal: 20, paddingVertical: 12 },
  connectButtonText: { color: "#fff", fontWeight: "600" },
});
