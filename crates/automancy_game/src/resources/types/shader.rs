use std::path::Path;

use crate::resources::{MutableResourceManager, ResourceError, SHADER_EXTS, read_recursively};

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    pub fn load_shader_file(&mut self, file: &Path) -> Result<(), ResourceError> {
        log::info!("Loading shader at {}.", file.display());

        let name = file
            .file_stem()
            .ok_or_else(|| ResourceError::NoFileStem(file.to_path_buf()))?
            .to_str()
            .ok_or_else(|| ResourceError::OsStringError(file.to_path_buf()))?
            .into();
        let shader = std::fs::read_to_string(file)?;

        self.shaders.insert(name, shader);

        Ok(())
    }

    pub fn load_shader_files(&mut self, dir: &Path) {
        let path = dir.join("shaders");

        for file in read_recursively(&path, SHADER_EXTS) {
            match self.load_shader_file(&file) {
                Ok(_) => {},
                Err(err) => err.log_err(),
            }
        }
    }
}
