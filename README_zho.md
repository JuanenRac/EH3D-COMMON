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

## 🎯 简介

* **项目容器**：`.eh3dproj` 包是标准 ZIP 压缩包，含 `project.json`、`integrity.json` 以及 `layers/`、`toolpaths/`、`vision/` 文件夹。
* 项目的**元数据**：名称、标识、单位、工件零点、图层、刀具、刀路和基准点，并带有语义化的格式版本。更新的主版本会被拒绝，而不是只理解一半。
* **完整性记录**：每个条目的 SHA-256 和大小。条目被改动、缺失或未列出的包不会被打开。
* **限制**：可能跳出包文件夹的条目名称会被拒绝，过大或过多的条目也会被拒绝。
* **遥测类型**：轴、主轴、机床状态和带严重程度的 GRBL 报警代码。
* **主题**：套件的颜色，并带有对比度检查。

## 🧩 在 EH3D 中的角色

EH3D-COMMON 是套件的基础：打开或保存项目的库和程序计划依赖它。目前它们都还不存在。

## 📂 仓库结构

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
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
* [更新日志](CHANGELOG.md)
* [参与贡献](CONTRIBUTING.md)
* [安全政策](SECURITY.md)
* [支持](SUPPORT.md)

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
