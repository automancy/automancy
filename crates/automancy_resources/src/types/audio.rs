use crate::{LoadResourceError, ResourceManager, AUDIO_EXT, COULD_NOT_GET_FILE_STEM};
use automancy_defs::kira::sound::static_sound::StaticSoundData;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

impl ResourceManager {
    pub fn load_audio(&mut self, dir: &Path) -> anyhow::Result<()> {
        let audio = dir.join("audio");

        if let Ok(audio) = read_dir(audio) {
            for file in audio
                .into_iter()
                .flatten()
                .map(|v| v.path())
                .filter(|v| v.extension() == Some(OsStr::new(AUDIO_EXT)))
            {
                log::info!("Loading audio at {file:?}");

                if let Ok(audio) = StaticSoundData::from_file(&file) {
                    let name = file
                        .file_stem()
                        .ok_or_else(|| {
                            LoadResourceError::InvalidFileError(
                                file.clone(),
                                COULD_NOT_GET_FILE_STEM,
                            )
                        })?
                        .to_str()
                        .ok_or_else(|| LoadResourceError::OsStringError(file.clone()))?;

                    self.audio.insert(name.into(), audio);

                    log::info!("Registered audio with name {name}");
                }
            }
        }

        Ok(())
    }
}
