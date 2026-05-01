export type TargetLanguage =
  | "en-US"
  | "ja-JP"
  | "ko-KR"
  | "zh-CN"
  | "zh-TW"
  | "fr-FR"
  | "de-DE"
  | "es-ES";

export type Tone = "calm" | "storytelling" | "podcast" | "academic" | "energetic";

export type JobStatus = "queued" | "processing" | "done" | "failed";

export interface AudioJob {
  id: string;
  fileName: string;
  language: TargetLanguage;
  tone: Tone;
  status: JobStatus;
  createdAt: string;
  updatedAt: string;
  sourceCharacters: number;
  outputCharacters: number;
  audioPath?: string;
  audioUrl?: string;
  error?: string;
}

