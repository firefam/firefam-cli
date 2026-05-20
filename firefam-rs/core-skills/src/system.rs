pub(crate) use firefam_skills::install_system_skills;
pub(crate) use firefam_skills::system_cache_root_dir;

use firefam_utils_absolute_path::AbsolutePathBuf;

pub(crate) fn uninstall_system_skills(firefam_home: &AbsolutePathBuf) {
    let _ = std::fs::remove_dir_all(system_cache_root_dir(firefam_home));
}
