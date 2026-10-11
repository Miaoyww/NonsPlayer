# Folia 音频处理与 NonsPlayer 对照

核对日期：2026-10-11。依据本地源码：Folia `fba12079`，NonsPlayer `1658af0`。本文是源码分析，未进行运行时试听或性能测量；后续工作区改动可能改变行为。

## 音频链路

Folia 使用两个 HTMLAudioElement 和同一个 Web Audio AudioContext。每个 deck 的主链为 source → ReplayGain → trim → 三频段 tone → crossfade gain → mix。另有 fade 前取样的回声支路与 ReplayGain 后的分析支路。共享链为 mix → 均衡器 → 音效 → 用户音量 → 播放/暂停渐变 → analyser → 输出。

来源：[deck 图](../../folia-major/src/services/automix/crossfadeGraph.ts#L100)、[共享图](../../folia-major/src/services/playbackGraph.ts#L38)。音量、歌曲增益、混合曲线和暂停渐变独立控制，避免相互覆盖。

NonsPlayer 使用 Rust/GStreamer playbin，渐变通过输出 bin 中的 audioamplify 与 Controller 实现；输出 bin 位于播放缓冲之后。用户音量由 playbin 的 StreamVolume 单独控制。

来源：[transitions.rs](../app/src-tauri/src/playback/transitions.rs#L33)。

## 过渡与我们的渐入渐出

| 项目         | NonsPlayer 当前渐变    | Folia Crossfade                 | Folia AutoMix                      |
| ------------ | ---------------------- | ------------------------------- | ---------------------------------- |
| 同时播放两首 | 否                     | 是                              | 混合接法是，短切接法仅有很短交接   |
| 时间         | 每段 800 ms，短曲缩短  | 设置最大时长 1–25 秒，默认 5 秒 | 根据音乐信息决定，上限 25 秒       |
| 规划         | 线性增益包络           | 看时长、可用尾部、前后静音      | 看节拍、调性、段落、能量、人声等   |
| 音频操作     | 单流淡出/淡入          | 双流交叉增益                    | 交叉增益、频段交接、可选回声与分轨 |
| 播放/暂停    | 与曲目边缘共用渐变功能 | 独立 200 ms 播放/暂停渐变       | 同左                               |

NonsPlayer 的自然换歌是 A 的末尾趋近零，然后 B 从零升起；无缝播放负责避免额外空隙，但不会令两首歌重叠。因此即使没有插入静音样本，开启渐变后衔接附近仍可能听到音量谷。手动切歌、暂停、停止等先执行淡出，再处理延后的命令。

来源：[包络](../app/src-tauri/src/playback/transitions.rs#L254)、[命令延后](../app/src-tauri/src/playback/player.rs#L437)。

Folia Crossfade 提前启动 B，在 A 的尾部让两者重叠。曲线基础为 cos/sin 等功率配对，再乘过渡中部最多衰减 1.5 dB 的余量；两端恢复单曲原增益。等功率曲线不能保证任意两段音频的峰值或主观响度恒定。

来源：[Crossfade 规划](../../folia-major/src/services/automix/crossfadePlanner.ts#L60)、[实际曲线](../../folia-major/src/services/automix/signalAnalysis.ts#L435)。

**代码与说明存在差异：** Crossfade 规划器自称固定形状，但公共执行层仍无条件调用 `planBlendShape`，根据实测出场响度调整交接比例、根据节拍调整时长（候选范围为原时长 ±25%，还受剩余时长限制），并执行响度平衡。因此设置值是规划上限，实际执行并非严格固定秒数和固定中点。

来源：[执行层](../../folia-major/src/services/automix/automixSession.ts#L1018)、[形状调整](../../folia-major/src/services/automix/signalAnalysis.ts#L374)、[共享平衡](../../folia-major/src/services/automix/automixSession.ts#L1191)。

Folia 播放/暂停渐变是独立输出 GainNode，200 ms 线性变化；暂停计时额外留 15 ms，并在实际暂停后保留 150 ms 静音以排空媒体尾部，之后短渐变恢复节点。用代次 token 取消过时操作。

来源：[playbackFade.ts](../../folia-major/src/services/playbackFade.ts#L19)。

## AutoMix

AutoMix 分为分析、规划和执行。`transitionStrategy` 选择 Crossfade 或 AutoMix 规划器；两者返回相同的 TransitionPlan，由同一个 session 执行。session 状态为 idle → armed → fading，进场 deck 提前加载，hold 解除后开始交接，settle 统一清理旧 deck、增益、滤波器、变速和分轨支路。

来源：[策略](../../folia-major/src/services/automix/transitionStrategy.ts#L138)、[会话](../../folia-major/src/services/automix/automixSession.ts)、[绑定](../../folia-major/src/services/automix/useAutomixDecks.ts)。

当前接法有四种：beatCut（约 40 ms 的短交接）、bassSwap（交接低频以减少低音叠加）、tailRide（在衰减尾音下引入下一首）、plainBlend（普通交叉淡化）。调性不合时缩短重叠，不把整首歌移调；可以选择段落/节拍落点和下一首入场位置，并结合人声结束位置控制窗口。

来源：[接法](../../folia-major/src/services/automix/transitionChooser.ts#L22)、[规划](../../folia-major/src/services/automix/transitionPlanner.ts)。

非分轨路径允许用 HTMLAudioElement 的 playbackRate + preservesPitch 调整出场速度；beatCut、过短交接和分轨路径禁用变速。不能仅凭规划结果认定实际执行了变速。

来源：[变速实现](../../folia-major/src/services/automix/tempoBend.ts#L18)、[执行限制](../../folia-major/src/services/automix/automixSession.ts#L992)。

两端分轨均准备好且窗口满足条件时，执行层可以交接 drums、bass、other、vocals 四轨，并通过临时软限幅总线混合。分轨失败或不完整时仍有主音频路径；AutoMix 完全缺少分析、节奏和歌词证据时明确回退默认 5 秒 Crossfade。Crossfade 模式不主动准备分轨。

Beat This! 为可选节拍模型；htdemucs 为可选四轨分离模型，后者通过 Python sidecar 在 CPU 上运行，而非直接在 Web Audio 中推理。没有模型仍能使用常规 DSP 的节奏、能量、调性分析与非分轨接法。分离优先取本地或已有缓存音频，不为此额外下载整曲。当前分离代码仍先解码整曲再截首尾窗口，故峰值内存不能仅按 30 秒窗口估算；缓存窗口存 Int16，使用时转换为 AudioBuffer。

来源：[分轨资源与缓存](../../folia-major/src/services/automix/stems.ts#L88)、[分离运行时](../../folia-major/electron/analysis/htdemucs_runner.py#L68)、[分析线程](../../folia-major/src/services/automix/analysisOffThread.ts#L47)。

切换模式的一个边界：session 的分轨使用条件不检查模式；从 AutoMix 改为 Crossfade 时若双端已缓存分轨，仍可能执行分轨手势。需实机复现确定可见影响，不能从 Crossfade 的说明认定它绝不触发分轨。

来源：[分轨条件](../../folia-major/src/services/automix/automixSession.ts#L980)、[分轨准备门](../../folia-major/src/services/automix/useAutomixDecks.ts#L719)、[无证据回退](../../folia-major/src/services/automix/transitionStrategy.ts#L151)。

## 响度均衡

### 整曲 ReplayGain

模式为 off / track / album，默认关闭。读取本地文件的 ReplayGain 标签，或 Navidrome、在线来源提供的数据。Album 缺增益/峰值字段时分别回退 Track 字段；缺增益时采用 0 dB。线性倍率为 `10^(gainDb/20)`，约 -6 dB 对应振幅减半。

若正增益且有有效峰值，用 `min(gainDb, -20*log10(peak))` 限制放大量。没有峰值便不能用这条规则保证安全。每首歌单独使用 ReplayGain GainNode，切换以 100 ms ramp 平滑处理；这是元数据增益，不是持续压缩动态。

来源：[计算](../../folia-major/src/utils/replayGain.ts#L26)、[应用](../../folia-major/src/hooks/usePlaybackAudioBridge.ts#L81)、[标签](../../folia-major/src/workers/metadataParser.worker.ts#L212)、[默认](../../folia-major/src/stores/usePlaybackStore.ts#L79)。

播放桥接未把 AutoMix 分析出的响度自动补成无标签歌曲的整曲 ReplayGain；缺标签的歌曲仍可能存在曲间响度差。

### 过渡期间平衡与保护

主音频交接每 100 ms 比较双方参考响度，只衰减正在离开的曲目，最大 6 dB；调整以 400 ms ramp 应用，衰减在本次交接中只增不减，结束恢复。已有 profile 时使用其全曲平均值，并计入已应用的 ReplayGain；缺失时读取 live analyser。代码各侧独立 fallback，单侧 profile 缺失时仍可能混用整曲与短时测量。

分析侧使用 K 加权能量与 -0.691 偏移，profile 先合成 mono，排除接近数字静音的帧后取平均。没有完整 EBU R128 integrated loudness 的绝对/相对门控流程，因此这里的 LUFS 标注应理解为用于比较的近似响度。

来源：[平衡](../../folia-major/src/services/automix/automixSession.ts#L280)、[最多 6 dB](../../folia-major/src/services/automix/signalAnalysis.ts#L338)、[K 加权](../../folia-major/src/services/automix/signalAnalysis.ts#L609)、[均值](../../folia-major/src/services/automix/trackProfile.ts#L607)。

普通曲线中部最多预留 1.5 dB；分轨总线使用 0.95 起弯的 tanh WaveShaper 软限幅，oversample 为 none。前者只是固定余量，后者只处理该临时分轨总线，均不构成覆盖后续 EQ/音效的最终真峰值保护。

来源：[余量](../../folia-major/src/services/automix/signalAnalysis.ts#L401)、[分轨软限幅](../../folia-major/src/services/automix/crossfadeGraph.ts#L332)、[共享 EQ/音效在后](../../folia-major/src/services/playbackGraph.ts#L51)。

NonsPlayer 当前 PlaybackOptions 只有 fades / gapless，播放核心尚未接入上述 ReplayGain、曲间响度分析或 AutoMix。

来源：[选项模型](../app/src-tauri/src/model/mod.rs#L20)、[播放核心](../app/src-tauri/src/playback/player.rs)。

## 对 NonsPlayer 的启示

可以保留现有播放/暂停渐变，把 Crossfade 作为独立的双曲目混合能力；ReplayGain 又是独立的每曲增益能力。实现 AutoMix 需要音频分析、双路时钟与缓冲、规划和混合执行，不能仅通过改变当前渐变曲线得到。

双路解码增加过渡期负担；离线 PCM 分析和分轨模型进一步增加内存、计算及运行时分发成本。上述为结构上的预期，未测量本机 CPU、内存或听感，不能据此宣称某一实现更快或更好听。
