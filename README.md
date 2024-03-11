# automancy!

*A game about automation, hexagons and magic; and there's no Conveyor Belts.*

You're in a world... a plane, more like, and you're a sort of, Demigod who doesn't have a form. Your goal? Make yourself less lonely. There's seemingly infinite amount of resources just under the plane, but you cannot make sense of their usage and properties... yet.

Research your way through the mystifying colours, and create the stuff of your dreams- gold, energy, lil' hexagonal creatures, and, most importantly- Factories!

Perhaps, you'd be the magnificent "automancer"... if you so choose to call yourself. It's not like anyone else is around, heh?...



***Game's WIP.***

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

***Run `run.sh` to run the game.***

***Alternatively, run `cargo run -p build_script` before running the game.***

***There should be IntelliJ configurations for running the game. Simply run config `Run`.***

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
