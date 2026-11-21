# EH3D-GERBERVIEWER: not yet

Good ideas that are not in the current version. Writing them here keeps anybody from implementing them early or twice.

| Idea | Why not yet | Revisit when |
|---|---|---|
| The window taking the click of the ruler | The logic of the click and the snap is written and tested, but the window does not call it and does not highlight the pads and tracks a measure names | the window has been looked at on a screen and finished |
| The window painting a layer with its opacity as a whole | The stack of layers and its software picture paint a layer as a whole, but the window paints each shape on its own, so the opacity adds up where they overlap | the drawing of the window groups the shapes of a layer before it paints them |
| Exact drawing of clear polarity, step and repeat and macro apertures | They are listed once each as not drawn exactly and `--exact` refuses them; drawing them needs the reader to expand them | the reader of the suite expands them with a test |
| The exact gap between every kind of shape in `measure` | The command `gap` does it between two layers, but `measure` keeps the numbers it gave so that its output does not change | a change in the output of `measure` is accepted |
| Real boards | No collection of real files has been used; the tests read the public example files | a person gives boards to compare with |
| Writing a layer | The program is a viewer; writing a file for a maker would need the format checked against the maker | a person asks for a file to be written |
| Comparing two versions of a layer (what was added and what was removed) | The program checks layers against each other but does not draw the difference between two revisions of one file | the shapes of a layer can be combined with the booleans of the shared geometry |
| Recognising the function of a layer by its attributes, name and content | A layer is drawn with the colour of its extension today, and no manual choice of the function exists | the window has a place to choose it |
