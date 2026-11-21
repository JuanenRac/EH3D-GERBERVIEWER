# EH3D-GERBERVIEWER: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| Measuring by clicking on the picture | The window shows layers and checks, but the pads and tracks a measure names are not highlighted | the window has been looked at on a screen and finished |
| A layer drawn with its opacity as a whole | Each shape is drawn on its own, so the opacity adds up where the shapes of one layer overlap | the drawing groups the shapes of a layer before it paints them |
| Exact drawing of clear polarity, step and repeat and macro apertures | They are listed once each as not drawn exactly and `--exact` refuses them; drawing them needs the reader to expand them | the reader of the suite expands them with a test |
| The gap between rectangular pads and regions | Today only round pads and tracks are compared, as capsules | a measure of the gap between polygons is written and tested |
| Real boards | No collection of real files has been used; the tests read the public example files | a person gives boards to compare with |
| Writing a layer | The program is a viewer; writing a file for a maker would need the format checked against the maker | a person asks for a file to be written |
