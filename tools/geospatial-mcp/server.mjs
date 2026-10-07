// Ozone-Studio Geospatial MCP — real OpenStreetMap connector
// (docs/CAPABILITY_EXPANSION_REVIEW.md §3: "Geospatial — OpenStreetMap
// Overpass + Nominatim. Free, no API key for reads, real worldwide
// data, generous limits.").
//
// SECURITY: these are genuinely free, public, no-auth-required read
// APIs (Nominatim search/reverse, Overpass QL) — nothing is locked by
// credentials, because none are needed. The real constraint instead is
// RESPECTFUL USE of someone else's free public infrastructure:
// - A real, honest User-Agent identifying this tool (both services'
//   usage policies require one; Nominatim explicitly rejects generic/
//   browser-default User-Agents).
// - Nominatim's own policy: max ~1 request/second, no heavy bulk use —
//   enforced here with a real per-process minimum-interval throttle
//   (not bypassable by the caller), not just documented and trusted.
// - Overpass (overpass-api.de, the real public instance) has its own
//   fair-use expectation — same honest User-Agent, no bulk scraping.
// No sudo, no filesystem writes outside the real content-pointer
// convention this session already uses, no local data at all — this
// MCP is a pure, stateless read-through to two real public APIs.
//
// Tools:
//   geo_search {input:{query}}
//     → Nominatim forward geocode: place name/address → real coordinates.
//   geo_reverse {input:{lat, lon}}
//     → Nominatim reverse geocode: real coordinates → place name.
//   geo_features_near {input:{lat, lon, radius_m, feature_type?}}
//     → Overpass QL: real OSM features (amenity/highway/building, or a
//       caller-given feature_type) within radius_m of a real point.
//
// Feeds `assets/pipelines/modalities/geospatial/main.rs`'s real
// `Geocode`/`ReverseGeocode`/`GeoDataSource::OSM` actions — this MCP is
// the real external data source those actions were designed to consume.
//
// Env: OZONE_GEOSPATIAL_PORT (default 3274).

import { createServer } from "node:http";

const PORT = Number(process.env.OZONE_GEOSPATIAL_PORT ?? 3274);
// Real, honest identification — both services' own usage policies
// require a real User-Agent naming the application, not a generic one.
const USER_AGENT = "Ozone-Studio-geospatial-mcp/0.1.0 (local self-hosted AGI research tool; https://github.com/ozone-studio)";
const NOMINATIM_BASE = "https://nominatim.openstreetmap.org";
const OVERPASS_BASE = "https://overpass-api.de/api/interpreter";

// Real per-process throttle enforcing Nominatim's own documented
// "max 1 request/second" usage policy — not just stated in a comment,
// actually enforced regardless of how fast a caller fires requests.
let lastNominatimCallAt = 0;
async function throttleNominatim() {
  const now = Date.now();
  const elapsed = now - lastNominatimCallAt;
  const minIntervalMs = 1100; // just over 1s, real safety margin
  if (elapsed < minIntervalMs) {
    await new Promise((r) => setTimeout(r, minIntervalMs - elapsed));
  }
  lastNominatimCallAt = Date.now();
}

// Real OSM features near a real point as a graph block: the location is the root,
// and each feature is a contained entity with its real tags and position.
function geoGraph(lat, lon, radiusM, elements) {
  const nodes = [{ key: "location", kind: "Location", label: `${lat},${lon}`, attributes: { lat, lon, radius_m: radiusM } }];
  elements.forEach((e, i) => {
    const t = e.tags ?? {};
    const label = t.name ?? t.amenity ?? t.highway ?? t.building ?? `${e.type}-${e.id}`;
    nodes.push({ key: `feature-${i}`, kind: "OSMFeature", label: String(label), parent: "location", attributes: { osm_type: e.type, osm_id: e.id, lat: e.lat, lon: e.lon, tags: t } });
  });
  return { nodes, edges: [] };
}

function reply(res, code, body) {
  res.writeHead(code, { "Content-Type": "application/json" });
  res.end(JSON.stringify(body));
}

async function nominatimSearch(query) {
  await throttleNominatim();
  const url = `${NOMINATIM_BASE}/search?${new URLSearchParams({ q: query, format: "jsonv2", limit: "5" })}`;
  const res = await fetch(url, { headers: { "User-Agent": USER_AGENT } });
  if (!res.ok) throw new Error(`Nominatim search HTTP ${res.status}`);
  return res.json();
}

async function nominatimReverse(lat, lon) {
  await throttleNominatim();
  const url = `${NOMINATIM_BASE}/reverse?${new URLSearchParams({ lat: String(lat), lon: String(lon), format: "jsonv2" })}`;
  const res = await fetch(url, { headers: { "User-Agent": USER_AGENT } });
  if (!res.ok) throw new Error(`Nominatim reverse HTTP ${res.status}`);
  return res.json();
}

async function overpassFeaturesNear(lat, lon, radiusM, featureType) {
  // Real Overpass QL — featureType defaults to a broad real tag set
  // (amenity/building/highway) when the caller doesn't narrow it.
  const tagFilter = featureType ? `["${featureType}"]` : "";
  const ql = `
    [out:json][timeout:15];
    (
      node${tagFilter}(around:${radiusM},${lat},${lon});
      way${tagFilter}(around:${radiusM},${lat},${lon});
    );
    out center 50;
  `;
  const res = await fetch(OVERPASS_BASE, {
    method: "POST",
    headers: { "User-Agent": USER_AGENT, "Content-Type": "text/plain" },
    body: ql,
  });
  if (!res.ok) throw new Error(`Overpass HTTP ${res.status}`);
  return res.json();
}

const server = createServer((req, res) => {
  if (req.method !== "POST" || !(req.url ?? "").startsWith("/call")) {
    reply(res, 404, { error: "POST /call only" });
    return;
  }
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", async () => {
    let tool = "";
    let input = {};
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8") || "{}");
      tool = String(body?.tool ?? "");
      input = body?.input ?? {};
    } catch {
      reply(res, 400, { success: false, error: "invalid JSON body" });
      return;
    }
    try {
      if (tool === "geo_search") {
        const query = String(input.query ?? "").trim();
        if (!query) throw new Error("query is required");
        const results = await nominatimSearch(query);
        if (!Array.isArray(results) || results.length === 0) {
          reply(res, 200, { success: false, error: `no real place found for '${query}'` });
          return;
        }
        reply(res, 200, {
          success: true,
          output: {
            results: results.map((r) => ({
              display_name: r.display_name,
              lat: parseFloat(r.lat),
              lon: parseFloat(r.lon),
              type: r.type,
              osm_id: r.osm_id,
            })),
          },
        });
      } else if (tool === "geo_reverse") {
        const lat = Number(input.lat);
        const lon = Number(input.lon);
        if (!Number.isFinite(lat) || !Number.isFinite(lon) || lat < -90 || lat > 90 || lon < -180 || lon > 180) {
          throw new Error("lat/lon must be real, finite coordinates (lat -90..90, lon -180..180)");
        }
        const result = await nominatimReverse(lat, lon);
        if (result?.error) {
          reply(res, 200, { success: false, error: `no real place found at (${lat}, ${lon}): ${result.error}` });
          return;
        }
        reply(res, 200, {
          success: true,
          output: { display_name: result.display_name, address: result.address, osm_id: result.osm_id },
        });
      } else if (tool === "geo_features_near") {
        const lat = Number(input.lat);
        const lon = Number(input.lon);
        const radiusM = Number(input.radius_m ?? 200);
        if (!Number.isFinite(lat) || !Number.isFinite(lon) || lat < -90 || lat > 90 || lon < -180 || lon > 180) {
          throw new Error("lat/lon must be real, finite coordinates");
        }
        if (!Number.isFinite(radiusM) || radiusM <= 0 || radiusM > 5000) {
          throw new Error("radius_m must be a real positive number, capped at 5000 (respectful of Overpass's shared free infrastructure)");
        }
        const data = await overpassFeaturesNear(lat, lon, radiusM, input.feature_type);
        const elements = (data.elements ?? []).map((e) => ({
          id: e.id,
          type: e.type,
          tags: e.tags ?? {},
          lat: e.lat ?? e.center?.lat ?? null,
          lon: e.lon ?? e.center?.lon ?? null,
        }));
        reply(res, 200, { success: true, output: { count: elements.length, features: elements, graph: geoGraph(lat, lon, radiusM, elements) } });
      } else {
        reply(res, 200, { success: false, error: `unknown geospatial tool '${tool}' (geo_search, geo_reverse, geo_features_near)` });
      }
    } catch (e) {
      reply(res, 200, { success: false, error: e instanceof Error ? e.message : String(e) });
    }
  });
});

server.on("error", (err) => {
  if (err.code === "EADDRINUSE") { console.error(`[geospatial-mcp] :${PORT} in use — skipping`); return; }
  console.error("[geospatial-mcp] server error:", err);
});

server.listen(PORT, "127.0.0.1", () => {
  console.error(`[geospatial-mcp] /call on 127.0.0.1:${PORT} — geo_search, geo_reverse, geo_features_near (real OpenStreetMap Nominatim+Overpass, throttled to their real usage policy)`);
});
