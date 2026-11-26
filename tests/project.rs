//! The `.eh3d` project: the job is read from `manufacturing/`, the result is kept in the branch of the viewer only, with a reference that tells when the job changed, and the window opens,
//! measures and saves. Every file is made here from invented data.

use eh3d_gerberviewer::eh3dfile::{self, JobFile, KIT, RESULT_DIR, SOURCE_BRANCH};
use eh3d_gerberviewer::lang::{Lang, Msg};
use eh3d_gerberviewer::ruler::Unit;
use eh3d_gerberviewer::window::{Status, Window};
use eh3d_project::{BranchUpdate, NewProject, OpenOptions, Project, SaveOptions, Writer};
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_eh3d-gerberview");
const COPPER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\n%ADD11R,2.0X1.0*%\nD10*\nX00000Y00000D02*\nX100000Y00000D01*\nD11*\nX50000Y50000D03*\nM02*\n";
const OTHER: &str =
    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX00000Y00000D02*\nX200000Y00000D01*\nM02*\n";
const DRILL: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX005.000Y005.000\nM30\n";

/// A project with a job of two layers (copper and drill) and a branch `pcb/` that nobody else may touch.
fn project(dir: &Path, copper: &str) -> PathBuf {
    let path = dir.join("board.eh3d");
    let new = NewProject {
        id: "test-id".into(),
        name: "Invented board".into(),
        units: "millimetres".into(),
        created: None,
    };
    Project::create(
        &path,
        new,
        &OpenOptions::default(),
        &SaveOptions::default().atomic,
    )
    .unwrap();
    write_branch(&path, "pcb", &[("board.json", b"{\"invented\":true}")]);
    write_job(&path, copper);
    path
}

fn write_branch(path: &Path, branch: &str, files: &[(&str, &[u8])]) {
    let mut update = BranchUpdate::new(branch, "1.0", "0.1.0");
    for (name, bytes) in files {
        update.entries.insert((*name).to_string(), bytes.to_vec());
    }
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .save_branch(&Writer::for_app(branch), update, &SaveOptions::default())
        .unwrap();
}

fn write_job(path: &Path, copper: &str) {
    let record = br#"{"format_version":"1.0.0","layers":[
        {"path":"layers/top.gbr","kind":"copper","name":"Top copper"},
        {"path":"layers/holes.drl","kind":"drill","name":"Holes"}]}"#;
    write_branch(
        path,
        SOURCE_BRANCH,
        &[
            ("project.json", record),
            ("layers/top.gbr", copper.as_bytes()),
            ("layers/holes.drl", DRILL.as_bytes()),
        ],
    );
}

fn hash(path: &Path, branch: &str) -> String {
    Project::open(path, &OpenOptions::default())
        .unwrap()
        .branch(branch)
        .unwrap()
        .hash
        .clone()
}

fn one_result(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    vec![("layers.svg".to_string(), bytes.to_vec())]
}

#[test]
fn the_job_is_read_from_the_manufacturing_branch_by_kind() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let all = eh3dfile::read_job(&path, &[]).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].rel, "layers/top.gbr");
    assert_eq!(all[0].file_name(), "top.gbr");
    assert_eq!(all[0].name, "Top copper");
    assert_eq!(all[0].bytes, COPPER.as_bytes());
    let drills = eh3dfile::read_job(&path, &["drill"]).unwrap();
    assert_eq!(drills.len(), 1);
    assert_eq!(drills[0].kind, "drill");
    assert!(eh3dfile::read_job(&path, &["paste"]).unwrap().is_empty());
    assert!(eh3dfile::is_project(&path));
    assert!(!eh3dfile::is_project(Path::new("top.gbr")));
}

#[test]
fn a_project_without_a_job_is_refused_with_its_reason() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.eh3d");
    let new = NewProject {
        id: "test-id".into(),
        name: "Empty".into(),
        units: "millimetres".into(),
        created: None,
    };
    Project::create(
        &path,
        new,
        &OpenOptions::default(),
        &SaveOptions::default().atomic,
    )
    .unwrap();
    let why = eh3dfile::read_job(&path, &[]).unwrap_err();
    assert!(why.contains(SOURCE_BRANCH), "{why}");
    assert!(eh3dfile::read_job(&dir.path().join("missing.eh3d"), &[]).is_err());
}

#[test]
fn the_result_goes_to_its_own_branch_and_the_other_branches_keep_their_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let (pcb, job_hash) = (hash(&path, "pcb"), hash(&path, SOURCE_BRANCH));
    let job = eh3dfile::read_job(&path, &[]).unwrap();
    let notices = eh3dfile::save_result(&KIT, &path, &job, &one_result(b"<svg/>")).unwrap();
    assert!(notices.is_empty(), "{notices:?}");
    assert_eq!(hash(&path, "pcb"), pcb);
    assert_eq!(hash(&path, SOURCE_BRANCH), job_hash);
    let state = eh3dfile::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["layers.svg"]);
    assert_eq!(state.inputs, ["layers/holes.drl", "layers/top.gbr"]);
    assert!(!state.out_of_date);
    assert!(state.warnings.is_empty(), "{:?}", state.warnings);
    assert_eq!(
        eh3dfile::read_result(&KIT, &path, "layers.svg").unwrap(),
        b"<svg/>"
    );
    assert!(eh3dfile::read_result(&KIT, &path, "other.svg").is_err());
}

#[test]
fn a_job_that_changes_leaves_the_result_out_of_date_until_it_is_made_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let job = eh3dfile::read_job(&path, &[]).unwrap();
    eh3dfile::save_result(&KIT, &path, &job, &one_result(b"<svg a/>")).unwrap();
    write_job(&path, OTHER);
    let state = eh3dfile::state(&KIT, &path).unwrap();
    assert!(state.out_of_date);
    assert!(state.warnings.iter().any(|w| w.starts_with("OUT OF DATE")));
    // a result made from the old layers is not saved over the new job
    let why = eh3dfile::save_result(&KIT, &path, &job, &one_result(b"<svg a/>")).unwrap_err();
    assert!(why.contains("changed while"), "{why}");
    let again = eh3dfile::read_job(&path, &[]).unwrap();
    eh3dfile::save_result(&KIT, &path, &again, &one_result(b"<svg b/>")).unwrap();
    assert!(!eh3dfile::state(&KIT, &path).unwrap().out_of_date);
}

#[test]
fn a_save_replaces_the_copies_and_results_that_were_there() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let job = eh3dfile::read_job(&path, &[]).unwrap();
    eh3dfile::save_result(&KIT, &path, &job, &one_result(b"<svg a/>")).unwrap();
    let copper: Vec<JobFile> = eh3dfile::read_job(&path, &["copper"]).unwrap();
    eh3dfile::save_result(
        &KIT,
        &path,
        &copper,
        &[("other.svg".to_string(), b"<svg b/>".to_vec())],
    )
    .unwrap();
    let state = eh3dfile::state(&KIT, &path).unwrap();
    assert_eq!(state.results, ["other.svg"]);
    assert_eq!(state.inputs, ["layers/top.gbr"]);
}

#[test]
fn nothing_is_written_when_there_is_nothing_to_save_or_a_name_is_not_safe() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let before = std::fs::read(&path).unwrap();
    let job = eh3dfile::read_job(&path, &[]).unwrap();
    assert!(eh3dfile::save_result(&KIT, &path, &[], &one_result(b"x")).is_err());
    assert!(eh3dfile::save_result(&KIT, &path, &job, &[]).is_err());
    let unsafe_name = vec![("../x".to_string(), b"x".to_vec())];
    assert!(eh3dfile::save_result(&KIT, &path, &job, &unsafe_name).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let missing = dir.path().join("missing.eh3d");
    assert!(eh3dfile::save_result(&KIT, &missing, &job, &one_result(b"x")).is_err());
    assert!(!missing.exists());
}

#[test]
fn the_command_line_draws_the_job_into_the_project_and_ends_with_2_when_it_is_out_of_date() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let run = |args: &[&str]| {
        Command::new(BIN)
            .arg("project")
            .args(args)
            .arg(&path)
            .output()
            .expect("the program runs")
    };
    assert_eq!(run(&["render"]).status.code(), Some(0));
    let svg = eh3dfile::read_result(&KIT, &path, "layers.svg").unwrap();
    assert!(svg.starts_with(b"<svg"));
    let info = run(&["info"]);
    assert_eq!(info.status.code(), Some(0));
    let text = String::from_utf8_lossy(&info.stdout);
    assert!(text.contains("layers/top.gbr (copper)"), "{text}");
    assert!(text.contains("layers.svg"), "{text}");
    write_job(&path, OTHER);
    assert_eq!(run(&["info"]).status.code(), Some(2));
    let wrong = Command::new(BIN)
        .args(["project", "nothing"])
        .output()
        .expect("the program runs");
    assert_eq!(wrong.status.code(), Some(64));
    assert_eq!(RESULT_DIR, "result/");
}

#[test]
fn the_window_opens_a_project_saves_its_picture_and_says_when_it_is_out_of_date() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let mut w = Window::new(Lang::En);
    assert!(!w.save_project());
    assert_eq!(w.status, Status::NoProject);
    assert!(w.add_layer(&path));
    assert_eq!(w.layers.len(), 2);
    assert!(w.layers[1].drill);
    assert!(matches!(w.status, Status::ProjectOpened { layers: 2, .. }));
    assert!(w.save_project());
    assert!(matches!(w.status, Status::ProjectSaved { .. }));
    write_job(&path, OTHER);
    assert!(w.open_project(&path));
    assert!(matches!(w.status, Status::ProjectStale { .. }));
    assert!(w.status.is_problem());
}

#[test]
fn the_layers_keep_their_own_opacity_and_order_and_a_hidden_one_is_warned() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let mut w = Window::new(Lang::En);
    assert!(w.add_layer(&path));
    assert!(w.stack_warnings().is_empty());
    assert_eq!(w.opacity_of(0), 0.85);
    assert!(w.set_layer_opacity(0, 0.0));
    assert!(!w.set_layer_opacity(0, 1.5));
    assert!(!w.set_layer_opacity(9, 0.5));
    assert_eq!(w.opacity_of(0), 0.0);
    assert_eq!(w.stack_warnings(), ["top.gbr can hardly be seen"]);
    assert_eq!(w.selected, Some(1));
    assert!(w.move_layer(1, 0));
    assert_eq!(w.selected, Some(0));
    assert_eq!(w.layers[0].name, "holes.drl");
    assert!(!w.move_layer(0, 0));
    assert!(!w.move_layer(0, 5));
}

#[test]
fn two_clicks_of_the_ruler_measure_from_the_pad_they_catch_in_the_unit_chosen() {
    let dir = tempfile::tempdir().unwrap();
    let top = dir.path().join("top.gtl");
    std::fs::write(&top, COPPER).unwrap();
    let mut w = Window::new(Lang::En);
    assert!(!w.ruler_click((0.0, 0.0)));
    assert!(w.add_layer(&top));
    assert!(w.ruler_lines().is_empty());
    w.set_ruler(true);
    assert_eq!(w.ruler_lines(), [Msg::RulerFirst.tr(Lang::En)]);
    // the rectangular pad is at 5, 5 mm: a click beside it is caught by its centre
    assert!(w.ruler_click((5.05, 5.0)));
    assert!(w.ruler_lines()[0].contains("centre of a pad"));
    assert!(w.ruler_click((8.0, 9.0)));
    let lines = w.ruler_lines();
    assert!(lines[2].starts_with("Distance 5.0000 mm"), "{lines:?}");
    w.set_unit(Unit::Mil);
    assert!(w.ruler_lines()[2].contains(" mil"));
    // a third click starts a new measure
    assert!(w.ruler_click((0.0, 0.0)));
    assert_eq!(w.ruler_lines().len(), 2);
    w.set_ruler(false);
    assert!(w.ruler_lines().is_empty());
}

#[test]
fn a_configuration_file_dropped_on_the_window_gives_the_settings() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("gerberview.cfg");
    std::fs::write(&cfg, "unit = mil\ndefault_opacity = 0.5\n").unwrap();
    let mut w = Window::new(Lang::En);
    assert!(w.add_layer(&cfg));
    assert_eq!(w.settings.unit, Unit::Mil);
    assert_eq!(w.opacity(), 0.5);
    assert!(matches!(w.status, Status::SettingsLoaded { .. }));
    std::fs::write(&cfg, "unit = furlong\n").unwrap();
    assert!(!w.load_settings(&cfg));
    assert_eq!(w.settings.unit, Unit::Mil);
}

#[test]
fn a_picture_is_never_written_over_a_project() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let before = std::fs::read(&path).unwrap();
    let layer = dir.path().join("top.gbr");
    std::fs::write(&layer, COPPER).unwrap();
    let out = Command::new(BIN)
        .arg("render")
        .arg(&path)
        .arg(&layer)
        .output()
        .expect("the program runs");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("project"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_damaged_project_is_refused_by_every_command_and_left_as_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let whole = std::fs::read(&path).unwrap();
    for cut in [0, 1, whole.len() / 2, whole.len() - 1] {
        std::fs::write(&path, &whole[..cut]).unwrap();
        for what in ["info", "render"] {
            let out = Command::new(BIN)
                .arg("project")
                .arg(what)
                .arg(&path)
                .output()
                .expect("the program runs");
            let code = out.status.code();
            assert!(
                code.is_some_and(|c| c != 0 && c != 101),
                "{what} {cut}: {code:?}"
            );
        }
        let mut w = Window::new(Lang::En);
        assert!(!w.add_layer(&path), "{cut}");
        assert!(w.layers.is_empty() && w.project.is_none(), "{cut}");
        assert_eq!(std::fs::read(&path).unwrap(), whole[..cut], "{cut}");
    }
}

#[test]
fn a_layer_with_a_hostile_name_does_not_get_into_the_markup_of_the_picture() {
    let dir = tempfile::tempdir().unwrap();
    let path = project(dir.path(), COPPER);
    let hostile = "layers/a\"><script>&.gbr";
    let record = br#"{"layers":[{"path":"layers/a\"><script>&.gbr","kind":"copper"}]}"#;
    write_branch(
        &path,
        SOURCE_BRANCH,
        &[("project.json", record), (hostile, COPPER.as_bytes())],
    );
    let out = Command::new(BIN)
        .arg("project")
        .arg("render")
        .arg(&path)
        .output()
        .expect("the program runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let svg = eh3dfile::read_result(&KIT, &path, "layers.svg").unwrap();
    let svg = String::from_utf8(svg).unwrap();
    assert!(!svg.contains("<script"), "{svg}");
    assert!(svg.contains("a&quot;&gt;&lt;script&gt;&amp;.gbr"), "{svg}");
}
