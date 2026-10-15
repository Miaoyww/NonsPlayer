# GitHub Release CI

发布流程由 [.github/workflows/publish.yml](../.github/workflows/publish.yml) 定义：推送 `v<主版本>.<次版本>.<补丁版本>` 标签后构建 Windows x64，上传 Actions Artifact，再发布到 GitHub Release。从 1.3.2 起暂缓 Android 构建和 APK 分发，Windows 发布不依赖 Android。当前不配置其他存储或自动更新服务。

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
- 将头图 PNG 保存为 `docs/release-assets/header-<版本>.png`，保留历史版本图片。在对应 CHANGELOG 条目开头使用 Markdown 图片语法嵌入，图片替代文本包含产品名和版本号；使用 GitHub Release 页面可直接访问的图片绝对 URL，优先引用本次发布标签下的仓库原始文件，确保提取后的 Release Notes 仍能显示图片。
- 正文按用户使用的功能分组，沿用顺序：音乐库与发现、本地音乐与播放、歌词与界面、后台播放与设置、插件与工具、稳定性修复、安装与使用提示、已知限制与发布验收。只保留本次有内容的章节。
- 各章节使用 `###` 中文标题与简洁项目列表。条目说明用户能做什么、行为如何变化或修复了什么问题；将同一功能的零散记录合并，避免重复。
- 后续版本以相对上一版本的已实现变化为主体；安装提示与已知限制按本次发布的实际情况更新。明确区分主程序、随包插件与独立安装插件，涉及操作时写出设置或入口名称。
- 发布前将 Unreleased 中属于本次发布的内容移入对应版本，保留 Unreleased 供后续开发使用。待验收事项与已通过验证的结果分开描述，性能结论以实测记录为依据。

## CI 执行内容

- 固定 Windows runner、Rust 1.95.0、Node 24、pnpm 10、Python 3.12 与 cargo-component 0.21.1；前端安装和 Rust 构建使用锁文件。
- 校验标签与四处应用版本一致，必须存在非空的对应发布说明；不使用 Unreleased 代替。
- 检查格式，运行前端测试、脚本测试与原生库／集成测试；构建灵动岛与测试 Component。
- QQ 音乐适配器构建前，分别按锁文件安装 `adapters/qqmusic` 与 `packages/wasm-crypto` 的 npm 开发依赖，并将 WAC 0.12.0 安装到 `.local/tools`；Rust 密码组件使用 `wasm32-unknown-unknown` 目标。
- 从现有引导脚本准备 GStreamer，并通过应用已有打包入口生成包含私有音频运行时的 MSI 和 NSIS EXE。
- 从同一次构建的主程序与 Windows 资源映射生成 `Nons_<版本>_windows_x64_portable.zip`，包含私有音频运行时、随包插件与使用说明，沿用安装包的许可文件打包策略；不包含构建缓存和调试符号。完整解压后运行 `Nons.exe`，无需安装应用或 GStreamer，但系统必须已有 Microsoft Edge WebView2 Runtime。
- 清除 SDK 路径后验证运行时文件、插件工厂与 WAV/FLAC/MP3/AAC/Vorbis/Opus 解码。CI 使用 `--skip-audio-output`，不要求 runner 存在音频设备；手动默认模式仍验证静音设备输出。
- 便携 ZIP 解压到独立目录后再次执行上述静音解码验证，确认归档后的音频运行时完整。
- 仅收集当前版本的 MSI、EXE、便携 ZIP 和匹配源码 Manifest 的灵动岛 ZIP，生成 `SHA256SUMS.txt`；Artifact 保留 14 天，发布任务下载后再次校验哈希。缺少便携 ZIP 时阻止发布。
- 新 Release 先创建为草稿，上传所有附件后公开；重跑时更新说明并覆盖同名附件，已有公开 Release 保持公开。

## 权限与配置

仓库需要启用 GitHub Actions，允许 workflow 使用 `GITHUB_TOKEN` 写入 Release。构建 job 仅有 `contents: read`，发布 job 才有 `contents: write`。不需要手动配置第三方密钥。

Windows 安装包没有配置代码签名。Android Release CI 已暂停；本地仍可构建调试签名试用 APK，正式签名与跨版本覆盖升级仍待配置。macOS/Linux 的运行时分发尚未适配，因此未加入发布构建。

## 本地检查发布元数据

```powershell
python scripts/release.py notes v1.0.0 .local/release-notes.md
python -m unittest discover -s scripts/tests
# 完成 pnpm --dir app build:release 后：
python scripts/release.py portable v1.0.0 app/src-tauri/target/release/bundle/portable
python scripts/release.py files v1.0.0 .local/release-files
# 仅本地手动收集 Android 试用包（不随当前 Release 发布）：
python scripts/release.py android-files v1.0.0 .local/android-release-files
```

收集目录必须为空，防止把上次版本的文件一起上传。整个 CI 在 GitHub runner 上执行成功后，才能确认托管环境的构建与发布通过。

便携 ZIP 不覆盖同名文件，重新打包前需先移走已有 ZIP。便携包包含 `portable.flag`，启动时以 EXE 所在目录为基准，设置、账号、曲库、插件及其数据统一保存在 `data/`，封面、歌词与音频运行时缓存位于 `data/cache/`，WebView 数据位于 `data/webview/`，临时解码音频位于 `data/temp/`。账号与插件凭据以文件保存到 `data/credentials/`，便于随目录移动，需妥善保管。便携版与安装版独立，旧 AppData 数据和系统凭据不会自动迁移；请完整移动目录并保留标记。程序目录不可写时启动报错，不回退到 AppData；安装版继续使用系统用户目录和凭据库。
