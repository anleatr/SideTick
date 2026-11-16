# SideTick 技术设计

当前目标：Windows x64 静态桌宠与累计工作计时。Rust 2024，单一 bongocat-app crate；不包含 GPUI、Live2D、音频、插件、网络检测或自动更新。

## 模块与生命周期

| 模块 | 职责 |
| --- | --- |
| model.rs | 严格 v1 状态、自然日计数、可注入毫秒值的计时器、小时格式化和滚轮余量 |
| storage.rs | 环境隔离、JSON 读取验证、同目录原子写入 |
| windows/mod.rs | Win32 消息循环、窗口、托盘、全局快捷键、交互及生命周期 |
| windows/paint.rs | PNG 缩放、预乘 Alpha 头像和半透明面板绘制 |

单一 UI 线程拥有 App、头像窗口和面板。GetMessageW 等待事件，不建立动画、输入监听或 GPU 工作线程。头像仅在初始化、尺寸、透明度或 DPI 等变化时重绘。1 秒系统定时器更新日期，检查退出期限与持久化；可见面板在显示值或计时状态变化时重绘，运行中本次时长按秒显示。隐藏面板不持续绘制。

RegisterHotKey 注册 Ctrl+Alt+L 并禁止按住重复触发。只处理自身窗口鼠标消息和指定热键，不注册 Raw Input 或全局键鼠 hook。左键使用原生窗口移动，右键移动达到阈值后缩放，否则切换面板。滚轮累积不足一格的余量，每格改变 5%，范围 25%～400%。

正常退出先停止计时并保存，随后释放热键、托盘、窗口和单实例 mutex。窗口状态在销毁前保持有效，FFI 集中在 Windows 模块。

## 时间与数据

Days 由首次使用与当前本地日期计算，从 0 起算，保存已经达到的最大天数，日期回退不会降低显示值。它不表示联网时间。

工作计时使用 Instant 提供的单调毫秒数；多次会话累加，Hours 显示一位小数，存储保持毫秒精度。暂停和正常退出后的时间不计入，重启始终暂停。当前没有睡眠/锁屏自动暂停逻辑，不承诺只统计实际活跃工作。

通过 Windows Known Folder API 获取 RoamingAppData，不依赖 APPDATA 环境变量。固定标识 com.ayangweb.bongo-cat 下按编译 feature 分 development 和 production，各有 personal-state.json 及独立命名 mutex。

状态包含 schema_version: 1、首次日期、Days、累计毫秒、缩放、透明度、置顶与位置。不自动导入旧配置。缺失文件创建默认值；非法 JSON、版本或范围导致启动错误并保留文件。atomic-write-file 在 flush 后原子提交。通常每五秒保存，交互及正常退出也保存；意外退出可能丢失最近未保存时间。

## 验证边界

justfile 是命令入口：dev-fast/dev 为 Debug Development；build 为 Release Production，只生成 exe；check 运行 fmt、全 target/feature Clippy、workspace 测试和 Release check，不生成 Release exe；dev-smoke 使用正常消息循环，五秒后退出。

自动化覆盖会话累加与重启暂停、日期边界、小时精度、滚轮范围与余量、原子存储与非法文件保留、透明角与半透明主体。Smoke 与人工交互不能互相替代。尚未完成完整 Windows 版本/DPI 矩阵、八小时 soak、安装包和签名验收。
