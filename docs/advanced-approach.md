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
| Part Navigator | The feature tree: the Origin (planes and axes), then every feature in the order it replays, with the rollback marker. Sketches stay where they were made. Below it, the selected feature's **Dependencies** (what it is made from, what is deleted with it, what would fail without it) and **Details** (its values, and Edit). |
| Assembly Navigator | The components of this part: where each part file is, whether it is found, how many times it is placed. Add a component, make a new one, update parts from their files, Save all. Right-click a component for Make displayed part, Hide/Show, Fix in place and Break link. |
| Work part banner | At the top of the Part Navigator in a part made the *work part* of an assembly: a switch for the assembly drawn around the part, and **Back to assembly**. |
| Cue line | Along the bottom: what to pick next, or the sketch's state ("fully constrained", "N degrees of freedom"). |

## Dialogs

Dialogs keep SolidCraft's look and work the NX way: docked at the left, a **Type** row first when a
command has several methods, numbered selection steps, collapsible groups (Extrude: Limits, Draft,
Boolean; Revolve: Limits, Boolean), and **OK / Apply / Cancel** with **Reset**. Apply makes the
feature and starts the next one. A middle click moves to the next pick and confirms once every
pick is made.

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
- A specify-point widget, a Preview group in dialogs, and groups for every dialog (Extrude and
  Revolve have them).
- Drawings and direct face editing are out of scope for this approach.
