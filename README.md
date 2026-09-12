# KAI (AI Desktop Assistant)

KAI is an advanced, privacy-first AI desktop assistant built with Tauri, React, Typescript, and Rust. It is designed to be your local, highly capable co-pilot, seamlessly integrating with your operating system, applications, and web services.

## About This Project

KAI goes beyond simple text generation by combining local LLM capabilities with native desktop APIs to see, hear, and interact with your computer just like you do. 

### Core Capabilities

- **Desktop Automation & Computer Use**: KAI can take control of your mouse and keyboard, manage your windows, open applications, and control media playback using native OS tools.
- **Vision & Perception**: Using local vision models (like LLaVA), KAI can take screenshots of your desktop or pictures from your webcam to understand what is on your screen or in your environment.
- **Voice Interaction**: Features Push-To-Talk (PTT) with global shortcuts, continuous Voice Activity Detection (VAD), audio transcription, and local, fast Text-to-Speech (TTS) using Piper.
- **Browser Automation**: Equipped with a headless Chrome instance to autonomously navigate the web, click elements, type into forms, and scrape information.
- **Long-term Memory**: Built-in SQLite database provides KAI with episodic and semantic memory, allowing it to remember facts about you and recall past conversations across sessions.
- **Python Sandboxing**: KAI can execute Python code dynamically in a secure `bwrap` sandbox for data analysis, math, and external API calls.
- **Integrations**: Includes specialized Python clients to read your Google Calendar events, fetch emails, and perform web searches.

## Tech Stack
- **Frontend**: React, TypeScript, Vite
- **Backend**: Rust (Tauri), SQLite (Memory)
- **Local AI**: Ollama (LLM & Vision inference), Piper (TTS), WebRTC (VAD)
- **Automation**: Enigo (Input), Headless Chrome, Playerctl, Wmctrl
- **Scripts**: Python (Calendar, Email, Web Search, Sandboxed Execution)

## Getting Started

### Prerequisites
- Node.js
- Rust (latest stable)
- Python 3
- Local Dependencies: `wmctrl`, `playerctl`, `bwrap` (Bubblewrap), `yt-dlp`, `mpv`
- Ollama (ensure you have run `ollama pull llava` for vision features)

### Installation

1. Install Node dependencies:
   ```bash
   npm install
   ```

2. Run the application in development mode:
   ```bash
   npm run tauri dev
   ```

### Usage
- Use **Ctrl + Space** to toggle the KAI window from anywhere.
- Hold **Alt + Space** to activate Push-To-Talk and speak to KAI directly.
- The app runs in your System Tray where you can quit or access options.

### Building for Production

```bash
npm run tauri build
```
