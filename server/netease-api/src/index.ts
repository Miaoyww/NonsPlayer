import { createRequire } from "node:module";
import express from "express";
import cors from "cors";

const require = createRequire(import.meta.url);
const { lyric_new, search } = require("@neteasecloudmusicapienhanced/api");

const PORT = parseInt(process.argv.find(a => a.startsWith("--port="))?.split("=")[1] || "25884", 10);

const app = express();
app.use(cors());
app.use(express.json());

// ── GET /api/lyric?id=<neteaseId> ──────────────────────────────

app.get("/api/lyric", async (req, res) => {
  try {
    const id = req.query.id as string;
    if (!id) {
      res.status(400).json({ error: "id required" });
      return;
    }
    const result = await lyric_new({ id });
    const lrc = result.body?.lrc?.lyric || "";
    const yrc = result.body?.yrc?.lyric || "";
    res.json({ lrc, yrc });
  } catch (err) {
    console.error("[netease-api] lyric error:", err);
    res.status(500).json({ error: String(err) });
  }
});

// ── GET /api/search?keywords=<term>&limit=5 ─────────────────────

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

// ── Start ───────────────────────────────────────────────────────

app.listen(PORT, "127.0.0.1", () => {
  // This line signals to Tauri that the server is ready
  console.log(`netease-api ready on port ${PORT}`);
});
