use std::path::Path;

use log;

use crate::resources::{FONT_EXTS, MutableResourceManager, ResourceError, read_recursively};

#[cfg_attr(feature = "profile", profiling::all_functions)]
impl MutableResourceManager {
    fn load_font_file(&mut self, path: &Path) -> Result<(), ResourceError> {
        log::info!("Loading font {}.", path.display());

        let ids = self.fonts.load_font_source(fontdb::Source::File(path.to_path_buf()));

        for &id in &ids {
            // SAFETY: always unsafe, just uh, don't change fonts while the game is running? lol
            unsafe { self.fonts.make_shared_face_data(id) };
        }

        for &id in &ids {
            let face = self.fonts.face(id).unwrap();
            log::info!(
                "Loaded font '{} ({:?}, {:?}, {:?})'!",
                face.families.first().map(|(name, _lang)| name).unwrap_or(&face.post_script_name),
                face.weight,
                face.style,
                face.stretch
            );
        }

        Ok(())
    }

    pub fn load_font_files(dir: &Path) {
        MutableResourceManager::with(|resource_man| {
            let path = dir.join("fonts");

            for entry in read_recursively(&path, FONT_EXTS) {
                match resource_man.load_font_file(entry.path()) {
                    Ok(_) => {},
                    Err(err) => err.log_err(),
                }
            }
        })
    }
}
