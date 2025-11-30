use kira::{PlaySoundError, sound::SoundData};

pub struct StubbedAudioManager;

impl StubbedAudioManager {
    pub fn play<D: SoundData>(&mut self, sound_data: D) -> Result<D::Handle, PlaySoundError<D::Error>> {
        Err(PlaySoundError::SoundLimitReached)
    }
}
