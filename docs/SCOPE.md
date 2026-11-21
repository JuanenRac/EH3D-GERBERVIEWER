# EH3D-GERBERVIEWER: scope

Status: **host-test**. EH3D-GERBERVIEWER is a library, a command line and a window that draw the layers of a board from its Gerber and drill files and measure them. It has been tried on a computer only, with the public example files and with hand-written ones: it has not been tried with a collection of real boards. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The viewer of the suite for the files that come from a board design: it reads the Gerber layers and the Excellon drill files, turns them into shapes in millimetres, draws them as one picture with a group and a colour for each layer and measures what a person wants to know before milling or drilling (the narrowest track, the smallest gap, the layers whose centre is off, the holes that are not inside the copper).

## What it does

* Reads pads (round, rectangular, obround), tracks with their width, regions and arcs, all in millimetres whatever the units of the file; the holes of a drill file become discs of the diameter of their tool.
* Tells a drill file by its header, whatever its extension is, and lists once each what cannot be drawn exactly (clear polarity, step and repeat, macro apertures); with `--exact` such a file is refused.
* Draws the layers as an SVG with a colour from the extension of each file, a transparency and the board seen from above, and writes it whole and never over a layer.
* Measures the narrowest track, the distance between two pads and the smallest gap between round pads and tracks, and checks that the layers line up and that the holes are inside the copper.
* Refuses a file over a size limit before reading it, and every error comes back as a message with an exit code that tells the kind of mistake.
* Shows the layers in a window (built with `--features gui`) in eight languages, with the same checks.

## What it will not do

* No decision about a machine: it does not move anything and does not make a toolpath (that is ISOCAM).
* No editing of the layers and no writing of a Gerber or a drill file.
* No exact drawing of what is not simple yet: clear polarity, step and repeat and macro apertures are listed, not drawn (see `NOT_YET.md`).
* No measure of the gap between rectangular pads or regions yet.
* No claim that anything was proven with real boards: the evidence of every capability is `simulator` or `host-test`.

## Where it sits

* **It reads:** Gerber and Excellon files as plain files, given by path; nothing else.
* **It writes:** an SVG picture, and text or numbers on the standard output; never over a layer it was given.
* **It is read by:** the other programs of the suite through those files, and ISOCAM and STENCILCAM as a library; programs exchange work through files, never by depending on each other in a way that needs the window.
* **It depends on:** `EH3D-CORE-LIBS` (the Gerber and drill readers and the geometry), built next to it as a sibling folder.

## What it never decides

Whether a board can be made, and whether a track or a gap is good enough for a process, is for the person and the maker of the board to decide: this program measures what the files say. A measure that comes out clean is a measure of the files read, not a verified board. It has no limits of a machine to decide and moves nothing, so it never replaces the emergency stop of a machine or the judgement of the person who stands at it.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
