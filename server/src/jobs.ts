import type { AudioJob, TargetLanguage, Tone } from "./types.js";
import { extractTextFromUpload } from "./services/extractText.js";
import { transformTextForAudio } from "./services/transformText.js";
import { generateAudio } from "./services/tts.js";

const jobs = new Map<string, AudioJob>();

function now() {
  return new Date().toISOString();
}

export function createJob(input: {
  id: string;
  fileName: string;
  filePath: string;
  language: TargetLanguage;
  tone: Tone;
}) {
  const createdAt = now();
  const job: AudioJob = {
    id: input.id,
    fileName: input.fileName,
    language: input.language,
    tone: input.tone,
    status: "queued",
    createdAt,
    updatedAt: createdAt,
    sourceCharacters: 0,
    outputCharacters: 0
  };

  jobs.set(input.id, job);
  void processJob(input.id, input.filePath);
  return job;
}

export function getJob(id: string) {
  return jobs.get(id);
}

export function listJobs() {
  return Array.from(jobs.values()).sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

async function processJob(id: string, filePath: string) {
  const job = jobs.get(id);
  if (!job) return;

  try {
    updateJob(id, { status: "processing" });
    const text = await extractTextFromUpload(filePath, job.fileName);
    const transformed = await transformTextForAudio({
      text,
      language: job.language,
      tone: job.tone
    });
    const audioUrl = await generateAudio({
      jobId: id,
      text: transformed,
      language: job.language,
      tone: job.tone
    });

    updateJob(id, {
      status: "done",
      sourceCharacters: text.length,
      outputCharacters: transformed.length,
      audioUrl
    });
  } catch (error) {
    updateJob(id, {
      status: "failed",
      error: error instanceof Error ? error.message : "Unknown processing error"
    });
  }
}

function updateJob(id: string, patch: Partial<AudioJob>) {
  const existing = jobs.get(id);
  if (!existing) return;
  jobs.set(id, { ...existing, ...patch, updatedAt: now() });
}

