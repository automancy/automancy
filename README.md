# automancy!

### -- TBD: Insert some more detailed description here --

### -- Game is WIP. --

-- NEW PREVIEW IMPENDING --

---

## ~ All the special credits ~

### Tech's

*automancy! is built on the shoulders of giants!*

Some of the major techs used:
- Rust (for the coding)
- wgpu (for the rendering)
- ractor (for the game's async handling)
    - Consequently: tokio
- yakui (for the GUI)
- Rhai (for the scripting)
    - TBD: we're switching to wasm
- Blender (for the modelling) 
- Rusty Object Notation - 'ron' (for the definition files)

### Folk's

*automancy! is a collaborative effort!*

Major programmers:
- Madeline Sparkles
- Mae Rosaline

Major modellers:
- Madeline Sparkles
- :c

Major SFX designers:
- :c

## ~ All the links ~

- Git: https://github.com/automancy/automancy
- Discord: https://discord.gg/ee9XebxNaa
- Bluesky (Madeline Sparkles): [@mouse.lgbt](https://bsky.app/profile/mouse.lgbt)

---

## ~ Dev Notes ~

### Designers

For SVG files, in order for them to be correctly converted to Blender files, the file needs to fit the following
criteria:

- The viewport is _exactly_ 160cm by 160cm (cm is used to reduce rounding errors).
- The file does not contain any color outside of fills.
    - That means, no gradients.
- The file does not contain any clipping or masking.
    - Use the boolean operators.
- The file does not contain any strokes.
    - Use "Stroke to Path" to convert them.

**Currently, the game supports neither materials nor textures,** **_and has no plans to support them._**

**Use either Vertex Paint or a material with only the base color (they get turned into vertex colors)**

### Scripts, aka How to Make The Game Real

-- WIP: VERY volatile and subject to change, do not doc this for now. --

### Programmers

#### The Standard™️ Operational®️ Procedure©️

- Install [just](https://github.com/casey/just)!
    - Then, execute `just run` or `just run dev` to run a dev build.
    - Or `just run staging` for a staging build, for a more performant build of the game.
    - And for release builds, `just run release`. This uses `opt-level=s` and `lto=fat` to minimize binary size.
    - If you just want to build the game, run `just build`. The same arguments should work too.
- After adding a dependency, please be sure to run:
    - `just sort`
        - This also depends on [cargo-sort](https://github.com/devinr528/cargo-sort).
    - `just license`
        - This also depends on [cargo-about](https://github.com/EmbarkStudios/cargo-about).

#### On Async

The rendering is single-threaded, the game logic is run with an actor system (ractor) on top of a Tokio runtime.

#### Write all tile logic in script-side

If you can't feasibly do that, *implement more handling in source code, and then write the logic.*

The "game" executable is supposed to be an engine, one very specialised to run hexagonal automation games. Today's hard-coded logic will become tomorrow's headache.

### Translators

-- WIP: we need a good way to define arbitrary languages and an easier way to write them. currently they are an absolute pain in the ass to write. --
