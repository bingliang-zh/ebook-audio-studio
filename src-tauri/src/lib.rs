use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tauri::{AppHandle, Manager};

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
    piper_path: Option<String>,
    model_id: Option<String>,
    model_path: Option<String>,
    speaker_id: Option<i64>,
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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct BuiltinModel {
    id: String,
    name: String,
    language: String,
    quality: String,
    size: String,
    recommended: bool,
    model_url: String,
    config_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalModel {
    id: String,
    name: String,
    language: String,
    quality: String,
    size: String,
    recommended: bool,
    model_path: String,
    config_path: String,
    speakers: Vec<Speaker>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Speaker {
    id: i64,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupState {
    piper_path: Option<String>,
    models_dir: String,
    builtin_models: Vec<BuiltinModel>,
    local_models: Vec<LocalModel>,
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
fn get_setup_state(app: AppHandle) -> Result<SetupState, String> {
    let models_dir = models_dir(&app)?;
    fs::create_dir_all(&models_dir)
        .map_err(|error| format!("Failed to create model directory: {error}"))?;

    Ok(SetupState {
        piper_path: find_piper_in_path(),
        models_dir: models_dir.to_string_lossy().to_string(),
        builtin_models: builtin_models(),
        local_models: local_models(&app)?,
    })
}

#[tauri::command]
fn download_builtin_model(app: AppHandle, model_id: String) -> Result<LocalModel, String> {
    let model = builtin_models()
        .into_iter()
        .find(|item| item.id == model_id)
        .ok_or_else(|| "Unknown model.".to_string())?;

    let model_dir = models_dir(&app)?.join(&model.id);
    fs::create_dir_all(&model_dir)
        .map_err(|error| format!("Failed to create model directory: {error}"))?;

    let model_path = model_dir.join("model.onnx");
    let config_path = model_dir.join("model.onnx.json");

    download_file(&model.model_url, &model_path)?;
    download_file(&model.config_url, &config_path)?;

    local_model_from_builtin(&model, model_path, config_path)
}

#[tauri::command]
fn synthesize_with_piper(
    app: AppHandle,
    request: SynthesizeRequest,
) -> Result<SynthesizeResult, String> {
    let piper_path = resolve_piper_path(request.piper_path.as_deref())?;
    let model_path = resolve_model_path(&app, &request)?;
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
        .args(speaker_args(request.speaker_id))
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

fn speaker_args(speaker_id: Option<i64>) -> Vec<String> {
    speaker_id
        .map(|id| vec!["--speaker".to_string(), id.to_string()])
        .unwrap_or_default()
}

fn resolve_piper_path(configured_path: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = configured_path.filter(|value| !value.trim().is_empty()) {
        return Ok(PathBuf::from(path));
    }

    find_piper_in_path().map(PathBuf::from).ok_or_else(|| {
        "Piper was not found. Install Piper or choose its executable in Settings.".to_string()
    })
}

fn resolve_model_path(app: &AppHandle, request: &SynthesizeRequest) -> Result<PathBuf, String> {
    if let Some(path) = request
        .model_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(PathBuf::from(path));
    }

    let model_id = request
        .model_id
        .as_deref()
        .ok_or_else(|| "Choose or download a voice model first.".to_string())?;
    let model = builtin_models()
        .into_iter()
        .find(|item| item.id == model_id)
        .ok_or_else(|| "Unknown model.".to_string())?;
    let model_path = models_dir(app)?.join(model.id).join("model.onnx");

    if !model_path.exists() {
        return Err("This model has not been downloaded yet.".to_string());
    }

    Ok(model_path)
}

fn models_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("models"))
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))
}

fn builtin_models() -> Vec<BuiltinModel> {
    vec![
        BuiltinModel {
            id: "en_US-lessac-low".to_string(),
            name: "English - Lessac Low".to_string(),
            language: "en-US".to_string(),
            quality: "Small".to_string(),
            size: "~30 MB".to_string(),
            recommended: true,
            model_url: "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/low/en_US-lessac-low.onnx".to_string(),
            config_url: "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/low/en_US-lessac-low.onnx.json".to_string(),
        },
        BuiltinModel {
            id: "en_US-lessac-medium".to_string(),
            name: "English - Lessac Medium".to_string(),
            language: "en-US".to_string(),
            quality: "Balanced".to_string(),
            size: "~65 MB".to_string(),
            recommended: false,
            model_url: "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx".to_string(),
            config_url: "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/en/en_US/lessac/medium/en_US-lessac-medium.onnx.json".to_string(),
        },
        BuiltinModel {
            id: "zh_CN-huayan-medium".to_string(),
            name: "中文 - Huayan Medium".to_string(),
            language: "zh-CN".to_string(),
            quality: "Balanced".to_string(),
            size: "~65 MB".to_string(),
            recommended: true,
            model_url: "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx".to_string(),
            config_url: "https://huggingface.co/rhasspy/piper-voices/resolve/v1.0.0/zh/zh_CN/huayan/medium/zh_CN-huayan-medium.onnx.json".to_string(),
        },
    ]
}

fn local_models(app: &AppHandle) -> Result<Vec<LocalModel>, String> {
    builtin_models()
        .into_iter()
        .filter_map(|model| {
            let model_dir = models_dir(app).ok()?.join(&model.id);
            let model_path = model_dir.join("model.onnx");
            let config_path = model_dir.join("model.onnx.json");

            if model_path.exists() && config_path.exists() {
                Some(local_model_from_builtin(&model, model_path, config_path))
            } else {
                None
            }
        })
        .collect()
}

fn local_model_from_builtin(
    model: &BuiltinModel,
    model_path: PathBuf,
    config_path: PathBuf,
) -> Result<LocalModel, String> {
    Ok(LocalModel {
        id: model.id.clone(),
        name: model.name.clone(),
        language: model.language.clone(),
        quality: model.quality.clone(),
        size: model.size.clone(),
        recommended: model.recommended,
        speakers: speakers_from_config(&config_path)?,
        model_path: model_path.to_string_lossy().to_string(),
        config_path: config_path.to_string_lossy().to_string(),
    })
}

fn speakers_from_config(config_path: &Path) -> Result<Vec<Speaker>, String> {
    let raw = fs::read_to_string(config_path)
        .map_err(|error| format!("Failed to read model config: {error}"))?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to parse model config: {error}"))?;

    let Some(map) = value
        .get("speaker_id_map")
        .and_then(|item| item.as_object())
    else {
        return Ok(Vec::new());
    };

    let mut speakers = map
        .iter()
        .filter_map(|(name, id)| {
            id.as_i64().map(|speaker_id| Speaker {
                id: speaker_id,
                name: name.clone(),
            })
        })
        .collect::<Vec<_>>();
    speakers.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(speakers)
}

fn download_file(url: &str, path: &Path) -> Result<(), String> {
    let mut response =
        reqwest::blocking::get(url).map_err(|error| format!("Download failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with HTTP {}", response.status()));
    }

    let temp_path = path.with_extension("download");
    let mut file =
        fs::File::create(&temp_path).map_err(|error| format!("Failed to create file: {error}"))?;
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let bytes = response
            .read(&mut buffer)
            .map_err(|error| format!("Failed while downloading: {error}"))?;
        if bytes == 0 {
            break;
        }
        file.write_all(&buffer[..bytes])
            .map_err(|error| format!("Failed to write download: {error}"))?;
    }

    fs::rename(&temp_path, path).map_err(|error| format!("Failed to finish download: {error}"))
}

fn find_piper_in_path() -> Option<String> {
    let executable = if cfg!(windows) { "piper.exe" } else { "piper" };
    let path_var = env::var_os("PATH")?;

    env::split_paths(&path_var)
        .map(|path| path.join(executable))
        .find(|candidate| candidate.exists())
        .map(|candidate| candidate.to_string_lossy().to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_setup_state,
            download_builtin_model,
            read_book_file,
            synthesize_with_piper
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
