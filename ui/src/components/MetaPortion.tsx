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

import React, { useState, useEffect, useRef } from 'react';
import { useOzoneStore } from '../services/store';

interface MetaPortionProps {
  width: number;
}

interface EmotionState {
  primary: string;
  intensity: number;
  secondary?: string;
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
    primary: 'curious',
    intensity: 0.6,
    secondary: 'focused'
  });
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
  const [voiceWaveform, setVoiceWaveform] = useState<number[]>(new Array(24).fill(0.1));
  const transcriptRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Simulate voice waveform animation
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

  // Toggle voice input
  const toggleVoice = async () => {
    if (!isConnected) return;
    
    try {
      if (!voiceActive) {
        await executePipeline(10, { action: 'start' });
        setVoiceActive(true);
      } else {
        await executePipeline(10, { action: 'stop' });
        setVoiceActive(false);
      }
    } catch (err) {
      console.error('Voice toggle failed:', err);
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
    
    // Execute prompt pipeline
    try {
      await executePipeline(9, { prompt: promptInput });
      setPromptInput('');
      if (textareaRef.current) {
        textareaRef.current.style.height = 'auto';
      }
      
      // Simulate response (in real implementation, this comes from backend)
      setIsSpeaking(true);
      setTimeout(() => {
        setTranscript(prev => [...prev, {
          id: Date.now(),
          role: 'assistant',
          content: 'Processing your request...',
          timestamp: Date.now(),
          emotion: emotionState.primary
        }]);
        setIsSpeaking(false);
      }, 1500);
    } catch (err) {
      console.error('Prompt submission failed:', err);
    }
  };

  // Get emotion color
  const getEmotionColor = (emotion: string): string => {
    const colors: Record<string, string> = {
      curious: '#3b82f6',
      focused: '#06b6d4',
      happy: '#4ade80',
      satisfied: '#22c55e',
      concerned: '#f59e0b',
      thoughtful: '#8b5cf6',
      calm: '#6366f1',
      excited: '#ec4899',
      neutral: '#64748b'
    };
    return colors[emotion] || colors.neutral;
  };

  // Get emotion emoji
  const getEmotionEmoji = (emotion: string): string => {
    const emojis: Record<string, string> = {
      curious: '🤔',
      focused: '🎯',
      happy: '😊',
      satisfied: '😌',
      concerned: '😟',
      thoughtful: '💭',
      calm: '😌',
      excited: '✨',
      neutral: '🧠'
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
              <span className="iloop-dot" style={{ backgroundColor: emotionColor }} />
              <span className="iloop-title">I-Loop Active</span>
            </div>
            <div className="iloop-reflection">
              <span className="reflection-label">Current reflection:</span>
              <span className="reflection-text">"What patterns am I recognizing?"</span>
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
