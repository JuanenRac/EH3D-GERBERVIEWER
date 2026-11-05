# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **The Windows executable carries the icon of the program:** the file shows it in the explorer and the taskbar, together with the name, the description and the version, embedded by `build.rs`; other systems are not affected.
- **A window:** built with `--features gui`, `eh3d-gerberview` with no arguments (or with a single layer file, or `gui [LAYER...]`) opens a window in the look of the other programs of the suite (amber on charcoal, the icon of the program). Layers and drill files are added from the list of files of the window or dropped on it; each one can be shown or hidden and taken out; they are drawn one over the other with the opacity of the parameters, with zoom (wheel) and pan (drag); the narrowest track, the smallest gap and the pads of the chosen layer can be measured; the alignment of the layers and the holes inside the copper can be checked; and the layers that are shown are exported as one SVG picture with the margin of the parameters. The command line works as before, and a build without the feature is only a command line.
- **The window speaks the eight languages:** English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal, chosen from the system and changeable in the window.
- **The logic of the window needs no screen:** `window` (layers, parameters, measures, checks, export, status), `viewport` (zoom and pan), `draw` (the shapes and the triangles of the regions), `browse` (the list of files) and `lang` (the words) are plain code with their own tests; `gui` only draws them.
- **A drill file is told by its header:** a file that starts with `M48` is read as a drill file whatever its extension is, and `check` counts it as one.
- **The layers are compared by their centres, and a test says so:** layers with the same centre and another size line up.
- **A test draws the public Gerber files of the readers** inside the box their summaries expect.
- **The commands of the README are run by a test.**

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
