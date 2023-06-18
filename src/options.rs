use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

use automancy_defs::hashbrown::HashMap;
use automancy_defs::winit::event::VirtualKeyCode;
use serde::{Deserialize, Serialize};

use crate::input::KeyActions;

#[derive(Serialize, Deserialize)]
pub struct Options {
    pub keymap: HashMap<u32, KeyActions>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            keymap: HashMap::from([
                (VirtualKeyCode::Z as u32, KeyActions::UNDO),
                (VirtualKeyCode::Escape as u32, KeyActions::PAUSE),
                (VirtualKeyCode::F3 as u32, KeyActions::DEBUG),
            ]),
        }
    }
}

impl Options {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let file = File::open("options.toml")?;
        let mut body = String::new();
        BufReader::new(file).read_to_string(&mut body)?;
        Ok(toml::de::from_str(body.as_str())?)
    }
    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let file = File::open("options.toml")?;
        let body = toml::ser::to_string_pretty(self)?;
        let mut buffer = BufWriter::new(file);
        write!(&mut buffer, "{body}")?;
        Ok(())
    }
}
