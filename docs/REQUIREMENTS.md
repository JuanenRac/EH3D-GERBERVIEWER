# EH3D-GERBERVIEWER: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because nothing of this program has been compared with a collection of real boards.

The register has 33 requirements: 27 implemented, 4 prototype, 2 planned and 0 validated. By progress: 27 done, 4 partial, 0 pending and 2 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `GRBV-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a simulated machine, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `file::name` is the test `name` of `tests/file.rs` (`src/file::name` is a test inside `src/file.rs`), which must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## Reading and drawing

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| GRBV-001 | A Gerber layer becomes shapes in millimetres, and inches are converted. | implemented | done | host-test | `viewer::a_gerber_layer_becomes_shapes_in_millimetres` | - |
| GRBV-002 | A region becomes a polygon and an arc ends where it should and is as long as the circle says. | implemented | done | host-test | `viewer::an_arc_ends_where_it_should_and_is_as_long_as_the_circle_says` | - |
| GRBV-003 | A track drawn with a rectangular pen is the swept rectangle and not a line with round ends. | implemented | done | host-test | `viewer::a_track_with_a_rectangular_pen_is_the_swept_rectangle` | - |
| GRBV-004 | The holes of a drill file are discs of the diameter of their tool. | implemented | done | host-test | `viewer::the_holes_of_a_drill_file_are_discs` | - |
| GRBV-005 | A text that starts with the drill header is a drill file whatever its name is. | implemented | done | host-test | `more::a_text_that_starts_with_the_excellon_header_is_a_drill_file_whatever_its_name_is` | - |
| GRBV-006 | What is not drawn exactly is said once, and `exact` refuses it. | implemented | done | host-test | `exact::render_says_what_is_not_drawn_exactly_and_exact_refuses` | Clear polarity, step and repeat and macro apertures are listed, not drawn. |
| GRBV-007 | The picture is an SVG with one group per layer, the same every time, with y up. | implemented | done | host-test | `viewer::the_picture_is_valid_looking_svg_with_one_group_per_layer` | - |
| GRBV-008 | The name of a layer cannot break out of the attribute, and a bad colour is grey. | implemented | done | host-test | `viewer::the_name_of_a_layer_cannot_break_out_of_the_attribute` | - |
| GRBV-009 | A picture is written whole, no temporary file stays and it is never written over a layer. | implemented | done | host-test | `atomic::a_picture_is_written_whole_and_no_temporary_file_stays` | - |
| GRBV-010 | The public Gerber files of the readers are drawn inside the box their summaries expect. | implemented | done | host-test | `golden::info_gives_the_shapes_and_the_box_of_each_example_file` | Public example files, not a collection of real boards. |

## Measures and checks

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| GRBV-011 | The narrowest track ignores pads and is none without tracks. | implemented | done | host-test | `more::the_narrowest_track_ignores_pads_and_is_none_without_tracks` | - |
| GRBV-012 | The smallest gap between two tracks is measured edge to edge. | implemented | done | host-test | `viewer::the_smallest_gap_between_two_tracks_is_measured_edge_to_edge` | - |
| GRBV-013 | The clearance between pads and tracks names the closest pair; rectangles and regions are not compared. | prototype | partial | host-test | `more::the_clearance_looks_at_tracks_as_capsules_and_names_the_closest_pair` | `measure` compares round pads and tracks only; `gap` and `scene_gap` compare every kind of shape. |
| GRBV-014 | Too many shapes to compare pair by pair is an error that says how many, not a hang. | implemented | done | host-test | `viewer::too_many_shapes_to_compare_pair_by_pair_is_an_error_that_says_how_many` | - |
| GRBV-015 | Layers that do not line up are reported, and a centre off by exactly the tolerance is fine. | implemented | done | host-test | `viewer::a_centre_off_by_exactly_the_tolerance_is_fine_and_a_hair_more_is_reported_with_its_signs` | - |
| GRBV-016 | Only the centres of the layers are compared, so a solder mask bigger than the copper lines up. | implemented | done | host-test | `more::layers_with_the_same_centre_and_other_sizes_line_up_because_only_the_centre_is_compared` | - |
| GRBV-017 | A hole outside the copper is reported, and a hole that touches the edge of the copper is inside. | implemented | done | host-test | `viewer::a_hole_that_touches_the_edge_of_the_copper_is_inside_and_one_beyond_it_is_not` | - |
| GRBV-018 | `check` says when there are no layers to compare instead of claiming that they line up. | implemented | done | host-test | `more::an_empty_layer_is_said_to_have_nothing_and_the_reference_is_the_first_one_with_something` | - |

## Robustness and window

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| GRBV-019 | Garbage, empty layers and cut files are read without a panic. | implemented | done | host-test | `fuzz::copies_with_bytes_changed_inserted_or_cut_are_read_without_a_panic` | Seeded generation, not a continuous fuzzer. |
| GRBV-020 | A run that goes well ends with 0, a wrong use with 64 and a file that cannot be read with 1. | implemented | done | host-test | `exit_codes::an_unknown_command_or_option_ends_with_64` | - |
| GRBV-021 | The window opens the sample files together, measures and checks like the command line. | prototype | partial | host-test | `src/window::the_sample_files_of_the_repository_open_together` | The logic is tested without a screen; no person has looked at the drawing. |
| GRBV-022 | A file that cannot be drawn is refused and the layers stay. | implemented | done | host-test | `src/window::a_file_that_cannot_be_drawn_is_refused_and_the_layers_stay` | - |
| GRBV-023 | The picture may not be a layer nor a file that is not an SVG. | implemented | done | host-test | `src/window::the_picture_may_not_be_a_layer_nor_a_file_that_is_not_an_svg` | - |
| GRBV-024 | Every sentence of the window has a text in each of the eight languages. | implemented | done | host-test | `src/lang::every_sentence_has_a_text_in_each_of_the_eight_languages` | - |

## Not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| GRBV-025 | A measure is made by clicking on the picture and names the pads and tracks it measured. | prototype | partial | host-test | `ruler::the_ruler_snaps_both_ends_and_names_the_pads_it_measured` | The logic of the click is written and tested; the window does not take the click yet and does not highlight the pads. |
| GRBV-026 | A layer is drawn with its opacity as a whole. | prototype | partial | host-test | `layers::the_opacity_of_a_layer_does_not_add_up_where_its_shapes_overlap` | The SVG picture and a software picture of a stack paint a layer as a whole; the window still paints shape by shape. |
| GRBV-027 | The gap between rectangular pads, obround pads, tracks and regions is measured exactly, and it is 0 when they touch. | implemented | done | host-test | `gap::two_rectangles_are_apart_by_the_distance_of_their_sides_or_corners` | - |
| GRBV-028 | A collection of real boards is drawn and measured and the numbers are compared with another viewer. | planned | blocked | none | `planned:real_boards::real_boards_are_drawn_and_measured_as_another_viewer_does` | Needs real boards from a person. |
| GRBV-029 | The window is looked at on a screen and matches the picture the program takes of itself. | planned | blocked | none | `planned:window_screen::the_window_matches_the_reference_picture` | Needs a person at a screen. |

## Click, layers and settings

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| GRBV-030 | The ruler gives the distance, the two differences and the angle in millimetres, inches or mils, and the angle runs from 0 to 360. | implemented | done | host-test | `ruler::the_angle_runs_from_0_to_360_anticlockwise_and_is_0_for_the_same_point` | - |
| GRBV-031 | A click is caught by the nearest vertex or pad centre within the tolerance, and a distance equal to the tolerance catches. | implemented | done | host-test | `ruler::a_click_at_exactly_the_tolerance_is_caught_and_a_hair_more_is_not` | - |
| GRBV-032 | A stack of layers keeps an opacity from 0 to 1 for each and an order, refuses what is outside the range and warns of a layer that is hidden or almost invisible. | implemented | done | host-test | `layers::a_composition_that_hides_a_layer_or_leaves_it_almost_invisible_is_warned` | - |
| GRBV-033 | The settings come from a configuration file with a range for each value, and a bad value names its line. | implemented | done | host-test | `settings::a_value_out_of_its_range_an_unknown_key_or_a_twice_written_key_names_its_line` | The window does not read the file yet. |
