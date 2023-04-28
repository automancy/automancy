use flexstr::ToSharedStr;
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

use crate::resource::{ResourceManager, OGG_EXT};

impl ResourceManager {
    pub fn load_audio(&mut self, dir: &Path) -> Option<()> {
        let audio = dir.join("audio");
        let audio = read_dir(audio).ok()?;

        audio
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(OGG_EXT)))
            .for_each(|file| {
                log::info!("loading audio at {file:?}");

                if let Ok(audio) = StaticSoundData::from_file(
                    file.clone(),
                    StaticSoundSettings::default().track(&self.track),
                ) {
                    self.audio.insert(
                        file.file_stem().unwrap().to_str().unwrap().to_shared_str(),
                        audio,
                    );
                }
            });

        Some(())
    }
}
