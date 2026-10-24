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

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Software | 🟡 | O contentor, os metadados e os tipos de telemetria funcionam como biblioteca Rust; nenhum programa da suite os usa ainda. |
| Testes | 🟢 | Passam 29 testes (`cargo test`): ida e volta, entradas alteradas, em falta ou não listadas, e nomes inseguros. |
| Formato de dados | 🟡 | O `.eh3dproj` é um ZIP normal com o SHA-256 de cada entrada, descrito em `docs/FORMAT.md`; uma versão principal mais recente é recusada. Por agora só os seus próprios testes o leem e escrevem. |
| Segurança | 🟡 | Os nomes de entrada inseguros e as entradas demasiado grandes ou numerosas são recusados, e um pacote alterado não abre; o pacote não está assinado nem cifrado e não teve revisão independente. |
| Base de firmware partilhada | 🟠 | A parte do firmware de nó que todos os nós partilham só é testada através do EH3D-NODE-BUNGARD, o único nó que a usa; ainda não existe um segundo nó. |
| Hardware real | 🟠 | A biblioteca não precisa de nenhum, mas a base de firmware que contém nunca correu numa placa. |
| Documentação | 🟢 | README em oito idiomas, a descrição do formato e o guia da base de firmware; as verificações comparam-nos com o manifesto e o registo de alterações. |

## 🎯 Descrição geral

* O **contentor de projeto**: um pacote `.eh3dproj` é um ficheiro ZIP padrão com `project.json`, `integrity.json` e as pastas `layers/`, `toolpaths/` e `vision/`.
* Os **metadados** de um projeto: nome, identidade, unidades, zero da peça, camadas, ferramentas, trajetórias e marcas fiduciais, com uma versão de formato semântica. Uma versão principal mais recente é recusada em vez de ser entendida a meio.
* O **registo de integridade**: o SHA-256 e o tamanho de cada entrada. Um pacote em que uma entrada mudou, falta ou não está listada não abre.
* Os **limites**: são recusados os nomes de entrada que poderiam sair da pasta do pacote, bem como as entradas demasiado grandes ou demasiado numerosas.
* Os **tipos de telemetria**: eixos, fuso, estados da máquina e códigos de alarme GRBL com a sua gravidade.
* O **tema**: as cores da suite, com uma verificação de contraste.
* A **base do firmware dos nós** (`firmware_base/`, com `tools/sync_firmware_base.py`): a parte do firmware dos nós ESP32 que todos os nós EH3D partilham - o documento de definições, a rede, o servidor do painel, utilizadores e sessões, a atualização com reversão, o registo, a autoproteção e os testes que as acompanham - escrita uma só vez com o nome do projeto como marcadores. Cada nó guarda a sua própria cópia e compila por si; a ferramenta escreve a base em cada nó e avisa quando uma cópia se desviou.

## 🧩 Papel no EH3D

O EH3D-COMMON é a base da suite: está previsto que as bibliotecas e os programas que abrem ou guardam um projeto dependam dele. Nenhum existe ainda. A base do firmware dos nós também vive aqui: o EH3D-NODE-BUNGARD é o primeiro nó que a usa.

## 📂 Estrutura do repositório

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── firmware_base/a parte partilhada do firmware dos nós
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
* [A base do firmware](docs/FIRMWARE_BASE.md): o que se partilha, como se mantém idêntica e como acrescentar um nó.
* [Registo de alterações](CHANGELOG.md)
* [Como contribuir](CONTRIBUTING.md)
* [Política de segurança](SECURITY.md)
* [Suporte](SUPPORT.md)

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.
