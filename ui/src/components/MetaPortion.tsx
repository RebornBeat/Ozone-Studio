/**
 * MetaPortion Component v0.4.0
 * 
 * The META section (30% width) displays:
 * - Consciousness emotions and state
 * - Voice visualization (WAV when speaking)
 * - Conversation transcript
 * - I-Loop status
 * - Primary prompt input
 * 
 * When consciousness is DISABLED: Shows a semi-transparent overlay
 * but keeps the beautiful META UI visible underneath to encourage enabling.
 */

import React, { useState, useEffect, useRef, useCallback } from 'react';
import { useOzoneStore } from '../services/store';
import { AgentActivityIndicator } from '../views/coordination/AgentActivity';
import {
  TranscriptEntry,
  modelStyle,
  renderMarkdown,
} from '../chatRendering';
import TranscriptEntryMeta from './TranscriptEntryMeta';
import ThinkingRow from './ThinkingRow';
import PausedBanner from './PausedBanner';
import { OrchestrationPanels, CoordinationStrip } from './OrchestrationPanels';
import { useConsciousnessToggles } from '../hooks/useConsciousnessToggles';
import { useChatSession } from '../hooks/useChatSession';
import { hostGet } from '../ozoneClient';
import { loadILoopStatus } from '../data/iLoopReflections';
import { getGraphEventClient, GraphEventFrame } from '../graphEventClient';

interface MetaPortionProps {
  width: number;
}

interface EmotionState {
  primary: string;
  intensity: number;
  secondary?: string;
  valence: number;
  arousal: number;
}

interface ILoopState {
  isActive: boolean;
  currentQuestion: string;
  questionsAsked: number;
  insightsGenerated: number;
}

export function MetaPortion({ width }: MetaPortionProps) {
  const {
    consciousnessEnabled,
    isConnected,
    promptInput,
    setPromptInput,
    submitPrompt,
    executePipeline,
    selectedModel,
    setSelectedModel,
    availableModels,
  } = useOzoneStore();
  
  const [voiceActive, setVoiceActive] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  // Honest voice failures (no TTS engine, STT error) — shown beside the mic, never swallowed.
  const [voiceError, setVoiceError] = useState<string | null>(null);
  // Live mic level (RMS from an AnalyserNode) — drives the orb + bars with
  // real captured audio, never simulated.
  const [micLevel, setMicLevel] = useState(0);
  const rafRef = useRef<number | null>(null);
  const [emotionState, setEmotionState] = useState<EmotionState>({
    primary: 'neutral',
    intensity: 0.3,
    secondary: undefined,
    valence: 0.0,
    arousal: 0.3,
  });
  const [iLoopState, setILoopState] = useState<ILoopState>({
    isActive: false,
    currentQuestion: '',
    questionsAsked: 0,
    insightsGenerated: 0,
  });
  // Chat context: Global or a specific workspace. Previously the only
  // "workspace scoping" was a dead read of window.ozone.sharedState (never
  // exposed by electron/preload.js — confirmed by grep, so project_id/
  // workspace_id were always undefined on every submit regardless of what
  // was selected in the Workspace tab). This is now a real, visible
  // switcher with its own state, and one transcript PER context so a
  // global conversation and a workspace conversation never mix in the same
  // view.
  type ChatContext = { type: 'global' } | { type: 'workspace'; id: number; name: string };
  const [chatContext, setChatContext] = useState<ChatContext>({ type: 'global' });
  const [workspaceOptions, setWorkspaceOptions] = useState<Array<{ id: number; name: string }>>([]);
  const contextKey = chatContext.type === 'global' ? 'global' : `ws:${chatContext.id}`;
  // Shared chat engine (same hook ProjectChatPanel uses): per-context
  // persisted transcripts, task discovery, timeout/awaiting recovery.
  const chat = useChatSession({
    workspaceId: chatContext.type === 'workspace' ? chatContext.id : undefined,
    contextKey: `meta:${contextKey}`,
    modelConfig: selectedModel ? { model_identifier: selectedModel } : undefined,
    consciousnessEnabled,
    onResult: async (result: any) => {
      // Voice + orb extras (global chat only).
      if (consciousnessEnabled && result?.response) {
        setIsSpeaking(true);
        try { await speakResponse(result.response); } finally { setIsSpeaking(false); }
      }
      fetchEmotionalState();
    },
  });
  const { transcript, setTranscript, isRunning, elapsedSeconds, awaiting: awaitingKind } = chat;
  const currentStepInfo = chat.stepInfo;
  const currentTaskId = chat.currentTaskId;

  // Fetch the real workspace list once connected (pipeline 6's real
  // ListWorkspaces action, assets/pipelines/general/workspace_tab/main.rs)
  // to populate the switcher — never fabricated/hardcoded options.
  useEffect(() => {
    if (!isConnected) return;
    (async () => {
      try {
        const result = await executePipeline(6, { action: 'ListWorkspaces' });
        const list = (result?.workspaces ?? []).map((w: any) => ({ id: w.id, name: w.name }));
        setWorkspaceOptions(list);
      } catch {
        /* switcher just shows Global-only if this fails */
      }
    })();
  }, [isConnected]);

  const [voiceWaveform, setVoiceWaveform] = useState<number[]>(new Array(24).fill(0.1));
  const transcriptRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const [cancelling, setCancelling] = useState(false);
  const handleStop = async () => {
    if (!currentTaskId || cancelling) return;
    setCancelling(true);
    try { await chat.stop(); } finally { setCancelling(false); }
  };

  // Emotion orb: read-only GET /consciousness/state (pipeline 40 was a dead
  // route). If the route/fields are absent the orb keeps its honest neutral
  // defaults and emotionSource says so.
  // [consciousness].show_emotional_state — hides the orb when false.
  const { showEmotionalState } = useConsciousnessToggles();
  const [emotionSource, setEmotionSource] = useState<'live' | 'default (no /consciousness/state data)'>('default (no /consciousness/state data)');
  const fetchEmotionalState = useCallback(async () => {
    if (!isConnected || !consciousnessEnabled) return;
    try {
      const res: any = await hostGet('/consciousness/state');
      const st = res?.state ?? res?.emotional_state ?? res;
      const primaryEmotion = st?.primary_emotions?.[0];
      // The host route returns emotional_state.primary_emotion as a STRING.
      const primaryName: string | undefined =
        typeof st?.primary_emotion === 'string' ? st.primary_emotion : primaryEmotion?.emotion;
      if (primaryName || primaryEmotion || typeof st?.valence === 'number') {
        setEmotionState({
          primary: primaryName || 'neutral',
          intensity: primaryEmotion?.intensity ?? Math.min(1, Math.abs(typeof st?.arousal === 'number' ? st.arousal : 0.3)),
          secondary: st?.primary_emotions?.[1]?.emotion,
          valence: st?.valence ?? 0.0,
          arousal: st?.arousal ?? 0.3,
        });
        setEmotionSource('live');
      } else {
        setEmotionSource('default (no /consciousness/state data)');
      }
    } catch {
      setEmotionSource('default (no /consciousness/state data)');
    }
  }, [isConnected, consciousnessEnabled]);

  // Real I-Loop status (2026-10-18) — was polling pipeline #44
  // (`GetILoopStatus`), a separate subprocess-per-call implementation whose
  // in-memory store resets to empty on every invocation and that nothing in
  // this codebase ever actually triggers (confirmed by grep) — it was
  // reading a permanently-empty store, which is why this always showed "0
  // insights"/the idle-fallback text regardless of real activity. The REAL
  // I-Loop (`src/orchestrator/i_loop.rs`'s `run_i_loop`, confirmed spawned
  // in the live host via `src/lib.rs:1114`) persists real reflections as
  // real ZSEI containers; `loadILoopStatus` reads those. See
  // `ui/src/data/iLoopReflections.ts`'s header for the full trace.
  const fetchILoopStatus = useCallback(async () => {
    if (!isConnected || !consciousnessEnabled) return;
    try {
      const status = await loadILoopStatus();
      setILoopState((prev) => ({
        // No live "currently reflecting" signal exists yet (the real loop
        // emits no start/stop event, only the final persisted reflection) —
        // left false, same honest default as before; not a regression.
        isActive: false,
        currentQuestion: status.currentReflection ?? prev.currentQuestion,
        questionsAsked: prev.questionsAsked,
        insightsGenerated: status.insightsGenerated,
      }));
    } catch (err) {
      console.warn('Failed to fetch I-Loop status:', err);
    }
  }, [isConnected, consciousnessEnabled]);

  // Poll consciousness state when enabled
  useEffect(() => {
    if (!isConnected || !consciousnessEnabled) return;

    // Initial fetch
    fetchEmotionalState();

    // Poll every 5 seconds for updates
    const interval = setInterval(() => {
      fetchEmotionalState();
    }, 5000);

    return () => clearInterval(interval);
  }, [isConnected, consciousnessEnabled, fetchEmotionalState]);

  // I-Loop: real reflections land roughly every 30+ minutes (the loop's own
  // budget-safety floor, `i_loop.rs`'s MIN_INTERVAL_SECS) — a 5s poll would
  // just be hitting a ZSEI query for data that almost never changes. Fetch
  // once for real historical state, then rely on the live graph-event
  // socket (already real and connected elsewhere, e.g. TaskDetailPanel) to
  // know exactly when a new one actually lands, instead of guessing a poll
  // interval.
  useEffect(() => {
    if (!isConnected || !consciousnessEnabled) return;
    fetchILoopStatus();
    const client = getGraphEventClient();
    client.connect();
    const off = client.onEvent((frame: GraphEventFrame) => {
      if (frame.event === 'created' && frame.container_type === 'CoordinationEvent') {
        // Best-effort: CoordinationEvent covers every shared-context event
        // (notes/decisions/handoffs too), not just i-loop reflections — the
        // frame itself carries no kind keyword to filter on more precisely.
        // The refetch is a small, cheap container-list read, not a reason
        // to invest in a server-side filter for this.
        fetchILoopStatus();
      }
    });
    return off;
  }, [isConnected, consciousnessEnabled, fetchILoopStatus]);

  // Brief, subtle highlight when a genuinely new reflection arrives —
  // separate from the constant idle pulse on .iloop-dot. Purely additive
  // polish; the widget's own layout/copy is unchanged.
  const [justReflected, setJustReflected] = useState(false);
  const lastReflectionRef = useRef<string>('');
  useEffect(() => {
    const text = iLoopState.currentQuestion;
    if (!text) return;
    const prev = lastReflectionRef.current;
    lastReflectionRef.current = text;
    // Skip the flash on the first real value this session (prev === '') —
    // only genuinely NEW reflections (a real change) should highlight.
    if (prev !== '' && prev !== text) {
      setJustReflected(true);
      const t = setTimeout(() => setJustReflected(false), 2200);
      return () => clearTimeout(t);
    }
  }, [iLoopState.currentQuestion]);

  // Voice waveform — REAL mic level while listening (AnalyserNode RMS);
  // synthesized motion only while the assistant is speaking.
  useEffect(() => {
    if (isSpeaking) {
      const interval = setInterval(() => {
        setVoiceWaveform(prev =>
          prev.map(() => 0.1 + Math.random() * 0.9)
        );
      }, 80);
      return () => clearInterval(interval);
    }
    if (voiceActive) {
      setVoiceWaveform(prev =>
        prev.map((_, i) => {
          const wave = Math.abs(Math.sin(Date.now() / 120 + i * 0.55));
          return Math.max(0.06, Math.min(1, micLevel * (0.55 + 0.45 * wave) + 0.04));
        })
      );
    } else {
      setVoiceWaveform(new Array(24).fill(0.12));
    }
  }, [isSpeaking, voiceActive, micLevel]);

  // Auto-scroll transcript
  useEffect(() => {
    if (transcriptRef.current) {
      transcriptRef.current.scrollTop = transcriptRef.current.scrollHeight;
    }
  }, [transcript]);

  // Auto-resize textarea
  const handleTextareaChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setPromptInput(e.target.value);
    // Auto-resize
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height = Math.min(textareaRef.current.scrollHeight, 120) + 'px';
    }
  };

  // ── Whisper-native voice capture ────────────────────────────────────────
  // AudioContext captures Float32 PCM at 16 kHz directly (the browser
  // resamples internally), so the chunks sent to pipeline #10 are already in
  // whisper's input format — no ffmpeg transcode server-side. PCM16 WAV is
  // assembled client-side.

  const startWhisperCapture = async () => {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    // 16 kHz context: the browser resamples the mic's native rate for us.
    const ctx = new AudioContext({ sampleRate: 16000 });
    const source = ctx.createMediaStreamSource(stream);
    const processor = ctx.createScriptProcessor(4096, 1, 1);
    const chunks: Float32Array[] = [];

    // Analyser feeds the UI orb/bars with the live captured level.
    const analyser = ctx.createAnalyser();
    analyser.fftSize = 512;
    source.connect(analyser);
    const levelBuf = new Uint8Array(analyser.fftSize);
    const tick = () => {
      analyser.getByteTimeDomainData(levelBuf);
      let sum = 0;
      for (let i = 0; i < levelBuf.length; i++) {
        const v = (levelBuf[i] - 128) / 128;
        sum += v * v;
      }
      setMicLevel(Math.min(1, Math.sqrt(sum / levelBuf.length) * 4));
      rafRef.current = requestAnimationFrame(tick);
    };
    tick();

    processor.onaudioprocess = (e: AudioProcessingEvent) => {
      chunks.push(new Float32Array(e.inputBuffer.getChannelData(0)));
    };
    source.connect(processor);
    // ScriptProcessor needs a destination connection to fire in some browsers;
    // connect to a zero-gain node so nothing audible leaks.
    const silent = ctx.createGain();
    silent.gain.value = 0;
    processor.connect(silent);
    silent.connect(ctx.destination);

    (window as any).__whisperCapture = { ctx, stream, processor, chunks };
  };

  // Assemble accumulated Float32 chunks → PCM16 → WAV bytes → base64.
  const encodeWhisperWav = async (): Promise<{ base64: string; format: string } | null> => {
    const capture = (window as any).__whisperCapture;
    if (!capture || capture.chunks.length === 0) return null;

    const total = capture.chunks.reduce((a: number, c: Float32Array) => a + c.length, 0);
    const pcm16 = new Int16Array(total);
    let offset = 0;
    for (const chunk of capture.chunks) {
      for (let i = 0; i < chunk.length; i++) {
        const s = Math.max(-1, Math.min(1, chunk[i]));
        pcm16[offset++] = s < 0 ? s * 0x8000 : s * 0x7fff;
      }
    }
    capture.chunks.length = 0;

    const sampleRate = capture.ctx.sampleRate;
    const header = new ArrayBuffer(44);
    const view = new DataView(header);
    const writeStr = (pos: number, s: string) => {
      for (let i = 0; i < s.length; i++) view.setUint8(pos + i, s.charCodeAt(i));
    };
    writeStr(0, 'RIFF');
    view.setUint32(4, 36 + pcm16.byteLength, true);
    writeStr(8, 'WAVE');
    writeStr(12, 'fmt ');
    view.setUint32(16, 16, true);
    view.setUint16(20, 1, true);              // PCM
    view.setUint16(22, 1, true);              // mono
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, sampleRate * 2, true); // byte rate
    view.setUint16(32, 2, true);              // block align
    view.setUint16(34, 16, true);             // bits
    writeStr(36, 'data');
    view.setUint32(40, pcm16.byteLength, true);

    const blob = new Blob([header, pcm16.buffer], { type: 'audio/wav' });
    const reader = new FileReader();
    reader.readAsDataURL(blob);
    return new Promise<{ base64: string; format: string } | null>((resolve) => {
      reader.onloadend = () => {
        const base64 = (reader.result as string)?.split(',')[1];
        resolve(base64 ? { base64, format: 'wav' } : null);
      };
    });
  };

  const stopWhisperCapture = async (): Promise<{ base64: string; format: string } | null> => {
    const capture = (window as any).__whisperCapture;
    if (!capture) return null;
    const wav = await encodeWhisperWav();
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    setMicLevel(0);
    try {
      capture.processor.disconnect();
      capture.stream.getTracks().forEach((t: MediaStreamTrack) => t.stop());
      await capture.ctx.close();
    } catch (e) {
      console.warn('capture cleanup:', e);
    }
    delete (window as any).__whisperCapture;
    return wav;
  };

  // Toggle voice input — whisper-native capture: accumulate PCM16/16 kHz WAV
  // while active, transcribe on stop (final transcript appended to prompt).
  const toggleVoice = async () => {
    if (!isConnected) return;

    try {
      if (!voiceActive) {
        await executePipeline(10, { action: 'StartListening' });
        await startWhisperCapture();
        setVoiceActive(true);
      } else {
        setVoiceActive(false);
        const wav = await stopWhisperCapture();
        await executePipeline(10, { action: 'StopListening' });

        if (!wav) return;
        const transcribeResult = await executePipeline(10, {
          action: 'ProcessAudio',
          audio_base64: wav.base64,
          format: 'wav',
        });

        if (transcribeResult && transcribeResult.success === false) {
          setVoiceError(`speech-to-text failed: ${transcribeResult.error ?? 'no transcript'}`);
        }
        if (transcribeResult?.transcription && transcribeResult.is_final) {
          setVoiceError(null);
          const prev = typeof promptInput === 'string' ? promptInput : '';
          const space = prev && !prev.endsWith(' ') ? ' ' : '';
          setPromptInput(prev + space + transcribeResult.transcription);
        }
      }
    } catch (err) {
      console.error('Voice toggle failed:', err);
      setVoiceError(`voice capture failed: ${err instanceof Error ? err.message : String(err)}`);
      // Cleanup partial capture on failure
      try { await stopWhisperCapture(); } catch { /* already gone */ }
      setVoiceActive(false);
    }
  };
  
  // Speak response aloud using voice pipeline with consciousness identity
  const speakResponse = async (text: string) => {
    if (!consciousnessEnabled) return;
    
    try {
      setIsSpeaking(true);
      
      // Get voice settings
      const settingsResult = await executePipeline(10, { action: 'GetSettings' });
      const settings = settingsResult?.settings;
      
      // Only speak if output is enabled
      if (!settings?.output_enabled) {
        setIsSpeaking(false);
        return;
      }
      
      // Get voice identity from self_model (consciousness pipeline #43)
      // This ensures the voice matches the consciousness identity
      let voiceStyle = settings?.output_voice || 'default';
      let speechRate = settings?.output_speed || 1.0;
      
      try {
        const identityResult = await executePipeline(43, { action: 'GetVoice' });
        if (identityResult?.voice) {
          const voice = identityResult.voice;
          
          // Map consciousness voice traits to TTS parameters
          // Warmth affects pitch/timbre selection
          // Formality affects pacing
          // Directness affects emphasis
          
          if (voice.warmth > 0.6) {
            voiceStyle = 'warm'; // Warmer voice variant
          } else if (voice.warmth < 0.4) {
            voiceStyle = 'neutral';
          }
          
          // Adjust speed based on formality (formal = slower, measured)
          if (voice.formality > 0.7) {
            speechRate = 0.9; // Slower, more deliberate
          } else if (voice.formality < 0.3) {
            speechRate = 1.1; // Slightly faster, casual
          }
        }
      } catch (e) {
        // If we can't get voice identity, use defaults
        console.warn('Could not get voice identity:', e);
      }
      
      const result = await executePipeline(10, {
        action: 'Speak',
        text,
        voice: voiceStyle,
        speed: speechRate,
      });
      
      // The voice pipeline reports success:false (with the engines it tried)
      // when no TTS engine is installed — surface it, don't stay silent.
      if (result && result.success === false) {
        setVoiceError(`speech failed: ${result.error ?? 'no TTS engine produced audio'}`);
      } else {
        setVoiceError(null);
      }
      // Play audio if returned
      if (result?.audio_base64) {
        const audio = new Audio(`data:audio/wav;base64,${result.audio_base64}`);
        (window as any).__audioPlaying = true;
        audio.onended = () => {
          setIsSpeaking(false);
          (window as any).__audioPlaying = false;
        };
        audio.onerror = () => {
          setIsSpeaking(false);
          (window as any).__audioPlaying = false;
        };
        await audio.play();
        return; // Don't set isSpeaking to false until audio ends
      }
    } catch (err) {
      console.warn('Speech failed:', err);
      setVoiceError(`speech failed: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      // Only set false if we didn't start playing audio
      if (!(window as any).__audioPlaying) {
        setIsSpeaking(false);
      }
    }
  };

  // Handle prompt submission
  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault?.();
    if (!promptInput.trim() || !isConnected || isRunning) return;
    const text = promptInput;
    // Clear immediately on send, not after the response arrives.
    setPromptInput('');
    if (textareaRef.current) textareaRef.current.style.height = 'auto';
    if ((window as any).ozone?.orchestrate) {
      await chat.send(text);
      return;
    }
    // Web fallback (no Electron bridge): the store's orchestratePrompt.
    setTranscript(prev => [...prev, { id: Date.now(), role: 'user', content: text, timestamp: Date.now() }]);
    const processingId = Date.now() + 1;
    setTranscript(prev => [...prev, { id: processingId, role: 'assistant', content: '⏳ Processing...', timestamp: Date.now() }]);
    try {
      const result = await useOzoneStore.getState().orchestratePrompt(text);
      setTranscript(prev => prev.map(en => en.id === processingId
        ? { ...en, content: result?.response || result?.error || '⚠ The host returned neither a response nor an error — this is a failure to answer, not an empty answer.' } : en));
      if (consciousnessEnabled && result?.response) await speakResponse(result.response);
      fetchEmotionalState();
    } catch (err) {
      setTranscript(prev => prev.map(en => en.id === processingId
        ? { ...en, content: `Error: ${err instanceof Error ? err.message : 'Request failed'}`, emotion: 'concerned' } : en));
    }
  };

  // Get emotion color based on valence and arousal
  const getEmotionColor = (emotion: string): string => {
    const colors: Record<string, string> = {
      // Positive emotions
      joy: '#4ade80',
      satisfaction: '#22c55e',
      gratitude: '#34d399',
      curiosity: '#3b82f6',
      anticipation: '#06b6d4',
      // Negative emotions
      frustration: '#f59e0b',
      concern: '#f97316',
      anxiety: '#ef4444',
      sadness: '#6366f1',
      // Neutral
      neutral: '#64748b',
      focused: '#8b5cf6',
      thoughtful: '#a855f7',
      calm: '#6366f1',
    };
    return colors[emotion] || colors.neutral;
  };

  // Get emotion emoji
  const getEmotionEmoji = (emotion: string): string => {
    const emojis: Record<string, string> = {
      joy: '😊',
      satisfaction: '😌',
      gratitude: '🙏',
      curiosity: '🤔',
      anticipation: '✨',
      frustration: '😤',
      concern: '😟',
      anxiety: '😰',
      sadness: '😢',
      neutral: '🧠',
      focused: '🎯',
      thoughtful: '💭',
      calm: '😌',
    };
    return emojis[emotion] || '🧠';
  };

  const emotionColor = getEmotionColor(emotionState.primary);

  return (
    <aside className="meta-portion" style={{ width: `${width}%` }}>
      {/* Main META Content */}
      <div className="meta-content">

        {/* Global/Workspace chat context switcher — real state (chatContext
            above), each context keeps its own transcript, and the selection
            actually reaches the backend via handleSubmit's workspace_id.
            Previously there was no visible switcher at all; workspace
            scoping silently depended on a shared-state bridge that was
            never wired to window.ozone. */}
        <div className="chat-context-switcher">
          <select
            className="chat-context-select"
            value={contextKey}
            onChange={(e) => {
              const v = e.target.value;
              if (v === 'global') {
                setChatContext({ type: 'global' });
              } else {
                const id = parseInt(v.slice(3), 10);
                const opt = workspaceOptions.find(w => w.id === id);
                setChatContext({ type: 'workspace', id, name: opt?.name ?? `Workspace ${id}` });
              }
            }}
            title="Chat context — Global or a specific workspace. Each keeps its own conversation."
          >
            <option value="global">🌐 Global</option>
            {workspaceOptions.map(w => (
              <option key={w.id} value={`ws:${w.id}`}>📁 {w.name}</option>
            ))}
          </select>
        </div>

        {/* Emotion Display — the consciousness-disabled overlay is scoped to
            JUST this wrapper (meta-decorative), not the whole meta-content.
            It used to be a sibling of transcript-section with position:
            absolute + inset:0 on meta-content itself, which visually covered
            the real conversation transcript too — chat still technically
            worked (the input lives outside meta-content entirely), but you
            couldn't see your own messages or the response appear, which is
            indistinguishable from "chat is broken". Scoping the overlay to
            only the decorative emotion/reflection area fixes that while
            keeping the exact same nudge-to-enable message. */}
        <div className={`meta-decorative ${!consciousnessEnabled ? 'has-overlay' : ''}`}>
          <div className="emotion-display" style={showEmotionalState ? undefined : { display: 'none' }}>
            <div
              title={`emotion source: ${emotionSource}`} className={`emotion-orb ${isSpeaking ? 'speaking' : ''}`}
              style={{
                '--emotion-color': emotionColor,
                '--emotion-glow': `${emotionColor}40`
              } as React.CSSProperties}
            >
              <span className="emotion-emoji">{getEmotionEmoji(emotionState.primary)}</span>
              <div className="orb-ring" />
              <div className="orb-ring ring-2" />
            </div>
            <div className="emotion-info">
              <span className="emotion-label">Emotional State</span>
              <span className="emotion-primary" style={{ color: emotionColor }}>
                {emotionState.primary.charAt(0).toUpperCase() + emotionState.primary.slice(1)}
              </span>
              <div className="emotion-bar-container">
                <div
                  className="emotion-bar"
                  style={{
                    width: `${emotionState.intensity * 100}%`,
                    backgroundColor: emotionColor
                  }}
                />
              </div>
              {emotionState.secondary && (
                <span className="emotion-secondary">
                  + {emotionState.secondary}
                </span>
              )}
              {/* Valence/Arousal indicators */}
              {consciousnessEnabled && (
                <div className="emotion-metrics">
                  <span title={`Valence: ${emotionState.valence.toFixed(2)}`}>
                    {emotionState.valence > 0 ? '😊' : emotionState.valence < 0 ? '😔' : '😐'}
                  </span>
                  <span title={`Arousal: ${emotionState.arousal.toFixed(2)}`}>
                    {emotionState.arousal > 0.6 ? '⚡' : emotionState.arousal < 0.3 ? '😴' : '🔹'}
                  </span>
                </div>
              )}
            </div>
          </div>

          {/* Consciousness Disabled Overlay - TRANSPARENT, scoped to the
              decorative section only (see comment above) */}
          {!consciousnessEnabled && (
            <div className="consciousness-overlay">
              <div className="overlay-message">
                <span className="overlay-icon">🧠</span>
                <h3>Consciousness Disabled</h3>
                <p>Enable in Settings for full META experience</p>
                <div className="overlay-features">
                  <span>✨ Emotional awareness</span>
                  <span>📚 Experience memory</span>
                  <span>🔄 Self-reflection</span>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Transcript — always fully visible/interactive regardless of
            consciousness state; this is the real chat, not a "preview" */}
        <div className="transcript-section" ref={transcriptRef}>
          <div className="transcript-header">
            <span>💬 Conversation</span>
            <AgentActivityIndicator />
          </div>
          {transcript.length === 0 ? (
            <div className="transcript-empty">
              <span className="empty-emoji">✨</span>
              <span className="empty-text">Start a conversation with OZONE</span>
              <span className="empty-hint">Type below or use voice input</span>
            </div>
          ) : (
            <div className="transcript-messages">
              {(() => {
                let lastAssistantModel: string | undefined;
                return transcript.map((entry) => {
                  const ms = modelStyle(entry.modelUsed);
                  const switched =
                    entry.role === 'assistant' &&
                    !!entry.modelUsed &&
                    !!lastAssistantModel &&
                    lastAssistantModel !== entry.modelUsed;
                  if (entry.role === 'assistant' && entry.modelUsed) {
                    lastAssistantModel = entry.modelUsed;
                  }
                  return (
                    <React.Fragment key={entry.id}>
                      {switched && (
                        <div
                          className="model-switch-divider"
                          style={{ display: 'flex', alignItems: 'center', gap: 10, margin: '10px 0' }}
                        >
                          <span style={{ flex: 1, height: 1, background: '#223046' }} />
                          <span
                            style={{
                              border: `1px solid ${ms.color}`,
                              color: ms.color,
                              background: ms.color + '14',
                              borderRadius: 999,
                              fontSize: 11,
                              padding: '3px 10px',
                              whiteSpace: 'nowrap',
                            }}
                          >
                            {ms.icon} switched to {entry.modelUsed}
                          </span>
                          <span style={{ flex: 1, height: 1, background: '#223046' }} />
                        </div>
                      )}
                      <div className={`message ${entry.role}`}>
                        <div className="message-header">
                          <span className="message-author">
                            {entry.role === 'user' ? 'You' : 'OZONE'}
                          </span>
                          {entry.emotion && (
                            <span className="message-emotion" style={{ color: getEmotionColor(entry.emotion) }}>
                              {getEmotionEmoji(entry.emotion)}
                            </span>
                          )}
                          <span className="message-time">
                            {new Date(entry.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                          </span>
                        </div>
                        <div
                          className="message-content"
                          style={{
                            overflowWrap: 'break-word',
                            wordBreak: 'break-word',
                            ...(entry.role === 'assistant' && entry.modelUsed
                              ? { borderLeft: `2px solid ${ms.color}88`, paddingLeft: 10 }
                              : {}),
                          }}
                        >
                          {entry.role === 'assistant' ? renderMarkdown(entry.content) : entry.content}
                        </div>
                        <TranscriptEntryMeta entry={entry} />
                      </div>
                    </React.Fragment>
                  );
                });
              })()}
              {/* In-transcript live indicator — the thread itself now shows
                  the run is alive, not just the panels above the input. Every
                  value here is real: elapsedSeconds ticks from submit time,
                  and currentStepInfo comes from the /task/list poll once a
                  real task record exists (stage 7+). Before that record
                  exists we say exactly that instead of inventing a stage. */}
              <ThinkingRow isRunning={isRunning} elapsedSeconds={elapsedSeconds} stepInfo={currentStepInfo} awaiting={awaitingKind} />
              <PausedBanner paused={chat.paused} onResume={chat.resumePaused} busy={isRunning} />
              <CoordinationStrip />
            </div>
          )}
        </div>

        {/* I-Loop Status (when consciousness enabled) — now wired to the
            REAL orchestrator-side loop; `justReflected` is purely additive
            (one extra class for a brief CSS highlight on genuine new data,
            see App.css), no change to this block's existing structure/copy. */}
        {consciousnessEnabled && (
          <div className={`iloop-section${justReflected ? ' just-reflected' : ''}`}>
            <div className="iloop-header">
              <span
                className={`iloop-dot ${iLoopState.isActive ? 'active' : ''}`}
                style={{ backgroundColor: iLoopState.isActive ? '#4ade80' : emotionColor }}
              />
              <span className="iloop-title">
                I-Loop {iLoopState.isActive ? 'Active' : 'Ready'}
              </span>
              <span className="iloop-stats">
                {iLoopState.insightsGenerated} insights
              </span>
            </div>
            <div className="iloop-reflection">
              <span className="reflection-label">Current reflection:</span>
              <span
                className={`reflection-text${justReflected ? ' just-reflected' : ''}`}
                title={iLoopState.currentQuestion || 'Waiting for next reflection cycle...'}
              >
                "{iLoopState.currentQuestion || 'Waiting for next reflection cycle...'}"
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Prompt Input - Always visible; voice lives HERE, in the chat */}
      <div className="meta-prompt">
        {/* Real, live stage-by-stage status — replaces the old single-line
            "Working… (elapsed)" indicator, which is why elapsedSeconds/
            currentStepInfo (still fed by the existing task-polling fallback
            below) are no longer rendered directly here; OrchestrationStatusPanel
            covers that ground with the real orchestration_stage/pipeline_progress
            WS frames instead. */}
        <OrchestrationPanels isRunning={isRunning} scope={{ workspaceId: undefined }} />
        <form onSubmit={handleSubmit}>
          <div className={`prompt-input-wrapper ${voiceActive ? 'listening' : ''}`}>
            {voiceActive && (
              <div className="listen-meter">
                {voiceWaveform.map((h, i) => (
                  <div
                    key={i}
                    className="listen-bar"
                    style={{ height: `${Math.max(3, h * 20)}px` }}
                  />
                ))}
              </div>
            )}
            <textarea
              ref={textareaRef}
              value={promptInput}
              onChange={handleTextareaChange}
              placeholder={
                voiceActive
                  ? "Listening… speak now (mic again to stop)"
                  : isConnected
                    ? "Chat with OZONE..."
                    : "Connecting to backend..."
              }
              disabled={!isConnected}
              rows={1}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleSubmit();
                }
              }}
            />
          </div>
          <div className="prompt-controls">
            {availableModels.length > 0 && (
              <select
                className="model-picker"
                value={selectedModel}
                onChange={(e) => setSelectedModel(e.target.value)}
                title="Model for the next message (Phase 6 per-request routing)"
                style={{
                  background: '#0b1120',
                  border: '1px solid #223046',
                  borderRadius: 8,
                  color: 'var(--color-text-muted)',
                  fontSize: 11,
                  padding: '4px 6px',
                  maxWidth: 130,
                }}
              >
                {availableModels.map((m) => (
                  <option key={m.identifier} value={m.identifier}>
                    {m.name}
                  </option>
                ))}
              </select>
            )}
            <button
              type="button"
              className={`control-btn voice-btn ${voiceActive ? 'active' : ''}`}
              onClick={toggleVoice}
              disabled={!isConnected}
              title={voiceActive ? 'Stop recording' : 'Voice input'}
            >
              {voiceActive ? '🔴' : '🎤'}
            </button>
            {voiceActive && (
              <span className="voice-db">level {Math.round(micLevel * 100)}%</span>
            )}
            {voiceError && (
              <span
                className="voice-db"
                role="alert"
                title={voiceError}
                onClick={() => setVoiceError(null)}
                style={{ color: '#f87171', cursor: 'pointer', maxWidth: 220, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
              >
                ⚠ {voiceError}
              </span>
            )}
            {isRunning ? (
              <button
                type="button"
                className="control-btn send-btn"
                onClick={handleStop}
                disabled={!currentTaskId || cancelling}
                title={
                  currentTaskId
                    ? 'Stop — finishes the current step, then halts'
                    : 'Locating the running task…'
                }
                style={{ background: '#3a1620', borderColor: '#7a2a3a', color: '#f87171' }}
              >
                {cancelling ? '…' : '■'}
              </button>
            ) : (
              <button
                type="submit"
                className="control-btn send-btn"
                disabled={!isConnected || !promptInput.trim()}
                title="Send message"
              >
                <span className="send-arrow">→</span>
              </button>
            )}
          </div>
        </form>
      </div>
    </aside>
  );
}
