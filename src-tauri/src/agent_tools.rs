use crate::llm::{Tool, ToolFunction};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::json;
use std::fs;
use std::io::Cursor;
use std::process::Command;
use xcap::Monitor;

pub fn get_available_tools() -> Vec<Tool> {
    vec![
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "read_file".to_string(),
                description: "Read the contents of a file. ONLY use if the user explicitly asks you to read or open a specific file.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to the file" }
                    },
                    "required": ["path"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "write_file".to_string(),
                description: "Write content to a file. Overwrites the file if it exists.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to the file" },
                        "content": { "type": "string", "description": "The content to write" }
                    },
                    "required": ["path", "content"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "open_app".to_string(),
                description: "Open an application by its executable or desktop name (e.g. 'spotify', 'gnome-calculator', 'brave').".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "app_name": { "type": "string", "description": "The name of the app to launch" }
                    },
                    "required": ["app_name"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "manage_window".to_string(),
                description: "Focus, close, or minimize a window by its title. Requires wmctrl.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["focus", "close", "minimize"] },
                        "window_name": { "type": "string", "description": "A substring of the window title to match" }
                    },
                    "required": ["action", "window_name"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "control_media".to_string(),
                description: "Play, pause, skip, or go to previous track on the system media player (Spotify, VLC, web browser).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["play", "pause", "play-pause", "next", "previous"] }
                    },
                    "required": ["action"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "play_song".to_string(),
                description: "Search YouTube and immediately stream the requested song/audio in the background. Use this when the user asks to play a specific song or artist and it is not already playing on Spotify/etc.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The name of the song or artist to play" }
                    },
                    "required": ["query"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "list_directory".to_string(),
                description: "List files and folders in a directory. ONLY use if the user explicitly asks to see files in a folder.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to the directory" }
                    },
                    "required": ["path"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "git_status".to_string(),
                description: "Get the git status of a repository. ONLY use if the user asks about git status.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute path to the git repository" }
                    },
                    "required": ["path"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "take_screenshot".to_string(),
                description: "Take a screenshot of the primary monitor. ONLY call this when the user EXPLICITLY asks you to 'look at their screen', 'see this', or 'read the screen'. DO NOT use this for casual conversation.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "capture_webcam".to_string(),
                description: "Take a picture using the user's webcam. ONLY call this when the user EXPLICITLY asks you to 'look at them', 'see them', or take a picture using the webcam.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "run_terminal_command".to_string(),
                description: "Run a bash shell command on the user's system. ONLY use if the user explicitly asks you to run a command or perform an OS action.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to run"
                        }
                    },
                    "required": ["command"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "read_emails".to_string(),
                description: "Read the user's latest emails. Specify account_alias (e.g. 'zoho_main') or 'all' to check all configured accounts. Default is 'all'.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "account_alias": {
                            "type": "string",
                            "description": "The account alias to read from, or 'all'"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Number of emails to read (default 5)"
                        }
                    },
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "read_calendar".to_string(),
                description: "Read the user's upcoming Google Calendar events.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "open_url".to_string(),
                description: "Open a URL in the user's default web browser. ONLY use if the user asks you to open a website.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "The URL to open" }
                    },
                    "required": ["url"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "search_web".to_string(),
                description: "Silently search the web (DuckDuckGo) and return the text results. ONLY use if the user asks a factual question that you don't know the answer to, or explicitly asks you to search the web.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    },
                    "required": ["query"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "remember_fact".to_string(),
                description: "Save an important fact about the user to long-term episodic memory. Call this autonomously when the user tells you something about themselves, their preferences, or important context that you should remember for future sessions.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "fact": { "type": "string", "description": "A concise summary of the fact to remember. E.g., 'User prefers dark mode' or 'User's name is Alex'" }
                    },
                    "required": ["fact"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "recall_memory".to_string(),
                description: "Search your semantic vector database for past memories. Use this if the user asks you about something they told you previously and you don't have it in your immediate context window.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string", "description": "The topic or keyword to search for" }
                    },
                    "required": ["topic"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "browser_action".to_string(),
                description: "Autonomously navigate the web using a visible browser. Actions: 'goto' (url), 'click' (selector), 'type' (selector, text), 'read' (returns text content of the page), 'close' (closes browser).".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["goto", "click", "type", "read", "close"] },
                        "url": { "type": "string", "description": "URL for goto action" },
                        "selector": { "type": "string", "description": "CSS selector for click/type actions" },
                        "text": { "type": "string", "description": "Text for type action" }
                    },
                    "required": ["action"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "move_mouse".to_string(),
                description: "Move the mouse to absolute screen coordinates (x, y). Use take_screenshot to determine coordinates first if necessary.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "x": { "type": "integer" },
                        "y": { "type": "integer" }
                    },
                    "required": ["x", "y"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "click_mouse".to_string(),
                description: "Click the mouse at the current position. Valid buttons: 'left', 'right', 'middle'.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "button": { "type": "string", "enum": ["left", "right", "middle"] }
                    },
                    "required": ["button"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "type_text".to_string(),
                description: "Type a sequence of characters at the current focused input field.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" }
                    },
                    "required": ["text"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "press_key".to_string(),
                description: "Press a specific key (e.g. 'return', 'escape', 'tab', 'backspace').".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" }
                    },
                    "required": ["key"]
                }),
            }
        },
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "run_python_code".to_string(),
                description: "Execute Python code in a sandboxed environment. Use this to do math, data analysis, or connect to APIs. You must print() the result to see the output.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "code": { "type": "string", "description": "The Python code to execute" }
                    },
                    "required": ["code"]
                }),
            }
        }
    ]
}

pub async fn execute_tool(name: &str, args: &serde_json::Value) -> Result<String, String> {
    match name {
        "read_file" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            fs::read_to_string(path).map_err(|e| e.to_string())
        }
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            let content = args.get("content").and_then(|v| v.as_str()).ok_or("Missing content")?;
            fs::write(path, content).map_err(|e| e.to_string())?;
            Ok(format!("Successfully wrote to {}", path))
        }
        "open_app" => {
            let app = args.get("app_name").and_then(|v| v.as_str()).ok_or("Missing app_name")?;
            let output = Command::new("gtk-launch").arg(app).output();
            if let Ok(out) = output {
                if out.status.success() {
                    return Ok(format!("Launched {}", app));
                }
            }
            Command::new(app).spawn().map_err(|e| format!("Failed to launch {}: {}", app, e))?;
            Ok(format!("Launched {} directly", app))
        }
        "manage_window" => {
            let action = args.get("action").and_then(|v| v.as_str()).ok_or("Missing action")?;
            let window_name = args.get("window_name").and_then(|v| v.as_str()).ok_or("Missing window_name")?;
            
            let mut cmd = Command::new("wmctrl");
            match action {
                "focus" => { cmd.arg("-a").arg(window_name); },
                "close" => { cmd.arg("-c").arg(window_name); },
                "minimize" => { cmd.arg("-r").arg(window_name).arg("-b").arg("add,hidden"); },
                _ => return Err("Invalid action".to_string()),
            }
            let output = cmd.output().map_err(|e| format!("Failed to run wmctrl (ensure it is installed): {}", e))?;
            if !output.status.success() {
                return Err(format!("wmctrl failed: {}", String::from_utf8_lossy(&output.stderr)));
            }
            Ok(format!("Successfully performed {} on window '{}'", action, window_name))
        }
        "control_media" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let output = Command::new("playerctl")
                .arg(action)
                .output();
            match output {
                Ok(o) if o.status.success() => Ok(format!("Successfully performed media action: {}", action)),
                Ok(o) => Err(format!("playerctl failed (no player running?): {}", String::from_utf8_lossy(&o.stderr))),
                Err(e) => Err(format!("Failed to execute playerctl (is it installed?): {}", e)),
            }
        },
        "play_song" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            println!("Attempting to play song: {}", query);
            
            // First stop any existing mpv instance using killall
            let _ = Command::new("killall").arg("mpv").output();
            
            // Spawn mpv in background
            let ytdl_path = std::env::var("HOME").unwrap_or_else(|_| "".to_string()) + "/.local/bin/yt-dlp";
            let spawn_res = Command::new("mpv")
                .arg(format!("ytdl://ytsearch:{}", query))
                .arg("--no-video")
                .arg("--ytdl-format=bestaudio")
                .arg(format!("--script-opts=ytdl_hook-ytdl_path={}", ytdl_path))
                .spawn();
                
            match spawn_res {
                Ok(_) => Ok(format!("Now playing: {} in the background.", query)),
                Err(e) => Err(format!("Failed to start mpv: {}. Make sure mpv and yt-dlp are installed.", e)),
            }
        }
        "list_directory" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            let mut entries = Vec::new();
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                entries.push(entry.file_name().to_string_lossy().to_string());
            }
            Ok(entries.join("\n"))
        }
        "git_status" => {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            let output = Command::new("git")
                .arg("status")
                .current_dir(path)
                .output()
                .map_err(|e| e.to_string())?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        "take_screenshot" => {
            let monitors = Monitor::all().map_err(|e| e.to_string())?;
            if let Some(primary) = monitors.first() {
                let image = primary.capture_image().map_err(|e| e.to_string())?;

                // Compress and format as base64 jpeg
                let mut buffer = Cursor::new(Vec::new());
                let (_width, _height) = (image.width(), image.height());
                // xcap returns an RgbaImage, we encode to JPEG
                let dyn_img = image::DynamicImage::ImageRgba8(image);
                dyn_img
                    .write_to(&mut buffer, image::ImageFormat::Jpeg)
                    .map_err(|e| format!("Failed to encode image: {}", e))?;

                let b64 = STANDARD.encode(buffer.into_inner());
                // We return a special marker string that the backend can intercept
                // and attach to the actual message context as an image block.
                Ok(format!("__IMAGE_B64__{}", b64))
            } else {
                Err("No monitors found".to_string())
            }
        }
        "capture_webcam" => {
            use nokhwa::Camera;
            use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
            
            let index = CameraIndex::Index(0);
            let requested = RequestedFormat::new::<nokhwa::pixel_format::RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
            
            let mut camera = Camera::new(index, requested).map_err(|e| format!("Failed to open webcam: {}", e))?;
            camera.open_stream().map_err(|e| format!("Failed to open webcam stream: {}", e))?;
            
            // grab a frame
            let frame = camera.frame().map_err(|e| format!("Failed to capture frame: {}", e))?;
            let img = frame.decode_image::<nokhwa::pixel_format::RgbFormat>().map_err(|e| format!("Failed to decode frame: {}", e))?;
            
            let mut buffer = Cursor::new(Vec::new());
            let dyn_img = image::DynamicImage::ImageRgb8(img);
            dyn_img
                .write_to(&mut buffer, image::ImageFormat::Jpeg)
                .map_err(|e| format!("Failed to encode webcam image: {}", e))?;
                
            let b64 = STANDARD.encode(buffer.into_inner());
            Ok(format!("__IMAGE_B64__{}", b64))
        }
        "run_terminal_command" => {
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or("Missing command")?;
            let output = Command::new("bash")
                .arg("-c")
                .arg(command)
                .output()
                .map_err(|e| e.to_string())?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.is_empty() {
                Ok(stdout)
            } else {
                Ok(format!("Stdout:\n{}\nStderr:\n{}", stdout, stderr))
            }
        }
        "read_emails" => {
            let target = args.get("account_alias").and_then(|v| v.as_str()).unwrap_or("all");
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(5);
            let script_path = "/home/sjay/Documents/KAI/src-tauri/scripts/email_client.py";
            let venv_python = "/home/sjay/.local/share/kai/venv/bin/python";
            
            let output = Command::new(venv_python)
                .arg(script_path)
                .arg(target)
                .arg(limit.to_string())
                .output()
                .map_err(|e| format!("Failed to run email client: {}", e))?;
                
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("{}\n{}", stdout, stderr))
        }
        "read_calendar" => {
            let script_path = "/home/sjay/Documents/KAI/src-tauri/scripts/calendar_client.py";
            let venv_python = "/home/sjay/.local/share/kai/venv/bin/python";
            
            let output = Command::new(venv_python)
                .arg(script_path)
                .output()
                .map_err(|e| format!("Failed to run calendar client: {}", e))?;
                
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("{}\n{}", stdout, stderr))
        }
        "open_url" => {
            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or("Missing url")?;
            #[cfg(target_os = "linux")]
            Command::new("xdg-open")
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            Command::new("open")
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())?;

            #[cfg(target_os = "windows")]
            Command::new("cmd")
                .args(["/C", "start", url])
                .spawn()
                .map_err(|e| e.to_string())?;

            Ok(format!("Opened URL: {}", url))
        }
        "search_web" => {
            let query = args.get("query").and_then(|v| v.as_str()).ok_or("Missing query")?;
            
            // Call the python script using the local venv
            let output = Command::new("../.venv/bin/python")
                .arg("search.py")
                .arg(query)
                .output()
                .map_err(|e| format!("Failed to run search script: {}. Make sure you are running from src-tauri.", e))?;
                
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if stdout.trim().is_empty() {
                if !stderr.trim().is_empty() {
                    return Err(format!("Search error: {}", stderr));
                }
                return Ok("No results found.".to_string());
            }
            
            Ok(stdout)
        }
        "remember_fact" => {
            let fact = args.get("fact").and_then(|v| v.as_str()).ok_or("Missing fact parameter")?;
            Ok(format!("__REMEMBER_FACT__{}", fact))
        }
        "recall_memory" => {
            let topic = args.get("topic").and_then(|v| v.as_str()).ok_or("Missing topic parameter")?;
            Ok(format!("__RECALL_MEMORY__{}", topic))
        }
        "browser_action" => {
            let action = args.get("action").and_then(|v| v.as_str()).ok_or("Missing action")?;
            
            // Local state to keep browser alive
            fn browser_state() -> &'static std::sync::Mutex<Option<(headless_chrome::Browser, std::sync::Arc<headless_chrome::Tab>)>> {
                static STATE: std::sync::OnceLock<std::sync::Mutex<Option<(headless_chrome::Browser, std::sync::Arc<headless_chrome::Tab>)>>> = std::sync::OnceLock::new();
                STATE.get_or_init(|| std::sync::Mutex::new(None))
            }
            
            let mut state = browser_state().lock().unwrap();
            
            if state.is_none() && action != "close" {
                let options = headless_chrome::LaunchOptions::default_builder()
                    .headless(false)
                    .build()
                    .map_err(|e| e.to_string())?;
                let browser = headless_chrome::Browser::new(options).map_err(|e| e.to_string())?;
                let tab = browser.new_tab().map_err(|e| e.to_string())?;
                *state = Some((browser, tab));
            }
            
            if action == "close" {
                *state = None;
                return Ok("Browser closed.".to_string());
            }
            
            let tab = state.as_ref().unwrap().1.clone();
            
            match action {
                "goto" => {
                    let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing url parameter")?;
                    tab.navigate_to(url).map_err(|e| e.to_string())?;
                    tab.wait_until_navigated().map_err(|e| e.to_string())?;
                    Ok(format!("Navigated to {}", url))
                },
                "click" => {
                    let selector = args.get("selector").and_then(|v| v.as_str()).ok_or("Missing selector")?;
                    let el = tab.wait_for_element(selector).map_err(|e| format!("Element not found: {}", e))?;
                    el.click().map_err(|e| e.to_string())?;
                    Ok(format!("Clicked element {}", selector))
                },
                "type" => {
                    let selector = args.get("selector").and_then(|v| v.as_str()).ok_or("Missing selector")?;
                    let text = args.get("text").and_then(|v| v.as_str()).ok_or("Missing text")?;
                    let el = tab.wait_for_element(selector).map_err(|e| format!("Element not found: {}", e))?;
                    el.click().map_err(|e| e.to_string())?;
                    el.type_into(text).map_err(|e| e.to_string())?;
                    Ok(format!("Typed text into {}", selector))
                },
                "read" => {
                    let el = tab.wait_for_element("body").map_err(|e| format!("Failed to find body: {}", e))?;
                    let text = el.get_inner_text().map_err(|e| e.to_string())?;
                    let mut truncated = text;
                    if truncated.len() > 15000 {
                        truncated.truncate(15000);
                        truncated.push_str("\n...[TRUNCATED]");
                    }
                    Ok(truncated)
                },
                _ => Err(format!("Unknown browser action: {}", action))
            }
        }
        "move_mouse" => {
            use enigo::{Enigo, MouseControllable};
            let x = args.get("x").and_then(|v| v.as_i64()).ok_or("Missing x")? as i32;
            let y = args.get("y").and_then(|v| v.as_i64()).ok_or("Missing y")? as i32;
            let mut enigo = Enigo::new();
            enigo.mouse_move_to(x, y);
            Ok(format!("Mouse moved to ({}, {})", x, y))
        }
        "click_mouse" => {
            use enigo::{Enigo, MouseControllable, MouseButton};
            let btn_str = args.get("button").and_then(|v| v.as_str()).unwrap_or("left");
            let button = match btn_str {
                "left" => MouseButton::Left,
                "right" => MouseButton::Right,
                "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            let mut enigo = Enigo::new();
            enigo.mouse_click(button);
            Ok(format!("Clicked {} mouse button", btn_str))
        }
        "type_text" => {
            use enigo::{Enigo, KeyboardControllable};
            let text = args.get("text").and_then(|v| v.as_str()).ok_or("Missing text")?;
            let mut enigo = Enigo::new();
            enigo.key_sequence(text);
            Ok(format!("Typed text: {}", text))
        }
        "press_key" => {
            use enigo::{Enigo, KeyboardControllable, Key};
            let key_str = args.get("key").and_then(|v| v.as_str()).ok_or("Missing key")?;
            let key = match key_str.to_lowercase().as_str() {
                "return" | "enter" => Key::Return,
                "escape" => Key::Escape,
                "tab" => Key::Tab,
                "backspace" => Key::Backspace,
                "space" => Key::Space,
                "up" => Key::UpArrow,
                "down" => Key::DownArrow,
                "left" => Key::LeftArrow,
                "right" => Key::RightArrow,
                _ => return Err(format!("Unsupported key: {}", key_str)),
            };
            let mut enigo = Enigo::new();
            enigo.key_click(key);
            Ok(format!("Pressed key: {}", key_str))
        }
        "run_python_code" => {
            let code = args.get("code").and_then(|v| v.as_str()).ok_or("Missing code parameter")?;
            
            let tmp_path = std::env::temp_dir().join("kai_sandbox.py");
            fs::write(&tmp_path, code).map_err(|e| format!("Failed to write python code: {}", e))?;
            
            let venv_path = dirs::home_dir().unwrap().join(".local/share/kai/venv");
            let python_bin = venv_path.join("bin/python");
            
            let output = Command::new("bwrap")
                .arg("--ro-bind").arg("/usr").arg("/usr")
                .arg("--ro-bind").arg("/lib").arg("/lib")
                .arg("--ro-bind-try").arg("/lib64").arg("/lib64")
                .arg("--ro-bind").arg("/bin").arg("/bin")
                .arg("--ro-bind-try").arg("/sbin").arg("/sbin")
                .arg("--ro-bind-try").arg("/etc/resolv.conf").arg("/etc/resolv.conf")
                .arg("--ro-bind-try").arg("/etc/ssl/certs").arg("/etc/ssl/certs")
                .arg("--ro-bind-try").arg("/etc/pki").arg("/etc/pki")
                .arg("--ro-bind").arg(&venv_path).arg(&venv_path)
                .arg("--bind").arg("/tmp").arg("/tmp")
                .arg("--unshare-all")
                .arg("--share-net")
                .arg("--die-with-parent")
                .arg("--dir").arg("/sandbox")
                .arg("--chdir").arg("/sandbox")
                .arg(&python_bin)
                .arg(&tmp_path)
                .output()
                .map_err(|e| format!("Failed to execute python via bwrap: {}", e))?;
                
            let mut result = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            
            if !stderr.trim().is_empty() {
                if !result.is_empty() {
                    result.push_str("\n--- STDERR ---\n");
                }
                result.push_str(&stderr);
            }
            
            if result.trim().is_empty() {
                Ok("Code executed successfully with no output.".to_string())
            } else {
                Ok(result)
            }
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
