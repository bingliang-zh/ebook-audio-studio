import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { basename, join } from "node:path";
import { config } from "../config.js";
import type { TargetLanguage, Tone } from "../types.js";

const audioDir = new URL("../../storage/audio/", import.meta.url);

export interface TtsInput {
  jobId: string;
  text: string;
  language: TargetLanguage;
  tone: Tone;
}

export async function generateAudio(input: TtsInput): Promise<string> {
  if (config.ttsProvider === "macos") {
    return generateMacosAudio(input);
  }

  return generateMockAudio(input);
}

async function generateMockAudio(input: TtsInput): Promise<string> {
  await mkdir(audioDir, { recursive: true });

  const fileName = `${input.jobId}.txt`;
  const filePath = join(audioDir.pathname, fileName);

  await writeFile(
    filePath,
    [
      "This is a mock audio artifact.",
      `Language: ${input.language}`,
      `Tone: ${input.tone}`,
      "",
      input.text
    ].join("\n"),
    "utf8"
  );

  return `/audio/${basename(filePath)}`;
}

async function generateMacosAudio(input: TtsInput): Promise<string> {
  await mkdir(audioDir, { recursive: true });

  const voice = voiceForLanguage(input.language);
  const rate = rateForTone(input.tone);
  const aiffPath = join(audioDir.pathname, `${input.jobId}.aiff`);
  const m4aPath = join(audioDir.pathname, `${input.jobId}.m4a`);

  await runCommand("/usr/bin/say", ["-v", voice, "-r", String(rate), "-o", aiffPath, input.text]);
  await runCommand("/usr/bin/afconvert", ["-f", "m4af", "-d", "aac", aiffPath, m4aPath]);

  return `/audio/${basename(m4aPath)}`;
}

function voiceForLanguage(language: TargetLanguage) {
  const voices: Record<TargetLanguage, string> = {
    "en-US": "Samantha",
    "ja-JP": "Kyoko",
    "ko-KR": "Yuna",
    "zh-CN": "Tingting",
    "zh-TW": "Meijia",
    "fr-FR": "Amelie",
    "de-DE": "Anna",
    "es-ES": "Monica"
  };

  return voices[language];
}

function rateForTone(tone: Tone) {
  const rates: Record<Tone, number> = {
    calm: 165,
    storytelling: 175,
    podcast: 185,
    academic: 160,
    energetic: 205
  };

  return rates[tone];
}

function runCommand(command: string, args: string[]) {
  return new Promise<void>((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"] });
    let stderr = "";

    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk.toString("utf8");
    });

    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve();
        return;
      }

      reject(new Error(`${basename(command)} failed with code ${code}: ${stderr.trim()}`));
    });
  });
}
