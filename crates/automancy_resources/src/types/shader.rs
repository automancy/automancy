use crate::{LoadResourceError, ResourceManager, COULD_NOT_GET_FILE_STEM, SHADER_EXT};
use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

impl ResourceManager {
    pub fn load_shaders(&mut self, dir: &Path) -> anyhow::Result<()> {
        let shaders = dir.join("shaders");
        if let Ok(shaders) = read_dir(shaders) {
            for file in shaders
                .into_iter()
                .flatten()
                .map(|v| v.path())
                .filter(|v| v.extension() == Some(OsStr::new(SHADER_EXT)))
            {
                log::info!("Loading shader at {file:?}");

                let name = file
                    .file_stem()
                    .ok_or_else(|| {
                        LoadResourceError::InvalidFileError(file.clone(), COULD_NOT_GET_FILE_STEM)
                    })?
                    .to_str()
                    .ok_or_else(|| LoadResourceError::OsStringError(file.clone()))?
                    .into();

                if let Ok(shader) = read_to_string(&file) {
                    self.shaders.insert(name, shader.into());
                }
            }
        }

        Ok(())
    }
}
