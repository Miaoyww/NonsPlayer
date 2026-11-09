# 桌面开放 API

开放服务供外部应用读取播放信息、获取完整歌词并控制 Nons。服务默认关闭，在“设置 → 网络与服务”启用；默认端口为 `46321`，默认仅本机访问，开启“允许局域网访问”后可使用设置中展示的局域网地址。

服务不使用账号、令牌、配对或客户端身份验证。HTTP 查询与控制使用同一监听地址，WebSocket 用于事件订阅。服务只在桌面端提供，生命周期独立于主窗口的 WebView。

## 接口

以下示例以 `http://127.0.0.1:46321` 为服务地址。所有 JSON 字段采用 camelCase，时间使用整数毫秒。

| 方法 | 路径                   | 用途                               |
| ---- | ---------------------- | ---------------------------------- |
| GET  | `/v1/health`           | 读取应用版本                       |
| GET  | `/v1/player`           | 当前播放状态、曲目、进度与队列版本 |
| GET  | `/v1/queue`            | 播放队列与随机顺序                 |
| POST | `/v1/player/commands`  | 播放控制                           |
| GET  | `/v1/lyrics?trackId=…` | 完整 TTML 歌词                     |
| GET  | `/v1/cover?trackId=…`  | 封面图片                           |
| GET  | `/v1/events`           | WebSocket 事件连接                 |
| GET  | `/v1/docs`             | 查看协议说明                       |

`trackId` 从当前播放或队列响应中取得，是不透明标识。客户端不解析其内容，也不提交本地路径、音频 URL 或上游请求头。

```javascript
const base = 'http://127.0.0.1:46321'
const player = await fetch(`${base}/v1/player`).then((response) => response.json())
console.log(player.current, player.positionMs, player.status)

await fetch(`${base}/v1/player/commands`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ action: 'pause' })
})
```

## 完整歌词

歌词正文的媒体类型为 `application/ttml+xml`，采用 AMLL 兼容的 TTML 约定。一份文档承载原文、逐行或逐词时间、已有翻译和音译。翻译采用 `ttm:role="x-translation"`，音译采用 `ttm:role="x-roman"`；缺少对应内容时不生成辅助文本。

QRC、YRC、LRC 由 Nons 解析并合并独立译注，再输出 TTML。普通 LRC 保持逐行精度。原生 TTML 在无需转换时保留正文；本地 CUE 歌词按分轨范围转换为当前曲目的时间轴。

响应头提供 `X-Nons-Effective-Offset-Ms`、`X-Nons-Lyric-Source`、`X-Nons-Profile-Version` 和 `ETag`。歌词正文不叠加全局或单曲偏移：显示时使用“音频位置减去有效偏移”，点击定位使用“行时间加上有效偏移”，最终跳转位置不小于零。有效偏移可以为负值。

```javascript
if (player.current) {
  const response = await fetch(
    `${base}/v1/lyrics?trackId=${encodeURIComponent(player.current.trackId)}`
  )
  if (response.ok) {
    const offsetMs = Number(response.headers.get('X-Nons-Effective-Offset-Ms'))
    const ttml = await response.text()
    console.log({ offsetMs, ttml })
  }
}
```

客户端可以携带 `If-None-Match` 复用已有正文；收到 `304` 时仍读取最新的偏移响应头。歌词更新事件只通知失效，客户端按需重新请求完整文档。

## 控制与事件

控制请求包括暂停、继续、停止、上一首、下一首、跳转进度、音量、随机模式、循环模式及跳转到队列歌曲。队列内容只读。

请求正文按 `action` 区分：

| action                                                  | 其他字段                      | 说明                                       |
| ------------------------------------------------------- | ----------------------------- | ------------------------------------------ |
| `resume`、`pause`、`toggle`、`stop`、`next`、`previous` | 无                            | 继续、暂停、切换播放、停止、下一首、上一首 |
| `seek`                                                  | `positionMs`                  | 非负整数毫秒                               |
| `volume`                                                | `volume`                      | 0–1 的有限数值                             |
| `shuffle`                                               | `queueRevision`、`shuffle`    | 将随机模式设为指定布尔值                   |
| `repeat`                                                | `queueRevision`、`repeatMode` | 循环模式为 `off`、`all` 或 `one`           |
| `jump`                                                  | `queueRevision`、`trackId`    | 播放队列中的指定曲目                       |

`queueRevision` 从最近的队列或播放响应取得。队列版本变化时返回 `409 stale_queue`，客户端重新读取后再决定是否重试。随机与循环沿用播放器的互斥规则：启用随机会关闭循环，启用循环会关闭随机。

命令由播放 Actor 串行处理，成功返回 `202 {"accepted":true}`；这表示播放核心已处理命令，音源解析、缓冲与实际播放结果继续以状态查询和事件为准。网络中断后不要自动重发 `next`、`previous` 或 `toggle`，先读取最新状态。

WebSocket 地址为 `ws://127.0.0.1:46321/v1/events`。连接后接收初始状态，再处理播放、队列、进度和歌词变化。切歌后以新的曲目标识读取歌词；不根据歌曲名称判断是否同一曲目。

```javascript
const socket = new WebSocket(base.replace(/^http/, 'ws') + '/v1/events')
socket.onmessage = ({ data }) => {
  const event = JSON.parse(data)
  console.log(event.type, event)
}
```

事件中的 `sequence`、播放 `revision` 与 `queueRevision` 分别表达发布批次、播放代次和队列版本。同一批次可以包含多个事件，进度也可以合并，不能将序号缺口直接判断为失步。连接断开或收到明确的失步通知后重新连接并同步快照；旧播放代次的进度不能覆盖当前曲目。

初始事件依次为 `ready`、`player.queue`、`player.state`、`player.progress`。队列事件包含 `items` 和 `shuffleOrder`；状态事件的 `index` 指向同一 `queueRevision` 的队列。歌词失效事件为 `lyrics.invalidated {trackId}`，`trackId` 为 `null` 时重新读取当前曲目的歌词。慢客户端收到 `events.lagged {resync:true}` 后连接会关闭，需要重新连接取得快照。WebSocket 只订阅事件，控制仍使用 HTTP。

错误响应使用稳定的 `code` 与面向人的 `message`。客户端按错误码处理忙碌、不可用、参数无效、队列冲突和资源缺失，不依赖中文文案。

服务允许跨源 HTTP 请求，并向浏览器暴露歌词缓存与偏移响应头。浏览器自身的本地网络访问权限仍受浏览器策略约束。
