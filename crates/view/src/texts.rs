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
