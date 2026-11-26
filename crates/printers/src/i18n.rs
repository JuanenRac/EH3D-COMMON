//! Every word of the crate that a program shows, in the eight languages of the suite (en, es, de,
//! fr, it, ja, zh and pt, the last one of Portugal). Names of makers, models and systems are not
//! translated. The translations have not been reviewed by native speakers.

use eh3d_printmaterials::Lang;

use crate::error::PrnCode;
use crate::model::{Adapter, Enclosure, Firmware, MulticolorStatus};
use crate::start::{Block, Caution, Level};

fn pick(lang: Lang, texts: [&'static str; 8]) -> &'static str {
    let i = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    texts[i]
}

impl PrnCode {
    /// The title of the code in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            PrnCode::Parse => [
                "Data file not valid",
                "Fichero de datos no válido",
                "Datendatei ungültig",
                "Fichier de données non valide",
                "File di dati non valido",
                "データファイルが無効です",
                "数据文件无效",
                "Ficheiro de dados inválido",
            ],
            PrnCode::SchemaVersion => [
                "Schema version not supported",
                "Versión de esquema no admitida",
                "Schemaversion nicht unterstützt",
                "Version de schéma non prise en charge",
                "Versione dello schema non supportata",
                "未対応のスキーマバージョンです",
                "不支持的架构版本",
                "Versão do esquema não suportada",
            ],
            PrnCode::DuplicateId => [
                "Duplicate identifier",
                "Identificador duplicado",
                "Doppelter Bezeichner",
                "Identifiant en double",
                "Identificatore duplicato",
                "識別子が重複しています",
                "标识符重复",
                "Identificador duplicado",
            ],
            PrnCode::IdMismatch => [
                "Identifier does not match the record",
                "El identificador no corresponde al registro",
                "Der Bezeichner passt nicht zum Eintrag",
                "L'identifiant ne correspond pas à l'enregistrement",
                "L'identificatore non corrisponde al record",
                "識別子がレコードと一致しません",
                "标识符与记录不符",
                "O identificador não corresponde ao registo",
            ],
            PrnCode::OutOfRange => [
                "Value out of range",
                "Valor fuera de rango",
                "Wert außerhalb des Bereichs",
                "Valeur hors plage",
                "Valore fuori intervallo",
                "値が範囲外です",
                "数值超出范围",
                "Valor fora do intervalo",
            ],
            PrnCode::Source => [
                "Source missing or not valid",
                "Falta la fuente o no es válida",
                "Quelle fehlt oder ist ungültig",
                "Source absente ou non valide",
                "Fonte mancante o non valida",
                "出典がないか無効です",
                "来源缺失或无效",
                "Fonte em falta ou inválida",
            ],
            PrnCode::Inconsistent => [
                "Values contradict each other",
                "Los valores se contradicen",
                "Werte widersprechen sich",
                "Les valeurs se contredisent",
                "I valori si contraddicono",
                "値が互いに矛盾しています",
                "数值相互矛盾",
                "Os valores contradizem-se",
            ],
            PrnCode::Text => [
                "Text not accepted",
                "Texto no admitido",
                "Text nicht zulässig",
                "Texte non accepté",
                "Testo non accettato",
                "受け付けられないテキストです",
                "文本不被接受",
                "Texto não aceite",
            ],
            PrnCode::TooLarge => [
                "Data file too large",
                "Fichero de datos demasiado grande",
                "Datendatei zu groß",
                "Fichier de données trop volumineux",
                "File di dati troppo grande",
                "データファイルが大きすぎます",
                "数据文件过大",
                "Ficheiro de dados demasiado grande",
            ],
        };
        pick(lang, texts)
    }
}

impl Enclosure {
    /// The words for the value in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            Enclosure::Open => [
                "Open frame",
                "Estructura abierta",
                "Offener Rahmen",
                "Châssis ouvert",
                "Telaio aperto",
                "オープンフレーム",
                "开放式框架",
                "Estrutura aberta",
            ],
            Enclosure::Enclosed => [
                "Enclosed",
                "Cerrada",
                "Geschlossen",
                "Fermée",
                "Chiusa",
                "密閉型",
                "封闭式",
                "Fechada",
            ],
            Enclosure::Unknown => NOT_PUBLISHED,
        };
        pick(lang, texts)
    }
}

const NOT_PUBLISHED: [&str; 8] = [
    "Not published",
    "No publicado",
    "Nicht angegeben",
    "Non publié",
    "Non pubblicato",
    "公表されていません",
    "未公布",
    "Não publicado",
];

impl Firmware {
    /// The words for the value in a language (the names of firmware are not translated).
    pub fn title_in(self, lang: Lang) -> &'static str {
        match self {
            Firmware::Bambu => "Bambu Lab",
            Firmware::Klipper => "Klipper",
            Firmware::Marlin => "Marlin",
            Firmware::Unknown => pick(lang, NOT_PUBLISHED),
        }
    }
}

impl Adapter {
    /// The words for the family of adapter in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            Adapter::BambuLan => [
                "Bambu Lab LAN mode",
                "Modo LAN de Bambu Lab",
                "LAN-Modus von Bambu Lab",
                "Mode LAN de Bambu Lab",
                "Modalità LAN di Bambu Lab",
                "Bambu Lab の LAN モード",
                "Bambu Lab 局域网模式",
                "Modo LAN da Bambu Lab",
            ],
            Adapter::KlipperMoonraker => [
                "Klipper through Moonraker",
                "Klipper mediante Moonraker",
                "Klipper über Moonraker",
                "Klipper via Moonraker",
                "Klipper tramite Moonraker",
                "Moonraker 経由の Klipper",
                "通过 Moonraker 的 Klipper",
                "Klipper através do Moonraker",
            ],
            Adapter::MarlinSerial => [
                "Marlin over a serial line",
                "Marlin por línea serie",
                "Marlin über eine serielle Leitung",
                "Marlin sur liaison série",
                "Marlin su linea seriale",
                "シリアル接続の Marlin",
                "串口连接的 Marlin",
                "Marlin por ligação série",
            ],
            Adapter::Octoprint => [
                "OctoPrint",
                "OctoPrint",
                "OctoPrint",
                "OctoPrint",
                "OctoPrint",
                "OctoPrint",
                "OctoPrint",
                "OctoPrint",
            ],
            Adapter::Prusalink => [
                "PrusaLink",
                "PrusaLink",
                "PrusaLink",
                "PrusaLink",
                "PrusaLink",
                "PrusaLink",
                "PrusaLink",
                "PrusaLink",
            ],
            Adapter::VendorProtocol => [
                "Protocol of the maker",
                "Protocolo del fabricante",
                "Protokoll des Herstellers",
                "Protocole du fabricant",
                "Protocollo del produttore",
                "メーカー独自のプロトコル",
                "厂商专有协议",
                "Protocolo do fabricante",
            ],
            Adapter::Unknown => NOT_PUBLISHED,
        };
        pick(lang, texts)
    }
}

impl MulticolorStatus {
    /// The words for the value in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            MulticolorStatus::Included => [
                "Included",
                "Incluido",
                "Enthalten",
                "Inclus",
                "Incluso",
                "付属",
                "已包含",
                "Incluído",
            ],
            MulticolorStatus::Optional => [
                "Optional, bought apart",
                "Opcional, se compra aparte",
                "Optional, separat erhältlich",
                "En option, vendu séparément",
                "Opzionale, venduto a parte",
                "オプション（別売り）",
                "选配，需另购",
                "Opcional, vendido à parte",
            ],
            MulticolorStatus::None => [
                "None", "No tiene", "Keine", "Aucun", "Nessuno", "なし", "无", "Nenhum",
            ],
            MulticolorStatus::Unknown => NOT_PUBLISHED,
        };
        pick(lang, texts)
    }
}

impl Block {
    /// The words for the reason in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            Block::NozzleTemperature => [
                "The nozzle does not get hot enough",
                "La boquilla no alcanza la temperatura necesaria",
                "Die Düse wird nicht heiß genug",
                "La buse ne chauffe pas assez",
                "L'ugello non raggiunge la temperatura necessaria",
                "ノズルの温度が足りません",
                "喷嘴温度不够高",
                "O bico não aquece o suficiente",
            ],
            Block::BedTemperature => [
                "The bed does not get hot enough",
                "La cama no alcanza la temperatura necesaria",
                "Das Druckbett wird nicht heiß genug",
                "Le plateau ne chauffe pas assez",
                "Il piatto non raggiunge la temperatura necessaria",
                "ベッドの温度が足りません",
                "热床温度不够高",
                "A mesa não aquece o suficiente",
            ],
            Block::NozzleTooSmall => [
                "The nozzle is too narrow for this filament",
                "La boquilla es demasiado estrecha para este filamento",
                "Die Düse ist für dieses Filament zu eng",
                "La buse est trop fine pour ce filament",
                "L'ugello è troppo piccolo per questo filamento",
                "このフィラメントにはノズル径が小さすぎます",
                "喷嘴过细，不适合此耗材",
                "O bico é demasiado estreito para este filamento",
            ],
            Block::Diameter => [
                "The filament does not come in the diameter of the printer",
                "El filamento no existe en el diámetro de la impresora",
                "Das Filament gibt es nicht im Durchmesser des Druckers",
                "Le filament n'existe pas au diamètre de l'imprimante",
                "Il filamento non esiste nel diametro della stampante",
                "プリンターに合う直径のフィラメントがありません",
                "此耗材没有打印机所用的直径",
                "O filamento não existe no diâmetro da impressora",
            ],
        };
        pick(lang, texts)
    }
}

impl Caution {
    /// The words for the caution in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            Caution::Enclosure => ["A closed printer is advised for this filament", "Se aconseja una impresora cerrada para este filamento", "Für dieses Filament wird ein geschlossener Drucker empfohlen", "Une imprimante fermée est conseillée pour ce filament", "Per questo filamento è consigliata una stampante chiusa", "このフィラメントには密閉型プリンターをお勧めします", "此耗材建议使用封闭式打印机", "Aconselha-se uma impressora fechada para este filamento"],
            Caution::EnclosureUnknown => ["A closed printer is advised and the maker does not say whether this one is", "Se aconseja una impresora cerrada y el fabricante no dice si esta lo es", "Ein geschlossener Drucker wird empfohlen; der Hersteller sagt nicht, ob dieser es ist", "Une imprimante fermée est conseillée et le fabricant ne dit pas si celle-ci l'est", "È consigliata una stampante chiusa e il produttore non dice se questa lo è", "密閉型が推奨されますが、この機種が密閉型かはメーカーが公表していません", "建议使用封闭式打印机，但厂商未说明本机是否封闭", "Aconselha-se uma impressora fechada e o fabricante não diz se esta o é"],
            Caution::HardenedNozzle => ["It wears a brass nozzle: use a hardened one", "Desgasta las boquillas de latón: usa una endurecida", "Verschleißt Messingdüsen: eine gehärtete verwenden", "Use les buses en laiton : prendre une buse durcie", "Consuma gli ugelli in ottone: usarne uno indurito", "真鍮ノズルを摩耗させます。硬化ノズルを使ってください", "会磨损黄铜喷嘴：请使用硬化喷嘴", "Desgasta os bicos de latão: use um bico endurecido"],
            Caution::NozzleLimit => ["The advised nozzle temperature is above the printer's limit", "La temperatura de boquilla aconsejada supera el límite de la impresora", "Die empfohlene Düsentemperatur liegt über der Grenze des Druckers", "La température de buse conseillée dépasse la limite de l'imprimante", "La temperatura dell'ugello consigliata supera il limite della stampante", "推奨ノズル温度がプリンターの上限を超えています", "建议的喷嘴温度超过打印机上限", "A temperatura de bico aconselhada excede o limite da impressora"],
            Caution::BedLimit => ["The advised bed temperature is above the printer's limit", "La temperatura de cama aconsejada supera el límite de la impresora", "Die empfohlene Betttemperatur liegt über der Grenze des Druckers", "La température de plateau conseillée dépasse la limite de l'imprimante", "La temperatura del piatto consigliata supera il limite della stampante", "推奨ベッド温度がプリンターの上限を超えています", "建议的热床温度超过打印机上限", "A temperatura de mesa aconselhada excede o limite da impressora"],
        };
        pick(lang, texts)
    }
}

impl Level {
    /// The name of the level in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            Level::Draft => [
                "Draft",
                "Borrador",
                "Entwurf",
                "Brouillon",
                "Bozza",
                "ドラフト",
                "草稿",
                "Rascunho",
            ],
            Level::Standard => [
                "Standard",
                "Estándar",
                "Standard",
                "Standard",
                "Standard",
                "標準",
                "标准",
                "Padrão",
            ],
            Level::Fine => [
                "Fine",
                "Fino",
                "Fein",
                "Fin",
                "Fine",
                "高精細",
                "精细",
                "Fino",
            ],
            Level::Strong => [
                "Strong",
                "Resistente",
                "Stabil",
                "Résistant",
                "Resistente",
                "高強度",
                "高强度",
                "Resistente",
            ],
        };
        pick(lang, texts)
    }
}

/// The words around the figures of a printer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Label {
    /// The build volume.
    Volume,
    /// The nozzle diameters.
    Nozzles,
    /// The diameter of the filament.
    Filament,
    /// The enclosure.
    Enclosure,
    /// A heated chamber.
    HeatedChamber,
    /// The highest nozzle temperature.
    NozzleMax,
    /// The highest bed temperature.
    BedMax,
    /// The multi-colour system.
    Multicolor,
    /// The firmware.
    Firmware,
    /// The family of adapter.
    Adapter,
    /// The files the printer takes.
    SendFormats,
    /// The pages of the maker.
    Sources,
    /// Yes.
    Yes,
    /// No.
    No,
    /// A figure the maker does not publish.
    NotPublished,
    /// The adapter was not documented by the maker.
    AdapterUnchecked,
    /// Nothing was tried on a machine.
    NotTried,
    /// The starting filaments.
    StartingFilaments,
    /// The starting processes.
    StartingProcesses,
}

impl Label {
    /// Every label.
    pub const ALL: [Label; 19] = [
        Label::Volume,
        Label::Nozzles,
        Label::Filament,
        Label::Enclosure,
        Label::HeatedChamber,
        Label::NozzleMax,
        Label::BedMax,
        Label::Multicolor,
        Label::Firmware,
        Label::Adapter,
        Label::SendFormats,
        Label::Sources,
        Label::Yes,
        Label::No,
        Label::NotPublished,
        Label::AdapterUnchecked,
        Label::NotTried,
        Label::StartingFilaments,
        Label::StartingProcesses,
    ];

    /// The label in a language.
    pub fn title_in(self, lang: Lang) -> &'static str {
        let texts = match self {
            Label::Volume => [
                "Build volume",
                "Volumen de impresión",
                "Bauraum",
                "Volume d'impression",
                "Volume di stampa",
                "造形サイズ",
                "打印体积",
                "Volume de impressão",
            ],
            Label::Nozzles => [
                "Nozzles",
                "Boquillas",
                "Düsen",
                "Buses",
                "Ugelli",
                "ノズル径",
                "喷嘴",
                "Bicos",
            ],
            Label::Filament => [
                "Filament diameter",
                "Diámetro del filamento",
                "Filamentdurchmesser",
                "Diamètre du filament",
                "Diametro del filamento",
                "フィラメント径",
                "耗材直径",
                "Diâmetro do filamento",
            ],
            Label::Enclosure => [
                "Enclosure",
                "Cerramiento",
                "Gehäuse",
                "Caisson",
                "Chiusura",
                "筐体",
                "外壳",
                "Estrutura",
            ],
            Label::HeatedChamber => [
                "Heated chamber",
                "Cámara calefactada",
                "Beheizte Kammer",
                "Chambre chauffée",
                "Camera riscaldata",
                "加熱チャンバー",
                "加热腔体",
                "Câmara aquecida",
            ],
            Label::NozzleMax => [
                "Highest nozzle temperature",
                "Temperatura máxima de boquilla",
                "Höchste Düsentemperatur",
                "Température maximale de buse",
                "Temperatura massima dell'ugello",
                "ノズル最高温度",
                "喷嘴最高温度",
                "Temperatura máxima do bico",
            ],
            Label::BedMax => [
                "Highest bed temperature",
                "Temperatura máxima de cama",
                "Höchste Betttemperatur",
                "Température maximale du plateau",
                "Temperatura massima del piatto",
                "ベッド最高温度",
                "热床最高温度",
                "Temperatura máxima da mesa",
            ],
            Label::Multicolor => [
                "Multicolour",
                "Multicolor",
                "Mehrfarbig",
                "Multicouleur",
                "Multicolore",
                "多色印刷",
                "多色",
                "Multicor",
            ],
            Label::Firmware => [
                "Firmware",
                "Firmware",
                "Firmware",
                "Micrologiciel",
                "Firmware",
                "ファームウェア",
                "固件",
                "Firmware",
            ],
            Label::Adapter => [
                "Connection family",
                "Familia de conexión",
                "Verbindungsfamilie",
                "Famille de connexion",
                "Famiglia di connessione",
                "接続方式",
                "连接类型",
                "Família de ligação",
            ],
            Label::SendFormats => [
                "Files it takes",
                "Ficheros que admite",
                "Akzeptierte Dateien",
                "Fichiers acceptés",
                "File accettati",
                "受け付けるファイル",
                "支持的文件",
                "Ficheiros aceites",
            ],
            Label::Sources => [
                "Sources", "Fuentes", "Quellen", "Sources", "Fonti", "出典", "来源", "Fontes",
            ],
            Label::Yes => ["Yes", "Sí", "Ja", "Oui", "Sì", "あり", "是", "Sim"],
            Label::No => ["No", "No", "Nein", "Non", "No", "なし", "否", "Não"],
            Label::NotPublished => NOT_PUBLISHED,
            Label::AdapterUnchecked => [
                "The maker does not document it: check it on the machine",
                "El fabricante no lo documenta: compruébalo en la máquina",
                "Vom Hersteller nicht dokumentiert: an der Maschine prüfen",
                "Non documenté par le fabricant : à vérifier sur la machine",
                "Non documentato dal produttore: verificare sulla macchina",
                "メーカーの文書がありません。実機で確認してください",
                "厂商未提供文档：请在机器上核实",
                "Não documentado pelo fabricante: confirme na máquina",
            ],
            Label::NotTried => [
                "Figures from the maker's pages, not tried on a machine",
                "Cifras de las páginas del fabricante, no probadas en una máquina",
                "Angaben von den Seiten des Herstellers, an keiner Maschine geprüft",
                "Chiffres des pages du fabricant, non essayés sur une machine",
                "Dati dalle pagine del produttore, non provati su una macchina",
                "メーカーのページの数値で、実機では未確認です",
                "数据来自厂商页面，未在机器上验证",
                "Valores das páginas do fabricante, não testados numa máquina",
            ],
            Label::StartingFilaments => [
                "Filaments to start from",
                "Filamentos de partida",
                "Filamente für den Anfang",
                "Filaments de départ",
                "Filamenti di partenza",
                "出発点となるフィラメント",
                "起步耗材",
                "Filamentos de partida",
            ],
            Label::StartingProcesses => [
                "Processes to start from",
                "Procesos de partida",
                "Prozesse für den Anfang",
                "Procédés de départ",
                "Processi di partenza",
                "出発点となるプロセス",
                "起步工艺",
                "Processos de partida",
            ],
        };
        pick(lang, texts)
    }
}
