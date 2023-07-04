use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};

use automancy_defs::flexstr::ToSharedStr;
use automancy_defs::log;

use crate::{ResourceManager, AUDIO_EXT};

impl ResourceManager {
    pub fn load_audio(&mut self, dir: &Path) -> Option<()> {
        let audio = dir.join("audio");
        let audio = read_dir(audio).ok()?;

        audio
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(AUDIO_EXT)))
            .for_each(|file| {
                log::info!("loading audio at {file:?}");

                if let Ok(audio) = StaticSoundData::from_file(
                    &file,
                    StaticSoundSettings::default().output_destination(&self.track),
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
