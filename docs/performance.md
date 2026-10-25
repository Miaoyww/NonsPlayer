# 可比较的 Windows Release 性能基线

性能探测使用真实 WebView、播放 Actor、WebAudio 与系统 WebView；CPU／内存覆盖 Nons 及可追踪的全部 WebView 子进程。只在显式编译 `performance-probe` 且设置绝对路径 `NONS_PERFORMANCE_ROOT` 时启用，普通启动不运行场景。每轮创建独立数据目录，关闭匿名统计与自动检查更新，不读取用户账号、曲库或队列。

## 构建和测量

在仓库根目录执行：

```powershell
pnpm --dir app format
pnpm --dir app tauri build --features performance-probe --no-bundle
./scripts/performance/measure.ps1 -Executable ./app/src-tauri/target/release/Nons.exe -Runs 5 -SampleSeconds 20 -AudioRuns 5
```

保留相同机器、供电模式、分辨率／缩放、窗口大小、音频输出设备及后台应用负载。测量期间不手动操作探测窗口。场景依次为首屏框架、空闲、普通播放、隐藏窗口后台播放、真实 AMLL 歌词页、1000 首和 10000 首队列。队列页使用真实虚拟列表；歌曲使用不同曲目 key、同一份固定 5 分钟音频，歌词为 300 行本地 LRC。并不模拟万首不同音频、封面或大型本地扫描。

默认每个进程进行 5 次单曲播放请求，每个 CPU／内存场景稳定 2 秒后采样 20 秒，共运行 5 个新进程。内存每秒采样；CPU 使用相邻样本相同进程的累计 CPU 时间差，短命子进程可能漏计。单核口径 100% 表示一个逻辑核心满载，整机口径除以逻辑核心数。私有字节与工作集分别记录，工作集求和可能重复计算共享页。

输出默认在 `.local/performance/<本机时间>/`，包括机器／构建元数据、每轮启动时间、事件 JSONL、逐进程采样 JSONL、汇总 JSON 和 Markdown 报告。源码提交号、工作区状态及 EXE SHA256 一起记录；未经提交的构建不能仅凭提交号复现。测量不能与构建、测试或其他高负载任务并行。

## 时间指标的边界

- **启动至首屏框架绘制**：进程启动调用前的系统时间，到真实工作区挂载后两次 `requestAnimationFrame` 的标记；包含进程启动、宿主初始化和 WebView 初始化，不代表远程音乐库加载完成。每轮为空白配置，未清空 OS 文件缓存，因此是新进程启动，不是冷机启动。
- **本地热文件请求至浏览器确认播放**：固定本地 WAV 经播放命令队列和资源准备，浏览器开始播放后回传事件，Rust 接受有效反馈时记录 `firstBrowserPlayingMs`。包含 IPC，不等同于首个 PCM 缓冲、声卡回环或扬声器实际首声。
- **大队列就绪**：前端发出探测请求至收到指定长度的真实队列并进入播放，包含测量专用固定曲目写入、IPC、队列发布和约 50ms 的观测轮询；与普通歌单网络取数分开。
- 当前没有音频磁盘播放缓存，本地热文件不代表在线音频缓存命中。真实在线取址／播放需另记账号、网络和来源，不能与确定性本地基线混算。

## 比较和门槛

```powershell
python scripts/performance/report.py .local/performance/<本次> --compare .local/performance/<基线>
```

比较器核对系统、CPU、内存、GPU、运行次数和采样配置，输出 p50／p95 的变化；不匹配时拒绝计算。分位数使用 nearest-rank，p95 在小样本中可能等于最大值。还需人工核对报告中的视口／缩放、音频设备和 WebView 版本，不能仅凭百分比认定回归。先保存实测基线，再根据重复测量的波动确定预算和门槛；当前没有自动失败阈值。

这套基线不替代 Windows 10／8GB 目标设备、macOS／Linux、真实音乐听感、网络来源、物理设备输出与跨版本更新验收。
