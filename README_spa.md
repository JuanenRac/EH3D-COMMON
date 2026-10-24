<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### El modelo de datos compartido de la EH3D Fabrication Suite: contenedor de proyecto, metadatos, comprobaciones de integridad y telemetría de la máquina

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que se ejecuta hoy:** una biblioteca en Rust con el contenedor de proyecto (`.eh3dproj`), los metadatos de un proyecto, comprobaciones de integridad SHA-256 y los tipos de telemetría de la máquina. Todo se ejecuta en pruebas en el ordenador de desarrollo; ningún programa de la suite la usa todavía y nada se ha ejecutado en una máquina.

---

> **Estado de seguridad (safety) y de seguridad informática:** esta biblioteca no es un sistema de seguridad funcional y no se ha controlado ninguna máquina con ella. El contenedor detecta un paquete dañado o alterado por accidente, pero no está firmado ni cifrado: no demuestra quién escribió un proyecto. Un paquete de otra persona es una entrada no fiable, por eso los nombres de entrada, los tamaños y el número de entradas están limitados.

## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.1` |
| Madurez | Andamiaje |
| Pruebas | 29 |
| Pila tecnológica | Rust |
| En hardware real | No aplica: es una biblioteca de datos, nada aquí habla con una máquina |

## 🎯 Descripción general

* El **contenedor de proyecto**: un paquete `.eh3dproj` es un archivo ZIP estándar con `project.json`, `integrity.json` y las carpetas `layers/`, `toolpaths/` y `vision/`.
* Los **metadatos** de un proyecto: nombre, identidad, unidades, cero de la pieza, capas, herramientas, trayectorias y marcas fiduciales, con una versión de formato semántica. Una versión mayor más nueva se rechaza en lugar de entenderse a medias.
* El **registro de integridad**: el SHA-256 y el tamaño de cada entrada. Un paquete cuya entrada cambió, falta o no figura en la lista no se abre.
* Los **límites**: se rechazan los nombres de entrada que podrían salir de la carpeta del paquete, y también las entradas demasiado grandes o demasiado numerosas.
* Los **tipos de telemetría**: ejes, husillo, estados de la máquina y códigos de alarma GRBL con su gravedad.
* El **tema**: los colores de la suite, con una comprobación de contraste.
* La **base del firmware de los nodos** (`firmware_base/`, con `tools/sync_firmware_base.py`): la parte del firmware de los nodos ESP32 que comparten todos los nodos EH3D - el documento de ajustes, la red, el servidor del panel, usuarios y sesiones, la actualización con vuelta atrás, el registro, la autoprotección y las pruebas que las acompañan - escrita una vez con el nombre del proyecto como marcadores. Cada nodo guarda su propia copia y compila por su cuenta; la herramienta escribe la base en cada nodo y avisa cuando una copia se ha desviado.

## 🧩 Papel en EH3D

EH3D-COMMON es la base de la suite: se prevé que las bibliotecas y los programas que abren o guardan un proyecto dependan de ella. Ninguno existe todavía. La base del firmware de los nodos también vive aquí: EH3D-NODE-BUNGARD es el primer nodo que la usa.

## 📂 Estructura del repositorio

```text
EH3D-COMMON/
├── src/          container.rs, project.rs, telemetry.rs, theme.rs, error.rs
├── tests/        the package: round trips, changed, missing and unlisted entries, unsafe names
├── docs/         FORMAT.md: the .eh3dproj format
├── firmware_base/la parte compartida del firmware de los nodos
├── tools/        ci_validate.py: manifest, versions, changelog and README checks
├── images/       banner
└── eh3d.project.json   the manifest of the project
```

## 🛠️ COMPILAR Y EJECUTAR

Necesitas una cadena de herramientas de Rust estable.

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
python -I tools/ci_validate.py
```

## 🔗 Proyectos relacionados

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): previsto: analizadores y motores de geometría que usarán estos tipos
* [EH3D-SUITE-LAUNCHER](https://github.com/JuanenRac/EH3D-SUITE-LAUNCHER): previsto: el centro que abrirá y guardará proyectos
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): la plataforma de robótica cuyas máquinas la suite deberá controlar en el futuro

## 📚 Documentación y comunidad

* [Formato de proyecto](docs/FORMAT.md): las entradas, los metadatos, el registro de integridad y las reglas para quien lee.
* [La base del firmware](docs/FIRMWARE_BASE.md): qué se comparte, cómo se mantiene idéntico y cómo añadir un nodo.
* [Registro de cambios](CHANGELOG.md)
* [Cómo contribuir](CONTRIBUTING.md)
* [Política de seguridad](SECURITY.md)
* [Soporte](SUPPORT.md)

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
