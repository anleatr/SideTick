# SideTick

[English](README.md)

轻量 Windows 桌面伴侣：静态猫咪头像与累计工作计时。基于 BongoCat，当前可执行文件及部分系统文案仍沿用 BongoCat 名称。

## 使用

| 操作 | 效果 |
| --- | --- |
| 右键点击头像 | 开关半透明时光面板 |
| 点击面板按钮或按 Ctrl+Alt+L | 开始或停止本次工作计时 |
| 左键拖动 | 移动头像 |
| 鼠标悬停后滚动滚轮 | 调整大小，范围 25%～400% |
| 右键拖动 | 缩放，不打开面板 |
| 右键点击托盘图标 | 计时、置顶、大小、透明度和退出设置 |

**Days** 从首次使用当天的 0 开始，按经过的本地自然日期递增，不表示实际联网时长。**Hours** 显示累计工作小时，四舍五入到小数点后一位。暂停期间不累加，重启后保留累计时间并保持暂停。正常退出保存本次计时；意外结束可能丢失最近一次周期保存后的时间（通常每五秒保存一次）。

再次右键、面板获得焦点时按 Esc，或点击其他应用即可关闭面板。关闭面板不会停止正在进行的计时。头像不播放动画，也不响应一般键盘和鼠标活动。Ctrl+Alt+L 是固定全局快捷键，启动时需要该组合未被其他应用占用。当前界面为中文。

## 构建与运行

需要 Windows x64、`rust-toolchain.toml` 指定的 Rust MSVC 工具链、包含 Windows SDK 的 Microsoft C++ Build Tools，以及已加入 PATH 的 `just`。目标最低系统为 Windows 10 1903+，不支持其他操作系统。

```powershell
just check
just dev-smoke
just dev-fast
```

`dev-fast` 编译并启动 Debug 开发版，后续编译复用增量缓存。未修改代码时可以直接运行 `target\debug\bongocat-app.exe`。重新编译前，先从托盘退出应用。

`just build` 生成 Production 程序 `target\release\bongocat-app.exe`，不会生成安装包或发布版本。`just check` 中的 Release 检查不会替换这个可执行文件。

## 数据

状态保存在 Windows 漫游应用数据目录下：

- Development：`com.ayangweb.bongo-cat\development\personal-state.json`
- Production：`com.ayangweb.bongo-cat\production\personal-state.json`

两种环境的累计时间、设置和单实例锁相互独立。不会自动导入旧版 BongoCat 配置或工作时长。请保留状态文件以保存累计记录，手动修改前先备份。无效状态会导致启动报错，不会自动覆盖原文件。

## 项目

[SideTick 仓库](https://github.com/anleatr/SideTick) · [贡献指南](CONTRIBUTING.zh-CN.md) · [更新日志](CHANGELOG.zh-CN.md)

基于 [BongoCat](https://github.com/ayangweb/BongoCat)，静态头像取自其已有模型封面资源。原有许可与署名要求继续适用，见 [LICENSE](LICENSE)。本项目不包含 Cubism Core 或 Live2D 模型。
