// useConsciousnessToggles — the two DISPLAY toggles from [consciousness] config
// (show_emotional_state, show_decision_reasoning), read from GET /consciousness/state.
// Defaults to "show" until the host answers (a missing route must not hide data).
// One shared 30 s cache so every chat entry does not re-fetch.

import { useEffect, useState } from 'react';
import { hostGet } from '../ozoneClient';

export interface ConsciousnessToggles {
  showEmotionalState: boolean;
  showDecisionReasoning: boolean;
}

const DEFAULTS: ConsciousnessToggles = { showEmotionalState: true, showDecisionReasoning: true };
let cache: { at: number; value: ConsciousnessToggles } | null = null;
let inflight: Promise<ConsciousnessToggles> | null = null;

async function load(): Promise<ConsciousnessToggles> {
  if (cache && Date.now() - cache.at < 30_000) return cache.value;
  if (inflight) return inflight;
  inflight = (async () => {
    try {
      const res: any = await hostGet('/consciousness/state');
      const t = res?.toggles ?? {};
      const value: ConsciousnessToggles = {
        showEmotionalState: t.show_emotional_state !== false,
        showDecisionReasoning: t.show_decision_reasoning !== false,
      };
      cache = { at: Date.now(), value };
      return value;
    } catch {
      return DEFAULTS;
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

export function useConsciousnessToggles(): ConsciousnessToggles {
  const [v, setV] = useState<ConsciousnessToggles>(cache?.value ?? DEFAULTS);
  useEffect(() => {
    let cancelled = false;
    load().then(t => { if (!cancelled) setV(t); });
    return () => { cancelled = true; };
  }, []);
  return v;
}
