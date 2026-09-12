import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './App.css';

interface ToolCallFunction {
  name: string;
  arguments: any;
}

interface ToolCall {
  function: ToolCallFunction;
}

declare global {
  interface Window {
    currentStreamId?: number;
  }
}

interface Message {
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  images?: string[];
  tool_calls?: ToolCall[];
}

interface ChatMessageDisplay extends Message {
  id: number;
}

interface DbMessage {
  id: number;
  message_json: string;
}

function App() {
  const [messages, setMessages] = useState<ChatMessageDisplay[]>([]);
  const [inputText, setInputText] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  
  const [isVoiceEnabled, setIsVoiceEnabledState] = useState(false);
  const isVoiceEnabledRef = useRef(false);
  const setIsVoiceEnabled = (enabled: boolean) => {
     setIsVoiceEnabledState(enabled);
     isVoiceEnabledRef.current = enabled;
  };
  const [isPttActive, setIsPttActive] = useState(false);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const pttChunksRef = useRef<Blob[]>([]);
  const chatEndRef = useRef<HTMLDivElement>(null);
  const sentenceBufferRef = useRef('');
  
  const messagesRef = useRef<ChatMessageDisplay[]>([]);

  const [isAwake, setIsAwakeState] = useState(false);
  const isAwakeRef = useRef(false);
  const awakeTimeoutRef = useRef<number | null>(null);

  const isSpeakingRef = useRef(false);

  const setIsAwake = (awake: boolean) => {
     setIsAwakeState(awake);
     isAwakeRef.current = awake;
  };

  const extendAwake = () => {
     if (awakeTimeoutRef.current) window.clearTimeout(awakeTimeoutRef.current);
     awakeTimeoutRef.current = window.setTimeout(() => {
         setIsAwake(false);
         console.log("KAI went back to sleep");
     }, 300000); // 5 minutes
  };

  useEffect(() => {
    messagesRef.current = messages;
  }, [messages]);

  const speak = async (text: string) => {
    if (!isVoiceEnabledRef.current) return;
    try {
      const cleanText = text.replace(/[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]/gu, '')
                            .replace(/[*_#`]/g, '');
      await invoke('speak_neural', { text: cleanText });
    } catch (e) {
      console.error("Neural TTS failed, falling back to OS TTS", e);
      if (!window.speechSynthesis) return;
      const utterance = new SpeechSynthesisUtterance(text);
      utterance.rate = 1.0;
      window.speechSynthesis.speak(utterance);
    }
  };

  useEffect(() => {
    loadHistory();
    
    const unlisten = listen<string>('llm-token', (event) => {
      setMessages(prev => {
         const lastMsg = prev[prev.length - 1];
         if (lastMsg && lastMsg.role === 'assistant' && lastMsg.id === window.currentStreamId) {
             const updated = [...prev];
             updated[updated.length - 1] = { ...lastMsg, content: lastMsg.content + event.payload };
             return updated;
         }
         return prev;
      });
      
      if (isVoiceEnabledRef.current) {
        sentenceBufferRef.current += event.payload;
        // Chunk aggressively on punctuation for lower latency Piper TTS
        if (/[.!?,:;]\s/.test(sentenceBufferRef.current) || sentenceBufferRef.current.endsWith('\n')) {
            const sentence = sentenceBufferRef.current.trim();
            if (sentence.length > 0) {
                const cleanText = sentence.replace(/[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]/gu, '')
                                .replace(/[*_#`]/g, '');
                invoke('speak_sentence', { text: cleanText }).catch(console.error);
            }
            sentenceBufferRef.current = '';
        }
      }
    });

    const unlistenVad = listen<string>('vad_speech_transcribed', (event) => {
      if (isSpeakingRef.current) {
          console.log("Ignored background speech because KAI is currently speaking.");
          return;
      }
      const transcript = event.payload;
      
      // Filter out common Whisper hallucinations for silence/static
      if (/^(thank you|thanks for watching|subscribe|thanks|you|hi|hello)\.?$/i.test(transcript.trim())) {
          return;
      }

      const clean = transcript.toLowerCase().replace(/[^a-z0-9 ]/g, '');
      
      // Removed overly generic English words (hi, hey, high, tie, k) that cause false positives from background noise
      const isWakeWord = /\b(kai|kay|ky|cai|guy|kite|kyle|chi|sky|chay)\b/i.test(clean) || clean.includes('kai');
      
      if (isWakeWord) {
        setIsAwake(true);
        extendAwake();
        
        let cleanedTranscript = transcript.replace(/\b(hey|hello|hi|good\s*morning|good\s*night|okay|ok)?\s*(kai|kay|ky|cai|guy|kite|kyle|chi|tie|high|sky|chay|k)\b/gi, '').trim();
        cleanedTranscript = cleanedTranscript.replace(/^[,.?! ]+/, '');

        if (cleanedTranscript.length > 0) {
            submitMessage(cleanedTranscript);
        } else {
            speak("Yes?");
        }
      } else if (isAwakeRef.current) {
        extendAwake();
        submitMessage(transcript);
      } else {
        console.log("Ignored background speech:", transcript);
      }
    });

    invoke('start_continuous_vad');
    setIsVoiceEnabled(true);

    const unlistenPttStart = listen('ptt-start', async () => {
       try {
           setIsPttActive(true);
           const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
           const recorder = new MediaRecorder(stream);
           pttChunksRef.current = [];
           recorder.ondataavailable = (e) => {
               if (e.data.size > 0) pttChunksRef.current.push(e.data);
           };
           recorder.onstop = async () => {
               const blob = new Blob(pttChunksRef.current, { type: 'audio/webm' });
               const buffer = await blob.arrayBuffer();
               const base64 = btoa(new Uint8Array(buffer).reduce((data, byte) => data + String.fromCharCode(byte), ''));
               stream.getTracks().forEach(track => track.stop());
               
               try {
                   setIsLoading(true);
                   const transcript = await invoke<string>('transcribe_audio', { audioBase64: base64 });
                   setIsLoading(false);
                   if (transcript.trim().length > 0) {
                       // Awaken KAI and run loop
                       setIsAwake(true);
                       extendAwake();
                       submitMessage(transcript);
                   }
               } catch (e) {
                   console.error("PTT Transcribe error:", e);
                   setIsLoading(false);
               }
           };
           recorder.start();
           mediaRecorderRef.current = recorder;
       } catch (err) {
           console.error("Failed to start PTT:", err);
           setIsPttActive(false);
       }
    });

    const unlistenPttStop = listen('ptt-stop', () => {
       setIsPttActive(false);
       if (mediaRecorderRef.current && mediaRecorderRef.current.state === 'recording') {
           mediaRecorderRef.current.stop();
       }
    });

    return () => {
      unlisten.then(f => f());
      unlistenVad.then(f => f());
      unlistenPttStart.then(f => f());
      unlistenPttStop.then(f => f());
    }
  }, []);

  const loadHistory = async () => {
    try {
      const coreMemories = await invoke<string[]>('get_core_memories');
      const memoryString = coreMemories.length > 0 ? `\n\nCORE MEMORIES ABOUT THE USER:\n${coreMemories.map(m => `- ${m}`).join('\n')}` : '';
      
      const history = await invoke<DbMessage[]>('get_history');
      if (history.length > 0) {
        const loadedMessages = history.map(dbMsg => {
          const msg: Message = JSON.parse(dbMsg.message_json);
          return { id: dbMsg.id, ...msg };
        });
        setMessages(loadedMessages);
      } else {
        const sysMsg: ChatMessageDisplay = { id: 1, role: 'system', content: `You are KAI, a helpful autonomous AI companion running locally on the user's laptop. \n\nYou have access to various tools to interact with the OS.\n- Use the browser_action tool to autonomously browse the web. You can navigate, click, type, and read web pages to accomplish user goals. Keep the browser open across multiple tool calls if needed, and close it when done.\n- Use run_python_code to execute python code to solve math, write scripts, or interact with APIs. You MUST print() output.\n- Use capture_webcam to see the user or their physical environment.\n- Use read_emails to check the user's configured inboxes (specify account_alias or 'all').\n- Use read_calendar to check upcoming Google Calendar events.\n- Use remember_fact to autonomously save important facts about the user.\n- Use recall_memory to search your semantic memory database for past context.\n- If the user asks you to do a task, MUST IMMEDIATELY use the appropriate tools. NEVER ask for permission.\n- If a tool returns an error or something fails, you MUST explain the issue to the user naturally. If you can fix it yourself using your tools, you MUST do so immediately without asking for help.\nCRITICAL: If the user asks you to play music, play a song, or play an artist, YOU MUST ALWAYS use the play_song tool. ONLY use the control_media tool if they explicitly ask to pause, skip, or resume an already playing media player.${memoryString}` };
        const initialMsg: ChatMessageDisplay = { id: 2, role: 'assistant', content: 'Hello! I am KAI. Your local AI assistant is ready.' };
        setMessages([sysMsg, initialMsg]);
        await invoke('save_message', { messageJson: JSON.stringify(sysMsg) });
        await invoke('save_message', { messageJson: JSON.stringify(initialMsg) });
      }
    } catch (e) {
      console.error("Failed to load history", e);
    }
  };

  const saveMessageToDb = async (msg: Message) => {
    try {
      await invoke('save_message', { messageJson: JSON.stringify(msg) });
    } catch (e) {
      console.error("Failed to save msg", e);
    }
  };

  const clearMemory = async () => {
    try {
      await invoke('clear_history');
      const coreMemories = await invoke<string[]>('get_core_memories');
      const memoryString = coreMemories.length > 0 ? `\n\nCORE MEMORIES ABOUT THE USER:\n${coreMemories.map(m => `- ${m}`).join('\n')}` : '';
      
      const sysMsg: ChatMessageDisplay = { id: Date.now(), role: 'system', content: `You are KAI, a helpful autonomous AI companion running locally on the user's laptop. \n\nYou have access to various tools to interact with the OS.\n- Use the browser_action tool to autonomously browse the web. You can navigate, click, type, and read web pages to accomplish user goals. Keep the browser open across multiple tool calls if needed, and close it when done.\n- Use run_python_code to execute python code to solve math, write scripts, or interact with APIs. You MUST print() output.\n- Use capture_webcam to see the user or their physical environment.\n- Use read_emails to check the user's configured inboxes (specify account_alias or 'all').\n- Use read_calendar to check upcoming Google Calendar events.\n- Use remember_fact to autonomously save important facts about the user.\n- Use recall_memory to search your semantic memory database for past context.\n- If the user asks you to do a task, MUST IMMEDIATELY use the appropriate tools. NEVER ask for permission.\n- If a tool returns an error or something fails, you MUST explain the issue to the user naturally. If you can fix it yourself using your tools, you MUST do so immediately without asking for help.\nCRITICAL: If the user asks you to play music, play a song, or play an artist, YOU MUST ALWAYS use the play_song tool. ONLY use the control_media tool if they explicitly ask to pause, skip, or resume an already playing media player.${memoryString}` };
      const initialMsg: ChatMessageDisplay = { id: Date.now() + 1, role: 'assistant', content: 'Memory cleared! Hello again!' };
      setMessages([sysMsg, initialMsg]);
      await saveMessageToDb(sysMsg);
      await saveMessageToDb(initialMsg);
    } catch (e) {
      console.error("Failed to clear", e);
    }
  };

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const submitMessage = async (text: string) => {
    if (!text.trim()) return;
    isSpeakingRef.current = true;
    const userMsg: ChatMessageDisplay = { id: Date.now(), role: 'user', content: text };
    const newMessages = [...messagesRef.current, userMsg];
    setMessages(newMessages);
    await saveMessageToDb(userMsg);
    await runAgentLoop(newMessages);
  };

  const handleSend = async () => {
    if (!inputText.trim() || isLoading) return;
    const text = inputText;
    setInputText('');
    await submitMessage(text);
  };

  const runAgentLoop = async (currentMessages: ChatMessageDisplay[]) => {
    setIsLoading(true);
    let loopMessages = [...currentMessages];
    
    try {
      let isDone = false;
      let iterations = 0;
      const MAX_ITERATIONS = 5;

      while (!isDone && iterations < MAX_ITERATIONS) {
        iterations++;
        const streamId = Date.now();
        window.currentStreamId = streamId;
        
        const asstMsg: ChatMessageDisplay = { id: streamId, role: 'assistant', content: '' };
        setMessages([...loopMessages, asstMsg]);

        // Strip images before sending to LLM to prevent multimodal crashes with non-multimodal models
        const messagesWithoutImages = loopMessages.map(m => {
            const newMsg = { ...m };
            delete newMsg.images;
            return newMsg;
        });

        let responseMsg: Message | null = null;
        let retries = 0;
        const MAX_RETRIES = 3;

        while (retries < MAX_RETRIES && !responseMsg) {
            try {
                responseMsg = await invoke<Message>('chat_with_messages', { messages: messagesWithoutImages });
            } catch (err: any) {
                retries++;
                console.error(`LLM connection failed (attempt ${retries}/${MAX_RETRIES}):`, err);
                if (retries >= MAX_RETRIES) {
                    throw err;
                }
                // Wait 2 seconds before retrying
                await new Promise(resolve => setTimeout(resolve, 2000));
            }
        }
        
        if (!responseMsg) {
            throw new Error("Failed to get response after retries");
        }
        
        asstMsg.content = responseMsg.content;
        asstMsg.tool_calls = responseMsg.tool_calls;
        
        loopMessages = [...loopMessages, asstMsg];
        setMessages(loopMessages);
        await saveMessageToDb(asstMsg);

        // Flush any remaining text in the streaming buffer
        if (isVoiceEnabledRef.current && sentenceBufferRef.current.trim().length > 0) {
             const cleanText = sentenceBufferRef.current.trim().replace(/[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]/gu, '')
                                .replace(/[*_#`]/g, '');
             invoke('speak_sentence', { text: cleanText }).catch(console.error);
             sentenceBufferRef.current = '';
        }

        if (responseMsg.tool_calls && responseMsg.tool_calls.length > 0) {
          isDone = false;
          for (const call of responseMsg.tool_calls) {
             let toolResult = "";
             let isError = false;
             try {
                 toolResult = await invoke<string>('run_agent_tool', { name: call.function.name, args: call.function.arguments });
             } catch (err: any) {
                 toolResult = `Error executing tool: ${err}`;
                 isError = true;
             }
             
             let images: string[] = [];
             let content = toolResult;
             
             if (!isError && toolResult.startsWith('__IMAGE_B64__')) {
                 const base64Str = toolResult.replace('__IMAGE_B64__', '');
                 images.push(base64Str);
                 try {
                     const description = await invoke<string>('describe_image', { b64: base64Str });
                     content = `Image captured successfully. Vision model description: ${description}`;
                 } catch (e: any) {
                     content = `Image captured, but vision model failed to analyze it: ${e}`;
                 }
             } else if (toolResult.startsWith('__REMEMBER_FACT__')) {
                 const fact = toolResult.replace('__REMEMBER_FACT__', '');
                 try {
                     await invoke('remember_semantic_fact', { fact });
                     content = "Fact successfully stored in semantic memory.";
                 } catch (e: any) {
                     content = `Failed to store fact: ${e}`;
                 }
             } else if (toolResult.startsWith('__RECALL_MEMORY__')) {
                 const query = toolResult.replace('__RECALL_MEMORY__', '');
                 try {
                     const memories = await invoke<string[]>('recall_semantic_memory', { query });
                     if (memories.length > 0) {
                         content = `Recalled memories:\n- ${memories.join('\n- ')}`;
                     } else {
                         content = "No relevant memories found.";
                     }
                 } catch (e: any) {
                     content = `Failed to recall memory: ${e}`;
                 }
             }
             
             const toolMsg: ChatMessageDisplay = { id: Date.now(), role: 'tool', content: content, images };
             loopMessages = [...loopMessages, toolMsg];
             setMessages(loopMessages);
             await saveMessageToDb(toolMsg);
          }
        } else {
          isDone = true;
        }
      }

      if (!isDone && iterations >= MAX_ITERATIONS) {
         const errorMsg: ChatMessageDisplay = { id: Date.now(), role: 'tool', content: "System Error: Maximum tool execution iterations reached. The task failed repeatedly." };
         const newMessages = [...loopMessages, errorMsg];
         setMessages(newMessages);
         await saveMessageToDb(errorMsg);
         speak("I'm sorry, I was unable to complete the task after several attempts.");
      }
    } catch (error) {
      console.error(error);
      const errorMsg: ChatMessageDisplay = { id: Date.now(), role: 'tool', content: `Error: ${error}` };
      const newMessages = [...loopMessages, errorMsg];
      setMessages(newMessages);
      await saveMessageToDb(errorMsg);
    } finally {
      setIsLoading(false);
      // Keep mic muted for an additional 6 seconds to allow TTS to finish playing
      setTimeout(() => {
          isSpeakingRef.current = false;
      }, 6000);
    }
  };

  return (
    <div className="app-container">
      <header className="app-header" style={{ padding: '20px', display: 'flex', justifyContent: 'space-between', background: '#1e1e1e', borderBottom: '1px solid #333' }}>
        <div className="header-left" style={{ display: 'flex', alignItems: 'center', gap: '15px' }}>
          <div className="logo" style={{ fontSize: '24px', fontWeight: 'bold', color: '#00d2ff' }}>KAI</div>
          <div className="status-indicator" style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12px' }}>
            <span className="dot online" style={{ width: '8px', height: '8px', borderRadius: '50%', background: '#4caf50' }}></span>
            LOCAL MODE
          </div>
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <button onClick={() => {
             const newEnabled = !isVoiceEnabled;
             if (isVoiceEnabled) {
                 window.speechSynthesis?.cancel();
                 invoke('stop_continuous_vad');
                 setIsAwake(false);
             } else {
                 invoke('start_continuous_vad');
             }
             setIsVoiceEnabled(newEnabled);
          }} style={{background: isVoiceEnabled ? (isAwake ? 'rgba(255, 152, 0, 0.4)' : 'rgba(76, 175, 80, 0.4)') : 'rgba(255,255,255,0.1)', color: 'white', border: 'none', padding: '6px 12px', borderRadius: '6px', cursor: 'pointer', fontSize: '12px'}}>
            Voice: {isVoiceEnabled ? (isAwake ? 'ON (Awake)' : 'ON (Standby)') : 'OFF'}
          </button>
          <button onClick={clearMemory} style={{background: 'rgba(255,255,255,0.1)', color: 'white', border: 'none', padding: '6px 12px', borderRadius: '6px', cursor: 'pointer', fontSize: '12px'}}>
            Clear Memory
          </button>
        </div>
      </header>

      {isVoiceEnabled ? (
        <main className="voice-mode-container" style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center', padding: '20px' }}>
             <div className={`orb ${!isAwake ? 'asleep' : ''} ${isLoading ? 'thinking' : ''}`}></div>
             <h2 style={{ marginTop: '40px', fontWeight: 300, color: isAwake ? '#00d2ff' : '#888' }}>
                {isLoading ? "Thinking..." : (isAwake ? "Listening..." : "Standby (Say 'KAI' or Hold Alt+Space)")}
             </h2>
             
             {isPttActive && (
               <div style={{ position: 'absolute', bottom: '30px', right: '30px', background: 'rgba(255, 0, 0, 0.2)', color: '#ff4444', padding: '12px 24px', borderRadius: '24px', border: '1px solid rgba(255,0,0,0.5)', display: 'flex', alignItems: 'center', gap: '12px', fontSize: '16px', fontWeight: 'bold' }}>
                 <div style={{ width: '12px', height: '12px', borderRadius: '50%', background: '#ff4444', animation: 'pulse 1s infinite' }}></div>
                 Walkie-Talkie Active
               </div>
             )}
             {messages.length > 0 && messages[messages.length - 1].role === 'assistant' && (
                 <p style={{ color: '#888', fontStyle: 'italic', maxWidth: '80%', textAlign: 'center', marginTop: '20px', fontSize: '18px' }}>
                    "{messages[messages.length - 1].content}"
                 </p>
             )}
        </main>
      ) : (
        <>
          <main className="chat-container" style={{ flex: 1, overflowY: 'auto', padding: '20px', display: 'flex', flexDirection: 'column', gap: '15px' }}>
            {messages.filter(m => m.role !== 'system').map((msg) => (
              <div key={msg.id} className={`message ${msg.role === 'user' ? 'user' : 'kai'}`} style={{ alignSelf: msg.role === 'user' ? 'flex-end' : 'flex-start', background: msg.role === 'user' ? '#00d2ff' : '#2a2a2a', color: msg.role === 'user' ? '#000' : '#fff', padding: '12px 16px', borderRadius: '12px', maxWidth: '80%' }}>
                <div className="message-sender" style={{ fontSize: '12px', opacity: 0.7, marginBottom: '4px' }}>{msg.role === 'user' ? 'You' : (msg.role === 'tool' ? 'System Tool' : 'KAI')}</div>
                <div className="message-content">{msg.content}</div>
                {msg.tool_calls && msg.tool_calls.map((call, i) => (
                  <div key={i} style={{marginTop: '10px', padding: '10px', background: 'rgba(0,0,0,0.2)', borderRadius: '8px', fontSize: '14px', fontFamily: 'monospace'}}>
                    ⚙️ {call.function.name}({JSON.stringify(call.function.arguments)})
                  </div>
                ))}
                {msg.images && msg.images.map((b64, i) => (
                   <div key={i} style={{marginTop: '10px'}}>
                      <img src={`data:image/jpeg;base64,${b64}`} alt="Screenshot" style={{maxWidth: '100%', borderRadius: '8px'}} />
                   </div>
                ))}
              </div>
            ))}
            {isLoading && (
              <div className="message kai" style={{ alignSelf: 'flex-start', background: '#2a2a2a', padding: '12px 16px', borderRadius: '12px' }}>
                <div className="message-sender" style={{ fontSize: '12px', opacity: 0.7, marginBottom: '4px' }}>KAI</div>
                <div className="message-content">Thinking...</div>
              </div>
            )}
            <div ref={chatEndRef} />
          </main>
          <footer className="input-area" style={{ padding: '20px', background: '#1e1e1e', borderTop: '1px solid #333' }}>
            <form 
              className="input-wrapper"
              onSubmit={(e) => {
                e.preventDefault();
                handleSend();
              }}
            >
              <div style={{ display: 'flex', border: '1px solid #444', borderRadius: '4px', overflow: 'hidden' }}>

                <input 
                  type="text" 
                  value={inputText}
                  onChange={(e) => setInputText(e.target.value)}
                  placeholder="Ask KAI anything..."
                  style={{ flex: 1, padding: '10px', background: '#333', color: '#fff', border: 'none', outline: 'none' }}
                  disabled={isLoading}
                />
                
                <button 
                  type="submit"
                  disabled={isLoading || !inputText.trim()}
                  style={{ padding: '10px 20px', background: '#00d2ff', color: '#000', border: 'none', fontWeight: 'bold', cursor: 'pointer' }}
                >
                  SEND
                </button>
              </div>
            </form>
          </footer>
        </>
      )}
    </div>
  );
}

export default App;
