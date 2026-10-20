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
| Selection bar | A **type filter** (All, Bodies, Faces, Edges, Vertices, Sketch Geometry; Customize has the finer checkboxes), a **scope** (Entire Design, Work Part Only, Work Part and Components) and an **intent** (Single, or Tangent Chain: picking an edge takes the edges that continue it smoothly). They apply to clicks and to picks into a dialog. |
| Resource bar | Vertical tabs at the left edge: **Part Navigator** and **Assembly Navigator**. Click the open one to hide it. |
| Part Navigator | The feature tree: the Origin (planes and axes), then every feature in the order it replays, with the rollback marker. Sketches stay where they were made. Select a feature and its **parents** (the features it requires: its sketches, the features that made the faces and edges it picked, read from their persistent names, and, for a join or cut that picks no geometry, the feature that made the body; not whatever else changed the same body before it) show in the warning colour and its **children** (what is built on it) in the addition colour, one step each way. Every row has an **eye** at its left that hides what the feature made: a sketch, a datum, or the body (hiding a chamfer hides the body, so every feature that built it shows as hidden). While a dialog waits for a pick, **clicking a feature in the tree answers it** wherever the input takes what the feature stands for: a pattern's features, a sketch's profiles, a datum plane, axis or point, or the bodies a feature made. A row that the input cannot use is selected as usual. A **Section Analysis**, while there is one, is a row of the tree under the Origin: its eye hides it, a double click edits it and its menu deletes it. Below the tree, the selected feature's **Dependencies** (its **Parent** features, which it is built on, and its **Child** features, built on it; worked out from the design's references once per change, so selecting a feature is instant) and **Details** (its values, and Edit). |
| Assembly Navigator | The components of this part: where each part file is, whether it is found, how many times it is placed. Add a component, make a new one, update parts from their files, Save all. Right-click a component for Make displayed part, Hide/Show, Fix in place and Break link. |
| Work part banner | At the top of the Part Navigator in a part made the *work part* of an assembly: a switch for the assembly drawn around the part, and **Back to assembly**. |
| Cue line | Along the bottom: what to pick next, or the sketch's state ("fully constrained", "N degrees of freedom"). |

## Dialogs

Dialogs keep SolidCraft's look and work the NX way: docked at the left, a **Type** row first when a
command has several methods, numbered selection steps, collapsible groups (Extrude: Section, Direction, Limits, Offset, Draft, Boolean; Revolve: Section, Axis, Limits, Boolean; Hole: Position, Form and Dimensions, Limits; Fillet and Chamfer: Radius or Offsets, Settings), and **OK / Apply / Cancel** with **Reset**. Apply makes the
feature and starts the next one. A middle click moves to the next pick and confirms once every
pick is made. **Extrude** is laid out the way the feature is described: Section, a Direction (a Specify Vector: the profile normal unless one is given, with Reverse; a direction across the normal is not supported yet), Limits with a **Start** (Value, Value (Other Side), Until Selected) and an **End** (Value, Value from Start, Symmetric Value, Until Selected, Through All), and an **Offset** (None, Symmetric, Inside, Outside, Two-Sided) that is the wall "thin" is in Classic. The End **Value** is measured from the sketch plane; **Value from Start** is the length from the start, which is the command's own distance (the dialog works the one out from the other). **Pattern** and **Mirror** are the two buttons of the Pattern panel. What they repeat (**Bodies** or **Features**) and, for Pattern, the **Layout** (Linear, Circular, Along Path) are chosen in the dialog. `solid.pattern` takes a `layout` and hands the rest to the layout's own command. The dialogs have no step commentary. **Revolve** has the same shape: Section, an **Axis** (a Specify Vector with Reverse: a picked axis or sketch line goes as it is, so the revolve follows it; any other way, or reversed, goes as a point and a direction), Limits with a **Start** (Profile Plane, Value (Other Side)) and an **End** (Value, Symmetric Value, Until Selected), and Boolean. **Mirror** and **Draft** take their plane as a selector row (with its own Plane dialog), in groups (Mirror: Objects, Mirror Plane, Settings; Draft: Faces, Neutral Plane, Angle); a plane made in that dialog is made first, so it cannot be used while editing a feature (make it, then pick it). The other dialogs without a layout of their own (Shell, Rib, Sweep, Loft, Combine, Thread, Scale, Offset Faces, Circular and Path patterns, Section…) have a **Selection** group and a **Settings** group. Dialogs that preview end with a **Preview** group: *Show result* turns the live
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
Origin** (the workpiece origin projected onto the plane, or any point). The direction starts at
the x axis and the origin at the workpiece origin, so a sketch needs only the plane and OK, and
there is always the chance to choose a vector or an origin first. The sketch stays attached to its plane, with `sketch.create`'s `reframe`
(`origin`, `x_dir`, `soft`) holding the direction and origin. Other dialogs take a specifier by `specifier::add`, read it with
`specifier::plane`/`vector`/`point` and draw it with `specifier::show`; Create Sketch is the first,
and the pickers of the other dialogs (Mirror and Draft planes, Revolve's axis, Extrude's direction) have
moved onto them; pattern directions and axes follow.

**In a sketch**, the Advanced ribbon puts **Finish Sketch** first (the only finish button; finishing with auto constrain is in its drop-down), with **Reattach Sketch** beside it:
the same Plane, Reference Direction and Sketch Origin as Create Sketch, applied to the sketch being
edited (or the one selected in the tree) with its geometry and dimensions kept. It waits for OK, and
what is left unspecified is left as it was (`sketch.reattach`, which is `sketch.redefine` with a
`reframe`). The text tool aligns its lines horizontally (left, center, right) and **vertically**
(baseline, top, middle, bottom) against its anchor.

**Selectors.** A plane, vector or point input is one **selector** row: click it, then click geometry or a feature in the tree and what you picked is inferred. The icon at its right opens that specifier's **own dialog** (Plane, Axis or Point), which is the same dialog wherever one is needed: its Type (Offset, At Angle, Bisector…; Edge, Face Normal, Two Points…; Existing Point, Midpoint, Coordinates…) and what that Type asks for, with OK and Cancel. OK hands the choice back to the dialog that asked, whose selector then says which way was chosen (× goes back to inferring); Cancel and Esc return to it unchanged. A vector's row also has Reverse. Create Sketch is three selectors, **Plane**, **Axis** (the x axis until one is picked) and **Point** (the workpiece origin until one is picked), then OK. The empty selectors say what they stand for.

The Home ribbon's **Datums** panel is four buttons, **Plane**, **Axis**, **Point** and **CSYS**,, **Plane**, **Axis** and **Point**, each with its ways of being defined in a drop-down (`datum.plane`, `datum.axis`, `datum.point`, `datum.csys` take a `type`). A **datum CSYS** (origin, a point on X and a point in XY; or on a plane or face) is a datum like the others and offers its planes, axes and origin by name (`Csys1:XY`, `Csys1:X`, `Csys1:O`) to everything that takes one: its row in the tree opens to list them, and clicking one picks it into a dialog. A new one shows as its triad (its planes start hidden). **Make Work Coordinate System** (the row's menu; `csys.set_work`) makes one the work coordinate system: a new sketch's origin and x axis, and the Workpiece Origin and Coordinate Axis choices, follow it. The design's own is the work coordinate system until one is named. **Datums** are one dialog each (Plane, Axis, Point) with a Type row: *Inferred* (the method follows
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

## Sketches and datums

- A sketch holds only what is drawn in it. Sketching on a face does not carry the face's edges in
  (Fusion does, by default), so a circle on a face is the one region, and picking it picks the
  circle. The Project tool is still there for a deliberate reference, and `sketch.create` takes
  `project_edges: true`. The Preferences row for projecting face edges is hidden in this approach.
- A datum depends on what it was built from: the features that made the faces, edges and
  vertices it picked, the datums it uses and the sketches whose points it takes. A sketch placed
  on a face depends on the feature that made the face. The Part Navigator's parent and child
  colours show it.

## Face mates

**Mate** (Assembly tab, `mate.create`) positions one component against another from two picked faces.
Touch puts the faces together (outsides meeting), Align makes them face the same way, Distance is
Touch with a gap, Concentric lines two round faces up on one axis, Parallel, Perpendicular and
Angle hold the faces parallel, square or at an angle while the part slides freely, and Fix grounds
the face's component. A mate is a joint underneath (touch, align and distance are planar joints,
concentric a cylindrical one, the three orientation mates a `parallel` joint with a `tilt`), so it is listed,
edited and driven as one, and its faces are kept by their persistent names, which now reach
through linked parts: the mate follows the face when the part changes shape.

## Not built yet

- Snap-point toggles in the selection bar (the sketch tools infer their snaps without toggles) and the connected-curve and face-edge intents.

- Referring to the assembly's geometry from the work part (picking a face of another component, or
  linking to it): the context is drawn but is not pickable.
- A shared evaluation between a part open in a tab and the same part linked in an assembly.
- Mates that remove all remaining freedom except by stacking them (each mate is one joint: the part still slides along the face).
- The linear pattern's direction pickers, the circular pattern's axis and Section's plane are still plain inputs (the feature pattern's own layout and the commands' parameters need more than the selector rows give), and Specify Point does not take a sketch point.
- Drawings and direct face editing are out of scope for this approach.
