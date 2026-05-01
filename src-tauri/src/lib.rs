use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BookContent {
    file_name: String,
    text: String,
    character_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesizeRequest {
    piper_path: String,
    model_path: String,
    output_path: String,
    text: String,
    language: String,
    tone: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SynthesizeResult {
    output_path: String,
}

#[tauri::command]
fn read_book_file(path: String) -> Result<BookContent, String> {
    let file_path = Path::new(&path);
    let file_name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("ebook")
        .to_string();

    let extension = file_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();

    if !matches!(
        extension.as_str(),
        "txt" | "md" | "markdown" | "html" | "htm"
    ) {
        return Err("Only .txt, .md, and .html files are supported for now.".to_string());
    }

    let raw =
        fs::read_to_string(file_path).map_err(|error| format!("Failed to read file: {error}"))?;
    let text = normalize_text(&raw, matches!(extension.as_str(), "html" | "htm"));
    let character_count = text.chars().count();

    if text.is_empty() {
        return Err("No readable text was found in this file.".to_string());
    }

    Ok(BookContent {
        file_name,
        text,
        character_count,
    })
}

#[tauri::command]
fn synthesize_with_piper(request: SynthesizeRequest) -> Result<SynthesizeResult, String> {
    let piper_path = Path::new(&request.piper_path);
    let model_path = Path::new(&request.model_path);
    let output_path = Path::new(&request.output_path);

    if !piper_path.exists() {
        return Err("Piper executable does not exist.".to_string());
    }

    if !model_path.exists() {
        return Err("Piper model file does not exist.".to_string());
    }

    if request.text.trim().is_empty() {
        return Err("There is no text to synthesize.".to_string());
    }

    if request.language.trim().is_empty() {
        return Err("A target language is required.".to_string());
    }

    let prepared_text = prepare_text_for_tts(&request.text);
    let length_scale = length_scale_for_tone(&request.tone);
    let mut child = Command::new(piper_path)
        .arg("--model")
        .arg(model_path)
        .arg("--length_scale")
        .arg(length_scale)
        .arg("--output_file")
        .arg(output_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start Piper: {error}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prepared_text.as_bytes())
            .map_err(|error| format!("Failed to send text to Piper: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("Failed to wait for Piper: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Piper failed: {}", stderr.trim()));
    }

    Ok(SynthesizeResult {
        output_path: output_path.to_string_lossy().to_string(),
    })
}

fn normalize_text(raw: &str, is_html: bool) -> String {
    let without_markup = if is_html {
        strip_html_tags(raw)
    } else {
        raw.to_string()
    };

    without_markup
        .replace(['#', '*', '_', '>', '`', '~'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_html_tags(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut inside_tag = false;

    for character in raw.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(character),
            _ => {}
        }
    }

    output
}

fn prepare_text_for_tts(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("{normalized}\n")
}

fn length_scale_for_tone(tone: &str) -> &'static str {
    match tone {
        "calm" => "1.15",
        "storytelling" => "1.0",
        "podcast" => "0.92",
        "academic" => "1.18",
        "energetic" => "0.84",
        _ => "1.0",
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            read_book_file,
            synthesize_with_piper
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
