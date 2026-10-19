//! An Import feature can save `part_face_names`: what the faces of a linked part are called in the design
//! it was linked from (per body of the file, per face). A face with a name there takes it here, so a
//! reference to it follows the part when the part changes shape; the face's place in the file
//! (`face:<k>`) stays an alias. Older designs have none and name their faces by place, as before; 0.14.x
//! asks for an update instead of opening a design that has them.
