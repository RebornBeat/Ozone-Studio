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

interface TranscriptEntry {
  id: number;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
  emotion?: string;
}

export function MetaPortion({ width }: MetaPortionProps) {
  const {
    consciousnessEnabled,
    isConnected,
    promptInput,
    setPromptInput,
    submitPrompt,
    executePipeline,
  } = useOzoneStore();
  
  const [voiceActive, setVoiceActive] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
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
  const [voiceWaveform, setVoiceWaveform] = useState<number[]>(new Array(24).fill(0.1));
  const transcriptRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

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

  // Voice waveform animation
  useEffect(() => {
    if (isSpeaking || voiceActive) {
      const interval = setInterval(() => {
        setVoiceWaveform(prev => 
          prev.map(() => 0.1 + Math.random() * 0.9)
        );
      }, 80);
      return () => clearInterval(interval);
    } else {
      setVoiceWaveform(new Array(24).fill(0.15));
    }
  }, [isSpeaking, voiceActive]);

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

  // Toggle voice input with proper transcription capture
  const toggleVoice = async () => {
    if (!isConnected) return;
    
    try {
      if (!voiceActive) {
        // Start listening
        const result = await executePipeline(10, { action: 'StartListening' });
        setVoiceActive(true);
        
        // Set up voice activity polling
        const pollInterval = setInterval(async () => {
          try {
            // Check for audio data from microphone (this would come from browser API)
            const audioData = await captureAudioFromMicrophone();
            if (audioData) {
              // Process audio to get transcription
              const transcribeResult = await executePipeline(10, {
                action: 'ProcessAudio',
                audio_base64: audioData.base64,
                format: audioData.format || 'webm',
              });
              
              if (transcribeResult?.transcription && transcribeResult.is_final) {
                // Add transcribed text to prompt input
                setPromptInput(prev => {
                  const space = prev && !prev.endsWith(' ') ? ' ' : '';
                  return prev + space + transcribeResult.transcription;
                });
              }
            }
          } catch (e) {
            console.warn('Voice processing error:', e);
          }
        }, 500); // Poll every 500ms
        
        // Store interval ID for cleanup
        (window as any).__voicePollInterval = pollInterval;
        
      } else {
        // Stop listening
        await executePipeline(10, { action: 'StopListening' });
        setVoiceActive(false);
        
        // Clear polling interval
        if ((window as any).__voicePollInterval) {
          clearInterval((window as any).__voicePollInterval);
          delete (window as any).__voicePollInterval;
        }
      }
    } catch (err) {
      console.error('Voice toggle failed:', err);
      setVoiceActive(false);
    }
  };
  
  // Capture audio from browser microphone API
  const captureAudioFromMicrophone = async (): Promise<{ base64: string; format: string } | null> => {
    // This integrates with the browser's MediaRecorder API
    // The actual implementation would capture audio chunks
    const mediaRecorder = (window as any).__mediaRecorder;
    if (mediaRecorder && mediaRecorder.audioChunks?.length > 0) {
      const audioBlob = new Blob(mediaRecorder.audioChunks, { type: 'audio/webm' });
      mediaRecorder.audioChunks = []; // Clear processed chunks
      
      // Convert to base64
      const reader = new FileReader();
      return new Promise((resolve) => {
        reader.onloadend = () => {
          const base64 = (reader.result as string)?.split(',')[1];
          resolve(base64 ? { base64, format: 'webm' } : null);
        };
        reader.readAsDataURL(audioBlob);
      });
    }
    return null;
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
      
      // Get current project context from shared state if available
      const currentProjectId = (window as any).ozone?.sharedState?.selectedProjectId;
      const currentWorkspaceId = (window as any).ozone?.sharedState?.selectedWorkspaceId;
      
      // Use orchestration (full flow) instead of direct pipeline call
      if ((window as any).ozone?.orchestrate) {
        const result = await (window as any).ozone.orchestrate({
          prompt: promptInput,
          project_id: currentProjectId,
          workspace_id: currentWorkspaceId,
          user_id: 1,
          device_id: 1,
          consciousness_enabled: consciousnessEnabled,
          token_budget: 100000,
        });
        
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
            emotion: emotionState.primary
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

        {/* Voice Visualization */}
        <div className={`voice-section ${isSpeaking ? 'speaking' : voiceActive ? 'listening' : ''}`}>
          <div className="voice-header">
            <span className="voice-status">
              {isSpeaking ? '🔊 Speaking' : voiceActive ? '🎤 Listening...' : '🔇 Voice Ready'}
            </span>
          </div>
          <div className="waveform-container">
            {voiceWaveform.map((height, i) => (
              <div 
                key={i} 
                className="wave-bar" 
                style={{ 
                  height: `${height * 50}px`,
                  backgroundColor: isSpeaking 
                    ? emotionColor 
                    : voiceActive ? '#4ade80' : '#374151',
                  animationDelay: `${i * 0.05}s`
                }}
              />
            ))}
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
              {transcript.map((entry) => (
                <div key={entry.id} className={`message ${entry.role}`}>
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
                  <div className="message-content">{entry.content}</div>
                </div>
              ))}
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

      {/* Prompt Input - Always visible */}
      <div className="meta-prompt">
        <form onSubmit={handleSubmit}>
          <div className="prompt-input-wrapper">
            <textarea
              ref={textareaRef}
              value={promptInput}
              onChange={handleTextareaChange}
              placeholder={isConnected ? "Chat with OZONE..." : "Connecting to backend..."}
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
            <button
              type="button"
              className={`control-btn voice-btn ${voiceActive ? 'active' : ''}`}
              onClick={toggleVoice}
              disabled={!isConnected}
              title={voiceActive ? 'Stop recording' : 'Voice input'}
            >
              {voiceActive ? '🔴' : '🎤'}
            </button>
            <button 
              type="submit" 
              className="control-btn send-btn"
              disabled={!isConnected || !promptInput.trim()}
              title="Send message"
            >
              <span className="send-arrow">→</span>
            </button>
          </div>
        </form>
      </div>
    </aside>
  );
}
