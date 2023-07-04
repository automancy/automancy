use std::ffi::OsStr;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

use automancy_defs::flexstr::ToSharedStr;
use automancy_defs::log;

use crate::{ResourceManager, SHADER_EXT};

impl ResourceManager {
    pub fn load_shaders(&mut self, dir: &Path) -> Option<()> {
        let shaders = dir.join("shaders");
        let shaders = read_dir(shaders).ok()?;

        shaders
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(SHADER_EXT)))
            .for_each(|file| {
                log::info!("loading shader at {file:?}");

                if let Ok(shader) = read_to_string(&file) {
                    self.shaders.insert(
                        file.file_stem().unwrap().to_str().unwrap().to_shared_str(),
                        shader,
                    );
                }
            });

        Some(())
    }
}
