use std::process::Command;
use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, source::Source};
use std::path::PathBuf;
use tauri::AppHandle;

#[tauri::command]
pub async fn speak_neural(text: String) -> Result<(), String> {
    let home = dirs::home_dir().unwrap();
    let piper_bin = home.join(".local/share/kai/venv/bin/piper");
    let model = home.join(".local/share/kai/models/en_US-lessac-medium.onnx");
    let out_wav = home.join(".local/share/kai/output.wav");
    
    // Check if Piper is installed
    if !piper_bin.exists() {
        return Err("Neural voice engine is not installed yet.".to_string());
    }

    // Convert text to speech using piper via command line
    let mut echo = Command::new("echo")
        .arg(&text)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to echo text: {}", e))?;

    let echo_stdout = echo.stdout.take().unwrap();

    let piper_status = Command::new(&piper_bin)
        .arg("--model")
        .arg(&model)
        .arg("--output_file")
        .arg(&out_wav)
        .stdin(echo_stdout)
        .status()
        .map_err(|e| format!("Failed to run piper: {}", e))?;

    if !piper_status.success() {
        return Err("Piper exited with an error".to_string());
    }

    // Play the generated audio
    std::thread::spawn(move || {
        if let Ok((_stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(file) = File::open(&out_wav) {
                let buf = BufReader::new(file);
                if let Ok(source) = Decoder::new(buf) {
                    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
                    sink.append(source);
                    sink.sleep_until_end();
                }
            }
        }
    });

    Ok(())
}

use std::process::ChildStdin;
pub struct PiperState(pub std::sync::Mutex<Option<ChildStdin>>);

#[tauri::command]
pub fn init_piper(state: tauri::State<'_, PiperState>) -> Result<(), String> {
    let home = dirs::home_dir().unwrap();
    let piper_bin = home.join(".local/share/kai/venv/bin/piper");
    let model = home.join(".local/share/kai/models/en_US-lessac-medium.onnx");
    
    if !piper_bin.exists() {
        println!("Neural voice engine is not installed yet.");
        return Ok(());
    }

    let cmd_str = format!(
        "\"{}\" --model \"{}\" --output_raw | aplay -r 22050 -f S16_LE -t raw -c 1",
        piper_bin.to_string_lossy(),
        model.to_string_lossy()
    );

    let child = Command::new("sh")
        .arg("-c")
        .arg(&cmd_str)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn piper daemon: {}", e))?;

    let stdin = child.stdin.unwrap();
    *state.0.lock().unwrap() = Some(stdin);
    println!("Piper daemon started successfully.");
    Ok(())
}

#[tauri::command]
pub fn speak_sentence(text: String, state: tauri::State<'_, PiperState>) -> Result<(), String> {
    use std::io::Write;
    let mut stdin_guard = state.0.lock().unwrap();
    if let Some(stdin) = stdin_guard.as_mut() {
        let clean_text = text.replace("\n", " ");
        if let Err(e) = stdin.write_all(format!("{}\n", clean_text).as_bytes()) {
            eprintln!("Failed to write to piper daemon: {}", e);
        }
        if let Err(e) = stdin.flush() {
            eprintln!("Failed to flush piper daemon: {}", e);
        }
    }
    Ok(())
}

use base64::{Engine as _, engine::general_purpose};
use std::io::Write;

#[tauri::command]
pub async fn transcribe_audio() -> Result<String, String> {
    let home = dirs::home_dir().unwrap();
    let wav_path = home.join(".local/share/kai/input.wav");
    transcribe_audio_impl(&wav_path)
}

fn transcribe_audio_impl(input_wav: &std::path::PathBuf) -> Result<String, String> {
    let home = dirs::home_dir().unwrap();
    let whisper_bin = home.join(".local/share/kai/venv/bin/whisper");
    
    if !whisper_bin.exists() {
        return Err("Universal Hearing (Whisper) is not fully installed.".to_string());
    }

    let output = Command::new(&whisper_bin)
        .current_dir(home.join(".local/share/kai/"))
        .arg(input_wav)
        .arg("--model")
        .arg("base")
        .arg("--language")
        .arg("en")
        .arg("--initial_prompt")
        .arg("Hey Kai. Hello Kai. Hi Kai. Okay Kai. Seneth Jayashan, KAI, local AI.")
        .output()
        .map_err(|e| format!("Failed to run whisper. Error: {}", e))?;
        
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Whisper failed: {}", err));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut transcript = String::new();
    for line in stdout.lines() {
        if let Some(idx) = line.find("] ") {
            transcript.push_str(&line[idx+2..]);
            transcript.push(' ');
        }
    }
    
    Ok(transcript.trim().to_string())
}

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound;
use tauri::{State, Emitter};

pub struct VadState(pub Arc<AtomicBool>);

#[tauri::command]
pub fn start_continuous_vad(app: tauri::AppHandle, state: State<'_, VadState>) -> Result<(), String> {
    if state.0.load(Ordering::SeqCst) {
        return Ok(());
    }
    state.0.store(true, Ordering::SeqCst);
    
    let is_running_clone = state.0.clone();
    let is_running_clone2 = state.0.clone();
    let home = dirs::home_dir().unwrap();
    
    thread::spawn(move || {
        let host = cpal::default_host();
        let device = host.default_input_device().expect("No input device available");
        let config = device.default_input_config().expect("Failed to get default input config");
        
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        
        let energy_threshold = 2000; 
        let min_speech_frames = (sample_rate as f32 * 0.3) as usize; 
        let max_silence_frames = (sample_rate as f32 * 1.5) as usize;
        
        let (tx, rx) = std::sync::mpsc::channel::<Vec<i16>>();
        
        let stream = match config.sample_format() {
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    if is_running_clone.load(Ordering::SeqCst) {
                        let _ = tx.send(data.to_vec());
                    }
                },
                |err| eprintln!("stream error: {}", err),
                None,
            ).unwrap(),
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    if is_running_clone.load(Ordering::SeqCst) {
                        let mut i16_data = Vec::with_capacity(data.len());
                        for &sample in data {
                            i16_data.push((sample * i16::MAX as f32) as i16);
                        }
                        let _ = tx.send(i16_data);
                    }
                },
                |err| eprintln!("stream error: {}", err),
                None,
            ).unwrap(),
            _ => panic!("Unsupported format"),
        };
        
        stream.play().unwrap();
        
        let mut silence_frames = 0;
        let mut speech_frames = 0;
        let mut is_speaking = false;
        let mut current_audio = Vec::new();
        let mut wav_index = 0;
        
        while is_running_clone2.load(Ordering::SeqCst) {
            if let Ok(chunk) = rx.recv_timeout(std::time::Duration::from_millis(100)) {
                let mut sum_sq: f64 = 0.0;
                for &sample in &chunk {
                    let s = sample as f64;
                    sum_sq += s * s;
                }
                let rms = (sum_sq / chunk.len() as f64).sqrt() as i32;
                
                if rms > energy_threshold {
                    silence_frames = 0;
                    speech_frames += chunk.len();
                    if !is_speaking && speech_frames > min_speech_frames {
                        is_speaking = true;
                    }
                } else {
                    silence_frames += chunk.len();
                }
                
                if is_speaking {
                    current_audio.extend_from_slice(&chunk);
                } else if speech_frames > 0 {
                    let keep = (sample_rate as f32 * 0.5) as usize;
                    let drain_count = current_audio.len().saturating_sub(keep);
                    if drain_count > 0 {
                        current_audio.drain(0..drain_count);
                    }
                    current_audio.extend_from_slice(&chunk);
                }
                
                if is_speaking && silence_frames > max_silence_frames {
                    is_speaking = false;
                    silence_frames = 0;
                    speech_frames = 0;
                    
                    let audio_to_save = std::mem::take(&mut current_audio);
                    if audio_to_save.len() < sample_rate as usize {
                        continue; // too short
                    }
                    
                    let wav_path = home.join(format!(".local/share/kai/input_vad_{}.wav", wav_index));
                    wav_index += 1;
                    
                    let app_clone = app.clone();
                    let sample_rate_clone = sample_rate;
                    let channels_clone = channels;
                    
                    std::thread::spawn(move || {
                        let spec = hound::WavSpec {
                            channels: channels_clone,
                            sample_rate: sample_rate_clone,
                            bits_per_sample: 16,
                            sample_format: hound::SampleFormat::Int,
                        };
                        
                        if let Ok(mut writer) = hound::WavWriter::create(&wav_path, spec) {
                            let mut success = true;
                            for sample in audio_to_save {
                                if let Err(e) = writer.write_sample(sample) {
                                    eprintln!("Failed to write sample to {:?}: {}", wav_path, e);
                                    success = false;
                                    break;
                                }
                            }
                            if success {
                                if let Err(e) = writer.finalize() {
                                    eprintln!("Failed to finalize {:?}: {}", wav_path, e);
                                    success = false;
                                }
                            }
                            
                            if success {
                                if let Ok(transcript) = transcribe_audio_impl(&wav_path) {
                                    if !transcript.trim().is_empty() {
                                        let _ = app_clone.emit("vad_speech_transcribed", transcript);
                                    }
                                }
                            }
                            
                            // Prevent disk full by deleting temp files immediately
                            let _ = std::fs::remove_file(&wav_path);
                        }
                    });
                }
            }
        }
        
        drop(stream);
    });
    
    Ok(())
}

#[tauri::command]
pub fn stop_continuous_vad(state: State<'_, VadState>) -> Result<(), String> {
    state.0.store(false, Ordering::SeqCst);
    Ok(())
}
