# Changelog

All notable changes to this project are documented here.

## [Unreleased]
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
