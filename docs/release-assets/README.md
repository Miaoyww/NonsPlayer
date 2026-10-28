# 发布头图

`header.png` 是当前发布头图，尺寸为 1600 × 800；`header.svg` 是同一头图的可编辑源文件。SVG 内嵌项目图标和真实运行截图，不依赖外部图片或字体。当前分支只保留这一套头图，发布新版本时覆盖这两个文件；历史版本由对应 Git tag 中的文件保留。

## 编辑与导出

双击打开 `editor.html`，修改版本号、说明、平台与发布类型。点击“下载 PNG”和“下载 SVG”后，始终保存为 `header.png` 和 `header.svg`，覆盖工作区中的当前文件。导出尺寸固定为 1600 × 800；长文案请先检查预览是否超出左侧区域。

也可以直接用文本编辑器或矢量设计软件打开 `header.svg`。`version`、`subtitle`、`platform`、`release-kind` 分别对应版本号、副标题、平台与发布类型；`title`、`desc` 是图片的无障碍描述，应与内容同步修改。

编辑器的“载入 SVG”只导入上述四项文字。更换截图时，需同步 `header.svg` 与 `editor.html` 中的内嵌图片。

## 发布时更新 CHANGELOG

新版本条目使用发布标签中的固定文件名，例如：

```markdown
![NonsPlayer v1.9.0 发布头图](https://raw.githubusercontent.com/Miaoyww/Nons/v1.9.0/docs/release-assets/header.png)
```

已有版本使用的 `header-<版本>.png` 地址不要改动；这些文件仍由历史 tag 提供。`library-1.3.0.png`、`lyrics-1.3.0.png` 等 README 截图属于独立素材，不随头图清理。
