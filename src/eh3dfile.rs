// EH3D-GERBERVIEWER - the `.eh3d` project of the suite.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//
//! The `.eh3d` project of the suite: the job is read from the branch `manufacturing/` and the picture made from it is kept in the branch `gerberviewer/` only, with a reference that
//! tells when the job changed. The reading, the writing and the references are the ones every manufacturing program shares (`eh3d_project::manufacturing` of EH3D-COMMON); this module
//! names the branch of the viewer.

use eh3d_project::appbranch::BranchKit;

pub use eh3d_project::manufacturing::{
    describe, is_project, out_of_date_message, read_job, read_result, save_result, state, JobFile,
    State, FORMAT, MAX_JOB_BYTES, MAX_JOB_FILES, RESULT_DIR, SOURCE_BRANCH,
};

/// The branch of the viewer.
pub const KIT: BranchKit<'static> = eh3d_project::manufacturing::kit(
    "gerberviewer",
    "gerberviewer.json",
    env!("CARGO_PKG_VERSION"),
);
