use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

use automancy_defs::flexstr::ToSharedStr;
use automancy_defs::log;

use crate::{LoadResourceError, ResourceManager, COULD_NOT_GET_FILE_STEM, SHADER_EXT};

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
                log::info!("loading shader at {file:?}");

                if let Ok(shader) = read_to_string(&file) {
                    self.shaders.insert(
                        file.file_stem()
                            .ok_or_else(|| {
                                LoadResourceError::InvalidFileError(
                                    file.clone(),
                                    COULD_NOT_GET_FILE_STEM,
                                )
                            })?
                            .to_str()
                            .ok_or_else(|| LoadResourceError::OsStringError(file.clone()))?
                            .to_shared_str(),
                        shader,
                    );
                }
            }
        }

        Ok(())
    }
}
