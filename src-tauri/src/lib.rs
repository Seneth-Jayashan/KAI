pub mod agent_tools;
pub mod llm;
use tauri::Emitter;
pub mod memory;
pub mod tools;
pub mod voice;

#[tauri::command]
async fn chat_with_messages(app: tauri::AppHandle, messages: Vec<llm::ChatMessage>) -> Result<llm::ChatMessage, String> {
    let client = llm::LlmClient::default().with_tools(agent_tools::get_available_tools());
    client.chat(&app, messages).await
}

#[tauri::command]
async fn describe_image(app: tauri::AppHandle, b64: String) -> Result<String, String> {
    println!("Routing image to vision model (llava)...");
    let vision_client = llm::LlmClient::new("llava");
    let vision_msg = llm::ChatMessage {
        role: "user".to_string(),
        content: "Describe this screenshot in detail. If there is text, read it. If it is a UI, describe what is visible.".to_string(),
        images: Some(vec![b64]),
        tool_calls: None,
    };

    match vision_client.chat(&app, vec![vision_msg]).await {
        Ok(resp) => Ok(resp.content),
        Err(e) => Err(format!("Vision model error. Please ensure you have run 'ollama pull llava' in your terminal. Error: {}", e)),
    }
}

#[tauri::command]
async fn run_agent_tool(name: String, args: serde_json::Value) -> Result<String, String> {
    agent_tools::execute_tool(&name, &args).await
}

use tauri_plugin_global_shortcut::{GlobalShortcutExt, Code, Modifiers, Shortcut, ShortcutState};
use tauri::Manager;
use tauri::tray::TrayIconBuilder;
use tauri::menu::{Menu, MenuItem};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(voice::VadState(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))));
            app.manage(voice::PiperState(std::sync::Mutex::new(None)));
            memory::init_db(app.handle()).expect("Failed to initialize database");
            if let Err(e) = voice::init_piper(app.state::<voice::PiperState>()) {
                eprintln!("Warning: Failed to init piper daemon: {}", e);
            }
            
            // Register Global Shortcut: Ctrl+Space to toggle KAI
            let ctrl_space = Shortcut::new(Some(Modifiers::CONTROL), Code::Space);
            let alt_space = Shortcut::new(Some(Modifiers::ALT), Code::Space);
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, event| {
                        if shortcut == &ctrl_space && event.state() == ShortcutState::Pressed {
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    window.hide().unwrap();
                                } else {
                                    window.show().unwrap();
                                    window.set_focus().unwrap();
                                }
                            }
                        } else if shortcut == &alt_space {
                            if event.state() == ShortcutState::Pressed {
                                let _ = app.emit("ptt-start", ());
                            } else if event.state() == ShortcutState::Released {
                                let _ = app.emit("ptt-stop", ());
                            }
                        }
                    })
                    .build(),
            )?;
            app.global_shortcut().register(ctrl_space)?;
            app.global_shortcut().register(alt_space)?;
            
            // Register System Tray
            let quit_i = MenuItem::with_id(app.handle(), "quit", "Quit KAI", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app.handle(), &[&quit_i])?;
            
            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .icon(app.default_window_icon().unwrap().clone())
                .build(app.handle())?;
                
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent app from closing, just hide the window instead
                api.prevent_close();
                window.hide().unwrap();
            }
        })
        .invoke_handler(tauri::generate_handler![
            chat_with_messages,
            describe_image,
            run_agent_tool,
            tools::get_system_info,
            memory::get_history,
            memory::save_message,
            memory::clear_history,
            memory::get_core_memories,
            memory::save_core_memory,
            memory::remember_semantic_fact,
            memory::recall_semantic_memory,
            voice::speak_neural,
            voice::transcribe_audio,
            voice::start_continuous_vad,
            voice::stop_continuous_vad,
            voice::init_piper,
            voice::speak_sentence
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

