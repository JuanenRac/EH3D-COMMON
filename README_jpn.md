<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D Fabrication Suite の共通データモデル：プロジェクト コンテナ、メタデータ、整合性チェック、機械テレメトリ

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.5-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 今日実際に動くもの：** プロジェクト コンテナ（`.eh3dproj`）、プロジェクトのメタデータ、SHA-256 による整合性チェック、機械テレメトリの型、スイートの配色を備えた Rust ライブラリと、スイートの ESP32 ノードがすべてコピーして使うファームウェア基盤（70 ファイルと、コピーを同一に保つツール）。115 件のテストがコンピューター上で動きます。穴あけ・ルーティングのプログラムとモダンなインターフェースは、共有クレートでファイルを読み込み作業を描画しますが、機械でも基板でも実行されたことはありません。

---

> **安全性とセキュリティの現状：** このライブラリは安全装置ではなく、これで機械を制御したことはありません。コンテナは、誤って壊れた、または変更されたパッケージを検出しますが、署名も暗号化もされていないため、誰がプロジェクトを書いたかは証明できません。他人から受け取ったパッケージは信頼できない入力なので、エントリ名・サイズ・数に上限を設けています。

## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.5` |
| 成熟度 | 足場 |
| テスト | 115 |
| 技術スタック | Rust（ライブラリ クレート）と C++17（ファームウェア基盤） |
| 実機での動作 | 該当なし：データ ライブラリで、機械とは通信しません。ファームウェア基盤は基板で動いたことがありません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [構成](#-リポジトリの構成) · [関連](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| コンテナ | 🟡 | `.eh3dproj` は各エントリの SHA-256 記録を持つ標準の ZIP で、読み書きができ、壊れた・変更された・未記載・危険なエントリを拒否します。今のところ読み書きするのは自身のテストだけです。 |
| メタデータ | 🟡 | プロジェクトのメタデータ（単位、ゼロ点、レイヤー、工具、ツールパス、基準マーク）は検証され、セマンティックなフォーマット バージョンを持ちます。1.0.0 が最初のフォーマットなので、移行はまだありません。 |
| テレメトリとテーマ | 🟢 | 軸、スピンドル、機械の状態、重大度付きの GRBL アラーム コード、コントラスト チェック付きのスイートの配色。すべてテスト済みです。 |
| テスト | 🟢 | 115 件のテストが通ります（`cargo test --workspace`）：メインクレートのモジュール内 16 件、コンテナ 14 件、八つの共有クレートで 85 件。 |
| セキュリティ | 🟡 | 危険な名前、大きすぎる／多すぎるエントリ、変更されたエントリは拒否されます。パッケージは署名も暗号化もされておらず、独立したレビューは受けていません。 |
| ファームウェア基盤 | 🟠 | ESP32 ノードが共有する 49 ファイルで、`tools/sync_firmware_base.py` が同一に保ちます。使っている唯一のノードである EH3D-NODE-BUNGARD 経由でのみテストされ、基板では動かしていません。 |
| 他のプログラムでの利用 | 🟡 | ワークスペースのクレートは EH3D-DRILLPRO、EH3D-ROUTEPRO、EH3D-BUNGARD-GUI、EH3D-MOTIONCNC、EH3D-SERVER が使っています（パスで参照するため、リポジトリを並べて置きます）。プロジェクトコンテナ自体はまだどのプログラムも使っていません。 |
| ドキュメント | 🟢 | 8 言語の README、フォーマットの説明、ファームウェア基盤のガイド。CI がマニフェストと変更履歴と照合します。 |

## 🎯 概要

* **プロジェクト コンテナ**：`.eh3dproj` パッケージは、`project.json`、`integrity.json`、フォルダー `layers/`・`toolpaths/`・`vision/` を持つ標準の ZIP アーカイブです。書き込みは決定的で（同じ内容なら同じバイト列）、一時ファイル経由で行われます。
* プロジェクトの**メタデータ**：名前、識別子、単位、ワークのゼロ点、レイヤー、工具、ツールパス、基準マーク。セマンティックなフォーマット バージョン（現在 1.0.0）を持ちます。新しいメジャー バージョンは中途半端に解釈せず拒否し、古いものは登録済みの移行が必要です（まだありません）。
* **整合性記録**：各エントリの SHA-256 とサイズ。エントリが変更・欠落・未記載のパッケージは開きません。
* **上限**：パッケージのフォルダーの外へ出る可能性のあるエントリ名は拒否され、大きすぎる、または多すぎるエントリも拒否されます。
* **テレメトリの型**：軸、スピンドル、ステータス ワードから読む機械の状態、重大度（警告または重大。未知のコードは重大扱い）付きの 9 つの GRBL アラーム コード。
* **テーマ**：スイートの配色。テストがすべての文字色と状態色に WCAG のコントラスト チェックを行います。
* **ノードのファームウェア基盤**（`firmware_base/` と `tools/sync_firmware_base.py`）：すべての EH3D ノードが共有する ESP32 ノード ファームウェアの部分（設定ドキュメント、ネットワーク、パネルのサーバー、ユーザーとセッション、ロールバック付き更新、ログ、自己保護、付属のテスト）を、プロジェクト名をプレースホルダーにして 1 か所に置いたものです。各ノードは自分のコピーを持ち、単独でビルドします。ツールが基盤を各ノードに書き込み、コピーのずれを知らせます。
* ワークスペースの**共有クレート**：`eh3d-view`（テーブル上のジョブ、11 のテーマ、8 言語のインターフェースの言葉）、`eh3d-archive`（ZIP アーカイブのジョブ）、`eh3d-bungard-formats`（Excellon、HP-GL、工具テーブル、設定のファイルと Gerber・DXF の外形）、`eh3d-controller`（マシンコントローラーのインターフェース、GRBL と模擬コントローラー）、`eh3d-filedialog`（開く・保存ウィンドウ）、`eh3d-fleet`（サーバーのマシンとステーション）。
* **テーブルの高さとドリルヘッドのノード：** `eh3d-levelling` は機械テーブルの高さマップ（測定点のグリッド、点の間の補間による面、傾きと高低差、ジョブの Z の補正、マップを保存するテキスト形式）を保持し、`eh3d-aux` は EH3D-NODE-BUNGARD-AUX のクライアント（状態、テーブルの測定、MJPEG のカメラ、トークン付きのリクエスト、小さな HTTP クライアント）です。

## 🧩 EH3D での役割

EH3D-COMMON はスイートの土台です。ワークスペースのクレート（表示とテーマ、ZIP アーカイブ、Bungard マシンのファイル、マシンコントローラー、ファイルウィンドウ、サーバーの機群）は、現在 EH3D-DRILLPRO、EH3D-ROUTEPRO、EH3D-BUNGARD-GUI、EH3D-MOTIONCNC、EH3D-SERVER が使っています。プロジェクトを開いたり保存したりするライブラリとプログラムはコンテナに依存する予定ですが、まだ 1 つもありません。ノードのファームウェアの土台もここにあり、EH3D-NODE-BUNGARD が最初にそれを使うノードです。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-COMMON のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>

このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## ✨ 機能

* **決定的でアトミックな書き込み：** 同じプロジェクトは常に同じバイト列になり、ファイルは一時ファイル経由で書かれるため、書き込みに失敗しても中途半端なパッケージは残りません。
* **読み込み時にすべて検査：** まず名前・数・サイズ、次に整合性記録と各エントリの照合、最後にメタデータ。最初の問題で型付きエラーとともに読み込みを止めます。
* **どのツールでも開ける形式：** 素の ZIP（deflate のみ）と素の JSON で、このクレートなしでも読めます。`docs/FORMAT.md` が文書化された規約です。
* **リーダーの互換性を維持：** 同じメジャー バージョンは未知の項目を無視して読み、新しいメジャー バージョンは拒否します。
* **失われないアラーム コード：** このクレートが知らないコードは `Unknown(n)` として保持され、重大として扱われます。
* **全ノード共通のファームウェア基盤：** 必要が生じた場所で編集し、`import` して他へ `sync` します。CI の `check` がコピーのずれを知らせます。

## 🔒 セキュリティと安全

コンテナは他人から送られてくる可能性のあるファイルを読むため、以下の各チェックは悪意のある、または壊れたパッケージへの防御であり、それぞれにテストがあります。

| 脅威 | 防御 |
|---|---|
| パッケージのフォルダーの外へ出る名前（`../x`、`/etc/x`、`C:\x`） | `check_entry_name` が、空・絶対パス・長すぎる名前（240 バイト超）、バックスラッシュ、コロン、制御文字、空・`.`・`..` の要素を、書き込み時と ZIP 内の各名前で再度拒否します |
| zip 爆弾、またはエントリ数が極端に多いパッケージ | エントリは最大 10,000 件、1 件 256 MiB、合計 1 GiB。1 件の読み込みは上限を 1 バイト超えたところで止まります |
| 途中で（偶然または故意に）変更されたエントリ | 各エントリの SHA-256 とサイズを `integrity.json` と比較し、違う・欠けている・未記載のエントリがあればパッケージ全体を拒否します |
| 同名のエントリが 2 つ | 読み込み時は重複として拒否し、書き込み時に置き換えたエントリは 1 つだけ残ります |
| 新しいプログラムのパッケージ | 新しいメジャー バージョンのフォーマットは、中途半端に解釈せず拒否します |
| 矛盾したメタデータ（空の名前、重複した工具 ID、NaN、負の直径、`layers/` の外のレイヤー） | `validate` が書き込み前と読み込み後に動き、無効なプロジェクトは書き込めません |
| unsafe コードなし | クレートは `#![forbid(unsafe_code)]` です |

**防御していないこと：** パッケージは署名も暗号化もされていません。エントリを書き換えられる人は整合性記録も書き換えられるため、検出できるのは破損や事故であり、作成者ではありません。ライブラリは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

| インターフェース | 内容 | 場所 |
|---|---|---|
| Rust API | `Package`（`new`、`add_entry`、`entry`、`entry_names`、`integrity`、`write`、`write_to_path`、`to_bytes`、`read`、`read_from_path`）、`ProjectMetadata`（`new`、`validate`、`to_json`、`read_metadata`）、テレメトリの型、`theme` | `src/` |
| ファイル形式 | `.eh3dproj`：`project.json`、`integrity.json`、データ フォルダー `layers/`・`toolpaths/`・`vision/` を持つ ZIP | [docs/FORMAT.md](docs/FORMAT.md) |
| エラー | 1 つの `Error` 列挙型：`Io`、`Json`、`Zip`、`UnsupportedFormat`、`MissingEntry`、`Integrity`、`UnsafePath`、`DuplicateEntry`、`TooLarge`、`InvalidMetadata` | `src/error.rs` |
| ファームウェア基盤のツール | `python tools/sync_firmware_base.py check`、`sync`、`import <ノード>`、`status` | [docs/FIRMWARE_BASE.md](docs/FIRMWARE_BASE.md) |

このリポジトリには、コマンドライン プログラムもサーバーもネットワーク アクセスもありません。

## ⚙️ 設定

ライブラリには設定ファイルがありません。上限はコード内の定数で、パッケージに何を入れられるかが分かるようにここに載せています。フォーマット バージョンはファイルの一部であり、設定ではありません。

| 値 | 上限 | 場所 |
|---|---|---|
| パッケージ内のエントリ数 | 1 〜 10,000 | `src/container.rs` の `MAX_ENTRIES` |
| 1 エントリのサイズ | 最大 256 MiB（268,435,456 バイト） | `MAX_ENTRY_BYTES` |
| 全エントリの合計サイズ | 最大 1 GiB | `MAX_TOTAL_BYTES` |
| エントリ名の長さ | 1 〜 240 バイト | `check_entry_name` |
| データ フォルダー | `layers/`、`toolpaths/`、`vision/` のみ | `src/project.rs` の `DATA_FOLDERS` |
| フォーマット バージョン | 1.0.0（同じメジャーは読み、新しいメジャーは拒否） | `FORMAT_VERSION` |
| 工具の直径 | 有限で 0 より大きい | `ProjectMetadata::validate` |

ファームウェア基盤には独自の設定ドキュメント（`node_settings.hpp`、`config_common.hpp`）があります。その値は各ノードのもので、EH3D-NODE-BUNGARD の README に説明があります。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.75 以降）が必要です。

```rust
use eh3d_common::{Package, ProjectMetadata};

fn main() -> eh3d_common::Result<()> {
    let mut package = Package::new(ProjectMetadata::new("Test board"));
    package.add_entry("layers/top.gbr", b"G04 test*".to_vec())?;
    package.write_to_path(std::path::Path::new("board.eh3dproj"))?;
    let again = Package::read_from_path(std::path::Path::new("board.eh3dproj"))?;   // すべてのエントリを検査
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

## 🧪 テスト

テストは 115 件です（`cargo test --workspace`、CI では Windows と Linux で実行）。モジュール内が 16 件（メタデータとその検証、アラーム コードと機械の状態、配色と WCAG コントラスト）、コンテナが 14 件（メモリとファイルでの往復、同じ内容なら同じバイト列、整合性記録、変更されたエントリ、変更されたサイズ、未記載のエントリ、欠けたエントリ、編集されたメタデータ、パッケージでないファイル、危険・悪意あるエントリ名、置き換えられたエントリ、無効なプロジェクト）です。CI は `cargo fmt --check`、警告をエラーとする `cargo clippy`、`python -I tools/ci_validate.py`（マニフェスト、バージョン、変更履歴、8 つの README）、`tools/sync_firmware_base.py check`（ファームウェア基盤のコピー）も実行します。ファームウェア基盤のホスト テスト（`firmware_base/tests/`）は、ここではなく EH3D-NODE-BUNGARD の中で動きます。八つの共有クレートがさらに 85 件を加えます：作業アーカイブ（6）、HP-GL の円弧・円・スケーリングと Gerber・DXF の外形を含む Bungard マシンのファイル（16）、スクリプト化した回線上のマシンコントローラー（7）、シーン、ビュー、テーマ、八言語のインターフェースの言葉（20）、開く・保存ウィンドウ（7）、サーバーのマシンとステーション（5）、テーブルの高さマップ（8）とドリルヘッドのノードのクライアント（4）。

## 🏗️ ビルドとリリース

バージョンは `Cargo.toml` と `eh3d.project.json` にあり、各バージョンには [CHANGELOG.md](CHANGELOG.md) の項目があります。リリースは GitHub で公開されます。ライブラリはこのリポジトリから Rust クレートとして利用し、crates.io にはありません。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-COMMON のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-COMMON/
├── src/
│   ├── container.rs                             パッケージ：読み書き、整合性、上限
│   ├── project.rs                               メタデータとその検証
│   ├── telemetry.rs                             軸、スピンドル、状態、アラーム コード
│   └── theme.rs error.rs lib.rs                 コントラスト付きの配色、エラー、クレートのルート
├── crates/                                      共有クレート：表示、ZIP、機械ファイル、コントローラー
├── firmware_base/
│   ├── core/                                    ハードウェア非依存：設定、認証、ネットワーク計画
│   ├── main/                                    ESP-IDF コード：ネットワーク、保存、パネル API
│   ├── tests/                                   上記すべてのホスト テスト
│   ├── tools/                                   パネルのパッケージ化、無視された sdkconfig 行の一覧
│   └── manifest.json                            どのファイルをどのノードへ送るか
├── tests/                                       パッケージ：往復、変更・欠落・未記載のエントリ、危険な名前
├── docs/                                        形式、ファームウェア基盤、2 つの図のデータ
├── tools/                                       CI と README のチェック、ファームウェア基盤の同期、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        Rust パッケージと固定された依存関係
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* まだどのプログラムもプロジェクトコンテナを使っていないため、フォーマットは自身のテストでしか読み書きされていません。
* パッケージは署名も暗号化もされておらず、ライブラリは独立したレビューを受けていません。
* 1.0.0 が最初のフォーマットなので、移行はありません。
* ファームウェア基盤は EH3D-NODE-BUNGARD 経由でしか試しておらず、基板では動いたことがありません。2 つ目のノードはまだありません。
* ライブラリは crates.io では公開していません。

**次のステップ**

* スイートの最初のプログラムに、これでプロジェクトを開いて保存させる（まず EH3D-CORE-LIBS）。
* ファームウェア基盤に 2 つ目のノードを加え、コピーが同一のままか確かめる。
* パッケージに任意の署名が必要かを決める。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS)：これらの型を使う予定のパーサー（Excellon、Gerber）とジオメトリ
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD)：ファームウェア基盤のコピーを持つ最初のノード
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER)：ファミリーの中心。プロジェクト コンテナはまだ使っていません
* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS)：スイートのドキュメント ハブとプロジェクト マップ
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC)：スイートが将来その機械を動かす予定のロボティクス プラットフォーム

## 📚 ドキュメント

* [プロジェクト形式](docs/FORMAT.md)：エントリ、メタデータ、整合性記録、リーダー向けのルール。
* [ファームウェア基盤](docs/FIRMWARE_BASE.md)：何を共有するか、どう同一に保つか、ノードの追加方法。
* [変更履歴](CHANGELOG.md)

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py`、`python -I tools/sync_firmware_base.py check` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照してください。
* README は 8 言語あります。文章を変更するときは 8 つのファイルすべてを変更し、2 つの図は `docs/architecture.json` と `docs/structure.json` から `tools/make_architecture.py` と `tools/make_structure.py` で描画します。
* ファームウェア基盤の変更は、いずれかのノードで行い、`tools/sync_firmware_base.py import` で取り込み、他のノードへ同期します。コピーを手で編集してはいけません。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問、アイデア、報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
