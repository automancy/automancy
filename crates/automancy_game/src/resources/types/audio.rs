use std::path::Path;

use kira::sound::static_sound::StaticSoundData;

use crate::resources::{AUDIO_EXTS, MutableResourceManager, ResourceError, read_recursively};

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    pub fn load_audio_file(&mut self, file: &Path) -> Result<(), ResourceError> {
        log::info!("Loading audio file at {}.", file.display());

        let name = file
            .file_stem()
            .ok_or_else(|| ResourceError::NoFileStem(file.to_path_buf()))?
            .to_str()
            .ok_or_else(|| ResourceError::OsStringError(file.to_path_buf()))?
            .to_string();
        let audio = StaticSoundData::from_file(file)?;

        self.audio.insert(name, audio);

        Ok(())
    }

    pub fn load_audio_files(&mut self, dir: &Path) {
        let path = dir.join("audio");

        for file in read_recursively(&path, AUDIO_EXTS) {
            match self.load_audio_file(&file) {
                Ok(_) => {},
                Err(err) => err.log_err(),
            }
        }
    }
}
