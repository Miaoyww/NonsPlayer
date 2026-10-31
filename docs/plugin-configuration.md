# 插件配置开发

Manifest 增加 `"configuration": "configuration.json"`。配置字段需要 `config` 权限；文件系统分别申请 `files:data` 和 `files:selected`。这些权限彼此独立。仅提供自定义页面且 `fields: []` 时，无需申请 `config`。

## 统一页面

```json
{
  "version": 1,
  "page": { "mode": "generated" },
  "fields": [
    {
      "key": "duration",
      "title": "预览时长",
      "description": "新出现的歌曲预览显示多久。",
      "schema": { "type": "integer", "minimum": 1, "maximum": 30 },
      "default": 10,
      "apply": "live",
      "group": "预览",
      "editor": { "kind": "slider", "min": 1, "max": 30, "step": 1, "unit": "秒" }
    }
  ]
}
```

字段键直接映射到 `snapshot.values[key]`，点号也是键的一部分。每项必须声明标题、说明、JSON Schema 与有效默认值。可用编辑器：`text`、`textarea`、`number`、`switch`、`slider`、`select`、`multiselect`、`color`、`file`、`directory`。选择器的 `options` 为 `{label,value}` 数组。文件/目录编辑器保存路径字符串，不自动授予文件访问权。复杂对象或数组可声明无 editor 的字段，交由自定义页面编辑。

开关和选择立即保存；文字与数字失焦或 Enter 保存；滑块松开后保存。失败保留草稿供重试。统一页面在停用状态也可编辑。`apply` 默认为 `live`；`reload` 会与加载时值比较并提示重载。

## 自定义页面

定义 `"page": { "mode": "custom", "pageId": "settings" }`，并在 Manifest 的 `contributes.pages` 注册同名页面、路径和前端导出。设置按钮在设置对话框内打开该页面；`usePluginRoute` 和 `usePluginNavigate` 使用对话框自己的导航。插件必须已加载。

```tsx
const config = usePluginConfig()
const snapshot = await config.getSnapshot()
await config.update({ duration: 15 }, snapshot.revision)
const unsubscribe = await config.subscribe((next) => console.log(next.values))
// 组件卸载时调用 unsubscribe；插件停用时宿主也会清理订阅。
```

`update` 校验整批修改并原子保存；旧 revision 会报冲突，重新读取后再决定是否重试。`reset(revision, keys?)` 恢复指定字段，省略 keys 恢复全部并清理遗留覆盖值。快照含 `values`、`revision`、`diagnostics` 和 `pendingReload`。规则变化导致旧值无效时读取默认值并诊断，原始值保留直到修复或重置。

WASM 通过现有 host call 使用 `config.get`（参数 `{}`）、`config.update`（`{patch,revision}`）和 `config.reset`（`{revision,keys?}`）。`event:config-changed` 接收 `{revision,pendingReload}`；收到事件后重新调用 get。前端通过 SDK 订阅，避免在事件中复制整个配置。

## 自行管理文件

`files:data` 提供 `data` 根，对应应用数据目录下 `plugin-data/<pluginId>`；安装代码目录不提供写权限。`files:selected` 允许用户在插件设置中通过原生选择器授权外部目录，可选择只读或读写，并随时撤销。SDK 的 `roots()` 只返回根 ID 和 writable，不暴露绝对路径。

```tsx
const files = usePluginFiles()
const file = await files.open('data', 'state.bin', 'readWrite')
try {
  await file.truncate(0)
  await file.write(0, new Uint8Array([1, 2, 3]))
  const bytes = await file.read(0, 3)
} finally {
  await file.close()
}
```

路径使用 `/` 分隔的相对路径，不接受 `..`、绝对路径或 Windows 驱动器路径。API 包括 stat、分页 list、mkdir、rename、remove 与文件 open/read/write/truncate/close。remove 仅移除文件或空目录。open 的 read 默认只读，readWrite 可创建，create 只允许创建新文件。句柄不能跨插件或加载代次；撤销授权关闭对应句柄。WASM 参数与结果沿用 JSON，二进制块用 base64，SDK 自动转 Uint8Array。

数据关闭插件后保留，不属于资源缓存清理。卸载默认保留配置、键值存储和专属数据，供同 ID 重装时恢复；主动关闭保留选项后才清理。外部授权总会撤销，外部文件始终保留。

## 限制与示例

定义文件最多 64KiB、128 个字段；每个字段的键最多 128 字节，标题 128 字节，说明 2048 字节（UTF-8）。禁用远程 JSON Schema 引用。配置写入快照最多 64KiB；配置读取响应最多 128KiB。文件每块最多 32KiB、每实例最多 16 个句柄、每插件最多 32 个外部目录，目录页最多 128 项且响应有大小上限。文件 IO 在阻塞工作线程执行；这些是设计上限，尚无吞吐基准。

统一页面真实示例为独立仓库 `netease-island` 的 `configuration.json`。自定义页面和二进制存储示例为 `plugins/settings-fixture`，仅供验证，不默认随应用分发：

```powershell
node scripts/build-plugin.mjs settings-fixture app/src-tauri/target/plugin-fixtures/settings-fixture
```

结构参考 [配置定义 schema](../plugins/configuration.schema.json)，接口参考 [SDK 类型](../packages/plugin-sdk/index.d.ts)。

## 授权目录配置

`editor.kind: "authorizedDirectory"` 用于保存读写授权的根 ID（string 默认值可为 `""`）。统一页面列出已有读写授权并提供“选择并授权”；一次选择完成目录授权和配置修订写入，配置写入失败则撤销本次新增授权。需要 `config` 与 `files:selected`。普通 `directory` 编辑器仍只保存路径，不授予权限。

撤销授权保留字段原值并显示失效提示，插件必须通过 `files.roots` 再验证。新任务不得使用失效或只读根；目录选择取消不修改原配置。
