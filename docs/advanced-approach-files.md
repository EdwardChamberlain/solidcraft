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
  approach, and links it in one step. The part starts empty, so it links as an empty component (no
  Import feature yet); open it, model something, and the bodies arrive on the next update. If every
  body is later deleted from the part, the component goes back to empty.
- Break Link keeps the bodies and forgets the file.

## Open parts, the file store and recovery

- **Open designs stand in for their files** (`workspace::with_open_designs`). A part that is open
  in another tab is read as it is now, unsaved edits included, wherever a link reads a part:
  switching to an assembly's tab, Update parts, Save All and opening a design all bring its
  parts up to date from the tabs first. The desktop app does this in the Advanced approach only;
  Classic keeps a link as it is until Update Links is asked for. Without a host (the CLI, a
  script) nothing is registered and the files are read.
- **Files go through a `DesignStore`** (`solidcraft_io::vfs`): the disk, or the browser's files,
  unless a host installs its own (`with_store` on a thread, `set_default_store` for all), such as a
  PDM backend. Linking, opening and saving all work through it.
- **`referenced_files(path)`** lists every part below a design, for a host that must fetch files
  before a command can read them (the browser): fetch what it lists, then open the design.
- **Autosave keeps every open design** with unsaved changes (one entry each, one lock for the app),
  and closing the app keeps what is unsaved. A crash report carries the design; the linked
  parts' bodies are inside it, so replaying it needs no part file.

## What a reference does not do yet

1. **The link is an exact copy, refreshed from the part**, not the part's own evaluation. The linked
   bodies are imported faces: a feature of the linking design that refers to one (a joint origin
   on a face, a fillet) holds its position, not the part's own face name, so a part that changes
   shape can move it. Following the part's faces needs the part's persistent names to cross the
   file boundary.
2. **Joints refer to one occurrence, not a path of occurrences** (ROADMAP.md, "Exact full-path
   assembly joint editing"). A linked part is a component of its own with its bodies flattened into
   one Import, so a part used twice is two components and a joint is never ambiguous; the
   limit applies to components kept inside a file.
3. **A part that uses another's geometry** (a feature of one file consuming a body of another) is
   not supported. A part's own recipe depends only on itself.
4. **No per-component configuration.** Configurations apply to the whole design.
5. **One design per session.** The open designs are registered for linking, but each is still its
   own `Session`, with its own undo; an edit in one tab reaches another when that tab is shown (or
   on an update), not while both are being edited.
6. **The browser host does not fetch before opening yet.** `referenced_files` is there; the web
   app does not call it.

## Format

One pending format file, `crates/doc/src/format/pending/part-identity.rs`: `id`, `approach` and
the link's `id`. Older designs open unchanged (no identity, Classic, a link with nothing to
compare).
