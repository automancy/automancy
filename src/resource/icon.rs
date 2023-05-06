use crate::resource::{ResourceManager, PNG_EXT};
use egui::ColorImage;
use flexstr::ToSharedStr;
use image::io::Reader;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

impl ResourceManager {
    pub fn load_icons(&mut self, dir: &Path, ui: &mut egui::Ui) -> Option<()> {
        let icons = dir.join("icons");
        let icons = read_dir(icons).ok()?;

        icons
            .into_iter()
            .flatten()
            .map(|v| v.path())
            .filter(|v| v.extension() == Some(OsStr::new(PNG_EXT)))
            .for_each(|file| {
                log::info!("loading icon at {file:?}");
                let name = file.file_stem().unwrap().to_str().unwrap().to_string();
                let image = Reader::open(file).unwrap().decode().unwrap().to_rgba8();

                self.icons.insert(
                    name.to_shared_str(),
                    ui.ctx().load_texture(
                        name,
                        ColorImage::from_rgba_unmultiplied(
                            [image.width() as _, image.height() as _],
                            image.as_flat_samples().as_slice(),
                        ),
                        Default::default(),
                    ),
                );
            });

        Some(())
    }
}
