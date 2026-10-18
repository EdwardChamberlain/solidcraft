//! A design saves its identity (`id`, a hex string minted at its first save and again for a Save
//! As copy), the modelling approach it was made in (`approach`: `advanced`; Classic is not
//! written) and, in each design link, the identity of the design it links (`id`). Older designs
//! need no transformation: without them a design has no identity and is Classic, and a link has
//! no identity to compare. 0.14.x asks for an update instead of opening a design that has them.
