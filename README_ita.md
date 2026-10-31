<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-COMMON banner" width="100%">
</p>

# 🧱 EH3D-COMMON

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Il modello di dati condiviso della EH3D Fabrication Suite: contenitore di progetto, metadati, controlli di integrità e telemetria macchina

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust con il contenitore di progetto (`.eh3dproj`), i metadati di un progetto, i controlli di integrità SHA-256, i tipi di telemetria macchina e i colori della suite, più la base del firmware che ogni nodo ESP32 della suite copia (70 file e lo strumento che mantiene le copie identiche). 164 test girano su un computer. I programmi di foratura e fresatura e l'interfaccia moderna leggono i loro file e disegnano i loro lavori con i crate condivisi; nulla di qui è stato eseguito su una macchina o su una scheda.

---

> **Stato di sicurezza funzionale e informatica:** questa libreria non è un sistema di sicurezza funzionale e nessuna macchina è stata controllata con essa. Il contenitore rileva un pacchetto danneggiato o alterato per errore, ma non è firmato né cifrato: non prova chi ha scritto un progetto. Un pacchetto di un'altra persona è un input non fidato, per questo nomi, dimensioni e numero delle voci sono limitati.

## 📋 In breve

| | |
|---|---|
| Versione | `0.0.6` |
| Maturità | Impalcatura |
| Test | 164 |
| Stack tecnologico | Rust (crate di libreria) e C++17 (base del firmware) |
| Su hardware reale | Non applicabile: una libreria di dati, nulla qui parla con una macchina; la base del firmware non è mai stata eseguita su una scheda |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura](#-struttura-del-repository) · [Correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Contenitore | 🟡 | `.eh3dproj` è uno ZIP standard con un registro SHA-256 di ogni voce; legge, scrive e rifiuta voci danneggiate, alterate, non elencate o non sicure. Finora lo leggono e scrivono solo i suoi test. |
| Metadati | 🟡 | I metadati di un progetto (unità, zero, livelli, utensili, percorsi, fiduciali) sono convalidati e portano una versione di formato semantica; non c'è ancora migrazione perché 1.0.0 è il primo formato. |
| Telemetria e tema | 🟢 | Assi, mandrino, stati della macchina, codici di allarme GRBL con la loro gravità e colori della suite con controllo del contrasto, tutto testato. |
| Test | 🟢 | Passano 164 test (`cargo test --workspace`): 16 nei moduli e 14 sul contenitore del crate principale, e 134 nei nove crate condivisi. |
| Sicurezza | 🟡 | Nomi non sicuri, voci troppo grandi o troppo numerose e qualsiasi voce alterata vengono rifiutati; il pacchetto non è firmato né cifrato e non ha avuto una revisione indipendente. |
| Base del firmware | 🟠 | 49 file condivisi dai nodi ESP32, mantenuti identici da `tools/sync_firmware_base.py`; provata solo tramite EH3D-NODE-BUNGARD, l'unico nodo che la usa, e mai eseguita su una scheda. |
| Uso da altri programmi | 🟡 | I crate del workspace sono usati da EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-BUNGARD-GUI, EH3D-MOTIONCNC ed EH3D-SERVER (per percorso, con i repository affiancati); il contenitore di progetto in sé non è ancora usato da alcun programma. |
| Documentazione | 🟢 | README in otto lingue, la descrizione del formato e la guida alla base del firmware; la CI le confronta con il manifesto e il registro delle modifiche. |

## 🎯 Panoramica

* Il **contenitore di progetto**: un pacchetto `.eh3dproj` è un archivio ZIP standard con `project.json`, `integrity.json` e le cartelle `layers/`, `toolpaths/` e `vision/`. La scrittura è deterministica (lo stesso contenuto dà gli stessi byte) e passa per un file temporaneo.
* I **metadati** di un progetto: nome, identità, unità, zero del pezzo, livelli, utensili, percorsi e fiduciali, con una versione di formato semantica (oggi 1.0.0). Una versione maggiore più nuova viene rifiutata invece di essere capita a metà; una più vecchia richiede una migrazione registrata, e non ne esiste ancora nessuna.
* Il **registro di integrità**: lo SHA-256 e la dimensione di ogni voce. Un pacchetto con una voce cambiata, mancante o non elencata non si apre.
* I **limiti**: i nomi di voce che potrebbero uscire dalla cartella del pacchetto vengono rifiutati, così come le voci troppo grandi o troppo numerose.
* I **tipi di telemetria**: assi, mandrino, stati della macchina letti dalle parole di stato e i nove codici di allarme GRBL con la loro gravità (avviso o critico; un codice sconosciuto conta come critico).
* Il **tema**: i colori della suite, con un controllo del contrasto WCAG che i test applicano a ogni colore di testo e di stato.
* La **base del firmware dei nodi** (`firmware_base/`, con `tools/sync_firmware_base.py`): la parte del firmware dei nodi ESP32 che ogni nodo EH3D condivide - il documento delle impostazioni, la rete, il server del pannello, utenti e sessioni, l'aggiornamento con ripristino, il registro, l'autoprotezione e i test relativi - scritta una volta con il nome del progetto come segnaposto. Ogni nodo tiene la propria copia e si compila da solo; lo strumento scrive la base in ogni nodo e segnala quando una copia si è discostata.
* I **crate condivisi** del workspace: `eh3d-view` (il lavoro sul piano, undici temi, le parole dell'interfaccia in otto lingue), `eh3d-archive` (lavori in archivi ZIP), `eh3d-bungard-formats` (i file Excellon, HP-GL, tabelle utensili e configurazione, i contorni Gerber e DXF), `eh3d-controller` (l'interfaccia dei controllori di macchina, GRBL e uno simulato), `eh3d-filedialog` (le finestre Apri e Salva) e `eh3d-fleet` (le macchine e stazioni del server).
* **L'altezza del tavolo e il nodo della testa di foratura:** `eh3d-levelling` conserva la mappa delle altezze del tavolo di una macchina (una griglia di punti misurati, la superficie tra loro per interpolazione, l'inclinazione e la differenza, la correzione della Z di un lavoro e una forma di testo per conservare la mappa), e `eh3d-aux` è il client di EH3D-NODE-BUNGARD-AUX (il suo stato, la sua misura del tavolo, la sua telecamera come MJPEG, le richieste con il token e un piccolo client HTTP).

## 🧩 Ruolo in EH3D

EH3D-COMMON è la base della suite. I crate del suo workspace (la vista e i temi, gli archivi ZIP, i file delle macchine Bungard, i controllori di macchina, le finestre dei file e la flotta del server) sono usati oggi da EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-BUNGARD-GUI, EH3D-MOTIONCNC ed EH3D-SERVER; le librerie e i programmi che aprono o salvano un progetto dovranno dipendere dal contenitore, e nessuno esiste ancora. Anche la base del firmware dei nodi vive qui: EH3D-NODE-BUNGARD è il primo nodo che la usa.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-COMMON: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>

Come si incastrano le parti di questo repository: chi lo usa, che cosa contiene e da che cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata rispetto al codice.

## ✨ Funzionalità

* **Scrittura deterministica e atomica:** lo stesso progetto dà sempre gli stessi byte e il file viene scritto tramite uno temporaneo, quindi una scrittura fallita non lascia un pacchetto a metà.
* **Tutto controllato in lettura:** prima nomi, numero e dimensioni, poi il registro di integrità contro ogni voce e infine i metadati; il primo problema ferma la lettura con un errore tipizzato.
* **Un formato che ogni strumento apre:** ZIP semplice (solo deflate) e JSON semplice, leggibili senza questo crate; `docs/FORMAT.md` è il contratto scritto.
* **I lettori restano compatibili:** la stessa versione maggiore viene letta ignorando ciò che non conosce; una versione maggiore più nuova viene rifiutata.
* **Codici di allarme che non si perdono:** un codice che questo crate non conosce resta come `Unknown(n)` e conta come critico.
* **Una base del firmware per tutti i nodi:** si modifica dove è nata l'esigenza, si fa `import`, si fa `sync` negli altri e il `check` della CI segnala quando una copia si discosta.
* **L'analizzatore di qualità e l'anello di LED della testa:** `eh3d-inspect` giudica una foto della telecamera della testa di foratura (fori presenti, rotondi, della misura giusta e al loro posto; canali fresati larghi quanto l'utensile, regolari e senza restringimenti; una foto abbastanza buona per giudicare) con un profilo di tolleranze, e dice conforme, non conforme o non valutabile. `eh3d-aux` accende e spegne anche l'anello di LED della telecamera, mostra a colori com'è andato un controllo, scatta una foto e consegna il profilo al server.

## 🔒 Sicurezza e protezione

Il contenitore legge file che altre persone possono inviare, quindi ogni controllo qui sotto è una difesa contro un pacchetto ostile o danneggiato, e ognuno ha il suo test.

| Minaccia | Difesa |
|---|---|
| Un nome che esce dalla cartella del pacchetto (`../x`, `/etc/x`, `C:\x`) | `check_entry_name` rifiuta un nome vuoto, assoluto o troppo lungo (oltre 240 byte), una barra rovesciata, i due punti, un carattere di controllo e qualsiasi parte vuota, `.` o `..`, in scrittura e di nuovo per ogni nome trovato in uno ZIP |
| Una zip bomb o un pacchetto con un numero enorme di voci | al massimo 10.000 voci, 256 MiB per voce e 1 GiB in totale; la lettura di una voce si ferma un byte oltre il suo limite |
| Una voce alterata per strada (per errore o di proposito) | lo SHA-256 e la dimensione di ogni voce vengono confrontati con `integrity.json`; una voce diversa, mancante o non elencata fa rifiutare l'intero pacchetto |
| Due voci con lo stesso nome | rifiutate come duplicato in lettura, e una voce sostituita in scrittura conserva una sola copia |
| Un pacchetto di un programma più nuovo | una versione maggiore di formato più nuova viene rifiutata invece di essere capita a metà |
| Metadati incoerenti (nome vuoto, id utensile ripetuto, NaN, diametro negativo, un livello fuori da `layers/`) | `validate` gira prima della scrittura e dopo la lettura; un progetto non valido non si può scrivere |
| Nessun codice unsafe | il crate è `#![forbid(unsafe_code)]` |

**Ciò che non è difeso:** il pacchetto non è firmato né cifrato. Chi può riscrivere le voci può riscrivere anche il registro di integrità, quindi rileva danni e incidenti, non un autore. La libreria non ha avuto una revisione indipendente. Segnala una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

| Interfaccia | Che cos'è | Dove |
|---|---|---|
| API Rust | `Package` (`new`, `add_entry`, `entry`, `entry_names`, `integrity`, `write`, `write_to_path`, `to_bytes`, `read`, `read_from_path`), `ProjectMetadata` (`new`, `validate`, `to_json`, `read_metadata`), i tipi di telemetria e `theme` | `src/` |
| Formato di file | `.eh3dproj`: uno ZIP con `project.json`, `integrity.json` e le cartelle dati `layers/`, `toolpaths/` e `vision/` | [docs/FORMAT.md](docs/FORMAT.md) |
| Errori | un solo enum `Error`: `Io`, `Json`, `Zip`, `UnsupportedFormat`, `MissingEntry`, `Integrity`, `UnsafePath`, `DuplicateEntry`, `TooLarge`, `InvalidMetadata` | `src/error.rs` |
| Strumento della base del firmware | `python tools/sync_firmware_base.py check`, `sync`, `import <nodo>` e `status` | [docs/FIRMWARE_BASE.md](docs/FIRMWARE_BASE.md) |

Questo repository non ha programma a riga di comando, né server, né accesso alla rete.

## ⚙️ Configurazione

La libreria non ha un file di configurazione: i suoi limiti sono costanti del codice, elencate qui perché si sappia cosa può contenere un pacchetto. La versione del formato fa parte del file, non è un'impostazione.

| Valore | Limite | Dove |
|---|---|---|
| Voci in un pacchetto | da 1 a 10.000 | `MAX_ENTRIES` in `src/container.rs` |
| Dimensione di una voce | fino a 256 MiB (268.435.456 byte) | `MAX_ENTRY_BYTES` |
| Dimensione di tutte le voci insieme | fino a 1 GiB | `MAX_TOTAL_BYTES` |
| Lunghezza del nome di una voce | da 1 a 240 byte | `check_entry_name` |
| Cartelle dati | solo `layers/`, `toolpaths/`, `vision/` | `DATA_FOLDERS` in `src/project.rs` |
| Versione del formato | 1.0.0 (la stessa versione maggiore viene letta, una più nuova rifiutata) | `FORMAT_VERSION` |
| Diametro dell'utensile | finito e maggiore di 0 | `ProjectMetadata::validate` |

La base del firmware ha un proprio documento di impostazioni (`node_settings.hpp`, `config_common.hpp`); i suoi valori appartengono a ogni nodo e sono descritti nel README di EH3D-NODE-BUNGARD.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.75 o successiva).

```rust
use eh3d_common::{Package, ProjectMetadata};

fn main() -> eh3d_common::Result<()> {
    let mut package = Package::new(ProjectMetadata::new("Test board"));
    package.add_entry("layers/top.gbr", b"G04 test*".to_vec())?;
    package.write_to_path(std::path::Path::new("board.eh3dproj"))?;
    let again = Package::read_from_path(std::path::Path::new("board.eh3dproj"))?;   // controlla ogni voce
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

## 🧪 Test

La suite ha 164 test (`cargo test --workspace`, eseguiti su Windows e Linux nella CI): 16 nei moduli (i metadati e la loro validazione, i codici di allarme e gli stati della macchina, i colori e il loro contrasto WCAG) e 14 sul contenitore (andata e ritorno in memoria e su file, stessi byte per lo stesso contenuto, il registro di integrità, una voce cambiata, una dimensione cambiata, una voce non elencata, una voce mancante, metadati modificati, un file che non è un pacchetto, nomi di voce non sicuri e ostili, una voce sostituita e un progetto non valido). La CI esegue anche `cargo fmt --check` e `cargo clippy` con gli avvisi come errori, `python -I tools/ci_validate.py` (manifesto, versioni, registro delle modifiche e gli otto README) e `tools/sync_firmware_base.py check` (le copie della base del firmware). I test su computer della base del firmware (`firmware_base/tests/`) girano dentro EH3D-NODE-BUNGARD, non qui. I nove crate condivisi ne aggiungono 134: gli archivi di lavoro (6), i file delle macchine Bungard, con archi, cerchi e scalatura HP-GL e i contorni Gerber e DXF (16), i controllori di macchina su una linea simulata (19) e la scena, le viste, i temi e le parole dell'interfaccia in otto lingue (20), le finestre Apri e Salva (7), le macchine e stazioni del server (5) e la mappa delle altezze del tavolo (8) il client del nodo della testa di foratura, con l'anello di LED e la foto (10) e l'analizzatore di qualità (43): i fori, i canali fresati, la qualità della foto, il profilo e il verdetto.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`, e ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). Le versioni vengono pubblicate su GitHub; la libreria si usa come crate Rust da questo repository e non è su crates.io.

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-COMMON" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-COMMON/
├── src/
│   ├── container.rs                             il pacchetto: lettura, scrittura, integrità e limiti
│   ├── project.rs                               i metadati e la loro validazione
│   ├── telemetry.rs                             assi, mandrino, stati e codici di allarme
│   └── theme.rs error.rs lib.rs                 i colori con contrasto, gli errori e la radice del crate
├── crates/                                      crate condivisi: vista, ZIP, file macchina, controllori, testa, qualità
├── firmware_base/
│   ├── core/                                    senza hardware: impostazioni, auth, piano di rete
│   ├── main/                                    codice ESP-IDF: rete, archivio, API del pannello
│   ├── tests/                                   i test su computer di tutto questo
│   ├── tools/                                   impacchetta il pannello; elenca le righe sdkconfig ignorate
│   └── manifest.json                            quale file va in quale nodo
├── tests/                                       il pacchetto: andata e ritorno, voci cambiate, mancanti e non elencate, nomi non sicuri
├── docs/                                        il formato, la base del firmware e i dati delle due immagini
├── tools/                                       controllo della CI e del README, sincronizzazione della base del firmware e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il pacchetto Rust e le sue dipendenze bloccate
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* Nessun programma usa ancora il contenitore di progetto, quindi il formato è stato letto e scritto solo dai suoi test.
* Il pacchetto non è firmato né cifrato e la libreria non ha avuto una revisione indipendente.
* Non c'è migrazione, perché 1.0.0 è il primo formato.
* La base del firmware è stata esercitata solo tramite EH3D-NODE-BUNGARD e non è mai stata eseguita su una scheda; un secondo nodo non esiste ancora.
* La libreria non è pubblicata su crates.io.

**Prossimi passi**

* Far aprire e salvare un progetto con essa al primo programma della suite (prima EH3D-CORE-LIBS).
* Aggiungere il secondo nodo alla base del firmware e verificare che le copie restino identiche.
* Decidere se i pacchetti richiedono una firma facoltativa.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): i parser (Excellon, Gerber) e la geometria che useranno questi tipi
* [EH3D-NODE-BUNGARD](https://github.com/JuanenRac/EH3D-NODE-BUNGARD): il primo nodo che tiene una copia della base del firmware
* [EH3D-SERVER](https://github.com/JuanenRac/EH3D-SERVER): il centro della famiglia; non usa ancora il contenitore di progetto
* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): il centro di documentazione della suite e la sua mappa dei progetti
* [HYDRA-UMC](https://github.com/JuanenRac/HYDRA-UMC): la piattaforma di robotica le cui macchine la suite dovrà pilotare in futuro

## 📚 Documentazione

* [Formato di progetto](docs/FORMAT.md): le voci, i metadati, il registro di integrità e le regole per i lettori.
* [La base del firmware](docs/FIRMWARE_BASE.md): cosa è condiviso, come si mantiene identico e come aggiungere un nodo.
* [Registro delle modifiche](CHANGELOG.md)

## 🤝 Contribuire e comunità

* Prima di una pull request esegui `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `python -I tools/ci_validate.py` e `python -I tools/sync_firmware_base.py check`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Una modifica alla base del firmware si fa in un nodo, si importa con `tools/sync_firmware_base.py import` e si sincronizza negli altri; non modificare mai una copia a mano.
* Segnala una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
