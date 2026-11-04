//! The words of the window in the eight languages of the repository (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal).

/// The languages of the window, in the order the lists of words below follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Zh,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two letters of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The name of the language in the language itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Pt => "Português",
        }
    }

    /// The language of a code such as `es`, `es_ES.UTF-8` or `pt-PT`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let two: String = code
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == two)
    }

    /// The language of the person from the variables of the system; English when it is none of the eight.
    pub fn from_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() {
                    return Lang::from_code(&v).unwrap_or(Lang::En);
                }
            }
        }
        Lang::En
    }
}

/// Puts the values in the places `{0}`, `{1}`... of a text.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = template.to_string();
    for (k, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Every word and sentence of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    Open,
    Export,
    Measure,
    Check,
    Language,
    Fit,
    DropHint,
    Layers,
    Parameters,
    Opacity,
    Margin,
    Tolerance,
    ArcTolerance,
    Remove,
    Results,
    ZoomHint,
    OpenTitle,
    ExportTitle,
    Folder,
    Up,
    AllFiles,
    Cancel,
    EmptyFolder,
    FileName,
    Ready,
    Opened,
    Exported,
    NoLayer,
    Failed,
    BadValue,
    SameFile,
    NotSvg,
    TooLarge,
    Incomplete,
    NothingToDraw,
    DrillTag,
    ReportNarrowest,
    ReportNoTracks,
    ReportGap,
    ReportFew,
    ReportPads,
    ReportLined,
    ReportHoles,
    ReportNothing,
    ReportFinding,
}

impl Msg {
    pub const ALL: [Msg; 45] = [
        Msg::Open,
        Msg::Export,
        Msg::Measure,
        Msg::Check,
        Msg::Language,
        Msg::Fit,
        Msg::DropHint,
        Msg::Layers,
        Msg::Parameters,
        Msg::Opacity,
        Msg::Margin,
        Msg::Tolerance,
        Msg::ArcTolerance,
        Msg::Remove,
        Msg::Results,
        Msg::ZoomHint,
        Msg::OpenTitle,
        Msg::ExportTitle,
        Msg::Folder,
        Msg::Up,
        Msg::AllFiles,
        Msg::Cancel,
        Msg::EmptyFolder,
        Msg::FileName,
        Msg::Ready,
        Msg::Opened,
        Msg::Exported,
        Msg::NoLayer,
        Msg::Failed,
        Msg::BadValue,
        Msg::SameFile,
        Msg::NotSvg,
        Msg::TooLarge,
        Msg::Incomplete,
        Msg::NothingToDraw,
        Msg::DrillTag,
        Msg::ReportNarrowest,
        Msg::ReportNoTracks,
        Msg::ReportGap,
        Msg::ReportFew,
        Msg::ReportPads,
        Msg::ReportLined,
        Msg::ReportHoles,
        Msg::ReportNothing,
        Msg::ReportFinding,
    ];

    /// The words in the order English, Spanish, German, French, Italian, Japanese, Chinese, Portuguese.
    fn words(self) -> [&'static str; 8] {
        match self {
            Msg::Open => [
                "Add layer...",
                "Añadir capa...",
                "Ebene hinzufügen...",
                "Ajouter une couche...",
                "Aggiungi livello...",
                "レイヤーを追加...",
                "添加图层...",
                "Adicionar camada...",
            ],
            Msg::Export => [
                "Export SVG...",
                "Exportar SVG...",
                "SVG exportieren...",
                "Exporter en SVG...",
                "Esporta SVG...",
                "SVG に書き出す...",
                "导出 SVG...",
                "Exportar SVG...",
            ],
            Msg::Measure => [
                "Measure the layer",
                "Medir la capa",
                "Ebene messen",
                "Mesurer la couche",
                "Misura il livello",
                "レイヤーを測定",
                "测量图层",
                "Medir a camada",
            ],
            Msg::Check => [
                "Check the alignment",
                "Comprobar la alineación",
                "Ausrichtung prüfen",
                "Vérifier l'alignement",
                "Controlla l'allineamento",
                "位置合わせを確認",
                "检查对齐",
                "Verificar o alinhamento",
            ],
            Msg::Language => [
                "Language",
                "Idioma",
                "Sprache",
                "Langue",
                "Lingua",
                "言語",
                "语言",
                "Idioma",
            ],
            Msg::Fit => [
                "Fit",
                "Ajustar",
                "Einpassen",
                "Ajuster",
                "Adatta",
                "全体表示",
                "适应窗口",
                "Ajustar",
            ],
            Msg::DropHint => [
                "Drop Gerber layers or drill files here or press Add",
                "Suelte aquí capas Gerber o ficheros de taladros o pulse Añadir",
                "Gerber-Ebenen oder Bohrdateien hierher ziehen oder auf Hinzufügen klicken",
                "Déposez des couches Gerber ou des fichiers de perçage ici ou appuyez sur Ajouter",
                "Trascina qui livelli Gerber o file di foratura o premi Aggiungi",
                "Gerber レイヤーやドリルファイルをここにドロップするか「追加」を押してください",
                "将 Gerber 层或钻孔文件拖到此处，或点击“添加”",
                "Largue aqui camadas Gerber ou ficheiros de furação ou prima Adicionar",
            ],
            Msg::Layers => [
                "Layers",
                "Capas",
                "Ebenen",
                "Couches",
                "Livelli",
                "レイヤー",
                "图层",
                "Camadas",
            ],
            Msg::Parameters => [
                "Parameters",
                "Parámetros",
                "Parameter",
                "Paramètres",
                "Parametri",
                "パラメーター",
                "参数",
                "Parâmetros",
            ],
            Msg::Opacity => [
                "Opacity (0 to 1)",
                "Opacidad (0 a 1)",
                "Deckkraft (0 bis 1)",
                "Opacité (0 à 1)",
                "Opacità (da 0 a 1)",
                "不透明度 (0〜1)",
                "不透明度 (0 到 1)",
                "Opacidade (0 a 1)",
            ],
            Msg::Margin => [
                "Margin of the picture (mm)",
                "Margen de la imagen (mm)",
                "Rand des Bildes (mm)",
                "Marge de l'image (mm)",
                "Margine dell'immagine (mm)",
                "画像の余白 (mm)",
                "图片边距 (mm)",
                "Margem da imagem (mm)",
            ],
            Msg::Tolerance => [
                "Alignment tolerance (mm)",
                "Tolerancia de alineación (mm)",
                "Ausrichtungstoleranz (mm)",
                "Tolérance d'alignement (mm)",
                "Tolleranza di allineamento (mm)",
                "位置合わせの許容差 (mm)",
                "对齐容差 (mm)",
                "Tolerância de alinhamento (mm)",
            ],
            Msg::ArcTolerance => [
                "Arc tolerance (mm)",
                "Tolerancia de los arcos (mm)",
                "Bogentoleranz (mm)",
                "Tolérance des arcs (mm)",
                "Tolleranza degli archi (mm)",
                "円弧の許容差 (mm)",
                "圆弧容差 (mm)",
                "Tolerância dos arcos (mm)",
            ],
            Msg::Remove => [
                "Remove",
                "Quitar",
                "Entfernen",
                "Retirer",
                "Rimuovi",
                "削除",
                "移除",
                "Remover",
            ],
            Msg::Results => [
                "Results",
                "Resultados",
                "Ergebnisse",
                "Résultats",
                "Risultati",
                "結果",
                "结果",
                "Resultados",
            ],
            Msg::ZoomHint => [
                "Wheel: zoom - drag: move",
                "Rueda: zoom - arrastrar: mover",
                "Rad: Zoom - Ziehen: verschieben",
                "Molette : zoom - glisser : déplacer",
                "Rotella: zoom - trascina: sposta",
                "ホイール: 拡大縮小 - ドラッグ: 移動",
                "滚轮：缩放 - 拖动：移动",
                "Roda: zoom - arrastar: mover",
            ],
            Msg::OpenTitle => [
                "Add a Gerber layer or a drill file",
                "Añadir una capa Gerber o un fichero de taladros",
                "Gerber-Ebene oder Bohrdatei hinzufügen",
                "Ajouter une couche Gerber ou un fichier de perçage",
                "Aggiungi un livello Gerber o un file di foratura",
                "Gerber レイヤーまたはドリルファイルを追加",
                "添加 Gerber 层或钻孔文件",
                "Adicionar uma camada Gerber ou um ficheiro de furação",
            ],
            Msg::ExportTitle => [
                "Export the picture (.svg)",
                "Exportar la imagen (.svg)",
                "Bild exportieren (.svg)",
                "Exporter l'image (.svg)",
                "Esporta l'immagine (.svg)",
                "画像を書き出す (.svg)",
                "导出图片 (.svg)",
                "Exportar a imagem (.svg)",
            ],
            Msg::Folder => [
                "Folder",
                "Carpeta",
                "Ordner",
                "Dossier",
                "Cartella",
                "フォルダー",
                "文件夹",
                "Pasta",
            ],
            Msg::Up => [
                "Up",
                "Subir",
                "Nach oben",
                "Dossier parent",
                "Su",
                "上へ",
                "上一级",
                "Subir",
            ],
            Msg::AllFiles => [
                "All files",
                "Todos los ficheros",
                "Alle Dateien",
                "Tous les fichiers",
                "Tutti i file",
                "すべてのファイル",
                "所有文件",
                "Todos os ficheiros",
            ],
            Msg::Cancel => [
                "Cancel",
                "Cancelar",
                "Abbrechen",
                "Annuler",
                "Annulla",
                "キャンセル",
                "取消",
                "Cancelar",
            ],
            Msg::EmptyFolder => [
                "(nothing to open here)",
                "(nada que abrir aquí)",
                "(hier gibt es nichts zu öffnen)",
                "(rien à ouvrir ici)",
                "(niente da aprire qui)",
                "(ここに開けるものはありません)",
                "(这里没有可打开的内容)",
                "(nada para abrir aqui)",
            ],
            Msg::FileName => [
                "File name",
                "Nombre del fichero",
                "Dateiname",
                "Nom du fichier",
                "Nome del file",
                "ファイル名",
                "文件名",
                "Nome do ficheiro",
            ],
            Msg::Ready => [
                "Ready",
                "Listo",
                "Bereit",
                "Prêt",
                "Pronto",
                "準備完了",
                "就绪",
                "Pronto",
            ],
            Msg::Opened => [
                "{0}: {1} shapes, {2} x {3} mm",
                "{0}: {1} formas, {2} x {3} mm",
                "{0}: {1} Formen, {2} x {3} mm",
                "{0} : {1} formes, {2} x {3} mm",
                "{0}: {1} forme, {2} x {3} mm",
                "{0}: 図形 {1} 個、{2} x {3} mm",
                "{0}：{1} 个图形，{2} x {3} mm",
                "{0}: {1} formas, {2} x {3} mm",
            ],
            Msg::Exported => [
                "Written to {0}",
                "Escrito en {0}",
                "Geschrieben nach {0}",
                "Écrit dans {0}",
                "Scritto in {0}",
                "{0} に書き出しました",
                "已写入 {0}",
                "Escrito em {0}",
            ],
            Msg::NoLayer => [
                "Add a layer first",
                "Añada primero una capa",
                "Zuerst eine Ebene hinzufügen",
                "Ajoutez d'abord une couche",
                "Aggiungi prima un livello",
                "先にレイヤーを追加してください",
                "请先添加图层",
                "Adicione primeiro uma camada",
            ],
            Msg::Failed => [
                "Error: {0}",
                "Error: {0}",
                "Fehler: {0}",
                "Erreur : {0}",
                "Errore: {0}",
                "エラー: {0}",
                "错误：{0}",
                "Erro: {0}",
            ],
            Msg::BadValue => [
                "{0}: {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0} : {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0}：{1}",
                "{0}: {1}",
            ],
            Msg::SameFile => [
                "The output file is the layer itself; choose another name",
                "El fichero de salida es la propia capa; elija otro nombre",
                "Die Ausgabedatei ist die Ebene selbst; einen anderen Namen wählen",
                "Le fichier de sortie est la couche elle-même ; choisissez un autre nom",
                "Il file di uscita è il livello stesso; scegli un altro nome",
                "出力ファイルがレイヤーそのものです。別の名前を選んでください",
                "输出文件就是该层本身；请选择其他名称",
                "O ficheiro de saída é a própria camada; escolha outro nome",
            ],
            Msg::NotSvg => [
                "The picture file must end in .svg",
                "El fichero de la imagen debe acabar en .svg",
                "Die Bilddatei muss auf .svg enden",
                "Le fichier de l'image doit se terminer par .svg",
                "Il file dell'immagine deve finire con .svg",
                "画像ファイルの名前は .svg で終わる必要があります",
                "图片文件名必须以 .svg 结尾",
                "O ficheiro da imagem deve terminar em .svg",
            ],
            Msg::TooLarge => [
                "{0}: the file is above the limit ({1} against {2})",
                "{0}: el fichero supera el límite ({1} frente a {2})",
                "{0}: die Datei überschreitet die Grenze ({1} gegenüber {2})",
                "{0} : le fichier dépasse la limite ({1} contre {2})",
                "{0}: il file supera il limite ({1} contro {2})",
                "{0}: ファイルが上限を超えています（{1}、上限 {2}）",
                "{0}：文件超过上限（{1}，上限 {2}）",
                "{0}: o ficheiro excede o limite ({1} contra {2})",
            ],
            Msg::Incomplete => [
                "{0}: the reader found {1} error(s): the layer is not complete",
                "{0}: el lector encontró {1} error(es): la capa no está completa",
                "{0}: der Leser fand {1} Fehler: die Ebene ist nicht vollständig",
                "{0} : le lecteur a trouvé {1} erreur(s) : la couche n'est pas complète",
                "{0}: il lettore ha trovato {1} errori: il livello non è completo",
                "{0}: リーダーが {1} 件のエラーを検出しました。レイヤーは不完全です",
                "{0}：读取器发现 {1} 个错误：该层不完整",
                "{0}: o leitor encontrou {1} erro(s): a camada não está completa",
            ],
            Msg::NothingToDraw => [
                "{0}: has nothing to draw",
                "{0}: no tiene nada que dibujar",
                "{0}: enthält nichts zum Zeichnen",
                "{0} : n'a rien à dessiner",
                "{0}: non ha nulla da disegnare",
                "{0}: 描画するものがありません",
                "{0}：没有可绘制的内容",
                "{0}: não tem nada para desenhar",
            ],
            Msg::DrillTag => [
                "drill",
                "taladros",
                "Bohrungen",
                "perçages",
                "foratura",
                "ドリル",
                "钻孔",
                "furação",
            ],
            Msg::ReportNarrowest => [
                "Narrowest track: {0} mm",
                "Pista más estrecha: {0} mm",
                "Schmalste Leiterbahn: {0} mm",
                "Piste la plus étroite : {0} mm",
                "Pista più stretta: {0} mm",
                "最も細いトラック: {0} mm",
                "最窄走线：{0} mm",
                "Pista mais estreita: {0} mm",
            ],
            Msg::ReportNoTracks => [
                "No tracks",
                "Sin pistas",
                "Keine Leiterbahnen",
                "Aucune piste",
                "Nessuna pista",
                "トラックなし",
                "没有走线",
                "Sem pistas",
            ],
            Msg::ReportGap => [
                "Smallest gap: {0} mm between shapes {1} and {2}",
                "Separación mínima: {0} mm entre las formas {1} y {2}",
                "Kleinster Abstand: {0} mm zwischen den Formen {1} und {2}",
                "Plus petit écart : {0} mm entre les formes {1} et {2}",
                "Distanza minima: {0} mm tra le forme {1} e {2}",
                "最小ギャップ: {0} mm（図形 {1} と {2} の間）",
                "最小间隙：{0} mm（图形 {1} 与 {2} 之间）",
                "Menor folga: {0} mm entre as formas {1} e {2}",
            ],
            Msg::ReportFew => [
                "Fewer than two pads or tracks",
                "Menos de dos pads o pistas",
                "Weniger als zwei Pads oder Leiterbahnen",
                "Moins de deux pastilles ou pistes",
                "Meno di due pad o piste",
                "パッドまたはトラックが2つ未満",
                "少于两个焊盘或走线",
                "Menos de dois pads ou pistas",
            ],
            Msg::ReportPads => [
                "{0} pads",
                "{0} pads",
                "{0} Pads",
                "{0} pastilles",
                "{0} pad",
                "パッド {0} 個",
                "{0} 个焊盘",
                "{0} pads",
            ],
            Msg::ReportLined => [
                "The layers line up",
                "Las capas están alineadas",
                "Die Ebenen passen zueinander",
                "Les couches sont alignées",
                "I livelli sono allineati",
                "レイヤーは揃っています",
                "各层对齐",
                "As camadas estão alinhadas",
            ],
            Msg::ReportHoles => [
                "The holes are inside the copper",
                "Los taladros están dentro del cobre",
                "Die Bohrungen liegen im Kupfer",
                "Les perçages sont dans le cuivre",
                "I fori sono dentro il rame",
                "穴は銅の内側にあります",
                "孔位于铜内",
                "Os furos estão dentro do cobre",
            ],
            Msg::ReportNothing => [
                "Fewer than two Gerber layers and no drill file with copper: nothing was checked",
                "Menos de dos capas Gerber y ningún fichero de taladros con cobre: no se comprobó nada",
                "Weniger als zwei Gerber-Ebenen und keine Bohrdatei mit Kupfer: es wurde nichts geprüft",
                "Moins de deux couches Gerber et aucun fichier de perçage avec du cuivre : rien n'a été vérifié",
                "Meno di due livelli Gerber e nessun file di foratura con rame: non è stato controllato nulla",
                "Gerber レイヤーが2つ未満で、銅と組になるドリルファイルもありません。何も確認していません",
                "Gerber 层少于两个，也没有与铜配合的钻孔文件：未检查任何内容",
                "Menos de duas camadas Gerber e nenhum ficheiro de furação com cobre: nada foi verificado",
            ],
            Msg::ReportFinding => [
                "{0}: {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0} : {1}",
                "{0}: {1}",
                "{0}: {1}",
                "{0}：{1}",
                "{0}: {1}",
            ],
        }
    }

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.words()[lang as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_has_a_text_in_each_of_the_eight_languages() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!m.tr(l).trim().is_empty(), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_list_of_sentences_has_no_repeats() {
        for (i, a) in Msg::ALL.iter().enumerate() {
            assert!(!Msg::ALL[i + 1..].contains(a), "{a:?} is listed twice");
        }
    }

    #[test]
    fn a_template_has_the_same_places_in_every_language() {
        for m in Msg::ALL {
            let places = |l: Lang| {
                (0..4)
                    .filter(|k| m.tr(l).contains(&format!("{{{k}}}")))
                    .count()
            };
            for l in Lang::ALL {
                assert_eq!(places(l), places(Lang::En), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_order_of_the_languages_matches_the_lists() {
        for (k, l) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(l as usize, k);
        }
        assert_eq!(Msg::Cancel.tr(Lang::Es), "Cancelar");
        assert_eq!(Msg::Cancel.tr(Lang::De), "Abbrechen");
        assert_eq!(Msg::Cancel.tr(Lang::Pt), "Cancelar");
    }

    #[test]
    fn a_language_is_found_from_the_code_of_the_system() {
        assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::from_code("PT-pt"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("xx"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn the_places_of_a_template_are_filled() {
        assert_eq!(fill("{1} and {0}", &["a", "b"]), "b and a");
        assert_eq!(fill("none", &["a"]), "none");
    }
}
