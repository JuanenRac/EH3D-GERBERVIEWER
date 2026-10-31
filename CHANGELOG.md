# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.1] - Gerber layers and drill files drawn as an SVG picture, with measures and alignment checks

- **The shapes:** pads, tracks, regions and arcs in millimetres whatever the units of the file; the holes of a drill file as discs; what cannot be drawn exactly is listed once.
- **The picture:** one group per layer with its colour and transparency, the board from above; names and colours are escaped and checked.
- **Measures:** the narrowest track, the distance between pads and the smallest gap between round pads and tracks.
- **Checks:** layers whose centre is off and holes that are not inside the copper.
- **The command line** `eh3d-gerberview` with `render`, `info`, `measure` and `check`.
- **14 tests** run on a computer, with damaged input.
