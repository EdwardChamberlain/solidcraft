//! A design can hold a datum coordinate system, `ConstructionCsys` (`{"ConstructionCsys": {"def": {"type":
//! "three_points" | "on_plane" | "frame", …}}}`), and name one the work coordinate system with
//! `work_csys` (the feature's id; absent: the design's own). Older designs have neither and open
//! unchanged; a design that uses one needs this version.
