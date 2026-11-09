#!/usr/bin/env python3
"""Interactively exercise the Nons desktop Open API."""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
import urllib.error
import urllib.request


DEFAULT_URL = "http://127.0.0.1:46321"
STATUS_LABELS = {
    "stopped": "已停止",
    "loading": "加载中",
    "playing": "播放中",
    "paused": "已暂停",
    "buffering": "缓冲中",
    "error": "错误",
}
PLAYBACK_MODE_LABELS = {
    "local": "本地",
    "stream": "流媒体",
}
REPEAT_LABELS = {
    "off": "关闭",
    "all": "全部",
    "one": "单曲",
}
HELP = """\
命令:
  info, status       读取当前歌曲与播放状态
  play, resume       继续播放
  pause              暂停
  toggle             在播放与暂停之间切换
  stop               停止播放
  next, prev         下一首 / 上一首
  seek <位置>        跳转，如 90、1:30、+10、-10（绝对值为秒/分:秒）
  volume <音量>      设置音量，如 60、60%、0.6
  shuffle <模式>     随机模式 on / off / toggle
  repeat <模式>      循环模式 off / all / one
  queue [页码]       查看队列，每页 20 首
  jump <序号>        播放队列中的指定曲目
  health             读取服务版本与能力
  raw [player|queue] 输出原始 JSON
  help, ?            显示本帮助
  quit, exit, q      退出
"""


class ApiError(RuntimeError):
    def __init__(self, message, *, status=None, code=None):
        super().__init__(message)
        self.status = status
        self.code = code


class OpenApiClient:
    def __init__(self, base_url, timeout):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    def _request(self, method, path, payload=None):
        body = None
        if payload is not None:
            body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        request = urllib.request.Request(
            f"{self.base_url}{path}",
            data=body,
            method=method,
            headers={"Accept": "application/json"},
        )
        if body is not None:
            request.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                raw = response.read()
                return response.status, self._json(raw)
        except urllib.error.HTTPError as error:
            raw = error.read()
            detail = self._json(raw)
            message = detail.get("message") if isinstance(detail, dict) else None
            code = detail.get("code") if isinstance(detail, dict) else None
            raise ApiError(
                message or f"HTTP {error.code}",
                status=error.code,
                code=code,
            ) from error
        except urllib.error.URLError as error:
            reason = getattr(error, "reason", error)
            raise ApiError(
                f"无法连接 {self.base_url}: {reason}\n"
                "请确认 Nons 正在运行，并在“设置 → 网络与服务”启用开放服务。"
            ) from error
        except TimeoutError as error:
            raise ApiError(f"请求 {path} 超时") from error

    @staticmethod
    def _json(raw):
        if not raw:
            return None
        try:
            return json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise ApiError("服务返回了无效的 JSON") from error

    def health(self):
        return self._request("GET", "/v1/health")[1]

    def player(self):
        return self._request("GET", "/v1/player")[1]

    def queue(self):
        return self._request("GET", "/v1/queue")[1]

    def command(self, payload):
        _, result = self._request("POST", "/v1/player/commands", payload)
        if not isinstance(result, dict) or result.get("accepted") is not True:
            raise ApiError("服务未确认播放命令")
        return result


def format_duration(milliseconds):
    total_seconds = max(0, int(milliseconds or 0)) // 1000
    hours, remainder = divmod(total_seconds, 3600)
    minutes, seconds = divmod(remainder, 60)
    if hours:
        return f"{hours}:{minutes:02d}:{seconds:02d}"
    return f"{minutes}:{seconds:02d}"


def format_error(error):
    if error.status is not None:
        code = f" [{error.code}]" if error.code else ""
        return f"HTTP {error.status}{code}: {error}"
    return str(error)


def show_health(client):
    health = client.health()
    capabilities = ", ".join(health.get("capabilities") or [])
    print(f"Nons {health.get('version', '未知版本')}，协议 v{health.get('protocolVersion', '?')}")
    if capabilities:
        print(f"能力: {capabilities}")


def show_player(client, player):
    current = player.get("current")
    status = STATUS_LABELS.get(player.get("status"), player.get("status") or "未知")
    index = player.get("index")
    queue_length = player.get("queueLength", 0)
    queue_position = f"{index + 1}/{queue_length}" if isinstance(index, int) else f"-/{queue_length}"
    print(f"状态: {status}  队列: {queue_position}  队列版本: {player.get('queueRevision', '?')}")
    if not current:
        print("当前没有曲目。")
        if player.get("error"):
            print(f"错误: {player['error']}")
        return

    artists = " / ".join(current.get("artists") or []) or "未知歌手"
    repeat_mode = player.get("repeatMode") or "off"
    print(f"曲目: {current.get('title') or '未知标题'}")
    print(f"歌手: {artists}")
    print(f"专辑: {current.get('album') or '未知专辑'}")
    print(
        "进度: "
        f"{format_duration(player.get('positionMs'))} / "
        f"{format_duration(player.get('durationMs') or current.get('durationMs'))}"
    )
    print(
        f"音量: {round(float(player.get('volume') or 0) * 100)}%  "
        f"随机: {'开' if player.get('shuffle') else '关'}  "
        f"循环: {REPEAT_LABELS.get(repeat_mode, repeat_mode)}"
    )
    details = [f"来源: {current.get('source') or '未知'}"]
    if player.get("actualQuality"):
        details.append(f"音质: {player['actualQuality']}")
    if player.get("playbackMode"):
        details.append(
            f"播放模式: {PLAYBACK_MODE_LABELS.get(player['playbackMode'], player['playbackMode'])}"
        )
    print("  ".join(details))
    print(f"trackId: {current.get('trackId')}")
    if current.get("coverUrl"):
        print(f"封面: {client_url(client, current['coverUrl'])}")
    if player.get("error"):
        print(f"错误: {player['error']}")


def show_queue(queue, page):
    items = queue.get("items") or []
    page_size = 20
    page_count = max(1, (len(items) + page_size - 1) // page_size)
    page = min(max(1, page), page_count)
    start = (page - 1) * page_size
    end = min(start + page_size, len(items))
    print(f"队列共 {len(items)} 首，第 {page}/{page_count} 页")
    for index in range(start, end):
        item = items[index]
        artists = " / ".join(item.get("artists") or []) or "未知歌手"
        print(
            f"{index + 1:>4}. {item.get('title') or '未知标题'} - {artists}  "
            f"[{format_duration(item.get('durationMs'))}]"
        )
    if not items:
        print("队列为空。")


def parse_position(value, current_ms):
    text = value.strip()
    if not text:
        raise ValueError("缺少跳转位置")
    sign = 1
    if text[0] in "+-":
        sign = -1 if text[0] == "-" else 1
        text = text[1:]
    if not text:
        raise ValueError("跳转位置无效")
    parts = text.split(":")
    if len(parts) > 3 or any(not part for part in parts):
        raise ValueError("跳转位置应为秒、分:秒或时:分:秒")
    try:
        numbers = [float(part) for part in parts]
    except ValueError as error:
        raise ValueError("跳转位置包含非数字内容") from error
    if any(not math.isfinite(number) or number < 0 for number in numbers):
        raise ValueError("跳转位置无效")
    seconds = 0.0
    for number in numbers:
        seconds = seconds * 60 + number
    if sign < 0 or value.strip()[0] == "+":
        target_ms = current_ms + sign * round(seconds * 1000)
    else:
        target_ms = round(seconds * 1000)
    return max(0, int(target_ms))


def parse_volume(value):
    text = value.strip()
    percent = text.endswith("%")
    if percent:
        text = text[:-1].strip()
    try:
        number = float(text)
    except ValueError as error:
        raise ValueError("音量应为 0–1 或 0–100%") from error
    if not math.isfinite(number):
        raise ValueError("音量应为有限数值")
    if percent or number > 1:
        number /= 100
    return min(1.0, max(0.0, number))


def parse_boolean(value):
    normalized = value.strip().lower()
    if normalized in {"on", "true", "1", "开", "开启"}:
        return True
    if normalized in {"off", "false", "0", "关", "关闭"}:
        return False
    raise ValueError("随机模式应为 on、off 或 toggle")


def client_url(client, value):
    return f"{client.base_url}{value}" if value.startswith("/") else value


def accepted_and_show(client):
    print("命令已接受。")
    show_player(client, client.player())


def command_with_queue_revision(client, action, fields, retry=True):
    player = client.player()
    payload = {"action": action, "queueRevision": player["queueRevision"], **fields}
    try:
        return client.command(payload)
    except ApiError as error:
        if retry and error.code == "stale_queue":
            player = client.player()
            payload["queueRevision"] = player["queueRevision"]
            return client.command(payload)
        raise


def handle_command(client, parts):
    command = parts[0].lower()
    args = parts[1:]

    if command in {"quit", "exit", "q"}:
        return False
    if command in {"help", "?"}:
        print(HELP, end="")
    elif command in {"info", "status"}:
        show_player(client, client.player())
    elif command in {"play", "resume", "r"}:
        client.command({"action": "resume"})
        accepted_and_show(client)
    elif command in {"pause", "p"}:
        client.command({"action": "pause"})
        accepted_and_show(client)
    elif command in {"toggle", "t"}:
        client.command({"action": "toggle"})
        accepted_and_show(client)
    elif command == "stop":
        client.command({"action": "stop"})
        accepted_and_show(client)
    elif command in {"next", "n"}:
        client.command({"action": "next"})
        accepted_and_show(client)
    elif command in {"prev", "previous", "b"}:
        client.command({"action": "previous"})
        accepted_and_show(client)
    elif command == "seek":
        if not args:
            raise ValueError("用法: seek <秒|分:秒|时:分:秒|+秒|-秒>")
        player = client.player()
        position_ms = parse_position(" ".join(args), player.get("positionMs") or 0)
        client.command({"action": "seek", "positionMs": position_ms})
        accepted_and_show(client)
    elif command == "volume":
        if not args:
            raise ValueError("用法: volume <0–1 或 0–100%>")
        volume = parse_volume(" ".join(args))
        client.command({"action": "volume", "volume": volume})
        accepted_and_show(client)
    elif command == "shuffle":
        if not args:
            raise ValueError("用法: shuffle <on|off|toggle>")
        mode = args[0].lower()
        if mode == "toggle":
            shuffle = not bool(client.player().get("shuffle"))
        else:
            shuffle = parse_boolean(mode)
        command_with_queue_revision(client, "shuffle", {"shuffle": shuffle})
        accepted_and_show(client)
    elif command == "repeat":
        if not args or args[0].lower() not in {"off", "all", "one"}:
            raise ValueError("用法: repeat <off|all|one>")
        command_with_queue_revision(
            client,
            "repeat",
            {"repeatMode": args[0].lower()},
        )
        accepted_and_show(client)
    elif command == "queue":
        page = 1
        if args:
            try:
                page = int(args[0])
            except ValueError as error:
                raise ValueError("队列页码应为整数") from error
        show_queue(client.queue(), page)
    elif command == "jump":
        if not args:
            raise ValueError("用法: jump <队列序号>")
        try:
            position = int(args[0])
        except ValueError as error:
            raise ValueError("队列序号应为整数") from error
        for attempt in range(2):
            queue = client.queue()
            items = queue.get("items") or []
            if not items:
                raise ValueError("队列为空")
            if position < 1 or position > len(items):
                raise ValueError(f"队列序号应在 1–{len(items)} 之间")
            payload = {
                "action": "jump",
                "queueRevision": queue["queueRevision"],
                "trackId": items[position - 1]["trackId"],
            }
            try:
                client.command(payload)
                break
            except ApiError as error:
                if attempt or error.code != "stale_queue":
                    raise
        accepted_and_show(client)
    elif command == "health":
        show_health(client)
    elif command == "raw":
        target = args[0].lower() if args else "player"
        if target == "player":
            value = client.player()
        elif target == "queue":
            value = client.queue()
        elif target == "health":
            value = client.health()
        else:
            raise ValueError("用法: raw [player|queue|health]")
        print(json.dumps(value, ensure_ascii=False, indent=2))
    elif command == "clear":
        print("\033[2J\033[H", end="")
    else:
        raise ValueError(f"未知命令: {command}。输入 help 查看命令。")
    return True


def repl(client):
    print("Nons 开放 API 交互测试")
    print(f"服务地址: {client.base_url}")
    print(HELP, end="")
    try:
        show_health(client)
        show_player(client, client.player())
    except ApiError as error:
        print(f"初始连接失败: {format_error(error)}")
        print("修复后输入 health 或 info 重试。")

    while True:
        try:
            line = input("\nNons> ").strip()
        except (EOFError, KeyboardInterrupt):
            print()
            return 0
        if not line:
            try:
                show_player(client, client.player())
            except ApiError as error:
                print(f"请求失败: {format_error(error)}")
            continue
        try:
            if not handle_command(client, line.split()):
                return 0
        except (ApiError, ValueError) as error:
            print(f"请求失败: {format_error(error) if isinstance(error, ApiError) else error}")
        except KeyboardInterrupt:
            print("\n已取消。")


def main():
    if not sys.stdout.isatty() and hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--url",
        default=os.environ.get("NONS_OPEN_API_URL", DEFAULT_URL),
        help=f"开放 API 地址（默认: {DEFAULT_URL}）",
    )
    parser.add_argument("--timeout", type=float, default=5.0, help="HTTP 超时秒数（默认: 5）")
    parser.add_argument(
        "-c",
        "--command",
        help="执行一条命令后退出，例如 --command 'info' 或 --command 'seek 1:30'",
    )
    options = parser.parse_args()
    if options.timeout <= 0:
        parser.error("--timeout 必须大于 0")

    client = OpenApiClient(options.url, options.timeout)
    if options.command:
        try:
            handle_command(client, options.command.split())
            return 0
        except KeyboardInterrupt:
            print()
            return 130
        except (ApiError, ValueError) as error:
            print(f"请求失败: {format_error(error) if isinstance(error, ApiError) else error}")
            return 1
    return repl(client)


if __name__ == "__main__":
    sys.exit(main())
