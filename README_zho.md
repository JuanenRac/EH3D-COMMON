<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | 🇨🇳 <b>中文</b> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D Fabrication Suite 的共享数据模型：项目容器、元数据、完整性检查和机床遥测

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**诚实性核对 - 今天真正能运行的内容：** 一个 Rust 库，包含项目容器（`.eh3dproj`）、项目元数据、SHA-256 完整性检查、机床遥测类型和套件的配色，另有套件中每个 ESP32 节点都会复制的固件基础（70 个文件和保持副本一致的工具）。164 个测试在计算机上运行。钻孔和铣削程序以及现代界面用共享 crate 读取文件并绘制作业；这里的内容也从未在机床或电路板上运行过。

---

> **安全（safety）与信息安全现状：** 该库不是安全系统（safety），也从未用它控制过任何机床。容器能发现因意外而损坏或被改动的包，但它没有签名也没有加密：无法证明项目是谁写的。他人提供的包属于不可信输入，因此对条目名称、大小和数量都设有限制。

## 📋 速览

| | |
|---|---|
| 版本 | `0.0.6` |
| 成熟度 | 脚手架 |
| 测试 | 164 |
| 技术栈 | Rust（库 crate）和 C++17（固件基础） |
| 真实硬件 | 不适用：这是数据库，不与机床通信；固件基础从未在电路板上运行 |

**跳转到：** [概述](#-概述) · [架构](#-架构) · [快速开始](#-快速开始) · [测试](#-测试) · [结构](#-仓库结构) · [相关项目](#-相关项目)

## 🚦 项目状态

每一行是本仓库的一个部分，灯号表示它今天做到了哪一步。内容依据仓库能够展示的证据（测试、构建、运行），而不是计划。

🟢 已完成并验证 · 🟡 可用，有已知限制 · 🟠 部分完成或尚未验证 · 🔴 未完成或不可用

| 领域 | 状态 | 在这里的含义 |
|---|---|---|
| 容器 | 🟡 | `.eh3dproj` 是标准 ZIP，带有每个条目的 SHA-256 记录；能读写，并拒绝损坏、被改动、未登记和不安全的条目。目前只有它自己的测试在读写。 |
| 元数据 | 🟡 | 项目元数据（单位、零点、图层、刀具、刀路、基准点）会被校验，并带有语义化的格式版本；因为 1.0.0 是第一个格式，所以暂时没有迁移。 |
| 遥测与主题 | 🟢 | 轴、主轴、机床状态、带严重程度的 GRBL 报警码，以及带对比度检查的套件配色，均已测试。 |
| 测试 | 🟢 | 164 个测试通过（`cargo test --workspace`）：主 crate 的模块内 16 个、容器 14 个，九个共享 crate 中 134 个。 |
| 安全 | 🟡 | 不安全的名称、过大或过多的条目以及任何被改动的条目都会被拒绝；包没有签名也没有加密，也未经独立审查。 |
| 固件基础 | 🟠 | ESP32 节点共享的 49 个文件，由 `tools/sync_firmware_base.py` 保持一致；只通过唯一使用它的节点 EH3D-NODE-BUNGARD 测试过，从未在电路板上运行。 |
| 被其他程序使用 | 🟡 | 工作区的 crate 已被 EH3D-DRILLPRO、EH3D-ROUTEPRO、EH3D-BUNGARD-GUI、EH3D-MOTIONCNC 和 EH3D-SERVER 使用（通过路径，仓库并排放置）；项目容器本身还没有任何程序使用。 |
| 文档 | 🟢 | 八种语言的 README、格式说明和固件基础指南；CI 会将它们与清单和变更日志对照。 |

## 🎯 概述

* **项目容器**：`.eh3dproj` 包是一个标准 ZIP 压缩包，含有 `project.json`、`integrity.json` 以及文件夹 `layers/`、`toolpaths/` 和 `vision/`。写入是确定性的（相同内容得到相同字节），并通过临时文件完成。
* 项目的**元数据**：名称、标识、单位、工件零点、图层、刀具、刀路和基准点，带有语义化的格式版本（目前为 1.0.0）。更新的主版本会被拒绝，而不是只理解一半；更旧的版本需要已注册的迁移，目前还没有。
* **完整性记录**：每个条目的 SHA-256 和大小。条目被改动、缺失或未登记的包无法打开。
* **限制**：可能跳出包文件夹的条目名称会被拒绝，过大或过多的条目也会被拒绝。
* **遥测类型**：轴、主轴、从状态字读取的机床状态，以及带严重程度（警告或严重；未知代码按严重处理）的九个 GRBL 报警码。
* **主题**：套件的配色，测试会对每种文字色和状态色做 WCAG 对比度检查。
* **节点的固件基础**（`firmware_base/` 和 `tools/sync_firmware_base.py`）：每个 EH3D 节点共享的 ESP32 节点固件部分（设置文档、网络、面板的服务器、用户与会话、可回滚的更新、日志、自我保护以及配套测试），以项目名称为占位符只写一份。每个节点保留自己的副本并独立编译；工具会把基础写入每个节点，并在副本出现偏差时报告。
* 工作区的**共享 crate**：`eh3d-view`（工作台上的作业、十一种主题、八种语言的界面文字）、`eh3d-archive`（ZIP 压缩包中的作业）、`eh3d-bungard-formats`（Excellon、HP-GL、刀具表和配置文件，Gerber 和 DXF 轮廓）、`eh3d-controller`（机床控制器接口，GRBL 和一个模拟控制器）、`eh3d-filedialog`（打开和保存窗口）和 `eh3d-fleet`（服务器的机床和工位）。
* **工作台高度与钻头节点：** `eh3d-levelling` 保存机床工作台的高度图（测量点网格、点间曲面插值、倾斜和高低差、作业 Z 的修正，以及保存高度图的文本形式），`eh3d-aux` 是 EH3D-NODE-BUNGARD-AUX 的客户端（其状态、对工作台的测量、MJPEG 摄像头、带令牌的请求和一个小型 HTTP 客户端）。

## 🧩 在 EH3D 中的角色

EH3D-COMMON 是套件的基础。其工作区的 crate（视图和主题、ZIP 压缩包、Bungard 机床的文件、机床控制器、文件窗口和服务器的机群）如今被 EH3D-DRILLPRO、EH3D-ROUTEPRO、EH3D-BUNGARD-GUI、EH3D-MOTIONCNC 和 EH3D-SERVER 使用；打开或保存项目的库和程序计划依赖该容器，目前还没有一个存在。节点的固件基础也在这里：EH3D-NODE-BUNGARD 是第一个使用它的节点。

## 🏛️ 架构

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-COMMON 的架构：谁在使用、包含什么、依赖什么" width="100%">
</p>

本仓库各部分如何协作：谁在使用它、它包含什么、它依赖什么。图片由 `docs/architecture.json` 绘制，并与代码核对。

## ✨ 功能

* **确定性的原子写入：** 相同项目总是得到相同字节，文件通过临时文件写入，所以写入失败不会留下半个包。
* **读取时全部检查：** 先检查名称、数量和大小，再用完整性记录核对每个条目，最后检查元数据；第一个问题就以带类型的错误中止读取。
* **任何工具都能打开的格式：** 普通 ZIP（仅 deflate）和普通 JSON，无需本 crate 即可阅读；`docs/FORMAT.md` 是成文的约定。
* **读取方保持兼容：** 同一主版本会忽略未知内容继续读取；更新的主版本会被拒绝。
* **不会丢失的报警码：** 本 crate 不认识的代码保存为 `Unknown(n)`，并按严重处理。
* **所有节点共用一套固件基础：** 在发现需求的地方修改，`import` 后再用 `sync` 写入其他节点，CI 中的 `check` 会在副本偏离时报告。
* **质量分析器和钻头的 LED 环形灯：** `eh3d-inspect` 根据一组公差配置判断钻头摄像头拍到的图像（孔是否存在、是否圆、尺寸是否正确、位置是否准确；铣削沟槽是否与刀具一样宽、是否均匀、有无变窄；图像是否足够清晰可判断），并给出合格、不合格或无法判定。`eh3d-aux` 还可打开和关闭摄像头周围的 LED 环形灯、用颜色显示检查结果、拍摄图像并把配置交给服务器。

## 🔒 安全与防护

容器会读取他人可能发来的文件，因此下面每项检查都是对恶意或损坏的包的防御，并且每项都有测试。

| 威胁 | 防御 |
|---|---|
| 跳出包文件夹的名称（`../x`、`/etc/x`、`C:\x`） | `check_entry_name` 拒绝空的、绝对的或过长（超过 240 字节）的名称、反斜杠、冒号、控制字符，以及任何空的、`.` 或 `..` 部分；写入时检查，读取 ZIP 中的每个名称时再检查一次 |
| zip 炸弹，或条目数量极多的包 | 最多 10,000 个条目，每个条目 256 MiB，总计 1 GiB；读取一个条目时超出上限 1 字节就会停止 |
| 条目在途中被改动（无意或有意） | 将每个条目的 SHA-256 和大小与 `integrity.json` 比较；不同、缺失或未登记的条目会使整个包被拒绝 |
| 两个同名条目 | 读取时作为重复被拒绝；写入时被替换的条目只保留一份 |
| 来自更新程序的包 | 更新的主格式版本会被拒绝，而不是只理解一半 |
| 元数据不一致（名称为空、刀具 id 重复、NaN、负直径、`layers/` 之外的图层） | `validate` 在写入前和读取后运行；无效的项目无法写入 |
| 无 unsafe 代码 | 该 crate 为 `#![forbid(unsafe_code)]` |

**没有防御的内容：** 包没有签名也没有加密。能重写条目的人也能重写完整性记录，所以它能发现损坏和意外，而不能证明作者。该库未经独立审查。请私下报告漏洞（见 [SECURITY.md](SECURITY.md)）。

## 🔌 接口

| 接口 | 内容 | 位置 |
|---|---|---|
| Rust API | `Package`（`new`、`add_entry`、`entry`、`entry_names`、`integrity`、`write`、`write_to_path`、`to_bytes`、`read`、`read_from_path`）、`ProjectMetadata`（`new`、`validate`、`to_json`、`read_metadata`）、遥测类型和 `theme` | `src/` |
| 文件格式 | `.eh3dproj`：含 `project.json`、`integrity.json` 和数据文件夹 `layers/`、`toolpaths/`、`vision/` 的 ZIP | [docs/FORMAT.md](docs/FORMAT.md) |
| 错误 | 一个 `Error` 枚举：`Io`、`Json`、`Zip`、`UnsupportedFormat`、`MissingEntry`、`Integrity`、`UnsafePath`、`DuplicateEntry`、`TooLarge`、`InvalidMetadata` | `src/error.rs` |
| 固件基础工具 | `python tools/sync_firmware_base.py check`、`sync`、`import <节点>` 和 `status` | [docs/FIRMWARE_BASE.md](docs/FIRMWARE_BASE.md) |

本仓库没有命令行程序、服务器，也没有网络访问。

## ⚙️ 配置

该库没有配置文件：它的限制是代码中的常量，列在这里是为了让读者知道一个包可以包含什么。格式版本是文件的一部分，而不是设置。

| 值 | 限制 | 位置 |
|---|---|---|
| 一个包中的条目数 | 1 到 10,000 | `src/container.rs` 中的 `MAX_ENTRIES` |
| 单个条目的大小 | 最大 256 MiB（268,435,456 字节） | `MAX_ENTRY_BYTES` |
| 所有条目的总大小 | 最大 1 GiB | `MAX_TOTAL_BYTES` |
| 条目名称的长度 | 1 到 240 字节 | `check_entry_name` |
| 数据文件夹 | 仅 `layers/`、`toolpaths/`、`vision/` | `src/project.rs` 中的 `DATA_FOLDERS` |
| 格式版本 | 1.0.0（读取相同主版本，拒绝更新的主版本） | `FORMAT_VERSION` |
| 刀具直径 | 有限且大于 0 | `ProjectMetadata::validate` |

固件基础有自己的设置文档（`node_settings.hpp`、`config_common.hpp`）；其值属于各个节点，在 EH3D-NODE-BUNGARD 的 README 中说明。

## 🚀 快速开始

需要稳定版 Rust 工具链（1.75 或更高）。

```rust
use eh3d_common::{Package, ProjectMetadata};

fn main() -> eh3d_common::Result<()> {
    let mut package = Package::new(ProjectMetadata::new("Test board"));
    package.add_entry("layers/top.gbr", b"G04 test*".to_vec())?;
    package.write_to_path(std::path::Path::new("board.eh3dproj"))?;
    let again = Package::read_from_path(std::path::Path::new("board.eh3dproj"))?;   // 检查每个条目
    println!("{} data entries", again.entry_names().count());
    Ok(())
}
```

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
python -I tools/sync_firmware_base.py check
```

## 🧪 测试

测试共 164 个（`cargo test --workspace`，CI 中在 Windows 和 Linux 上运行）：模块内 16 个（元数据及其校验、报警码和机床状态、配色及其 WCAG 对比度），容器 14 个（内存和文件的往返、相同内容得到相同字节、完整性记录、被改动的条目、被改动的大小、未登记的条目、缺失的条目、被编辑的元数据、不是包的文件、不安全和恶意的条目名称、被替换的条目、无效的项目）。CI 还运行 `cargo fmt --check`、将警告视为错误的 `cargo clippy`、`python -I tools/ci_validate.py`（清单、版本、变更日志和八个 README）以及 `tools/sync_firmware_base.py check`（固件基础的副本）。固件基础的主机测试（`firmware_base/tests/`）在 EH3D-NODE-BUNGARD 中运行，而不在这里。九个共享 crate 另有 134 个：作业压缩包（6 个）、Bungard 机器的文件及 HP-GL 圆弧、圆和缩放以及 Gerber 和 DXF 轮廓（16 个）、在模拟线路上的机器控制器（7 个）、场景、视图、主题以及八种语言的界面文字（20 个）、打开和保存窗口（7 个）、服务器的机床和工位（5 个）以及工作台高度图（8 个）钻头节点的客户端（含 LED 环形灯和图像，10 个）以及质量分析器（43 个）：孔、铣削沟槽、图像质量、配置和结论。

## 🏗️ 构建与发布

版本号位于 `Cargo.toml` 和 `eh3d.project.json`，每个版本在 [CHANGELOG.md](CHANGELOG.md) 中都有条目。版本发布在 GitHub 上；该库以本仓库的 Rust crate 形式使用，不在 crates.io 上。

## 📂 仓库结构

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-COMMON 的仓库结构" width="100%">
</p>

<details>
<summary>目录树的文本版</summary>

```text
EH3D-COMMON/
├── src/
│   ├── container.rs                             包：读写、完整性和限制
│   ├── project.rs                               元数据及其校验
│   ├── telemetry.rs                             轴、主轴、状态和报警码
│   └── theme.rs error.rs lib.rs                 带对比度的配色、错误和 crate 根
├── crates/                                      共享 crate：视图、ZIP、机器文件、控制器、钻头节点、质量分析
├── firmware_base/
│   ├── core/                                    无硬件：设置、认证、网络方案
│   ├── main/                                    ESP-IDF 代码：网络、存储、面板 API
│   ├── tests/                                   以上全部的主机测试
│   ├── tools/                                   打包面板；列出被忽略的 sdkconfig 行
│   └── manifest.json                            哪个文件进入哪个节点
├── tests/                                       包：往返、被改动、缺失和未登记的条目、不安全的名称
├── docs/                                        格式、固件基础和两张图的数据
├── tools/                                       CI 和 README 检查、固件基础同步及绘图工具
├── images/                                      横幅图以及架构和仓库结构图片
├── .github/                                     CI 与发布工作流、Issue 和 PR 模板
├── eh3d.project.json                            清单：版本、角色、成熟度和构建命令
├── Cargo.toml Cargo.lock                        Rust 包及其锁定的依赖
├── CHANGELOG.md                                 每个版本的变更
├── SECURITY.md SUPPORT.md                       如何报告漏洞及在哪里提问
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   参与贡献、行为准则和许可证
```

</details>

## 🗺️ 路线图与已知限制

**目前已知的限制**

* 还没有程序使用项目容器，所以这个格式只被它自己的测试读写过。
* 包没有签名也没有加密，该库也未经独立审查。
* 因为 1.0.0 是第一个格式，所以没有迁移。
* 固件基础只通过 EH3D-NODE-BUNGARD 验证过，从未在电路板上运行；还没有第二个节点。
* 该库没有发布到 crates.io。

**下一步**

* 让套件的第一个程序用它打开和保存项目（先从 EH3D-CORE-LIBS 开始）。
* 把第二个节点加入固件基础，并检查副本保持一致。
* 决定包是否需要可选的签名。

## 🌍 语言

README 和文本提供英语、西班牙语、法语、意大利语、德语、汉语、日语和欧洲葡萄牙语版本。

## 🔗 相关项目

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS)：将使用这些类型的解析器（Excellon、Gerber）和几何库
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD)：第一个保存固件基础副本的节点
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER)：家族的中心；目前还不使用项目容器
* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS)：套件的文档中心及其项目地图
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC)：套件未来要驱动其机器的机器人平台

## 📚 文档

* [项目格式](docs/FORMAT.md)：条目、元数据、完整性记录以及对读取方的规则。
* [固件基础](docs/FIRMWARE_BASE.md)：共享什么、如何保持一致以及如何添加节点。
* [变更日志](CHANGELOG.md)

## 🤝 参与贡献与社区

* 提交 pull request 之前请运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` 和 `python -I tools/sync_firmware_base.py check`，全部通过。参见 [CONTRIBUTING.md](CONTRIBUTING.md) 和[行为准则](CODE_OF_CONDUCT.md)。
* README 有八种语言：修改文字时要同步修改全部八个文件，两张图片由 `tools/make_architecture.py` 和 `tools/make_structure.py` 根据 `docs/architecture.json` 和 `docs/structure.json` 绘制。
* 对固件基础的修改应在某个节点中进行，用 `tools/sync_firmware_base.py import` 导入，再同步到其他节点；不要手工编辑任何一个副本。
* 请私下报告漏洞：[SECURITY.md](SECURITY.md)。问题、想法和报告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 许可证

软件：GPL-3.0-or-later。见 [LICENSE](LICENSE)。套件的硬件设计使用 CERN OHL-S。
