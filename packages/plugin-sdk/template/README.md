# 插件项目

使用 Nons 插件 SDK 的前端插件模板。

```bash
pnpm install
pnpm build
```

`pnpm build` 执行 TypeScript 检查与打包，输出 `dist/` 与 `<id>-<版本>.zip`。在 Nons 设置 → 插件中安装 ZIP 并确认权限。

页面入口为 `frontend/index.tsx`，Manifest 与扩展声明见 `manifest.json`。WIT 契约与 Manifest schema 位于 `node_modules/@nonsplayer/plugin-sdk/`。
