# @nonsplayer/plugin-sdk

Nons 插件开发包：TypeScript 类型、WIT 契约、Manifest schema 与 `nons-plugin` 构建 CLI。

包版本独立维护；`package.json` 的 `nons.pluginApi` 与 `nons.uiApi` 声明对应的插件 API 与 UI API 版本，`nons-plugin` 用它校验 Manifest 的 `engines` 范围。

## 安装

```bash
pnpm add -D @nonsplayer/plugin-sdk react @types/react
```

React 由宿主提供，插件只依赖类型；构建时 React、JSX runtime 与 SDK 会被外部化到宿主的 `_host/` 模块。

## 命令

```bash
nons-plugin create my-plugin --name "我的插件"
nons-plugin build
nons-plugin validate dist/manifest.json --root dist
```

- `create`：从模板创建前端插件项目。
- `build`：TypeScript 检查、打包 `frontend/index.tsx`、复制 Manifest／配置／资产、校验产物并生成 ZIP。声明 `backend` 时先运行 `cargo component build`。
- `validate`：校验 Manifest 结构、权限、扩展引用与版本范围；`--root` 同时检查打包文件的路径、大小和链接。

## 文件

- `index.d.ts`、`music.d.ts`、`native.d.ts`：公开类型。
- `wit/plugin.wit`：WASM 后端的 `nons:plugin@1.0.0` 契约。
- `schemas/manifest.schema.json`：Manifest 编辑器 schema；宿主的 Rust 校验器仍是运行时权威。

## 开发本包

在 Nons 仓库中运行 `pnpm --dir packages/plugin-sdk install` 安装依赖。`wit/` 与 `schemas/` 由 `scripts/sync-assets.mjs` 在 `pretest`、`prepack` 时从宿主仓库复制。
