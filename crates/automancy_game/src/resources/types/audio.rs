use std::path::Path;

use automancy_data::id::IdInterner;
use kira::sound::static_sound::StaticSoundData;

use crate::resources::{AUDIO_EXTS, MutableResourceManager, ResourceError, read_recursively};

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    pub fn load_audio_file(&mut self, _interner: &mut IdInterner, path: &Path) -> Result<(), ResourceError> {
        log::info!("Loading audio file at {}.", path.display());

        let name = path
            .file_stem()
            .ok_or_else(|| ResourceError::NoFileStem(path.to_path_buf()))?
            .to_str()
            .ok_or_else(|| ResourceError::OsStringError(path.to_path_buf()))?
            .to_string();
        let audio = StaticSoundData::from_file(path)?;

        self.audio.insert(name, audio);

        Ok(())
    }

    pub fn load_audio_files(dir: &Path) {
        MutableResourceManager::with_interner(|resource_man, interner| {
            let path = dir.join("audio");

            for entry in read_recursively(&path, AUDIO_EXTS) {
                match resource_man.load_audio_file(interner, entry.path()) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }
}
