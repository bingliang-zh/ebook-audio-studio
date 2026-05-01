import type { TargetLanguage, Tone } from "../types.js";

const languageNames: Record<TargetLanguage, string> = {
  "en-US": "English",
  "ja-JP": "Japanese",
  "ko-KR": "Korean",
  "zh-CN": "Simplified Chinese",
  "zh-TW": "Traditional Chinese",
  "fr-FR": "French",
  "de-DE": "German",
  "es-ES": "Spanish"
};

const toneNames: Record<Tone, string> = {
  calm: "calm and clear",
  storytelling: "narrative and immersive",
  podcast: "conversational podcast style",
  academic: "precise and academic",
  energetic: "bright and energetic"
};

export async function transformTextForAudio(input: {
  text: string;
  language: TargetLanguage;
  tone: Tone;
}): Promise<string> {
  const normalized = input.text.replace(/\s+/g, " ").trim();
  const excerpt = normalized.slice(0, 12000);

  return [
    `Target language: ${languageNames[input.language]}`,
    `Narration tone: ${toneNames[input.tone]}`,
    "",
    excerpt
  ].join("\n");
}

