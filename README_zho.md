<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D Fabrication Suite 的共享数据模型：项目容器、元数据、完整性校验与机床遥测

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实性核对 - 今天真正能运行的内容：** 一个 Rust 库，包含项目容器（`.eh3dproj`）、项目元数据、SHA-256 完整性校验和机床遥测类型。这一切都在开发计算机上的测试中运行；套件中还没有任何程序使用它，也没有在任何机床上运行过。

---

> **安全（safety）与信息安全现状：** 本库不是安全系统，也从未用它控制过任何机床。容器可以发现因意外而损坏或被改动的包，但它既没有签名也没有加密：它不能证明项目是谁写的。来自他人的包属于不可信输入，因此条目名称、大小和数量都受到限制。

## 📋 概览

| | |
|---|---|
| 版本 | `0.0.1` |
| 成熟度 | 脚手架 |
| 测试 | 29 |
| 技术栈 | Rust |
| 真实硬件 | 不适用：这是一个数据库，这里没有任何内容与机床通信 |

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 软件 | 🟡 | 容器、元数据和遥测类型可作为 Rust 库使用；套件中还没有任何程序在用它们。 |
| 测试 | 🟢 | 29 项测试通过（`cargo test`）：往返读写、被改动、缺失和未列出的条目，以及不安全的名称。 |
| 数据格式 | 🟡 | `.eh3dproj` 是标准 ZIP，附带每个条目的 SHA-256 记录，格式写在 `docs/FORMAT.md` 中；更新的主版本会被拒绝。目前只有它自己的测试在读写它。 |
| 安全性 | 🟡 | 不安全的条目名、过大或过多的条目会被拒绝，被改动的包无法打开；包本身没有签名也没有加密，也没有经过独立审查。 |
| 共享固件基础 | 🟠 | 所有节点共用的节点固件部分只通过唯一使用它的 EH3D-NODE-BUNGARD 进行测试；还没有第二个节点。 |
| 真实硬件 | 🟠 | 库本身不需要硬件，但其中的固件基础从未在电路板上运行过。 |
| 文档 | 🟢 | 八种语言的 README、格式说明和固件基础指南；检查会将它们与清单和变更记录核对。 |

## 🎯 简介

* **项目容器**：`.eh3dproj` 包是标准 ZIP 压缩包，含 `project.json`、`integrity.json` 以及 `layers/`、`toolpaths/`、`vision/` 文件夹。
* 项目的**元数据**：名称、标识、单位、工件零点、图层、刀具、刀路和基准点，并带有语义化的格式版本。更新的主版本会被拒绝，而不是只理解一半。
* **完整性记录**：每个条目的 SHA-256 和大小。条目被改动、缺失或未列出的包不会被打开。
* **限制**：可能跳出包文件夹的条目名称会被拒绝，过大或过多的条目也会被拒绝。
* **遥测类型**：轴、主轴、机床状态和带严重程度的 GRBL 报警代码。
* **主题**：套件的颜色，并带有对比度检查。
* **节点固件基础**（`firmware_base/`，配套 `tools/sync_firmware_base.py`）：所有 EH3D 节点共享的 ESP32 节点固件部分（设置文档、网络、面板服务器、用户与会话、可回滚的更新、日志、自我保护以及相应的测试），以项目名称作占位符只写一次。每个节点保留自己的副本并独立编译；该工具把基础写入每个节点，并在某个副本偏离时报告。

## 🧩 在 EH3D 中的角色

EH3D-COMMON 是套件的基础：打开或保存项目的库和程序计划依赖它。目前它们都还不存在。节点固件基础也放在这里：EH3D-NODE-BUNGARD 是第一个使用它的节点。

## 📂 仓库结构

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── firmware_base/节点固件的共享部分
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ 构建与运行

需要稳定版的 Rust 工具链。

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 相关项目

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): 计划中：将使用这些类型的解析器和几何引擎
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): 计划中：将打开和保存项目的中心
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): 套件未来要驱动其机床的机器人平台

## 📚 文档与社区

* [项目格式](docs/FORMAT.md)：条目、元数据、完整性记录以及读取方的规则。
* [固件基础](docs/FIRMWARE_BASE.md)：共享什么、如何保持一致，以及如何添加节点。
* [更新日志](CHANGELOG.md)
* [参与贡献](CONTRIBUTING.md)
* [安全政策](SECURITY.md)
* [支持](SUPPORT.md)

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
