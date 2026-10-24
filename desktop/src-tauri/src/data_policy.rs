//! Platform composition policy for notype's persistent data directory.

use std::path::{Path, PathBuf};

/// Resolve persistent data independently of the executable's versioned location.
/// The platform supplies its per-user app directory; runtime receipts may select
/// a different absolute directory through NOTYPE_DATA_DIR.
pub fn resolve_data_dir(
    per_user_directory: &Path,
    configured_directory: Option<&Path>,
) -> Result<PathBuf, String> {
    let directory = configured_directory.unwrap_or(per_user_directory);
    if !directory.is_absolute() {
        return Err("notype data directory must be an absolute path".to_string());
    }
    Ok(directory.to_path_buf())
}
