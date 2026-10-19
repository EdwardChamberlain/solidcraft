# The Advanced (NX) modelling approach

SolidCraft has two ways of working, chosen in **Preferences › General › Modelling approach**:

- **Classic (Fusion 360)** is the layout and workflow SolidCraft started with: a ribbon of geometry
  tabs, a parts browser, a history timeline and floating dialogs. One design file holds its
  components.
- **Advanced (NX)** works the way a part-centred CAD system does. Every file is a *part*. A part
  holds its own feature tree and can add other part files as components, so parts are reused, not
  copied, and each file can be managed on its own (version control, a PDM system). The window
  follows suit: a strict-order feature tree, docked step-by-step dialogs and navigators.

The two use the same modelling engine and the same commands. Files are **not converted** between
approaches: a design records the approach it was made in, and opening one made in the other switches
the window to it. New designs are made in the approach that is chosen when they are created.

Switching also selects the matching mouse mapping (middle drag orbits, Shift+middle pans,
Ctrl+middle zooms) and turns on the radial menu on the right button, while the default mapping is
still in use.

## The window

| Part | What it is |
|---|---|
| Ribbon | **Home** (Feature, Datum, Pattern), **Assembly**, **Analysis**, **View**, **Tools**; **Sketch** appears while a sketch is open. The same commands as Classic, grouped and cut down to a part-modelling set. |
| Selection bar | The selection filter: what a click in the viewport can pick. |
| Resource bar | Vertical tabs at the left edge: **Part Navigator** and **Assembly Navigator**. Click the open one to hide it. |
| Part Navigator | The feature tree: the Origin (planes and axes), then every feature in the order it replays, with the rollback marker. Sketches stay where they were made. Select a feature and its **parents** (what it is built on) show in the warning colour and its **children** (what is built on it) in the addition colour, one step each way; a legend sits in the tree's heading. Every row has an **eye** at its left that hides what the feature made: a sketch, a datum, or the body (hiding a chamfer hides the body, so every feature that built it shows as hidden). Below the tree, the selected feature's **Dependencies** (its **Parent** features, which it is built on, and its **Child** features, built on it; worked out from the design's references once per change, so selecting a feature is instant) and **Details** (its values, and Edit). |
| Assembly Navigator | The components of this part: where each part file is, whether it is found, how many times it is placed. Add a component, make a new one, update parts from their files, Save all. Right-click a component for Make displayed part, Hide/Show, Fix in place and Break link. |
| Work part banner | At the top of the Part Navigator in a part made the *work part* of an assembly: a switch for the assembly drawn around the part, and **Back to assembly**. |
| Cue line | Along the bottom: what to pick next, or the sketch's state ("fully constrained", "N degrees of freedom"). |

## Dialogs

Dialogs keep SolidCraft's look and work the NX way: docked at the left, a **Type** row first when a
command has several methods, numbered selection steps, collapsible groups (Extrude: Section, Direction, Limits, Offset, Draft, Boolean; Revolve: Limits, Boolean; Hole: Position, Form and Dimensions, Limits; Fillet and Chamfer: Radius or Offsets, Settings), and **OK / Apply / Cancel** with **Reset**. Apply makes the
feature and starts the next one. A middle click moves to the next pick and confirms once every
pick is made. **Extrude** is laid out the way the feature is described: Section, a Direction (a Specify Vector: the profile normal unless one is given, with Reverse; a direction across the normal is not supported yet), Limits with a **Start** (Value, Value (Other Side), Symmetric Value, Until Selected) and an **End** (Value, Until Selected, Through All), and an **Offset** (None, Symmetric, Inside, Outside, Two-Sided) that is the wall "thin" is in Classic. The End distance is measured from the Start, as the command's distance is. Dialogs that preview end with a **Preview** group: *Show result* turns the live
result off and on (OK still makes the feature). A To Object or From Object extent takes a vertex or
construction point, a face or an edge (the place picked).

**Specifiers** are the plane, vector and point inputs every Advanced dialog shares (`specifier.rs`).
Each has a **Type** that starts at *Inferred* (pick what you have and the way follows) and lists the
explicit ways after it:

| Specifier | Types |
|---|---|
| Plane | Inferred, Plane or Face, Offset from Plane, At Angle to Plane, Bisector, Along Path, Two Edges, Three Points, Tangent to Face, Perpendicular to Plane. A plane that is made (not picked) becomes a datum plane feature the dialog runs first. |
| Vector | Inferred, Edge or Line, Face or Plane Normal, Two Points, Coordinate Axis; always with **Reverse**. |
| Point | Inferred, Existing Point, Midpoint of Edge, Where Clicked, Coordinates, Workpiece Origin. |

A specifier's picks are inputs of its dialog (so the viewport sends clicks as it always does) and
what it makes is plain data the commands already take. **Create Sketch** is built on three of them:
the **Plane**, the **Reference Direction** (the sketch's x axis; Reverse flips it) and the **Sketch
Origin** (the workpiece origin projected onto the plane, or any point). It waits for OK; the
sketch stays attached to its plane, with `sketch.create`'s `reframe` (`origin`, `x_dir`) holding the
direction and origin. Other dialogs take a specifier by `specifier::add`, read it with
`specifier::plane`/`vector`/`point` and draw it with `specifier::show`; Create Sketch is the first,
and the pickers the other dialogs have today (Mirror and Draft planes, pattern directions and axes,
Revolve's axis) move onto them one at a time.

**Datums** are one dialog each (Plane, Axis, Point) with a Type row: *Inferred* (the method follows
what you pick) and every construction method the engine has. Click a plane in the Origin rows to pick
it into the dialog. A pattern's direction takes an axis, an edge or a planar face.

## A first part, and a part that uses parts

1. **File › New**, with Advanced chosen. Save it (**Ctrl+S**): a design gets its identity at its
   first save.
2. **Home › Sketch**, pick a plane, draw, dimension, finish.
3. **Home › Extrude**. Edit it later from the Part Navigator (double click, or Details › Edit).
4. In a second part, **Assembly › New Part Component** makes `<name>.solidcraft` beside this file and
   links it in as an empty component; **Add component** links an existing part file. Open the part
   from the Assembly Navigator to model it; its bodies appear in the assembly when you switch back.
5. To edit a part with its assembly around it, click **Edit in context** on its row in the Assembly
   Navigator (or right-click › Edit in context). The part's tab opens and the rest of the assembly
   is drawn around it, faded, where it stands in the part's own frame, so you can see what the
   part must clear or fit. The context is for the eye: it can't be picked and no feature can refer
   to it. **Back to assembly** returns to the assembly's tab.
6. Edit the part and switch back to the assembly's tab: the assembly shows the part as it is now,
   saved or not. **Save all** writes every open design and brings the assemblies up to date.

See [advanced-approach-files.md](advanced-approach-files.md) for how parts reference parts, what
is built and what is not yet.

## Not built yet

- Referring to the assembly's geometry from the work part (picking a face of another component, or
  linking to it): the context is drawn but is not pickable.
- A shared evaluation between a part open in a tab and the same part linked in an assembly.
- Mates between faces of different parts (assemblies use joints).
- Groups for the remaining dialogs (Extrude, Revolve, Hole and the blends have them), and a Specify Point that also takes a
  sketch point (it takes a vertex or construction point, a face or an edge).
- Drawings and direct face editing are out of scope for this approach.
