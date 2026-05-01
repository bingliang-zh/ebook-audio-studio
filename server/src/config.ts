import "dotenv/config";

const maxUploadMb = Number.parseInt(process.env.MAX_UPLOAD_MB ?? "50", 10);

export const config = {
  port: Number.parseInt(process.env.PORT ?? "4000", 10),
  publicBaseUrl: process.env.PUBLIC_BASE_URL ?? "http://localhost:4000",
  maxUploadMb: Number.isFinite(maxUploadMb) ? maxUploadMb : 50,
  ttsProvider: process.env.TTS_PROVIDER ?? "mock"
};

