use core::{cell::Cell, fmt::Write};
use std::{fs::File, path::Path};

use native_dialog::{DialogBuilder, MessageLevel};
use uuid::Uuid;

fn write_crash_msg<P: AsRef<Path>>(buffer: &mut impl Write, file_path: P) -> std::fmt::Result {
    writeln!(buffer, "Oh, sorry... automancy! has crashed.")?;
    writeln!(buffer, "To help diagnose the problem, you can send us a crash report.")?;

    writeln!(buffer)?;

    writeln!(
        buffer,
        "The game has generated a report at this location for more information:\n{}",
        file_path.as_ref().display(),
    )?;

    writeln!(buffer)?;

    writeln!(
        buffer,
        "Submit an issue on the Git repository or tag us in the Discord server, and include the report as an attachment:"
    )?;
    writeln!(buffer, "- Git: https://github.com/automancy/automancy")?;
    writeln!(buffer, "- Discord: https://discord.gg/ee9XebxNaa")?;
    writeln!(buffer, "Alternatively, send an Email to these addresses:")?;
    writeln!(buffer, "- Madeline Sparkles (madeline@mouse.lgbt)")?;

    writeln!(buffer)?;

    writeln!(
        buffer,
        "We take privacy seriously, and do not perform any kinds of automated error collection! In order to improve the game, we rely on people to submit reports."
    )?;
    writeln!(buffer, "Thank you for understanding!")?;

    Ok(())
}

pub(crate) fn install_panic_hook() -> anyhow::Result<()> {
    let eyre = color_eyre::config::HookBuilder::blank()
        .add_default_filters()
        .capture_span_trace_by_default(true)
        .display_env_section(false);
    let (panic_hook, eyre_hook) = eyre.into_hooks();
    eyre_hook.install()?;

    std::panic::set_hook(Box::new(move |info| {
        thread_local! {
            static ALREADY_PANICKED: Cell<bool> = const { Cell::new(false) };
        }
        // only generate report for the first panic, the rest is most likely panics caused by the first one.
        if ALREADY_PANICKED.get() {
            return;
        }
        ALREADY_PANICKED.set(true);

        let report = panic_hook.panic_report(info);

        let uuid = Uuid::new_v4().hyphenated().to_string();
        let tmp_dir = std::env::temp_dir();
        let file_name = format!("automancy-report-{uuid}.txt");
        let file_path = tmp_dir.join(file_name);

        if let Ok(mut file) = File::create(&file_path) {
            use std::io::Write;

            let _ = write!(file, "{}", strip_ansi_escapes::strip_str(report.to_string()));
        }

        let mut message = String::new();
        let _ = write_crash_msg(&mut message, &file_path);

        eprintln!("{}", report);
        eprintln!();
        eprintln!("{}", message);

        let mut dialog_text = String::new();
        let _ = writeln!(dialog_text, "{}", message);
        let _ = writeln!(dialog_text);
        let _ = writeln!(dialog_text);
        let _ = writeln!(dialog_text, "Do you want to open the report file right now?");

        if let Ok(confirm) = DialogBuilder::message()
            .set_title("automancy crash dialog")
            .set_text(dialog_text)
            .set_level(MessageLevel::Error)
            .confirm()
            .show()
            && confirm
        {
            let _ = open::that_detached(file_path);
        }
    }));

    Ok(())
}
