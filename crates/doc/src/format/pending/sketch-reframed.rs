//! A sketch's plane may be `Reframed`: another plane with the sketch's x axis along a chosen
//! direction and its origin at a chosen point (`{"Reframed": {"base": …, "origin": [x,y,z],
//! "x_dir": [x,y,z]}}`, each of the two optional). Older designs have no such plane and open
//! unchanged; a design that uses one needs this version to open.
