import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  CheckCircle2,
  ChevronDown,
  Download,
  FileAudio,
  FileText,
  FolderOpen,
  Loader2,
  Settings2
} from "lucide-react";
import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";

type TargetLanguage = "en-US" | "zh-CN" | "ja-JP" | "ko-KR" | "zh-TW" | "fr-FR" | "de-DE" | "es-ES";
type Tone = "calm" | "storytelling" | "podcast" | "academic" | "energetic";
type Status = "idle" | "ready" | "generating" | "done";
type OutputFormat = "mp3" | "wav";

interface BookContent {
  fileName: string;
  text: string;
  characterCount: number;
}

interface BuiltinModel {
  id: string;
  name: string;
  language: string;
  quality: string;
  size: string;
  recommended: boolean;
}

interface LocalModel extends BuiltinModel {
  modelPath: string;
  configPath: string;
  speakers: Speaker[];
}

interface Speaker {
  id: number;
  name: string;
}

interface SetupState {
  piperPath?: string;
  ffmpegPath?: string;
  modelsDir: string;
  builtinModels: BuiltinModel[];
  localModels: LocalModel[];
}

interface SynthesizeResult {
  outputPath: string;
}

const languages: Array<{ value: TargetLanguage; label: string }> = [
  { value: "zh-CN", label: "中文简体" },
  { value: "en-US", label: "English" },
  { value: "ja-JP", label: "日本語" },
  { value: "ko-KR", label: "한국어" },
  { value: "zh-TW", label: "中文繁体" },
  { value: "fr-FR", label: "Français" },
  { value: "de-DE", label: "Deutsch" },
  { value: "es-ES", label: "Español" }
];

const tones: Array<{ value: Tone; label: string }> = [
  { value: "calm", label: "平静清晰" },
  { value: "storytelling", label: "故事叙述" },
  { value: "podcast", label: "播客聊天" },
  { value: "academic", label: "学术精确" },
  { value: "energetic", label: "轻快有活力" }
];

export function App() {
  const [setup, setSetup] = useState<SetupState | null>(null);
  const [bookPath, setBookPath] = useState("");
  const [book, setBook] = useState<BookContent | null>(null);
  const [manualPiperPath, setManualPiperPath] = useState("");
  const [manualModelPath, setManualModelPath] = useState("");
  const [selectedModelId, setSelectedModelId] = useState("");
  const [speakerId, setSpeakerId] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [outputFormat, setOutputFormat] = useState<OutputFormat>("mp3");
  const [language, setLanguage] = useState<TargetLanguage>("zh-CN");
  const [tone, setTone] = useState<Tone>("storytelling");
  const [status, setStatus] = useState<Status>("idle");
  const [isDownloadingEngine, setIsDownloadingEngine] = useState(false);
  const [isDownloadingEncoder, setIsDownloadingEncoder] = useState(false);
  const [isDownloading, setIsDownloading] = useState<string | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [previewUrl, setPreviewUrl] = useState("");
  const [showSettings, setShowSettings] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refreshSetup();
  }, []);

  useEffect(() => {
    if (!setup || selectedModelId) return;

    const recommendedLocal = setup.localModels.find((model) => model.recommended) ?? setup.localModels[0];
    if (recommendedLocal) {
      setSelectedModelId(recommendedLocal.id);
      setLanguage(recommendedLocal.language as TargetLanguage);
    }
  }, [selectedModelId, setup]);

  const selectedModel = useMemo(
    () => setup?.localModels.find((model) => model.id === selectedModelId),
    [selectedModelId, setup]
  );
  const piperPath = manualPiperPath || setup?.piperPath || "";
  const ffmpegPath = setup?.ffmpegPath || "";
  const modelPath = manualModelPath || selectedModel?.modelPath || "";
  const hasEncoderForFormat = outputFormat === "wav" || Boolean(ffmpegPath);
  const canGenerate = Boolean(
    book && outputPath && piperPath && hasEncoderForFormat && (selectedModelId || manualModelPath) && status !== "generating"
  );
  const canPreview = Boolean(
    piperPath && hasEncoderForFormat && (selectedModelId || manualModelPath) && !isPreviewing && status !== "generating"
  );
  const estimatedMinutes = useMemo(() => {
    if (!book) return 0;
    return Math.max(1, Math.round(book.characterCount / 520));
  }, [book]);

  async function refreshSetup() {
    try {
      const state = await invoke<SetupState>("get_setup_state");
      setSetup(state);
    } catch (setupError) {
      setError(formatError(setupError));
    }
  }

  async function chooseBook() {
    setError(null);
    const selected = await pickPath({
      title: "选择电子书",
      filters: [{ name: "Text ebook", extensions: ["txt", "md", "markdown", "html", "htm"] }]
    });

    if (!selected) return;

    try {
      const content = await invoke<BookContent>("read_book_file", { path: selected });
      setBookPath(selected);
      setBook(content);
      setStatus("ready");

      if (!outputPath) {
        setOutputPath(defaultOutputPath(selected, outputFormat));
      }
    } catch (readError) {
      setBook(null);
      setBookPath("");
      setStatus("idle");
      setError(formatError(readError));
    }
  }

  async function downloadEngine() {
    setError(null);
    setIsDownloadingEngine(true);

    try {
      const path = await invoke<string>("download_piper_engine");
      setManualPiperPath(path);
      await refreshSetup();
    } catch (downloadError) {
      setError(formatError(downloadError));
    } finally {
      setIsDownloadingEngine(false);
    }
  }

  async function downloadModel(modelId: string) {
    setError(null);
    setIsDownloading(modelId);

    try {
      const model = await invoke<LocalModel>("download_builtin_model", { modelId });
      await refreshSetup();
      setSelectedModelId(model.id);
      setLanguage(model.language as TargetLanguage);
    } catch (downloadError) {
      setError(formatError(downloadError));
    } finally {
      setIsDownloading(null);
    }
  }

  async function downloadEncoder() {
    setError(null);
    setIsDownloadingEncoder(true);

    try {
      await invoke<string>("download_ffmpeg_encoder");
      await refreshSetup();
    } catch (downloadError) {
      setError(formatError(downloadError));
    } finally {
      setIsDownloadingEncoder(false);
    }
  }

  async function choosePiper() {
    const selected = await pickPath({ title: "选择 Piper 可执行文件" });
    if (selected) setManualPiperPath(selected);
  }

  async function chooseModel() {
    const selected = await pickPath({
      title: "选择 Piper ONNX 模型",
      filters: [{ name: "Piper model", extensions: ["onnx"] }]
    });
    if (selected) {
      setManualModelPath(selected);
      setSelectedModelId("");
      setSpeakerId("");
    }
  }

  async function chooseOutput() {
    const selected = await save({
      title: `选择输出 ${outputFormat.toUpperCase()} 文件`,
      defaultPath: outputPath || `ebook-audio.${outputFormat}`,
      filters:
        outputFormat === "mp3"
          ? [{ name: "MP3 audio", extensions: ["mp3"] }]
          : [{ name: "WAV audio", extensions: ["wav"] }]
    });
    if (selected) setOutputPath(selected);
  }

  function changeOutputFormat(format: OutputFormat) {
    setOutputFormat(format);
    setPreviewUrl("");
    if (outputPath) {
      setOutputPath(replaceExtension(outputPath, format));
    } else if (bookPath) {
      setOutputPath(defaultOutputPath(bookPath, format));
    }
  }

  async function previewAudio() {
    const text = book?.text || previewText(language);

    setIsPreviewing(true);
    setError(null);
    setPreviewUrl("");

    try {
      const result = await invoke<SynthesizeResult>("synthesize_preview", {
        request: buildSynthesizeRequest(text, "")
      });
      setPreviewUrl(convertFileSrc(result.outputPath));
    } catch (previewError) {
      setError(formatError(previewError));
    } finally {
      setIsPreviewing(false);
    }
  }

  async function generateAudio() {
    if (!book) return;

    setStatus("generating");
    setError(null);

    try {
      const result = await invoke<SynthesizeResult>("synthesize_with_piper", {
        request: buildSynthesizeRequest(book.text, outputPath)
      });
      setOutputPath(result.outputPath);
      setStatus("done");
    } catch (generateError) {
      setStatus("ready");
      setError(formatError(generateError));
    }
  }

  function buildSynthesizeRequest(text: string, targetOutputPath: string) {
    return {
      piperPath: piperPath || null,
      modelId: manualModelPath ? null : selectedModelId,
      modelPath: manualModelPath || null,
      speakerId: speakerId ? Number(speakerId) : null,
      outputPath: targetOutputPath,
      outputFormat,
      text,
      language,
      tone
    };
  }

  return (
    <main className="app-shell">
      <section className="workspace">
        <div className="upload-panel">
          <div>
            <p className="eyebrow">Ebook Audio Studio</p>
            <h1>开箱即用的本地电子书转音频</h1>
          </div>

          <div className="setup-stack">
            <SetupStep index="1" title="准备声音模型">
              <EngineStatus
                piperPath={piperPath}
                isDownloading={isDownloadingEngine}
                onDownload={() => void downloadEngine()}
                onChoose={choosePiper}
              />
            </SetupStep>

            <SetupStep index="2" title="下载声音模型">
              <div className="model-list">
                {setup?.builtinModels.map((model) => {
                  const local = setup.localModels.find((item) => item.id === model.id);
                  const isActive = selectedModelId === model.id;

                  return (
                    <button
                      key={model.id}
                      type="button"
                      className={`model-card ${isActive ? "is-active" : ""}`}
                      onClick={() => {
                        if (local) {
                          setSelectedModelId(local.id);
                          setManualModelPath("");
                          setLanguage(local.language as TargetLanguage);
                        } else {
                          void downloadModel(model.id);
                        }
                      }}
                    >
                      <span>
                        <strong>{model.name}</strong>
                        <small>
                          {model.quality} · {model.size}
                        </small>
                      </span>
                      {local ? (
                        <CheckCircle2 aria-hidden="true" />
                      ) : isDownloading === model.id ? (
                        <Loader2 className="spin" aria-hidden="true" />
                      ) : (
                        <Download aria-hidden="true" />
                      )}
                    </button>
                  );
                })}
              </div>
            </SetupStep>

            <SetupStep index="3" title="选择电子书">
              <button type="button" className="picker-button" onClick={chooseBook}>
                <FolderOpen aria-hidden="true" />
                选择 .txt / .md / .html
              </button>
              <PathValue value={bookPath} placeholder="还没有选择电子书" />
            </SetupStep>

            <SetupStep index="4" title="生成音频">
              <div className="field-grid">
                <label>
                  <span>语言</span>
                  <select value={language} onChange={(event) => setLanguage(event.target.value as TargetLanguage)}>
                    {languages.map((item) => (
                      <option key={item.value} value={item.value}>
                        {item.label}
                      </option>
                    ))}
                  </select>
                </label>

                <label>
                  <span>格式</span>
                  <select value={outputFormat} onChange={(event) => changeOutputFormat(event.target.value as OutputFormat)}>
                    <option value="mp3">MP3</option>
                    <option value="wav">WAV</option>
                  </select>
                </label>
              </div>

              <div className="field-grid">
                <label>
                  <span>Tone</span>
                  <select value={tone} onChange={(event) => setTone(event.target.value as Tone)}>
                    {tones.map((item) => (
                      <option key={item.value} value={item.value}>
                        {item.label}
                      </option>
                    ))}
                  </select>
                </label>
              </div>

              {outputFormat === "mp3" && !ffmpegPath ? (
                <div className="encoder-card">
                  <span>MP3 需要下载一次编码器。</span>
                  <button type="button" onClick={() => void downloadEncoder()} disabled={isDownloadingEncoder}>
                    {isDownloadingEncoder ? <Loader2 className="spin" aria-hidden="true" /> : <Download aria-hidden="true" />}
                    下载 MP3 编码器
                  </button>
                </div>
              ) : null}

              {selectedModel?.speakers.length ? (
                <label className="full-field">
                  <span>Speaker</span>
                  <select value={speakerId} onChange={(event) => setSpeakerId(event.target.value)}>
                    <option value="">默认 speaker</option>
                    {selectedModel.speakers.map((speaker) => (
                      <option key={speaker.id} value={speaker.id}>
                        {speaker.name} · #{speaker.id}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}

              <button type="button" className="ghost-button" onClick={chooseOutput}>
                <FileAudio aria-hidden="true" />
                选择输出位置
              </button>
              <PathValue value={outputPath} placeholder={`生成的 .${outputFormat} 会保存在这里`} />

              {error ? <p className="error-text">{error}</p> : null}

              <button type="button" className="ghost-button" disabled={!canPreview} onClick={() => void previewAudio()}>
                {isPreviewing ? <Loader2 className="spin" aria-hidden="true" /> : <FileAudio aria-hidden="true" />}
                预览声音
              </button>
              {previewUrl ? (
                <audio className="audio-preview" controls src={previewUrl}>
                  你的系统不支持音频预览。
                </audio>
              ) : null}

              <button type="button" disabled={!canGenerate} onClick={() => void generateAudio()}>
                {status === "generating" ? <Loader2 className="spin" aria-hidden="true" /> : <FileAudio aria-hidden="true" />}
                生成 {outputFormat.toUpperCase()}
              </button>
            </SetupStep>
          </div>

          <button type="button" className="settings-toggle" onClick={() => setShowSettings((value) => !value)}>
            <Settings2 aria-hidden="true" />
            Settings
            <ChevronDown aria-hidden="true" />
          </button>

          {showSettings ? (
            <div className="settings-panel">
              <PathValue label="Piper" value={piperPath} placeholder="未检测到 Piper，请手动选择" />
              <PathValue label="MP3 编码器" value={ffmpegPath} placeholder="未检测到 FFmpeg，选择 MP3 时可一键下载" />
              <button type="button" className="ghost-button" onClick={choosePiper}>
                选择 Piper 程序
              </button>
              <PathValue label="模型目录" value={setup?.modelsDir ?? ""} placeholder="模型目录加载中" />
              <PathValue label="当前模型" value={modelPath} placeholder="未选择模型" />
              <button type="button" className="ghost-button" onClick={chooseModel}>
                手动选择 .onnx 模型
              </button>
            </div>
          ) : null}
        </div>

        <div className="jobs-panel">
          <div className="panel-heading">
            <h2>本地任务</h2>
            <span className="status-pill">{statusLabel(status)}</span>
          </div>

          {book ? (
            <article className="job-row">
              <div className="job-main">
                <FileText aria-hidden="true" />
                <div>
                  <h3>{book.fileName}</h3>
                  <p>
                    {book.characterCount.toLocaleString()} 字符 · 约 {estimatedMinutes} 分钟
                  </p>
                </div>
              </div>

              <div className="preview-text">{book.text.slice(0, 2000)}</div>

              {status === "done" ? (
                <div className="success-banner">
                  <CheckCircle2 aria-hidden="true" />
                  <span>已生成：{outputPath}</span>
                </div>
              ) : null}
            </article>
          ) : (
            <div className="empty-state">下载一个模型，选择电子书，然后生成音频</div>
          )}
        </div>
      </section>
    </main>
  );
}

function SetupStep({ index, title, children }: { index: string; title: string; children: ReactNode }) {
  return (
    <section className="setup-step">
      <div className="step-heading">
        <span>{index}</span>
        <h2>{title}</h2>
      </div>
      {children}
    </section>
  );
}

function EngineStatus({
  piperPath,
  isDownloading,
  onDownload,
  onChoose
}: {
  piperPath: string;
  isDownloading: boolean;
  onDownload: () => void;
  onChoose: () => void;
}) {
  if (piperPath) {
    return (
      <div className="engine-status is-ready">
        <CheckCircle2 aria-hidden="true" />
        <span>已找到本机 Piper 引擎</span>
      </div>
    );
  }

  return (
    <div className="engine-status">
      <Settings2 aria-hidden="true" />
      <span>未检测到本地引擎。下载后即可离线生成音频。</span>
      <button type="button" onClick={onDownload} disabled={isDownloading}>
        {isDownloading ? <Loader2 className="spin" aria-hidden="true" /> : <Download aria-hidden="true" />}
        下载引擎
      </button>
      <button type="button" className="ghost-button" onClick={onChoose} disabled={isDownloading}>
        选择 Piper
      </button>
    </div>
  );
}

function PathValue({ label, value, placeholder }: { label?: string; value: string; placeholder: string }) {
  return (
    <div className="path-value">
      {label ? <span>{label}</span> : null}
      <code>{value || placeholder}</code>
    </div>
  );
}

async function pickPath(options: Parameters<typeof open>[0]) {
  const selected = await open({ multiple: false, directory: false, ...options });
  if (Array.isArray(selected)) return selected[0] ?? null;
  return selected;
}

function defaultOutputPath(inputPath: string, format: OutputFormat) {
  return replaceExtension(inputPath, format);
}

function replaceExtension(inputPath: string, format: OutputFormat) {
  const outputPath = inputPath.replace(/\.[^.\\/]+$/, `.${format}`);
  return outputPath === inputPath ? `${inputPath}.${format}` : outputPath;
}

function previewText(language: TargetLanguage) {
  if (language === "zh-CN" || language === "zh-TW") {
    return "这是一段声音预览。你可以用它检查语速、语气和说话人是否适合长时间收听。";
  }

  return "This is a short voice preview. Use it to check the voice, tone, and speaking speed before generating the full audiobook.";
}

function statusLabel(status: Status) {
  const labels: Record<Status, string> = {
    idle: "等待配置",
    ready: "可以生成",
    generating: "生成中",
    done: "已完成"
  };

  return labels[status];
}

function formatError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
