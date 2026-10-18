# Parts that contain parts (the Advanced approach's file model)

The Advanced approach works with one kind of file, a **part**. A part has its own feature tree
and may add other part files as components, to any depth, so a part is reused by linking it,
not copying it, and a file can be managed on its own (version control, a PDM system). This note
is the contract for the file model: what is built, what each piece relies on, and what is not
built yet.

Designs are **not** converted between approaches. A design records the approach it was made in
(`Document::approach`, saved as `advanced`; Classic is not written), and opening one made in the
other approach switches the window to it (Preferences › Modelling approach).

## Identity

- `Document::id`: 128 random bits as hex, minted when a design is first saved and kept for good,
  Save As included (the file is the same design's copy, and a link made to either still finds
  it). The undo and redo states carry it too, so undoing after a save never changes it. Empty
  until the first save.
- A link (`DesignLink`) stores the identity of the design it linked. Update Links warns when the
  file at the link's path is now a different design (it still updates: the path is the link).
- The path is the address, the id is the check. Paths are relative to the linking design's folder
  and written with `/`, so a folder of parts moves between systems.
- Where the platform has no randomness (the browser build), the process still gives every
  minted id its own value, but two sessions can mint the same one. A host that needs better can
  set the id before saving.

## References

A component of a part can be linked to another part file (`component.insert_design`, or
`component.new_part` for a part that does not exist yet).

- The linked part is opened and evaluated on its own. Its solid bodies, as placed in it, join the
  component through one Import feature (exact B-rep, kept in the linking design so it still
  opens when the part file is missing).
- **Update Links** (`component.update_links`) reads the files again and replaces the Import
  bodies where the part changed. A part's own links are brought up to date first, so editing a
  leaf part reaches every design above it on the next update, without opening or saving the
  parts in between.
- **No cycles.** A design can't link itself, nor a design that already links it however far down
  (`reaches`, depth and file counts capped; a chain too long to follow is refused).
- **New Part** makes `<name>.solidcraft` beside the saved design (or at `path`), with the same
  approach, and links it in one step. A part needs a solid to be linked, so the new file starts
  with a 10 mm "Placeholder" box; delete it after modelling the part.
- Break Link keeps the bodies and forgets the file.

## What a reference does not do yet

These are the known limits of the exact-copy link, in the order they are likely to matter:

1. **The link is a copy that is refreshed, not a live view.** Editing a part updates the designs
   above it on **Update Links** (and on open, in the desktop app), not as the part is edited.
2. **Persistent names stop at the file boundary.** The linked bodies' faces are imported faces:
   a feature of the linking design that refers to one (a joint origin on a face, a fillet) holds
   its position, not the part's own face name, so a part that changes shape can move it.
3. **Joints refer to a single occurrence**, not a full path of occurrences
   (ROADMAP.md, "Exact full-path assembly joint editing").
4. **A part that uses another's geometry** (a feature of one file consuming a body of another) is
   not supported. A part's own recipe depends only on itself.
5. **No per-component configuration.** Configurations apply to the whole design.
6. **One design per session.** Every open file is a full `Session`; there is no shared
   workspace yet, so a part open in another tab and the same part linked into an assembly are two
   separate evaluations, and an edit shows up in the other after saving and updating.
7. **File access** is `solidcraft_io::vfs`, a module of functions (disk natively, a pre-filled
   map in the browser). Opening in two phases on the web (read the references, let the host fetch
   them, then evaluate) and a store that a PDM system can implement need it to become a trait.
8. **Recovery and crash reports** still cover one design: background tabs are not autosaved.

Each of these is a separate piece of engine work; none changes the file format already written
here, except the first two, which would add what a live reference stores.

## Format

One pending format file, `crates/doc/src/format/pending/part-identity.rs`: `id`, `approach` and
the link's `id`. Older designs open unchanged (no identity, Classic, a link with nothing to
compare).
