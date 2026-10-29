//! The words of the launcher, the viewer and the themes, in the eight languages of the project.
//! Order: English, Spanish, Italian, German, French, Portuguese (of Portugal), Chinese, Japanese.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    It,
    De,
    Fr,
    Pt,
    Zh,
    Ja,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::It,
        Lang::De,
        Lang::Fr,
        Lang::Pt,
        Lang::Zh,
        Lang::Ja,
    ];

    /// The code of the configuration file and of the command line.
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
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.trim().to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == c)
    }
}

macro_rules! texts {
    ($($name:ident => [$($t:expr),+ $(,)?]),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Text { $($name),+ }
        pub const ALL_TEXTS: &[Text] = &[$(Text::$name),+];
        const TABLE: &[[&str; 8]] = &[$([$($t),+]),+];
        impl Text {
            fn index(self) -> usize { ALL_TEXTS.iter().position(|t| *t == self).unwrap_or(0) }
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
}

pub fn tr(lang: Lang, text: Text) -> &'static str {
    let col = Lang::ALL.iter().position(|l| *l == lang).unwrap_or(0);
    TABLE[text.index()][col]
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
            // only the proper names may be the same as the English one in some language; no language is left with the English text of a sentence
            let en = tr(Lang::En, t);
            let same = Lang::ALL.iter().filter(|&&l| tr(l, t) == en).count();
            assert!(same <= 5, "{t:?} is the English text in {same} languages");
        }
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
        for l in Lang::ALL {
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
}
