<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | 🇪🇸 <b>Español</b> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### El modelo de datos compartido de la EH3D Fabrication Suite: contenedor de proyecto, metadatos, comprobaciones de integridad y telemetría de máquina

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.5-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Comprobación de honestidad - lo que se ejecuta hoy:** una biblioteca en Rust con el contenedor de proyecto (`.eh3dproj`), los metadatos de un proyecto, las comprobaciones de integridad SHA-256, los tipos de telemetría de máquina y los colores de la suite, más la base de firmware que copia cada nodo ESP32 de la suite (70 ficheros y la herramienta que mantiene las copias idénticas). 115 pruebas se ejecutan en un ordenador. Los programas de taladrado y de fresado y la interfaz moderna leen sus ficheros y dibujan sus trabajos con los crates compartidos; nada de aquí se ha ejecutado en una máquina ni en una placa.

---

> **Estado de seguridad (safety) y de seguridad informática:** esta biblioteca no es un sistema de seguridad (safety) y con ella no se ha controlado ninguna máquina. El contenedor detecta un paquete dañado o alterado por accidente, pero no está firmado ni cifrado: no prueba quién escribió un proyecto. Un paquete de otra persona es una entrada no confiable, por eso se limitan los nombres, los tamaños y el número de entradas.

## 📋 De un vistazo

| | |
|---|---|
| Versión | `0.0.5` |
| Madurez | Andamiaje |
| Pruebas | 115 |
| Pila tecnológica | Rust (crate de biblioteca) y C++17 (base de firmware) |
| En hardware real | No aplica: una biblioteca de datos, nada aquí habla con una máquina; la base de firmware nunca se ha ejecutado en una placa |

**Ir a:** [Resumen](#-resumen) · [Arquitectura](#-arquitectura) · [Inicio rápido](#-inicio-rápido) · [Pruebas](#-pruebas) · [Estructura](#-estructura-del-repositorio) · [Relacionados](#-proyectos-relacionados)

## 🚦 Estado del proyecto

Cada línea es una parte de este repositorio y la luz dice hasta dónde llega hoy. Se escribe con lo que el repositorio puede demostrar (pruebas, compilaciones, ejecuciones), no con lo previsto.

🟢 Hecho y verificado · 🟡 Funciona, con límites conocidos · 🟠 Hecho a medias o sin verificar · 🔴 Sin hacer o sin funcionar

| Área | Estado | Qué significa aquí |
|---|---|---|
| Contenedor | 🟡 | `.eh3dproj` es un ZIP estándar con un registro SHA-256 de cada entrada; lee, escribe y rechaza entradas dañadas, alteradas, sin listar o inseguras. Hasta ahora solo lo leen y escriben sus propias pruebas. |
| Metadatos | 🟡 | Los metadatos de un proyecto (unidades, cero, capas, herramientas, trayectorias, fiduciales) se validan y llevan una versión de formato semántica; aún no hay migración porque 1.0.0 es el primer formato. |
| Telemetría y tema | 🟢 | Ejes, husillo, estados de la máquina, los códigos de alarma GRBL con su gravedad y los colores de la suite con comprobación de contraste, todo probado. |
| Pruebas | 🟢 | Pasan 115 pruebas (`cargo test --workspace`): 16 en los módulos y 14 sobre el contenedor del crate principal, y 85 en los ocho crates compartidos. |
| Seguridad | 🟡 | Se rechazan los nombres inseguros, las entradas demasiado grandes o numerosas y cualquier entrada alterada; el paquete no está firmado ni cifrado y no ha tenido revisión independiente. |
| Base de firmware | 🟠 | 49 ficheros compartidos por los nodos ESP32, mantenidos idénticos por `tools/sync_firmware_base.py`; probada solo a través de EH3D-NODE-BUNGARD, el único nodo que la usa, y nunca ejecutada en una placa. |
| Uso por otros programas | 🟡 | Los crates del workspace los usan EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-BUNGARD-GUI, EH3D-MOTIONCNC y EH3D-SERVER (por ruta, con los repositorios uno junto a otro); el contenedor de proyectos todavía no lo usa ningún programa. |
| Documentación | 🟢 | README en ocho idiomas, la descripción del formato y la guía de la base de firmware; el CI las compara con el manifiesto y el registro de cambios. |

## 🎯 Resumen

* El **contenedor de proyecto**: un paquete `.eh3dproj` es un archivo ZIP estándar con `project.json`, `integrity.json` y las carpetas `layers/`, `toolpaths/` y `vision/`. El escritor es determinista (el mismo contenido da los mismos bytes) y escribe a través de un fichero temporal.
* Los **metadatos** de un proyecto: nombre, identidad, unidades, cero de la pieza, capas, herramientas, trayectorias y fiduciales, con una versión de formato semántica (hoy 1.0.0). Una versión mayor más nueva se rechaza en lugar de entenderse a medias; una más antigua necesita una migración registrada, y todavía no existe ninguna.
* El **registro de integridad**: el SHA-256 y el tamaño de cada entrada. Un paquete cuya entrada cambió, falta o no está listada no se abre.
* Los **límites**: se rechazan los nombres de entrada que podrían salir de la carpeta del paquete, y también las entradas demasiado grandes o demasiado numerosas.
* Los **tipos de telemetría**: ejes, husillo, estados de la máquina leídos de las palabras de estado y los nueve códigos de alarma GRBL con su gravedad (aviso o crítica; un código desconocido cuenta como crítico).
* El **tema**: los colores de la suite, con una comprobación de contraste WCAG que las pruebas aplican a cada color de texto y de estado.
* La **base de firmware de los nodos** (`firmware_base/`, con `tools/sync_firmware_base.py`): la parte del firmware de los nodos ESP32 que comparten todos los nodos EH3D - el documento de ajustes, la red, el servidor del panel, usuarios y sesiones, la actualización con vuelta atrás, el registro, la autoprotección y las pruebas que los acompañan - escrita una vez con el nombre del proyecto como marcadores. Cada nodo guarda su propia copia y compila por su cuenta; la herramienta escribe la base en cada nodo y avisa cuando una copia se ha desviado.
* Los **crates compartidos** del workspace: `eh3d-view` (el trabajo en la mesa, once temas, las palabras de la interfaz en ocho idiomas), `eh3d-archive` (trabajos en archivos ZIP), `eh3d-bungard-formats` (los ficheros Excellon, HP-GL, tablas de herramientas y configuración, los contornos Gerber y DXF), `eh3d-controller` (la interfaz de los controladores de máquina, GRBL y uno simulado), `eh3d-filedialog` (las ventanas Abrir y Guardar) y `eh3d-fleet` (las máquinas y estaciones del servidor).
* **La altura de la mesa y el nodo del cabezal de taladro:** `eh3d-levelling` guarda el mapa de alturas de la mesa de una máquina (una malla de puntos medidos, la superficie entre ellos por interpolación, la inclinación y la diferencia, la corrección de la Z de un trabajo y una forma de texto para guardar el mapa), y `eh3d-aux` es el cliente de EH3D-NODE-BUNGARD-AUX (su estado, su medida de la mesa, su cámara como MJPEG, las peticiones con el token y un pequeño cliente HTTP).

## 🧩 Papel en EH3D

EH3D-COMMON es la base de la suite. Los crates de su workspace (la vista y los temas, los archivos ZIP, los ficheros de las máquinas Bungard, los controladores de máquina, las ventanas de ficheros y la flota del servidor) los usan hoy EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-BUNGARD-GUI, EH3D-MOTIONCNC y EH3D-SERVER; está previsto que las bibliotecas y los programas que abren o guardan un proyecto dependan del contenedor, y ninguno existe todavía. La base del firmware de los nodos también vive aquí: EH3D-NODE-BUNGARD es el primer nodo que la usa.

## 🏛️ Arquitectura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitectura de EH3D-COMMON: quién lo usa, qué contiene y de qué depende" width="100%">
</p>

Cómo encajan las partes de este repositorio: quién lo usa, qué contiene y de qué depende. La imagen se dibuja desde `docs/architecture.json` y se comprueba contra el código.

## ✨ Funciones

* **Escritura determinista y atómica:** el mismo proyecto da siempre los mismos bytes y el fichero se escribe a través de uno temporal, así que una escritura fallida no deja un paquete a medias.
* **Todo comprobado al leer:** primero nombres, número y tamaños, luego el registro de integridad contra cada entrada y por último los metadatos; el primer problema detiene la lectura con un error tipado.
* **Un formato que abre cualquier herramienta:** ZIP normal (solo deflate) y JSON normal, legible sin este crate; `docs/FORMAT.md` es el contrato escrito.
* **Los lectores siguen siendo compatibles:** se lee la misma versión mayor ignorando lo que no se conoce; una versión mayor más nueva se rechaza.
* **Códigos de alarma que no se pierden:** un código que este crate no conoce se conserva como `Unknown(n)` y se trata como crítico.
* **Una base de firmware para todos los nodos:** se edita donde apareció la necesidad, se hace `import`, se hace `sync` en los demás y el `check` del CI avisa cuando una copia se desvía.

## 🔒 Seguridad y protección

El contenedor lee ficheros que pueden enviar otras personas, así que cada comprobación de abajo es una defensa contra un paquete hostil o dañado, y cada una tiene su prueba.

| Amenaza | Defensa |
|---|---|
| Un nombre que sale de la carpeta del paquete (`../x`, `/etc/x`, `C:\x`) | `check_entry_name` rechaza un nombre vacío, absoluto o demasiado largo (más de 240 bytes), una barra invertida, dos puntos, un carácter de control y cualquier parte vacía, `.` o `..`, al escribir y de nuevo con cada nombre hallado en un ZIP |
| Una bomba zip o un paquete con un número enorme de entradas | como máximo 10 000 entradas, 256 MiB por entrada y 1 GiB en total; la lectura de una entrada se detiene un byte por encima de su límite |
| Una entrada alterada por el camino (por accidente o a propósito) | se comparan el SHA-256 y el tamaño de cada entrada con `integrity.json`; una entrada distinta, ausente o sin listar rechaza todo el paquete |
| Dos entradas con el mismo nombre | se rechazan como duplicado al leer, y una entrada reemplazada al escribir conserva una sola copia |
| Un paquete de un programa más nuevo | una versión mayor de formato más nueva se rechaza en lugar de entenderse a medias |
| Metadatos incoherentes (nombre vacío, id de herramienta repetido, NaN, diámetro negativo, una capa fuera de `layers/`) | `validate` se ejecuta antes de escribir y después de leer; un proyecto no válido no se puede escribir |
| Sin código inseguro | el crate es `#![forbid(unsafe_code)]` |

**Lo que no se defiende:** el paquete no está firmado ni cifrado. Quien pueda reescribir las entradas puede reescribir también el registro de integridad, así que detecta daños y accidentes, no a un autor. La biblioteca no ha tenido revisión independiente. Informa de una vulnerabilidad en privado (véase [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

| Interfaz | Qué es | Dónde |
|---|---|---|
| API de Rust | `Package` (`new`, `add_entry`, `entry`, `entry_names`, `integrity`, `write`, `write_to_path`, `to_bytes`, `read`, `read_from_path`), `ProjectMetadata` (`new`, `validate`, `to_json`, `read_metadata`), los tipos de telemetría y `theme` | `src/` |
| Formato de fichero | `.eh3dproj`: un ZIP con `project.json`, `integrity.json` y las carpetas de datos `layers/`, `toolpaths/` y `vision/` | [docs/FORMAT.md](docs/FORMAT.md) |
| Errores | un solo enum `Error`: `Io`, `Json`, `Zip`, `UnsupportedFormat`, `MissingEntry`, `Integrity`, `UnsafePath`, `DuplicateEntry`, `TooLarge`, `InvalidMetadata` | `src/error.rs` |
| Herramienta de la base de firmware | `python tools/sync_firmware_base.py check`, `sync`, `import <nodo>` y `status` | [docs/FIRMWARE_BASE.md](docs/FIRMWARE_BASE.md) |

Este repositorio no tiene programa de línea de comandos, ni servidor, ni acceso a la red.

## ⚙️ Configuración

La biblioteca no tiene fichero de configuración: sus límites son constantes del código, listadas aquí para que quien lea sepa qué puede contener un paquete. La versión del formato forma parte del fichero, no es un ajuste.

| Valor | Límite | Dónde |
|---|---|---|
| Entradas en un paquete | de 1 a 10 000 | `MAX_ENTRIES` en `src/container.rs` |
| Tamaño de una entrada | hasta 256 MiB (268 435 456 bytes) | `MAX_ENTRY_BYTES` |
| Tamaño de todas las entradas juntas | hasta 1 GiB | `MAX_TOTAL_BYTES` |
| Longitud de un nombre de entrada | de 1 a 240 bytes | `check_entry_name` |
| Carpetas de datos | solo `layers/`, `toolpaths/`, `vision/` | `DATA_FOLDERS` en `src/project.rs` |
| Versión del formato | 1.0.0 (se lee la misma versión mayor, se rechaza una mayor más nueva) | `FORMAT_VERSION` |
| Diámetro de herramienta | finito y mayor que 0 | `ProjectMetadata::validate` |

La base de firmware tiene su propio documento de ajustes (`node_settings.hpp`, `config_common.hpp`); sus valores pertenecen a cada nodo y se describen en el README de EH3D-NODE-BUNGARD.

## 🚀 Inicio rápido

Necesitas una cadena de herramientas estable de Rust (1.75 o posterior).

```rust
use eh3d_common::{Package, ProjectMetadata};

fn main() -> eh3d_common::Result<()> {
    let mut package = Package::new(ProjectMetadata::new("Test board"));
    package.add_entry("layers/top.gbr", b"G04 test*".to_vec())?;
    package.write_to_path(std::path::Path::new("board.eh3dproj"))?;
    let again = Package::read_from_path(std::path::Path::new("board.eh3dproj"))?;   // comprueba cada entrada
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

## 🧪 Pruebas

La suite tiene 115 pruebas (`cargo test --workspace`, ejecutadas en Windows y Linux en el CI): 16 dentro de los módulos (los metadatos y su validación, los códigos de alarma y los estados de la máquina, los colores y su contraste WCAG) y 14 sobre el contenedor (ida y vuelta en memoria y en fichero, los mismos bytes para el mismo contenido, el registro de integridad, una entrada cambiada, un tamaño cambiado, una entrada sin listar, una entrada ausente, metadatos editados, un fichero que no es un paquete, nombres de entrada inseguros y hostiles, una entrada reemplazada y un proyecto no válido). El CI también ejecuta `cargo fmt --check` y `cargo clippy` con advertencias como errores, `python -I tools/ci_validate.py` (manifiesto, versiones, registro de cambios y los ocho README) y `tools/sync_firmware_base.py check` (las copias de la base de firmware). Las pruebas en el ordenador de la base de firmware (`firmware_base/tests/`) se ejecutan dentro de EH3D-NODE-BUNGARD, no aquí. Los ocho crates compartidos añaden 85 más: los archivos de trabajo (6), los ficheros de las máquinas Bungard, con los arcos, círculos y escalado de HP-GL y los contornos Gerber y DXF (16), los controladores de máquina sobre una línea simulada (19) y la escena, las vistas, los temas y las palabras de la interfaz en ocho idiomas (20), las ventanas Abrir y Guardar (7), las máquinas y estaciones del servidor (5) y el mapa de alturas de la mesa (8) y el cliente del nodo del cabezal de taladro (4).

## 🏗️ Compilación y publicación

La versión vive en `Cargo.toml` y en `eh3d.project.json`, y cada versión tiene su entrada en [CHANGELOG.md](CHANGELOG.md). Las versiones se publican en GitHub; la biblioteca se consume como crate de Rust desde este repositorio y no está en crates.io.

## 📂 Estructura del repositorio

<p align="center">
  <img src="images/repository-structure.svg" alt="Estructura del repositorio EH3D-COMMON" width="100%">
</p>

<details>
<summary>Versión en texto del árbol</summary>

```text
EH3D-COMMON/
├── src/
│   ├── container.rs                             el paquete: lectura, escritura, integridad y límites
│   ├── project.rs                               los metadatos y su validación
│   ├── telemetry.rs                             ejes, husillo, estados y códigos de alarma
│   └── theme.rs error.rs lib.rs                 los colores con contraste, los errores y la raíz del crate
├── crates/                                      crates compartidos: vista, ZIP, ficheros de máquina, controladores
├── firmware_base/
│   ├── core/                                    sin hardware: ajustes, autenticación, plan de red
│   ├── main/                                    código ESP-IDF: red, almacén, API del panel
│   ├── tests/                                   las pruebas en el ordenador de todo lo anterior
│   ├── tools/                                   empaqueta el panel; lista líneas de sdkconfig ignoradas
│   └── manifest.json                            qué fichero va a qué nodo
├── tests/                                       el paquete: idas y vueltas, entradas cambiadas, ausentes y sin listar, nombres inseguros
├── docs/                                        el formato, la base de firmware y los datos de las dos imágenes
├── tools/                                       comprobación del CI y del README, sincronización de la base de firmware y herramientas de imágenes
├── images/                                      banner y las imágenes de Arquitectura y Estructura del repositorio
├── .github/                                     flujos de CI y de publicación, plantillas de issues y pull requests
├── eh3d.project.json                            manifiesto: versión, rol, madurez y comando de compilación
├── Cargo.toml Cargo.lock                        el paquete de Rust y sus dependencias fijadas
├── CHANGELOG.md                                 qué cambió en cada versión
├── SECURITY.md SUPPORT.md                       cómo informar de una vulnerabilidad y dónde preguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   cómo contribuir, el código de conducta y la licencia
```

</details>

## 🗺️ Hoja de ruta y límites conocidos

**Límites conocidos hoy**

* Ningún programa usa todavía el contenedor de proyectos, así que el formato solo lo han leído y escrito sus propias pruebas.
* El paquete no está firmado ni cifrado, y la biblioteca no ha tenido revisión independiente.
* No hay migración, porque 1.0.0 es el primer formato.
* La base de firmware solo se ha ejercitado a través de EH3D-NODE-BUNGARD y nunca se ha ejecutado en una placa; todavía no existe un segundo nodo.
* La biblioteca no está publicada en crates.io.

**Siguiente**

* Que el primer programa de la suite abra y guarde un proyecto con ella (primero EH3D-CORE-LIBS).
* Añadir el segundo nodo a la base de firmware y comprobar que las copias siguen idénticas.
* Decidir si los paquetes necesitan una firma opcional.

## 🌍 Idiomas

El README y los textos existen en inglés, español, francés, italiano, alemán, chino, japonés y portugués de Portugal.

## 🔗 Proyectos relacionados

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): los analizadores (Excellon, Gerber) y la geometría que usarán estos tipos
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): el primer nodo que guarda una copia de la base de firmware
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): el centro de la familia; todavía no usa el contenedor de proyecto
* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): el centro de documentación de la suite y su mapa de proyectos
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): la plataforma de robótica cuyas máquinas la suite debe manejar en el futuro

## 📚 Documentación

* [Formato de proyecto](docs/FORMAT.md): las entradas, los metadatos, el registro de integridad y las reglas para los lectores.
* [La base de firmware](docs/FIRMWARE_BASE.md): qué se comparte, cómo se mantiene idéntico y cómo añadir un nodo.
* [Registro de cambios](CHANGELOG.md)

## 🤝 Contribuir y comunidad

* Antes de una pull request ejecuta `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `python -I tools/ci_validate.py` y `python -I tools/sync_firmware_base.py check`: todo debe pasar. Consulta [CONTRIBUTING.md](CONTRIBUTING.md) y el [código de conducta](CODE_OF_CONDUCT.md).
* El README existe en ocho idiomas: un cambio en el texto se hace en los ocho archivos, y las dos imágenes se dibujan a partir de `docs/architecture.json` y `docs/structure.json` con `tools/make_architecture.py` y `tools/make_structure.py`.
* Un cambio en la base de firmware se hace en un nodo, se importa con `tools/sync_firmware_base.py import` y se sincroniza en los demás; no edites nunca una copia a mano.
* Informa de una vulnerabilidad en privado: [SECURITY.md](SECURITY.md). Preguntas, ideas e informes: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCIA

Software: GPL-3.0-or-later. Véase [LICENSE](LICENSE). Los diseños de hardware de la suite usan CERN OHL-S.
