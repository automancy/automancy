# automancy!

A game about automation, hexagons and magic; and there's no Conveyor Belts.

Game's WIP.

---

Tech used: Rust, wgpu, ractor, egui, Rhai

Lead dev(s): Madeline Sparkles, Mae Rosaline

---

https://user-images.githubusercontent.com/33348966/225474725-9f6494d3-bc51-4969-add9-ea9cfea03578.mp4

---

Links:

- Fedi(Mastodon): https://gamedev.lgbt/@automancy
- Git: https://github.com/automancy/automancy
- Discord: https://discord.gg/ee9XebxNaa

## Development Notes

### Designers

For SVG files, in order for them to be correctly converted to Blender files, the file needs to fit the following
criteria:

- The viewport is *exactly* 160cm by 160cm (cm is used to reduce rounding errors).
- The file does not contain any color outside of fills.
    - That means, no gradients.
- The file does not contain any clipping or masking.
    - Use the boolean operators.
- The file does not contain any strokes.
    - Use "Stroke to Path" to convert them.

**Currently, the game supports neither materials nor textures,** ***and has no plans to support them.***

**Use either Vertex Paint or a material with only the base color (they get turned into vertex colors)**

### Scripts

`input.data` (aka DataMap) needs to be *manually assigned if you make modifications*.

### Software

The rendering is single-threaded, the game logic is run with an actor system on top of a Tokio runtime.

"Scripts" are called "functions" as the name is taken in-game by what would otherwise be called "recipes."

- The weird terminology comes from the fact that "recipes" doesn't make sense for machines.

#### Write all tile logic within functions

If you can't feasibly do that, *implement more handling in source code, and then write the logic in functions.*

### Translators

[WIP]
