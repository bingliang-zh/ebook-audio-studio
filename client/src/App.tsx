import { Download, FileAudio, Loader2, Upload } from "lucide-react";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { apiUrl, assetUrl } from "./api";

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

type JobStatus = "queued" | "processing" | "done" | "failed";

interface AudioJob {
  id: string;
  fileName: string;
  language: TargetLanguage;
  tone: Tone;
  status: JobStatus;
  createdAt: string;
  updatedAt: string;
  sourceCharacters: number;
  outputCharacters: number;
  audioUrl?: string;
  error?: string;
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
  const [file, setFile] = useState<File | null>(null);
  const [language, setLanguage] = useState<TargetLanguage>("zh-CN");
  const [tone, setTone] = useState<Tone>("storytelling");
  const [jobs, setJobs] = useState<AudioJob[]>([]);
  const [isUploading, setIsUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeJobs = useMemo(
    () => jobs.some((job) => job.status === "queued" || job.status === "processing"),
    [jobs]
  );

  useEffect(() => {
    void loadJobs();
  }, []);

  useEffect(() => {
    if (!activeJobs) return;

    const timer = window.setInterval(() => {
      void loadJobs();
    }, 1500);

    return () => window.clearInterval(timer);
  }, [activeJobs]);

  async function loadJobs() {
    const response = await fetch(apiUrl("/api/jobs"));
    const payload = (await response.json()) as { jobs: AudioJob[] };
    setJobs(payload.jobs);
  }

  async function submitJob(event: FormEvent) {
    event.preventDefault();
    if (!file) return;

    setIsUploading(true);
    setError(null);

    try {
      const body = new FormData();
      body.append("ebook", file);
      body.append("language", language);
      body.append("tone", tone);

      const response = await fetch(apiUrl("/api/jobs"), {
        method: "POST",
        body
      });

      const payload = await response.json();
      if (!response.ok) {
        throw new Error(payload.error ?? "上传失败");
      }

      setFile(null);
      await loadJobs();
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : "上传失败");
    } finally {
      setIsUploading(false);
    }
  }

  return (
    <main className="app-shell">
      <section className="workspace">
        <div className="upload-panel">
          <div>
            <p className="eyebrow">Ebook Audio Studio</p>
            <h1>把电子书转换成可离线收听的音频</h1>
          </div>

          <form onSubmit={submitJob} className="upload-form">
            <label className="drop-zone">
              <Upload aria-hidden="true" />
              <span>{file ? file.name : "选择电子书文件"}</span>
              <small>支持 .txt / .md / .html，PDF 和 EPUB 已预留解析入口</small>
              <input
                type="file"
                accept=".txt,.md,.markdown,.html,.htm,.pdf,.epub"
                onChange={(event) => setFile(event.target.files?.[0] ?? null)}
              />
            </label>

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

            {error ? <p className="error-text">{error}</p> : null}

            <button type="submit" disabled={!file || isUploading}>
              {isUploading ? <Loader2 className="spin" aria-hidden="true" /> : <FileAudio aria-hidden="true" />}
              生成音频
            </button>
          </form>
        </div>

        <div className="jobs-panel">
          <div className="panel-heading">
            <h2>任务</h2>
            <button type="button" className="ghost-button" onClick={() => void loadJobs()}>
              刷新
            </button>
          </div>

          <div className="job-list">
            {jobs.length === 0 ? (
              <div className="empty-state">还没有任务</div>
            ) : (
              jobs.map((job) => <JobRow key={job.id} job={job} />)
            )}
          </div>
        </div>
      </section>
    </main>
  );
}

function JobRow({ job }: { job: AudioJob }) {
  const audioUrl = job.audioUrl ? assetUrl(job.audioUrl) : undefined;

  return (
    <article className="job-row">
      <div className="job-main">
        <FileAudio aria-hidden="true" />
        <div>
          <h3>{job.fileName}</h3>
          <p>
            {job.language} · {job.tone} · {statusLabel(job.status)}
          </p>
          {job.error ? <p className="error-text">{job.error}</p> : null}
        </div>
      </div>

      {job.status === "done" && audioUrl ? (
        <div className="audio-actions">
          <audio controls src={audioUrl} />
          <a href={audioUrl} download>
            <Download aria-hidden="true" />
            下载
          </a>
        </div>
      ) : null}
    </article>
  );
}

function statusLabel(status: JobStatus) {
  const labels: Record<JobStatus, string> = {
    queued: "排队中",
    processing: "处理中",
    done: "已完成",
    failed: "失败"
  };

  return labels[status];
}
