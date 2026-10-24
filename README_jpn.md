<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D Fabrication Suite の共有データモデル：プロジェクトコンテナ、メタデータ、整合性チェック、マシンのテレメトリ

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 今日実際に動くもの：** プロジェクトコンテナ（`.eh3dproj`）、プロジェクトのメタデータ、SHA-256 整合性チェック、マシンのテレメトリ型を備えた Rust ライブラリです。すべて開発用コンピューター上のテストで動作します。スイートのプログラムはまだこれを使っておらず、実際のマシンでは何も実行していません。

---

> **安全性とセキュリティの現状：** このライブラリは安全システムではなく、これでマシンを制御したことはありません。コンテナは偶然に壊れたり変更されたりしたパッケージを検出しますが、署名も暗号化もされていません。誰がプロジェクトを書いたかは証明できません。他人から受け取ったパッケージは信頼できない入力として扱うため、エントリー名・サイズ・数を制限しています。

## 📋 概要

| | |
|---|---|
| バージョン | `0.0.1` |
| 成熟度 | 足場 |
| テスト | 29 |
| 技術スタック | Rust |
| 実機での動作 | 該当なし：データライブラリであり、マシンと通信するものはありません |

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| ソフトウェア | 🟡 | コンテナ、メタデータ、テレメトリの型は Rust ライブラリとして動作します。スイートのどのプログラムもまだ使っていません。 |
| テスト | 🟢 | 29 件のテストが通ります（`cargo test`）：往復、変更・欠落・未記載のエントリ、安全でない名前。 |
| データ形式 | 🟡 | `.eh3dproj` は各エントリの SHA-256 記録を持つ標準 ZIP で、`docs/FORMAT.md` に記述されています。新しいメジャーバージョンは拒否されます。今のところ読み書きするのは自身のテストだけです。 |
| セキュリティ | 🟡 | 安全でないエントリ名や、大きすぎる・多すぎるエントリは拒否され、変更されたパッケージは開けません。パッケージは署名も暗号化もされておらず、独立したレビューも受けていません。 |
| 共有ファームウェア基盤 | 🟠 | すべてのノードが共有するノードファームウェアの部分は、それを使う唯一のノードである EH3D-NODE-BUNGARD を通してのみテストされています。2 つ目のノードはまだありません。 |
| 実機 | 🟠 | ライブラリには不要ですが、含まれているファームウェア基盤は一度も基板上で動いたことがありません。 |
| ドキュメント | 🟢 | 8 言語の README、形式の説明、ファームウェア基盤のガイド。検査でマニフェストと変更履歴との整合を確認しています。 |

## 🎯 概要説明

* **プロジェクトコンテナ**：`.eh3dproj` パッケージは標準の ZIP アーカイブで、`project.json`、`integrity.json`、フォルダー `layers/`、`toolpaths/`、`vision/` を含みます。
* プロジェクトの**メタデータ**：名前、識別子、単位、ワークのゼロ点、レイヤー、工具、ツールパス、フィデューシャルと、セマンティックなフォーマットバージョン。新しいメジャーバージョンは中途半端に解釈せず拒否します。
* **整合性レコード**：各エントリーの SHA-256 とサイズ。エントリーが変更・欠落・未記載のパッケージは開きません。
* **制限**：パッケージのフォルダーの外に出られる名前のエントリーは拒否され、大きすぎるエントリーや多すぎるエントリーも拒否されます。
* **テレメトリ型**：軸、スピンドル、マシン状態、重大度付きの GRBL アラームコード。
* **テーマ**：スイートの色とコントラストチェック。
* **ノードのファームウェアベース**（`firmware_base/`、`tools/sync_firmware_base.py` 付き）：すべての EH3D ノードが共有する ESP32 ノードファームウェアの部分（設定ドキュメント、ネットワーク、パネルのサーバー、ユーザーとセッション、ロールバック付き更新、ログ、自己保護、およびそれに付随するテスト）を、プロジェクト名をプレースホルダーにして 1 か所に書いたものです。各ノードは自分のコピーを持って単独でビルドし、ツールがベースを各ノードに書き込み、コピーがずれたときに知らせます。

## 🧩 EH3D での役割

EH3D-COMMON はスイートの土台です。プロジェクトを開いたり保存したりするライブラリやプログラムが、これに依存する予定です。それらはまだ存在しません。ノードのファームウェアベースもここにあります。EH3D-NODE-BUNGARD がそれを使う最初のノードです。

## 📂 リポジトリ構成

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── firmware_base/ノードのファームウェアの共有部分
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ ビルドと実行

安定版の Rust ツールチェーンが必要です。

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 関連プロジェクト

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): 予定：これらの型を使うパーサーとジオメトリエンジン
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): 予定：プロジェクトを開いて保存するハブ
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): スイートが将来制御する予定のマシンを持つロボティクスプラットフォーム

## 📚 ドキュメントとコミュニティ

* [プロジェクト形式](docs/FORMAT.md)：エントリー、メタデータ、整合性レコード、読み取り側のルール。
* [ファームウェアベース](docs/FIRMWARE_BASE.md)：何を共有するか、どう同一に保つか、ノードの追加方法。
* [変更履歴](CHANGELOG.md)
* [貢献方法](CONTRIBUTING.md)
* [セキュリティポリシー](SECURITY.md)
* [サポート](SUPPORT.md)

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
