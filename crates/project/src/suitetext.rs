//! The words of the entries of the window for a project file of the suite (`.eh3d`), in the eight languages: English, Spanish, German,
//! French, Italian, Japanese, Chinese and Portuguese of Portugal. A message has numbered holes (`{0}`, `{1}`, ...) that [`fill`] fills.

/// The language codes, in the order of the columns of the table.
pub const CODES: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];

macro_rules! words {
    ($($name:ident => [$($text:expr),+ $(,)?],)+) => {
        /// Every message about a project of the suite.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum S {
            $(
                #[doc = stringify!($name)]
                $name,
            )+
        }

        impl S {
            /// Every message.
            pub const ALL: &'static [S] = &[$(S::$name),+];

            /// The message in the eight languages, in the order of [`CODES`].
            pub fn words(self) -> [&'static str; 8] {
                match self {
                    $(S::$name => [$($text),+],)+
                }
            }
        }
    };
}

words! {
    Menu => ["Suite project (.eh3d)", "Proyecto de la suite (.eh3d)", "Suite-Projekt (.eh3d)", "Projet de la suite (.eh3d)", "Progetto della suite (.eh3d)", "スイートプロジェクト (.eh3d)", "套件项目 (.eh3d)", "Projeto da suite (.eh3d)"],
    MenuOpen => ["Open suite project...", "Abrir proyecto de la suite...", "Suite-Projekt öffnen...", "Ouvrir un projet de la suite...", "Apri progetto della suite...", "スイートプロジェクトを開く...", "打开套件项目...", "Abrir projeto da suite..."],
    MenuSave => ["Save suite project", "Guardar proyecto de la suite", "Suite-Projekt speichern", "Enregistrer le projet de la suite", "Salva progetto della suite", "スイートプロジェクトを保存", "保存套件项目", "Guardar projeto da suite"],
    MenuSaveAs => ["Save suite project as...", "Guardar proyecto de la suite como...", "Suite-Projekt speichern unter...", "Enregistrer le projet de la suite sous...", "Salva progetto della suite con nome...", "スイートプロジェクトに名前を付けて保存...", "套件项目另存为...", "Guardar projeto da suite como..."],
    MenuImport => ["Import a file and keep the original...", "Importar un fichero y guardar el original...", "Datei importieren und Original behalten...", "Importer un fichier et garder l'original...", "Importa un file e conserva l'originale...", "ファイルを取り込み原本を保持...", "导入文件并保留原件...", "Importar um ficheiro e guardar o original..."],
    MenuTake => ["Take from another branch...", "Tomar de otra rama...", "Aus einem anderen Zweig übernehmen...", "Prendre d'une autre branche...", "Prendi da un altro ramo...", "別のブランチから取り込む...", "从其他分支取用...", "Obter de outro ramo..."],
    MenuExtract => ["Extract a branch...", "Extraer una rama...", "Zweig entpacken...", "Extraire une branche...", "Estrai un ramo...", "ブランチを展開...", "提取分支...", "Extrair um ramo..."],
    MenuBranches => ["Branches of the project...", "Ramas del proyecto...", "Zweige des Projekts...", "Branches du projet...", "Rami del progetto...", "プロジェクトのブランチ...", "项目分支...", "Ramos do projeto..."],
    Project => ["Suite project: {0}", "Proyecto de la suite: {0}", "Suite-Projekt: {0}", "Projet de la suite : {0}", "Progetto della suite: {0}", "スイートプロジェクト：{0}", "套件项目：{0}", "Projeto da suite: {0}"],
    OutOfDate => ["OUT OF DATE: {0} was derived from {1} before it changed ({2})", "DESACTUALIZADO: {0} se derivó de {1} antes de que cambiara ({2})", "VERALTET: {0} wurde aus {1} abgeleitet, bevor sich dieser änderte ({2})", "PÉRIMÉ : {0} a été dérivé de {1} avant sa modification ({2})", "NON AGGIORNATO: {0} è stato derivato da {1} prima che cambiasse ({2})", "古い状態：{0} は {1} が変更される前に派生しました（{2}）", "已过期：{0} 是在 {1} 更改之前派生的（{2}）", "DESATUALIZADO: {0} foi derivado de {1} antes de este mudar ({2})"],
    SignedProject => ["The project is signed: writing removes the signature", "El proyecto está firmado: al escribir se quita la firma", "Das Projekt ist signiert: Schreiben entfernt die Signatur", "Le projet est signé : l'écriture retire la signature", "Il progetto è firmato: la scrittura rimuove la firma", "プロジェクトは署名されています。書き込むと署名は削除されます", "项目已签名：写入会移除签名", "O projeto está assinado: a escrita remove a assinatura"],
    SignedBranch => ["The branch {0} is signed: writing removes its signature", "La rama {0} está firmada: al escribir se quita su firma", "Der Zweig {0} ist signiert: Schreiben entfernt seine Signatur", "La branche {0} est signée : l'écriture retire sa signature", "Il ramo {0} è firmato: la scrittura rimuove la sua firma", "ブランチ {0} は署名されています。書き込むと署名は削除されます", "分支 {0} 已签名：写入会移除其签名", "O ramo {0} está assinado: a escrita remove a assinatura"],
    ConfirmTitle => ["Write to this project?", "¿Escribir en este proyecto?", "In dieses Projekt schreiben?", "Écrire dans ce projet ?", "Scrivere in questo progetto?", "このプロジェクトに書き込みますか？", "写入此项目？", "Escrever neste projeto?"],
    WarnForeign => ["{0} is a project that was not opened here: its branch {1} will be written", "{0} es un proyecto que no se ha abierto aquí: se escribirá su rama {1}", "{0} ist ein hier nicht geöffnetes Projekt: sein Zweig {1} wird geschrieben", "{0} est un projet qui n'a pas été ouvert ici : sa branche {1} sera écrite", "{0} è un progetto non aperto qui: il suo ramo {1} verrà scritto", "{0} はここで開いていないプロジェクトです。ブランチ {1} が書き込まれます", "{0} 是未在此打开的项目：将写入其分支 {1}", "{0} é um projeto que não foi aberto aqui: o ramo {1} será escrito"],
    WarnChanged => ["The branch {0} of {1} has changed since it was read here", "La rama {0} de {1} ha cambiado desde que se leyó aquí", "Der Zweig {0} von {1} hat sich seit dem Lesen geändert", "La branche {0} de {1} a changé depuis sa lecture ici", "Il ramo {0} di {1} è cambiato da quando è stato letto qui", "{1} のブランチ {0} は、ここで読み込んだ後に変更されています", "{1} 的分支 {0} 自读取后已更改", "O ramo {0} de {1} mudou desde que foi lido aqui"],
    Write => ["Write", "Escribir", "Schreiben", "Écrire", "Scrivi", "書き込む", "写入", "Escrever"],
    NoticeProjectSignature => ["The signature of the project was removed", "Se ha quitado la firma del proyecto", "Die Signatur des Projekts wurde entfernt", "La signature du projet a été retirée", "La firma del progetto è stata rimossa", "プロジェクトの署名を削除しました", "已移除项目签名", "A assinatura do projeto foi removida"],
    NoticeBranchSignature => ["The signature of the branch was removed", "Se ha quitado la firma de la rama", "Die Signatur des Zweigs wurde entfernt", "La signature de la branche a été retirée", "La firma del ramo è stata rimossa", "ブランチの署名を削除しました", "已移除分支签名", "A assinatura do ramo foi removida"],
    NoticeImportReplaced => ["The import that was there before was replaced", "Se ha sustituido la importación que había antes", "Der vorherige Import wurde ersetzt", "L'importation précédente a été remplacée", "L'importazione precedente è stata sostituita", "以前の取り込みを置き換えました", "已替换之前的导入", "A importação anterior foi substituída"],
    Opened => ["{0} was opened", "Se ha abierto {0}", "{0} wurde geöffnet", "{0} a été ouvert", "{0} è stato aperto", "{0} を開きました", "已打开 {0}", "{0} foi aberto"],
    NoWork => ["{0} has no branch {1} yet: saving puts the work there", "{0} todavía no tiene rama {1}: al guardar se pone en ella el trabajo", "{0} hat noch keinen Zweig {1}: Speichern legt die Arbeit dort ab", "{0} n'a pas encore de branche {1} : l'enregistrement y place le travail", "{0} non ha ancora il ramo {1}: il salvataggio vi mette il lavoro", "{0} にはまだブランチ {1} がありません。保存すると作業がそこに入ります", "{0} 还没有分支 {1}：保存时工作会放入其中", "{0} ainda não tem o ramo {1}: ao guardar, o trabalho é colocado nele"],
    Saved => ["Saved in the branch {0} of {1}", "Guardado en la rama {0} de {1}", "Im Zweig {0} von {1} gespeichert", "Enregistré dans la branche {0} de {1}", "Salvato nel ramo {0} di {1}", "{1} のブランチ {0} に保存しました", "已保存到 {1} 的分支 {0}", "Guardado no ramo {0} de {1}"],
    NothingToSave => ["There is nothing to save", "No hay nada que guardar", "Es gibt nichts zu speichern", "Il n'y a rien à enregistrer", "Non c'è nulla da salvare", "保存するものがありません", "没有可保存的内容", "Não há nada para guardar"],
    NothingNew => ["Nothing has changed: the project was not written", "No ha cambiado nada: no se ha escrito el proyecto", "Nichts hat sich geändert: das Projekt wurde nicht geschrieben", "Rien n'a changé : le projet n'a pas été écrit", "Nulla è cambiato: il progetto non è stato scritto", "変更がないため、プロジェクトは書き込まれませんでした", "没有任何更改：未写入项目", "Nada mudou: o projeto não foi escrito"],
    NeedProject => ["Open or save a suite project first", "Abre o guarda antes un proyecto de la suite", "Bitte zuerst ein Suite-Projekt öffnen oder speichern", "Ouvrez ou enregistrez d'abord un projet de la suite", "Apri o salva prima un progetto della suite", "先にスイートプロジェクトを開くか保存してください", "请先打开或保存套件项目", "Abra ou guarde primeiro um projeto da suite"],
    NeedExtension => ["A suite project is a file that ends in .eh3d", "Un proyecto de la suite es un fichero que termina en .eh3d", "Ein Suite-Projekt ist eine Datei mit der Endung .eh3d", "Un projet de la suite est un fichier qui finit par .eh3d", "Un progetto della suite è un file che termina in .eh3d", "スイートプロジェクトは .eh3d で終わるファイルです", "套件项目是以 .eh3d 结尾的文件", "Um projeto da suite é um ficheiro que termina em .eh3d"],
    Imported => ["{0} was imported; the original is kept", "Se ha importado {0}; el original se conserva", "{0} wurde importiert; das Original bleibt erhalten", "{0} a été importé ; l'original est conservé", "{0} è stato importato; l'originale è conservato", "{0} を取り込みました。原本は保持されます", "已导入 {0}；原件已保留", "{0} foi importado; o original é conservado"],
    Taken => ["{0}/{1} was taken into the project", "{0}/{1} se ha tomado dentro del proyecto", "{0}/{1} wurde in das Projekt übernommen", "{0}/{1} a été pris dans le projet", "{0}/{1} è stato preso nel progetto", "{0}/{1} をプロジェクトに取り込みました", "已将 {0}/{1} 取入项目", "{0}/{1} foi obtido para o projeto"],
    Extracted => ["{0} files written to {1}", "{0} ficheros escritos en {1}", "{0} Dateien nach {1} geschrieben", "{0} fichiers écrits dans {1}", "{0} file scritti in {1}", "{0} 個のファイルを {1} に書き出しました", "已将 {0} 个文件写入 {1}", "{0} ficheiros escritos em {1}"],
    NothingToTake => ["No other branch has files to take", "Ninguna otra rama tiene ficheros que tomar", "Kein anderer Zweig hat Dateien zum Übernehmen", "Aucune autre branche n'a de fichiers à prendre", "Nessun altro ramo ha file da prendere", "取り込めるファイルがある別のブランチはありません", "其他分支没有可取用的文件", "Nenhum outro ramo tem ficheiros para obter"],
    ChooseFile => ["Choose a file of another branch", "Elige un fichero de otra rama", "Eine Datei eines anderen Zweigs wählen", "Choisissez un fichier d'une autre branche", "Scegli un file di un altro ramo", "別のブランチのファイルを選んでください", "请选择其他分支的文件", "Escolha um ficheiro de outro ramo"],
    ChooseBranch => ["Choose the branch to extract", "Elige la rama que se va a extraer", "Den zu entpackenden Zweig wählen", "Choisissez la branche à extraire", "Scegli il ramo da estrarre", "展開するブランチを選んでください", "请选择要提取的分支", "Escolha o ramo a extrair"],
    Extract => ["Extract", "Extraer", "Entpacken", "Extraire", "Estrai", "展開", "提取", "Extrair"],
    ReadAgain => ["Read again", "Volver a leer", "Neu lesen", "Relire", "Rileggi", "再読み込み", "重新读取", "Ler novamente"],
    Close => ["Close", "Cerrar", "Schließen", "Fermer", "Chiudi", "閉じる", "关闭", "Fechar"],
    ColBranch => ["Branch", "Rama", "Zweig", "Branche", "Ramo", "ブランチ", "分支", "Ramo"],
    ColApp => ["Program", "Programa", "Programm", "Programme", "Programma", "プログラム", "程序", "Programa"],
    ColFormat => ["Format", "Formato", "Format", "Format", "Formato", "形式", "格式", "Formato"],
    ColFiles => ["Files", "Ficheros", "Dateien", "Fichiers", "File", "ファイル", "文件", "Ficheiros"],
    ColState => ["State", "Estado", "Stand", "État", "Stato", "状態", "状态", "Estado"],
    StateCurrent => ["current", "actual", "aktuell", "à jour", "attuale", "最新", "最新", "atual"],
    StateStale => ["out of date: {0} has changed", "desactualizada: {0} ha cambiado", "veraltet: {0} hat sich geändert", "périmée : {0} a changé", "non aggiornato: {0} è cambiato", "古い状態：{0} が変更されました", "已过期：{0} 已更改", "desatualizado: {0} mudou"],
    StateSigned => ["signed (not checked)", "firmada (sin comprobar)", "signiert (nicht geprüft)", "signée (non vérifiée)", "firmato (non verificato)", "署名あり（未検証）", "已签名（未校验）", "assinado (não verificado)"],
    StateSealed => ["sealed", "sellada", "versiegelt", "scellée", "sigillato", "封印済み", "已封存", "selado"],
    StateEncrypted => ["encrypted", "cifrada", "verschlüsselt", "chiffrée", "cifrato", "暗号化", "已加密", "cifrado"],
    Original => ["Original kept: {0} ({1} bytes)", "Original guardado: {0} ({1} bytes)", "Original aufbewahrt: {0} ({1} Bytes)", "Original conservé : {0} ({1} octets)", "Originale conservato: {0} ({1} byte)", "保持している原本：{0}（{1} バイト）", "已保留原件：{0}（{1} 字节）", "Original guardado: {0} ({1} bytes)"],
    ReportRead => ["Read: {0}", "Leído: {0}", "Gelesen: {0}", "Lu : {0}", "Letto: {0}", "読み込んだ内容：{0}", "已读取：{0}", "Lido: {0}"],
    ReportNotRead => ["Not read: {0}", "No leído: {0}", "Nicht gelesen: {0}", "Non lu : {0}", "Non letto: {0}", "読み込めなかった内容：{0}", "未读取：{0}", "Não lido: {0}"],
    ReportLost => ["Lost on export: {0}", "Se pierde al exportar: {0}", "Beim Export verloren: {0}", "Perdu à l'export : {0}", "Perso nell'esportazione: {0}", "書き出し時に失われる内容：{0}", "导出时丢失：{0}", "Perde-se na exportação: {0}"],
    SettingsWithToken => ["The settings file is not put in the project: it holds the token of the server", "El fichero de ajustes no se pone en el proyecto: contiene el token del servidor", "Die Einstellungsdatei kommt nicht ins Projekt: sie enthält das Token des Servers", "Le fichier de réglages n'est pas mis dans le projet : il contient le jeton du serveur", "Il file delle impostazioni non viene messo nel progetto: contiene il token del server", "設定ファイルはサーバーのトークンを含むため、プロジェクトに入れません", "设置文件含有服务器令牌，未放入项目", "O ficheiro de definições não é colocado no projeto: contém o token do servidor"],
    MotionWithCredential => ["The motion file is not put in the project: it holds the credential of the link", "El fichero de movimiento no se pone en el proyecto: contiene la credencial del enlace", "Die Bewegungsdatei kommt nicht ins Projekt: sie enthält den Zugangsschlüssel der Verbindung", "Le fichier de mouvement n'est pas mis dans le projet : il contient l'identifiant de la liaison", "Il file di movimento non viene messo nel progetto: contiene la credenziale del collegamento", "動作設定ファイルは接続の認証情報を含むため、プロジェクトに入れません", "运动文件含有连接凭据，未放入项目", "O ficheiro de movimento não é colocado no projeto: contém a credencial da ligação"],
    AttachmentDangerous => ["{0} is dangerous: it is {1}. Continue only if you trust it", "{0} es peligroso: es {1}. Continúa solo si confías en él", "{0} ist gefährlich: es ist {1}. Nur fortfahren, wenn Sie der Datei vertrauen", "{0} est dangereux : c'est {1}. Continuez uniquement si vous lui faites confiance", "{0} è pericoloso: è {1}. Continua solo se ti fidi", "{0} は危険です（{1}）。信頼できる場合のみ続行してください", "{0} 有危险：它是{1}。仅在信任它时继续", "{0} é perigoso: é {1}. Continue apenas se confiar nele"],
    DangerUnknown => ["of a kind the suite does not know", "de un tipo que la suite no conoce", "von einem Typ, den die Suite nicht kennt", "d'un type que la suite ne connaît pas", "di un tipo che la suite non conosce", "スイートが認識できない種類", "套件无法识别的类型", "de um tipo que a suite não conhece"],
    DangerExecutable => ["a program", "un programa", "ein Programm", "un programme", "un programma", "プログラム", "程序", "um programa"],
    DangerScript => ["a script", "un script", "ein Skript", "un script", "uno script", "スクリプト", "脚本", "um script"],
    DangerMacros => ["a document with macros", "un documento con macros", "ein Dokument mit Makros", "un document avec des macros", "un documento con macro", "マクロを含む文書", "含宏的文档", "um documento com macros"],
    DangerArchive => ["a compressed archive", "un archivo comprimido", "ein komprimiertes Archiv", "une archive compressée", "un archivio compresso", "圧縮アーカイブ", "压缩包", "um ficheiro comprimido"],
    DangerActive => ["a page that can run code", "una página que puede ejecutar código", "eine Seite, die Code ausführen kann", "une page qui peut exécuter du code", "una pagina che può eseguire codice", "コードを実行できるページ", "可运行代码的页面", "uma página que pode executar código"],
    AttachmentLinkOnly => ["{0} is only a link: {1}. Nothing is downloaded by itself", "{0} es solo un enlace: {1}. Nada se descarga solo", "{0} ist nur ein Link: {1}. Nichts wird von selbst heruntergeladen", "{0} n'est qu'un lien : {1}. Rien n'est téléchargé tout seul", "{0} è solo un collegamento: {1}. Nulla viene scaricato da solo", "{0} はリンクのみです：{1}。自動ではダウンロードされません", "{0} 只是链接：{1}。不会自动下载任何内容", "{0} é apenas uma ligação: {1}. Nada é transferido por si só"],
    AttachmentTooLarge => ["{0} is larger than the limit of {1} MB", "{0} supera el límite de {1} MB", "{0} ist größer als die Grenze von {1} MB", "{0} dépasse la limite de {1} Mo", "{0} supera il limite di {1} MB", "{0} は上限の {1} MB を超えています", "{0} 超过 {1} MB 的限制", "{0} ultrapassa o limite de {1} MB"],
    ProgramNotCurrent => ["The program is not put in the project: work it out again first", "El programa no se pone en el proyecto: calcúlalo de nuevo antes", "Das Programm kommt nicht ins Projekt: bitte erst neu berechnen", "Le programme n'est pas mis dans le projet : recalculez-le d'abord", "Il programma non viene messo nel progetto: ricalcolalo prima", "プログラムはプロジェクトに入れません。先に再計算してください", "程序未放入项目：请先重新计算", "O programa não é colocado no projeto: calcule-o novamente primeiro"],
    LimitsKept => ["The server, the limits and the interlocks of your own settings stay: the project cannot loosen them", "Se mantienen el servidor, los límites y los enclavamientos de tus ajustes: el proyecto no puede relajarlos", "Server, Grenzwerte und Verriegelungen Ihrer Einstellungen bleiben: das Projekt kann sie nicht lockern", "Le serveur, les limites et les verrouillages de vos réglages restent : le projet ne peut pas les assouplir", "Server, limiti e interblocchi delle tue impostazioni restano: il progetto non può allentarli", "サーバー、制限、インターロックはご自身の設定が保たれます。プロジェクトで緩めることはできません", "服务器、限值和联锁保留您自己的设置：项目无法放宽它们", "O servidor, os limites e os encravamentos das suas definições mantêm-se: o projeto não os pode aliviar"],
    MachineFilesNotApplied => ["The table of tools and the motion file of the project are not applied; extract the branch to use them", "La tabla de herramientas y el fichero de movimiento del proyecto no se aplican; extrae la rama para usarlos", "Werkzeugtabelle und Bewegungsdatei des Projekts werden nicht angewendet; zum Verwenden den Zweig entpacken", "La table des outils et le fichier de mouvement du projet ne sont pas appliqués ; extrayez la branche pour les utiliser", "La tabella utensili e il file di movimento del progetto non sono applicati; estrai il ramo per usarli", "プロジェクトの工具表と動作設定ファイルは適用されません。使うにはブランチを展開してください", "项目的刀具表和运动文件不会生效；如需使用请提取该分支", "A tabela de ferramentas e o ficheiro de movimento do projeto não são aplicados; extraia o ramo para os usar"],
    PutTools => ["Put the table of tools in the project", "Poner la tabla de herramientas en el proyecto", "Werkzeugtabelle ins Projekt legen", "Mettre la table des outils dans le projet", "Metti la tabella utensili nel progetto", "工具表をプロジェクトに入れる", "将刀具表放入项目", "Colocar a tabela de ferramentas no projeto"],
    PutMotion => ["Put the motion file in the project", "Poner el fichero de movimiento en el proyecto", "Bewegungsdatei ins Projekt legen", "Mettre le fichier de mouvement dans le projet", "Metti il file di movimento nel progetto", "動作設定ファイルをプロジェクトに入れる", "将运动文件放入项目", "Colocar o ficheiro de movimento no projeto"],
    Busy => ["Stop the program and wait for the machine first", "Detén antes el programa y espera a la máquina", "Bitte zuerst das Programm stoppen und auf die Maschine warten", "Arrêtez d'abord le programme et attendez la machine", "Ferma prima il programma e attendi la macchina", "先にプログラムを停止し、機械の完了を待ってください", "请先停止程序并等待机器", "Pare primeiro o programa e aguarde pela máquina"],
    NotOverProject => ["A suite project is not written over with a program", "No se escribe un programa encima de un proyecto de la suite", "Ein Suite-Projekt wird nicht mit einem Programm überschrieben", "Un projet de la suite n'est pas écrasé par un programme", "Un progetto della suite non viene sovrascritto da un programma", "スイートプロジェクトをプログラムで上書きすることはできません", "不能用程序覆盖套件项目", "Um projeto da suite não é substituído por um programa"],
    JobOpened => ["Project job, layer 1 of {0}. Ctrl+S saves the plan", "Trabajo del proyecto, capa 1 de {0}. Ctrl+S guarda el plan", "Projektauftrag, Ebene 1 von {0}. Strg+S speichert den Plan", "Travail du projet, couche 1 sur {0}. Ctrl+S garde le plan", "Lavoro del progetto, livello 1 di {0}. Ctrl+S salva il piano", "プロジェクトのジョブ、レイヤー 1/{0}。Ctrl+S で計画を保存", "项目作业，第 1 层，共 {0} 层。Ctrl+S 保存计划", "Trabalho do projeto, camada 1 de {0}. Ctrl+S guarda o plano"],
    JobStale => ["OUT OF DATE: the job changed. Ctrl+S saves a new plan", "DESACTUALIZADO: el trabajo cambió. Ctrl+S guarda un plan nuevo", "VERALTET: Auftrag geändert. Strg+S speichert einen neuen Plan", "PÉRIMÉ : le travail a changé. Ctrl+S garde un nouveau plan", "NON AGGIORNATO: lavoro cambiato. Ctrl+S salva un nuovo piano", "古い状態：ジョブが変更されました。Ctrl+S で新しい計画を保存", "已过期：作业已更改。Ctrl+S 保存新计划", "DESATUALIZADO: o trabalho mudou. Ctrl+S guarda um plano novo"],
    JobSaved => ["{0} saved in the project", "{0} guardado en el proyecto", "{0} im Projekt gespeichert", "{0} enregistré dans le projet", "{0} salvato nel progetto", "{0} をプロジェクトに保存しました", "{0} 已保存到项目", "{0} guardado no projeto"],
}

impl S {
    /// The message in the language of a code such as `es` or `pt-PT` (anything else gives English).
    pub fn tr(self, code: &str) -> &'static str {
        let two = code.split(['-', '_']).next().unwrap_or("");
        let i = CODES
            .iter()
            .position(|c| c.eq_ignore_ascii_case(two))
            .unwrap_or(0);
        self.words()[i]
    }
}

/// The text with `{0}`, `{1}`, ... replaced by the values; what a value holds is never replaced again, and a hole with no value stays.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let hole = after.find('}').and_then(|close| {
            let value = after[..close].parse::<usize>().ok()?;
            Some((*values.get(value)?, close))
        });
        match hole {
            Some((value, close)) => {
                out.push_str(value);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A message with its values, told in whatever language the window is in when it is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said(pub S, pub Vec<String>);

impl Said {
    /// A message with no value.
    pub fn plain(word: S) -> Said {
        Said(word, Vec::new())
    }

    /// The message in a language.
    pub fn text(&self, code: &str) -> String {
        let values: Vec<&str> = self.1.iter().map(String::as_str).collect();
        fill(self.0.tr(code), &values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holes(s: &str) -> Vec<char> {
        let mut found: Vec<char> = s
            .split('{')
            .skip(1)
            .filter_map(|part| part.chars().next())
            .collect();
        found.sort_unstable();
        found
    }

    #[test]
    fn every_message_is_written_in_the_eight_languages_with_the_same_holes() {
        for &m in S::ALL {
            let words = m.words();
            for (i, text) in words.iter().enumerate() {
                assert!(!text.trim().is_empty(), "{m:?} is empty in {}", CODES[i]);
                assert_eq!(text.trim(), *text, "{m:?} in {}", CODES[i]);
                assert_eq!(holes(text), holes(words[0]), "{m:?} in {}", CODES[i]);
            }
            let same = words.iter().skip(1).filter(|t| **t == words[0]).count();
            assert!(same < 6, "{m:?} is not translated");
            for brazilian in ["arquivo", "tela", "salvar", "usuário"] {
                let pt = words[7].to_lowercase();
                assert!(
                    !pt.split_whitespace().any(|w| w == brazilian),
                    "{m:?}: {pt}"
                );
            }
        }
    }

    #[test]
    fn the_holes_are_filled_once_and_a_code_picks_the_language() {
        assert_eq!(fill("{0} and {1}", &["{1}", "b"]), "{1} and b");
        assert_eq!(fill("{2} {x} {", &["a"]), "{2} {x} {");
        assert_eq!(S::Write.tr("es"), "Escribir");
        assert_eq!(S::Write.tr("pt-PT"), "Escrever");
        assert_eq!(S::Write.tr("xx"), "Write");
        let said = Said(S::Saved, vec!["laser".into(), "a.eh3d".into()]);
        assert_eq!(said.text("en"), "Saved in the branch laser of a.eh3d");
    }
}
