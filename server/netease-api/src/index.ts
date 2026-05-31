import { createRequire } from "node:module";
import express from "express";
import cors from "cors";

const require = createRequire(import.meta.url);
const { lyric_new, lyric, search } = require("@neteasecloudmusicapienhanced/api");

const BASE_PORT = parseInt(process.argv.find(a => a.startsWith("--port="))?.split("=")[1] || "37562", 10);
const MAX_PORT = BASE_PORT + 2; // try 3 ports

const app = express();
app.use(cors({ origin: true, credentials: true }));
app.use(express.json());

// ── Helpers ─────────────────────────────────────────────────────────

/** Detect if lyric text is in JSON format rather than plain LRC/YRC. */
function isJsonFormatLyric(text: string): boolean {
  if (!text) return false;
  return text.startsWith("{") && (text.includes('"t"') || text.includes('"c"'));
}

// ── GET /api/lyric?id=<neteaseId> ──────────────────────────────────

app.get("/api/lyric", async (req, res) => {
  try {
    const id = req.query.id as string;
    if (!id) {
      res.status(400).json({ error: "id required" });
      return;
    }

    // 1. Try lyric_new first (supports YRC word-level lyrics + translations)
    const resultNew = await lyric_new({ id });
    let lrc = resultNew.body?.lrc?.lyric || "";
    let yrc = resultNew.body?.yrc?.lyric || "";
    let tlyric = resultNew.body?.tlyric?.lyric || "";
    let romalrc = resultNew.body?.romalrc?.lyric || "";
    let ytlrc = resultNew.body?.ytlrc?.lyric || "";
    let yromalrc = resultNew.body?.yromalrc?.lyric || "";

    // 2. If lrc looks like JSON (not real lyric text), fall back to old endpoint
    if (isJsonFormatLyric(lrc)) {
      try {
        const resultOld = await lyric({ id });
        lrc = resultOld.body?.lrc?.lyric || "";
        tlyric = resultOld.body?.tlyric?.lyric || tlyric;
        if (isJsonFormatLyric(yrc) || !yrc) {
          yrc = resultOld.body?.yrc?.lyric || resultOld.body?.tlyric?.lyric || "";
        }
      } catch { /* old endpoint fallback failed, keep original */ }
    }

    // 3. Also clear JSON-format lyrics
    if (isJsonFormatLyric(yrc)) yrc = "";
    if (isJsonFormatLyric(tlyric)) tlyric = "";
    if (isJsonFormatLyric(ytlrc)) ytlrc = "";

    res.json({ lrc, yrc, tlyric, romalrc, ytlrc, yromalrc });
  } catch (err) {
    // If lyric_new errors entirely, try old endpoint as last resort
    try {
      const id = req.query.id as string;
      const resultOld = await lyric({ id });
      res.json({
        lrc: resultOld.body?.lrc?.lyric || "",
        yrc: resultOld.body?.yrc?.lyric || "",
        tlyric: resultOld.body?.tlyric?.lyric || "",
        romalrc: "",
        ytlrc: "",
        yromalrc: "",
      });
    } catch (err2) {
      console.error("[netease-api] lyric error:", err2);
      res.status(500).json({ error: String(err2) });
    }
  }
});

// ── GET /api/search?keywords=<term>&limit=5 ─────────────────────────

app.get("/api/search", async (req, res) => {
  try {
    const keywords = req.query.keywords as string;
    const limit = parseInt(req.query.limit as string || "5", 10);
    if (!keywords) {
      res.status(400).json({ error: "keywords required" });
      return;
    }
    const result = await search({ keywords, type: 1, limit });
    const songs = (result.body?.result?.songs || []).map((s: any) => ({
      id: String(s.id),
      name: s.name || "",
      artists: (s.ar || s.artists || []).map((a: any) => a.name).join("/"),
      album: s.al?.name || s.album?.name || "",
    }));
    res.json({ songs });
  } catch (err) {
    console.error("[netease-api] search error:", err);
    res.status(500).json({ error: String(err) });
  }
});

// ── Start with port fallback ────────────────────────────────────────

async function startServer(): Promise<void> {
  for (let port = BASE_PORT; port <= MAX_PORT; port++) {
    try {
      await new Promise<void>((resolve, reject) => {
        const server = app.listen(port, "127.0.0.1", () => resolve());
        server.on("error", reject);
      });
      // Structured output — Rust parses this line
      console.log(`NETEASE_API_PORT=${port}`);
      console.log(`[netease-api] ready on port ${port}`);
      return;
    } catch (err: any) {
      if (err.code === "EADDRINUSE") {
        console.error(`[netease-api] port ${port} occupied, trying next...`);
        continue;
      }
      throw err;
    }
  }
  console.error("[netease-api] all ports occupied, exiting");
  process.exit(1);
}

startServer();
