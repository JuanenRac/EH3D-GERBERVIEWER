# EH3D-GERBERVIEWER: status

## Current state

EH3D-GERBERVIEWER is a working program on a computer: a library (`eh3d_gerberviewer`), a command line (`eh3d-gerberview`) and a window (built with `--features gui`). It draws and measures the layers of the example files, checks them against the public Gerber files of the readers, and reads the job of a `.eh3d` project to keep its picture in its own branch. The suite of tests runs on Windows and Linux in the CI. The window has not been looked at on a screen, and no collection of real boards has been used.

## Capabilities

The official state of each capability is in `REQUIREMENTS.md`. The register has 37 requirements: 30 implemented, 5 prototype, 2 planned and 0 validated. By progress: 30 done, 5 partial, 0 pending and 2 blocked. This page never states a state that `REQUIREMENTS.md` does not.

## Open questions

* How the window should paint a layer as a whole (the stack and its software picture exist; the window still paints shape by shape).
* How the window should highlight the pads and tracks that a measure of the ruler names.
* Which of the shapes that are listed today as not drawn exactly should be drawn first.

## What it needs from other repositories

* `EH3D-CORE-LIBS`: the Gerber and drill readers and the geometry crate, as a sibling folder.
* `EH3D-COMMON`: the project container (`eh3d-project`), as a sibling folder at the tag v0.3.3 that the workflows take.
* A collection of real boards, from a person, to check the layers against.

## Known technical debt and omitted tests

* The command `measure` still compares only round pads and tracks; the exact gap between every kind of shape is the command `gap` between two layers.
* The window draws the overlapping shapes of one layer one over the other, so the opacity adds up where they meet, and it reads a configuration file only when one is opened or dropped on it.
* The drawing of the window is not covered by tests; only its logic is, and no person has looked at it on a screen.
* Tests that need a screen or a real board are not written: the register names each as `planned:` with the test that will show it.
