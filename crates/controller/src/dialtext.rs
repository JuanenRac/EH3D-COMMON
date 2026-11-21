//! The words of the errors and the alarms of the controllers in the eight languages of the suite (English, Spanish, German, French, Italian,
//! Japanese, Chinese and Portuguese of Portugal): the alarms and the errors of GRBL by number, the numbers that grblHAL adds to them, the
//! kinds of error of Marlin (which says them in words, and which this crate numbers from 101) and the kinds of error of Klipper (numbered
//! from 201). [`explain`] picks the table of the family; a number that is in no table gives the text of the controller as it came.

use crate::Family;

/// The language codes, in the order of the columns of the tables.
pub const LANGS: [&str; 8] = ["en", "es", "de", "fr", "it", "ja", "zh", "pt"];

fn column(lang: &str) -> usize {
    LANGS.iter().position(|l| *l == lang).unwrap_or(0)
}

/// The text of `ALARM:N` in the language (English for a language that is not one of the eight); `None` for a number not in the table.
pub fn alarm(code: u32, lang: &str) -> Option<&'static str> {
    let row: [&str; 8] = match code {
        1 => [
            "A hard limit was hit. The position is probably lost: home the machine again.",
            "Se activó un límite duro. La posición probablemente se perdió: vuelva a hacer el home.",
            "Ein Hardware-Endschalter wurde ausgelöst. Die Position ist wahrscheinlich verloren: Referenzfahrt wiederholen.",
            "Une limite matérielle a été atteinte. La position est probablement perdue : refaites la prise d'origine.",
            "Si è attivato un limite fisico. La posizione è probabilmente persa: rifare l'homing.",
            "ハードリミットが作動しました。位置は失われた可能性があります。原点復帰をやり直してください。",
            "触发了硬限位。位置可能已丢失：请重新回零。",
            "Um limite físico foi acionado. A posição provavelmente perdeu-se: volte a fazer o homing.",
        ],
        2 => [
            "The move goes past the travel of the machine. The position is kept; the alarm can be cleared.",
            "El movimiento sale del recorrido de la máquina. La posición se conserva; se puede quitar la alarma.",
            "Die Bewegung überschreitet den Verfahrweg der Maschine. Die Position bleibt erhalten; der Alarm kann gelöscht werden.",
            "Le mouvement dépasse la course de la machine. La position est conservée ; l'alarme peut être effacée.",
            "Il movimento supera la corsa della macchina. La posizione è conservata; l'allarme si può cancellare.",
            "移動が機械の可動範囲を超えています。位置は保持されており、アラームは解除できます。",
            "运动超出机床行程。位置已保留，可以清除报警。",
            "O movimento ultrapassa o curso da máquina. A posição mantém-se; o alarme pode ser limpo.",
        ],
        3 => [
            "Reset while moving. The position cannot be trusted: home the machine again.",
            "Reinicio en pleno movimiento. No se puede fiar de la posición: vuelva a hacer el home.",
            "Zurücksetzen während der Bewegung. Der Position ist nicht zu trauen: Referenzfahrt wiederholen.",
            "Réinitialisation en plein mouvement. La position n'est plus fiable : refaites la prise d'origine.",
            "Reset durante il movimento. La posizione non è affidabile: rifare l'homing.",
            "移動中にリセットされました。位置は信頼できません。原点復帰をやり直してください。",
            "运动中复位。位置不可信：请重新回零。",
            "Reinício durante o movimento. Não se pode confiar na posição: volte a fazer o homing.",
        ],
        4 => [
            "Probe failed: the probe was not in its expected state before the cycle.",
            "Fallo de sonda: la sonda no estaba en su estado esperado antes del ciclo.",
            "Tastfehler: Der Taster war vor dem Zyklus nicht im erwarteten Zustand.",
            "Échec de la sonde : elle n'était pas dans l'état attendu avant le cycle.",
            "Sonda in errore: non era nello stato previsto prima del ciclo.",
            "プローブ失敗：サイクル開始前のプローブの状態が想定と異なります。",
            "探针失败：循环开始前探针不在预期状态。",
            "Falha da sonda: a sonda não estava no estado esperado antes do ciclo.",
        ],
        5 => [
            "Probe failed: it touched nothing within the travel asked for.",
            "Fallo de sonda: no tocó nada en el recorrido pedido.",
            "Tastfehler: Innerhalb des geforderten Wegs wurde nichts berührt.",
            "Échec de la sonde : elle n'a rien touché sur la course demandée.",
            "Sonda in errore: non ha toccato nulla nella corsa richiesta.",
            "プローブ失敗：指定した移動量の間に何も接触しませんでした。",
            "探针失败：在要求的行程内没有接触到任何东西。",
            "Falha da sonda: não tocou em nada no curso pedido.",
        ],
        6 => [
            "Homing failed: reset during the cycle.",
            "Fallo de homing: reinicio durante el ciclo.",
            "Referenzfahrt fehlgeschlagen: Zurücksetzen während des Zyklus.",
            "Échec de la prise d'origine : réinitialisation pendant le cycle.",
            "Homing fallito: reset durante il ciclo.",
            "原点復帰失敗：サイクル中にリセットされました。",
            "回零失败：循环中被复位。",
            "Falha do homing: reinício durante o ciclo.",
        ],
        7 => [
            "Homing failed: the door was opened during the cycle.",
            "Fallo de homing: se abrió la puerta durante el ciclo.",
            "Referenzfahrt fehlgeschlagen: Die Tür wurde während des Zyklus geöffnet.",
            "Échec de la prise d'origine : la porte a été ouverte pendant le cycle.",
            "Homing fallito: lo sportello è stato aperto durante il ciclo.",
            "原点復帰失敗：サイクル中にドアが開きました。",
            "回零失败：循环中门被打开。",
            "Falha do homing: a porta foi aberta durante o ciclo.",
        ],
        8 => [
            "Homing failed: the limit switch was not cleared when pulling off. Raise the pull-off or check the wiring.",
            "Fallo de homing: no se liberó el final de carrera al retirarse. Aumente el retroceso o revise el cableado.",
            "Referenzfahrt fehlgeschlagen: Der Endschalter wurde beim Freifahren nicht freigegeben. Freifahrweg erhöhen oder Verkabelung prüfen.",
            "Échec de la prise d'origine : le fin de course n'a pas été libéré au dégagement. Augmentez le dégagement ou vérifiez le câblage.",
            "Homing fallito: il finecorsa non è stato liberato nel ritiro. Aumentare il ritiro o controllare il cablaggio.",
            "原点復帰失敗：引き戻し時にリミットスイッチが解除されませんでした。引き戻し量を増やすか配線を確認してください。",
            "回零失败：退让时未能释放限位开关。请增大退让距离或检查接线。",
            "Falha do homing: o fim de curso não foi libertado ao recuar. Aumente o recuo ou verifique a cablagem.",
        ],
        9 => [
            "Homing failed: no limit switch was found within the search distance.",
            "Fallo de homing: no se encontró el final de carrera en la distancia de búsqueda.",
            "Referenzfahrt fehlgeschlagen: Innerhalb des Suchwegs wurde kein Endschalter gefunden.",
            "Échec de la prise d'origine : aucun fin de course trouvé sur la distance de recherche.",
            "Homing fallito: nessun finecorsa trovato nella distanza di ricerca.",
            "原点復帰失敗：探索距離内にリミットスイッチが見つかりませんでした。",
            "回零失败：在搜索距离内未找到限位开关。",
            "Falha do homing: não se encontrou o fim de curso na distância de procura.",
        ],
        _ => return None,
    };
    Some(row[column(lang)])
}

/// The text of `error:N` in the language; `None` for a number not in the table.
pub fn error(code: u32, lang: &str) -> Option<&'static str> {
    let row: [&str; 8] = match code {
        1 => [
            "A G-code word has no letter.",
            "Una palabra de G-code no tiene letra.",
            "Einem G-Code-Wort fehlt der Buchstabe.",
            "Un mot de G-code n'a pas de lettre.",
            "Una parola di G-code non ha la lettera.",
            "Gコードのワードに文字がありません。",
            "G 代码字缺少字母。",
            "Uma palavra de G-code não tem letra.",
        ],
        2 => [
            "A number is not valid or is missing.",
            "Un número no es válido o falta.",
            "Eine Zahl ist ungültig oder fehlt.",
            "Un nombre n'est pas valide ou manque.",
            "Un numero non è valido o manca.",
            "数値が無効または不足しています。",
            "数字无效或缺失。",
            "Um número não é válido ou falta.",
        ],
        3 => [
            "The '$' command is not known or not supported.",
            "El comando '$' no se conoce o no se admite.",
            "Der '$'-Befehl ist unbekannt oder wird nicht unterstützt.",
            "La commande '$' est inconnue ou non prise en charge.",
            "Il comando '$' non è riconosciuto o non è supportato.",
            "'$' コマンドが未知か未対応です。",
            "'$' 命令未知或不受支持。",
            "O comando '$' não é conhecido ou não é suportado.",
        ],
        5 => [
            "Homing is not enabled in the settings.",
            "El homing no está activado en los ajustes.",
            "Die Referenzfahrt ist in den Einstellungen nicht aktiviert.",
            "La prise d'origine n'est pas activée dans les réglages.",
            "L'homing non è abilitato nelle impostazioni.",
            "設定で原点復帰が有効になっていません。",
            "设置中未启用回零。",
            "O homing não está ativado nas definições.",
        ],
        8 => [
            "The '$' command only works while the controller is idle.",
            "El comando '$' solo funciona con el controlador en reposo.",
            "Der '$'-Befehl funktioniert nur im Leerlauf der Steuerung.",
            "La commande '$' ne marche que lorsque le contrôleur est au repos.",
            "Il comando '$' funziona solo a controllore fermo.",
            "'$' コマンドはコントローラーが待機中のときだけ使えます。",
            "'$' 命令仅在控制器空闲时可用。",
            "O comando '$' só funciona com o controlador em repouso.",
        ],
        9 => [
            "G-code is locked while the controller is in an alarm or a jog.",
            "El G-code está bloqueado con el controlador en alarma o en jog.",
            "G-Code ist gesperrt, solange die Steuerung im Alarm oder im Tippbetrieb ist.",
            "Le G-code est verrouillé tant que le contrôleur est en alarme ou en jog.",
            "Il G-code è bloccato mentre il controllore è in allarme o in jog.",
            "アラーム中またはジョグ中はGコードが使えません。",
            "控制器报警或点动期间 G 代码被锁定。",
            "O G-code está bloqueado enquanto o controlador está em alarme ou em jog.",
        ],
        10 => [
            "Soft limits cannot be on without homing.",
            "Los límites blandos no pueden estar activos sin homing.",
            "Software-Endschalter sind ohne Referenzfahrt nicht möglich.",
            "Les limites logicielles ne peuvent pas être actives sans prise d'origine.",
            "I limiti software non possono essere attivi senza homing.",
            "原点復帰なしではソフトリミットを有効にできません。",
            "没有回零时不能启用软限位。",
            "Os limites por software não podem estar ativos sem homing.",
        ],
        11 => [
            "The line is too long and was not run.",
            "La línea es demasiado larga y no se ejecutó.",
            "Die Zeile ist zu lang und wurde nicht ausgeführt.",
            "La ligne est trop longue et n'a pas été exécutée.",
            "La riga è troppo lunga e non è stata eseguita.",
            "行が長すぎるため実行されませんでした。",
            "该行过长，未执行。",
            "A linha é demasiado longa e não foi executada.",
        ],
        15 => [
            "The jog goes past the travel of the machine and was ignored.",
            "El jog sale del recorrido de la máquina y se ignoró.",
            "Der Tippbetrieb überschreitet den Verfahrweg und wurde ignoriert.",
            "Le jog dépasse la course de la machine et a été ignoré.",
            "Il jog supera la corsa della macchina ed è stato ignorato.",
            "ジョグが機械の可動範囲を超えるため無視されました。",
            "点动超出机床行程，已忽略。",
            "O jog ultrapassa o curso da máquina e foi ignorado.",
        ],
        20 => [
            "The line has a G-code command that is not supported or not valid.",
            "La línea tiene un comando G-code no admitido o no válido.",
            "Die Zeile enthält einen nicht unterstützten oder ungültigen G-Code-Befehl.",
            "La ligne contient une commande G-code non prise en charge ou non valide.",
            "La riga contiene un comando G-code non supportato o non valido.",
            "この行には未対応または無効なGコードがあります。",
            "该行含有不支持或无效的 G 代码命令。",
            "A linha tem um comando G-code não suportado ou inválido.",
        ],
        21 => [
            "The line has two commands of the same modal group.",
            "La línea tiene dos comandos del mismo grupo modal.",
            "Die Zeile enthält zwei Befehle derselben Modalgruppe.",
            "La ligne contient deux commandes du même groupe modal.",
            "La riga contiene due comandi dello stesso gruppo modale.",
            "この行には同じモーダルグループのコマンドが2つあります。",
            "该行含有同一模态组的两个命令。",
            "A linha tem dois comandos do mesmo grupo modal.",
        ],
        22 => [
            "No feed has been set yet.",
            "Todavía no se ha fijado ningún avance.",
            "Es wurde noch kein Vorschub gesetzt.",
            "Aucune avance n'a encore été définie.",
            "Non è ancora stato impostato alcun avanzamento.",
            "送り速度がまだ設定されていません。",
            "尚未设定进给速度。",
            "Ainda não foi definido nenhum avanço.",
        ],
        23 => [
            "A command needs a whole number.",
            "Un comando necesita un número entero.",
            "Ein Befehl benötigt eine ganze Zahl.",
            "Une commande demande un nombre entier.",
            "Un comando richiede un numero intero.",
            "コマンドには整数が必要です。",
            "命令需要整数。",
            "Um comando precisa de um número inteiro.",
        ],
        24 => [
            "Two commands in the line both need the axis words.",
            "Dos comandos de la línea necesitan ambos las palabras de eje.",
            "Zwei Befehle in der Zeile benötigen beide die Achsenwörter.",
            "Deux commandes de la ligne demandent toutes deux les mots d'axe.",
            "Due comandi nella riga richiedono entrambi le parole degli assi.",
            "この行の2つのコマンドがどちらも軸ワードを必要としています。",
            "该行中有两个命令都需要轴字。",
            "Dois comandos da linha precisam ambos das palavras de eixo.",
        ],
        33 => [
            "The target of the move is not valid for this command.",
            "El destino del movimiento no es válido para este comando.",
            "Das Bewegungsziel ist für diesen Befehl ungültig.",
            "La cible du mouvement n'est pas valide pour cette commande.",
            "La destinazione del movimento non è valida per questo comando.",
            "このコマンドでは移動先が無効です。",
            "该命令的运动目标无效。",
            "O destino do movimento não é válido para este comando.",
        ],
        _ => return None,
    };
    Some(row[column(lang)])
}

/// The text of a refusal or an alarm of GRBL in the language, or the text the controller gave when the number is in no table.
pub fn explain_grbl(kind: &str, code: Option<u32>, given: &str, lang: &str) -> String {
    let known = code.and_then(|c| match kind {
        "alarm" => alarm(c, lang),
        _ => error(c, lang),
    });
    match known {
        Some(t) => t.to_string(),
        None => given.to_string(),
    }
}

/// The errors (`error:N`) that grblHAL has and GRBL 1.1 does not.
pub const HAL_ERRORS: [u32; 10] = [39, 40, 41, 44, 45, 46, 50, 51, 52, 53];

/// The alarms (`ALARM:N`) that grblHAL has and GRBL 1.1 does not.
pub const HAL_ALARMS: [u32; 6] = [10, 11, 12, 13, 14, 17];

/// The kinds of error of Marlin, with the number the program gives each.
pub const MARLIN_KINDS: [u32; 8] = [101, 102, 103, 104, 105, 106, 107, 108];

/// The kinds of error of Klipper, with the number the program gives each.
pub const KLIPPER_KINDS: [u32; 8] = [201, 202, 203, 204, 205, 206, 207, 208];

/// The failures of the HTTP transport of Klipper, with the number the program gives each (they follow the kinds above).
pub const KLIPPER_LINK_KINDS: [u32; 11] = crate::httprpc::HTTP_KINDS;

/// The text of `error:N` of grblHAL in the language; `None` for a number that is not one of the ones it adds.
pub fn hal_error(code: u32, lang: &str) -> Option<&'static str> {
    let row: [&str; 8] = match code {
        39 => [
            "A value of the line is out of the range the controller takes.",
            "Un valor de la línea está fuera del rango que admite el controlador.",
            "Ein Wert der Zeile liegt außerhalb des Bereichs, den die Steuerung annimmt.",
            "Une valeur de la ligne est hors de la plage acceptée par le contrôleur.",
            "Un valore della riga è fuori dall'intervallo accettato dal controllore.",
            "行内の値がコントローラーの受け付ける範囲外です。",
            "该行的某个值超出了控制器可接受的范围。",
            "Um valor da linha está fora do intervalo que o controlador aceita.",
        ],
        40 => [
            "A tool change is waiting: finish it before the next move.",
            "Hay un cambio de herramienta pendiente: termínelo antes del siguiente movimiento.",
            "Ein Werkzeugwechsel steht aus: Schließen Sie ihn vor der nächsten Bewegung ab.",
            "Un changement d'outil est en attente : terminez-le avant le mouvement suivant.",
            "Un cambio utensile è in attesa: terminarlo prima del movimento successivo.",
            "工具交換が保留中です。次の移動の前に完了してください。",
            "有换刀正在等待：请在下一次运动前完成。",
            "Há uma troca de ferramenta pendente: conclua-a antes do movimento seguinte.",
        ],
        41 => [
            "The spindle is not running.",
            "El husillo no está en marcha.",
            "Die Spindel läuft nicht.",
            "La broche ne tourne pas.",
            "Il mandrino non è in rotazione.",
            "スピンドルが回転していません。",
            "主轴没有运转。",
            "O fuso não está a rodar.",
        ],
        44 => [
            "The spindle speed is out of the range of the machine.",
            "La velocidad del husillo está fuera del rango de la máquina.",
            "Die Spindeldrehzahl liegt außerhalb des Bereichs der Maschine.",
            "La vitesse de la broche est hors de la plage de la machine.",
            "La velocità del mandrino è fuori dall'intervallo della macchina.",
            "スピンドル回転数が機械の範囲外です。",
            "主轴转速超出了机床的范围。",
            "A velocidade do fuso está fora do intervalo da máquina.",
        ],
        45 => [
            "A limit switch is pressed: the move is refused.",
            "Un final de carrera está pulsado: se rechaza el movimiento.",
            "Ein Endschalter ist betätigt: Die Bewegung wird abgelehnt.",
            "Une fin de course est enfoncée : le mouvement est refusé.",
            "Un finecorsa è premuto: il movimento è rifiutato.",
            "リミットスイッチが押されています。移動は拒否されました。",
            "限位开关被触发：运动被拒绝。",
            "Um fim de curso está premido: o movimento é recusado.",
        ],
        46 => [
            "The machine must be homed before it moves.",
            "Hay que hacer el home de la máquina antes de moverla.",
            "Die Maschine muss vor der Bewegung referenziert werden.",
            "La machine doit faire sa prise d'origine avant de bouger.",
            "La macchina deve fare l'homing prima di muoversi.",
            "移動する前に機械の原点復帰が必要です。",
            "机床移动前必须先回零。",
            "A máquina tem de fazer o homing antes de se mover.",
        ],
        50 => [
            "The emergency stop is active.",
            "La parada de emergencia está activa.",
            "Der Not-Halt ist aktiv.",
            "L'arrêt d'urgence est actif.",
            "L'arresto di emergenza è attivo.",
            "非常停止が作動中です。",
            "急停处于激活状态。",
            "A paragem de emergência está ativa.",
        ],
        51 => [
            "A motor driver reports a fault.",
            "Un driver de motor informa de un fallo.",
            "Ein Motortreiber meldet einen Fehler.",
            "Un driver de moteur signale un défaut.",
            "Un driver di motore segnala un guasto.",
            "モータードライバーが異常を報告しています。",
            "某个电机驱动器报告故障。",
            "Um driver de motor comunica uma falha.",
        ],
        52 => [
            "The value of the setting is out of range.",
            "El valor del ajuste está fuera de rango.",
            "Der Wert der Einstellung liegt außerhalb des Bereichs.",
            "La valeur du réglage est hors plage.",
            "Il valore dell'impostazione è fuori intervallo.",
            "設定値が範囲外です。",
            "设置值超出范围。",
            "O valor da definição está fora do intervalo.",
        ],
        53 => [
            "The setting is disabled in this build.",
            "El ajuste está desactivado en esta compilación.",
            "Die Einstellung ist in diesem Build deaktiviert.",
            "Le réglage est désactivé dans cette version.",
            "L'impostazione è disattivata in questa versione.",
            "この版では設定が無効になっています。",
            "该设置在此版本中被禁用。",
            "A definição está desativada nesta versão.",
        ],
        _ => return None,
    };
    Some(row[column(lang)])
}

/// The text of `ALARM:N` of grblHAL in the language; `None` for a number that is not one of the ones it adds.
pub fn hal_alarm(code: u32, lang: &str) -> Option<&'static str> {
    let row: [&str; 8] = match code {
        10 => [
            "The emergency stop was pressed. Release it and unlock the machine.",
            "Se pulsó la parada de emergencia. Suéltela y desbloquee la máquina.",
            "Der Not-Halt wurde gedrückt. Lösen Sie ihn und entsperren Sie die Maschine.",
            "L'arrêt d'urgence a été enfoncé. Relâchez-le et déverrouillez la machine.",
            "È stato premuto l'arresto di emergenza. Rilasciarlo e sbloccare la macchina.",
            "非常停止が押されました。解除して機械のロックを外してください。",
            "急停已被按下。请释放并解锁机床。",
            "A paragem de emergência foi premida. Solte-a e desbloqueie a máquina.",
        ],
        11 => [
            "The machine must be homed before it moves.",
            "Hay que hacer el home de la máquina antes de moverla.",
            "Die Maschine muss vor der Bewegung referenziert werden.",
            "La machine doit faire sa prise d'origine avant de bouger.",
            "La macchina deve fare l'homing prima di muoversi.",
            "移動する前に機械の原点復帰が必要です。",
            "机床移动前必须先回零。",
            "A máquina tem de fazer o homing antes de se mover.",
        ],
        12 => [
            "A limit switch is pressed at start or after a reset.",
            "Un final de carrera está pulsado al arrancar o tras un reinicio.",
            "Ein Endschalter ist beim Start oder nach einem Reset betätigt.",
            "Une fin de course est enfoncée au démarrage ou après une réinitialisation.",
            "Un finecorsa è premuto all'avvio o dopo un reset.",
            "起動時またはリセット後にリミットスイッチが押されています。",
            "启动或复位后限位开关处于触发状态。",
            "Um fim de curso está premido ao arrancar ou após um reinício.",
        ],
        13 => [
            "The probe is already triggered before the cycle: the probe protection stopped the machine.",
            "La sonda ya está activada antes del ciclo: la protección de la sonda detuvo la máquina.",
            "Der Taster ist schon vor dem Zyklus ausgelöst: Der Tasterschutz hat die Maschine angehalten.",
            "La sonde est déjà déclenchée avant le cycle : la protection de la sonde a arrêté la machine.",
            "La sonda è già attivata prima del ciclo: la protezione della sonda ha fermato la macchina.",
            "サイクル前にプローブが既に作動しています。プローブ保護により機械を停止しました。",
            "循环开始前探针已被触发：探针保护使机床停止。",
            "A sonda já está acionada antes do ciclo: a proteção da sonda parou a máquina.",
        ],
        14 => [
            "The spindle did not reach its speed in time.",
            "El husillo no alcanzó su velocidad a tiempo.",
            "Die Spindel hat ihre Drehzahl nicht rechtzeitig erreicht.",
            "La broche n'a pas atteint sa vitesse à temps.",
            "Il mandrino non ha raggiunto la velocità in tempo.",
            "スピンドルが時間内に回転数に達しませんでした。",
            "主轴未能及时达到转速。",
            "O fuso não atingiu a sua velocidade a tempo.",
        ],
        17 => [
            "A motor driver reports a fault.",
            "Un driver de motor informa de un fallo.",
            "Ein Motortreiber meldet einen Fehler.",
            "Un driver de moteur signale un défaut.",
            "Un driver di motore segnala un guasto.",
            "モータードライバーが異常を報告しています。",
            "某个电机驱动器报告故障。",
            "Um driver de motor comunica uma falha.",
        ],
        _ => return None,
    };
    Some(row[column(lang)])
}

/// The text of a kind of error of Marlin (101 and up) in the language; `None` for another number.
pub fn marlin(code: u32, lang: &str) -> Option<&'static str> {
    let row: [&str; 8] = match code {
        101 => [
            "Marlin does not know this command.",
            "Marlin no conoce este comando.",
            "Marlin kennt diesen Befehl nicht.",
            "Marlin ne connaît pas cette commande.",
            "Marlin non conosce questo comando.",
            "Marlin はこのコマンドを認識しません。",
            "Marlin 不认识此命令。",
            "O Marlin não conhece este comando.",
        ],
        102 => [
            "The number of the line is not the one that follows the last: the line is asked for again.",
            "El número de línea no es el siguiente al último: se pide la línea de nuevo.",
            "Die Zeilennummer folgt nicht auf die letzte: Die Zeile wird erneut angefordert.",
            "Le numéro de ligne ne suit pas le dernier : la ligne est redemandée.",
            "Il numero di riga non segue l'ultimo: la riga viene richiesta di nuovo.",
            "行番号が直前の番号に続いていません。行を再要求します。",
            "行号不是上一行号的下一个：将重新请求该行。",
            "O número da linha não é o que segue o último: a linha é pedida de novo.",
        ],
        103 => [
            "The checksum of the line does not match: the line was spoiled on the way.",
            "La suma de comprobación de la línea no coincide: la línea se estropeó por el camino.",
            "Die Prüfsumme der Zeile stimmt nicht: Die Zeile wurde unterwegs verfälscht.",
            "La somme de contrôle de la ligne ne correspond pas : la ligne a été altérée en chemin.",
            "Il checksum della riga non corrisponde: la riga si è rovinata lungo il percorso.",
            "行のチェックサムが一致しません。回線上で行が壊れました。",
            "该行的校验和不匹配：这一行在传输途中被损坏。",
            "A soma de verificação da linha não coincide: a linha estragou-se pelo caminho.",
        ],
        104 => [
            "Homing failed: an end stop was not found.",
            "Fallo de homing: no se encontró un final de carrera.",
            "Referenzfahrt fehlgeschlagen: Ein Endschalter wurde nicht gefunden.",
            "Échec de la prise d'origine : une fin de course n'a pas été trouvée.",
            "Homing fallito: un finecorsa non è stato trovato.",
            "原点復帰失敗：エンドストップが見つかりませんでした。",
            "回零失败：未找到限位开关。",
            "Falha do homing: não foi encontrado um fim de curso.",
        ],
        105 => [
            "The probe failed: it touched nothing, or it was already triggered.",
            "Fallo de sonda: no tocó nada, o ya estaba activada.",
            "Tastfehler: Der Taster hat nichts berührt oder war schon ausgelöst.",
            "Échec de la sonde : elle n'a rien touché, ou elle était déjà déclenchée.",
            "Sonda in errore: non ha toccato nulla, oppure era già attivata.",
            "プローブ失敗：何にも接触しなかったか、既に作動していました。",
            "探针失败：没有接触到任何东西，或一开始就已触发。",
            "Falha da sonda: não tocou em nada, ou já estava acionada.",
        ],
        106 => [
            "Marlin is halted. Nothing moves until it is unlocked (and some halts need a reset of the board).",
            "Marlin está detenido. Nada se mueve hasta desbloquearlo (y algunas paradas piden reiniciar la placa).",
            "Marlin ist angehalten. Nichts bewegt sich, bis entsperrt wird (und manche Halts verlangen einen Reset der Platine).",
            "Marlin est à l'arrêt. Rien ne bouge avant le déverrouillage (et certains arrêts demandent de réinitialiser la carte).",
            "Marlin è fermo. Nulla si muove finché non viene sbloccato (e alcuni arresti richiedono il reset della scheda).",
            "Marlin は停止しています。ロック解除まで何も動きません（基板のリセットが必要な停止もあります）。",
            "Marlin 已停机。解锁之前什么都不会动（有些停机还需要复位主板）。",
            "O Marlin está parado. Nada se move até ser desbloqueado (e algumas paragens pedem o reinício da placa).",
        ],
        107 => [
            "A temperature error stopped Marlin. Look at the heater and the sensor before going on.",
            "Un error de temperatura detuvo Marlin. Revise el calentador y el sensor antes de seguir.",
            "Ein Temperaturfehler hat Marlin angehalten. Prüfen Sie Heizung und Sensor, bevor Sie weitermachen.",
            "Une erreur de température a arrêté Marlin. Vérifiez le chauffage et la sonde avant de continuer.",
            "Un errore di temperatura ha fermato Marlin. Controllare il riscaldatore e il sensore prima di proseguire.",
            "温度エラーで Marlin が停止しました。続行する前にヒーターとセンサーを確認してください。",
            "温度错误使 Marlin 停机。继续之前请检查加热器和传感器。",
            "Um erro de temperatura parou o Marlin. Verifique o aquecedor e o sensor antes de continuar.",
        ],
        108 => [
            "Marlin reported an error that this program does not know: its own text follows.",
            "Marlin informó de un error que este programa no conoce: sigue su propio texto.",
            "Marlin hat einen Fehler gemeldet, den dieses Programm nicht kennt: Sein eigener Text folgt.",
            "Marlin a signalé une erreur que ce programme ne connaît pas : son propre texte suit.",
            "Marlin ha segnalato un errore che questo programma non conosce: segue il suo testo.",
            "Marlin がこのプログラムの知らないエラーを報告しました。Marlin 自身のテキストを表示します。",
            "Marlin 报告了本程序不认识的错误：下面是它自己的文字。",
            "O Marlin comunicou um erro que este programa não conhece: segue-se o seu próprio texto.",
        ],
        _ => return None,
    };
    Some(row[column(lang)])
}

/// The text of a kind of error of Klipper in the language: `201` to `208`, the numbers `klipper::classify` gives, and `209` to `219`, the
/// failures of the HTTP transport ([`crate::httprpc`]); `None` for a number that is not one of them.
pub fn klipper(code: u32, lang: &str) -> Option<&'static str> {
    let row: [&str; 8] = match code {
        201 => [
            "Klipper does not know this command.",
            "Klipper no conoce este comando.",
            "Klipper kennt diesen Befehl nicht.",
            "Klipper ne connaît pas cette commande.",
            "Klipper non conosce questo comando.",
            "Klipper はこのコマンドを認識しません。",
            "Klipper 不认识此命令。",
            "O Klipper não conhece este comando.",
        ],
        202 => [
            "An axis must be homed before it moves.",
            "Un eje debe hacer el homing antes de moverse.",
            "Eine Achse muss referenziert sein, bevor sie fährt.",
            "Un axe doit faire sa prise d'origine avant de bouger.",
            "Un asse deve fare l'homing prima di muoversi.",
            "軸は移動する前に原点復帰が必要です。",
            "轴在移动之前必须先回零。",
            "Um eixo tem de fazer o homing antes de se mover.",
        ],
        203 => [
            "The move goes past the travel of the machine and was not made.",
            "El movimiento sale del recorrido de la máquina y no se hizo.",
            "Die Bewegung überschreitet den Verfahrweg der Maschine und wurde nicht ausgeführt.",
            "Le mouvement dépasse la course de la machine et n'a pas été fait.",
            "Il movimento supera la corsa della macchina e non è stato eseguito.",
            "移動が機械のストロークを超えているため、実行されませんでした。",
            "该移动超出了机器的行程，未执行。",
            "O movimento sai do curso da máquina e não foi feito.",
        ],
        204 => [
            "Klipper is shut down or lost the connection to a board: the machine is stopped until it is unlocked.",
            "Klipper está apagado o perdió la conexión con una placa: la máquina está parada hasta que se desbloquee.",
            "Klipper ist heruntergefahren oder hat die Verbindung zu einer Platine verloren: Die Maschine steht, bis sie entsperrt wird.",
            "Klipper est arrêté ou a perdu la liaison avec une carte : la machine est arrêtée jusqu'à son déverrouillage.",
            "Klipper è in arresto o ha perso il collegamento con una scheda: la macchina resta ferma fino allo sblocco.",
            "Klipper が停止したか、ボードとの接続を失いました。ロック解除までマシンは停止します。",
            "Klipper 已关闭或与某块板失去连接：在解锁之前机器保持停止。",
            "O Klipper está desligado ou perdeu a ligação a uma placa: a máquina fica parada até ser desbloqueada.",
        ],
        205 => [
            "A heater or a temperature sensor failed: Klipper stopped the machine.",
            "Falló un calentador o un sensor de temperatura: Klipper paró la máquina.",
            "Eine Heizung oder ein Temperatursensor ist ausgefallen: Klipper hat die Maschine angehalten.",
            "Un chauffage ou un capteur de température est en défaut : Klipper a arrêté la machine.",
            "Un riscaldatore o un sensore di temperatura è guasto: Klipper ha fermato la macchina.",
            "ヒーターまたは温度センサーが故障したため、Klipper がマシンを停止しました。",
            "加热器或温度传感器出现故障：Klipper 已停止机器。",
            "Um aquecedor ou um sensor de temperatura falhou: o Klipper parou a máquina.",
        ],
        206 => [
            "Klipper is not ready: it is starting or must be restarted.",
            "Klipper no está listo: está arrancando o debe reiniciarse.",
            "Klipper ist nicht bereit: Es startet gerade oder muss neu gestartet werden.",
            "Klipper n'est pas prêt : il démarre ou doit être redémarré.",
            "Klipper non è pronto: si sta avviando o deve essere riavviato.",
            "Klipper の準備ができていません。起動中か、再起動が必要です。",
            "Klipper 尚未就绪：正在启动或需要重启。",
            "O Klipper não está pronto: está a arrancar ou tem de ser reiniciado.",
        ],
        207 => [
            "The message was not understood: it arrived spoiled or incomplete.",
            "El mensaje no se entendió: llegó estropeado o incompleto.",
            "Die Nachricht wurde nicht verstanden: Sie kam verfälscht oder unvollständig an.",
            "Le message n'a pas été compris : il est arrivé altéré ou incomplet.",
            "Il messaggio non è stato compreso: è arrivato rovinato o incompleto.",
            "メッセージを理解できませんでした。壊れているか不完全な状態で届きました。",
            "无法理解该消息：它在传输中损坏或不完整。",
            "A mensagem não foi compreendida: chegou estragada ou incompleta.",
        ],
        208 => [
            "Klipper refused the line.",
            "Klipper rechazó la línea.",
            "Klipper hat die Zeile abgelehnt.",
            "Klipper a refusé la ligne.",
            "Klipper ha rifiutato la riga.",
            "Klipper は行を拒否しました。",
            "Klipper 拒绝了该行。",
            "O Klipper recusou a linha.",
        ],
        209 => [
            "Moonraker answered with an HTTP error status.",
            "Moonraker respondió con un estado de error HTTP.",
            "Moonraker hat mit einem HTTP-Fehlerstatus geantwortet.",
            "Moonraker a répondu par un statut d'erreur HTTP.",
            "Moonraker ha risposto con uno stato di errore HTTP.",
            "Moonraker が HTTP エラーステータスで応答しました。",
            "Moonraker 返回了 HTTP 错误状态。",
            "O Moonraker respondeu com um estado de erro HTTP.",
        ],
        210 => [
            "Moonraker refused the credential or asked for one: check the API key.",
            "Moonraker rechazó la credencial o pidió una: compruebe la clave de API.",
            "Moonraker hat den Zugangsschlüssel abgelehnt oder verlangt einen: Prüfen Sie den API-Schlüssel.",
            "Moonraker a refusé l'identifiant ou en demande un : vérifiez la clé d'API.",
            "Moonraker ha rifiutato la credenziale o ne richiede una: controlli la chiave API.",
            "Moonraker が認証情報を拒否したか、認証情報を要求しています。API キーを確認してください。",
            "Moonraker 拒绝了凭据或要求提供凭据：请检查 API 密钥。",
            "O Moonraker recusou a credencial ou pediu uma: verifique a chave de API.",
        ],
        211 => [
            "Moonraker answered with a redirection, which is not followed: check the address and the port.",
            "Moonraker respondió con una redirección, que no se sigue: compruebe la dirección y el puerto.",
            "Moonraker hat mit einer Weiterleitung geantwortet, der nicht gefolgt wird: Prüfen Sie Adresse und Port.",
            "Moonraker a répondu par une redirection, qui n'est pas suivie : vérifiez l'adresse et le port.",
            "Moonraker ha risposto con un reindirizzamento, che non viene seguito: controlli l'indirizzo e la porta.",
            "Moonraker がリダイレクトで応答しましたが、追従しません。アドレスとポートを確認してください。",
            "Moonraker 返回了重定向，但不会跟随：请检查地址和端口。",
            "O Moonraker respondeu com um redirecionamento, que não é seguido: verifique o endereço e a porta.",
        ],
        212 => [
            "The answer of Moonraker is larger than the limit set.",
            "La respuesta de Moonraker supera el límite establecido.",
            "Die Antwort von Moonraker ist größer als die festgelegte Grenze.",
            "La réponse de Moonraker dépasse la limite fixée.",
            "La risposta di Moonraker supera il limite impostato.",
            "Moonraker の応答が設定された上限を超えています。",
            "Moonraker 的响应超过了设定的上限。",
            "A resposta do Moonraker excede o limite definido.",
        ],
        213 => [
            "The answer of Moonraker is not JSON.",
            "La respuesta de Moonraker no es JSON.",
            "Die Antwort von Moonraker ist kein JSON.",
            "La réponse de Moonraker n'est pas du JSON.",
            "La risposta di Moonraker non è JSON.",
            "Moonraker の応答は JSON ではありません。",
            "Moonraker 的响应不是 JSON。",
            "A resposta do Moonraker não é JSON.",
        ],
        214 => [
            "The answer of Moonraker is not well-formed HTTP.",
            "La respuesta de Moonraker no es HTTP bien formado.",
            "Die Antwort von Moonraker ist kein wohlgeformtes HTTP.",
            "La réponse de Moonraker n'est pas du HTTP bien formé.",
            "La risposta di Moonraker non è HTTP ben formato.",
            "Moonraker の応答は正しい形式の HTTP ではありません。",
            "Moonraker 的响应不是格式正确的 HTTP。",
            "A resposta do Moonraker não é HTTP bem formado.",
        ],
        215 => [
            "The address of Moonraker is refused: only the local network is allowed unless the switch for other networks is on.",
            "Se rechaza la dirección de Moonraker: solo se admite la red local mientras no se active el interruptor de otras redes.",
            "Die Adresse von Moonraker wird abgelehnt: Erlaubt ist nur das lokale Netz, solange der Schalter für andere Netze aus ist.",
            "L'adresse de Moonraker est refusée : seul le réseau local est autorisé tant que l'interrupteur des autres réseaux est désactivé.",
            "L'indirizzo di Moonraker è rifiutato: è ammessa solo la rete locale finché l'interruttore per le altre reti è spento.",
            "Moonraker のアドレスは拒否されました。他のネットワークを許可するスイッチがオフの間は、ローカルネットワークのみ使用できます。",
            "Moonraker 的地址被拒绝：在“允许其他网络”开关关闭时，仅允许本地网络。",
            "O endereço do Moonraker é recusado: só a rede local é permitida enquanto o interruptor das outras redes estiver desligado.",
        ],
        216 => [
            "Too many requests are waiting for Moonraker.",
            "Hay demasiadas peticiones esperando a Moonraker.",
            "Zu viele Anfragen warten auf Moonraker.",
            "Trop de requêtes attendent Moonraker.",
            "Troppe richieste sono in attesa di Moonraker.",
            "Moonraker への要求が多すぎて待機しています。",
            "等待 Moonraker 的请求过多。",
            "Há demasiados pedidos à espera do Moonraker.",
        ],
        217 => [
            "Moonraker cannot be reached at the address and port given.",
            "No se puede alcanzar a Moonraker en la dirección y el puerto indicados.",
            "Moonraker ist unter der angegebenen Adresse und dem angegebenen Port nicht erreichbar.",
            "Moonraker est injoignable à l'adresse et au port indiqués.",
            "Moonraker non è raggiungibile all'indirizzo e alla porta indicati.",
            "指定されたアドレスとポートで Moonraker に接続できません。",
            "无法通过指定的地址和端口连接到 Moonraker。",
            "Não é possível ligar ao Moonraker no endereço e na porta indicados.",
        ],
        218 => [
            "The connection to Moonraker closed before the answer was complete.",
            "La conexión con Moonraker se cerró antes de completar la respuesta.",
            "Die Verbindung zu Moonraker wurde geschlossen, bevor die Antwort vollständig war.",
            "La connexion à Moonraker s'est fermée avant la fin de la réponse.",
            "La connessione a Moonraker si è chiusa prima che la risposta fosse completa.",
            "応答が完了する前に Moonraker との接続が閉じられました。",
            "在响应完成之前，与 Moonraker 的连接已关闭。",
            "A ligação ao Moonraker fechou antes de a resposta estar completa.",
        ],
        219 => [
            "Moonraker took too long to answer.",
            "Moonraker tardó demasiado en responder.",
            "Moonraker hat zu lange für die Antwort gebraucht.",
            "Moonraker a mis trop de temps à répondre.",
            "Moonraker ha impiegato troppo tempo a rispondere.",
            "Moonraker の応答に時間がかかりすぎました。",
            "Moonraker 响应超时。",
            "O Moonraker demorou demasiado a responder.",
        ],
        _ => return None,
    };
    Some(row[column(lang)])
}

/// The text of a refusal or an alarm of a controller of this family in the language: the table of GRBL for the families that speak GRBL,
/// the numbers that only grblHAL has, the kinds of Marlin and of Klipper, and the text the controller gave when no table has the number (`kind` is
/// `alarm` or `error`).
pub fn explain(family: Family, kind: &str, code: Option<u32>, given: &str, lang: &str) -> String {
    let own = code.and_then(|c| match (family, kind) {
        (Family::Marlin, _) => marlin(c, lang),
        (Family::Klipper, _) => klipper(c, lang),
        (Family::GrblHal, "alarm") => hal_alarm(c, lang),
        (Family::GrblHal, _) => hal_error(c, lang),
        _ => None,
    });
    match own {
        Some(t) => t.to_string(),
        None => match family {
            Family::Marlin | Family::Klipper => given.to_string(),
            _ => explain_grbl(kind, code, given, lang),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_eight(texts: Vec<&'static str>, what: &str) {
        assert_eq!(texts.len(), 8, "{what}");
        assert!(texts.iter().all(|t| t.len() > 8), "{what}");
        let mut un = texts.clone();
        un.sort_unstable();
        un.dedup();
        assert_eq!(un.len(), 8, "{what}: two languages say the same");
    }

    #[test]
    fn every_number_of_the_dialects_has_all_eight_languages_and_they_differ() {
        for c in HAL_ERRORS {
            all_eight(
                LANGS.iter().map(|l| hal_error(c, l).unwrap()).collect(),
                &format!("hal error {c}"),
            );
        }
        for c in HAL_ALARMS {
            all_eight(
                LANGS.iter().map(|l| hal_alarm(c, l).unwrap()).collect(),
                &format!("hal alarm {c}"),
            );
        }
        for c in MARLIN_KINDS {
            all_eight(
                LANGS.iter().map(|l| marlin(c, l).unwrap()).collect(),
                &format!("marlin {c}"),
            );
        }
        for c in KLIPPER_KINDS {
            all_eight(
                LANGS.iter().map(|l| klipper(c, l).unwrap()).collect(),
                &format!("klipper {c}"),
            );
        }
        for c in KLIPPER_LINK_KINDS {
            all_eight(
                LANGS.iter().map(|l| klipper(c, l).unwrap()).collect(),
                &format!("klipper link {c}"),
            );
        }
        assert!(klipper(220, "en").is_none() && klipper(200, "en").is_none());
    }

    #[test]
    fn the_numbers_of_grbl_are_not_redefined_and_the_numbers_of_marlin_are_not_those_of_grbl() {
        for c in HAL_ERRORS {
            assert!(
                error(c, "en").is_none(),
                "{c} is already in the table of GRBL"
            );
        }
        for c in HAL_ALARMS {
            assert!(alarm(c, "en").is_none(), "{c}");
        }
        for c in MARLIN_KINDS {
            assert!(c > 100 && error(c, "en").is_none());
        }
        for c in KLIPPER_KINDS.into_iter().chain(KLIPPER_LINK_KINDS) {
            assert!(c > 200 && error(c, "en").is_none() && marlin(c, "en").is_none());
        }
        // the numbers of the two groups are one run, with no number in two groups
        let all: Vec<u32> = KLIPPER_KINDS
            .into_iter()
            .chain(KLIPPER_LINK_KINDS)
            .collect();
        assert_eq!(all, (201..=219).collect::<Vec<u32>>());
        assert!(
            hal_error(20, "en").is_none()
                && hal_alarm(1, "en").is_none()
                && marlin(9, "en").is_none()
        );
    }

    #[test]
    fn explain_picks_the_table_of_the_family_and_keeps_the_text_of_the_controller_otherwise() {
        // GRBL numbers go through the table of GRBL for the families that speak it
        assert_eq!(
            explain(Family::GrblHal, "error", Some(22), "x", "fr"),
            "Aucune avance n'a encore été définie."
        );
        // a number that only grblHAL has
        assert!(explain(Family::GrblHal, "error", Some(46), "x", "es").contains("home"));
        assert!(explain(Family::GrblHal, "alarm", Some(10), "x", "de").contains("Not-Halt"));
        // the same number means nothing for plain GRBL
        assert_eq!(
            explain(Family::Grbl, "error", Some(46), "error:46", "es"),
            "error:46"
        );
        // Marlin
        assert!(explain(Family::Marlin, "error", Some(104), "x", "en").starts_with("Homing failed"));
        assert_eq!(
            explain(Family::Marlin, "error", Some(22), "raw text", "en"),
            "raw text"
        );
        assert_eq!(explain(Family::Marlin, "alarm", None, "raw", "ja"), "raw");
        // Klipper
        assert!(explain(Family::Klipper, "error", Some(202), "x", "en").contains("homed"));
        assert!(explain(Family::Klipper, "alarm", Some(204), "x", "de").contains("Klipper"));
        assert_eq!(
            explain(Family::Klipper, "error", Some(22), "raw text", "en"),
            "raw text"
        );
    }

    #[test]
    fn the_portuguese_of_the_dialects_is_that_of_portugal() {
        let mut text = String::new();
        for c in HAL_ERRORS {
            text.push_str(hal_error(c, "pt").unwrap());
        }
        for c in MARLIN_KINDS {
            text.push_str(marlin(c, "pt").unwrap());
        }
        for c in KLIPPER_KINDS.into_iter().chain(KLIPPER_LINK_KINDS) {
            text.push_str(klipper(c, "pt").unwrap());
        }
        assert!(text.contains("definição") || text.contains("ficheiro") || text.contains("fuso"));
        for brazilian in [
            "configuração",
            "arquivo",
            "fuso horário",
            "tela",
            "celular",
            "usuário",
        ] {
            assert!(!text.contains(brazilian), "{brazilian}");
        }
    }
}
