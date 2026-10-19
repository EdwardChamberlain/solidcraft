//! A sketch's plane may be `Reframed`: another plane with the sketch's x axis along a chosen
//! direction and its origin at a chosen point (`{"Reframed": {"base": …, "origin": [x,y,z],
//! "x_dir": [x,y,z], "soft": true}}`, each optional; `soft` lets a direction square to the plane fall back to the plane's own x axis). Older designs have no such plane and open
//! unchanged; a design that uses one needs this version to open.
