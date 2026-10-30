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
    ResumeTitle => ["Job interrupted", "Trabajo interrumpido", "Lavoro interrotto", "Auftrag unterbrochen", "Travail interrompu", "Trabalho interrompido", "作业被中断", "ジョブが中断されました"],
    ResumeBody => ["The last job was left in the middle (the window, the computer or the link went down). Do you want to go on from where it was?", "El último trabajo quedó a medias (se cerró la ventana, el equipo o el enlace). ¿Quieres seguir por donde iba?", "L'ultimo lavoro è rimasto a metà (finestra, computer o collegamento caduti). Vuoi continuare da dove era arrivato?", "Der letzte Auftrag blieb unvollendet (Fenster, Rechner oder Verbindung ausgefallen). Möchten Sie dort weitermachen, wo er stand?", "Le dernier travail est resté en cours (fenêtre, ordinateur ou liaison tombés). Voulez-vous continuer là où il en était ?", "O último trabalho ficou a meio (a janela, o computador ou a ligação falhou). Quer continuar por onde ia?", "上一个作业未完成（窗口、电脑或连接中断）。是否从中断处继续？", "前回のジョブは途中で止まりました（ウィンドウ、PC、接続の停止）。続きから再開しますか？"],
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
