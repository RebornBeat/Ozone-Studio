// useVoiceIO — voice capture (STT) + speech (TTS) via the voice pipeline (#10),
// extracted from MetaPortion so project chat can reuse the exact same logic.
//
// Behaviour is a faithful copy of MetaPortion's: 16 kHz mono PCM16 WAV capture,
// RMS level meter, `ProcessAudio` transcription on stop, `Speak` gated on
// consciousness enabled + `output_enabled`. One honesty change: when the
// pipeline reports `success:false` (no TTS engine installed) the error is
// surfaced in `voiceError` instead of silently playing nothing.
//
// MetaPortion is NOT modified by this extraction; adopting the hook there is a
// separate follow-up so the two chats can't diverge by accident.

import { useCallback, useEffect, useRef, useState } from 'react';

export type ExecutePipeline = (pipelineId: number, input: Record<string, any>) => Promise<any>;

export interface UseVoiceIOOptions {
  isConnected: boolean;
  executePipeline: ExecutePipeline;
  /** speakResponse is a no-op unless consciousness is enabled (MetaPortion rule). */
  consciousnessEnabled: boolean;
  /** Called with the final transcript when a capture is transcribed. */
  onTranscript: (text: string) => void;
}

export interface UseVoiceIO {
  voiceActive: boolean;
  isSpeaking: boolean;
  /** 0..1 live RMS level while capturing, 0 otherwise. */
  micLevel: number;
  /** Last voice failure (capture, STT, or TTS engine); null when clear. */
  voiceError: string | null;
  toggleVoice: () => Promise<void>;
  speakResponse: (text: string) => Promise<void>;
  clearVoiceError: () => void;
}

interface Capture {
  ctx: AudioContext;
  stream: MediaStream;
  processor: ScriptProcessorNode;
  chunks: Float32Array[];
}

export function useVoiceIO(opts: UseVoiceIOOptions): UseVoiceIO {
  const { isConnected, executePipeline, consciousnessEnabled, onTranscript } = opts;
  const [voiceActive, setVoiceActive] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [micLevel, setMicLevel] = useState(0);
  const [voiceError, setVoiceError] = useState<string | null>(null);
  const rafRef = useRef<number | null>(null);
  const captureRef = useRef<Capture | null>(null);
  const audioPlayingRef = useRef(false);

  const startCapture = useCallback(async () => {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    const ctx = new AudioContext({ sampleRate: 16000 });
    const source = ctx.createMediaStreamSource(stream);
    const processor = ctx.createScriptProcessor(4096, 1, 1);
    const chunks: Float32Array[] = [];

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
    const silent = ctx.createGain();
    silent.gain.value = 0;
    processor.connect(silent);
    silent.connect(ctx.destination);

    captureRef.current = { ctx, stream, processor, chunks };
  }, []);

  const encodeWav = useCallback(async (): Promise<string | null> => {
    const capture = captureRef.current;
    if (!capture || capture.chunks.length === 0) return null;
    const total = capture.chunks.reduce((a, c) => a + c.length, 0);
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
    view.setUint16(20, 1, true);
    view.setUint16(22, 1, true);
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, sampleRate * 2, true);
    view.setUint16(32, 2, true);
    view.setUint16(34, 16, true);
    writeStr(36, 'data');
    view.setUint32(40, pcm16.byteLength, true);

    const blob = new Blob([header, pcm16.buffer], { type: 'audio/wav' });
    const reader = new FileReader();
    reader.readAsDataURL(blob);
    return new Promise<string | null>((resolve) => {
      reader.onloadend = () => {
        const b64 = (reader.result as string)?.split(',')[1];
        resolve(b64 || null);
      };
    });
  }, []);

  const stopCapture = useCallback(async (): Promise<string | null> => {
    const capture = captureRef.current;
    if (!capture) return null;
    const wav = await encodeWav();
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    setMicLevel(0);
    try {
      capture.processor.disconnect();
      capture.stream.getTracks().forEach((t) => t.stop());
      await capture.ctx.close();
    } catch (e) {
      console.warn('capture cleanup:', e);
    }
    captureRef.current = null;
    return wav;
  }, [encodeWav]);

  const toggleVoice = useCallback(async () => {
    if (!isConnected) return;
    try {
      if (!voiceActive) {
        setVoiceError(null);
        await executePipeline(10, { action: 'StartListening' });
        await startCapture();
        setVoiceActive(true);
      } else {
        setVoiceActive(false);
        const wav = await stopCapture();
        await executePipeline(10, { action: 'StopListening' });
        if (!wav) return;
        const res = await executePipeline(10, {
          action: 'ProcessAudio',
          audio_base64: wav,
          format: 'wav',
        });
        if (res?.success === false && res?.error) {
          setVoiceError(`Transcription failed: ${res.error}`);
        } else if (res?.transcription && res.is_final) {
          onTranscript(res.transcription);
        }
      }
    } catch (err: any) {
      console.error('Voice toggle failed:', err);
      setVoiceError(`Voice capture failed: ${err?.message ?? String(err)}`);
      try {
        await stopCapture();
      } catch {
        /* already gone */
      }
      setVoiceActive(false);
    }
  }, [isConnected, voiceActive, executePipeline, startCapture, stopCapture, onTranscript]);

  const speakResponse = useCallback(
    async (text: string) => {
      if (!consciousnessEnabled) return;
      try {
        setIsSpeaking(true);
        const settingsResult = await executePipeline(10, { action: 'GetSettings' });
        const settings = settingsResult?.settings;
        if (!settings?.output_enabled) {
          setIsSpeaking(false);
          return;
        }

        let voiceStyle: string = settings?.output_voice || 'default';
        let speechRate: number = settings?.output_speed || 1.0;
        try {
          const identity = await executePipeline(43, { action: 'GetVoice' });
          const v = identity?.voice;
          if (v) {
            if (v.warmth > 0.6) voiceStyle = 'warm';
            else if (v.warmth < 0.4) voiceStyle = 'neutral';
            if (v.formality > 0.7) speechRate = 0.9;
            else if (v.formality < 0.3) speechRate = 1.1;
          }
        } catch (e) {
          console.warn('Could not get voice identity:', e);
        }

        const result = await executePipeline(10, {
          action: 'Speak',
          text,
          voice: voiceStyle,
          speed: speechRate,
        });

        if (result?.success === false) {
          // Honest TTS failure (no engine installed, etc.) — surface it.
          setVoiceError(`Speech unavailable: ${result.error ?? 'TTS failed'}`);
          return;
        }
        if (result?.audio_base64) {
          const audio = new Audio(`data:audio/wav;base64,${result.audio_base64}`);
          audioPlayingRef.current = true;
          const done = () => {
            setIsSpeaking(false);
            audioPlayingRef.current = false;
          };
          audio.onended = done;
          audio.onerror = () => {
            setVoiceError('Speech playback failed (audio could not be decoded)');
            done();
          };
          await audio.play();
          return; // isSpeaking cleared by onended/onerror
        }
      } catch (err: any) {
        console.warn('Speech failed:', err);
        setVoiceError(`Speech failed: ${err?.message ?? String(err)}`);
      } finally {
        if (!audioPlayingRef.current) setIsSpeaking(false);
      }
    },
    [consciousnessEnabled, executePipeline],
  );

  useEffect(
    () => () => {
      if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
      const c = captureRef.current;
      if (c) {
        try {
          c.stream.getTracks().forEach((t) => t.stop());
          void c.ctx.close();
        } catch {
          /* ignore */
        }
      }
    },
    [],
  );

  return {
    voiceActive,
    isSpeaking,
    micLevel,
    voiceError,
    toggleVoice,
    speakResponse,
    clearVoiceError: () => setVoiceError(null),
  };
}
