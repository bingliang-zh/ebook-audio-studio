import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { CheckCircle2, FileAudio, FileText, FolderOpen, Loader2, Settings2 } from "lucide-react";
import { useMemo, useState } from "react";

type TargetLanguage =
  | "en-US"
  | "ja-JP"
  | "ko-KR"
  | "zh-CN"
  | "zh-TW"
  | "fr-FR"
  | "de-DE"
  | "es-ES";

type Tone = "calm" | "storytelling" | "podcast" | "academic" | "energetic";
type Status = "idle" | "ready" | "generating" | "done";

interface BookContent {
  fileName: string;
  text: string;
  characterCount: number;
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
  const [bookPath, setBookPath] = useState("");
  const [book, setBook] = useState<BookContent | null>(null);
  const [piperPath, setPiperPath] = useState("");
  const [modelPath, setModelPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [language, setLanguage] = useState<TargetLanguage>("zh-CN");
  const [tone, setTone] = useState<Tone>("storytelling");
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);

  const canGenerate = Boolean(book && piperPath && modelPath && outputPath && status !== "generating");
  const estimatedMinutes = useMemo(() => {
    if (!book) return 0;
    return Math.max(1, Math.round(book.characterCount / 520));
  }, [book]);

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
        setOutputPath(defaultOutputPath(selected));
      }
    } catch (readError) {
      setBook(null);
      setBookPath("");
      setStatus("idle");
      setError(formatError(readError));
    }
  }

  async function choosePiper() {
    const selected = await pickPath({ title: "选择 Piper 可执行文件" });
    if (selected) setPiperPath(selected);
  }

  async function chooseModel() {
    const selected = await pickPath({
      title: "选择 Piper ONNX 模型",
      filters: [{ name: "Piper model", extensions: ["onnx"] }]
    });
    if (selected) setModelPath(selected);
  }

  async function chooseOutput() {
    const selected = await save({
      title: "选择输出 WAV 文件",
      defaultPath: outputPath || "ebook-audio.wav",
      filters: [{ name: "WAV audio", extensions: ["wav"] }]
    });
    if (selected) setOutputPath(selected);
  }

  async function generateAudio() {
    if (!book) return;

    setStatus("generating");
    setError(null);

    try {
      const result = await invoke<SynthesizeResult>("synthesize_with_piper", {
        request: {
          piperPath,
          modelPath,
          outputPath,
          text: book.text,
          language,
          tone
        }
      });
      setOutputPath(result.outputPath);
      setStatus("done");
    } catch (generateError) {
      setStatus("ready");
      setError(formatError(generateError));
    }
  }

  return (
    <main className="app-shell">
      <section className="workspace">
        <div className="upload-panel">
          <div>
            <p className="eyebrow">Ebook Audio Studio</p>
            <h1>本地模型生成电子书音频</h1>
          </div>

          <div className="upload-form">
            <button type="button" className="picker-button" onClick={chooseBook}>
              <FolderOpen aria-hidden="true" />
              选择电子书
            </button>
            <PathValue label="当前文件" value={bookPath} placeholder="支持 .txt / .md / .html" />

            <div className="field-grid">
              <label>
                <span>目标语言</span>
                <select value={language} onChange={(event) => setLanguage(event.target.value as TargetLanguage)}>
                  {languages.map((item) => (
                    <option key={item.value} value={item.value}>
                      {item.label}
                    </option>
                  ))}
                </select>
              </label>

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

            <div className="model-grid">
              <button type="button" className="ghost-button" onClick={choosePiper}>
                <Settings2 aria-hidden="true" />
                选择 Piper
              </button>
              <button type="button" className="ghost-button" onClick={chooseModel}>
                <FileAudio aria-hidden="true" />
                选择模型
              </button>
            </div>

            <PathValue label="Piper 程序" value={piperPath} placeholder="例如 /opt/homebrew/bin/piper" />
            <PathValue label="模型文件" value={modelPath} placeholder="选择 .onnx 模型" />

            <button type="button" className="ghost-button" onClick={chooseOutput}>
              <FileAudio aria-hidden="true" />
              选择输出位置
            </button>
            <PathValue label="输出文件" value={outputPath} placeholder="生成的 .wav 会保存在这里" />

            {error ? <p className="error-text">{error}</p> : null}

            <button type="button" disabled={!canGenerate} onClick={() => void generateAudio()}>
              {status === "generating" ? <Loader2 className="spin" aria-hidden="true" /> : <FileAudio aria-hidden="true" />}
              生成 WAV
            </button>
          </div>
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
            <div className="empty-state">选择电子书、Piper 程序和本地模型后开始生成</div>
          )}
        </div>
      </section>
    </main>
  );
}

function PathValue({ label, value, placeholder }: { label: string; value: string; placeholder: string }) {
  return (
    <div className="path-value">
      <span>{label}</span>
      <code>{value || placeholder}</code>
    </div>
  );
}

async function pickPath(options: Parameters<typeof open>[0]) {
  const selected = await open({ multiple: false, directory: false, ...options });
  if (Array.isArray(selected)) return selected[0] ?? null;
  return selected;
}

function defaultOutputPath(inputPath: string) {
  const outputPath = inputPath.replace(/\.[^.\\/]+$/, ".wav");
  return outputPath === inputPath ? `${inputPath}.wav` : outputPath;
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
