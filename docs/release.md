# GitHub Release CI

发布流程由 [.github/workflows/publish.yml](../.github/workflows/publish.yml) 定义：推送 `v<主版本>.<次版本>.<补丁版本>` 标签后并行构建 Windows x64、Android ARM64 试用 APK 和 Linux x64 安装包，上传 Actions Artifact，再发布到 GitHub Release 与 Cloudflare R2。Windows x64 安装版通过 Tauri updater 插件更新，便携版仅提示并打开压缩包下载地址；Linux 与 Android 当前仅发布附件，不接入自动更新。

## 发布前准备

1. 同步 `app/package.json`、`app/src-tauri/tauri.conf.json`、`app/src-tauri/Cargo.toml` 与 `Cargo.lock` 中的应用版本。
2. 将用户可见变化整理到 CHANGELOG 对应的 `## [版本]` 条目，保留 Unreleased 供下次开发使用。日期可写为 `## [版本] - YYYY-MM-DD`。
3. 随包插件的 `engines.app` 兼容当前主版本；插件自身版本和 SDK/API 版本独立维护，不必与应用同步。
4. 完成安装、账号、实际音频设备与依赖许可等发布前验收。CI 的静音解码验证不替代实机安装和听音验证。
5. 提交并推送以上改动，再创建和推送版本标签。例如在版本为 1.0.0 的提交上执行：

```powershell
git tag -a v1.0.0 -m "Nons 1.0.0"
git push origin v1.0.0
```

不删除或移动已发布标签。失败后可在 GitHub Actions 中重新运行该标签的 workflow。

## Release Notes 格式

后续版本沿用 [CHANGELOG 中的 v1.0.0 发布说明](../CHANGELOG.md#100)作为格式与语气范例。

- 版本条目使用 `## [版本]`，可附实际发布日期；正文最前面放本次版本头图，再用一段中文概述本次版本重点、支持平台与分发形式。
- 每次发布都包含头图，沿用 [发布头图素材与编辑器](release-assets/README.md)的蓝白布局：左上角使用项目图标，左侧显示带 `v` 前缀的版本号、发布文案与平台，右侧错位叠放真实运行截图。更新版本号、发布类型与截图，使其对应本次发布；后续版本的发布类型按实际填写。
- 当前分支只保留 `docs/release-assets/header.png` 和可编辑源 `docs/release-assets/header.svg`，每次发布覆盖这两个文件；不要再新增版本化头图文件。对应 CHANGELOG 条目开头使用 Markdown 图片语法，替代文本包含产品名和版本号，地址使用 `https://raw.githubusercontent.com/Miaoyww/Nons/v<版本>/docs/release-assets/header.png`，确保提取后的 Release Notes 仍能显示对应版本的头图。已有版本的 `header-<版本>.png` 链接保持不变。
- 正文按用户使用的功能分组，沿用顺序：音乐库与发现、本地音乐与播放、歌词与界面、后台播放与设置、插件与工具、稳定性修复、安装与使用提示、已知限制与发布验收。只保留本次有内容的章节。
- 各章节使用 `###` 中文标题与简洁项目列表。条目说明用户能做什么、行为如何变化或修复了什么问题；将同一功能的零散记录合并，避免重复。
- 后续版本以相对上一版本的已实现变化为主体；安装提示与已知限制按本次发布的实际情况更新。明确区分主程序、随包插件与独立安装插件，涉及操作时写出设置或入口名称。
- 发布前将 Unreleased 中属于本次发布的内容移入对应版本，保留 Unreleased 供后续开发使用。待验收事项与已通过验证的结果分开描述，性能结论以实测记录为依据。

## CI 执行内容

- 固定 Windows runner、Rust 1.95.0、Node 24、pnpm 10、Python 3.12 与 cargo-component 0.21.1；前端安装和 Rust 构建使用锁文件。
- 校验标签与四处应用版本一致，必须存在非空的对应发布说明；不使用 Unreleased 代替。
- 检查格式，运行前端测试、脚本测试与原生库／集成测试；构建灵动岛与测试 Component。
- QQ 音乐适配器使用 cargo-component 0.21.1 和自身 Cargo.lock 构建，目标为 `wasm32-unknown-unknown`。GD 音源仍需按锁文件安装 `adapters/gdstudio` 与 `packages/wasm-crypto` 的 npm 开发依赖，并将 WAC 0.12.0 安装到 `.local/tools`。
- 通过应用打包入口生成 MSI 和 NSIS EXE，音频使用系统 WebView；Linux 同一次构建生成 AppImage 与 deb，Android 构建 QQMusic、GDStudio 和 Xiaobai 并嵌入 APK。
- 从同一次构建的主程序与 Windows 资源映射生成 `Nons_<版本>_windows_x64_portable.zip`，包含随包插件、使用说明与匹配的 `Nons.pdb` 调试符号，沿用安装包的许可文件打包策略，不包含构建缓存。安装包同样携带匹配符号，请保留以支持错误日志中的函数名与代码行号。完整解压后运行 `Nons.exe`，无需安装应用，但系统必须已有 Microsoft Edge WebView2 Runtime。
- 前端播放测试覆盖 WebAudio 的媒体控制、进度反馈、设备选择及交叉淡入淡出；实际格式与物理输出仍需在各目标平台的系统 WebView 验收。
- Windows 收集当前版本的 MSI、EXE、便携 ZIP 和匹配源码 Manifest 的灵动岛 ZIP，生成 `SHA256SUMS.txt`；Android 收集 APK 与 `SHA256SUMS-android.txt`；Linux 收集 AppImage、deb 与 `SHA256SUMS-linux.txt`。Artifact 保留 14 天，发布任务下载后再次校验哈希。缺少 Windows 便携 ZIP、Android APK 或任一 Linux 包时阻止发布。
- 新 Release 先创建为草稿，上传所有附件后公开；重跑时更新说明并覆盖同名附件，已有公开 Release 保持公开。

## 权限与配置

仓库需要启用 GitHub Actions，允许 workflow 使用 `GITHUB_TOKEN` 写入 Release。构建 job 仅有 `contents: read`，发布 job 才有 `contents: write`。需配置以下 Secrets 和 Variables：

| 类型     | 名称                                 | 用途                                                     |
| -------- | ------------------------------------ | -------------------------------------------------------- |
| Secret   | `TAURI_SIGNING_PRIVATE_KEY`          | Tauri updater 私钥正文，与 tauri.conf.json 的公钥对应    |
| Secret   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 私钥密码；无密码密钥可留空                               |
| Secret   | `ANDROID_KEY_ALIAS`                  | Android upload keystore 的别名                           |
| Secret   | `ANDROID_KEY_PASSWORD`               | Android upload keystore 密码                             |
| Secret   | `ANDROID_KEY_BASE64`                 | Android upload keystore 的 Base64 内容                   |
| Secret   | `NONS_TELEMETRY_INGEST_KEY`          | nons-api 上报密钥，构建时嵌入 Rust；不能使用统计读取密钥 |
| Secret   | `NONS_TELEMETRY_STATS_KEY`           | 读取发布构建统计快照；仅在构建步骤使用，不进入产物       |
| Secret   | `R2_ACCESS_KEY_ID`                   | 允许写入 veto 桶的 R2 S3 凭据 ID                         |
| Secret   | `R2_SECRET_ACCESS_KEY`               | 对应 R2 S3 凭据密钥                                      |
| Variable | `R2_ACCOUNT_ID`                      | R2 所属 Cloudflare 账户 ID                               |

私钥保存在仓库外并备份，不写入 Git；公钥随应用提交。缺少签名密钥会使发布构建失败。CI 应同时配置上报密钥，客户端开发构建与 plugin-probe 不上报；本地发布构建需自行提供同名环境变量。

## 双源自动更新

- R2 固定使用 `veto` 桶中的 `nons/releases/` 前缀，与 `veto/releases/` 隔离；域名 `https://download.miaoyww.top`。版本产物位于 `nons/releases/v<版本>/`，更新清单位于 `nons/releases/latest.json`。
- GitHub 的 `latest.json` 使用 GitHub Release 附件地址，R2 清单使用同一版本的 R2 地址；两份清单的版本、发布时间与签名一致。Windows x64 使用 NSIS EXE 更新，MSI 和 EXE 都保留独立签名附件。Tauri 插件直接安装原始安装包，未启用 ZIP 更新包支持。
- GitHub 附件上传并公开后，再上传 R2 版本产物，全部成功后最后写入 R2 清单；版本文件设置长期缓存，最新清单设置 `no-cache`。上传仅写入 Nons 前缀，不修改 Veto 数据。R2 上传失败会使 CI 失败，GitHub 已发布内容仍可用。
- 发布客户端启动 10 秒后检查一次；关于页可手动检查。单次清单请求超时 8 秒，R2 失败再检查 GitHub；R2 正常返回无新版时结束。下载失败或签名错误时独立查询 GitHub，只有版本一致才下载备用包，并重新使用应用公钥验证签名，避免静默换成其他版本。
- 下载由 Tauri 插件执行并验证签名，最长 15 分钟、上限 512MiB。插件下载期间使用内存保存包，完成后转存临时文件并释放该缓冲；仅保留一个待安装包，安装前或正常退出时清理临时文件，未实现跨启动断点续传。安装再次验证签名，只有用户点击“重启并安装”后才停止播放、保存状态并启动安装器；Windows updater 直接退出，由安装器重新启动应用。
- 便携版不调用安装器，先探测 R2 压缩包地址，失败时打开 GitHub 对应版本；用户退出应用后解压新版并保留 `data/`。macOS/Linux/Android 尚未开放自动安装更新。
- 旧版本未包含 updater，第一次切换到支持更新的版本仍需手动安装。真实跨版本升级、安装器交互及线上双源故障回退须在签名版本发布后验证，单元测试和编译不替代这些验收。

## 匿名遥测

生产地址为 `https://nons-api.miaoyww.top/v1/event`，契约见相邻 `nons-api` 仓库 README。默认开启，可在设置 → 通用 → 匿名使用统计关闭；设置与首次随机生成的 UUID v4 安装标识保存在现有 SQLite settings 表，便携版使用其独立 data 目录。安装标识能关联多次启动，不代表自然人数，不来自账号或硬件。

每个发布进程生成一次启动快照；组件数组暂时固定为空并保留旧接口字段。启动时间、事件 UUID、安装 ID、版本和平台在重试中保持不变；不采集账号、歌曲、文件路径、硬件标识、IP 或 User-Agent 字段。网络基础设施仍会处理请求源地址。正文最多 64KiB，累计播放数只上传该安装的本地累计值。

单次请求超时 8 秒，不跟随重定向；仅对网络失败、429 和 5xx 进行重试。关闭开关会取消当前等待／在途请求并阻止后续上报；本地播放历史仍继续记录，重新开启后从累计快照补报。开发和探针进程不贡献统计，日志不记录请求正文、安装标识和密钥。

关于页的累计使用人次和累计播放歌曲数在发布构建前从 `GET /v1/summary` 固定注入，并显示统计快照日期；运行时不请求接口，未注入快照时显示“暂无统计数据”。发布顺序为：先迁移并部署 API，再配置 `NONS_TELEMETRY_STATS_KEY`，最后运行客户端发布构建。

Windows MSI/NSIS 安装包没有配置 Authenticode 代码签名；CI 的 `TAURI_SIGNING_PRIVATE_KEY` 只用于 Tauri updater，不会签署安装包。Android Release 附件使用固定 upload keystore 签名，但 Google Play 上架、Play App Signing 与 AAB 发布仍待配置。Linux AppImage 与 deb 没有配置发行包签名；AppImage 的可选 GPG 签名也未启用。Linux 与 Android 发布附带的 SHA-256 校验和用于检查文件完整性，不提供独立于发布渠道的签名身份。

## 本地检查发布元数据

```powershell
python scripts/release.py notes v1.0.0 .local/release-notes.md
python -m unittest discover -s scripts/tests
# 完成 pnpm --dir app build:release 后：
python scripts/release.py portable v1.0.0 app/src-tauri/target/release/bundle/portable
python scripts/release.py files v1.0.0 .local/release-files
# 收集 Android 试用包：
python scripts/release.py android-files v1.0.0 .local/android-release-files
python scripts/release.py linux-files v1.0.0 .local/linux-release-files
```

收集目录必须为空，防止把上次版本的文件一起上传。整个 CI 在 GitHub runner 上执行成功后，才能确认托管环境的构建与发布通过。

便携 ZIP 不覆盖同名文件，重新打包前需先移走已有 ZIP。便携包包含 `portable.flag`，启动时以 EXE 所在目录为基准，设置、账号、曲库、插件及其数据统一保存在 `data/`，封面、歌词与音频运行时缓存位于 `data/cache/`，WebView 数据位于 `data/webview/`，临时解码音频位于 `data/temp/`。账号与插件凭据以文件保存到 `data/credentials/`，便于随目录移动，需妥善保管。便携版与安装版独立，旧 AppData 数据和系统凭据不会自动迁移；请完整移动目录并保留标记。程序目录不可写时启动报错，不回退到 AppData；安装版继续使用系统用户目录和凭据库。
