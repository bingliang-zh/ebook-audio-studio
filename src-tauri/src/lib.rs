use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
    output_format: String,
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
    ffmpeg_path: Option<String>,
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
        piper_path: find_piper(&app),
        ffmpeg_path: find_ffmpeg(&app),
        models_dir: models_dir.to_string_lossy().to_string(),
        builtin_models: builtin_models(),
        local_models: local_models(&app)?,
    })
}

#[tauri::command]
fn download_piper_engine(app: AppHandle) -> Result<String, String> {
    let package = piper_package()?;
    let engine_dir = engine_dir(&app)?;
    fs::create_dir_all(&engine_dir)
        .map_err(|error| format!("Failed to create engine directory: {error}"))?;

    let archive_path = engine_dir.join(package.file_name);
    download_file(package.url, &archive_path)?;
    extract_archive(&archive_path, &engine_dir)?;

    let piper_path = find_piper_in_dir(&engine_dir).ok_or_else(|| {
        "Downloaded Piper package did not contain a Piper executable.".to_string()
    })?;

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&piper_path)
            .map_err(|error| format!("Failed to inspect Piper executable: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&piper_path, permissions)
            .map_err(|error| format!("Failed to make Piper executable: {error}"))?;
    }

    Ok(piper_path.to_string_lossy().to_string())
}

#[tauri::command]
fn download_ffmpeg_encoder(app: AppHandle) -> Result<String, String> {
    let package = ffmpeg_package()?;
    let encoder_dir = encoder_dir(&app)?;
    fs::create_dir_all(&encoder_dir)
        .map_err(|error| format!("Failed to create encoder directory: {error}"))?;

    let binary_name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let ffmpeg_path = encoder_dir.join(binary_name);
    download_file(package.url, &ffmpeg_path)?;

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&ffmpeg_path)
            .map_err(|error| format!("Failed to inspect FFmpeg executable: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ffmpeg_path, permissions)
            .map_err(|error| format!("Failed to make FFmpeg executable: {error}"))?;
    }

    Ok(ffmpeg_path.to_string_lossy().to_string())
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
    let piper_path = resolve_piper_path(&app, request.piper_path.as_deref())?;
    let model_path = resolve_model_path(&app, &request)?;
    let output_path = PathBuf::from(&request.output_path);

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

    synthesize_text(
        &app,
        &piper_path,
        &model_path,
        request.speaker_id,
        &output_path,
        &request.output_format,
        &request.text,
        &request.tone,
    )?;

    Ok(SynthesizeResult {
        output_path: output_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn synthesize_preview(
    app: AppHandle,
    request: SynthesizeRequest,
) -> Result<SynthesizeResult, String> {
    let piper_path = resolve_piper_path(&app, request.piper_path.as_deref())?;
    let model_path = resolve_model_path(&app, &request)?;
    let preview_dir = app
        .path()
        .app_cache_dir()
        .map(|path| path.join("previews"))
        .map_err(|error| format!("Failed to resolve preview directory: {error}"))?;
    fs::create_dir_all(&preview_dir)
        .map_err(|error| format!("Failed to create preview directory: {error}"))?;

    let extension = extension_for_format(&request.output_format)?;
    let output_path = preview_dir.join(format!("preview.{extension}"));
    let preview_text = request.text.chars().take(360).collect::<String>();

    synthesize_text(
        &app,
        &piper_path,
        &model_path,
        request.speaker_id,
        &output_path,
        &request.output_format,
        &preview_text,
        &request.tone,
    )?;

    Ok(SynthesizeResult {
        output_path: output_path.to_string_lossy().to_string(),
    })
}

fn synthesize_text(
    app: &AppHandle,
    piper_path: &Path,
    model_path: &Path,
    speaker_id: Option<i64>,
    output_path: &Path,
    output_format: &str,
    text: &str,
    tone: &str,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("There is no text to synthesize.".to_string());
    }

    let output_format = normalized_format(output_format)?;
    let wav_path = if output_format == "wav" {
        output_path.to_path_buf()
    } else {
        output_path.with_extension("tmp.wav")
    };

    let prepared_text = prepare_text_for_tts(text);
    let length_scale = length_scale_for_tone(tone);
    let mut child = Command::new(piper_path)
        .arg("--model")
        .arg(model_path)
        .args(speaker_args(speaker_id))
        .arg("--length_scale")
        .arg(length_scale)
        .arg("--output_file")
        .arg(&wav_path)
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

    if output_format == "mp3" {
        convert_wav_to_mp3(app, &wav_path, output_path)?;
        let _ = fs::remove_file(&wav_path);
    }

    Ok(())
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

fn resolve_piper_path(app: &AppHandle, configured_path: Option<&str>) -> Result<PathBuf, String> {
    if let Some(path) = configured_path.filter(|value| !value.trim().is_empty()) {
        return Ok(PathBuf::from(path));
    }

    find_piper(app).map(PathBuf::from).ok_or_else(|| {
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

fn engine_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("engine"))
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))
}

fn encoder_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("encoder"))
        .map_err(|error| format!("Failed to resolve app data directory: {error}"))
}

struct PiperPackage {
    url: &'static str,
    file_name: &'static str,
}

struct FfmpegPackage {
    url: &'static str,
}

fn ffmpeg_package() -> Result<FfmpegPackage, String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("macos", "aarch64") => Ok(FfmpegPackage {
            url: "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-darwin-arm64",
        }),
        ("macos", "x86_64") => Ok(FfmpegPackage {
            url: "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-darwin-x64",
        }),
        ("linux", "x86_64") => Ok(FfmpegPackage {
            url: "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-linux-x64",
        }),
        ("linux", "aarch64") => Ok(FfmpegPackage {
            url: "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-linux-arm64",
        }),
        ("windows", "x86_64") => Ok(FfmpegPackage {
            url: "https://github.com/eugeneware/ffmpeg-static/releases/download/b6.1.1/ffmpeg-win32-x64",
        }),
        _ => Err(format!("Automatic FFmpeg download is not available for {os}/{arch}.")),
    }
}

fn normalized_format(format: &str) -> Result<&'static str, String> {
    match format {
        "mp3" => Ok("mp3"),
        "wav" | "" => Ok("wav"),
        _ => Err("Unsupported output format. Choose WAV or MP3.".to_string()),
    }
}

fn extension_for_format(format: &str) -> Result<&'static str, String> {
    normalized_format(format)
}

fn convert_wav_to_mp3(app: &AppHandle, wav_path: &Path, mp3_path: &Path) -> Result<(), String> {
    let ffmpeg_path = find_ffmpeg(app)
        .map(PathBuf::from)
        .ok_or_else(|| "MP3 output requires FFmpeg. Download the MP3 encoder first.".to_string())?;

    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(wav_path)
        .arg("-codec:a")
        .arg("libmp3lame")
        .arg("-b:a")
        .arg("128k")
        .arg(mp3_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Failed to start FFmpeg: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("FFmpeg failed: {}", stderr.trim()));
    }

    Ok(())
}

fn piper_package() -> Result<PiperPackage, String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("macos", "aarch64") => Ok(PiperPackage {
            url: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_macos_aarch64.tar.gz",
            file_name: "piper_macos_aarch64.tar.gz",
        }),
        ("macos", "x86_64") => Ok(PiperPackage {
            url: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_macos_x64.tar.gz",
            file_name: "piper_macos_x64.tar.gz",
        }),
        ("linux", "x86_64") => Ok(PiperPackage {
            url: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_x86_64.tar.gz",
            file_name: "piper_linux_x86_64.tar.gz",
        }),
        ("linux", "aarch64") => Ok(PiperPackage {
            url: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_linux_aarch64.tar.gz",
            file_name: "piper_linux_aarch64.tar.gz",
        }),
        ("windows", "x86_64") => Ok(PiperPackage {
            url: "https://github.com/rhasspy/piper/releases/download/2023.11.14-2/piper_windows_amd64.zip",
            file_name: "piper_windows_amd64.zip",
        }),
        _ => Err(format!("Automatic Piper download is not available for {os}/{arch}.")),
    }
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file_name = archive_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    if file_name.ends_with(".tar.gz") {
        let archive_file = fs::File::open(archive_path)
            .map_err(|error| format!("Failed to open Piper archive: {error}"))?;
        let decoder = flate2::read::GzDecoder::new(archive_file);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(destination)
            .map_err(|error| format!("Failed to extract Piper archive: {error}"))?;
        return Ok(());
    }

    if file_name.ends_with(".zip") {
        let archive_file = fs::File::open(archive_path)
            .map_err(|error| format!("Failed to open Piper archive: {error}"))?;
        let mut archive = zip::ZipArchive::new(archive_file)
            .map_err(|error| format!("Failed to read Piper zip: {error}"))?;
        archive
            .extract(destination)
            .map_err(|error| format!("Failed to extract Piper zip: {error}"))?;
        return Ok(());
    }

    Err("Unsupported Piper archive format.".to_string())
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

fn find_piper(app: &AppHandle) -> Option<String> {
    engine_dir(app)
        .ok()
        .and_then(|path| find_piper_in_dir(&path))
        .map(|path| path.to_string_lossy().to_string())
        .or_else(find_piper_in_path)
}

fn find_ffmpeg(app: &AppHandle) -> Option<String> {
    encoder_dir(app)
        .ok()
        .map(|path| {
            path.join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            })
        })
        .filter(|candidate| candidate.exists())
        .map(|path| path.to_string_lossy().to_string())
        .or_else(find_ffmpeg_in_path)
}

fn find_ffmpeg_in_path() -> Option<String> {
    let executable = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    let path_var = env::var_os("PATH")?;

    env::split_paths(&path_var)
        .map(|path| path.join(executable))
        .find(|candidate| candidate.exists())
        .map(|candidate| candidate.to_string_lossy().to_string())
}

fn find_piper_in_path() -> Option<String> {
    let executable = if cfg!(windows) { "piper.exe" } else { "piper" };
    let path_var = env::var_os("PATH")?;

    env::split_paths(&path_var)
        .map(|path| path.join(executable))
        .find(|candidate| candidate.exists())
        .map(|candidate| candidate.to_string_lossy().to_string())
}

fn find_piper_in_dir(dir: &Path) -> Option<PathBuf> {
    let executable = if cfg!(windows) { "piper.exe" } else { "piper" };
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(current).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name == executable)
            {
                return Some(path);
            }
        }
    }

    None
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_setup_state,
            download_piper_engine,
            download_ffmpeg_encoder,
            download_builtin_model,
            read_book_file,
            synthesize_preview,
            synthesize_with_piper
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
