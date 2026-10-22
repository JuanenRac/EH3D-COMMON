<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### O modelo de dados partilhado da EH3D Fabrication Suite: contentor de projeto, metadados, verificações de integridade e telemetria da máquina

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca em Rust com o contentor de projeto (`.eh3dproj`), os metadados de um projeto, verificações de integridade SHA-256 e os tipos de telemetria da máquina. Tudo funciona em testes no computador de desenvolvimento; nenhum programa da suite a usa ainda e nada foi executado numa máquina.

---

> **Estado de segurança (safety) e de segurança informática:** esta biblioteca não é um sistema de segurança e nenhuma máquina foi controlada com ela. O contentor deteta um pacote danificado ou alterado por acidente, mas não está assinado nem cifrado: não prova quem escreveu um projeto. Um pacote de outra pessoa é uma entrada não fiável, por isso os nomes das entradas, os tamanhos e o número de entradas estão limitados.

## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.1` |
| Maturidade | Estrutura inicial |
| Testes | 29 |
| Tecnologias | Rust |
| Em hardware real | Não aplicável: é uma biblioteca de dados, nada aqui comunica com uma máquina |

## 🎯 Descrição geral

* O **contentor de projeto**: um pacote `.eh3dproj` é um ficheiro ZIP padrão com `project.json`, `integrity.json` e as pastas `layers/`, `toolpaths/` e `vision/`.
* Os **metadados** de um projeto: nome, identidade, unidades, zero da peça, camadas, ferramentas, trajetórias e marcas fiduciais, com uma versão de formato semântica. Uma versão principal mais recente é recusada em vez de ser entendida a meio.
* O **registo de integridade**: o SHA-256 e o tamanho de cada entrada. Um pacote em que uma entrada mudou, falta ou não está listada não abre.
* Os **limites**: são recusados os nomes de entrada que poderiam sair da pasta do pacote, bem como as entradas demasiado grandes ou demasiado numerosas.
* Os **tipos de telemetria**: eixos, fuso, estados da máquina e códigos de alarme GRBL com a sua gravidade.
* O **tema**: as cores da suite, com uma verificação de contraste.

## 🧩 Papel no EH3D

O EH3D-COMMON é a base da suite: está previsto que as bibliotecas e os programas que abrem ou guardam um projeto dependam dele. Nenhum existe ainda.

## 📂 Estrutura do repositório

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ COMPILAR E EXECUTAR

É preciso uma cadeia de ferramentas Rust estável.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Projetos relacionados

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): previsto: analisadores e motores de geometria que usarão estes tipos
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): previsto: o centro que abrirá e guardará projetos
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): a plataforma de robótica cujas máquinas a suite deverá controlar no futuro

## 📚 Documentação e comunidade

* [Formato do projeto](docs/FORMAT.md): as entradas, os metadados, o registo de integridade e as regras para quem lê.
* [Registo de alterações](CHANGELOG.md)
* [Como contribuir](CONTRIBUTING.md)
* [Política de segurança](SECURITY.md)
* [Suporte](SUPPORT.md)

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
