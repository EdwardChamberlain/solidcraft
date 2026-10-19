//! The designs open in the window, as linked parts see them.
//!
//! A linked part is read from its file (`cmd::component_link`). When the same part is open in
//! another tab, its file is behind what is on screen: edits made since it was saved are not in
//! it. A host that has several designs open (the desktop app's tabs) says which ones, by path,
//! around the calls that read parts ([`with_open_designs`]); a part found here is taken as it is
//! now instead of from its file, so an assembly follows the part being edited without it being
//! saved first. Without a host (the CLI, a script) nothing is registered and the files are read.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use solidcraft_doc::Document;

thread_local! {
    static OPEN: RefCell<Vec<(PathBuf, Arc<Document>)>> = const { RefCell::new(Vec::new()) };
}

/// Run `f` with these designs (path, design) open on this thread. Designs without a path can't
/// be linked and are left out by the caller.
pub fn with_open_designs<R>(designs: impl IntoIterator<Item = (String, Arc<Document>)>, f: impl FnOnce() -> R) -> R {
    struct Restore(Vec<(PathBuf, Arc<Document>)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OPEN.with(|o| *o.borrow_mut() = std::mem::take(&mut self.0));
        }
    }
    let open: Vec<_> = designs.into_iter().map(|(path, doc)| (crate::cmd::normalized(Path::new(&path)), doc)).collect();
    let _restore = Restore(OPEN.with(|o| std::mem::replace(&mut *o.borrow_mut(), open)));
    f()
}

/// The open design saved at `file`, as it is now.
pub(crate) fn open_design(file: &Path) -> Option<Document> {
    let file = crate::cmd::normalized(file);
    OPEN.with(|o| o.borrow().iter().find(|(p, _)| *p == file).map(|(_, d)| (**d).clone()))
}

/// Do these two paths name the same file (made absolute, `.` and `..` worked out)?
pub fn same_file(a: &str, b: &str) -> bool {
    crate::cmd::normalized(Path::new(a)) == crate::cmd::normalized(Path::new(b))
}
