# 贡献指南

[English](CONTRIBUTING.md)

阅读 [AGENTS.md](AGENTS.md) 和 [技术设计](docs/technical-design.md)。当前仓库仅包含 `crates/bongocat-app` 中的 Rust 2024 Windows x64 应用。

## 开发

按 [README](README.zh-CN.md) 安装前置工具。使用 `just dev-fast` 增量开发、`just test` 运行测试、`just check` 检查格式、严格 Clippy、workspace 测试与 Release 编译，使用 `just dev-smoke` 进行五秒启动及退出检查。

Smoke 使用 Development 数据，不模拟交互。先关闭已有实例：单实例限制导致的提前退出不能证明启动成功。手动验证面板开关、Ctrl+Alt+L 开始/停止、重启后累计时间、拖动、滚轮缩放和托盘设置。明确报告未验证行为，不把删除状态或本地备份作为常规清理。

## 提交与文档

保留无关修改，集成前 fetch，在聚焦的功能分支开发，采用 Conventional Commits。未经明确授权，不强制推送、不发布 tag 或 Release。首次同步前核对远端历史；空仓库没有可用于创建 PR 的目标分支。

README、本指南和 CHANGELOG 的中英文版本保持对应。每项功能交付每种语言至多一条 Unreleased 摘要，范围变化时原位改写。保留已发布历史：现有 2.x 记录描述上游 BongoCat，不代表 SideTick 发版。决策写入 `docs/adr/`，当前架构写入 `docs/technical-design.md`。

Win32 代码放在 `src/windows/`，计时逻辑放在 `src/model.rs`，持久化放在 `src/storage.rs`。不重新引入全局输入跟踪或持续动画循环。直接依赖固定版本，保留 Cargo.lock。构建产物、备份、凭证和个人状态不进入 Git。
