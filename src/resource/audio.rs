use crate::resource::{LoadResource, ResourceManager};
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use kira::sound::Sound;
use std::any::Any;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

impl LoadResource<StaticSoundData> for ResourceManager {
    fn load(resource_man: &mut ResourceManager, file: &Path) -> Option<()> {
        log::info!("loading audio at {:?}", file);

        if let Ok(audio) = StaticSoundData::from_file(
            file.clone(),
            StaticSoundSettings::default().track(&resource_man.track),
        ) {
            resource_man.audio.insert(
                file.file_stem().unwrap().to_str().unwrap().to_string(),
                audio,
            );
        };
        Some(())
    }

    const FILTER: dyn FnMut(&PathBuf) -> bool = (|v| v.extension() == Some(OsStr::new("ogg")));

    const DIR: String = String::from("audio");
}
