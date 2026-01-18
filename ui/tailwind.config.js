/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/renderer/**/*.{js,ts,jsx,tsx}",
    "./src/renderer/index.html",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Primary brand colors
        ozone: {
          50: '#f0f9ff',
          100: '#e0f2fe',
          200: '#bae6fd',
          300: '#7dd3fc',
          400: '#38bdf8',
          500: '#0ea5e9',
          600: '#0284c7',
          700: '#0369a1',
          800: '#075985',
          900: '#0c4a6e',
          950: '#082f49',
        },
        // Consciousness/awareness colors
        conscious: {
          50: '#fdf4ff',
          100: '#fae8ff',
          200: '#f5d0fe',
          300: '#f0abfc',
          400: '#e879f9',
          500: '#d946ef',
          600: '#c026d3',
          700: '#a21caf',
          800: '#86198f',
          900: '#701a75',
          950: '#4a044e',
        },
        // Background colors
        surface: {
          light: '#ffffff',
          DEFAULT: '#f8fafc',
          dark: '#0f172a',
          darker: '#020617',
        },
        // Emotional state colors
        emotion: {
          joy: '#fbbf24',       // Amber
          trust: '#34d399',     // Emerald
          fear: '#a78bfa',      // Violet
          surprise: '#f472b6',  // Pink
          sadness: '#60a5fa',   // Blue
          disgust: '#a3e635',   // Lime
          anger: '#f87171',     // Red
          anticipation: '#fb923c', // Orange
          curiosity: '#22d3ee', // Cyan
          contentment: '#86efac', // Green
          frustration: '#fca5a5', // Light red
        },
        // Connection/network colors
        network: {
          connected: '#22c55e',
          syncing: '#eab308',
          disconnected: '#ef4444',
          limited: '#f97316',
        },
        // Task status colors
        task: {
          queued: '#94a3b8',
          running: '#3b82f6',
          completed: '#22c55e',
          failed: '#ef4444',
          paused: '#f59e0b',
          cancelled: '#6b7280',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
      fontSize: {
        'xxs': '0.625rem',
      },
      spacing: {
        '18': '4.5rem',
        '88': '22rem',
        '128': '32rem',
      },
      borderRadius: {
        '4xl': '2rem',
      },
      boxShadow: {
        'glow': '0 0 20px rgba(14, 165, 233, 0.3)',
        'glow-lg': '0 0 40px rgba(14, 165, 233, 0.4)',
        'consciousness': '0 0 30px rgba(217, 70, 239, 0.3)',
        'inner-glow': 'inset 0 0 20px rgba(14, 165, 233, 0.2)',
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'pulse-consciousness': 'pulseConsciousness 4s ease-in-out infinite',
        'float': 'float 6s ease-in-out infinite',
        'glow': 'glow 2s ease-in-out infinite',
        'spin-slow': 'spin 8s linear infinite',
        'bounce-slow': 'bounce 3s ease-in-out infinite',
        'fade-in': 'fadeIn 0.3s ease-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'slide-down': 'slideDown 0.3s ease-out',
        'slide-left': 'slideLeft 0.3s ease-out',
        'slide-right': 'slideRight 0.3s ease-out',
        'scale-in': 'scaleIn 0.2s ease-out',
        'progress': 'progress 2s ease-in-out infinite',
      },
      keyframes: {
        pulseConsciousness: {
          '0%, 100%': { 
            boxShadow: '0 0 20px rgba(217, 70, 239, 0.3)',
            transform: 'scale(1)',
          },
          '50%': { 
            boxShadow: '0 0 40px rgba(217, 70, 239, 0.5)',
            transform: 'scale(1.02)',
          },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-10px)' },
        },
        glow: {
          '0%, 100%': { boxShadow: '0 0 20px rgba(14, 165, 233, 0.3)' },
          '50%': { boxShadow: '0 0 40px rgba(14, 165, 233, 0.6)' },
        },
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideDown: {
          '0%': { transform: 'translateY(-10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideLeft: {
          '0%': { transform: 'translateX(10px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
        slideRight: {
          '0%': { transform: 'translateX(-10px)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
        scaleIn: {
          '0%': { transform: 'scale(0.95)', opacity: '0' },
          '100%': { transform: 'scale(1)', opacity: '1' },
        },
        progress: {
          '0%': { backgroundPosition: '200% 0' },
          '100%': { backgroundPosition: '-200% 0' },
        },
      },
      backgroundImage: {
        'gradient-radial': 'radial-gradient(var(--tw-gradient-stops))',
        'gradient-conic': 'conic-gradient(from 180deg at 50% 50%, var(--tw-gradient-stops))',
        'consciousness-gradient': 'linear-gradient(135deg, rgba(217, 70, 239, 0.1), rgba(14, 165, 233, 0.1))',
        'loading-shimmer': 'linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent)',
      },
      backdropBlur: {
        xs: '2px',
      },
      transitionDuration: {
        '400': '400ms',
      },
      zIndex: {
        '60': '60',
        '70': '70',
        '80': '80',
        '90': '90',
        '100': '100',
      },
    },
  },
  plugins: [
    // Custom plugin for emotion indicator utilities
    function({ addUtilities, theme }) {
      const emotionColors = theme('colors.emotion');
      const newUtilities = {};
      
      Object.entries(emotionColors).forEach(([name, color]) => {
        newUtilities[`.emotion-indicator-${name}`] = {
          backgroundColor: color,
          boxShadow: `0 0 10px ${color}`,
        };
        newUtilities[`.emotion-border-${name}`] = {
          borderColor: color,
          boxShadow: `0 0 5px ${color}`,
        };
        newUtilities[`.emotion-text-${name}`] = {
          color: color,
        };
      });
      
      addUtilities(newUtilities);
    },
    // Custom plugin for glass morphism
    function({ addUtilities }) {
      addUtilities({
        '.glass': {
          backgroundColor: 'rgba(255, 255, 255, 0.1)',
          backdropFilter: 'blur(10px)',
          border: '1px solid rgba(255, 255, 255, 0.1)',
        },
        '.glass-dark': {
          backgroundColor: 'rgba(0, 0, 0, 0.3)',
          backdropFilter: 'blur(10px)',
          border: '1px solid rgba(255, 255, 255, 0.05)',
        },
        '.glass-ozone': {
          backgroundColor: 'rgba(14, 165, 233, 0.1)',
          backdropFilter: 'blur(10px)',
          border: '1px solid rgba(14, 165, 233, 0.2)',
        },
        '.glass-consciousness': {
          backgroundColor: 'rgba(217, 70, 239, 0.1)',
          backdropFilter: 'blur(10px)',
          border: '1px solid rgba(217, 70, 239, 0.2)',
        },
      });
    },
    // Custom plugin for scrollbar styling
    function({ addUtilities }) {
      addUtilities({
        '.scrollbar-thin': {
          scrollbarWidth: 'thin',
          '&::-webkit-scrollbar': {
            width: '6px',
            height: '6px',
          },
        },
        '.scrollbar-ozone': {
          '&::-webkit-scrollbar-track': {
            backgroundColor: 'rgba(15, 23, 42, 0.3)',
            borderRadius: '3px',
          },
          '&::-webkit-scrollbar-thumb': {
            backgroundColor: 'rgba(14, 165, 233, 0.5)',
            borderRadius: '3px',
            '&:hover': {
              backgroundColor: 'rgba(14, 165, 233, 0.7)',
            },
          },
        },
        '.scrollbar-hide': {
          scrollbarWidth: 'none',
          '&::-webkit-scrollbar': {
            display: 'none',
          },
        },
      });
    },
  ],
};
