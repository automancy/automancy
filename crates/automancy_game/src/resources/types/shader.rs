use std::path::Path;

use crate::resources::{MutableResourceManager, ResourceError, SHADER_EXTS, read_recursively};

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    pub fn load_shader_file(&mut self, path: &Path) -> Result<(), ResourceError> {
        log::info!("Loading shader at {}.", path.display());

        let name = path
            .file_stem()
            .ok_or_else(|| ResourceError::NoFileStem(path.to_path_buf()))?
            .to_str()
            .ok_or_else(|| ResourceError::OsStringError(path.to_path_buf()))?
            .into();
        let shader = std::fs::read_to_string(path)?;

        self.shaders.insert(name, shader);

        Ok(())
    }

    pub fn load_shader_files(dir: &Path) {
        MutableResourceManager::with(|resource_man| {
            let path = dir.join("shaders");

            for entry in read_recursively(&path, SHADER_EXTS) {
                match resource_man.load_shader_file(entry.path()) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }
}
