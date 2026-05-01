import cors from "cors";
import express from "express";
import multer from "multer";
import { nanoid } from "nanoid";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { config } from "./config.js";
import { createJob, getJob, listJobs } from "./jobs.js";
import type { TargetLanguage, Tone } from "./types.js";

const uploadDir = new URL("../storage/uploads/", import.meta.url);
await mkdir(uploadDir, { recursive: true });

const storage = multer.diskStorage({
  destination: (_request, _file, callback) => callback(null, uploadDir.pathname),
  filename: (_request, file, callback) => callback(null, `${Date.now()}-${nanoid()}-${file.originalname}`)
});

const upload = multer({
  storage,
  limits: {
    fileSize: config.maxUploadMb * 1024 * 1024
  }
});

const app = express();
app.use(cors());
app.use(express.json());
app.use("/audio", express.static(join(process.cwd(), "storage/audio")));

app.get("/api/health", (_request, response) => {
  response.json({ ok: true });
});

app.get("/api/jobs", (_request, response) => {
  response.json({ jobs: listJobs() });
});

app.get("/api/jobs/:id", (request, response) => {
  const job = getJob(request.params.id);
  if (!job) {
    response.status(404).json({ error: "Job not found" });
    return;
  }

  response.json({ job });
});

app.post("/api/jobs", upload.single("ebook"), (request, response) => {
  if (!request.file) {
    response.status(400).json({ error: "Missing ebook file" });
    return;
  }

  const language = request.body.language as TargetLanguage | undefined;
  const tone = request.body.tone as Tone | undefined;

  if (!language || !tone) {
    response.status(400).json({ error: "Missing target language or tone" });
    return;
  }

  const job = createJob({
    id: nanoid(),
    fileName: request.file.originalname,
    filePath: request.file.path,
    language,
    tone
  });

  response.status(202).json({ job });
});

app.listen(config.port, () => {
  console.log(`Server listening on http://localhost:${config.port}`);
});

