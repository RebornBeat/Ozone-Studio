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

interface ThinkingEntry {
  stage: string;
  raw_response: string;
  tokens_used?: number;
  model_used?: string;
  eval_tokens_per_sec?: number;
  prompt_eval_tokens_per_sec?: number;
  load_time_ms?: number;
}

interface TranscriptEntry {
  id: number;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
  emotion?: string;
  /** Which model produced this response — real per-call data forwarded from
   * OrchestrateResponse.model_used, not guessed. */
  modelUsed?: string;
  stageCount?: number;
  /** Full "thinking cycle" — one entry per real LLM call this run made
   * (AMT-building passes, blueprint drafting, zero-shot simulation, step
   * execution), forwarded from OrchestrateResponse.thinking_log. Empty/
   * absent for user messages and older responses from before this existed. */
  thinkingLog?: ThinkingEntry[];
}

// Per-model accent so a model switch mid-conversation is visible at a
// glance, not just a faint text label. Matched by substring against
// whatever model_used/model_identifier string the backend reports.
const MODEL_STYLES: Record<string, { color: string; icon: string }> = {
  bitnet: { color: '#22d3ee', icon: '⚡' },
  anthropic: { color: '#fb923c', icon: '◈' },
  claude: { color: '#fb923c', icon: '◈' },
  openrouter: { color: '#a78bfa', icon: '◇' },
  openai: { color: '#10b981', icon: '◆' },
  gpt: { color: '#10b981', icon: '◆' },
  zcode: { color: '#4ade80', icon: '❖' },
};
const DEFAULT_MODEL_STYLE = { color: '#6ec3ff', icon: '●' };
function modelStyle(model?: string): { color: string; icon: string } {
  if (!model) return DEFAULT_MODEL_STYLE;
  const key = Object.keys(MODEL_STYLES).find((k) =>
    model.toLowerCase().includes(k),
  );
  return key ? MODEL_STYLES[key] : DEFAULT_MODEL_STYLE;
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
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
  // Which transcript entries have their "Thinking" section expanded —
  // collapsed by default since raw_response content can be long.
  const [expandedThinking, setExpandedThinking] = useState<Set<number>>(new Set());
  const [voiceWaveform, setVoiceWaveform] = useState<number[]>(new Array(24).fill(0.1));
  const transcriptRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // In-flight orchestration tracking: /orchestrate blocks until the whole
  // run finishes, so there's no execution_id to cancel-by upfront. We poll
  // /task/list for the newest running task right after firing the request —
  // once found, both Stop and live step progress become available.
  const [isRunning, setIsRunning] = useState(false);
  const [currentTaskId, setCurrentTaskId] = useState<number | null>(null);
  const [currentStepInfo, setCurrentStepInfo] = useState<{
    completed: number;
    total: number;
    lastAction?: string;
  } | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const taskPollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const stopTaskPolling = useCallback(() => {
    if (taskPollRef.current) {
      clearInterval(taskPollRef.current);
      taskPollRef.current = null;
    }
  }, []);

  const handleStop = async () => {
    if (!currentTaskId || cancelling) return;
    setCancelling(true);
    try {
      await (window as any).ozone?.task?.cancel?.(currentTaskId);
    } catch (e) {
      console.warn('Cancel request failed:', e);
    } finally {
      setCancelling(false);
    }
    // The orchestrator stops before its NEXT step once cancellation lands —
    // a step already in flight still runs to completion, so this doesn't
    // resolve instantly. isRunning clears itself when the orchestrate
    // promise in handleSubmit finally settles.
  };

  // Fetch actual emotional state from backend
  const fetchEmotionalState = useCallback(async () => {
    if (!isConnected || !consciousnessEnabled) return;
    
    try {
      const result = await executePipeline(40, { action: 'GetCurrent' });
      if (result?.state) {
        const state = result.state;
        const primaryEmotion = state.primary_emotions?.[0];
        
        setEmotionState({
          primary: primaryEmotion?.emotion || 'neutral',
          intensity: primaryEmotion?.intensity || 0.3,
          secondary: state.primary_emotions?.[1]?.emotion,
          valence: state.valence ?? 0.0,
          arousal: state.arousal ?? 0.3,
        });
      }
    } catch (err) {
      console.warn('Failed to fetch emotional state:', err);
    }
  }, [isConnected, consciousnessEnabled, executePipeline]);

  // Fetch actual I-Loop status from backend (Reflection pipeline #44)
  const fetchILoopStatus = useCallback(async () => {
    if (!isConnected || !consciousnessEnabled) return;
    
    try {
      const result = await executePipeline(44, { action: 'GetILoopStatus' });
      if (result?.i_loop) {
        const loop = result.i_loop;
        setILoopState({
          isActive: loop.current_state?.is_active ?? false,
          currentQuestion: loop.question?.text || 'Reflecting on patterns...',
          questionsAsked: loop.current_state?.questions_asked ?? 0,
          insightsGenerated: loop.current_state?.insights_generated ?? 0,
        });
      }
    } catch (err) {
      console.warn('Failed to fetch I-Loop status:', err);
    }
  }, [isConnected, consciousnessEnabled, executePipeline]);

  // Poll consciousness state when enabled
  useEffect(() => {
    if (!isConnected || !consciousnessEnabled) return;
    
    // Initial fetch
    fetchEmotionalState();
    fetchILoopStatus();
    
    // Poll every 5 seconds for updates
    const interval = setInterval(() => {
      fetchEmotionalState();
      fetchILoopStatus();
    }, 5000);
    
    return () => clearInterval(interval);
  }, [isConnected, consciousnessEnabled, fetchEmotionalState, fetchILoopStatus]);

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

        if (transcribeResult?.transcription && transcribeResult.is_final) {
          setPromptInput((prev: string) => {
            const space = prev && !prev.endsWith(' ') ? ' ' : '';
            return prev + space + transcribeResult.transcription;
          });
        }
      }
    } catch (err) {
      console.error('Voice toggle failed:', err);
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
    } finally {
      // Only set false if we didn't start playing audio
      if (!(window as any).__audioPlaying) {
        setIsSpeaking(false);
      }
    }
  };

  // Handle prompt submission
  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!promptInput.trim() || !isConnected) return;
    
    // Add user message to transcript
    const userEntry: TranscriptEntry = {
      id: Date.now(),
      role: 'user',
      content: promptInput,
      timestamp: Date.now()
    };
    setTranscript(prev => [...prev, userEntry]);
    
    // Use the orchestrator for full 14-stage flow
    try {
      setIsSpeaking(true);
      setIsRunning(true);
      setCurrentTaskId(null);
      setCurrentStepInfo(null);

      // Get current project context from shared state if available
      const currentProjectId = (window as any).ozone?.sharedState?.selectedProjectId;
      const currentWorkspaceId = (window as any).ozone?.sharedState?.selectedWorkspaceId;

      // Use orchestration (full flow) instead of direct pipeline call
      if ((window as any).ozone?.orchestrate) {
        const orchestratePromise = (window as any).ozone.orchestrate({
          prompt: promptInput,
          project_id: currentProjectId,
          workspace_id: currentWorkspaceId,
          user_id: 1,
          device_id: 1,
          consciousness_enabled: consciousnessEnabled,
          token_budget: 100000,
          // Real per-request model selection (Phase 6 routing) — only sent
          // when the user picked something other than the host's default.
          model_config: selectedModel
            ? { model_identifier: selectedModel }
            : undefined,
        });

        // Find the task this request creates so Stop + step progress work
        // while /orchestrate is still blocking on the response.
        let foundTaskId: number | null = null;
        taskPollRef.current = setInterval(async () => {
          try {
            if (!foundTaskId) {
              const list = await (window as any).ozone?.task?.list?.();
              const running = (list?.tasks ?? [])
                .filter((t: any) => String(t.status ?? '').toLowerCase().includes('running')
                  || String(t.status ?? '').toLowerCase().includes('inprogress'))
                .sort((a: any, b: any) => (b.created_at ?? 0) - (a.created_at ?? 0));
              if (running.length > 0) {
                foundTaskId = running[0].task_id;
                setCurrentTaskId(foundTaskId);
                useOzoneStore.getState().setLastTaskId(foundTaskId);
              }
            } else {
              const detail = await (window as any).ozone?.task?.status?.(foundTaskId);
              const steps = detail?.steps ?? [];
              if (steps.length > 0) {
                setCurrentStepInfo({
                  completed: steps.filter((s: any) => s.status === 'completed').length,
                  total: steps.length,
                  lastAction: steps[steps.length - 1]?.action,
                });
              }
            }
          } catch {
            /* transient — keep polling */
          }
        }, 900);

        const result = await orchestratePromise;
        stopTaskPolling();
        setIsRunning(false);
        setCurrentTaskId(null);
        setCurrentStepInfo(null);

        setPromptInput('');
        if (textareaRef.current) {
          textareaRef.current.style.height = 'auto';
        }

        // Add response to transcript
        if (result.response) {
          setTranscript(prev => [...prev, {
            id: Date.now(),
            role: 'assistant',
            content: result.response,
            timestamp: Date.now(),
            emotion: emotionState.primary,
            modelUsed: result.model_used,
            stageCount: Array.isArray(result.stages_completed)
              ? result.stages_completed.length
              : undefined,
            thinkingLog: Array.isArray(result.thinking_log)
              ? result.thinking_log
              : undefined,
          }]);

          // Speak the response if voice output is enabled
          if (consciousnessEnabled) {
            await speakResponse(result.response);
          }
        }
        
        // Update emotion if consciousness provided gate result
        if (result.consciousness_gate?.decision === 'Proceed') {
          setEmotionState(prev => ({
            ...prev,
            primary: 'satisfied',
            intensity: result.consciousness_gate.confidence || 0.8
          }));
        }
        
        // Trigger emotional update based on task success
        if (consciousnessEnabled && result.success) {
          try {
            await executePipeline(40, {
              action: 'ProcessTrigger',
              trigger_type: 'task_success',
              source: 'prompt_completion',
              intensity: 0.6,
            });
            // Refresh emotional state after trigger
            fetchEmotionalState();
          } catch (err) {
            console.warn('Failed to process emotional trigger:', err);
          }
        }
      } else {
        // Fallback: Use store's submitPrompt with response subscription
        const promptText = promptInput;
        setPromptInput('');
        if (textareaRef.current) {
          textareaRef.current.style.height = 'auto';
        }
        
        // Add processing indicator
        const processingId = Date.now();
        setTranscript(prev => [...prev, {
          id: processingId,
          role: 'assistant',
          content: '⏳ Processing...',
          timestamp: Date.now(),
          emotion: emotionState.primary
        }]);
        
        try {
          // Call store's orchestratePrompt which handles the full flow
          const { orchestratePrompt } = useOzoneStore.getState();
          const result = await orchestratePrompt(promptText);
          
          // Replace processing indicator with actual response
          const responseText = result?.response || 'Response received';
          setTranscript(prev => prev.map(entry => 
            entry.id === processingId 
              ? {
                  ...entry,
                  content: responseText,
                  emotion: result?.consciousness_gate?.decision === 'Proceed' ? 'satisfied' : emotionState.primary
                }
              : entry
          ));
          
          // Speak the response if voice output is enabled
          if (consciousnessEnabled && result?.response) {
            await speakResponse(result.response);
          }
          
          // Update emotional state based on success
          if (consciousnessEnabled && result?.success) {
            try {
              await executePipeline(40, {
                action: 'ProcessTrigger',
                trigger_type: 'task_success',
                source: 'prompt_fallback',
                intensity: 0.5,
              });
              fetchEmotionalState();
            } catch (e) {
              console.warn('Failed to process emotional trigger:', e);
            }
          }
        } catch (fallbackErr) {
          // Update processing indicator with error
          setTranscript(prev => prev.map(entry =>
            entry.id === processingId
              ? {
                  ...entry,
                  content: `Error: ${fallbackErr instanceof Error ? fallbackErr.message : 'Request failed'}`,
                  emotion: 'concerned'
                }
              : entry
          ));
        }
      }
    } catch (err) {
      console.error('Prompt orchestration failed:', err);
      setTranscript(prev => [...prev, {
        id: Date.now(),
        role: 'assistant',
        content: `Error: ${err instanceof Error ? err.message : 'Unknown error'}`,
        timestamp: Date.now(),
        emotion: 'concerned'
      }]);
      
      // Trigger negative emotional update
      if (consciousnessEnabled) {
        try {
          await executePipeline(40, {
            action: 'ProcessTrigger',
            trigger_type: 'task_failure',
            source: 'prompt_error',
            intensity: 0.4,
          });
          fetchEmotionalState();
        } catch (e) {
          console.warn('Failed to process emotional trigger:', e);
        }
      }
    } finally {
      setIsSpeaking(false);
      stopTaskPolling();
      setIsRunning(false);
      setCurrentTaskId(null);
      setCurrentStepInfo(null);
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
      <div className={`meta-content ${!consciousnessEnabled ? 'has-overlay' : ''}`}>
        
        {/* Emotion Display */}
        <div className="emotion-display">
          <div 
            className={`emotion-orb ${isSpeaking ? 'speaking' : ''}`}
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

        {/* Transcript */}
        <div className="transcript-section" ref={transcriptRef}>
          <div className="transcript-header">
            <span>💬 Conversation</span>
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
                          style={
                            entry.role === 'assistant' && entry.modelUsed
                              ? { borderLeft: `2px solid ${ms.color}88`, paddingLeft: 10 }
                              : undefined
                          }
                        >
                          {entry.content}
                        </div>
                        {entry.role === 'assistant' && (entry.modelUsed || entry.stageCount) && (
                          <div
                            className="message-meta"
                            style={{ display: 'flex', gap: 6, alignItems: 'center', marginTop: 5 }}
                          >
                            {entry.modelUsed && (
                              <span
                                style={{
                                  border: `1px solid ${ms.color}55`,
                                  color: ms.color,
                                  background: ms.color + '14',
                                  borderRadius: 999,
                                  fontSize: 11,
                                  padding: '2px 8px',
                                  display: 'inline-flex',
                                  gap: 5,
                                  alignItems: 'center',
                                }}
                              >
                                {ms.icon} {entry.modelUsed}
                              </span>
                            )}
                            {entry.stageCount ? (
                              <span style={{ fontSize: 11, opacity: 0.5 }}>{entry.stageCount} stages</span>
                            ) : null}
                            {entry.thinkingLog && entry.thinkingLog.length > 0 && (
                              <button
                                onClick={() => {
                                  setExpandedThinking(prev => {
                                    const next = new Set(prev);
                                    if (next.has(entry.id)) {
                                      next.delete(entry.id);
                                    } else {
                                      next.add(entry.id);
                                    }
                                    return next;
                                  });
                                }}
                                style={{
                                  fontSize: 11,
                                  opacity: 0.6,
                                  background: 'none',
                                  border: '1px solid #223046',
                                  borderRadius: 999,
                                  padding: '2px 8px',
                                  cursor: 'pointer',
                                  color: 'inherit',
                                }}
                              >
                                {expandedThinking.has(entry.id) ? '▾' : '▸'} Thinking ({entry.thinkingLog.length})
                              </button>
                            )}
                          </div>
                        )}
                        {entry.thinkingLog && entry.thinkingLog.length > 0 && expandedThinking.has(entry.id) && (
                          <div
                            className="thinking-log"
                            style={{
                              marginTop: 8,
                              display: 'flex',
                              flexDirection: 'column',
                              gap: 6,
                            }}
                          >
                            {entry.thinkingLog.map((t, i) => (
                              <div
                                key={i}
                                style={{
                                  border: '1px solid #223046',
                                  borderRadius: 8,
                                  padding: '8px 10px',
                                  fontSize: 12,
                                  background: 'rgba(255,255,255,0.02)',
                                }}
                              >
                                <div
                                  style={{
                                    display: 'flex',
                                    justifyContent: 'space-between',
                                    gap: 8,
                                    opacity: 0.7,
                                    marginBottom: 4,
                                    fontSize: 11,
                                  }}
                                >
                                  <span>{t.stage}</span>
                                  <span>
                                    {t.tokens_used != null ? `${t.tokens_used} tok` : ''}
                                    {t.eval_tokens_per_sec != null
                                      ? ` · ${t.eval_tokens_per_sec.toFixed(1)} tok/s`
                                      : ''}
                                  </span>
                                </div>
                                <div style={{ whiteSpace: 'pre-wrap', opacity: 0.85 }}>
                                  {t.raw_response}
                                </div>
                              </div>
                            ))}
                          </div>
                        )}
                      </div>
                    </React.Fragment>
                  );
                });
              })()}
            </div>
          )}
        </div>

        {/* I-Loop Status (when consciousness enabled) */}
        {consciousnessEnabled && (
          <div className="iloop-section">
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
              <span className="reflection-text">
                "{iLoopState.currentQuestion || 'Waiting for next reflection cycle...'}"
              </span>
            </div>
          </div>
        )}

        {/* Consciousness Disabled Overlay - TRANSPARENT */}
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

      {/* Prompt Input - Always visible; voice lives HERE, in the chat */}
      <div className="meta-prompt">
        {isRunning && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              fontSize: 11.5,
              color: '#8b98ab',
              padding: '2px 4px 6px',
            }}
          >
            <span
              style={{
                width: 6,
                height: 6,
                borderRadius: '50%',
                background: '#6ec3ff',
                animation: 'pulse 1.2s ease-in-out infinite',
                flex: 'none',
              }}
            />
            {currentStepInfo ? (
              <span>
                Step {currentStepInfo.completed + 1} of {currentStepInfo.total}
                {currentStepInfo.lastAction ? ` — ${currentStepInfo.lastAction}` : ''}
              </span>
            ) : (
              <span>Starting…</span>
            )}
          </div>
        )}
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
                  color: '#8b98ab',
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
