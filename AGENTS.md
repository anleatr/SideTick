# SideTick AI Working Agreement

适用于整个仓库。开始前完整阅读本文件及更深层 AGENTS.md，保留用户已有修改。

## 当前项目

用户已明确将原 BongoCat 工程收敛为个人 Windows 静态桌宠，并指定 anleatr/SideTick 为同步仓库。原工程中已移除模块的架构和发布流程不再代表本项目能力。

- Rust 2024，唯一应用 crate 为 crates/bongocat-app，仅支持 x86_64-pc-windows-msvc。
- 静态 PNG 头像、Win32 layered windows、GDI 与消息驱动重绘。
- 不重新引入 GPUI、Live2D、Raw Input、一般键鼠跟踪、音频、插件、macOS 后端或持续动画循环。
- 保留原标识 com.ayangweb.bongo-cat；Development/Production 在编译期确定，数据和单实例锁隔离。
- 当前架构见 docs/technical-design.md，取舍见 docs/adr/0001-static-companion.md，构建入口以 justfile 为准。
- Days 是经过的本地自然日期；工作计时由单调时钟驱动，Hours 一位小数；Ctrl+Alt+L 是固定开关。

## 实施规则

按现有行为、已有 helper/类型、标准库、系统原生能力、已有依赖、经调研的成熟依赖、最小自有实现的顺序选择。只修改当前任务，不做无关重构，不覆盖用户修改。

计时模型与系统窗口类型隔离；Win32/unsafe 只在 Windows 模块，说明生命周期和线程安全不变量。优先 RAII，不用 unsafe Send/Sync 绕过线程模型，不让 panic 穿过 FFI。

直接依赖精确 pin；新增或升级前核对维护、许可证、版本和目标支持，更新 lock 并解释用途和替换边界。移除功能时删除未使用依赖，不用 allow 掩盖检查错误。Cargo.toml 不写解释性注释，设计理由放 ADR。

状态仅接受严格 schema_version: 1，原子写入，失败保留文件；不自动导入旧开发格式。已正式发布数据的演进必须保持可读或提供经验证的迁移。未发布功能不维护中间格式兼容分支。状态清理必须明确说明原因，不擅自删除用户计时或备份。

## 验证与文档

just 是构建入口：
- just dev-fast / just dev：Debug Development 编译并运行。
- just dev-smoke：Development 正常消息循环，五秒后退出。
- just test：workspace 测试。
- just check：fmt、严格全 target/feature Clippy、workspace 测试、Release check。
- just build：Release Production exe；不代表安装包、签名或发布。

Smoke 前确认无已有实例，不能把单实例提前退出当作验证通过。交付需要按风险验证面板、快捷键、拖动、缩放、保存、重启和正常退出。不得把编译通过说成实机验证通过；未跑的平台矩阵、长时间运行或发布验证必须如实说明。

README、CONTRIBUTING、CHANGELOG 同步维护中英文。Technical Design 描述当前架构，ADR 记录决策与验收缺口。一个 PR 或同一待交付功能，每种语言最多一条 Unreleased 摘要；开发中的修复不另增条目，核心交付变化时改写原条目。保留上游已发布历史，不创建未经要求的版本标题。

最终报告列出改动、验证结果、未运行检查、风险、CHANGELOG 判断、需要用户处理的本地状态和剩余任务。文档任务不虚构代码验证。

## Git 与同步

未经明确授权不 commit、push、开 PR、release 或打 tag。同步请求授权完成相应提交与推送，不授权强制覆盖远端或发布版本。

默认主分支 master，以 origin/HEAD 为准；主分支不直接提交功能。需要提交时使用语义功能分支，保留当前用户分支和修改，禁止擅自 reset 或覆盖冲突。

顺序：fetch → 核对/合入上游 → 解决冲突 → 最终树验证 → 审查 staged diff → Conventional Commit → push。无权限或网络失败时报告阻塞，不绕过访问限制，不声称推送完成。冲突按语义解决，不用 ours/theirs 覆盖实质修改。不得提交 target、source-backups、本机状态、凭证或密钥。

有适用目标分支时按仓库协作方式创建 PR，完成后切回主分支。空仓库首次同步前必须确认远端历史；没有目标分支时不能伪造 PR。SideTick 是个人项目，未经要求不向原 BongoCat 上游发送功能 PR。没有明确发布请求，不打 tag 或创建 Release。
