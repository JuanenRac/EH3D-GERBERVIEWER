# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **Third-party notices in every package (`docs/THIRD_PARTY_NOTICES`):** the packages of the releases now carry the copyright notices and licence texts of the Rust crates and of the data the program is built with, and the CI fails when a new dependency or licence is not in the file.

- **A supposed coordinate format is a doubt (safety):** a drill or Gerber file whose coordinate format or zero suppression had to be supposed may be at another scale, and nothing said so in the window or the checks. It is now treated like the other doubts: the scene says it, `--exact` refuses it and a gap measured on it is not conclusive. The shared readers are taken at v0.1.4, which gives these two warnings, leaves out zero sets and end positions and tells what it passes over. 1 test.

## [0.1.1] - The window says what a layer may be missing and checks no longer pass on copper that was not drawn

- **The window says what a layer may be missing (fix):** the warnings of the readers (a file with no end command, no units, text after its end...) and what is not drawn exactly (a hole left out, routes of a drill file, a macro aperture) were dropped without a word by the window and, for the warnings, by `project render`. They are listed in the results, with the reader's headline in the language of the window.
- **`gap --min` no longer passes on missing copper (fix):** with a macro aperture, a step and repeat or another shape left out, or a file that may be cut or has no units, a gap that is big enough ends with 2 as not conclusive. `measure` and `ruler` now write the same notes, and `measure` says how many rectangular pads, obround pads and regions it did not look at. `--exact` also refuses a file with no end command or no units.
- **`check` no longer calls good what it did not look at (fix):** two layers with nothing to draw "lined up", a slot outside the copper was not looked at (only round holes were) and a drill file with no hole gave "the holes are inside the copper". All three are findings now.
- 6 new tests of these cases.

- **Known limits of the README brought in line with the program:** the command `gap` compares every kind of shape between two layers, while `measure` and the window compare only round pads and tracks inside one layer; the two clicks of the ruler are no longer listed as pending. The eight languages say the same.
- **A picture is never written over a project:** `render OUT.svg LAYER...` with the two arguments the other way round (`render board.eh3d top.gbr`) wrote the picture over the `.eh3d` project and lost the work of every program in it; an output that ends in `.eh3d` is refused now.
- 3 new tests: damaged and cut projects are refused by every command and left as they were, and a hostile name of a layer does not get into the markup of the picture.

## [0.1.0] - Opens the shared project of the suite and measures with layer stacks

- **The reading and writing of the job of an `.eh3d` project are shared:** they are the module `eh3d_project::manufacturing` of EH3D-COMMON, the same for every manufacturing program; the commands, the window and the files of a project do not change.
- **The project of the suite (`.eh3d`):** `project info FILE` shows the job (the branch `manufacturing/`, a `.eh3dproj` package) and what is kept in the project, and `project render FILE` draws the layers of the job and keeps the picture in the branch `gerberviewer/` only, with a copy of the layers it was made from and a reference to the content of the job; a job that changes afterwards makes the picture out of date (exit 2). The other branches keep their content. 11 new tests with invented projects.
- **The window uses the ruler and the layer stack:** two clicks on the selected layer measure from the pad or vertex they catch (mm, inches or mils), each layer has its own opacity and can be raised or lowered, a hidden or almost invisible layer is warned, a dropped `.cfg` file gives the settings and a dropped `.eh3d` file opens its job so that the picture can be saved back. The words are in the eight languages; no person has looked at the window yet.
- The crate `eh3d-project` of EH3D-COMMON is now a dependency; Rust 1.89 or newer is needed.
- **Behaviour tests at the exact edge:** `tests/behaviour.rs` adds 38 deterministic tests without changing the behaviour: the zoom and pan of the window with seeded views, the list of files and the file browser, the triangles of a region (n corners give n minus 2 triangles with the same area, the largest region that is split), the limits of each setting and of the configuration file, the stack of layers (limits, order, warnings, the picture), the gaps and their pair limit, the ruler (the exact tolerance, ties, angles, the three lines in each unit), the header of a drill file, the words of the eight languages and the command line (a picture written twice is the same file, a picture that would overwrite a layer is refused).
- The shared workflow checker (`tools/workflow_hygiene.py`) is the current one of the documentation hub: the filters under a trigger (branches, paths, types) are no longer read as events.
- **Built against the released shared library:** the workflows take the tag v0.3.3 of EH3D-COMMON instead of its main branch.

## [0.0.9] - Exact gap, snapping ruler, layer stack and settings file

- **Requirements with identifiers and tests that name them:** `docs/REQUIREMENTS.md` is the register of what the program must do, one row for each capability with an identifier (`GRBV-001` on), its state (planned, prototype, implemented or validated), how far it is (done, partial, pending or blocked), its evidence and the test that shows it; a capability that is not written names the test that will show it. `tests/reqs.rs` reads the table and fails when a test does not exist, when a state does not agree with the progress or the evidence, when `docs/STATUS.md` does not say the same numbers, when a README does not link the documents or when a document holds a date or a private reference. The five documents of the family (`docs/SCOPE.md`, `docs/REQUIREMENTS.md`, `docs/STATUS.md`, `docs/NOT_YET.md` and `AGENTS.md`) are in the repository and are linked from the eight READMEs.
- **The gap between any two shapes of two layers is measured exactly:** the new command `gap LAYER_A LAYER_B` finds the closest pair between a round, rectangular or obround pad, a track and a region of one layer and those of the other, edge to edge, and names both with their kind; a gap of 0 means they touch or overlap. With `--min MM` the command ends with 2 when the gap is below the minimum, the code of a check that ran and was not passed. Shapes with a number that is not valid are counted and reported instead of hidden, and a comparison of more pairs than the limit is an error that says how many there were.
- **A ruler between two points with snap:** the new command `ruler LAYER X0 Y0 X1 Y1` catches each point with the nearest vertex or pad centre within a tolerance and prints the distance, the two differences and the angle in millimetres, inches or mils (`--unit`), naming the pad, track or region each end caught. This is the logic that a click on the picture will use.
- **A stack of layers with an opacity and an order:** a layer has an opacity from 0 to 1 (anything else is refused and the layer stays as it was), the order of composition can be changed, and the stack warns of a shown layer that is almost invisible or that may be hidden under a solid layer above it. A software picture of the stack paints each layer as a whole, so the opacity does not add up where two shapes of one layer overlap.
- **A configuration file for the tuned values:** `examples/gerberview.cfg` lists, with their defaults, the snap tolerance, the unit, the minimum visible opacity, the opacity of a new layer and the limits of layers and of compared pairs; `--config FILE` reads it in `ruler` and `gap`. A value outside its range, an unknown key or a key written twice is an error that names its line.

## [0.0.8] - Stricter drill header and option checks

- **The CI and the release build take the shared libraries at their newest releases:** the pins of the sibling repositories in the workflows are raised to the releases the program now needs, so that the build and the binaries of a release use them.
- **The README says the maturity of the manifest:** the badge and the Maturity row of the eight READMEs read functional, as the manifest does, and the check of the status lights now fails when they differ.
- **`render` writes the picture through a temporary file and never over a layer:** the SVG is written beside and renamed, so a failure leaves no half picture under its name, and an output that is one of the layers once the paths are resolved (`./a.txt` for `a.txt`) is refused.
- **`render` and `check` say what a layer is not drawn exactly:** each note (clear polarity drawn as dark, step and repeat, a macro aperture) goes to the error output, and the new option `--exact` makes both refuse to write the picture or to judge the layers while there is any.
- **The drill header may be followed by a comment or a setting:** `M48;made by...`, `M48 ;...` and `M48 METRIC` are drill files whatever their name is, as before for `M48` and `M48,...`; `M481` and `M48X1` still are not.
- **An option of another command is refused by name:** `info --exact`, `measure --opacity 0.5`, `check --margin 1` and `render --tolerance 1` end with 64 and say which option does not belong to which command, instead of reading the option as a layer that cannot be opened.
- **`render` refuses an opacity above 1:** `--opacity 7` ends with 64 (the picture used to clamp it to 1 without saying); 0 to 1 is accepted.

## [0.0.7] - Fuzz and golden-output tests and the shared command-line reader

- **The shared validation tools are updated:** the validator also runs the workflow and lessons checks of the suite with their latest rules, including a check to run before every push.
- **The CI validation checks the workflows and the lessons of past CI failures against a baseline:** `tools/workflow_hygiene.py` and `tools/lessons_lint.py` (copies of the shared tools) run in the validation, and only findings that are not in `tools/workflow_hygiene_baseline.json` and `tools/lessons_lint_baseline.json` fail it, so what exists today stays as it is and a new one is caught. `python tools/workflow_hygiene.py --root . --detail` and `python tools/lessons_lint.py --repo .` list the findings.
- **The validation of the CI is the reusable one of the documentation hub:** the job `validate` calls `validate.yml` of EH3D-DOCS (pinned by commit) and the check of the CHANGELOG is a job of its own.
- **Fuzz tests with a fixed seed:** every prefix of the example layer, 400 damaged copies of it and bytes that are not a layer are read into a scene without a panic and without a number that is not finite.

- **The options of the command line are read by the shared `cli_args.rs`:** the same file that the other small programs of the suite carry (an option with no value is an error that names it, a number must be finite), in place of a reader of its own.

- The exit code follows the convention of the suite on the command line: 64 for a wrong use (an unknown command, option or setting, or something that is missing), 1 for a failure of the work.

- **Golden outputs on the example files:** three tests run `info`, `measure` and `check` on `examples/sample.gtl` and `examples/sample.ncd` and compare the output with the numbers worked out by hand from those files (the shapes and the box of each file, the narrowest track, the pads, and the holes inside the copper).

## [0.0.6] - The window with several layers, drills and measurements, and the Windows icon

- **Binaries for every system:** `release-binaries.yml` builds the program with its window for Windows (64 and 32 bits), Linux and macOS (Apple silicon and Intel), packs each one with the licence, the readme, the icon and the examples, writes `SHA256SUMS` and a build manifest, attests their provenance, verifies the artifact as a stranger would, prepares a draft release only after the owner approves it (a dry run is the default) and opens an issue `build-failed` when anything does not finish.
- **The Windows executable carries the icon of the program:** the file shows it in the explorer and the taskbar, together with the name, the description and the version, embedded by `build.rs`; other systems are not affected.
- **A window:** built with `--features gui`, `eh3d-gerberview` with no arguments (or with a single layer file, or `gui [LAYER...]`) opens a window in the look of the other programs of the suite (amber on charcoal, the icon of the program). Layers and drill files are added from the list of files of the window or dropped on it; each one can be shown or hidden and taken out; they are drawn one over the other with the opacity of the parameters, with zoom (wheel) and pan (drag); the narrowest track, the smallest gap and the pads of the chosen layer can be measured; the alignment of the layers and the holes inside the copper can be checked; and the layers that are shown are exported as one SVG picture with the margin of the parameters. The command line works as before, and a build without the feature is only a command line.
- **The window speaks the eight languages:** English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal, chosen from the system and changeable in the window.
- **The logic of the window needs no screen:** `window` (layers, parameters, measures, checks, export, status), `viewport` (zoom and pan), `draw` (the shapes and the triangles of the regions), `browse` (the list of files) and `lang` (the words) are plain code with their own tests; `gui` only draws them.
- **A drill file is told by its header:** a file that starts with `M48` is read as a drill file whatever its extension is, and `check` counts it as one.
- **The layers are compared by their centres, and a test says so:** layers with the same centre and another size line up.
- **A test draws the public Gerber files of the readers** inside the box their summaries expect.
- **The commands of the README are run by a test.**
- **A picture mode for the window:** `--shot FILE.png` (built with `--features gui`) opens the window at the size of `--size WxH` (320 to 8192, 1180x760 by default), with `--theme forge|dark|light`, `--lang CODE` and the files of `--open FILE`, waits `--wait N` frames (0 to 3600, 6 by default), saves what it drew as a PNG file and closes by itself. An existing file is never replaced unless `--replace` is given, the window keeps no settings in this mode, the exit code is 0 when the file was written, 1 when it was not (also when the picture does not come back within 600 frames) and 2 for wrong arguments, and a build without the window says so. The arguments are read by plain code with its tests.

## [0.0.5] - Audit fixes, the new reading limits, tests and the application icon

- **The application icon and the logos of the program:** `images/brand/` holds the icon (SVG, PNG sizes, `.ico` and `.icns`), the horizontal and vertical logos in a light and a dark version and the social preview image.
- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- **Safer render:** `render` refuses an output file that looks like a layer or is also an input, so a layer is never overwritten by the picture; `.drd` files are drill files and a name with no dot is not.
- **Limited reading and arc tolerance:** a layer is refused by its size before it is read into memory (`--max-bytes`, message in the eight languages) and `--arc-tolerance` sets how closely arcs are followed.
- **A damaged layer is not drawn in silence:** the notices of the reader are shown, and every command fails (exit code 1) with a layer that has reading errors or nothing to draw.
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.
- **More tests:** the measures and the clearance at their limits, the comparison of the layers, the holes outside the copper, the colours of the layers and the picture with odd colours, opacities and names.

## [0.0.4] - CI hardening, toolchain pin and shared validator

- **Faster, bounded CI:** the Cargo build is cached, a run that a newer push supersedes is cancelled, every job has a time limit, and the Rust jobs live in their own workflow that is skipped when only documents or images change.
- **A fixed Rust toolchain:** `rust-toolchain.toml` names the compiler version (with rustfmt and clippy) that the CI installs, so a new stable release cannot turn the lints red without a change here.
- **Monthly dependency updates:** Dependabot opens one grouped pull request a month for the GitHub Actions.
- **A line for every change:** a pull request that changes code must add a line under `[Unreleased]` of this file (`tools/changelog_pr_check.py`; the label `no-changelog` skips it).
- **Shared checks updated:** the validator of the suite checks the text of the banner against the manifest and reads the files that git tracks.
- **`check` says when it had nothing to compare:** with only a drill file, or a single layer, it reports that there are no layers to compare instead of saying that the layers line up.
- **`measure` refuses extra arguments** with a usage error instead of ignoring them.
- **More tests:** the comparison of layers at the exact tolerance, holes that touch the copper, tracks that overlap or run parallel, the limit of shapes that are measured pair by pair, colours and opacities that are not valid, the names of layers in the picture and every command of the command line. Two layers with the same file name are now both compared.

## [0.0.3] - Shapes from the shared geometry crate

- **The shapes come from the shared geometry crate:** `scene` re-exports `Scene`, `Shape`, `Bounds` and the conversions of `eh3d-geometry`, so the public paths do not change and the CAM programs share the same types.

## [0.0.2] - Notices by code and bytes reading
- **The CI builds against the tag of each sibling:** the sibling repositories are read from `depends_on` of the manifest (the ones they need in turn included) and checked out at the tag the repository is built from, so a build does not move when a sibling changes; a separate job, `Siblings at main`, builds against the main branch of every sibling to show a break before the sibling is tagged and does not hold a merge.

- **Each dependency on a sibling repository by path says the version it needs** (`version = ">=0.0.1", path = "../EH3D-..."`), so Cargo stops at once when the sibling next to this one is older than the program expects.
- **Rectangular pens:** a track drawn with a rectangular or obround aperture is drawn as the swept rectangle (a polygon) instead of a round-ended line as wide as the narrow side of the pen.
- **The banner of the README** shows the role and the technologies of this program (it showed the text of the shared library).

## [0.0.1] - Gerber layers and drill files drawn as an SVG picture, with measures and alignment checks

- **The shapes:** pads, tracks, regions and arcs in millimetres whatever the units of the file; the holes of a drill file as discs; what cannot be drawn exactly is listed once.
- **The picture:** one group per layer with its colour and transparency, the board from above; names and colours are escaped and checked.
- **Measures:** the narrowest track, the distance between pads and the smallest gap between round pads and tracks.
- **Checks:** layers whose centre is off and holes that are not inside the copper.
- **The command line** `eh3d-gerberview` with `render`, `info`, `measure` and `check`.
- **14 tests** run on a computer, with damaged input.
