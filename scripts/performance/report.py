"""Summarize raw process-tree samples; never combine CPU samples across missing processes."""
import argparse
import json
import math
from pathlib import Path


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def rows(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8-sig").splitlines() if line.strip()]


def stats(values):
    values = sorted(values)
    if not values:
        return {"n": 0, "p50": None, "p95": None, "max": None}
    return {"n": len(values), **{name: round(values[max(0, math.ceil(len(values) * q) - 1)], 3) for name, q in [("p50", .5), ("p95", .95), ("max", 1)]}}


def summarize(root):
    metadata = read_json(root / "metadata.json")
    first_screen, audio, phases, queue_ready = [], [], {}, {}
    for run in sorted(root.glob("run-*")):
        events = rows(run / "events.jsonl")
        if not any(e["event"] == "finished" and e["value"].get("success") for e in events):
            raise ValueError(f"Incomplete run: {run.name}")
        launch = read_json(run / "launch.json")["launchEpochMs"]
        first_screen.extend(e["epochMs"] - launch for e in events if e["event"] == "first_screen")
        audio.extend(e["value"]["firstOutputBufferMs"] for e in events if e["event"] == "play_ready" and e["value"]["count"] == 1)
        for e in events:
            if e["event"] == "play_ready" and e["value"]["count"] > 1:
                queue_ready.setdefault(str(e["value"]["count"]), []).append(e["value"]["frontendReadyMs"])
        samples = rows(run / "samples.jsonl")
        starts = {e["value"]["phase"]: e["epochMs"] for e in events if e["event"] == "phase_start"}
        ends = {e["value"]["phase"]: e["epochMs"] for e in events if e["event"] == "phase_end"}
        for phase, start in starts.items():
            data = phases.setdefault(phase, {"privateMiB": [], "workingSetMiB": [], "cpuOneCorePercent": [], "cpuMachinePercent": []})
            previous = None
            for sample in samples:
                if not start <= sample["epochMs"] <= ends[phase]:
                    continue
                current = {p["pid"]: p for p in sample["processes"]}
                data["privateMiB"].append(sum(p["privateBytes"] for p in current.values()) / 2**20)
                data["workingSetMiB"].append(sum(p["workingSetBytes"] for p in current.values()) / 2**20)
                if previous:
                    elapsed = (sample["epochMs"] - previous[0]) / 1000
                    # Only common process IDs have two counters; transient children are explicitly excluded.
                    cpu = sum(max(0, current[pid]["cpuSeconds"] - previous[1][pid]["cpuSeconds"]) for pid in current.keys() & previous[1].keys())
                    if elapsed > 0:
                        value = cpu / elapsed * 100
                        data["cpuOneCorePercent"].append(value)
                        data["cpuMachinePercent"].append(value / metadata["logicalProcessors"])
                previous = (sample["epochMs"], current)
    result = {"firstScreenMs": stats(first_screen), "warmLocalOutputBufferMs": stats(audio), "queueReadyMs": {count: stats(values) for count, values in queue_ready.items()}, "phases": {phase: {key: stats(values) for key, values in data.items()} for phase, data in phases.items()}}
    (root / "summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    lines = ["# Nons Release 性能基线", "", f"测量时间：{metadata['recordedAt']}。CPU：{', '.join(metadata['cpu'])}；系统：{metadata['os']} {metadata['osVersion']}；内存：{metadata['ramBytes'] / 2**30:.1f} GiB。", "", f"程序 SHA256：`{metadata['sha256']}`；源提交：`{metadata['commit']}`；存在工作区改动：{metadata['dirty']}。", "", "分位数采用 nearest-rank；n 是样本数，小样本 p95 不代表长期尾延迟。启动为新进程和独立空白配置，未清空 OS 文件缓存，不称为冷机启动。", "", "| 指标 | n | p50 | p95 | 最大值 |", "|---|---:|---:|---:|---:|"]
    for label, key in [("启动至首屏框架绘制（ms）", "firstScreenMs"), ("本地热文件请求至首个输出 PCM 缓冲（ms）", "warmLocalOutputBufferMs")]:
        s = result[key]; lines.append(f"| {label} | {s['n']} | {s['p50']} | {s['p95']} | {s['max']} |")
    for count, s in result["queueReadyMs"].items():
        lines.append(f"| {count} 首队列准备至前端收到并播放（ms） | {s['n']} | {s['p50']} | {s['p95']} | {s['max']} |")
    lines += ["", "| 场景 | 私有内存 p50 / p95（MiB） | 工作集 p50 / p95（MiB） | CPU 单核口径 p50 / p95（%） | CPU 整机口径 p50 / p95（%） |", "|---|---:|---:|---:|---:|"]
    for phase, data in result["phases"].items():
        cells = [f"{data[key]['p50']} / {data[key]['p95']}" for key in ["privateMiB", "workingSetMiB", "cpuOneCorePercent", "cpuMachinePercent"]]
        lines.append(f"| {phase} | " + " | ".join(cells) + " |")
    if "runs" in metadata:
        lines += ["", f"配置：{metadata['runs']} 个新进程；每进程 {metadata['audioRuns']} 次单曲请求；每场景稳定 2 秒后采样 {metadata['sampleSeconds']} 秒；进程采样间隔 {metadata['sampleIntervalMs']}ms。"]
    lines += ["", "测量边界：", "", "- 包含播放器及全部可追踪 WebView 子进程；工作集相加可能重复计算共享页。CPU 使用相邻样本共有进程的累计 CPU 差值，短命子进程可能漏计。", "- PCM 时间取输出转换器交给设备 sink 前的首个缓冲，包含预滚，不是声卡回环、扬声器首声或听感延迟。音频固定为 48 kHz 双声道 16-bit 静音 WAV，歌词为本地 LRC。", "- 当前没有音频磁盘播放缓存；本地热文件不能代表在线缓存命中或真实网络取址延迟。", "- 首屏在工作区挂载后两次 requestAnimationFrame 标记，代表框架可绘制，不代表远程音乐库已经加载完成。", "- 测量构建启用 performance-probe，场景控制包含少量轮询；首次队列就绪后等待 2 秒才进行 CPU／内存采样。未设置性能门槛，后续应使用相同环境、窗口、样本和测量配置对比。", ""]
    (root / "report.md").write_text("\n".join(lines), encoding="utf-8")
    return result


def compare(root, baseline):
    before_meta, after_meta = read_json(baseline / "metadata.json"), read_json(root / "metadata.json")
    for key in ["osVersion", "cpu", "logicalProcessors", "ramBytes", "gpu", "soundDevices", "powerScheme", "gstreamerVersion", "sampleSeconds", "sampleIntervalMs", "audioRuns", "runs", "workload"]:
        if before_meta[key] != after_meta[key]:
            raise ValueError(f"Cannot compare different {key}: rerun with the same environment and workload")
    environment = lambda directory: next(e["value"] for e in rows(next(directory.glob("run-*")) / "events.jsonl") if e["event"] == "first_screen")
    before_env, after_env = environment(baseline), environment(root)
    for key in ["viewport", "dpr", "userAgent"]:
        if before_env[key] != after_env[key]:
            raise ValueError(f"Cannot compare different {key}: rerun with the same environment and workload")
    before, after = read_json(baseline / "summary.json"), read_json(root / "summary.json")
    metrics = [("firstScreenMs", before["firstScreenMs"], after["firstScreenMs"]), ("warmLocalOutputBufferMs", before["warmLocalOutputBufferMs"], after["warmLocalOutputBufferMs"])]
    for count, value in after["queueReadyMs"].items():
        metrics.append((f"queue_{count}_ready_ms", before["queueReadyMs"][count], value))
    for phase, data in after["phases"].items():
        for key, value in data.items():
            metrics.append((f"{phase}.{key}", before["phases"][phase][key], value))
    lines = ["# 性能基线对比", "", f"基线：{baseline.name}；本次：{root.name}。正数表示本次更高，负数表示更低；不自动判断通过或失败。", "", "| 指标 | 基线 p50 | 本次 p50 | p50 变化 | p95 变化 |", "|---|---:|---:|---:|---:|"]
    for label, old, new in metrics:
        delta = lambda key: "—" if old[key] in (None, 0) or new[key] is None else f"{(new[key] / old[key] - 1) * 100:+.2f}%"
        lines.append(f"| {label} | {old['p50']} | {new['p50']} | {delta('p50')} | {delta('p95')} |")
    (root / "comparison.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("root", type=Path); parser.add_argument("--compare", type=Path)
    args = parser.parse_args()
    print(json.dumps(summarize(args.root), ensure_ascii=False, indent=2))
    if args.compare:
        compare(args.root, args.compare)
