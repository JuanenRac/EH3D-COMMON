//! The words of the launcher, the viewer and the themes, in the eight languages of the project.
//! Order: English, Spanish, Italian, German, French, Portuguese (of Portugal), Chinese, Japanese.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    En,
    Es,
    It,
    De,
    Fr,
    Pt,
    Zh,
    Ja,
    Bg,
    Hr,
    Cs,
    Da,
    Nl,
    Et,
    Fi,
    El,
    Hu,
    Ga,
    Lv,
    Lt,
    Mt,
    Pl,
    Ro,
    Sk,
    Sl,
    Sv,
    No,
    Is,
    Uk,
    Ru,
    Sr,
    Sq,
    Ca,
    Gl,
    Eu,
    PtBr,
    FrCa,
}

/// How many languages have their words inside the program (the first of [`Lang::ALL`]); the rest are read from the files of `locales/`.
pub const CORE_LANGS: usize = 8;

impl Lang {
    pub const ALL: [Lang; 37] = [
        Lang::En,
        Lang::Es,
        Lang::It,
        Lang::De,
        Lang::Fr,
        Lang::Pt,
        Lang::Zh,
        Lang::Ja,
        Lang::Bg,
        Lang::Hr,
        Lang::Cs,
        Lang::Da,
        Lang::Nl,
        Lang::Et,
        Lang::Fi,
        Lang::El,
        Lang::Hu,
        Lang::Ga,
        Lang::Lv,
        Lang::Lt,
        Lang::Mt,
        Lang::Pl,
        Lang::Ro,
        Lang::Sk,
        Lang::Sl,
        Lang::Sv,
        Lang::No,
        Lang::Is,
        Lang::Uk,
        Lang::Ru,
        Lang::Sr,
        Lang::Sq,
        Lang::Ca,
        Lang::Gl,
        Lang::Eu,
        Lang::PtBr,
        Lang::FrCa,
    ];

    /// The code of the configuration file and of the command line (`pt-br`, `fr-ca` for the variants).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::It => "it",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::Pt => "pt",
            Lang::Zh => "zh",
            Lang::Ja => "ja",
            Lang::Bg => "bg",
            Lang::Hr => "hr",
            Lang::Cs => "cs",
            Lang::Da => "da",
            Lang::Nl => "nl",
            Lang::Et => "et",
            Lang::Fi => "fi",
            Lang::El => "el",
            Lang::Hu => "hu",
            Lang::Ga => "ga",
            Lang::Lv => "lv",
            Lang::Lt => "lt",
            Lang::Mt => "mt",
            Lang::Pl => "pl",
            Lang::Ro => "ro",
            Lang::Sk => "sk",
            Lang::Sl => "sl",
            Lang::Sv => "sv",
            Lang::No => "no",
            Lang::Is => "is",
            Lang::Uk => "uk",
            Lang::Ru => "ru",
            Lang::Sr => "sr",
            Lang::Sq => "sq",
            Lang::Ca => "ca",
            Lang::Gl => "gl",
            Lang::Eu => "eu",
            Lang::PtBr => "pt-br",
            Lang::FrCa => "fr-ca",
        }
    }

    /// The name of the language in that language.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::It => "Italiano",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::Pt => "Português",
            Lang::Zh => "中文",
            Lang::Ja => "日本語",
            Lang::Bg => "Български",
            Lang::Hr => "Hrvatski",
            Lang::Cs => "Čeština",
            Lang::Da => "Dansk",
            Lang::Nl => "Nederlands",
            Lang::Et => "Eesti",
            Lang::Fi => "Suomi",
            Lang::El => "Ελληνικά",
            Lang::Hu => "Magyar",
            Lang::Ga => "Gaeilge",
            Lang::Lv => "Latviešu",
            Lang::Lt => "Lietuvių",
            Lang::Mt => "Malti",
            Lang::Pl => "Polski",
            Lang::Ro => "Română",
            Lang::Sk => "Slovenčina",
            Lang::Sl => "Slovenščina",
            Lang::Sv => "Svenska",
            Lang::No => "Norsk",
            Lang::Is => "Íslenska",
            Lang::Uk => "Українська",
            Lang::Ru => "Русский",
            Lang::Sr => "Српски",
            Lang::Sq => "Shqip",
            Lang::Ca => "Català",
            Lang::Gl => "Galego",
            Lang::Eu => "Euskara",
            Lang::PtBr => "Português (Brasil)",
            Lang::FrCa => "Français (Canada)",
        }
    }

    /// A variant falls back to its main language for the words it does not change.
    pub fn fallback(self) -> Option<Lang> {
        match self {
            Lang::PtBr => Some(Lang::Pt),
            Lang::FrCa => Some(Lang::Fr),
            _ => None,
        }
    }

    /// The language with that code (`es`, `ES`, `pt_BR`, `pt-br`); a region that is not a variant of the project gives its main language (`es-MX` is `es`).
    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.trim().to_ascii_lowercase().replace('_', "-");
        if let Some(l) = Lang::ALL.into_iter().find(|l| l.code() == c) {
            return Some(l);
        }
        let main = c.split('-').next().unwrap_or("");
        Lang::ALL.into_iter().find(|l| l.code() == main)
    }

    fn is_core(self) -> bool {
        Lang::ALL
            .iter()
            .position(|l| *l == self)
            .is_some_and(|i| i < CORE_LANGS)
    }
}

fn source(lang: Lang) -> &'static str {
    match lang {
        Lang::Bg => include_str!("../locales/bg.tsv"),
        Lang::Hr => include_str!("../locales/hr.tsv"),
        Lang::Cs => include_str!("../locales/cs.tsv"),
        Lang::Da => include_str!("../locales/da.tsv"),
        Lang::Nl => include_str!("../locales/nl.tsv"),
        Lang::Et => include_str!("../locales/et.tsv"),
        Lang::Fi => include_str!("../locales/fi.tsv"),
        Lang::El => include_str!("../locales/el.tsv"),
        Lang::Hu => include_str!("../locales/hu.tsv"),
        Lang::Ga => include_str!("../locales/ga.tsv"),
        Lang::Lv => include_str!("../locales/lv.tsv"),
        Lang::Lt => include_str!("../locales/lt.tsv"),
        Lang::Mt => include_str!("../locales/mt.tsv"),
        Lang::Pl => include_str!("../locales/pl.tsv"),
        Lang::Ro => include_str!("../locales/ro.tsv"),
        Lang::Sk => include_str!("../locales/sk.tsv"),
        Lang::Sl => include_str!("../locales/sl.tsv"),
        Lang::Sv => include_str!("../locales/sv.tsv"),
        Lang::No => include_str!("../locales/no.tsv"),
        Lang::Is => include_str!("../locales/is.tsv"),
        Lang::Uk => include_str!("../locales/uk.tsv"),
        Lang::Ru => include_str!("../locales/ru.tsv"),
        Lang::Sr => include_str!("../locales/sr.tsv"),
        Lang::Sq => include_str!("../locales/sq.tsv"),
        Lang::Ca => include_str!("../locales/ca.tsv"),
        Lang::Gl => include_str!("../locales/gl.tsv"),
        Lang::Eu => include_str!("../locales/eu.tsv"),
        Lang::PtBr => include_str!("../locales/pt-br.tsv"),
        Lang::FrCa => include_str!("../locales/fr-ca.tsv"),
        _ => "",
    }
}

/// The words of a language that is read from its file: `Name<TAB>text` per line, `#` for comments, `\n` for a line break.
fn extra(lang: Lang) -> &'static std::collections::HashMap<&'static str, &'static str> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static MAPS: OnceLock<Vec<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let maps = MAPS.get_or_init(|| {
        Lang::ALL
            .iter()
            .map(|l| {
                let mut m = HashMap::new();
                for line in source(*l).lines() {
                    let line = line.trim_end_matches('\r');
                    if line.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('\t') {
                        if !v.trim().is_empty() {
                            let v: &'static str =
                                Box::leak(v.replace("\\n", "\n").into_boxed_str());
                            m.insert(k.trim(), v);
                        }
                    }
                }
                m
            })
            .collect()
    });
    let i = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    &maps[i]
}

/// How many of the texts a language has of its own (not taken from English or from its main language).
pub fn coverage(lang: Lang) -> (usize, usize) {
    if lang.is_core() {
        return (ALL_TEXTS.len(), ALL_TEXTS.len());
    }
    let m = extra(lang);
    (
        ALL_TEXTS
            .iter()
            .filter(|t| m.contains_key(t.name()))
            .count(),
        ALL_TEXTS.len(),
    )
}

macro_rules! texts {
    ($($name:ident => [$($t:expr),+ $(,)?]),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Text { $($name),+ }
        pub const ALL_TEXTS: &[Text] = &[$(Text::$name),+];
        const TABLE: &[[&str; 8]] = &[$([$($t),+]),+];
        impl Text {
            fn index(self) -> usize { ALL_TEXTS.iter().position(|t| *t == self).unwrap_or(0) }
            /// The name of the text, the key of the files of the languages.
            pub fn name(self) -> &'static str { match self { $(Text::$name => stringify!($name)),+ } }
        }
    };
}

texts! {
    ThemeLabel => ["Theme", "Tema", "Tema", "Design", "Thème", "Tema", "主题", "テーマ"],
    ThemeForge => ["Forge", "Forja", "Fucina", "Schmiede", "Forge", "Forja", "锻造", "鍛冶場"],
    ThemeNeon => ["Neon Arcade", "Arcade neón", "Arcade neon", "Neon-Arcade", "Arcade néon", "Arcade néon", "霓虹街机", "ネオンアーケード"],
    ThemeGlass => ["Glass Pro", "Cristal Pro", "Vetro Pro", "Glas Pro", "Verre Pro", "Vidro Pro", "玻璃专业版", "ガラス・プロ"],
    ThemeWindowsClassic => ["Classic Windows", "Windows clásico", "Windows classico", "Windows klassisch", "Windows classique", "Windows clássico", "经典 Windows", "クラシック Windows"],
    ThemeSystem => ["Follow Windows colours", "Seguir los colores de Windows", "Segui i colori di Windows", "Windows-Farben übernehmen", "Suivre les couleurs de Windows", "Seguir as cores do Windows", "跟随 Windows 颜色", "Windows の色に合わせる"],
    ThemeMidnight => ["Midnight", "Medianoche", "Mezzanotte", "Mitternacht", "Minuit", "Meia-noite", "午夜", "ミッドナイト"],
    ThemePaper => ["Paper (light)", "Papel (claro)", "Carta (chiaro)", "Papier (hell)", "Papier (clair)", "Papel (claro)", "纸张（浅色）", "ペーパー（明るい）"],
    ThemeForest => ["Forest", "Bosque", "Foresta", "Wald", "Forêt", "Floresta", "森林", "フォレスト"],
    ThemeRetro => ["Retro terminal", "Terminal retro", "Terminale retrò", "Retro-Terminal", "Terminal rétro", "Terminal retro", "复古终端", "レトロ端末"],
    ThemeOcean => ["Ocean", "Océano", "Oceano", "Ozean", "Océan", "Oceano", "海洋", "オーシャン"],
    ThemeContrast => ["High contrast", "Alto contraste", "Alto contrasto", "Hoher Kontrast", "Contraste élevé", "Alto contraste", "高对比度", "ハイコントラスト"],
    LauncherAsk => ["What do you want to run?", "¿Qué quieres ejecutar?", "Cosa vuoi eseguire?", "Was möchten Sie starten?", "Que voulez-vous lancer ?", "O que quer executar?", "您想运行什么？", "何を起動しますか？"],
    LauncherClassicMtc => ["Classic MTC", "Clásico MTC", "Classico MTC", "Klassisch MTC", "Classique MTC", "Clássico MTC", "经典 MTC", "クラシック MTC"],
    LauncherClassicAtc => ["Classic ATC", "Clásico ATC", "Classico ATC", "Klassisch ATC", "Classique ATC", "Clássico ATC", "经典 ATC", "クラシック ATC"],
    LauncherModern => ["Modern (MTC + ATC)", "Moderno (MTC + ATC)", "Moderno (MTC + ATC)", "Modern (MTC + ATC)", "Moderne (MTC + ATC)", "Moderno (MTC + ATC)", "现代版（MTC + ATC）", "モダン（MTC + ATC）"],
    Remember => ["Remember my choice", "Recordar mi elección", "Ricorda la mia scelta", "Auswahl merken", "Mémoriser mon choix", "Lembrar a minha escolha", "记住我的选择", "選択を記憶する"],
    PressEnter => ["Press Enter", "Pulsa Intro", "Premi Invio", "Eingabetaste drücken", "Appuyez sur Entrée", "Prima Enter", "按回车键", "Enter キーを押す"],
    ViewPlan => ["2D - top view", "2D - vista desde arriba", "2D - vista dall'alto", "2D - Draufsicht", "2D - vue de dessus", "2D - vista de cima", "2D - 俯视图", "2D - 上面図"],
    ViewMachine => ["3D - machine", "3D - máquina", "3D - macchina", "3D - Maschine", "3D - machine", "3D - máquina", "3D - 机器", "3D - 機械"],
    LayerHoles => ["Drill holes", "Taladros", "Fori", "Bohrungen", "Perçages", "Furos", "钻孔", "ドリル穴"],
    LayerPaths => ["Route paths", "Trayectorias de fresado", "Percorsi di fresatura", "Fräsbahnen", "Trajets de fraisage", "Trajetos de fresagem", "铣削路径", "ルーター経路"],
    LayerMachine => ["Machine", "Máquina", "Macchina", "Maschine", "Machine", "Máquina", "机器", "機械"],
    LayerGrid => ["Grid", "Cuadrícula", "Griglia", "Raster", "Grille", "Grelha", "网格", "グリッド"],
    Progress => ["Progress", "Progreso", "Avanzamento", "Fortschritt", "Progression", "Progresso", "进度", "進捗"],
    TableFromConfig => ["Table from Config", "Mesa de Config", "Piano da Config", "Tisch aus der Konfiguration", "Table de la configuration", "Mesa da Configuração", "来自配置的工作台", "設定のテーブル"],
    Drilling => ["Drilling", "Taladrado", "Foratura", "Bohren", "Perçage", "Furação", "钻孔", "穴あけ"],
    Routing => ["Routing", "Fresado", "Fresatura", "Fräsen", "Fraisage", "Fresagem", "铣削", "ルーター加工"],
    ClassicMissing => ["The classic program is not next to this one", "El programa clásico no está junto a este", "Il programma classico non è accanto a questo", "Das klassische Programm liegt nicht neben diesem", "Le programme classique n'est pas à côté de celui-ci", "O programa clássico não está junto a este", "找不到与本程序放在一起的经典程序", "クラシック版がこのプログラムの隣にありません"],
    Start => ["Start", "Iniciar", "Avvia", "Start", "Démarrer", "Iniciar", "开始", "開始"],
    Pause => ["Pause", "Pausa", "Pausa", "Pause", "Pause", "Pausa", "暂停", "一時停止"],
    Stop => ["Stop", "Parar", "Ferma", "Stopp", "Arrêt", "Parar", "停止", "停止"],
    Language => ["Language", "Idioma", "Lingua", "Sprache", "Langue", "Idioma", "语言", "言語"],
    Holes => ["holes", "agujeros", "fori", "Bohrungen", "trous", "furos", "个孔", "穴"],
    Tools => ["tools", "herramientas", "utensili", "Werkzeuge", "outils", "ferramentas", "把刀具", "工具"],
    SimulatedJob => ["Simulated job: no machine is moved", "Trabajo simulado: no se mueve ninguna máquina", "Lavoro simulato: nessuna macchina si muove", "Simulierter Auftrag: keine Maschine bewegt sich", "Travail simulé : aucune machine ne bouge", "Trabalho simulado: nenhuma máquina se move", "模拟作业：不会移动任何机器", "シミュレーション：機械は動きません"],
    NoFile => ["No file loaded", "Ningún archivo cargado", "Nessun file caricato", "Keine Datei geladen", "Aucun fichier chargé", "Nenhum ficheiro carregado", "未载入文件", "ファイルが読み込まれていません"],
    Ok => ["Ok", "Aceptar", "Ok", "OK", "Ok", "Ok", "确定", "決定"],
    Abort => ["Abort", "Abortar", "Annulla", "Abbruch", "Annuler", "Abortar", "中止", "中止"],
    AskSpindle => ["OK to start the spindle?", "¿Arrancar el husillo?", "Avviare il mandrino?", "Spindel starten?", "Démarrer la broche ?", "Arrancar o fuso?", "确定启动主轴吗？", "主軸を始動しますか？"],
    AskTool => ["Insert tool", "Inserte la herramienta", "Inserire l'utensile", "Werkzeug einsetzen", "Insérez l'outil", "Insira a ferramenta", "请装入刀具", "工具を取り付けてください"],
    Connection => ["Connection", "Conexión", "Connessione", "Verbindung", "Connexion", "Ligação", "连接", "接続"],
    OutsideTable => ["Does not fit on the table", "No cabe en la mesa", "Non entra nel piano", "Passt nicht auf den Tisch", "Ne tient pas sur la table", "Não cabe na mesa", "超出工作台范围", "テーブルに収まりません"],
    OpenFile => ["Open file", "Abrir archivo", "Apri file", "Datei öffnen", "Ouvrir un fichier", "Abrir ficheiro", "打开文件", "ファイルを開く"],
    About => ["About", "Acerca de", "Informazioni", "Info", "À propos", "Sobre", "关于", "バージョン情報"],
    Close => ["Close", "Cerrar", "Chiudi", "Schließen", "Fermer", "Fechar", "关闭", "閉じる"],
    SettingsTitle => ["Settings", "Ajustes", "Impostazioni", "Einstellungen", "Réglages", "Definições", "设置", "設定"],
    PanelTitle => ["Panelization", "Panelización", "Panelizzazione", "Nutzen", "Panélisation", "Panelização", "拼板", "面付け"],
    Columns => ["Columns", "Columnas", "Colonne", "Spalten", "Colonnes", "Colunas", "列数", "列"],
    Rows => ["Rows", "Filas", "Righe", "Zeilen", "Lignes", "Linhas", "行数", "行"],
    SpacingX => ["Spacing X (mm)", "Separación X (mm)", "Distanza X (mm)", "Abstand X (mm)", "Écart X (mm)", "Espaçamento X (mm)", "X 间距 (mm)", "X 間隔 (mm)"],
    SpacingY => ["Spacing Y (mm)", "Separación Y (mm)", "Distanza Y (mm)", "Abstand Y (mm)", "Écart Y (mm)", "Espaçamento Y (mm)", "Y 间距 (mm)", "Y 間隔 (mm)"],
    PanelModePitch => ["Centre to centre", "De centro a centro", "Da centro a centro", "Mittenabstand", "Entre centres", "De centro a centro", "中心距", "中心間隔"],
    PanelModeGap => ["Gap between boards", "Hueco entre placas", "Spazio tra le schede", "Lücke zwischen Platinen", "Espace entre cartes", "Folga entre placas", "板间空隙", "基板の間隔"],
    PanelModeFit => ["Fill the table", "Llenar la mesa", "Riempi il piano", "Tisch füllen", "Remplir la table", "Preencher a mesa", "铺满工作台", "テーブルを埋める"],
    PanelCopies => ["copies", "copias", "copie", "Kopien", "copies", "cópias", "份", "枚"],
    Apply => ["Apply", "Aplicar", "Applica", "Anwenden", "Appliquer", "Aplicar", "应用", "適用"],
    ResetView => ["Reset view", "Restablecer vista", "Ripristina vista", "Ansicht zurücksetzen", "Réinitialiser la vue", "Repor vista", "重置视图", "表示をリセット"],
    DropHint => ["Drop a .ncd, .plt or .zip file here", "Suelta aquí un archivo .ncd, .plt o .zip", "Trascina qui un file .ncd, .plt o .zip", "Eine .ncd-, .plt- oder .zip-Datei hier ablegen", "Déposez ici un fichier .ncd, .plt ou .zip", "Largue aqui um ficheiro .ncd, .plt ou .zip", "将 .ncd、.plt 或 .zip 文件拖到此处", ".ncd・.plt・.zip ファイルをここにドロップ"],
    NavHint2d => ["Drag to move, wheel to zoom", "Arrastra para mover, rueda para zoom", "Trascina per spostare, rotella per zoom", "Ziehen zum Verschieben, Rad zum Zoomen", "Glisser pour déplacer, molette pour zoomer", "Arraste para mover, roda para ampliar", "拖动移动，滚轮缩放", "ドラッグで移動、ホイールで拡大縮小"],
    NavHint3d => ["Drag to turn, wheel to zoom", "Arrastra para girar, rueda para zoom", "Trascina per ruotare, rotella per zoom", "Ziehen zum Drehen, Rad zum Zoomen", "Glisser pour tourner, molette pour zoomer", "Arraste para rodar, roda para ampliar", "拖动旋转，滚轮缩放", "ドラッグで回転、ホイールで拡大縮小"],
    Version => ["Version", "Versión", "Versione", "Version", "Version", "Versão", "版本", "バージョン"],
    Licence => ["Licence", "Licencia", "Licenza", "Lizenz", "Licence", "Licença", "许可证", "ライセンス"],
    AboutBody => ["The modern interface for the Bungard CCD drilling and routing machines.", "La interfaz moderna para las máquinas de taladrar y fresar Bungard CCD.", "L'interfaccia moderna per le macchine di foratura e fresatura Bungard CCD.", "Die moderne Oberfläche für die Bohr- und Fräsmaschinen Bungard CCD.", "L'interface moderne des machines de perçage et de fraisage Bungard CCD.", "A interface moderna das máquinas de furar e fresar Bungard CCD.", "Bungard CCD 钻孔与铣削机的现代界面。", "Bungard CCD ドリル・ルーター機のためのモダンなインターフェース。"],
    TableWidth => ["Usable width X (mm)", "Ancho útil X (mm)", "Larghezza utile X (mm)", "Nutzbare Breite X (mm)", "Largeur utile X (mm)", "Largura útil X (mm)", "可用宽度 X (mm)", "使用可能幅 X (mm)"],
    TableDepth => ["Usable depth Y (mm)", "Fondo útil Y (mm)", "Profondità utile Y (mm)", "Nutzbare Tiefe Y (mm)", "Profondeur utile Y (mm)", "Profundidade útil Y (mm)", "可用深度 Y (mm)", "使用可能奥行 Y (mm)"],
    Save => ["Save", "Guardar", "Salva", "Speichern", "Enregistrer", "Guardar", "保存", "保存"],
    Saved => ["Saved", "Guardado", "Salvato", "Gespeichert", "Enregistré", "Guardado", "已保存", "保存しました"],
    OpenFolderUp => ["Parent folder", "Carpeta superior", "Cartella superiore", "Übergeordneter Ordner", "Dossier parent", "Pasta superior", "上级文件夹", "親フォルダー"],
    ShowDrill => ["Drilling", "Taladrado", "Foratura", "Bohren", "Perçage", "Furação", "钻孔", "穴あけ"],
    ConnPort => ["Port", "Puerto", "Porta", "Anschluss", "Port", "Porta", "端口", "ポート"],
    ConnAddress => ["Address", "Dirección", "Indirizzo", "Adresse", "Adresse", "Endereço", "地址", "アドレス"],
    ConnToken => ["Access token", "Token de acceso", "Token di accesso", "Zugriffstoken", "Jeton d'accès", "Token de acesso", "访问令牌", "アクセストークン"],
    ConnMachine => ["Machine id", "Id de máquina", "Id macchina", "Maschinen-ID", "Id de la machine", "Id da máquina", "机器编号", "マシンID"],
    ConnTest => ["Test connection", "Probar conexión", "Prova connessione", "Verbindung testen", "Tester la connexion", "Testar ligação", "测试连接", "接続をテスト"],
    ConnWorks => ["Connection works", "La conexión funciona", "La connessione funziona", "Verbindung funktioniert", "La connexion fonctionne", "A ligação funciona", "连接正常", "接続できます"],
    BoardTitle => ["Board", "Placa", "Piastra", "Platine", "Carte", "Placa", "电路板", "基板"],
    ToolsTitle => ["Tools", "Herramientas", "Utensili", "Werkzeuge", "Outils", "Ferramentas", "刀具", "工具"],
    SetAskOperator => ["Ask me at tool changes and when the spindle starts", "Preguntarme en los cambios de herramienta y al arrancar el husillo", "Chiedere ai cambi utensile e all'avvio del mandrino", "Bei Werkzeugwechsel und Spindelstart nachfragen", "Me demander aux changements d'outil et au démarrage de la broche", "Perguntar nas trocas de ferramenta e no arranque do fuso", "换刀和主轴启动时询问我", "工具交換と主軸始動のときに確認する"],
    SetToolsOnOpen => ["Show the tools window when I open a file", "Mostrar la ventana de herramientas al abrir un fichero", "Mostrare la finestra degli utensili quando apro un file", "Werkzeugfenster beim Öffnen einer Datei zeigen", "Afficher la fenêtre des outils à l'ouverture d'un fichier", "Mostrar a janela de ferramentas ao abrir um ficheiro", "打开文件时显示刀具窗口", "ファイルを開くときに工具ウィンドウを表示する"],
    PreviewShow => ["Preview", "Vista previa", "Anteprima", "Vorschau", "Aperçu", "Pré-visualização", "预览", "プレビュー"],
    PreviewNone => ["No preview", "Sin vista previa", "Nessuna anteprima", "Keine Vorschau", "Pas d'aperçu", "Sem pré-visualização", "无预览", "プレビューなし"],
    SelectTool => ["Select", "Seleccionar", "Seleziona", "Auswählen", "Sélectionner", "Selecionar", "选择", "選択"],
    CopySel => ["Copy", "Copiar", "Copia", "Kopieren", "Copier", "Copiar", "复制", "コピー"],
    PasteSel => ["Paste", "Pegar", "Incolla", "Einfügen", "Coller", "Colar", "粘贴", "貼り付け"],
    DeleteSel => ["Delete", "Eliminar", "Elimina", "Löschen", "Supprimer", "Eliminar", "删除", "削除"],
    RemoveTool => ["Remove the tool from the job", "Quitar la herramienta del trabajo", "Rimuovi l'utensile dal lavoro", "Werkzeug aus dem Auftrag entfernen", "Retirer l'outil du travail", "Retirar a ferramenta do trabalho", "从作业中移除刀具", "ジョブから工具を外す"],
    PortsNone => ["No serial ports found", "No se encuentran puertos serie", "Nessuna porta seriale trovata", "Keine serielle Schnittstelle gefunden", "Aucun port série trouvé", "Nenhuma porta série encontrada", "未找到串口", "シリアルポートが見つかりません"],
    PortsRefresh => ["Look for ports again", "Buscar puertos de nuevo", "Cerca di nuovo le porte", "Schnittstellen neu suchen", "Chercher à nouveau les ports", "Procurar portas de novo", "重新查找端口", "ポートを再検索"],
    UndoEdit => ["Undo", "Deshacer", "Annulla", "Rückgängig", "Annuler", "Anular", "撤销", "元に戻す"],
    RedoEdit => ["Redo", "Rehacer", "Ripeti", "Wiederholen", "Rétablir", "Refazer", "重做", "やり直し"],
    CutSel => ["Cut", "Cortar", "Taglia", "Ausschneiden", "Couper", "Cortar", "剪切", "切り取り"],
    StartAtTool => ["Start the job from this tool", "Empezar el trabajo por esta herramienta", "Inizia il lavoro da questo utensile", "Auftrag mit diesem Werkzeug beginnen", "Commencer le travail par cet outil", "Começar o trabalho por esta ferramenta", "从此刀具开始作业", "この工具からジョブを開始"],
    RackTitle => ["Tool rack", "Barra de herramientas", "Rack utensili", "Werkzeugleiste", "Barre d'outils", "Barra de ferramentas", "刀具架", "工具ラック"],
    RackTestTool => ["Slot 0: test tool in the spindle, the slot stays free", "Ranura 0: herramienta de prueba en el husillo, la ranura queda libre", "Posizione 0: utensile di prova nel mandrino, la posizione resta libera", "Platz 0: Testwerkzeug in der Spindel, der Platz bleibt frei", "Emplacement 0 : outil de test dans la broche, l'emplacement reste libre", "Ranhura 0: ferramenta de teste no fuso, a ranhura fica livre", "位置 0：主轴中装测试刀具，该位置保持空闲", "スロット0：スピンドルにテスト工具、スロットは空けておく"],
    RackReload => ["Put these tools in the slots and press Continue", "Pon estas herramientas en las ranuras y pulsa Continuar", "Metti questi utensili nelle posizioni e premi Continua", "Setzen Sie diese Werkzeuge in die Plätze und drücken Sie Weiter", "Placez ces outils dans les emplacements et appuyez sur Continuer", "Coloque estas ferramentas nas ranhuras e prima Continuar", "将这些刀具放入刀槽后点击继续", "これらの工具をスロットに入れて「続行」を押してください"],
    RackGroup => ["Load", "Carga", "Carico", "Bestückung", "Chargement", "Carga", "装载", "装填"],
    RackTips => ["Tools 38 mm long, 21 mm from the top of the ring to the tip; air 6-8 bar; depth limiter with its opening towards the slots.", "Herramientas de 38 mm, 21 mm del anillo a la punta; aire 6-8 bar; limitador de profundidad con la abertura hacia las ranuras.", "Utensili da 38 mm, 21 mm dall'anello alla punta; aria 6-8 bar; limitatore di profondità con l'apertura verso le posizioni.", "Werkzeuge 38 mm lang, 21 mm vom Ring bis zur Spitze; Druckluft 6-8 bar; Tiefenbegrenzer mit der Öffnung zu den Plätzen.", "Outils de 38 mm, 21 mm de l'anneau à la pointe ; air 6-8 bar ; limiteur de profondeur avec l'ouverture vers les emplacements.", "Ferramentas de 38 mm, 21 mm do anel à ponta; ar 6-8 bar; limitador de profundidade com a abertura virada para as ranhuras.", "刀具长 38 毫米，从环顶到刀尖 21 毫米；气压 6-8 bar；限深器开口朝向刀槽。", "工具は長さ38 mm、リング上端から先端まで21 mm。空気圧6〜8 bar。深さリミッターの開口はスロット側に向けます。"],
    SndVoice => ["Voice", "Voz", "Voce", "Stimme", "Voix", "Voz", "语音", "音声"],
    SndFemale => ["female", "mujer", "donna", "weiblich", "femme", "mulher", "女声", "女性"],
    SndMale => ["male", "hombre", "uomo", "männlich", "homme", "homem", "男声", "男性"],
    HeadOpen => ["Head node", "Nodo de la cabeza", "Nodo della testa", "Kopfknoten", "Nœud de la tête", "Nó da cabeça", "钻头节点", "ヘッドノード"],
    HeadTitle => ["Node of the drill head", "Nodo del cabezal de taladro", "Nodo della testa di foratura", "Knoten am Bohrkopf", "Nœud de la tête de perçage", "Nó da cabeça de furação", "钻头上的节点", "ドリルヘッドのノード"],
    HeadOff => ["Not used", "Sin usar", "Non usato", "Nicht benutzt", "Non utilisé", "Sem usar", "不使用", "使用しない"],
    HeadDirect => ["Directly", "Directo", "Diretto", "Direkt", "Direct", "Direto", "直连", "直接"],
    HeadServer => ["Through the server", "Por el servidor", "Tramite il server", "Über den Server", "Par le serveur", "Pelo servidor", "经服务器", "サーバー経由"],
    HeadCamera => ["Show the camera of the node", "Mostrar la cámara del nodo", "Mostra la telecamera del nodo", "Kamera des Knotens zeigen", "Afficher la caméra du nœud", "Mostrar a câmara do nó", "显示节点的摄像头", "ノードのカメラを表示する"],
    HeadSpeed => ["Spindle", "Husillo", "Mandrino", "Spindel", "Broche", "Fuso", "主轴", "スピンドル"],
    HeadTemp => ["Temperature", "Temperatura", "Temperatura", "Temperatur", "Température", "Temperatura", "温度", "温度"],
    HeadProtectOpen => ["Protect output OPEN: the spindle must stop", "Salida de protección ABIERTA: el husillo debe parar", "Uscita di protezione APERTA: il mandrino deve fermarsi", "Schutzausgang OFFEN: die Spindel muss stoppen", "Sortie de protection OUVERTE : la broche doit s'arrêter", "Saída de proteção ABERTA: o fuso deve parar", "保护输出已断开：主轴必须停止", "保護出力が開いています：スピンドルを止める必要があります"],
    HeadProtectClosed => ["Protect output closed", "Salida de protección cerrada", "Uscita di protezione chiusa", "Schutzausgang geschlossen", "Sortie de protection fermée", "Saída de proteção fechada", "保护输出已闭合", "保護出力は閉じています"],
    HeadMeasure => ["Measure the table here", "Medir la mesa aquí", "Misura il tavolo qui", "Tisch hier messen", "Mesurer la table ici", "Medir a mesa aqui", "在此处测量工作台", "ここでテーブルを測定"],
    HeadServerNote => ["Through the server of the connection window: its address, its token and the id of the machine.", "Por el servidor de la ventana de conexión: su dirección, su token y el id de la máquina.", "Tramite il server della finestra di connessione: il suo indirizzo, il suo token e l'id della macchina.", "Über den Server des Verbindungsfensters: seine Adresse, sein Token und die ID der Maschine.", "Par le serveur de la fenêtre de connexion : son adresse, son jeton et l'id de la machine.", "Pelo servidor da janela de ligação: o seu endereço, o seu token e o id da máquina.", "经连接窗口中的服务器：其地址、令牌和机器编号。", "接続ウィンドウのサーバー経由：そのアドレス、トークン、マシン ID。"],
    HeadNoServer => ["Write the address, the token and the machine id of the server in the connection window.", "Escribe la dirección, el token y el id de máquina del servidor en la ventana de conexión.", "Scrivi l'indirizzo, il token e l'id macchina del server nella finestra di connessione.", "Trage Adresse, Token und Maschinen-ID des Servers im Verbindungsfenster ein.", "Écrivez l'adresse, le jeton et l'id de machine du serveur dans la fenêtre de connexion.", "Escreva o endereço, o token e o id da máquina do servidor na janela de ligação.", "请在连接窗口中填写服务器的地址、令牌和机器编号。", "接続ウィンドウにサーバーのアドレス、トークン、マシン ID を入力してください。"],
    HeadCameraModel => ["Camera model", "Modelo de cámara", "Modello di telecamera", "Kameramodell", "Modèle de caméra", "Modelo de câmara", "摄像头型号", "カメラの型式"],
    HeadNoLink => ["Write the address of the node (host or host:port).", "Escribe la dirección del nodo (host o host:puerto).", "Scrivi l'indirizzo del nodo (host o host:porta).", "Schreibe die Adresse des Knotens (Host oder Host:Port).", "Écrivez l'adresse du nœud (hôte ou hôte:port).", "Escreva o endereço do nó (host ou host:porta).", "请填写节点地址（主机或 主机:端口）。", "ノードのアドレス（ホスト または ホスト:ポート）を入力してください。"],
    HeadWaiting => ["Waiting for the node...", "Esperando al nodo...", "In attesa del nodo...", "Warte auf den Knoten...", "En attente du nœud...", "À espera do nó...", "正在等待节点...", "ノードを待っています..."],
    HeadCameraOff => ["Camera off", "Cámara apagada", "Telecamera spenta", "Kamera aus", "Caméra éteinte", "Câmara desligada", "摄像头已关闭", "カメラ オフ"],
    HeatOk => ["ok", "bien", "ok", "ok", "ok", "bem", "正常", "正常"],
    HeatWarm => ["warm", "tibio", "caldo", "warm", "chaud", "morno", "偏热", "温かい"],
    HeatHot => ["HOT", "CALIENTE", "TROPPO CALDO", "HEISS", "TROP CHAUD", "QUENTE", "过热", "高温"],
    HeatUnknown => ["unknown", "desconocida", "sconosciuta", "unbekannt", "inconnue", "desconhecida", "未知", "不明"],
    LightTitle => ["Light ring", "Aro de luz", "Anello luminoso", "Lichtring", "Anneau lumineux", "Anel de luz", "环形灯", "リングライト"],
    LightOn => ["Light on", "Luz encendida", "Luce accesa", "Licht an", "Lumière allumée", "Luz ligada", "灯已开", "点灯中"],
    LightOff => ["Light off", "Luz apagada", "Luce spenta", "Licht aus", "Lumière éteinte", "Luz desligada", "灯已关", "消灯中"],
    LightNone => ["no ring on this node", "este nodo no tiene aro", "questo nodo non ha l'anello", "dieser Knoten hat keinen Ring", "ce nœud n'a pas d'anneau", "este nó não tem anel", "此节点没有环形灯", "このノードにはリングがありません"],
    InspOpen => ["Quality check", "Verificación de calidad", "Controllo qualità", "Qualitätsprüfung", "Contrôle qualité", "Verificação de qualidade", "质量检查", "品質チェック"],
    InspTitle => ["Quality check of the work", "Verificación de calidad del trabajo", "Controllo qualità del lavoro", "Qualitätsprüfung der Arbeit", "Contrôle qualité du travail", "Verificação de qualidade do trabalho", "加工质量检查", "加工の品質チェック"],
    InspRun => ["Check now", "Verificar ahora", "Controlla ora", "Jetzt prüfen", "Contrôler maintenant", "Verificar agora", "立即检查", "今すぐチェック"],
    InspRunning => ["Checking...", "Verificando...", "Controllo in corso...", "Prüfe...", "Contrôle en cours...", "A verificar...", "正在检查...", "チェック中..."],
    InspVerdictOk => ["OK", "CORRECTO", "CONFORME", "IN ORDNUNG", "VALIDE", "APROVADO", "合格", "合格OK"],
    InspVerdictNotOk => ["NOT OK", "NO CORRECTO", "NON CONFORME", "NICHT IN ORDNUNG", "NON VALIDE", "REPROVADO", "不合格", "不合格NG"],
    InspVerdictUnknown => ["CANNOT JUDGE", "NO SE PUEDE JUZGAR", "NON VALUTABILE", "NICHT BEURTEILBAR", "NON ÉVALUABLE", "NÃO AVALIÁVEL", "无法判定", "判定不可"],
    InspIdle => ["Not checked yet", "Aún sin verificar", "Non ancora controllato", "Noch nicht geprüft", "Pas encore contrôlé", "Ainda sem verificar", "尚未检查", "未チェック"],
    InspChecks => ["Checks", "Comprobaciones", "Controlli", "Prüfungen", "Contrôles", "Verificações", "检查项目", "チェック項目"],
    InspCheckHoles => ["Drilled holes", "Taladros", "Fori", "Bohrungen", "Perçages", "Furos", "钻孔", "穴あけ"],
    InspCheckMilling => ["Milled channels", "Canales fresados", "Canali fresati", "Gefräste Kanäle", "Canaux fraisés", "Canais fresados", "铣削沟槽", "フライス溝"],
    InspProfile => ["Analyser profile", "Perfil del analizador", "Profilo dell'analizzatore", "Analyseprofil", "Profil de l'analyseur", "Perfil do analisador", "分析器配置", "解析プロファイル"],
    InspPresetStandard => ["Standard", "Estándar", "Standard", "Standard", "Standard", "Padrão", "标准", "標準"],
    InspPresetFine => ["Fine pitch", "Paso fino", "Passo fine", "Feinraster", "Pas fin", "Passo fino", "细间距", "ファインピッチ"],
    InspPresetProto => ["Prototype", "Prototipo", "Prototipo", "Prototyp", "Prototype", "Protótipo", "原型", "試作"],
    InspLight => ["Light for the picture", "Luz para la foto", "Luce per la foto", "Licht für das Bild", "Lumière pour la photo", "Luz para a foto", "拍照用灯光", "撮影用の照明"],
    InspLightKeep => ["as it is", "como está", "com'è", "unverändert", "telle quelle", "como está", "保持现状", "そのまま"],
    InspHolePick => ["Hole of the job in the centre", "Agujero del trabajo en el centro", "Foro del lavoro al centro", "Bohrung des Auftrags in der Mitte", "Trou du travail au centre", "Furo do trabalho ao centro", "位于中心的作业孔", "中央にあるジョブの穴"],
    InspNoJob => ["Load a drill file to check its holes", "Carga un fichero de taladros para verificarlos", "Carica un file di foratura per controllarli", "Bohrdatei laden, um die Bohrungen zu prüfen", "Chargez un fichier de perçage pour les contrôler", "Carrega um ficheiro de furação para os verificar", "加载钻孔文件以检查孔", "穴を確認するにはドリルファイルを読み込んでください"],
    InspScale => ["Scale (pixels per mm)", "Escala (píxeles por mm)", "Scala (pixel per mm)", "Maßstab (Pixel pro mm)", "Échelle (pixels par mm)", "Escala (píxeis por mm)", "比例（像素/毫米）", "スケール（ピクセル/mm）"],
    InspCalibrate => ["Calibrate from this hole", "Calibrar con este agujero", "Calibra con questo foro", "Mit diesem Loch kalibrieren", "Étalonner avec ce trou", "Calibrar com este furo", "用此孔校准", "この穴で校正"],
    InspSave => ["Save profile", "Guardar perfil", "Salva profilo", "Profil speichern", "Enregistrer le profil", "Guardar perfil", "保存配置", "プロファイルを保存"],
    InspTune => ["Tolerances", "Tolerancias", "Tolleranze", "Toleranzen", "Tolérances", "Tolerâncias", "公差", "許容差"],
    InspNeedLink => ["Set the node of the drill head first (Head node window).", "Configura antes el nodo del cabezal (ventana Nodo de la cabeza).", "Imposta prima il nodo della testa (finestra Nodo della testa).", "Zuerst den Knoten des Bohrkopfs einrichten (Fenster Kopfknoten).", "Réglez d'abord le nœud de la tête (fenêtre Nœud de la tête).", "Configura primeiro o nó da cabeça (janela Nó da cabeça).", "请先设置钻头节点（钻头节点窗口）。", "先にヘッドノードを設定してください（ヘッドノード画面）。"],
    InspHolesOk => ["holes ok", "agujeros bien", "fori ok", "Bohrungen in Ordnung", "trous conformes", "furos bem", "个孔合格", "穴 合格"],
    InspChannel => ["channel width", "ancho de canal", "larghezza canale", "Kanalbreite", "largeur de canal", "largura do canal", "沟槽宽度", "溝の幅"],
    InspTuneHoleDia => ["Hole size tolerance (mm)", "Tolerancia del tamaño del agujero (mm)", "Tolleranza dimensione foro (mm)", "Toleranz der Lochgröße (mm)", "Tolérance de taille du trou (mm)", "Tolerância do tamanho do furo (mm)", "孔径公差（毫米）", "穴径の許容差 (mm)"],
    InspTuneHolePos => ["Hole position tolerance (mm)", "Tolerancia de posición (mm)", "Tolleranza di posizione (mm)", "Lagetoleranz (mm)", "Tolérance de position (mm)", "Tolerância de posição (mm)", "孔位公差（毫米）", "位置の許容差 (mm)"],
    InspTuneRound => ["Least roundness (1 = circle)", "Redondez mínima (1 = círculo)", "Rotondità minima (1 = cerchio)", "Mindestrundheit (1 = Kreis)", "Rondeur minimale (1 = cercle)", "Redondeza mínima (1 = círculo)", "最小圆度（1 = 圆）", "最小真円度 (1 = 円)"],
    InspTuneDebris => ["Most debris around a hole", "Máx. rebaba alrededor del agujero", "Massimo residuo attorno al foro", "Max. Grat um die Bohrung", "Bavure max. autour du trou", "Máx. rebarba à volta do furo", "孔周围最大毛刺", "穴周りのバリ上限"],
    InspTuneMillWidth => ["Channel width (mm)", "Ancho del canal (mm)", "Larghezza del canale (mm)", "Kanalbreite (mm)", "Largeur du canal (mm)", "Largura do canal (mm)", "沟槽宽度（毫米）", "溝の幅 (mm)"],
    InspTuneMillTol => ["Channel width tolerance", "Tolerancia del ancho", "Tolleranza della larghezza", "Toleranz der Breite", "Tolérance de largeur", "Tolerância da largura", "宽度公差", "幅の許容差"],
    InspTuneMillMin => ["Narrowest channel (mm)", "Canal más estrecho (mm)", "Canale più stretto (mm)", "Engster Kanal (mm)", "Canal le plus étroit (mm)", "Canal mais estreito (mm)", "最窄沟槽（毫米）", "最小の溝幅 (mm)"],
    InspTuneSpread => ["Most uneven channel", "Canal más irregular", "Canale più irregolare", "Ungleichmäßigster Kanal", "Canal le plus irrégulier", "Canal mais irregular", "最不均匀沟槽", "最も不均一な溝"],
    InspTuneMinMean => ["Darkest picture accepted", "Foto más oscura aceptada", "Foto più scura accettata", "Dunkelstes akzeptiertes Bild", "Photo la plus sombre acceptée", "Foto mais escura aceite", "可接受的最暗图像", "許容する最も暗い画像"],
    InspTuneMinSharp => ["Least sharpness", "Nitidez mínima", "Nitidezza minima", "Mindestschärfe", "Netteté minimale", "Nitidez mínima", "最低清晰度", "最小シャープネス"],
    InspFindHoleMissing => ["no hole there", "no hay agujero", "nessun foro", "keine Bohrung", "pas de trou", "não há furo", "此处无孔", "穴がありません"],
    InspFindHoleSize => ["wrong size", "tamaño incorrecto", "dimensione errata", "falsche Größe", "mauvaise taille", "tamanho errado", "孔径不符", "サイズ不良"],
    InspFindHoleOff => ["off centre", "descentrado", "fuori centro", "außermittig", "décentré", "descentrado", "偏离中心", "中心ずれ"],
    InspFindHoleRound => ["not round", "no es redondo", "non rotondo", "nicht rund", "pas rond", "não é redondo", "不圆", "真円でない"],
    InspFindHoleDebris => ["burrs or dust around", "rebabas o polvo alrededor", "bave o polvere attorno", "Grat oder Staub rundherum", "bavures ou poussière autour", "rebarbas ou pó à volta", "周围有毛刺或灰尘", "周囲にバリや粉塵"],
    InspFindHoleView => ["out of view", "fuera de la imagen", "fuori immagine", "außerhalb des Bildes", "hors de l'image", "fora da imagem", "不在画面内", "画像の外"],
    InspFindMillNone => ["no channels seen", "no se ven canales", "nessun canale visibile", "keine Kanäle sichtbar", "aucun canal visible", "não se veem canais", "未看到沟槽", "溝が見えません"],
    InspFindMillNarrow => ["channels too narrow", "canales demasiado estrechos", "canali troppo stretti", "Kanäle zu schmal", "canaux trop étroits", "canais demasiado estreitos", "沟槽过窄", "溝が狭すぎます"],
    InspFindMillWide => ["channels too wide", "canales demasiado anchos", "canali troppo larghi", "Kanäle zu breit", "canaux trop larges", "canais demasiado largos", "沟槽过宽", "溝が広すぎます"],
    InspFindMillPinch => ["a channel pinches: risk of a short", "un canal se estrecha: riesgo de cortocircuito", "un canale si restringe: rischio di corto", "ein Kanal verengt sich: Kurzschlussgefahr", "un canal se pince : risque de court-circuit", "um canal estreita: risco de curto-circuito", "沟槽变窄：有短路风险", "溝がくびれています：短絡の恐れ"],
    InspFindMillUneven => ["channels uneven", "canales irregulares", "canali irregolari", "Kanäle ungleichmäßig", "canaux irréguliers", "canais irregulares", "沟槽不均匀", "溝が不均一です"],
    InspFindDark => ["picture too dark: more light", "foto demasiado oscura: más luz", "foto troppo scura: più luce", "Bild zu dunkel: mehr Licht", "photo trop sombre : plus de lumière", "foto demasiado escura: mais luz", "图像过暗：需要更多光", "画像が暗すぎます：照明を上げてください"],
    InspFindBright => ["picture too bright: less light", "foto demasiado clara: menos luz", "foto troppo chiara: meno luce", "Bild zu hell: weniger Licht", "photo trop claire : moins de lumière", "foto demasiado clara: menos luz", "图像过亮：需要更少光", "画像が明るすぎます：照明を下げてください"],
    InspFindContrast => ["picture without contrast", "foto sin contraste", "foto senza contrasto", "Bild ohne Kontrast", "photo sans contraste", "foto sem contraste", "图像缺乏对比度", "コントラストがありません"],
    InspFindBlurry => ["picture out of focus", "foto desenfocada", "foto sfocata", "Bild unscharf", "photo floue", "foto desfocada", "图像未对焦", "ピントが合っていません"],
    InspFindScale => ["set the scale first", "fija antes la escala", "imposta prima la scala", "zuerst den Maßstab festlegen", "réglez d'abord l'échelle", "define primeiro a escala", "请先设置比例", "先にスケールを設定してください"],
    InspFindNothing => ["no check is switched on", "no hay ninguna comprobación activa", "nessun controllo attivo", "keine Prüfung aktiv", "aucun contrôle activé", "nenhuma verificação ativa", "未启用任何检查", "有効なチェックがありません"],
    InspFindNoHoles => ["no hole of the job in view", "ningún agujero del trabajo a la vista", "nessun foro del lavoro in vista", "keine Bohrung des Auftrags im Bild", "aucun trou du travail en vue", "nenhum furo do trabalho à vista", "画面中没有作业孔", "視野にジョブの穴がありません"],
    CatOpen => ["Catalogue of bits", "Catálogo de brocas y fresas", "Catalogo di punte e frese", "Katalog der Bohrer und Fräser", "Catalogue de forets et de fraises", "Catálogo de brocas e fresas", "钻头和铣刀目录", "ドリルとエンドミルのカタログ"],
    CatTitle => ["Catalogue of bits for PCB", "Catálogo de brocas y fresas para PCB", "Catalogo di punte e frese per PCB", "Katalog der Bohrer und Fräser für Leiterplatten", "Catalogue de forets et de fraises pour circuits imprimés", "Catálogo de brocas e fresas para PCB", "PCB 钻头和铣刀目录", "基板用ドリルとエンドミルのカタログ"],
    CatShank => ["Every bit has the 1/8 inch (3.175 mm) shank that the chuck of a Bungard machine holds. The numbers are starting points for FR-4: try them on a scrap board.", "Todas las herramientas tienen el mango de 1/8 de pulgada (3,175 mm) que sujeta el portabrocas de una Bungard. Los números son puntos de partida para FR-4: pruébalos en una placa de desecho.", "Tutti gli utensili hanno il gambo da 1/8 di pollice (3,175 mm) che il mandrino di una Bungard trattiene. I numeri sono punti di partenza per FR-4: provali su una scheda di scarto.", "Alle Werkzeuge haben den 1/8-Zoll-Schaft (3,175 mm), den das Spannfutter einer Bungard hält. Die Zahlen sind Startwerte für FR-4: erst an einer Restplatine probieren.", "Tous les outils ont la queue de 1/8 de pouce (3,175 mm) que le mandrin d'une Bungard serre. Les chiffres sont des points de départ pour le FR-4 : essayez-les sur une carte de rebut.", "Todas as ferramentas têm a haste de 1/8 de polegada (3,175 mm) que o mandril de uma Bungard segura. Os números são pontos de partida para FR-4: experimente numa placa de desperdício.", "所有刀具都是 Bungard 夹头可夹持的 1/8 英寸（3.175 毫米）刀柄。数值是 FR-4 的起点：请先在废板上试切。", "すべての工具は Bungard のチャックが保持する 1/8 インチ（3.175 mm）のシャンクです。数値は FR-4 向けの出発点です。まず捨て基板で試してください。"],
    CatAll => ["All kinds", "Todos los tipos", "Tutti i tipi", "Alle Arten", "Tous les types", "Todos os tipos", "全部类型", "すべての種類"],
    KindDrill => ["Drills", "Brocas", "Punte", "Bohrer", "Forets", "Brocas", "钻头", "ドリル"],
    KindEndMill => ["End mills (square)", "Fresas rectas", "Frese cilindriche", "Schaftfräser (gerade)", "Fraises droites", "Fresas retas", "平头铣刀", "エンドミル（スクエア）"],
    KindDownCut => ["Down-cut routers", "Fresas de corte descendente", "Frese a taglio discendente", "Fräser mit Druckschnitt (abwärts)", "Fraises à coupe descendante", "Fresas de corte descendente", "下切铣刀", "ダウンカットルーター"],
    KindUpCut => ["Up-cut routers", "Fresas de corte ascendente", "Frese a taglio ascendente", "Fräser mit Zugschnitt (aufwärts)", "Fraises à coupe montante", "Fresas de corte ascendente", "上切铣刀", "アップカットルーター"],
    KindDiamond => ["Diamond-cut routers (corn)", "Fresas de diamante (corn)", "Frese a diamante (corn)", "Diamant-Fräser (Corn)", "Fraises à denture diamant (corn)", "Fresas de diamante (corn)", "菱形纹铣刀（玉米刀）", "ダイヤモンドカット（コーン）ルーター"],
    KindVBit => ["V-bits (engraving, isolation)", "Fresas en V (grabado, aislamiento)", "Frese a V (incisione, isolamento)", "V-Fräser (Gravur, Isolation)", "Fraises en V (gravure, isolation)", "Fresas em V (gravação, isolamento)", "V 型刀（雕刻、隔离）", "Vビット（彫刻・アイソレーション）"],
    KindBall => ["Ball nose", "Fresas de punta esférica", "Frese a punta sferica", "Kugelfräser", "Fraises à bout sphérique", "Fresas de ponta esférica", "球头铣刀", "ボールエンドミル"],
    KindSpot => ["Spot drills", "Brocas de centrar", "Punte da centro", "Anbohrer", "Forets à pointer", "Brocas de centrar", "定位钻", "センタードリル"],
    KindCountersink => ["Countersinks", "Avellanadores", "Svasatori", "Senker", "Fraises à chanfreiner", "Escareadores", "倒角钻", "面取りカッター"],
    CatCount => ["bits", "herramientas", "utensili", "Werkzeuge", "outils", "ferramentas", "把刀具", "本の工具"],
    KindChipBreaker => ["Chip-breaker routers", "Fresas rompevirutas", "Frese rompitruciolo", "Spanbrecher-Fräser", "Fraises brise-copeaux", "Fresas quebra-aparas", "断屑铣刀", "チップブレーカールーター"],
    KindSlotDrill => ["Slot drills (for oblong holes)", "Brocas para ranuras (agujeros alargados)", "Punte per asole (fori oblunghi)", "Langlochbohrer (Nippelbohren)", "Forets pour trous oblongs", "Brocas para ranhuras (furos oblongos)", "长孔钻头", "長穴用ドリル"],
    KindCoated => ["Diamond-coated routers", "Fresas con recubrimiento de diamante", "Frese con rivestimento diamantato", "Diamantbeschichtete Fräser", "Fraises revêtues de diamant", "Fresas com revestimento de diamante", "金刚石涂层铣刀", "ダイヤモンドコーティングルーター"],
    CatSeriesCol => ["series", "serie", "serie", "Serie", "série", "série", "系列", "シリーズ"],
    CatFluteCol => ["L mm", "L mm", "L mm", "Schn. mm", "Lg mm", "C mm", "刃长 mm", "刃長 mm"],
    CatPick => ["Click a bit of the list to see it and what it is for.", "Pulse una herramienta de la lista para verla y saber para qué sirve.", "Fai clic su un utensile dell'elenco per vederlo e sapere a cosa serve.", "Klicken Sie ein Werkzeug der Liste an, um es und seinen Zweck zu sehen.", "Cliquez sur un outil de la liste pour le voir et savoir à quoi il sert.", "Clique numa ferramenta da lista para a ver e saber para que serve.", "点击列表中的刀具即可查看其外形和用途。", "一覧の工具をクリックすると、形状と用途が表示されます。"],
    CatSpindle => ["The turns and the feeds are the maker's. A spindle that cannot turn that fast takes the same bit at its own speed, with the feed in proportion (the same bite for each turn).", "Las vueltas y los avances son los del fabricante. Un husillo que no gira tan rápido usa la misma herramienta a su velocidad, con el avance en proporción (el mismo bocado por vuelta).", "Giri e avanzamenti sono quelli del costruttore. Un mandrino che non gira così veloce usa lo stesso utensile alla sua velocità, con l'avanzamento in proporzione (lo stesso passo per giro).", "Drehzahlen und Vorschübe sind die des Herstellers. Eine Spindel, die nicht so schnell dreht, nimmt dasselbe Werkzeug mit ihrer Drehzahl und dem Vorschub im selben Verhältnis (gleicher Vorschub pro Umdrehung).", "Les vitesses et les avances sont celles du fabricant. Une broche qui ne tourne pas aussi vite utilise le même outil à sa vitesse, avec l'avance en proportion (la même prise par tour).", "As rotações e os avanços são os do fabricante. Um fuso que não roda tão depressa usa a mesma ferramenta à sua velocidade, com o avanço em proporção (a mesma mordida por volta).", "转速和进给为制造商数据。转速达不到的主轴按自身转速使用同一刀具，进给按比例降低（每转切削量不变）。", "回転数と送りはメーカーの値です。そこまで回らない主軸は同じ工具をその回転数で使い、送りは比例して下げます（1回転あたりの切込みは同じ）。"],
    SpindleLbl => ["Spindle of the machine", "Husillo de la máquina", "Mandrino della macchina", "Spindel der Maschine", "Broche de la machine", "Fuso da máquina", "机床主轴", "マシンの主軸"],
    SpindleOriginal => ["Original Bungard, 63000 rpm", "Bungard original, 63000 rpm", "Bungard originale, 63000 giri/min", "Bungard-Original, 63000 U/min", "Bungard d'origine, 63000 tr/min", "Bungard original, 63000 rpm", "Bungard 原装，63000 转/分", "Bungard 純正、63000 rpm"],
    SpindleVfd => ["VFD spindle GDZ 80-2.2, 24000 rpm", "Husillo con VFD GDZ 80-2.2, 24000 rpm", "Mandrino con VFD GDZ 80-2.2, 24000 giri/min", "Spindel mit VFD GDZ 80-2.2, 24000 U/min", "Broche avec VFD GDZ 80-2.2, 24000 tr/min", "Fuso com VFD GDZ 80-2.2, 24000 rpm", "变频主轴 GDZ 80-2.2，24000 转/分", "VFD 主軸 GDZ 80-2.2、24000 rpm"],
    SpindleNote => ["Each spindle has its limits of speed, feed and bit size (they can be edited in the settings file). The catalogue and the tools warn when a bit asks for more than the spindle can give.", "Cada husillo tiene sus límites de velocidad, avance y tamaño de herramienta (se editan en el fichero de ajustes). El catálogo y las herramientas avisan cuando una herramienta pide más de lo que el husillo puede dar.", "Ogni mandrino ha i suoi limiti di velocità, avanzamento e dimensione dell'utensile (modificabili nel file delle impostazioni). Il catalogo e gli utensili avvisano quando un utensile chiede più di quanto il mandrino può dare.", "Jede Spindel hat ihre Grenzen für Drehzahl, Vorschub und Werkzeuggröße (in der Einstellungsdatei änderbar). Katalog und Werkzeuge warnen, wenn ein Werkzeug mehr verlangt, als die Spindel leisten kann.", "Chaque broche a ses limites de vitesse, d'avance et de taille d'outil (modifiables dans le fichier de réglages). Le catalogue et les outils avertissent quand un outil demande plus que la broche ne peut donner.", "Cada fuso tem os seus limites de velocidade, avanço e tamanho da ferramenta (editam-se no ficheiro de definições). O catálogo e as ferramentas avisam quando uma ferramenta pede mais do que o fuso pode dar.", "每个主轴都有转速、进给和刀具尺寸的限制（可在设置文件中修改）。刀具要求超出主轴能力时，目录和刀具窗口会发出提示。", "主軸ごとに回転数・送り・工具サイズの限界があります（設定ファイルで編集可）。工具が主軸の能力を超えるときは、カタログと工具の画面で警告します。"],
    CatSpindleLbl => ["Spindle", "Husillo", "Mandrino", "Spindel", "Broche", "Fuso", "主轴", "主軸"],
    FitOk => ["This spindle can take it with the maker's values.", "Este husillo lo admite con los valores del fabricante.", "Questo mandrino lo accetta con i valori del costruttore.", "Diese Spindel schafft es mit den Herstellerwerten.", "Cette broche l'accepte avec les valeurs du fabricant.", "Este fuso aceita-o com os valores do fabricante.", "该主轴可按制造商数值使用。", "この主軸はメーカーの値のまま使えます。"],
    FitSlower => ["This spindle needs it slower. Suggested values:", "Este husillo necesita ir más lento. Valores recomendados:", "Questo mandrino deve andare più lento. Valori consigliati:", "Diese Spindel muss langsamer arbeiten. Empfohlene Werte:", "Cette broche doit aller plus lentement. Valeurs conseillées :", "Este fuso precisa de ir mais devagar. Valores recomendados:", "该主轴需要降低速度。建议值：", "この主軸では遅くする必要があります。推奨値："],
    FitLowerRpm => ["The spindle turns slower than the bit asks for: XY and Z go slower in proportion.", "El husillo gira menos de lo que pide la herramienta: XY y Z deben ir más lentos en proporción.", "Il mandrino gira meno di quanto chiede l'utensile: XY e Z devono andare più lenti in proporzione.", "Die Spindel dreht langsamer, als das Werkzeug verlangt: XY und Z müssen im selben Verhältnis langsamer fahren.", "La broche tourne moins vite que ne le demande l'outil : XY et Z doivent aller plus lentement en proportion.", "O fuso roda menos do que a ferramenta pede: XY e Z devem ir mais devagar em proporção.", "主轴转速低于刀具要求：XY 和 Z 须按比例降低速度。", "主軸が工具の要求より遅いため、XY と Z を比例して遅くします。"],
    FitSlowZ => ["Z must go slower on this machine.", "Z debe ir más lento en esta máquina.", "Lo Z deve andare più lento su questa macchina.", "Z muss an dieser Maschine langsamer fahren.", "Z doit aller plus lentement sur cette machine.", "Z deve ir mais devagar nesta máquina.", "此机床上 Z 轴须降低速度。", "このマシンでは Z を遅くする必要があります。"],
    FitSlowXy => ["The XY feed must go slower on this machine.", "El desplazamiento XY debe ir más lento en esta máquina.", "L'avanzamento XY deve andare più lento su questa macchina.", "Der XY-Vorschub muss an dieser Maschine langsamer sein.", "L'avance XY doit être plus lente sur cette machine.", "O avanço XY deve ser mais lento nesta máquina.", "此机床上 XY 进给须降低。", "このマシンでは XY の送りを遅くする必要があります。"],
    FitNoThick => ["NOT COMPATIBLE: the bit is thicker than this spindle takes.", "INCOMPATIBLE: la herramienta es más gruesa de lo que admite este husillo.", "NON COMPATIBILE: l'utensile è più grosso di quanto accetta questo mandrino.", "NICHT KOMPATIBEL: Das Werkzeug ist dicker, als diese Spindel aufnimmt.", "INCOMPATIBLE : l'outil est plus gros que ce que cette broche accepte.", "INCOMPATÍVEL: a ferramenta é mais grossa do que este fuso aceita.", "不兼容：刀具比该主轴允许的更粗。", "非対応：工具がこの主軸で扱える太さを超えています。"],
    FitNoSlow => ["NOT COMPATIBLE: this spindle turns far too slowly for this bit (it would break or burn the board).", "INCOMPATIBLE: este husillo gira demasiado despacio para esta herramienta (se rompería o quemaría la placa).", "NON COMPATIBILE: questo mandrino gira troppo lentamente per questo utensile (si romperebbe o brucerebbe la scheda).", "NICHT KOMPATIBEL: Diese Spindel dreht für dieses Werkzeug viel zu langsam (es bräche oder würde die Platine verbrennen).", "INCOMPATIBLE : cette broche tourne beaucoup trop lentement pour cet outil (il casserait ou brûlerait la carte).", "INCOMPATÍVEL: este fuso roda demasiado devagar para esta ferramenta (partir-se-ia ou queimaria a placa).", "不兼容：该主轴转速远低于此刀具所需（会断刀或烧坏板子）。", "非対応：この主軸は工具に対して回転が遅すぎます（折れたり基板が焦げたりします）。"],
    CatUseFor => ["Use these values for tool", "Usar estos valores en la herramienta", "Usa questi valori per l'utensile", "Diese Werte für Werkzeug verwenden", "Utiliser ces valeurs pour l'outil", "Usar estes valores na ferramenta", "将这些数值用于刀具", "この値を工具に使う"],
    ToolFromCat => ["Catalogue", "Catálogo", "Catalogo", "Katalog", "Catalogue", "Catálogo", "取自目录", "カタログから"],
    CatTaken => ["Values taken into the tool.", "Valores trasladados a la herramienta.", "Valori trasferiti all'utensile.", "Werte ins Werkzeug übernommen.", "Valeurs reportées dans l'outil.", "Valores passados para a ferramenta.", "数值已写入刀具。", "値を工具に反映しました。"],
    OpenManyHint => ["Ctrl+click or Shift+click chooses several files (drilling and routing); they open together", "Ctrl+clic o Mayús+clic elige varios archivos (taladrado y fresado); se abren a la vez", "Ctrl+clic o Maiusc+clic sceglie più file (foratura e fresatura); si aprono insieme", "Strg+Klick oder Umschalt+Klick wählt mehrere Dateien (Bohren und Fräsen); sie werden zusammen geöffnet", "Ctrl+clic ou Maj+clic choisit plusieurs fichiers (perçage et fraisage) ; ils s'ouvrent ensemble", "Ctrl+clique ou Shift+clique escolhe vários ficheiros (furação e fresagem); abrem-se em conjunto", "Ctrl+点击或 Shift+点击可选择多个文件（钻孔和铣削），一起打开", "Ctrl+クリックまたは Shift+クリックで複数のファイル（穴あけと切削）を選び、まとめて開きます"],
    JobNothingOn => ["Nothing to do: switch on the drilling or the routing and at least one of its tools.", "Nada que hacer: active el taladrado o el fresado y al menos una de sus herramientas.", "Niente da fare: attivi la foratura o la fresatura e almeno uno dei suoi utensili.", "Nichts zu tun: Bohren oder Fräsen und mindestens eines seiner Werkzeuge einschalten.", "Rien à faire : activez le perçage ou le fraisage et au moins un de ses outils.", "Nada a fazer: ative a furação ou a fresagem e pelo menos uma das suas ferramentas.", "无事可做：请启用钻孔或铣削，并至少启用其中一把刀具。", "実行するものがありません。穴あけまたは切削と、その工具を 1 つ以上オンにしてください。"],
    JobPartHint => ["Click to switch on or off; both on: the drilling and then the routing", "Pulse para activar o no; con los dos: primero el taladrado y luego el fresado", "Clic per attivare o no; con entrambi: prima la foratura e poi la fresatura", "Klicken zum Ein-/Ausschalten; beide an: erst Bohren, dann Fräsen", "Cliquez pour activer ou non ; les deux : d'abord le perçage puis le fraisage", "Clique para ativar ou não; com os dois: primeiro a furação e depois a fresagem", "点击开关；两者都开：先钻孔后铣削", "クリックでオン/オフ。両方オンなら穴あけ→切削の順"],
    ToolOffHint => ["Click the dot of a tool to leave it out of the job", "Pulse el punto de una herramienta para dejarla fuera del trabajo", "Clic sul punto di un utensile per escluderlo dal lavoro", "Auf den Punkt eines Werkzeugs klicken, um es auszulassen", "Cliquez sur le point d'un outil pour l'exclure du travail", "Clique no ponto de uma ferramenta para a deixar fora do trabalho", "点击刀具的圆点可将其排除在作业之外", "工具の点をクリックすると作業から外します"],
    CatFlute => ["Cutting length", "Longitud de corte", "Lunghezza di taglio", "Schneidenlänge", "Longueur de coupe", "Comprimento de corte", "刃长", "刃長"],
    CatShankLbl => ["Shank", "Mango", "Gambo", "Schaft", "Queue", "Haste", "刀柄", "シャンク"],
    CatTotal => ["Total length", "Longitud total", "Lunghezza totale", "Gesamtlänge", "Longueur totale", "Comprimento total", "总长", "全長"],
    CatPoint => ["Point angle", "Ángulo de punta", "Angolo della punta", "Spitzenwinkel", "Angle de pointe", "Ângulo da ponta", "顶角", "先端角"],
    CatSchematic => ["Schematic drawing, not to scale.", "Dibujo esquemático, sin escala.", "Disegno schematico, non in scala.", "Schematische Zeichnung, nicht maßstäblich.", "Dessin schématique, pas à l'échelle.", "Desenho esquemático, sem escala.", "示意图，未按比例。", "模式図です（縮尺は不同）。"],
    NoteDrill => ["Solid-carbide drill for through holes. The small ones (under 0.5 mm) are for vias, the medium ones for component legs, the big ones for connectors and mounting holes. Withdraw fast, and change it after a few thousand holes.", "Broca de metal duro para agujeros pasantes. Las pequeñas (menos de 0,5 mm) son para vías, las medianas para patillas de componentes y las grandes para conectores y agujeros de montaje. Salga rápido y cámbiela tras unos miles de agujeros.", "Punta in metallo duro per fori passanti. Le piccole (sotto 0,5 mm) sono per i via, le medie per i reofori e le grandi per connettori e fori di fissaggio. Risalire in fretta e cambiarla dopo qualche migliaio di fori.", "Vollhartmetallbohrer für Durchgangslöcher. Die kleinen (unter 0,5 mm) sind für Durchkontaktierungen, die mittleren für Bauteilbeine, die großen für Stecker und Befestigungslöcher. Schnell zurückziehen und nach einigen tausend Löchern wechseln.", "Foret en carbure pour trous débouchants. Les petits (moins de 0,5 mm) sont pour les vias, les moyens pour les pattes de composants, les grands pour les connecteurs et les trous de fixation. Remonter vite et le changer après quelques milliers de trous.", "Broca de metal duro para furos passantes. As pequenas (menos de 0,5 mm) são para vias, as médias para patilhas de componentes e as grandes para conectores e furos de fixação. Recue depressa e mude-a ao fim de alguns milhares de furos.", "整体硬质合金钻头，用于通孔。小号（0.5 mm 以下）用于过孔，中号用于元件引脚，大号用于连接器和安装孔。快速退刀，钻几千个孔后更换。", "貫通穴用の超硬ドリルです。小径（0.5 mm 未満）はビア用、中径は部品の足用、大径はコネクタや取付穴用です。素早く引き上げ、数千穴で交換してください。"],
    NoteEndMill => ["Straight flutes and a flat end: slots, pockets, isolation of wide tracks and the outline of thin boards. It leaves a clean floor.", "Filos rectos y punta plana: ranuras, cajeras, aislamiento de pistas anchas y contorno de placas finas. Deja un fondo limpio.", "Taglienti dritti e punta piatta: asole, tasche, isolamento di piste larghe e contorno di schede sottili. Lascia un fondo pulito.", "Gerade Nuten und flache Stirn: Nuten, Taschen, Isolation breiter Leiterbahnen und Kontur dünner Platinen. Hinterlässt einen sauberen Boden.", "Goujures droites et bout plat : rainures, poches, isolation des pistes larges et contour des cartes minces. Laisse un fond propre.", "Goivas retas e ponta plana: ranhuras, cavidades, isolamento de pistas largas e contorno de placas finas. Deixa um fundo limpo.", "直槽平头：用于开槽、挖槽、宽走线隔离和薄板外形，底面干净。", "直刃・平底：溝、ポケット、太いパターンのアイソレーション、薄い基板の外形に使います。底面がきれいに仕上がります。"],
    NoteDownCut => ["Left-hand router: it pushes the chips down. The upper edge of the board comes out without burrs, so use it for the outline when the top is the side that shows (the board must be held down well).", "Fresa a izquierdas: empuja la viruta hacia abajo. El canto superior de la placa sale sin rebabas; úsela para el contorno cuando la cara superior es la que se ve (la placa debe quedar bien sujeta).", "Fresa sinistra: spinge il truciolo verso il basso. Il bordo superiore esce senza bave; usala per il contorno quando la faccia superiore è quella in vista (la scheda deve essere ben fissata).", "Linksschneidender Fräser: drückt die Späne nach unten. Die Oberkante der Platine wird gratfrei; für die Kontur verwenden, wenn die Oberseite sichtbar ist (die Platine muss gut festgehalten sein).", "Fraise à gauche : elle pousse les copeaux vers le bas. Le bord supérieur de la carte sort sans bavures ; à utiliser pour le contour quand la face du dessus est celle qui se voit (la carte doit être bien maintenue).", "Fresa à esquerda: empurra a apara para baixo. O rebordo superior da placa fica sem rebarbas; use-a no contorno quando a face de cima é a que se vê (a placa tem de ficar bem presa).", "左旋铣刀：把切屑向下压，板子上缘无毛刺。上表面外露时用于外形铣削（板子必须压紧）。", "左刃ルーター：切りくずを下へ押します。基板の上側の縁にバリが出ません。上面が見える面のときの外形に使います（基板はしっかり固定してください）。"],
    NoteUpCut => ["Right-hand router: it pulls the chips up and out of the cut. Good for slots and pockets; it can lift the board and leave burrs on the upper edge.", "Fresa a derechas: saca la viruta hacia arriba. Buena para ranuras y cajeras; puede levantar la placa y dejar rebabas en el canto superior.", "Fresa destra: tira il truciolo verso l'alto. Buona per asole e tasche; può sollevare la scheda e lasciare bave sul bordo superiore.", "Rechtsschneidender Fräser: zieht die Späne nach oben aus dem Schnitt. Gut für Nuten und Taschen; kann die Platine anheben und Grate an der Oberkante hinterlassen.", "Fraise à droite : elle remonte les copeaux. Bonne pour les rainures et les poches ; elle peut soulever la carte et laisser des bavures sur le bord supérieur.", "Fresa à direita: puxa a apara para cima. Boa para ranhuras e cavidades; pode levantar a placa e deixar rebarbas no rebordo superior.", "右旋铣刀：把切屑向上带出。适合开槽和挖槽，可能抬起板子并在上缘留下毛刺。", "右刃ルーター：切りくずを上へ引き出します。溝やポケットに向いていますが、基板が浮いて上縁にバリが出ることがあります。"],
    NoteDiamond => ["Diamond-cut (corn) router: many small teeth that grind the glass fibre instead of tearing it. The usual choice for the outline of FR-4; it leaves a rough but clean edge and lasts long.", "Fresa de diamante (corn): muchos dientes pequeños que trituran la fibra de vidrio en vez de rasgarla. La opción habitual para el contorno del FR-4; deja un canto áspero pero limpio y dura mucho.", "Fresa a diamante (corn): tanti piccoli denti che frantumano la fibra di vetro invece di strapparla. La scelta abituale per il contorno dell'FR-4; lascia un bordo ruvido ma pulito e dura a lungo.", "Diamantverzahnter Fräser (Corn): viele kleine Zähne, die die Glasfaser zermahlen statt reißen. Die übliche Wahl für die FR-4-Kontur; hinterlässt eine raue, aber saubere Kante und hält lange.", "Fraise à denture diamant (corn) : beaucoup de petites dents qui broient la fibre de verre au lieu de l'arracher. Le choix habituel pour le contour du FR-4 ; elle laisse un bord rugueux mais propre et dure longtemps.", "Fresa de diamante (corn): muitos dentes pequenos que trituram a fibra de vidro em vez de a rasgar. A escolha habitual para o contorno do FR-4; deixa um rebordo áspero mas limpo e dura muito.", "菱形纹（玉米）铣刀：许多小齿把玻璃纤维磨碎而不是撕裂，是 FR-4 外形的常用选择，边缘粗糙但干净，寿命长。", "ダイヤモンドカット（コーン）ルーター：小さな刃が多数あり、ガラス繊維を裂かずに粉砕します。FR-4 の外形に標準的で、縁はざらつきますがきれいで、長持ちします。"],
    NoteChip => ["Chip-breaker router: its teeth break the chips into short pieces that leave the cut easily. Cuts faster than the diamond cut and keeps the burrs small.", "Fresa rompevirutas: sus dientes parten la viruta en trozos cortos que salen fácilmente del corte. Corta más rápido que la de diamante y mantiene las rebabas pequeñas.", "Fresa rompitruciolo: i denti spezzano il truciolo in pezzi corti che escono facilmente dal taglio. Taglia più in fretta di quella a diamante e tiene piccole le bave.", "Spanbrecher-Fräser: Die Zähne brechen die Späne in kurze Stücke, die den Schnitt leicht verlassen. Schneidet schneller als der Diamantfräser und hält die Grate klein.", "Fraise brise-copeaux : ses dents cassent les copeaux en petits morceaux qui sortent facilement de la coupe. Coupe plus vite que la fraise diamant et garde de petites bavures.", "Fresa quebra-aparas: os dentes partem a apara em pedaços curtos que saem facilmente do corte. Corta mais depressa do que a de diamante e mantém as rebarbas pequenas.", "断屑铣刀：刀齿把切屑折成短段，容易排出。比菱形纹铣刀切得快，毛刺小。", "チップブレーカールーター：刃が切りくずを短く折り、排出しやすくします。ダイヤモンドカットより速く切れ、バリも小さく抑えられます。"],
    NoteVBit => ["V-bit (engraving bit): the groove is as wide as the depth makes it, so the depth sets the width of the isolation. A narrow angle with a fine tip makes fine tracks possible.", "Fresa en V (grabado): el surco es tan ancho como lo hace la profundidad, así que la profundidad fija el ancho del aislamiento. Un ángulo estrecho con punta fina permite pistas finas.", "Fresa a V (incisione): il solco è largo quanto la profondità lo fa, quindi la profondità fissa la larghezza dell'isolamento. Un angolo stretto con punta fine permette piste sottili.", "V-Fräser (Gravur): Die Nut ist so breit, wie die Tiefe sie macht; die Tiefe bestimmt also die Breite der Isolation. Ein enger Winkel mit feiner Spitze ermöglicht feine Leiterbahnen.", "Fraise en V (gravure) : la rainure est aussi large que la profondeur la fait, donc la profondeur fixe la largeur de l'isolation. Un angle étroit avec une pointe fine permet des pistes fines.", "Fresa em V (gravação): o sulco é tão largo quanto a profundidade o faz, por isso a profundidade fixa a largura do isolamento. Um ângulo estreito com ponta fina permite pistas finas.", "V 型刀（雕刻刀）：槽宽取决于下刀深度，所以深度决定隔离宽度。小角度加细刀尖可做细走线。", "Vビット（彫刻ビット）：溝の幅は切込み深さで決まり、深さがアイソレーションの幅を決めます。角度が狭く先端が細いと細いパターンが可能です。"],
    NoteBall => ["Ball-nose end mill: a rounded groove, for engraving and for shaping. Not for isolating tracks.", "Fresa de punta esférica: surco redondeado, para grabar y dar forma. No sirve para aislar pistas.", "Fresa a punta sferica: solco arrotondato, per incisioni e sagomature. Non serve per isolare le piste.", "Kugelfräser: abgerundete Nut, zum Gravieren und Formen. Nicht zum Isolieren von Leiterbahnen.", "Fraise à bout sphérique : rainure arrondie, pour graver et modeler. Pas pour isoler des pistes.", "Fresa de ponta esférica: sulco arredondado, para gravar e dar forma. Não serve para isolar pistas.", "球头铣刀：槽底圆弧，用于雕刻和造型，不用于走线隔离。", "ボールエンドミル：丸い溝になり、彫刻や造形に使います。パターンのアイソレーションには向きません。"],
    NoteSpot => ["Spot drill: a short, stiff point that starts a hole exactly where it is wanted, so the next drill does not wander. For the big holes and for hard boards.", "Broca de centrar: punta corta y rígida que empieza el agujero justo donde se quiere, para que la broca siguiente no se desvíe. Para los agujeros grandes y las placas duras.", "Punta da centro: punta corta e rigida che inizia il foro esattamente dove serve, così la punta successiva non devia. Per i fori grandi e le schede dure.", "Anbohrer: eine kurze, steife Spitze, die das Loch genau dort beginnt, wo es sein soll, damit der nächste Bohrer nicht wandert. Für große Löcher und harte Platinen.", "Foret à pointer : une pointe courte et rigide qui amorce le trou exactement là où il faut, pour que le foret suivant ne dérive pas. Pour les grands trous et les cartes dures.", "Broca de centrar: ponta curta e rígida que inicia o furo exatamente onde se quer, para que a broca seguinte não se desvie. Para os furos grandes e as placas duras.", "定位钻：短而刚性好的钻尖，在需要的位置起孔，让后面的钻头不跑偏。用于大孔和硬板。", "センタードリル：短く剛性のある先端で、狙った位置に穴を起こし、次のドリルが逃げないようにします。大きな穴や硬い基板用です。"],
    NoteCountersink => ["Countersink: chamfers the edge of a hole, for screw heads or to take the burr off.", "Avellanador: achaflana el borde de un agujero, para cabezas de tornillo o para quitar la rebaba.", "Svasatore: smussa il bordo di un foro, per le teste delle viti o per togliere la bava.", "Senker: fast die Kante eines Lochs an, für Schraubenköpfe oder zum Entgraten.", "Fraise à chanfreiner : chanfreine le bord d'un trou, pour les têtes de vis ou pour enlever la bavure.", "Escareador: chanfra a borda de um furo, para cabeças de parafuso ou para tirar a rebarba.", "倒角钻：给孔口倒角，用于螺钉头或去毛刺。", "面取りカッター：穴の縁を面取りします。ねじ頭の座ぐりやバリ取りに使います。"],
    NoteSlot => ["Slot drill: a drill with a stiff web made to cut oblong holes by moving sideways as it drills (the connector slots). Do not use it for round holes: the others are cheaper.", "Broca para ranuras: broca con alma rígida hecha para cortar agujeros alargados moviéndose de lado mientras perfora (las ranuras de los conectores). No la use para agujeros redondos: las otras son más baratas.", "Punta per asole: punta con anima rigida fatta per tagliare fori oblunghi spostandosi di lato mentre fora (le asole dei connettori). Non usarla per i fori tondi: le altre costano meno.", "Langlochbohrer: ein Bohrer mit steifer Seele, der Langlöcher schneidet, indem er sich beim Bohren seitwärts bewegt (die Schlitze der Stecker). Nicht für runde Löcher verwenden: die anderen sind billiger.", "Foret pour trous oblongs : un foret à âme rigide fait pour couper des trous oblongs en se déplaçant de côté pendant qu'il perce (les fentes des connecteurs). Ne pas l'utiliser pour des trous ronds : les autres coûtent moins.", "Broca para ranhuras: broca com alma rígida feita para cortar furos oblongos movendo-se de lado enquanto fura (as ranhuras dos conectores). Não a use para furos redondos: as outras são mais baratas.", "长孔钻头：钻芯刚性强，钻孔时横向移动以切出长孔（连接器的槽）。不要用于圆孔，其他钻头更便宜。", "長穴用ドリル：芯が太く、穿孔しながら横に動かして長穴（コネクタのスロット）を切ります。丸穴には使わないでください。他のドリルのほうが安価です。"],
    NoteCoated => ["Flat-end router with a diamond coating: for abrasive boards (high-Tg FR-4, halogen free, carbon fibre) and for aluminium-backed boards, where an uncoated one wears out. It costs more and lasts several times longer.", "Fresa de punta plana con recubrimiento de diamante: para placas abrasivas (FR-4 de alto Tg, sin halógenos, fibra de carbono) y con base de aluminio, donde una sin recubrir se desgasta. Cuesta más y dura varias veces más.", "Fresa a punta piatta con rivestimento diamantato: per schede abrasive (FR-4 ad alto Tg, senza alogeni, fibra di carbonio) e con base in alluminio, dove una non rivestita si consuma. Costa di più e dura molte volte di più.", "Flachfräser mit Diamantbeschichtung: für abrasive Platinen (High-Tg-FR-4, halogenfrei, Kohlefaser) und Platinen mit Aluminiumträger, bei denen ein unbeschichteter verschleißt. Er kostet mehr und hält ein Mehrfaches.", "Fraise à bout plat avec revêtement diamant : pour les cartes abrasives (FR-4 à Tg élevé, sans halogène, fibre de carbone) et à support aluminium, où une fraise non revêtue s'use. Elle coûte plus cher et dure plusieurs fois plus.", "Fresa de ponta plana com revestimento de diamante: para placas abrasivas (FR-4 de Tg alto, sem halogéneos, fibra de carbono) e com base de alumínio, onde uma sem revestimento se desgasta. Custa mais e dura várias vezes mais.", "金刚石涂层平头铣刀：用于磨蚀性板材（高 Tg FR-4、无卤、碳纤维）和铝基板，无涂层刀具在这些材料上磨损很快。价格更高，寿命长数倍。", "ダイヤモンドコーティングの平底ルーター：研磨性の高い基板（高Tg FR-4、ハロゲンフリー、カーボン）やアルミベース基板用で、コーティングなしでは摩耗します。価格は高めですが、寿命は数倍です。"],
    LevTab => ["Table calibration", "Calibración de la mesa", "Calibrazione del tavolo", "Tischkalibrierung", "Étalonnage de la table", "Calibração da mesa", "工作台校准", "テーブル校正"],
    LevMeasure => ["Measure the table", "Medir la mesa", "Misura il tavolo", "Tisch vermessen", "Mesurer la table", "Medir a mesa", "测量工作台", "テーブルを測定"],
    LevStop => ["Stop measuring", "Parar la medida", "Ferma la misura", "Messung stoppen", "Arrêter la mesure", "Parar a medição", "停止测量", "測定を停止"],
    LevClear => ["Clear the map", "Borrar el mapa", "Cancella la mappa", "Karte löschen", "Effacer la carte", "Apagar o mapa", "清除图", "マップを消去"],
    LevCols => ["Points along X", "Puntos en X", "Punti in X", "Punkte in X", "Points en X", "Pontos em X", "X 方向点数", "X 方向の点数"],
    LevRows => ["Points along Y", "Puntos en Y", "Punti in Y", "Punkte in Y", "Points en Y", "Pontos em Y", "Y 方向点数", "Y 方向の点数"],
    LevPoints => ["Points measured", "Puntos medidos", "Punti misurati", "Gemessene Punkte", "Points mesurés", "Pontos medidos", "已测点数", "測定済みの点"],
    LevSpread => ["Spread", "Diferencia", "Differenza", "Spanne", "Écart", "Diferença", "高低差", "高低差"],
    LevTilt => ["Tilt X / Y", "Inclinación X / Y", "Inclinazione X / Y", "Neigung X / Y", "Inclinaison X / Y", "Inclinação X / Y", "倾斜 X / Y", "傾き X / Y"],
    LevDemo => ["Simulated probe: no probe node is connected yet, the heights come from a simulated table.", "Palpador simulado: aún no hay un nodo palpador conectado, las alturas salen de una mesa simulada.", "Sonda simulata: nessun nodo sonda è ancora collegato, le altezze vengono da un tavolo simulato.", "Simulierter Taster: noch kein Tasterknoten verbunden, die Höhen stammen von einem simulierten Tisch.", "Palpeur simulé : aucun nœud palpeur n'est encore connecté, les hauteurs viennent d'une table simulée.", "Apalpador simulado: ainda não há um nó apalpador ligado, as alturas vêm de uma mesa simulada.", "模拟探头：尚未连接探头节点，高度来自模拟的工作台。", "シミュレートしたプローブ：プローブノードは未接続で、高さはシミュレートしたテーブルのものです。"],
    LevApply => ["Correct the Z of the jobs with this map", "Corregir la Z de los trabajos con este mapa", "Correggere la Z dei lavori con questa mappa", "Z der Aufträge mit dieser Karte korrigieren", "Corriger le Z des travaux avec cette carte", "Corrigir o Z dos trabalhos com este mapa", "用此图修正作业的 Z", "このマップでジョブの Z を補正する"],
    ResumeTitle => ["Job interrupted", "Trabajo interrumpido", "Lavoro interrotto", "Auftrag unterbrochen", "Travail interrompu", "Trabalho interrompido", "作业被中断", "ジョブが中断されました"],
    ResumeBody => ["The last job was left in the middle (the window, the computer or the link went down). Do you want to go on from where it was?", "El último trabajo quedó a medias (se cerró la ventana, el equipo o el enlace). ¿Quieres seguir por donde iba?", "L'ultimo lavoro è rimasto a metà (finestra, computer o collegamento caduti). Vuoi continuare da dove era arrivato?", "Der letzte Auftrag blieb unvollendet (Fenster, Rechner oder Verbindung ausgefallen). Möchten Sie dort weitermachen, wo er stand?", "Le dernier travail est resté en cours (fenêtre, ordinateur ou liaison tombés). Voulez-vous continuer là où il en était ?", "O último trabalho ficou a meio (a janela, o computador ou a ligação falhou). Quer continuar por onde ia?", "上一个作业未完成（窗口、电脑或连接中断）。是否从中断处继续？", "前回のジョブは途中で止まりました（ウィンドウ、PC、接続の停止）。続きから再開しますか？"],
    ResumeChanged => ["The files or the settings changed since the job stopped: it cannot go on safely", "Los archivos o los ajustes han cambiado desde que se paró el trabajo: no puede continuar con seguridad", "I file o le impostazioni sono cambiati da quando il lavoro si è fermato: non può proseguire in sicurezza", "Die Dateien oder Einstellungen haben sich seit dem Stopp des Auftrags geändert: er kann nicht sicher fortgesetzt werden", "Les fichiers ou les réglages ont changé depuis l'arrêt du travail : il ne peut pas reprendre en sécurité", "Os ficheiros ou as definições mudaram desde que o trabalho parou: não pode continuar em segurança", "自作业停止后文件或设置已更改：无法安全地继续", "ジョブが停止してからファイルまたは設定が変更されました。安全に続行できません"],
    ResumeContinue => ["Continue the job", "Seguir el trabajo", "Continua il lavoro", "Auftrag fortsetzen", "Reprendre le travail", "Continuar o trabalho", "继续作业", "ジョブを続ける"],
    ResumeCancel => ["Cancel it", "Cancelar", "Annulla", "Verwerfen", "Annuler", "Cancelar", "取消", "破棄する"],
    TabsOn => ["Route the outside with tabs", "Rutear el exterior con muescas", "Fresare l'esterno con ponticelli", "Außenkontur mit Haltestegen fräsen", "Détourer l'extérieur avec des ponts", "Cortar o exterior com pontes", "外轮廓铣削并留连接点", "外形を切り残し付きでルーティング"],
    TabsPerSide => ["Tabs per side", "Muescas por lado", "Ponticelli per lato", "Stege pro Seite", "Ponts par côté", "Pontes por lado", "每边连接点数", "1辺あたりの切り残し数"],
    TabWidth => ["Tab width", "Ancho de la muesca", "Larghezza del ponticello", "Stegbreite", "Largeur du pont", "Largura da ponte", "连接点宽度", "切り残しの幅"],
    TabPlace => ["Where the tabs go", "Dónde van las muescas", "Dove vanno i ponticelli", "Wo die Stege sitzen", "Où placer les ponts", "Onde ficam as pontes", "连接点位置", "切り残しの位置"],
    TabEven => ["Spread evenly along the side", "Repartidas por el lado", "Distribuiti sul lato", "Gleichmäßig verteilt", "Réparties sur le côté", "Repartidas pelo lado", "沿边均匀分布", "辺に均等に配置"],
    TabEnds => ["Near the ends of the side", "Cerca de los extremos", "Vicino alle estremità", "Nahe den Enden der Seite", "Près des extrémités", "Perto das extremidades", "靠近边的两端", "辺の両端寄り"],
    TabMiddle => ["Together in the middle", "Juntas en el centro", "Insieme al centro", "Gemeinsam in der Mitte", "Ensemble au milieu", "Juntas ao centro", "集中在中间", "中央にまとめる"],
    SetSkipLauncher => ["Open this window directly next time", "Abrir esta ventana directamente la próxima vez", "Aprire direttamente questa finestra la prossima volta", "Dieses Fenster beim nächsten Mal direkt öffnen", "Ouvrir directement cette fenêtre la prochaine fois", "Abrir esta janela diretamente da próxima vez", "下次直接打开此窗口", "次回からこの画面を直接開く"],
    ColName => ["Name", "Nombre", "Nome", "Name", "Nom", "Nome", "名称", "名前"],
    ColDate => ["Date modified (UTC)", "Fecha de modificación (UTC)", "Data modifica (UTC)", "Änderungsdatum (UTC)", "Date de modification (UTC)", "Data de modificação (UTC)", "修改日期 (UTC)", "更新日時 (UTC)"],
    ColType => ["Type", "Tipo", "Tipo", "Typ", "Type", "Tipo", "类型", "種類"],
    ColSize => ["Size", "Tamaño", "Dimensione", "Größe", "Taille", "Tamanho", "大小", "サイズ"],
    TypeFolder => ["Folder", "Carpeta", "Cartella", "Ordner", "Dossier", "Pasta", "文件夹", "フォルダー"],
    TypeZip => ["ZIP archive", "Archivo ZIP", "Archivio ZIP", "ZIP-Archiv", "Archive ZIP", "Arquivo ZIP", "ZIP 压缩包", "ZIP アーカイブ"],
    Places => ["Places", "Lugares", "Risorse", "Orte", "Emplacements", "Locais", "位置", "場所"],
    PlaceExamples => ["Examples", "Ejemplos", "Esempi", "Beispiele", "Exemples", "Exemplos", "示例", "サンプル"],
    PlaceDesktop => ["Desktop", "Escritorio", "Desktop", "Desktop", "Bureau", "Ambiente de trabalho", "桌面", "デスクトップ"],
    PlaceDocuments => ["Documents", "Documentos", "Documenti", "Dokumente", "Documents", "Documentos", "文档", "ドキュメント"],
    PlaceDownloads => ["Downloads", "Descargas", "Download", "Downloads", "Téléchargements", "Transferências", "下载", "ダウンロード"],
    FileNameLabel => ["File name", "Nombre de archivo", "Nome file", "Dateiname", "Nom du fichier", "Nome do ficheiro", "文件名", "ファイル名"],
    FilterSupported => ["All job files", "Todos los archivos de trabajo", "Tutti i file di lavoro", "Alle Auftragsdateien", "Tous les fichiers de travail", "Todos os ficheiros de trabalho", "所有作业文件", "すべての作業ファイル"],
    FilterDrill => ["Drilling files", "Archivos de taladrado", "File di foratura", "Bohrdateien", "Fichiers de perçage", "Ficheiros de furação", "钻孔文件", "穴あけファイル"],
    FilterRoute => ["Routing files", "Archivos de fresado", "File di fresatura", "Fräsdateien", "Fichiers de fraisage", "Ficheiros de fresagem", "铣削文件", "ルーター加工ファイル"],
    FilterZip => ["ZIP archives", "Archivos ZIP", "Archivi ZIP", "ZIP-Archive", "Archives ZIP", "Arquivos ZIP", "ZIP 压缩包", "ZIP アーカイブ"],
    FilterAny => ["All files", "Todos los archivos", "Tutti i file", "Alle Dateien", "Tous les fichiers", "Todos os ficheiros", "所有文件", "すべてのファイル"],
    NavBack => ["Back", "Atrás", "Indietro", "Zurück", "Précédent", "Retroceder", "返回", "戻る"],
    NavUp => ["Up", "Subir", "Su", "Nach oben", "Dossier parent", "Subir", "上一级", "上へ"],
    ThisPc => ["This PC", "Este equipo", "Questo PC", "Dieser PC", "Ce PC", "Este PC", "此电脑", "このPC"],
    SimSpeed => ["Simulation speed", "Velocidad de la simulación", "Velocità della simulazione", "Simulationsgeschwindigkeit", "Vitesse de la simulation", "Velocidade da simulação", "模拟速度", "シミュレーション速度"],
    DefaultValues => ["Defaults", "Valores por defecto", "Valori predefiniti", "Standardwerte", "Valeurs par défaut", "Valores predefinidos", "默认值", "既定値"],
    TypeValueHint => ["Click a value to type it", "Pulsa un valor para escribirlo", "Clicca un valore per digitarlo", "Auf einen Wert klicken, um ihn einzugeben", "Cliquez sur une valeur pour la saisir", "Clique num valor para o escrever", "点击数值即可输入", "値をクリックして入力"],
    TabGeneral => ["General", "General", "Generale", "Allgemein", "Général", "Geral", "常规", "一般"],
    TabJob => ["Job", "Trabajo", "Lavoro", "Auftrag", "Travail", "Trabalho", "作业", "作業"],
    Units => ["Units", "Unidades", "Unità", "Einheiten", "Unités", "Unidades", "单位", "単位"],
    UnitMm => ["Millimetres (mm)", "Milímetros (mm)", "Millimetri (mm)", "Millimeter (mm)", "Millimètres (mm)", "Milímetros (mm)", "毫米 (mm)", "ミリメートル (mm)"],
    UnitMil => ["Mils (0.001 inch)", "Milésimas de pulgada (mil)", "Millesimi di pollice (mil)", "Mil (0,001 Zoll)", "Millièmes de pouce (mil)", "Milésimos de polegada (mil)", "密耳 (0.001 英寸)", "ミル (0.001 インチ)"],
    ModelLabel => ["Machine model", "Modelo de máquina", "Modello di macchina", "Maschinenmodell", "Modèle de machine", "Modelo de máquina", "机器型号", "機種"],
    ToolChange => ["Tool change", "Cambio de herramienta", "Cambio utensile", "Werkzeugwechsel", "Changement d'outil", "Troca de ferramenta", "换刀", "工具交換"],
    TcManual => ["Manual (MTC)", "Manual (MTC)", "Manuale (MTC)", "Manuell (MTC)", "Manuel (MTC)", "Manual (MTC)", "手动 (MTC)", "手動 (MTC)"],
    TcAuto => ["Automatic (ATC)", "Automático (ATC)", "Automatico (ATC)", "Automatisch (ATC)", "Automatique (ATC)", "Automático (ATC)", "自动 (ATC)", "自動 (ATC)"],
    EmergencyStop => ["Emergency stop", "Parada de emergencia", "Arresto di emergenza", "Not-Halt", "Arrêt d'urgence", "Paragem de emergência", "紧急停止", "非常停止"],
    EmergencyShow => ["Show the emergency button (it can be dragged anywhere)", "Mostrar el botón de emergencia (se puede arrastrar a cualquier sitio)", "Mostra il pulsante di emergenza (si può trascinare ovunque)", "Not-Halt-Taste anzeigen (frei verschiebbar)", "Afficher le bouton d'urgence (déplaçable partout)", "Mostrar o botão de emergência (pode ser arrastado para qualquer sítio)", "显示紧急按钮（可拖到任意位置）", "非常停止ボタンを表示（どこへでもドラッグできます）"],
    RackPlaces => ["tool places", "posiciones de herramienta", "posti utensile", "Werkzeugplätze", "places d'outil", "posições de ferramenta", "刀位", "工具位置"],
    Rotate => ["Rotate", "Girar", "Ruota", "Drehen", "Pivoter", "Rodar", "旋转", "回転"],
    FollowTool => ["Follow the tool", "Seguir la herramienta", "Segui l'utensile", "Werkzeug folgen", "Suivre l'outil", "Seguir a ferramenta", "跟随刀具", "工具を追う"],
    Enlarge2d => ["Enlarge the 2D view", "Ampliar la vista 2D", "Ingrandisci la vista 2D", "2D-Ansicht vergrößern", "Agrandir la vue 2D", "Ampliar a vista 2D", "放大 2D 视图", "2D 表示を拡大"],
    MoveDesign => ["Move the design only", "Mover solo el diseño", "Sposta solo il disegno", "Nur das Design verschieben", "Déplacer le dessin seul", "Mover só o desenho", "只移动图形", "図面だけ動かす"],
    MoveBoard => ["Move the board", "Mover la placa", "Sposta la piastra", "Platine verschieben", "Déplacer la carte", "Mover a placa", "移动电路板", "基板を移動"],
    ShowBoth => ["Show the 3D view", "Mostrar la vista 3D", "Mostra la vista 3D", "3D-Ansicht zeigen", "Afficher la vue 3D", "Mostrar a vista 3D", "显示 3D 视图", "3D 表示を表示"],
    OnTable => ["On the table", "En la mesa", "Sul piano", "Auf dem Tisch", "Sur la table", "Na mesa", "在工作台上", "テーブル上"],
    OnBoard => ["On the board", "En la placa", "Sulla piastra", "Auf der Platine", "Sur la carte", "Na placa", "在电路板上", "基板上"],
    Resume => ["Resume", "Continuar", "Riprendi", "Fortsetzen", "Reprendre", "Continuar", "继续", "再開"],
    NotAnOutline => ["is not the outline of a board (open its profile or edge layer)", "no es el contorno de una placa (abre su capa de perfil o de borde)", "non è il contorno di una piastra (apri il suo livello di profilo o di bordo)", "ist nicht der Umriss einer Platine (die Profil- oder Kantenebene öffnen)", "n'est pas le contour d'une carte (ouvrez sa couche de profil ou de bord)", "não é o contorno de uma placa (abra a sua camada de perfil ou de bordo)", "不是电路板的轮廓（请打开其轮廓或边缘层）", "基板の外形ではありません（外形またはエッジのレイヤーを開いてください）"],
    HolesDrilled => ["Holes drilled in all", "Agujeros hechos en total", "Fori eseguiti in totale", "Gebohrte Löcher insgesamt", "Trous percés au total", "Furos feitos no total", "累计钻孔数", "累計の穴あけ数"],
    ResetCounts => ["Reset the counts", "Poner a cero los contadores", "Azzera i contatori", "Zähler zurücksetzen", "Remettre les compteurs à zéro", "Repor os contadores a zero", "计数清零", "カウントをリセット"],
    Recent => ["Recent", "Recientes", "Recenti", "Zuletzt", "Récents", "Recentes", "最近", "最近使ったファイル"],
    McPosition => ["Position", "Posición", "Posizione", "Position", "Position", "Posição", "位置", "位置"],
    McWork => ["Work", "Pieza", "Pezzo", "Werkstück", "Pièce", "Peça", "工件", "ワーク"],
    McMachine => ["Machine", "Máquina", "Macchina", "Maschine", "Machine", "Máquina", "机床", "マシン"],
    McJog => ["Jog", "Movimiento manual", "Movimento manuale", "Handfahren", "Déplacement manuel", "Movimento manual", "点动", "ジョグ"],
    McStep => ["Step (mm)", "Paso (mm)", "Passo (mm)", "Schritt (mm)", "Pas (mm)", "Passo (mm)", "步长 (mm)", "ステップ (mm)"],
    McHome => ["Home", "Referencia", "Origine", "Referenzfahrt", "Prise d'origine", "Referência", "回零", "原点復帰"],
    McZero => ["Zero here", "Cero aquí", "Zero qui", "Hier nullen", "Zéro ici", "Zero aqui", "此处清零", "ここをゼロ"],
    McUnlock => ["Unlock", "Desbloquear", "Sblocca", "Entsperren", "Déverrouiller", "Desbloquear", "解锁", "ロック解除"],
    McProgram => ["Program", "Programa", "Programma", "Programm", "Programme", "Programa", "程序", "プログラム"],
    McOpenGcode => ["Open G-code", "Abrir G-code", "Apri G-code", "G-Code öffnen", "Ouvrir un G-code", "Abrir G-code", "打开 G 代码", "G コードを開く"],
    McMdi => ["Command (MDI)", "Orden (MDI)", "Comando (MDI)", "Befehl (MDI)", "Commande (MDI)", "Comando (MDI)", "命令 (MDI)", "コマンド (MDI)"],
    McSend => ["Send", "Enviar", "Invia", "Senden", "Envoyer", "Enviar", "发送", "送信"],
    McLog => ["Log", "Registro", "Registro", "Protokoll", "Journal", "Registo", "日志", "ログ"],
    McState => ["State", "Estado", "Stato", "Zustand", "État", "Estado", "状态", "状態"],
    McController => ["Controller", "Controlador", "Controllore", "Steuerung", "Contrôleur", "Controlador", "控制器", "コントローラー"],
    McSimulator => ["Simulator", "Simulador", "Simulatore", "Simulator", "Simulateur", "Simulador", "模拟器", "シミュレーター"],
    McNotDriven => ["not driven yet", "aún sin controlador", "non ancora pilotato", "noch nicht angesteuert", "pas encore piloté", "ainda sem controlador", "尚未支持", "未対応"],
    McToolpath => ["Toolpath", "Trayectoria", "Percorso", "Werkzeugbahn", "Trajectoire", "Trajetória", "刀路", "ツールパス"],
    McLine => ["Line", "Línea", "Riga", "Zeile", "Ligne", "Linha", "行", "行"],
    McSpindle => ["Spindle", "Husillo", "Mandrino", "Spindel", "Broche", "Fuso", "主轴", "主軸"],
    McFeedRate => ["Feed", "Avance", "Avanzamento", "Vorschub", "Avance", "Avanço", "进给", "送り"],
    McStIdle => ["Idle", "Libre", "Fermo", "Bereit", "Repos", "Livre", "空闲", "待機"],
    McStRun => ["Running", "En marcha", "In esecuzione", "Läuft", "En marche", "Em curso", "运行", "実行中"],
    McStHold => ["Paused", "En pausa", "In pausa", "Pausiert", "En pause", "Em pausa", "暂停", "一時停止中"],
    McStHome => ["Homing", "Buscando referencia", "Ricerca origine", "Referenzfahrt", "Prise d'origine", "A referenciar", "回零中", "原点復帰中"],
    McStAlarm => ["ALARM", "ALARMA", "ALLARME", "ALARM", "ALARME", "ALARME", "报警", "アラーム"],
    McStUnknown => ["Unknown", "Desconocido", "Sconosciuto", "Unbekannt", "Inconnu", "Desconhecido", "未知", "不明"],
    McZeroXyz => ["Set the work zero here (X, Y and Z)", "Poner el cero de pieza aquí (X, Y y Z)", "Imposta lo zero pezzo qui (X, Y e Z)", "Werkstücknullpunkt hier setzen (X, Y und Z)", "Fixer le zéro pièce ici (X, Y et Z)", "Pôr o zero da peça aqui (X, Y e Z)", "在此设置工件零点（X、Y、Z）", "ここをワークゼロに設定（X、Y、Z）"],
    McZeroXy => ["Set the work zero here (X and Y)", "Poner el cero de pieza aquí (X e Y)", "Imposta lo zero pezzo qui (X e Y)", "Werkstücknullpunkt hier setzen (X und Y)", "Fixer le zéro pièce ici (X et Y)", "Pôr o zero da peça aqui (X e Y)", "在此设置工件零点（X、Y）", "ここをワークゼロに設定（X、Y）"],
    McGoG28 => ["Go to the stored position G28", "Ir a la posición guardada G28", "Vai alla posizione memorizzata G28", "Zur gespeicherten Position G28 fahren", "Aller à la position mémorisée G28", "Ir para a posição guardada G28", "前往已存位置 G28", "保存位置 G28 へ移動"],
    McGoG30 => ["Go to the stored position G30", "Ir a la posición guardada G30", "Vai alla posizione memorizzata G30", "Zur gespeicherten Position G30 fahren", "Aller à la position mémorisée G30", "Ir para a posição guardada G30", "前往已存位置 G30", "保存位置 G30 へ移動"],
    McGoZeroXy => ["Go to the work zero in X and Y", "Ir al cero de pieza en X e Y", "Vai allo zero pezzo in X e Y", "Zum Werkstücknullpunkt in X und Y fahren", "Aller au zéro pièce en X et Y", "Ir para o zero da peça em X e Y", "移动到 X、Y 工件零点", "X・Y のワークゼロへ移動"],
    McGoZeroZ => ["Go to the work zero in Z", "Ir al cero de pieza en Z", "Vai allo zero pezzo in Z", "Zum Werkstücknullpunkt in Z fahren", "Aller au zéro pièce en Z", "Ir para o zero da peça em Z", "移动到 Z 工件零点", "Z のワークゼロへ移動"],
    McOperator => ["Operator", "Operador", "Operatore", "Bediener", "Opérateur", "Operador", "操作员", "オペレーター"],
    McGcodeTab => ["GCode", "Código G", "Codice G", "G-Code", "Code G", "Código G", "G 代码", "G コード"],
    McIoTab => ["IO", "E/S", "I/O", "E/A", "E/S", "E/S", "输入输出", "入出力"],
    McActiveG => ["Active G-codes", "G-codes activos", "G-code attivi", "Aktive G-Codes", "G-codes actifs", "G-codes ativos", "当前 G 代码", "有効な G コード"],
    McActiveM => ["Active M-codes", "M-codes activos", "M-code attivi", "Aktive M-Codes", "M-codes actifs", "M-codes ativos", "当前 M 代码", "有効な M コード"],
    McSpeed => ["Speed", "Velocidad", "Velocità", "Geschwindigkeit", "Vitesse", "Velocidade", "速度", "速度"],
    McZeroAxis => ["Set the zero of this axis here", "Poner aquí el cero de este eje", "Imposta qui lo zero di questo asse", "Nullpunkt dieser Achse hier setzen", "Fixer ici le zéro de cet axe", "Pôr aqui o zero deste eixo", "在此设置该轴的零点", "この軸のゼロをここに設定"],
    McInputs => ["Inputs", "Entradas", "Ingressi", "Eingänge", "Entrées", "Entradas", "输入", "入力"],
    McOutputs => ["Outputs", "Salidas", "Uscite", "Ausgänge", "Sorties", "Saídas", "输出", "出力"],
    McJogSpeed => ["Jog speed", "Velocidad de jog", "Velocità di jog", "Jog-Geschwindigkeit", "Vitesse de jog", "Velocidade de jog", "点动速度", "ジョグ速度"],
    MenuFile => ["File", "Archivo", "File", "Datei", "Fichier", "Ficheiro", "文件", "ファイル"],
    MenuView => ["View", "Ver", "Visualizza", "Ansicht", "Affichage", "Ver", "视图", "表示"],
    MenuHelp => ["Help", "Ayuda", "Aiuto", "Hilfe", "Aide", "Ajuda", "帮助", "ヘルプ"],
    MenuExit => ["Exit", "Salir", "Esci", "Beenden", "Quitter", "Sair", "退出", "終了"],
    McSaveGcode => ["Save G-code ...", "Guardar G-code ...", "Salva G-code ...", "G-Code speichern ...", "Enregistrer le G-code ...", "Guardar G-code ...", "保存 G 代码 ...", "G コードを保存 ..."],
    McEditProgram => ["Edit the program ...", "Editar el programa ...", "Modifica il programma ...", "Programm bearbeiten ...", "Modifier le programme ...", "Editar o programa ...", "编辑程序 ...", "プログラムを編集 ..."],
    McRewind => ["Rewind to the first line", "Volver a la primera línea", "Torna alla prima riga", "Zur ersten Zeile zurück", "Retour à la première ligne", "Voltar à primeira linha", "回到第一行", "最初の行に戻る"],
    McClearProgram => ["Clear the program", "Borrar el programa", "Cancella il programma", "Programm löschen", "Effacer le programme", "Apagar o programa", "清除程序", "プログラムを消去"],
    McFitTable => ["Fit the table", "Ajustar a la mesa", "Adatta al piano", "An den Tisch anpassen", "Ajuster à la table", "Ajustar à mesa", "适应工作台", "テーブルに合わせる"],
    McFitProgram => ["Fit the program", "Ajustar al programa", "Adatta al programma", "An das Programm anpassen", "Ajuster au programme", "Ajustar ao programa", "适应程序", "プログラムに合わせる"],
    McShowButtons => ["Show the buttons of the zeros", "Mostrar los botones de ceros", "Mostra i pulsanti degli zeri", "Nullpunkt-Tasten zeigen", "Afficher les boutons de zéros", "Mostrar os botões de zeros", "显示零点按钮", "ゼロボタンを表示"],
    McShowProgram => ["Show the program", "Mostrar el programa", "Mostra il programma", "Programm zeigen", "Afficher le programme", "Mostrar o programa", "显示程序", "プログラムを表示"],
    McShowLog => ["Show the log", "Mostrar el registro", "Mostra il registro", "Protokoll zeigen", "Afficher le journal", "Mostrar o registo", "显示日志", "ログを表示"],
    McShowRapids => ["Show the rapid moves", "Mostrar los movimientos rápidos", "Mostra i movimenti rapidi", "Eilgänge zeigen", "Afficher les déplacements rapides", "Mostrar os movimentos rápidos", "显示快速移动", "早送りを表示"],
    McSpindleOn => ["Spindle on (M3)", "Husillo en marcha (M3)", "Mandrino acceso (M3)", "Spindel ein (M3)", "Broche en marche (M3)", "Fuso ligado (M3)", "主轴开 (M3)", "主軸オン (M3)"],
    McSpindleOff => ["Spindle off (M5)", "Husillo parado (M5)", "Mandrino spento (M5)", "Spindel aus (M5)", "Broche arrêtée (M5)", "Fuso desligado (M5)", "主轴关 (M5)", "主軸オフ (M5)"],
    McCoolantOn => ["Coolant on (M8)", "Refrigerante activo (M8)", "Refrigerante acceso (M8)", "Kühlmittel ein (M8)", "Arrosage en marche (M8)", "Refrigerante ligado (M8)", "冷却液开 (M8)", "クーラントオン (M8)"],
    McCoolantOff => ["Coolant off (M9)", "Refrigerante parado (M9)", "Refrigerante spento (M9)", "Kühlmittel aus (M9)", "Arrosage arrêté (M9)", "Refrigerante desligado (M9)", "冷却液关 (M9)", "クーラントオフ (M9)"],
    McSetG28 => ["Store this position as G28", "Guardar esta posición como G28", "Memorizza questa posizione come G28", "Diese Position als G28 speichern", "Mémoriser cette position comme G28", "Guardar esta posição como G28", "将此位置存为 G28", "この位置を G28 として保存"],
    McSetG30 => ["Store this position as G30", "Guardar esta posición como G30", "Memorizza questa posizione come G30", "Diese Position als G30 speichern", "Mémoriser cette position comme G30", "Guardar esta posição como G30", "将此位置存为 G30", "この位置を G30 として保存"],
    McCancel => ["Cancel", "Cancelar", "Annulla", "Abbrechen", "Annuler", "Cancelar", "取消", "キャンセル"],
    McSecGeneral => ["General", "General", "Generale", "Allgemein", "Général", "Geral", "常规", "一般"],
    McSecInterface => ["User interface", "Interfaz de usuario", "Interfaccia utente", "Benutzeroberfläche", "Interface utilisateur", "Interface do utilizador", "用户界面", "ユーザーインターフェース"],
    McSecAxes => ["Axes and limits", "Ejes y límites", "Assi e limiti", "Achsen und Grenzen", "Axes et limites", "Eixos e limites", "轴和限位", "軸とリミット"],
    McSecJog => ["Jog", "Movimiento manual", "Movimento manuale", "Handfahren", "Déplacement manuel", "Movimento manual", "点动", "ジョグ"],
    McSecServer => ["Server", "Servidor", "Server", "Server", "Serveur", "Servidor", "服务器", "サーバー"],
    McServerAddress => ["Server address (host:port)", "Dirección del servidor (host:puerto)", "Indirizzo del server (host:porta)", "Serveradresse (Host:Port)", "Adresse du serveur (hôte:port)", "Endereço do servidor (anfitrião:porta)", "服务器地址（主机:端口）", "サーバーのアドレス（ホスト:ポート）"],
    McServerToken => ["Access token", "Token de acceso", "Token di accesso", "Zugriffstoken", "Jeton d'accès", "Token de acesso", "访问令牌", "アクセストークン"],
    McServerMachine => ["Machine id on the server", "Id de la máquina en el servidor", "Id della macchina sul server", "Maschinen-ID auf dem Server", "Id de la machine sur le serveur", "Id da máquina no servidor", "服务器上的机器标识", "サーバー上のマシンID"],
    McSendServer => ["Send to the server", "Enviar al servidor", "Invia al server", "An den Server senden", "Envoyer au serveur", "Enviar para o servidor", "发送到服务器", "サーバーへ送信"],
    McUnitInch => ["Inches", "Pulgadas", "Pollici", "Zoll", "Pouces", "Polegadas", "英寸", "インチ"],
    McLimits => ["Travel limits (mm): minimum and maximum of each axis", "Límites de recorrido (mm): mínimo y máximo de cada eje", "Limiti di corsa (mm): minimo e massimo di ogni asse", "Verfahrgrenzen (mm): Minimum und Maximum jeder Achse", "Limites de course (mm) : minimum et maximum de chaque axe", "Limites de curso (mm): mínimo e máximo de cada eixo", "行程限位 (mm)：每个轴的最小和最大值", "移動範囲 (mm)：各軸の最小値と最大値"],
    McDefaultStep => ["Step at the start (mm)", "Paso al empezar (mm)", "Passo all'avvio (mm)", "Schritt beim Start (mm)", "Pas au démarrage (mm)", "Passo ao iniciar (mm)", "启动时的步长 (mm)", "起動時のステップ (mm)"],
    McDefaultFeed => ["Jog speed at the start (mm/min)", "Velocidad de jog al empezar (mm/min)", "Velocità di jog all'avvio (mm/min)", "Jog-Geschwindigkeit beim Start (mm/min)", "Vitesse de jog au démarrage (mm/min)", "Velocidade de jog ao iniciar (mm/min)", "启动时的点动速度 (mm/min)", "起動時のジョグ速度 (mm/min)"],
    McSettingsSaved => ["The settings are saved and take effect now.", "Los ajustes están guardados y se aplican ya.", "Le impostazioni sono salvate e valgono subito.", "Die Einstellungen sind gespeichert und gelten sofort.", "Les réglages sont enregistrés et valent tout de suite.", "As definições estão guardadas e aplicam-se já.", "设置已保存并立即生效。", "設定は保存され、すぐに有効になります。"],
    McFilterGcode => ["G-code files", "Ficheros G-code", "File G-code", "G-Code-Dateien", "Fichiers G-code", "Ficheiros G-code", "G 代码文件", "G コードファイル"],
    McReplaceAsk => ["already exists. Replace it?", "ya existe. ¿Reemplazarlo?", "esiste già. Sostituirlo?", "existiert bereits. Ersetzen?", "existe déjà. Le remplacer ?", "já existe. Substituí-lo?", "已存在。要替换吗？", "は既に存在します。置き換えますか？"],
    McReplace => ["Replace", "Reemplazar", "Sostituisci", "Ersetzen", "Remplacer", "Substituir", "替换", "置き換え"],
    McTypeName => ["Type a file name.", "Escribe un nombre de fichero.", "Digita un nome di file.", "Gib einen Dateinamen ein.", "Tapez un nom de fichier.", "Escreva um nome de ficheiro.", "请输入文件名。", "ファイル名を入力してください。"],
    McNoSuchFile => ["There is no such file.", "No existe ese fichero.", "Il file non esiste.", "Diese Datei gibt es nicht.", "Ce fichier n'existe pas.", "Esse ficheiro não existe.", "没有这个文件。", "そのファイルはありません。"],
    McSaveTitle => ["Save the program", "Guardar el programa", "Salva il programma", "Programm speichern", "Enregistrer le programme", "Guardar o programa", "保存程序", "プログラムを保存"],
    McOpenTitle => ["Open a G-code program", "Abrir un programa G-code", "Apri un programma G-code", "G-Code-Programm öffnen", "Ouvrir un programme G-code", "Abrir um programa G-code", "打开 G 代码程序", "G コードプログラムを開く"],
    McAboutBody => ["The machine-control program of EH3D: it drives a simulated machine for now.", "El programa de control de máquina de EH3D: por ahora maneja una máquina simulada.", "Il programma di controllo macchina di EH3D: per ora pilota una macchina simulata.", "Das Maschinensteuerprogramm von EH3D: vorerst steuert es eine simulierte Maschine.", "Le programme de commande de machine d'EH3D : pour l'instant il pilote une machine simulée.", "O programa de controlo de máquina do EH3D: por agora comanda uma máquina simulada.", "EH3D 的机床控制程序：目前驱动一台模拟机床。", "EH3D のマシン制御プログラム：現時点では模擬マシンを制御します。"],
    FleetTitle => ["Machines and stations", "Máquinas y estaciones", "Macchine e stazioni", "Maschinen und Stationen", "Machines et stations", "Máquinas e estações", "机床和工位", "マシンとステーション"],
    FleetNoStation => ["No station", "Sin estación", "Senza stazione", "Ohne Station", "Sans station", "Sem estação", "无工位", "ステーションなし"],
    FleetWaiting => ["Waiting for the operator", "Esperando al operador", "In attesa dell'operatore", "Wartet auf den Bediener", "En attente de l'opérateur", "À espera do operador", "等待操作员", "オペレーター待ち"],
    FleetReady => ["jobs ready", "trabajos listos", "lavori pronti", "Aufträge bereit", "travaux prêts", "trabalhos prontos", "个作业就绪", "件のジョブが準備済み"],
    FleetUse => ["Use this machine", "Usar esta máquina", "Usa questa macchina", "Diese Maschine verwenden", "Utiliser cette machine", "Usar esta máquina", "使用这台机床", "このマシンを使う"],
    FleetTarget => ["Machine of the jobs", "Máquina de los trabajos", "Macchina dei lavori", "Maschine der Aufträge", "Machine des travaux", "Máquina dos trabalhos", "作业使用的机床", "ジョブのマシン"],
    FleetNoServer => ["The connection is not an EH3D server: choose it in Connection to see the machines.", "La conexión no es un servidor EH3D: elígelo en Conexión para ver las máquinas.", "La connessione non è un server EH3D: sceglilo in Connessione per vedere le macchine.", "Die Verbindung ist kein EH3D-Server: In Verbindung wählen, um die Maschinen zu sehen.", "La connexion n'est pas un serveur EH3D : choisissez-le dans Connexion pour voir les machines.", "A ligação não é um servidor EH3D: escolha-o em Ligação para ver as máquinas.", "当前连接不是 EH3D 服务器：请在“连接”中选择它以查看机床。", "接続は EH3D サーバーではありません。「接続」で選ぶとマシンが表示されます。"],
    FleetRefresh => ["Refresh", "Actualizar", "Aggiorna", "Aktualisieren", "Actualiser", "Atualizar", "刷新", "更新"],
    JobOutsideTable => ["The job does not fit on the table: move the board or make the table bigger", "El trabajo no cabe en la mesa: mueve la placa o agranda la mesa", "Il lavoro non sta sul piano: sposta la piastra o ingrandisci il piano", "Der Auftrag passt nicht auf den Tisch: Platine verschieben oder Tisch vergrößern", "Le travail ne tient pas sur la table : déplacez la carte ou agrandissez la table", "O trabalho não cabe na mesa: mova a placa ou aumente a mesa", "作业超出工作台：请移动电路板或加大工作台", "ジョブがテーブルに収まりません。基板を移動するかテーブルを大きくしてください"],
    LblThickness => ["Thickness (mm)", "Grosor (mm)", "Spessore (mm)", "Dicke (mm)", "Épaisseur (mm)", "Espessura (mm)", "厚度 (mm)", "厚さ (mm)"],
    LblCopper => ["Copper (µm)", "Cobre (µm)", "Rame (µm)", "Kupfer (µm)", "Cuivre (µm)", "Cobre (µm)", "铜箔 (µm)", "銅 (µm)"],
    LblCopperSides => ["Copper sides", "Caras de cobre", "Facce di rame", "Kupferseiten", "Faces cuivrées", "Faces de cobre", "覆铜面", "銅の面"],
    LblOneSide => ["1 side", "1 cara", "1 faccia", "1 Seite", "1 face", "1 face", "单面", "片面"],
    LblTwoSides => ["2 sides", "2 caras", "2 facce", "2 Seiten", "2 faces", "2 faces", "双面", "両面"],
    LblSize => ["Size (mm)", "Tamaño (mm)", "Dimensioni (mm)", "Größe (mm)", "Taille (mm)", "Tamanho (mm)", "尺寸 (mm)", "サイズ (mm)"],
    LblFromFiles => ["From the files", "De los archivos", "Dai file", "Aus den Dateien", "Des fichiers", "Dos ficheiros", "取自文件", "ファイルから"],
    LblMaker => ["Maker", "Fabricante", "Produttore", "Hersteller", "Fabricant", "Fabricante", "厂商", "メーカー"],
    LblWidth => ["Width", "Ancho", "Larghezza", "Breite", "Largeur", "Largura", "宽度", "幅"],
    LblDepth => ["Depth", "Fondo", "Profondità", "Tiefe", "Profondeur", "Profundidade", "深度", "奥行"],
    LblPhotoresist => ["Photosensitive resist", "Resina fotosensible", "Resist fotosensibile", "Fotolack", "Résine photosensible", "Resina fotossensível", "感光胶", "感光レジスト"],
    LblExtraStroke => ["Extra stroke (mm)", "Carrera extra (mm)", "Corsa extra (mm)", "Zusatzhub (mm)", "Course en plus (mm)", "Curso extra (mm)", "额外行程 (mm)", "追加ストローク (mm)"],
    LblFeed => ["Feed (mm/min)", "Avance (mm/min)", "Avanzamento (mm/min)", "Vorschub (mm/min)", "Avance (mm/min)", "Avanço (mm/min)", "进给 (mm/min)", "送り (mm/min)"],
    LblSpeedRpm => ["Speed (rpm)", "Velocidad (rpm)", "Velocità (giri/min)", "Drehzahl (U/min)", "Vitesse (tr/min)", "Velocidade (rpm)", "转速 (rpm)", "回転数 (rpm)"],
    LblDepthMm => ["Depth (mm)", "Profundidad (mm)", "Profondità (mm)", "Tiefe (mm)", "Profondeur (mm)", "Profundidade (mm)", "深度 (mm)", "深さ (mm)"],
    LblPasses => ["Passes", "Pasadas", "Passate", "Durchgänge", "Passes", "Passagens", "遍数", "パス数"],
    LblFeedXy => ["Feed XY (mm/min)", "Avance XY (mm/min)", "Avanzamento XY (mm/min)", "Vorschub XY (mm/min)", "Avance XY (mm/min)", "Avanço XY (mm/min)", "XY 进给 (mm/min)", "XY 送り (mm/min)"],
    LblFeedZ => ["Feed Z (mm/min)", "Avance Z (mm/min)", "Avanzamento Z (mm/min)", "Vorschub Z (mm/min)", "Avance Z (mm/min)", "Avanço Z (mm/min)", "Z 进给 (mm/min)", "Z 送り (mm/min)"],
    LblDefault => ["Def.", "Def.", "Pred.", "Std.", "Déf.", "Pred.", "默认", "既定"],
    LblFileCoords => ["Coordinates of the files", "Coordenadas de los archivos", "Coordinate dei file", "Koordinaten der Dateien", "Coordonnées des fichiers", "Coordenadas dos ficheiros", "文件坐标", "ファイルの座標"],
    LblNoRack => ["The machine file has no rack positions yet", "El fichero de la máquina aún no tiene posiciones del almacén", "Il file della macchina non ha ancora le posizioni del magazzino", "Die Maschinendatei hat noch keine Magazinpositionen", "Le fichier de la machine n'a pas encore les positions du magasin", "O ficheiro da máquina ainda não tem as posições do carregador", "机器文件中还没有刀库位置", "機械ファイルにはまだラックの位置がありません"],
    LblWorkArea => ["Work area", "Área de trabajo", "Area di lavoro", "Arbeitsbereich", "Zone de travail", "Área de trabalho", "工作区", "作業エリア"],
    LblWholeMachine => ["Whole machine", "Máquina completa", "Macchina intera", "Ganze Maschine", "Machine entière", "Máquina completa", "整机", "機械全体"],
    LblLayers => ["Layers", "Capas", "Livelli", "Ebenen", "Calques", "Camadas", "图层", "レイヤー"],
    SoundFinish => ["Sound when the job ends", "Sonido al terminar el trabajo", "Suono alla fine del lavoro", "Ton am Ende des Auftrags", "Son à la fin du travail", "Som ao terminar o trabalho", "作业结束时的声音", "作業終了時の音"],
    SoundTool => ["Sound when a tool is asked for", "Sonido al pedir una herramienta", "Suono alla richiesta di un utensile", "Ton bei Werkzeuganforderung", "Son à la demande d'un outil", "Som ao pedir uma ferramenta", "请求刀具时的声音", "工具を求めるときの音"],
    SoundCheckOk => ["Sound when the visual check is correct", "Sonido cuando la comprobación visual es correcta", "Suono quando il controllo visivo è corretto", "Ton, wenn die Sichtprüfung in Ordnung ist", "Son quand le contrôle visuel est correct", "Som quando a verificação visual está correta", "目视检查合格时的声音", "目視チェックが合格のときの音"],
    SoundCheckBad => ["Sound when the visual check is not correct", "Sonido cuando la comprobación visual no es correcta", "Suono quando il controllo visivo non è corretto", "Ton, wenn die Sichtprüfung nicht in Ordnung ist", "Son quand le contrôle visuel n'est pas correct", "Som quando a verificação visual não está correta", "目视检查不合格时的声音", "目視チェックが不合格のときの音"],
    SoundFileHint => ["WAV file (empty: the default tone)", "Archivo WAV (vacío: el tono por defecto)", "File WAV (vuoto: il tono predefinito)", "WAV-Datei (leer: der Standardton)", "Fichier WAV (vide : la tonalité par défaut)", "Ficheiro WAV (vazio: o tom predefinido)", "WAV 文件（留空：默认提示音）", "WAV ファイル（空欄：既定の音）"],
    SoundTest => ["Test", "Probar", "Prova", "Testen", "Tester", "Testar", "试听", "試す"],
    SndOff => ["No sound", "Sin sonido", "Nessun suono", "Kein Ton", "Aucun son", "Sem som", "无声", "音なし"],
    SndDefault => ["The standard tone", "El tono estándar", "Il tono standard", "Der Standardton", "La tonalité standard", "O tom padrão", "标准音调", "標準の音"],
    SndFile => ["A file of mine (WAV)", "Un fichero mío (WAV)", "Un mio file (WAV)", "Meine Datei (WAV)", "Mon fichier (WAV)", "Um ficheiro meu (WAV)", "我的文件 (WAV)", "自分のファイル (WAV)"],
    SndChime => ["chime", "carillón", "scampanio", "Glocke", "carillon", "carrilhão", "钟声", "チャイム"],
    SndRise => ["rising notes", "notas que suben", "note che salgono", "steigende Töne", "notes montantes", "notas a subir", "上行音阶", "上がる音"],
    SndFall => ["falling notes", "notas que bajan", "note che scendono", "fallende Töne", "notes descendantes", "notas a descer", "下行音阶", "下がる音"],
    SndFanfare => ["fanfare", "fanfarria", "fanfara", "Fanfare", "fanfare", "fanfarra", "号角", "ファンファーレ"],
    SndTriple => ["three notes", "tres notas", "tre note", "drei Töne", "trois notes", "três notas", "三个音", "3 つの音"],
    SndBell => ["bell", "campana", "campana", "Glocke groß", "cloche", "sino", "大钟", "ベル"],
    SndBeep => ["beep", "pitido", "bip", "Piep", "bip", "apito", "哔声", "ビープ"],
    SndDouble => ["double beep", "pitido doble", "doppio bip", "Doppelpiep", "double bip", "apito duplo", "双哔声", "ダブルビープ"],
    SndTripleBeep => ["triple beep", "pitido triple", "triplo bip", "Dreifachpiep", "triple bip", "apito triplo", "三哔声", "トリプルビープ"],
    SndAlarm => ["alarm", "alarma", "allarme", "Alarm", "alarme", "alarme", "警报", "アラーム"],
    SndKnock => ["knock", "golpe", "colpo", "Klopfen", "coups", "pancada", "敲击", "ノック"],
    SndSiren => ["siren", "sirena", "sirena", "Sirene", "sirène", "sirene", "警笛", "サイレン"],
    HudJob => ["Job", "Trabajo", "Lavoro", "Auftrag", "Travail", "Trabalho", "作业", "ジョブ"],
    HudFeedZ => ["Z feed", "Avance Z", "Avanzamento Z", "Vorschub Z", "Avance Z", "Avanço Z", "Z 进给", "Z 送り"],
    HudSpindle => ["Spindle", "Husillo", "Mandrino", "Spindel", "Broche", "Fuso", "主轴", "主軸"],
    HudTool => ["Tool", "Herramienta", "Utensile", "Werkzeug", "Outil", "Ferramenta", "刀具", "工具"],
    LauncherModernTag => ["Modern", "Moderno", "Moderno", "Modern", "Moderne", "Moderno", "现代版", "モダン"],
    CatSeries => ["series", "serie", "serie", "Serie", "série", "série", "系列", "シリーズ"],
    MakerGeneric => ["Generic", "Genérico", "Generico", "Generisch", "Générique", "Genérico", "通用", "汎用"],
    MatFr4PositiveResist => ["FR4 epoxy, positive photoresist", "FR4 epoxi, fotorresina positiva", "FR4 epossidico, fotoresist positivo", "FR4-Epoxid, Positivlack", "FR4 époxy, résine photosensible positive", "FR4 epóxi, fotorresiste positivo", "FR4 环氧，正性光刻胶", "FR4 エポキシ、ポジ型フォトレジスト"],
    MatFr4Bare => ["FR4 epoxy, bare copper", "FR4 epoxi, cobre desnudo", "FR4 epossidico, rame nudo", "FR4-Epoxid, blankes Kupfer", "FR4 époxy, cuivre nu", "FR4 epóxi, cobre nu", "FR4 环氧，裸铜", "FR4 エポキシ、銅箔のみ"],
    MatPaperResist => ["Laminated paper (FR2), photoresist", "Papel laminado (FR2), fotorresina", "Carta laminata (FR2), fotoresist", "Hartpapier (FR2), Fotolack", "Papier stratifié (FR2), résine photosensible", "Papel laminado (FR2), fotorresiste", "层压纸（FR2），感光胶", "紙基材（FR2）、フォトレジスト付き"],
    MatSheets => ["FR2 / FR3 / CEM-1 sheets", "Planchas FR2 / FR3 / CEM-1", "Lastre FR2 / FR3 / CEM-1", "Platten FR2 / FR3 / CEM-1", "Plaques FR2 / FR3 / CEM-1", "Placas FR2 / FR3 / CEM-1", "FR2 / FR3 / CEM-1 板材", "FR2 / FR3 / CEM-1 シート"],
    MatFr4Series16 => ["FR4 500 series, 1.6 mm", "FR4 serie 500, 1,6 mm", "FR4 serie 500, 1,6 mm", "FR4 Serie 500, 1,6 mm", "FR4 série 500, 1,6 mm", "FR4 série 500, 1,6 mm", "FR4 500 系列，1.6 mm", "FR4 500 シリーズ、1.6 mm"],
    MatFr4Series08 => ["FR4 500 series, 0.8 mm", "FR4 serie 500, 0,8 mm", "FR4 serie 500, 0,8 mm", "FR4 Serie 500, 0,8 mm", "FR4 série 500, 0,8 mm", "FR4 série 500, 0,8 mm", "FR4 500 系列，0.8 mm", "FR4 500 シリーズ、0.8 mm"],
    MatFr4Series600 => ["FR4 600 series, positive presensitized", "FR4 serie 600, presensibilizada positiva", "FR4 serie 600, presensibilizzata positiva", "FR4 Serie 600, positiv vorsensibilisiert", "FR4 série 600, présensibilisée positive", "FR4 série 600, pré-sensibilizada positiva", "FR4 600 系列，正性预涂感光", "FR4 600シリーズ、ポジ感光済み"],
    MatThermalClad => ["Thermal Clad (aluminium)", "Thermal Clad (aluminio)", "Thermal Clad (alluminio)", "Thermal Clad (Aluminium)", "Thermal Clad (aluminium)", "Thermal Clad (alumínio)", "Thermal Clad（铝基）", "Thermal Clad（アルミ）"],
    MatPyralux => ["Pyralux AP flexible", "Pyralux AP flexible", "Pyralux AP flessibile", "Pyralux AP flexibel", "Pyralux AP flexible", "Pyralux AP flexível", "Pyralux AP 柔性板", "Pyralux AP フレキシブル"],
    MatAluminiumSheet => ["Aluminium sheet (5052 / 6061)", "Chapa de aluminio (5052 / 6061)", "Lamiera di alluminio (5052 / 6061)", "Aluminiumblech (5052 / 6061)", "Tôle d'aluminium (5052 / 6061)", "Chapa de alumínio (5052 / 6061)", "铝板（5052 / 6061）", "アルミ板（5052 / 6061）"],
    MatCopperSheet => ["Copper sheet (C110)", "Chapa de cobre (C110)", "Lamiera di rame (C110)", "Kupferblech (C110)", "Tôle de cuivre (C110)", "Chapa de cobre (C110)", "铜板（C110）", "銅板（C110）"],
    MatBirchPlywood => ["Birch plywood", "Contrachapado de abedul", "Compensato di betulla", "Birkensperrholz", "Contreplaqué de bouleau", "Contraplacado de bétula", "桦木胶合板", "バーチ合板"],
    MatBalsa => ["Balsa wood", "Madera de balsa", "Legno di balsa", "Balsaholz", "Bois de balsa", "Madeira de balsa", "轻木", "バルサ材"],
    MatBasswood => ["Basswood (lime)", "Madera de tilo", "Legno di tiglio", "Lindenholz", "Bois de tilleul", "Madeira de tília", "椴木", "シナノキ材"],
    MatPtfeSheet => ["PTFE sheet", "Plancha de PTFE", "Lastra di PTFE", "PTFE-Platte", "Plaque de PTFE", "Placa de PTFE", "PTFE 板", "PTFE 板"],
    MatAcrylic => ["Acrylic (PMMA)", "Acrílico (PMMA)", "Acrilico (PMMA)", "Acryl (PMMA)", "Acrylique (PMMA)", "Acrílico (PMMA)", "亚克力（PMMA）", "アクリル（PMMA）"],
    MatPolycarbonate => ["Polycarbonate (PC)", "Policarbonato (PC)", "Policarbonato (PC)", "Polycarbonat (PC)", "Polycarbonate (PC)", "Policarbonato (PC)", "聚碳酸酯（PC）", "ポリカーボネート（PC）"],
    MatNylon => ["Nylon (PA6)", "Nailon (PA6)", "Nylon (PA6)", "Nylon (PA6)", "Nylon (PA6)", "Nylon (PA6)", "尼龙（PA6）", "ナイロン（PA6）"],
    MatG10Plain => ["G10 (FR4 without copper)", "G10 (FR4 sin cobre)", "G10 (FR4 senza rame)", "G10 (FR4 ohne Kupfer)", "G10 (FR4 sans cuivre)", "G10 (FR4 sem cobre)", "G10（无铜 FR4）", "G10（銅なし FR4）"],
    MatPhenolicPaper => ["Phenolic paper (Pertinax)", "Papel fenólico (Pertinax)", "Carta fenolica (Pertinax)", "Phenolharzpapier (Pertinax)", "Papier phénolique (Pertinax)", "Papel fenólico (Pertinax)", "酚醛纸板（Pertinax）", "フェノール紙（ペルチナックス）"],
    LblNoCopper => ["No copper", "Sin cobre", "Senza rame", "Ohne Kupfer", "Sans cuivre", "Sem cobre", "无铜", "銅なし"],
    LinkNode => ["EH3D node", "Nodo EH3D", "Nodo EH3D", "EH3D-Knoten", "Nœud EH3D", "Nó EH3D", "EH3D 节点", "EH3D ノード"],
    LinkServer => ["EH3D server", "Servidor EH3D", "Server EH3D", "EH3D-Server", "Serveur EH3D", "Servidor EH3D", "EH3D 服务器", "EH3D サーバー"],
    CatTip => ["tip", "punta", "punta", "Spitze", "pointe", "ponta", "刀尖", "先端"],
    LinkDemo => ["Demo", "Demo", "Demo", "Demo", "Démo", "Demo", "演示", "デモ"],
    CatBoardApplied => ["Board material:", "Material de la placa:", "Materiale della scheda:", "Plattenmaterial:", "Matériau de la plaque :", "Material da placa:", "板材：", "基板材料："],
    CatNoteLowerRpm => ["This material wants fewer turns than the bit asks for: the speed was lowered.", "Este material pide menos vueltas que la herramienta: se ha bajado la velocidad.", "Questo materiale chiede meno giri dell'utensile: la velocità è stata ridotta.", "Dieses Material verlangt weniger Umdrehungen als das Werkzeug: die Drehzahl wurde gesenkt.", "Ce matériau demande moins de tours que l'outil : la vitesse a été réduite.", "Este material pede menos rotações do que a ferramenta: a velocidade foi reduzida.", "该材料所需转速低于刀具要求：转速已降低。", "この材料は工具の指定より低い回転数を求めるため、回転数を下げました。"],
    CatNoteBelowSpindle => ["This material wants fewer turns than this spindle can give: the least it turns was used, which is too fast for the material.", "Este material pide menos vueltas de las que puede dar este husillo: se usa el mínimo del husillo, demasiado rápido para el material.", "Questo materiale chiede meno giri di quelli che questo mandrino può dare: si usa il minimo del mandrino, troppo veloce per il materiale.", "Dieses Material verlangt weniger Umdrehungen, als diese Spindel liefern kann: es gilt ihr Minimum, das für das Material zu schnell ist.", "Ce matériau demande moins de tours que cette broche ne peut en donner : le minimum de la broche est utilisé, trop rapide pour le matériau.", "Este material pede menos rotações do que este fuso consegue dar: usa-se o mínimo do fuso, demasiado rápido para o material.", "该材料所需转速低于此主轴的最低转速：已采用主轴最低转速，对该材料来说仍然过快。", "この材料は主軸の最低回転数より低い回転数を求めます。最低回転数を使いますが、材料には速すぎます。"],
    CatNoteFluteShort => ["The cutting length of this bit does not go through the board and the extra: it will not cut all the way through.", "La longitud de corte de esta herramienta no atraviesa la placa más el sobrante: no cortará del todo.", "La lunghezza di taglio di questo utensile non attraversa la scheda più il sovrappiù: non taglierà fino in fondo.", "Die Schneidenlänge dieses Werkzeugs reicht nicht durch die Platte plus Zugabe: es schneidet nicht ganz durch.", "La longueur de coupe de cet outil ne traverse pas la plaque plus le surplus : il ne coupera pas jusqu'au bout.", "O comprimento de corte desta ferramenta não atravessa a placa mais o excedente: não cortará por completo.", "此刀具的切削长度不足以穿透板材加余量：无法完全切穿。", "この工具の刃長は基板と余裕分を貫通しません。完全には切り抜けません。"],
    ToolBitWarn => ["Check the catalogue bit", "Revisar la del catálogo", "Controlla quello del catalogo", "Katalogwerkzeug prüfen", "Vérifier l'outil du catalogue", "Verificar a do catálogo", "请检查目录刀具", "カタログの工具を確認"],
    ClipCut => ["Cut", "Cortar", "Taglia", "Ausschneiden", "Couper", "Cortar", "剪切", "切り取り"],
    ClipCopy => ["Copy", "Copiar", "Copia", "Kopieren", "Copier", "Copiar", "复制", "コピー"],
    ClipPaste => ["Paste", "Pegar", "Incolla", "Einfügen", "Coller", "Colar", "粘贴", "貼り付け"],
    ClipSelectAll => ["Select all", "Seleccionar todo", "Seleziona tutto", "Alles auswählen", "Tout sélectionner", "Selecionar tudo", "全选", "すべて選択"],
    ClipUndo => ["Undo", "Deshacer", "Annulla", "Rückgängig", "Annuler", "Anular", "撤销", "元に戻す"],
    FullScreen => ["Full screen", "Pantalla completa", "Schermo intero", "Vollbild", "Plein écran", "Ecrã inteiro", "全屏", "全画面表示"],
    RackSetupTitle => ["Tool bar", "Barra de herramientas", "Barra utensili", "Werkzeugleiste", "Barre d'outils", "Barra de ferramentas", "刀具排", "工具バー"],
    RackFirst => ["First tool", "Primera herramienta", "Primo utensile", "Erstes Werkzeug", "Premier outil", "Primeira ferramenta", "第一把刀具", "最初の工具"],
    RackLast => ["Last tool", "Última herramienta", "Ultimo utensile", "Letztes Werkzeug", "Dernier outil", "Última ferramenta", "最后一把刀具", "最後の工具"],
    RackToolLength => ["Tool length", "Longitud de la herramienta", "Lunghezza utensile", "Werkzeuglänge", "Longueur de l'outil", "Comprimento da ferramenta", "刀具长度", "工具の長さ"],
    RackRingToTip => ["Ring to tip", "Del anillo a la punta", "Dall'anello alla punta", "Vom Ring zur Spitze", "De l'anneau à la pointe", "Do anel à ponta", "环到刀尖", "リングから先端まで"],
    RackClearance => ["Height above the tools", "Altura sobre las herramientas", "Altezza sopra gli utensili", "Höhe über den Werkzeugen", "Hauteur au-dessus des outils", "Altura sobre as ferramentas", "高出刀具的距离", "工具の上の高さ"],
    RackApproach => ["Approach from the side", "Entrada por el lado", "Ingresso dal lato", "Anfahrt von der Seite", "Approche par le côté", "Entrada pelo lado", "从侧面进入", "横から進入"],
    RackSetupNote => ["The places between the first and the last are evenly spaced. This has not been tried on a real machine: check each point with the spindle stopped.", "Los puestos entre el primero y el último se reparten a partes iguales. No se ha probado en una máquina real: comprueba cada punto con el husillo parado.", "I posti tra il primo e l'ultimo sono distribuiti in modo uniforme. Non è stato provato su una macchina reale: controlla ogni punto con il mandrino fermo.", "Die Plätze zwischen dem ersten und dem letzten sind gleichmäßig verteilt. An einer echten Maschine nicht erprobt: Jeden Punkt bei stehender Spindel prüfen.", "Les places entre la première et la dernière sont réparties à égale distance. Non essayé sur une vraie machine : vérifiez chaque point broche à l'arrêt.", "Os lugares entre o primeiro e o último ficam igualmente espaçados. Não foi testado numa máquina real: verifique cada ponto com o fuso parado.", "第一个与最后一个刀位之间的位置等距分布。尚未在真实机床上试过：请在主轴停止时检查每个点。", "最初と最後の位置の間は等間隔に配置されます。実機では未検証です。主軸を止めた状態で各点を確認してください。"],
    RackInHead => ["In the head", "En el cabezal", "Nella testa", "Im Kopf", "Dans la tête", "No cabeçote", "主轴上", "ヘッド内"],
    RackToTake => ["To take", "Para coger", "Da prendere", "Zu holen", "À prendre", "A pegar", "待取", "取る工具"],
    RackToLeave => ["To leave", "Para dejar", "Da lasciare", "Abzulegen", "À déposer", "A deixar", "待放", "戻す位置"],
    RackOutside => ["A point of the bar is outside the reach of the machine", "Un punto de la barra está fuera del alcance de la máquina", "Un punto della barra è fuori dalla portata della macchina", "Ein Punkt der Leiste liegt außerhalb der Reichweite der Maschine", "Un point de la barre est hors de portée de la machine", "Um ponto da barra está fora do alcance da máquina", "刀具排上有一个点超出了机床的行程范围", "バーの1点が機械の可動範囲外です"],
    BoardTooBig => ["Does not fit the table of the active machine, even turned 90 degrees", "No cabe en la mesa de la máquina activa, ni girada 90 grados", "Non entra nel piano della macchina attiva, nemmeno ruotata di 90 gradi", "Passt nicht auf den Tisch der aktiven Maschine, auch nicht um 90 Grad gedreht", "Ne tient pas sur la table de la machine active, même tournée de 90 degrés", "Não cabe na mesa da máquina ativa, nem rodada 90 graus", "放不到当前机器的工作台上，即使旋转 90 度也不行", "90度回転させても、有効な機械のテーブルに収まりません"],
    BoardSwap => ["Swap width and depth", "Intercambiar ancho y fondo", "Scambia larghezza e profondità", "Breite und Tiefe tauschen", "Échanger largeur et profondeur", "Trocar largura e profundidade", "交换宽度和深度", "幅と奥行きを入れ替える"],
    UpdTitle => ["Updates", "Actualizaciones", "Aggiornamenti", "Aktualisierungen", "Mises à jour", "Atualizações", "更新", "アップデート"],
    UpdButton => ["Update", "Actualizar", "Aggiorna", "Aktualisieren", "Mettre à jour", "Atualizar", "更新", "更新"],
    UpdInstalled => ["Installed", "Instalada", "Installata", "Installiert", "Installée", "Instalada", "已安装", "インストール済み"],
    UpdLatest => ["Latest on GitHub", "Última en GitHub", "Ultima su GitHub", "Neueste auf GitHub", "Dernière sur GitHub", "Mais recente no GitHub", "GitHub 上的最新版", "GitHub の最新版"],
    UpdCheck => ["Check now", "Buscar ahora", "Controlla ora", "Jetzt prüfen", "Vérifier maintenant", "Procurar agora", "立即检查", "今すぐ確認"],
    UpdInstall => ["Download and install", "Descargar e instalar", "Scarica e installa", "Laden und installieren", "Télécharger et installer", "Transferir e instalar", "下载并安装", "ダウンロードしてインストール"],
    UpdUndo => ["Undo the update", "Deshacer la actualización", "Annulla l'aggiornamento", "Aktualisierung rückgängig", "Annuler la mise à jour", "Anular a atualização", "撤销更新", "更新を元に戻す"],
    UpdRestart => ["Restart now", "Reiniciar ahora", "Riavvia ora", "Jetzt neu starten", "Redémarrer maintenant", "Reiniciar agora", "立即重启", "今すぐ再起動"],
    UpdNotes => ["What is new", "Novedades", "Novità", "Neuigkeiten", "Nouveautés", "Novidades", "更新内容", "新機能"],
    UpdUpToDate => ["This is the latest version", "Esta es la última versión", "Questa è l'ultima versione", "Dies ist die neueste Version", "C'est la dernière version", "Esta é a versão mais recente", "已是最新版本", "最新バージョンです"],
    UpdAvailable => ["A new version is available", "Hay una versión nueva", "È disponibile una nuova versione", "Eine neue Version ist verfügbar", "Une nouvelle version est disponible", "Há uma nova versão", "有新版本可用", "新しいバージョンがあります"],
    UpdStepAsk => ["Ask GitHub for the release", "Preguntar a GitHub por la versión", "Chiedi la versione a GitHub", "GitHub nach der Version fragen", "Demander la version à GitHub", "Perguntar ao GitHub pela versão", "向 GitHub 查询版本", "GitHub にリリースを問い合わせ"],
    UpdStepDownload => ["Download the package", "Descargar el paquete", "Scarica il pacchetto", "Paket laden", "Télécharger le paquet", "Transferir o pacote", "下载安装包", "パッケージをダウンロード"],
    UpdStepVerify => ["Check the SHA-256", "Comprobar el SHA-256", "Controlla lo SHA-256", "SHA-256 prüfen", "Vérifier le SHA-256", "Verificar o SHA-256", "校验 SHA-256", "SHA-256 を確認"],
    UpdStepInstall => ["Put it in place", "Ponerlo en su sitio", "Mettilo al suo posto", "An die Stelle setzen", "Le mettre en place", "Colocá-lo no lugar", "替换程序", "所定の場所に置く"],
    UpdHistory => ["History", "Historial", "Cronologia", "Verlauf", "Historique", "Histórico", "历史", "履歴"],
    UpdOff => ["Updates are switched off", "Las actualizaciones están desactivadas", "Gli aggiornamenti sono disattivati", "Aktualisierungen sind ausgeschaltet", "Les mises à jour sont désactivées", "As atualizações estão desativadas", "更新已关闭", "アップデートはオフです"],
    UpdAllow => ["Allow updates from GitHub", "Permitir actualizar desde GitHub", "Consenti aggiornamenti da GitHub", "Aktualisierung von GitHub erlauben", "Autoriser la mise à jour depuis GitHub", "Permitir atualizar a partir do GitHub", "允许从 GitHub 更新", "GitHub からの更新を許可"],
    UpdAtStart => ["Look for a new version when the program starts", "Buscar una versión nueva al arrancar", "Cerca una nuova versione all'avvio", "Beim Start nach neuer Version suchen", "Chercher une nouvelle version au démarrage", "Procurar nova versão ao arrancar", "启动时检查新版本", "起動時に新しいバージョンを確認"],
    UpdNoPackage => ["This release has no package for this system", "Esta versión no trae paquete para este sistema", "Questa versione non ha un pacchetto per questo sistema", "Diese Version hat kein Paket für dieses System", "Cette version n'a pas de paquet pour ce système", "Esta versão não traz pacote para este sistema", "此版本没有适用于本系统的安装包", "このリリースにはこのシステム用のパッケージがありません"],
    UpdFailed => ["The update failed", "La actualización falló", "L'aggiornamento non è riuscito", "Die Aktualisierung ist fehlgeschlagen", "La mise à jour a échoué", "A atualização falhou", "更新失败", "アップデートに失敗しました"],
    UpdDone => ["Done. Restart the program to use it", "Hecho. Reinicia el programa para usarla", "Fatto. Riavvia il programma per usarla", "Fertig. Programm neu starten, um sie zu nutzen", "Terminé. Redémarrez le programme pour l'utiliser", "Concluído. Reinicie o programa para a usar", "完成。请重启程序以使用新版本", "完了しました。使うにはプログラムを再起動してください"],
    UpdPrivacy => ["Nothing is installed without your say. Only the release and its package are asked for; nothing about you or your machine is sent.", "No se instala nada sin que lo pidas. Solo se piden la versión y su paquete; no se envía nada de ti ni de tu máquina.", "Non si installa nulla senza il tuo consenso. Si chiedono solo la versione e il suo pacchetto; non si invia nulla su di te o sulla tua macchina.", "Ohne Ihre Zustimmung wird nichts installiert. Es werden nur die Version und ihr Paket abgerufen; nichts über Sie oder Ihre Maschine wird gesendet.", "Rien n'est installé sans votre accord. Seuls la version et son paquet sont demandés ; rien sur vous ni sur votre machine n'est envoyé.", "Nada é instalado sem o seu pedido. Só se pedem a versão e o seu pacote; nada sobre si ou a sua máquina é enviado.", "未经你同意不会安装任何内容。只会请求版本及其安装包，不会发送任何关于你或你的机器的信息。", "あなたの操作なしに何もインストールされません。リリースとそのパッケージを取得するだけで、あなたやマシンの情報は送信されません。"],
    UpdHistUpdated => ["Updated", "Actualizada", "Aggiornato", "Aktualisiert", "Mise à jour", "Atualizada", "已更新", "更新しました"],
    UpdHistUndone => ["Went back", "Se volvió atrás", "Tornato indietro", "Zurückgesetzt", "Retour arrière", "Voltou atrás", "已回退", "元に戻しました"],
    TableNoun => ["Table", "Mesa", "Piano", "Tisch", "Table", "Mesa", "工作台", "テーブル"],
    AllMakers => ["All makers", "Todos los fabricantes", "Tutti i produttori", "Alle Hersteller", "Tous les fabricants", "Todos os fabricantes", "所有制造商", "すべてのメーカー"],

    SummaryTitle => ["Check before starting", "Revisa antes de empezar", "Controlla prima di avviare", "Vor dem Start prüfen", "Vérifiez avant de démarrer", "Verifique antes de iniciar", "开始前请检查", "開始前に確認"],
    SummaryHoles => ["Holes", "Agujeros", "Fori", "Bohrungen", "Trous", "Furos", "孔数", "穴数"],
    SummaryTools => ["Tools", "Herramientas", "Utensili", "Werkzeuge", "Outils", "Ferramentas", "刀具", "工具"],
    SummarySize => ["Size (mm)", "Tamaño (mm)", "Dimensioni (mm)", "Größe (mm)", "Taille (mm)", "Tamanho (mm)", "尺寸 (mm)", "サイズ (mm)"],
    SummaryTime => ["Estimated time", "Tiempo estimado", "Tempo stimato", "Geschätzte Zeit", "Durée estimée", "Tempo estimado", "预计时间", "推定時間"],
    SummaryReach => ["Reach of the axes (mm)", "Alcance de los ejes (mm)", "Escursione degli assi (mm)", "Reichweite der Achsen (mm)", "Course des axes (mm)", "Alcance dos eixos (mm)", "各轴行程范围 (mm)", "各軸の到達範囲 (mm)"],
    WarnTiny => ["The whole work is only {} x {} mm: check the unit of the file", "Todo el trabajo mide solo {} x {} mm: revisa la unidad del archivo", "Tutto il lavoro misura solo {} x {} mm: controlla l'unità del file", "Die gesamte Arbeit misst nur {} x {} mm: Einheit der Datei prüfen", "Tout le travail ne mesure que {} x {} mm : vérifiez l'unité du fichier", "Todo o trabalho mede apenas {} x {} mm: verifique a unidade do ficheiro", "整个作业只有 {} x {} mm：请检查文件的单位", "作業全体が {} x {} mm しかありません：ファイルの単位を確認してください"],
    WarnBig => ["The work ({} x {} mm) is bigger than the table ({} x {} mm)", "El trabajo ({} x {} mm) es mayor que la mesa ({} x {} mm)", "Il lavoro ({} x {} mm) è più grande del piano ({} x {} mm)", "Die Arbeit ({} x {} mm) ist größer als der Tisch ({} x {} mm)", "Le travail ({} x {} mm) est plus grand que la table ({} x {} mm)", "O trabalho ({} x {} mm) é maior do que a mesa ({} x {} mm)", "作业 ({} x {} mm) 大于工作台 ({} x {} mm)", "作業 ({} x {} mm) がテーブル ({} x {} mm) より大きいです"],
    WarnFar => ["The work starts {} mm from the corner of the table: check the offset and the unit", "El trabajo empieza a {} mm de la esquina de la mesa: revisa el desplazamiento y la unidad", "Il lavoro inizia a {} mm dall'angolo del piano: controlla lo scostamento e l'unità", "Die Arbeit beginnt {} mm von der Tischecke entfernt: Versatz und Einheit prüfen", "Le travail commence à {} mm du coin de la table : vérifiez le décalage et l'unité", "O trabalho começa a {} mm do canto da mesa: verifique o desvio e a unidade", "作业起点距工作台角落 {} mm：请检查偏移和单位", "作業はテーブルの角から {} mm の位置から始まります：オフセットと単位を確認してください"],
    WarnScale => ["The machine file stretches the work (scale {} x {}, not 1)", "El archivo de la máquina estira el trabajo (escala {} x {}, no 1)", "Il file della macchina deforma il lavoro (scala {} x {}, non 1)", "Die Maschinendatei streckt die Arbeit (Maßstab {} x {}, nicht 1)", "Le fichier de la machine déforme le travail (échelle {} x {}, pas 1)", "O ficheiro da máquina estica o trabalho (escala {} x {}, não 1)", "机器文件会拉伸作业（比例 {} x {}，不是 1）", "機械ファイルが作業を伸縮します（倍率 {} x {}、1 ではありません）"],
    DryOut => ["The plan takes axis {} {} mm past its course", "El plan lleva el eje {} {} mm más allá de su recorrido", "Il piano porta l'asse {} {} mm oltre la sua corsa", "Der Plan fährt Achse {} {} mm über ihren Verfahrweg hinaus", "Le plan emmène l'axe {} {} mm au-delà de sa course", "O plano leva o eixo {} {} mm além do seu curso", "计划使 {} 轴超出行程 {} mm", "計画では {} 軸が可動範囲を {} mm 超えます"],
    DryOk => ["Dry run: the exact plan stays inside the course of every axis", "Simulación en seco: el plan exacto queda dentro del recorrido de cada eje", "Prova a secco: il piano esatto resta entro la corsa di ogni asse", "Trockenlauf: Der genaue Plan bleibt im Verfahrweg jeder Achse", "Essai à blanc : le plan exact reste dans la course de chaque axe", "Simulação em seco: o plano exato fica dentro do curso de cada eixo", "空跑：精确计划保持在每个轴的行程之内", "ドライラン：正確な計画はすべての軸の可動範囲内に収まります"],
    McSumMoves => ["Moves (rapid / cutting)", "Movimientos (rápidos / de corte)", "Movimenti (rapidi / di taglio)", "Bewegungen (Eilgang / Schnitt)", "Mouvements (rapides / de coupe)", "Movimentos (rápidos / de corte)", "移动次数（快速 / 切削）", "移動回数（早送り / 切削）"],
    McSumCut => ["Cutting distance (mm)", "Distancia de corte (mm)", "Distanza di taglio (mm)", "Schnittweg (mm)", "Distance de coupe (mm)", "Distância de corte (mm)", "切削距离 (mm)", "切削距離 (mm)"],
    McSumArcs => ["{} arc moves (G2/G3) are not counted in the time or the reach", "{} movimientos en arco (G2/G3) no se cuentan en el tiempo ni en el alcance", "{} movimenti ad arco (G2/G3) non sono contati nel tempo né nell'escursione", "{} Kreisbewegungen (G2/G3) werden bei Zeit und Reichweite nicht mitgezählt", "{} mouvements en arc (G2/G3) ne sont pas comptés dans la durée ni dans la course", "{} movimentos em arco (G2/G3) não são contados no tempo nem no alcance", "{} 个圆弧移动 (G2/G3) 不计入时间和行程范围", "{} 件の円弧移動 (G2/G3) は時間と到達範囲に含まれません"],
    McSumNoFeed => ["{} cutting moves come before any feed rate (F): the feed of the jog is assumed", "{} movimientos de corte van antes de cualquier avance (F): se supone el avance del jog", "{} movimenti di taglio vengono prima di qualsiasi avanzamento (F): si assume l'avanzamento del jog", "{} Schnittbewegungen stehen vor jedem Vorschub (F): der Vorschub des Handverfahrens wird angenommen", "{} mouvements de coupe précèdent toute avance (F) : l'avance du jog est supposée", "{} movimentos de corte vêm antes de qualquer avanço (F): assume-se o avanço do jog", "{} 个切削移动出现在任何进给速度 (F) 之前：按点动进给速度计算", "{} 件の切削移動が送り速度 (F) の指定より前にあります：ジョグの送り速度を仮定します"],
    McSumStartAnyway => ["Start anyway", "Empezar de todos modos", "Avvia comunque", "Trotzdem starten", "Démarrer quand même", "Iniciar mesmo assim", "仍然开始", "それでも開始"],
    McConfirmStart => ["Show the summary before a program starts", "Mostrar el resumen antes de empezar un programa", "Mostra il riepilogo prima di avviare un programma", "Zusammenfassung vor dem Start eines Programms zeigen", "Afficher le résumé avant le démarrage d'un programme", "Mostrar o resumo antes de iniciar um programa", "程序开始前显示摘要", "プログラム開始前に概要を表示"],
    McRapidFeed => ["Rapid speed (mm/min)", "Velocidad rápida (mm/min)", "Velocità rapida (mm/min)", "Eilganggeschwindigkeit (mm/min)", "Vitesse rapide (mm/min)", "Velocidade rápida (mm/min)", "快速移动速度 (mm/min)", "早送り速度 (mm/min)"],
    McShortcuts => ["Keyboard shortcuts", "Atajos de teclado", "Scorciatoie da tastiera", "Tastenkürzel", "Raccourcis clavier", "Atalhos de teclado", "键盘快捷键", "キーボードショートカット"],
    McShowToolbar => ["Toolbar", "Barra de herramientas", "Barra degli strumenti", "Werkzeugleiste", "Barre d'outils", "Barra de ferramentas", "工具栏", "ツールバー"],
    McOpenFailed => ["The file cannot be opened: {}", "No se puede abrir el archivo: {}", "Impossibile aprire il file: {}", "Die Datei kann nicht geöffnet werden: {}", "Le fichier ne peut pas être ouvert : {}", "Não é possível abrir o ficheiro: {}", "无法打开文件：{}", "ファイルを開けません：{}"],
    McSecSafety => ["Safety", "Seguridad", "Sicurezza", "Sicherheit", "Sécurité", "Segurança", "安全", "安全"],
    McSumReject => ["Refuse a program that leaves the travel of the machine", "Rechazar un programa que sale del recorrido de la máquina", "Rifiuta un programma che esce dalla corsa della macchina", "Programm ablehnen, das den Verfahrweg der Maschine verlässt", "Refuser un programme qui sort de la course de la machine", "Recusar um programa que sai do curso da máquina", "拒绝超出机器行程的程序", "機械の可動範囲を超えるプログラムを拒否する"],
    McQueue => ["Queue", "Cola", "Coda", "Warteschlange", "File d'attente", "Fila", "队列", "キュー"],
    McQueueWaiting => ["Waiting", "En espera", "In attesa", "Wartet", "En attente", "A aguardar", "等待中", "待機中"],
    McQueueDone => ["Done", "Hecho", "Fatto", "Fertig", "Terminé", "Concluído", "已完成", "完了"],
    McQueueFailed => ["Not finished", "Sin terminar", "Non terminato", "Nicht beendet", "Non terminé", "Não terminado", "未完成", "未完了"],
    McQueueSkipped => ["Skipped", "Omitido", "Saltato", "Übersprungen", "Ignoré", "Ignorado", "已跳过", "スキップ済み"],
    McQueueRemove => ["Remove from the queue", "Quitar de la cola", "Rimuovi dalla coda", "Aus der Warteschlange entfernen", "Retirer de la file", "Retirar da fila", "从队列中移除", "キューから削除"],
    McQueueSkip => ["Skip this program", "Saltar este programa", "Salta questo programma", "Dieses Programm überspringen", "Ignorer ce programme", "Ignorar este programa", "跳过此程序", "このプログラムをスキップ"],
    McQueueClear => ["Clear the queue", "Vaciar la cola", "Svuota la coda", "Warteschlange leeren", "Vider la file", "Esvaziar a fila", "清空队列", "キューを空にする"],
    McQueueContinue => ["Go on with the queue", "Seguir con la cola", "Continua con la coda", "Warteschlange fortsetzen", "Reprendre la file", "Continuar com a fila", "继续队列", "キューを続ける"],
    McQueueHalted => ["The queue stopped: a program did not finish", "La cola se detuvo: un programa no terminó", "La coda si è fermata: un programma non è terminato", "Die Warteschlange hält an: ein Programm wurde nicht beendet", "La file s'est arrêtée : un programme ne s'est pas terminé", "A fila parou: um programa não terminou", "队列已停止：有程序未完成", "キューが停止しました：未完了のプログラムがあります"],
    McGrblTravel => ["Read the travel of the machine from the controller and use the smaller one", "Leer el recorrido de la máquina del controlador y usar el menor", "Leggi la corsa della macchina dal controllore e usa la minore", "Verfahrweg der Maschine von der Steuerung lesen und den kleineren verwenden", "Lire la course de la machine depuis le contrôleur et utiliser la plus petite", "Ler o curso da máquina do controlador e usar o menor", "从控制器读取机器行程并使用较小的值", "コントローラーから機械のストロークを読み取り、小さい方を使う"],
    MachineNameLbl => ["Machine name", "Nombre de la máquina", "Nome della macchina", "Maschinenname", "Nom de la machine", "Nome da máquina", "机器名称", "機械名"],
    MachineMakeLbl => ["Make and model", "Marca y modelo", "Marca e modello", "Hersteller und Modell", "Marque et modèle", "Marca e modelo", "品牌和型号", "メーカーと型式"],
    SpindleCopyNote => ["Provisional values copied from the CCD MTC (KaVo 4010 spindle): there is no official data for this model yet. They are kept apart for this machine and changing them here does not touch any other model.", "Valores provisionales copiados de la CCD MTC (husillo KaVo 4010): aún no hay datos oficiales de este modelo. Se guardan aparte para esta máquina y cambiarlos aquí no toca ningún otro modelo.", "Valori provvisori copiati dalla CCD MTC (mandrino KaVo 4010): per questo modello non ci sono ancora dati ufficiali. Sono conservati a parte per questa macchina e modificarli qui non tocca nessun altro modello.", "Vorläufige Werte, von der CCD MTC kopiert (Spindel KaVo 4010): Für dieses Modell gibt es noch keine offiziellen Daten. Sie werden für diese Maschine getrennt gespeichert; eine Änderung hier betrifft kein anderes Modell.", "Valeurs provisoires copiées de la CCD MTC (broche KaVo 4010) : il n'y a pas encore de données officielles pour ce modèle. Elles sont conservées à part pour cette machine et les modifier ici ne touche aucun autre modèle.", "Valores provisórios copiados da CCD MTC (fuso KaVo 4010): ainda não há dados oficiais deste modelo. Ficam guardados à parte para esta máquina e alterá-los aqui não afeta nenhum outro modelo.", "临时值，复制自 CCD MTC（KaVo 4010 主轴）：该型号暂无官方数据。它们为本机单独保存，在此修改不会影响其他型号。", "CCD MTC（KaVo 4010 スピンドル）からコピーした暫定値です。この機種の公式データはまだありません。この機械用に別保存され、ここで変更しても他の機種には影響しません。"],
    DesignLocked => ["Design locked while the machine works", "Diseño bloqueado mientras la máquina trabaja", "Disegno bloccato mentre la macchina lavora", "Design gesperrt, solange die Maschine arbeitet", "Conception verrouillée pendant que la machine travaille", "Desenho bloqueado enquanto a máquina trabalha", "机器工作时设计已锁定", "機械の稼働中はデザインをロック"],
    HudHolesLbl => ["holes", "agujeros", "fori", "Bohrungen", "trous", "furos", "孔", "穴"],
    HudSegmentsLbl => ["segments", "tramos", "tratti", "Abschnitte", "tronçons", "troços", "段", "区間"],
    HudNext => ["Next", "Siguiente", "Prossimo", "Nächstes", "Suivant", "Seguinte", "下一个", "次"],
    HudToolsLbl => ["tools", "herramientas", "utensili", "Werkzeuge", "outils", "ferramentas", "刀具", "工具"],
    HudLeft => ["left", "restante", "restano", "verbleibend", "restant", "restante", "剩余", "残り"],
    HudChanging => ["Changing tool", "Cambiando herramienta", "Cambio utensile", "Werkzeugwechsel", "Changement d'outil", "A trocar a ferramenta", "正在换刀", "工具交換中"],
    HudKindDrill => ["drill", "broca", "punta", "Bohrer", "foret", "broca", "钻头", "ドリル"],
    HudKindCutter => ["cutter", "fresa", "fresa", "Fräser", "fraise", "fresa", "铣刀", "カッター"],
    HudRackSlot => ["rack", "rack", "rack", "Magazin", "magasin", "rack", "刀库", "ラック"],
    MatNoData => ["No manufacturer data", "Sin datos del fabricante", "Senza dati del produttore", "Keine Herstellerangaben", "Sans données du fabricant", "Sem dados do fabricante", "无厂商数据", "メーカーのデータなし"],
    MatTypeAll => ["All types", "Todos los tipos", "Tutti i tipi", "Alle Typen", "Tous les types", "Todos os tipos", "所有类型", "すべての種類"],
    MatTypeLaminate => ["Laminate", "Laminado", "Laminato", "Laminat", "Stratifié", "Laminado", "层压板", "ラミネート"],
    MatTypeRf => ["PTFE / RF", "PTFE / RF", "PTFE / RF", "PTFE / HF", "PTFE / RF", "PTFE / RF", "PTFE / 射频", "PTFE / 高周波"],
    MatTypeCeramic => ["Ceramic filled", "Con carga cerámica", "Caricato ceramica", "Keramikgefüllt", "Chargé céramique", "Com carga cerâmica", "陶瓷填充", "セラミック充填"],
    MatTypeMetal => ["Metal core", "Núcleo metálico", "Anima metallica", "Metallkern", "Âme métallique", "Núcleo metálico", "金属基", "金属コア"],
    MatTypeSheet => ["Plain sheets", "Planchas lisas", "Lastre lisce", "Platten ohne Kupfer", "Plaques nues", "Chapas lisas", "无铜板材", "無銅の板材"],
    MatSearch => ["Search", "Buscar", "Cerca", "Suchen", "Rechercher", "Pesquisar", "搜索", "検索"],
    MatNoMatch => ["No material matches the filters", "Ningún material cumple los filtros", "Nessun materiale corrisponde ai filtri", "Kein Material passt zu den Filtern", "Aucun matériau ne correspond aux filtres", "Nenhum material corresponde aos filtros", "没有符合筛选条件的材料", "条件に合う材料がありません"],
    MatAskValues => ["The manufacturer gives no thickness or copper for this material: type the values of your board", "El fabricante no da espesor ni cobre para este material: escribe los valores de tu placa", "Il produttore non indica spessore né rame per questo materiale: digita i valori della tua scheda", "Der Hersteller nennt für dieses Material keine Dicke und kein Kupfer: Werte der eigenen Platine eingeben", "Le fabricant n'indique ni épaisseur ni cuivre pour ce matériau : saisissez les valeurs de votre carte", "O fabricante não indica espessura nem cobre para este material: escreva os valores da sua placa", "厂商未提供此材料的厚度和铜厚：请输入您板子的数值", "このメーカーはこの材料の厚みと銅厚を示していません。お使いの基板の値を入力してください"],
    MatUsual => ["Usual values (not from the manufacturer)", "Valores habituales (no son del fabricante)", "Valori usuali (non del produttore)", "Übliche Werte (nicht vom Hersteller)", "Valeurs usuelles (pas du fabricant)", "Valores habituais (não são do fabricante)", "常用值（非厂商数据）", "一般的な値（メーカーの値ではありません）"],
    JogTitle => ["Move the machine", "Mover la máquina", "Muovi la macchina", "Maschine bewegen", "Déplacer la machine", "Mover a máquina", "移动机床", "機械を動かす"],
    JogStep => ["Step", "Paso", "Passo", "Schritt", "Pas", "Passo", "步距", "ステップ"],
    JogContinuous => ["Continuous", "Continuo", "Continuo", "Kontinuierlich", "Continu", "Contínuo", "连续", "連続"],
    JogSpeed => ["Speed", "Velocidad", "Velocità", "Geschwindigkeit", "Vitesse", "Velocidade", "速度", "速度"],
    JogSlow => ["Slow", "Lenta", "Lenta", "Langsam", "Lente", "Lenta", "慢", "低速"],
    JogMedium => ["Medium", "Media", "Media", "Mittel", "Moyenne", "Média", "中", "中速"],
    JogFast => ["Fast", "Rápida", "Veloce", "Schnell", "Rapide", "Rápida", "快", "高速"],
    JogMachine => ["Machine", "Máquina", "Macchina", "Maschine", "Machine", "Máquina", "机床", "機械"],
    JogWork => ["Work", "Trabajo", "Lavoro", "Werkstück", "Travail", "Trabalho", "工件", "ワーク"],
    JogSetOrigin => ["Work origin here", "Origen de trabajo aquí", "Origine del lavoro qui", "Werkstücknullpunkt hier", "Origine du travail ici", "Origem do trabalho aqui", "在此设置工件原点", "ここをワーク原点に"],
    JogHome => ["Home", "Inicio", "Home", "Referenzfahrt", "Origine machine", "Início", "回原点", "原点復帰"],
    JogSaved => ["Saved positions", "Posiciones guardadas", "Posizioni salvate", "Gespeicherte Positionen", "Positions enregistrées", "Posições guardadas", "已保存位置", "保存した位置"],
    JogSave => ["Save", "Guardar", "Salva", "Speichern", "Enregistrer", "Guardar", "保存", "保存"],
    JogGo => ["Go", "Ir", "Vai", "Hin", "Aller", "Ir", "前往", "移動"],
    JogBusyJob => ["A job is running: stop it before moving the machine", "Hay un trabajo en marcha: páralo antes de mover la máquina", "Un lavoro è in corso: fermalo prima di muovere la macchina", "Ein Auftrag läuft: vor dem Verfahren der Maschine anhalten", "Un travail est en cours : arrêtez-le avant de déplacer la machine", "Há um trabalho em curso: pare-o antes de mover a máquina", "作业正在运行：请先停止再移动机床", "作業の実行中です。機械を動かす前に停止してください"],
    JogBusyStop => ["The emergency stop is on", "La parada de emergencia está activada", "L'arresto di emergenza è attivo", "Der Not-Halt ist aktiv", "L'arrêt d'urgence est activé", "A paragem de emergência está ativa", "急停已激活", "非常停止が有効です"],
    JogLimit => ["End of travel", "Fin de recorrido", "Fine corsa", "Ende des Verfahrwegs", "Fin de course", "Fim de curso", "行程终点", "可動範囲の端"],
    JogRaiseZ => ["Raise Z first: the head is too low to move across", "Sube Z antes: el cabezal está demasiado bajo para desplazarse", "Alza prima Z: la testa è troppo bassa per spostarsi", "Zuerst Z anheben: Der Kopf ist zum Verfahren zu tief", "Relevez d'abord Z : la tête est trop basse pour se déplacer", "Suba primeiro o Z: a cabeça está demasiado baixa para se deslocar", "请先抬高 Z：刀头太低，无法横向移动", "先にZを上げてください。ヘッドが低すぎて横に動けません"],
    JogLive => ["Real machine", "Máquina real", "Macchina reale", "Echte Maschine", "Machine réelle", "Máquina real", "真实机床", "実機"],
    JogDemo => ["Simulated head", "Cabezal simulado", "Testa simulata", "Simulierter Kopf", "Tête simulée", "Cabeça simulada", "模拟刀头", "シミュレーションのヘッド"],
    StageDoneTitle => ["The drilling is done. Now comes the routing", "El trabajo de taladros está hecho. Ahora toca el de fresado", "La foratura è terminata. Ora tocca alla fresatura", "Das Bohren ist fertig. Jetzt folgt das Fräsen", "Le perçage est terminé. Place au fraisage", "A furação está concluída. Agora segue-se a fresagem", "钻孔已完成，接下来进行铣削", "穴あけが完了しました。次はルーター加工です"],
    StageFollow => ["Follow the routing process", "Seguir el proceso de fresado", "Segui il processo di fresatura", "Fräsvorgang verfolgen", "Suivre le processus de fraisage", "Acompanhar o processo de fresagem", "查看铣削过程", "ルーター加工の進行を見る"],
    JobFinishedRemove => ["The job has finished. You may remove the board.", "El trabajo ha terminado. Puede retirar la placa.", "Il lavoro è terminato. Puoi rimuovere la scheda.", "Der Auftrag ist fertig. Sie können die Platine entnehmen.", "Le travail est terminé. Vous pouvez retirer la carte.", "O trabalho terminou. Pode retirar a placa.", "作业已完成，可以取下电路板。", "作業が完了しました。基板を取り外せます。"],
    HudBlkTool => ["Tool and step", "Herramienta y paso", "Utensile e passo", "Werkzeug und Schritt", "Outil et étape", "Ferramenta e passo", "刀具与步骤", "工具とステップ"],
    HudBlkTime => ["Time", "Tiempos", "Tempi", "Zeiten", "Temps", "Tempos", "时间", "時間"],
    HudBlkPos => ["Position and depth", "Posición y profundidad", "Posizione e profondità", "Position und Tiefe", "Position et profondeur", "Posição e profundidade", "位置与深度", "位置と深さ"],
    HudBlkCount => ["Counters", "Contadores", "Contatori", "Zähler", "Compteurs", "Contadores", "计数", "カウンター"],
    HudElapsed => ["elapsed", "transcurrido", "trascorso", "vergangen", "écoulé", "decorrido", "已用", "経過"],
    HudEndsAt => ["ends at", "fin prevista", "fine prevista", "Ende um", "fin prévue", "fim previsto", "预计结束", "終了予定"],
    HudDepth => ["depth", "profundidad", "profondità", "Tiefe", "profondeur", "profundidade", "深度", "深さ"],
    HudFeed => ["feed", "avance", "avanzamento", "Vorschub", "avance", "avanço", "进给", "送り"],
    HudChanges => ["tool changes", "cambios de herramienta", "cambi utensile", "Werkzeugwechsel", "changements d'outil", "trocas de ferramenta", "换刀次数", "工具交換回数"],
    HudHoleOne => ["hole", "taladro", "foro", "Bohrung", "perçage", "furo", "孔", "穴"],
    HudSegmentOne => ["segment", "tramo", "tratto", "Abschnitt", "tronçon", "troço", "段", "区間"],
    SlotsTitle => ["Positions in the bar", "Posiciones en la barra", "Posizioni nella barra", "Positionen in der Leiste", "Positions dans la barre", "Posições na barra", "刀库中的位置", "ラックの位置"],
    SlotsAuto => ["Assign in order", "Asignar en orden", "Assegna in ordine", "In Reihenfolge zuweisen", "Attribuer dans l'ordre", "Atribuir por ordem", "按顺序分配", "順番に割り当て"],
    SlotsClear => ["Clear assignment", "Borrar asignación", "Cancella assegnazione", "Zuweisung löschen", "Effacer l'attribution", "Apagar atribuição", "清除分配", "割り当てを消去"],
    SlotsNone => ["no slot", "sin hueco", "nessun posto", "kein Platz", "aucune place", "sem posição", "未分配", "未割り当て"],
    SlotsPending => ["pending", "pendiente", "in attesa", "ausstehend", "en attente", "pendente", "待处理", "未実行"],
    SlotsUsing => ["in use", "en uso", "in uso", "in Benutzung", "en cours", "em uso", "使用中", "使用中"],
    SlotsDone => ["done", "hecha", "fatto", "erledigt", "terminé", "feita", "已完成", "完了"],
    SlotsNoSlot => ["No slot is assigned to", "No hay hueco asignado a", "Nessun posto assegnato a", "Kein Platz zugewiesen für", "Aucune place attribuée à", "Não há posição atribuída a", "未分配位置给", "位置が未割り当て"],
    SlotsShared => ["Two tools share the slot", "Dos herramientas comparten el hueco", "Due utensili condividono il posto", "Zwei Werkzeuge teilen sich den Platz", "Deux outils partagent la place", "Duas ferramentas partilham a posição", "两把刀具共用同一位置", "2本の工具が同じ位置を使っています"],
    SlotsUndefined => ["The machine file does not define the slot", "El fichero de la máquina no define el hueco", "Il file della macchina non definisce il posto", "Die Maschinendatei definiert den Platz nicht", "Le fichier de la machine ne définit pas la place", "O ficheiro da máquina não define a posição", "机器文件未定义该位置", "マシンファイルにこの位置の定義がありません"],
    SlotsNote => ["Each tool goes in the slot you choose. The choice is saved for this machine and used again for the same tools. Put the tools in the bar as shown before you start.", "Cada herramienta va en el hueco que elijas. La elección se guarda para esta máquina y se reutiliza con las mismas herramientas. Coloca las herramientas en la barra como se indica antes de empezar.", "Ogni utensile va nel posto che scegli. La scelta viene salvata per questa macchina e riutilizzata con gli stessi utensili. Metti gli utensili nella barra come indicato prima di iniziare.", "Jedes Werkzeug kommt auf den gewählten Platz. Die Wahl wird für diese Maschine gespeichert und für dieselben Werkzeuge wiederverwendet. Legen Sie die Werkzeuge vor dem Start wie angezeigt in die Leiste.", "Chaque outil va dans la place choisie. Le choix est enregistré pour cette machine et réutilisé pour les mêmes outils. Placez les outils dans la barre comme indiqué avant de commencer.", "Cada ferramenta vai na posição que escolher. A escolha é guardada para esta máquina e reutilizada com as mesmas ferramentas. Coloque as ferramentas na barra como indicado antes de começar.", "每把刀具放在您选择的位置。该选择会为本机保存，并在使用相同刀具时重复使用。开始前请按所示将刀具放入刀库。", "各工具は選んだ位置に入れます。選択はこのマシン用に保存され、同じ工具で再利用されます。開始前に表示のとおり工具をバーに入れてください。"],
    LogTitle => ["Log", "Registro", "Registro", "Protokoll", "Journal", "Registo", "日志", "ログ"],
    LogWrite => ["Write the log", "Escribir el registro", "Scrivi il registro", "Protokoll schreiben", "Écrire le journal", "Escrever o registo", "写入日志", "ログを書き込む"],
    LogDetail => ["Level of detail", "Nivel de detalle", "Livello di dettaglio", "Detailgrad", "Niveau de détail", "Nível de detalhe", "详细程度", "詳細レベル"],
    LogMaxSize => ["Largest size of a file", "Tamaño máximo de un fichero", "Dimensione massima di un file", "Maximale Größe einer Datei", "Taille maximale d'un fichier", "Tamanho máximo de um ficheiro", "单个文件的最大大小", "ファイルの最大サイズ"],
    LogOpenFolder => ["Open the folder of the log", "Abrir la carpeta del registro", "Apri la cartella del registro", "Ordner des Protokolls öffnen", "Ouvrir le dossier du journal", "Abrir a pasta do registo", "打开日志文件夹", "ログのフォルダを開く"],
    LogCopyPath => ["Copy the path of the log", "Copiar la ruta del registro", "Copia il percorso del registro", "Pfad des Protokolls kopieren", "Copier le chemin du journal", "Copiar o caminho do registo", "复制日志路径", "ログのパスをコピー"],
    LogCopied => ["Path copied", "Ruta copiada", "Percorso copiato", "Pfad kopiert", "Chemin copié", "Caminho copiado", "路径已复制", "パスをコピーしました"],
    LogNote => ["The changes apply the next time the program starts.", "Los cambios se aplican la próxima vez que se inicie el programa.", "Le modifiche si applicano al prossimo avvio del programma.", "Die Änderungen gelten beim nächsten Start des Programms.", "Les changements s'appliquent au prochain démarrage du programme.", "As alterações aplicam-se na próxima vez que o programa iniciar.", "更改将在下次启动程序时生效。", "変更は次回プログラムを起動したときに反映されます。"],
    WarnRackUnknownField => ["The server ignored a field of the tool changer that it does not know:", "El servidor ignoró un campo del cambiador que no conoce:", "Il server ha ignorato un campo del cambio utensile che non conosce:", "Der Server hat ein ihm unbekanntes Feld des Werkzeugwechslers ignoriert:", "Le serveur a ignoré un champ du changeur qu'il ne connaît pas :", "O servidor ignorou um campo do trocador que não conhece:", "服务器忽略了它不认识的换刀器字段：", "サーバーは未知のツールチェンジャーのフィールドを無視しました："],
    WarnRequestNewer => ["The server read the request as the newest version it knows", "El servidor leyó la petición como la versión más reciente que conoce", "Il server ha letto la richiesta come l'ultima versione che conosce", "Der Server hat die Anfrage als neueste ihm bekannte Version gelesen", "Le serveur a lu la demande comme la dernière version qu'il connaît", "O servidor leu o pedido como a versão mais recente que conhece", "服务器将该请求按其所知的最新版本读取", "サーバーはこの要求を既知の最新バージョンとして読み取りました"],
    WarnRackNeedsV2 => ["The server ignored the tool changer: the request was of the first version", "El servidor ignoró el cambiador: la petición era de la primera versión", "Il server ha ignorato il cambio utensile: la richiesta era della prima versione", "Der Server hat den Werkzeugwechsler ignoriert: die Anfrage war von der ersten Version", "Le serveur a ignoré le changeur : la demande était de la première version", "O servidor ignorou o trocador: o pedido era da primeira versão", "服务器忽略了换刀器：请求属于第一版", "サーバーはツールチェンジャーを無視しました：要求は第1版でした"],
    WarnRackNoChanger => ["This machine changes its tools by hand: the tools are not chained", "Esta máquina cambia las herramientas a mano: no se encadenan", "Questa macchina cambia gli utensili a mano: non vengono concatenati", "Diese Maschine wechselt die Werkzeuge von Hand: sie werden nicht verkettet", "Cette machine change ses outils à la main : ils ne sont pas enchaînés", "Esta máquina troca as ferramentas à mão: não são encadeadas", "此机器手动换刀：刀具不会衔接", "このマシンは工具を手で交換します：工具は連結されません"],
    WarnOutOfCourse => ["The work goes past the course of the machine on the axis:", "El trabajo se sale del recorrido de la máquina en el eje:", "Il lavoro supera la corsa della macchina sull'asse:", "Die Arbeit überschreitet den Verfahrweg der Maschine auf der Achse:", "Le travail dépasse la course de la machine sur l'axe :", "O trabalho ultrapassa o curso da máquina no eixo:", "作业超出了机器在该轴上的行程：", "作業が次の軸で機械の可動範囲を超えています："],
}

impl crate::theme::ThemeId {
    /// The word that names the theme in the settings window.
    pub fn text(self) -> Text {
        use crate::theme::ThemeId as T;
        match self {
            T::Forge => Text::ThemeForge,
            T::Neon => Text::ThemeNeon,
            T::Glass => Text::ThemeGlass,
            T::WindowsClassic => Text::ThemeWindowsClassic,
            T::System => Text::ThemeSystem,
            T::Midnight => Text::ThemeMidnight,
            T::Paper => Text::ThemePaper,
            T::Forest => Text::ThemeForest,
            T::Retro => Text::ThemeRetro,
            T::Ocean => Text::ThemeOcean,
            T::Contrast => Text::ThemeContrast,
        }
    }
}

pub fn tr(lang: Lang, text: Text) -> &'static str {
    let row = &TABLE[text.index()];
    let mut l = lang;
    loop {
        let i = Lang::ALL.iter().position(|x| *x == l).unwrap_or(0);
        if i < CORE_LANGS {
            return row[i];
        }
        if let Some(s) = extra(l).get(text.name()) {
            return s;
        }
        match l.fallback() {
            Some(base) => l = base,
            None => return row[0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_exists_in_the_eight_languages() {
        for &t in ALL_TEXTS {
            for l in Lang::ALL {
                assert!(!tr(l, t).trim().is_empty(), "{t:?} in {l:?}");
            }
            let core = &Lang::ALL[..CORE_LANGS];
            // only the proper names may be the same as the English one in some language; no language is left with the English text of a sentence
            let en = tr(Lang::En, t);
            let same = core.iter().filter(|&&l| tr(l, t) == en).count();
            assert!(same <= 5, "{t:?} is the English text in {same} languages");
        }
    }

    #[test]
    fn the_words_of_the_log_of_the_settings_exist_in_the_thirty_seven_languages_and_the_sentence_is_translated(
    ) {
        let all = [
            Text::LogTitle,
            Text::LogWrite,
            Text::LogDetail,
            Text::LogMaxSize,
            Text::LogOpenFolder,
            Text::LogCopyPath,
            Text::LogCopied,
            Text::LogNote,
        ];
        for l in Lang::ALL {
            for t in all {
                assert!(!tr(l, t).trim().is_empty(), "{t:?} in {l:?}");
            }
            if l != Lang::En {
                assert_ne!(tr(l, Text::LogNote), tr(Lang::En, Text::LogNote), "{l:?}");
            }
        }
        // the Portuguese of Portugal says registo, not the Brazilian log
        assert!(tr(Lang::Pt, Text::LogWrite).contains("registo"));
    }

    #[test]
    fn the_warnings_of_the_server_about_the_request_exist_in_the_thirty_seven_languages() {
        let all = [
            Text::WarnRackUnknownField,
            Text::WarnRequestNewer,
            Text::WarnRackNeedsV2,
            Text::WarnRackNoChanger,
            Text::WarnOutOfCourse,
        ];
        for l in Lang::ALL {
            for t in all {
                assert!(!tr(l, t).trim().is_empty(), "{t:?} in {l:?}");
                if l != Lang::En {
                    assert_ne!(tr(l, t), tr(Lang::En, t), "{t:?} in {l:?}");
                }
            }
        }
        // the variants speak the language they come from, and the Portuguese of Portugal says ferramentas, not the Brazilian word
        assert_eq!(
            tr(Lang::PtBr, Text::WarnRackNoChanger),
            tr(Lang::Pt, Text::WarnRackNoChanger)
        );
        assert_eq!(
            tr(Lang::FrCa, Text::WarnRackNeedsV2),
            tr(Lang::Fr, Text::WarnRackNeedsV2)
        );
        assert!(tr(Lang::Pt, Text::WarnRackNoChanger).contains("ferramentas"));
    }

    #[test]
    fn the_chinese_and_the_japanese_use_their_own_letters() {
        for &t in ALL_TEXTS {
            for l in [Lang::Zh, Lang::Ja] {
                assert!(
                    tr(l, t).chars().any(|c| (c as u32) > 0x2e80),
                    "{t:?} in {l:?}"
                );
            }
        }
    }

    #[test]
    fn the_names_of_the_themes_differ_in_each_language() {
        for l in Lang::ALL.into_iter().filter(|l| l.is_core()) {
            let names = [
                Text::ThemeForge,
                Text::ThemeNeon,
                Text::ThemeGlass,
                Text::ThemeWindowsClassic,
                Text::ThemeSystem,
            ]
            .map(|t| tr(l, t));
            for i in 0..names.len() {
                for j in i + 1..names.len() {
                    assert_ne!(names[i], names[j], "{l:?}");
                }
            }
        }
    }

    #[test]
    fn the_codes_of_the_languages_come_back() {
        for l in Lang::ALL {
            assert_eq!(Lang::from_code(l.code()), Some(l));
        }
        assert_eq!(Lang::from_code(" PT "), Some(Lang::Pt));
        assert_eq!(Lang::from_code("xx"), None);
    }

    #[test]
    fn the_files_of_the_languages_only_name_texts_that_exist() {
        let names: std::collections::HashSet<&str> = ALL_TEXTS.iter().map(|t| t.name()).collect();
        for l in Lang::ALL.into_iter().filter(|l| !l.is_core()) {
            for line in source(l).lines() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                let (k, v) = line
                    .split_once('\t')
                    .unwrap_or_else(|| panic!("{l:?}: no tab in {line:?}"));
                assert!(names.contains(k.trim()), "{l:?}: unknown text {k:?}");
                assert!(!v.trim().is_empty(), "{l:?}: {k} is empty");
            }
        }
    }

    #[test]
    fn a_text_a_language_does_not_have_comes_from_english_or_from_its_main_language() {
        let any_missing = Lang::ALL
            .into_iter()
            .filter(|l| !l.is_core())
            .any(|l| coverage(l).0 < coverage(l).1);
        if any_missing {
            let t = ALL_TEXTS[0];
            assert!(!tr(Lang::Bg, t).is_empty());
        }
        // a variant takes the main language where it does not change a word
        for &t in ALL_TEXTS {
            if !extra(Lang::PtBr).contains_key(t.name()) {
                assert_eq!(tr(Lang::PtBr, t), tr(Lang::Pt, t));
            }
            if !extra(Lang::FrCa).contains_key(t.name()) {
                assert_eq!(tr(Lang::FrCa, t), tr(Lang::Fr, t));
            }
        }
    }

    #[test]
    fn the_regions_that_are_not_variants_give_their_main_language() {
        assert_eq!(Lang::from_code("pt_BR"), Some(Lang::PtBr));
        assert_eq!(Lang::from_code("fr-CA"), Some(Lang::FrCa));
        assert_eq!(Lang::from_code("es-MX"), Some(Lang::Es));
        assert_eq!(Lang::from_code("pt-PT"), Some(Lang::Pt));
        assert_eq!(Lang::ALL.len(), 37);
    }

    #[test]
    fn every_new_language_has_all_its_texts_and_the_variants_their_own_words() {
        for l in Lang::ALL.into_iter().filter(|l| !l.is_core()) {
            let (have, total) = coverage(l);
            if l.fallback().is_some() {
                assert!(have >= 20 && have < total, "{l:?}: {have}/{total}");
            } else {
                assert_eq!(have, total, "{l:?} is missing {} texts", total - have);
            }
        }
        // the alphabets are the language's own
        let in_range = |l: Lang, lo: char, hi: char| {
            ALL_TEXTS
                .iter()
                .filter(|t| tr(l, **t).chars().any(|c| (lo..=hi).contains(&c)))
                .count()
        };
        for l in [Lang::Bg, Lang::Uk, Lang::Ru, Lang::Sr] {
            assert!(in_range(l, '\u{400}', '\u{4ff}') > 300, "{l:?}");
        }
        assert!(in_range(Lang::El, '\u{370}', '\u{3ff}') > 300);
    }

    /// The languages that have a file of their own with every text (the variants are the two with a main language to fall back on).
    fn full_languages() -> Vec<Lang> {
        Lang::ALL
            .into_iter()
            .filter(|l| !l.is_core() && l.fallback().is_none())
            .collect()
    }

    #[test]
    fn the_project_has_thirty_seven_languages_with_different_codes_and_the_core_ones_come_first() {
        assert_eq!(Lang::ALL.len(), 37);
        let mut codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 37);
        assert_eq!(full_languages().len(), 27);
        for (i, l) in Lang::ALL.iter().enumerate() {
            assert_eq!(l.is_core(), i < CORE_LANGS, "{l:?}");
        }
    }

    #[test]
    fn every_language_that_has_a_file_has_every_text_in_it_and_no_text_that_does_not_exist() {
        let names: std::collections::HashSet<&str> = ALL_TEXTS.iter().map(|t| t.name()).collect();
        for l in full_languages() {
            let (own, all) = coverage(l);
            assert_eq!(own, all, "{l:?} has {own} of {all} texts of its own");
            for key in extra(l).keys() {
                assert!(
                    names.contains(key),
                    "{l:?}: the file has '{key}', which is not a text"
                );
            }
        }
    }

    #[test]
    fn no_text_of_any_language_is_empty_has_blanks_around_it_a_tab_or_a_broken_marker() {
        for &t in ALL_TEXTS {
            for l in Lang::ALL {
                let v = tr(l, t);
                assert!(!v.trim().is_empty(), "{t:?} in {l:?} is empty");
                assert_eq!(v, v.trim(), "{t:?} in {l:?} has blanks around it");
                assert!(!v.contains('\t'), "{t:?} in {l:?} has a tab");
                assert!(!v.contains('\r'), "{t:?} in {l:?} has a carriage return");
                assert!(
                    !v.contains("\\n"),
                    "{t:?} in {l:?} has a line break that was not turned into one"
                );
                assert!(
                    !v.contains("TODO") && !v.contains("FIXME") && !v.contains("???"),
                    "{t:?} in {l:?}"
                );
                assert_eq!(
                    v.matches('{').count(),
                    v.matches('}').count(),
                    "{t:?} in {l:?}: unbalanced braces: {v}"
                );
            }
        }
    }

    #[test]
    fn the_markers_of_a_text_are_the_same_in_every_language() {
        // `{}` is where a number or a name goes: a language that has more or fewer of them than English would print a wrong sentence or panic
        for &t in ALL_TEXTS {
            let english = tr(Lang::En, t).matches("{}").count();
            for l in Lang::ALL {
                assert_eq!(
                    tr(l, t).matches("{}").count(),
                    english,
                    "{t:?} in {l:?}: {}",
                    tr(l, t)
                );
            }
        }
    }

    #[test]
    fn a_line_break_in_a_text_is_in_the_same_places_in_every_language() {
        for &t in ALL_TEXTS {
            let english = tr(Lang::En, t).matches('\n').count();
            for l in Lang::ALL {
                assert_eq!(tr(l, t).matches('\n').count(), english, "{t:?} in {l:?}");
            }
        }
    }

    #[test]
    fn the_variants_have_only_what_they_change_and_the_rest_is_their_main_language() {
        for (variant, main) in [(Lang::PtBr, Lang::Pt), (Lang::FrCa, Lang::Fr)] {
            let (own, all) = coverage(variant);
            assert!(own < all, "{variant:?} would be a full copy of {main:?}");
            for &t in ALL_TEXTS {
                if !extra(variant).contains_key(t.name()) {
                    assert_eq!(tr(variant, t), tr(main, t), "{t:?} in {variant:?}");
                }
            }
        }
    }

    #[test]
    fn the_names_of_the_texts_are_unique_and_each_text_is_found_by_its_name() {
        let mut names: Vec<&str> = ALL_TEXTS.iter().map(|t| t.name()).collect();
        let n = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "two texts have the same name");
        for &t in ALL_TEXTS {
            assert!(
                !t.name().is_empty() && t.name().chars().all(|c| c.is_ascii_alphanumeric()),
                "{t:?}"
            );
        }
    }

    #[test]
    fn every_file_of_a_language_has_one_tab_in_each_line_and_no_key_twice() {
        for l in full_languages() {
            let mut seen = std::collections::HashSet::new();
            for line in source(l).lines() {
                let line = line.trim_end_matches('\r');
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }
                let (k, v) = line
                    .split_once('\t')
                    .unwrap_or_else(|| panic!("{l:?}: a line without a tab: {line}"));
                assert!(
                    !v.contains('\t'),
                    "{l:?}: more than one tab in the line of {k}"
                );
                assert_eq!(k, k.trim(), "{l:?}: blanks around the key {k}");
                assert!(seen.insert(k.to_string()), "{l:?}: the key {k} is twice");
            }
        }
    }
}
