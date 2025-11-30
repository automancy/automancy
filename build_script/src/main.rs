use core::sync::atomic::Ordering;
use std::{
    env::current_dir,
    ffi::OsStr,
    fs::metadata,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
    thread,
};

use walkdir::WalkDir;

fn load_recursively(path: &Path, extension: &OsStr) -> Vec<PathBuf> {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|v| v.path().extension() == Some(extension))
        .map(|v| v.path().to_path_buf())
        .collect()
}

fn file_check(path: &Path, out_path: &Path) -> bool {
    if std::env::var("SKIP_CHECK")
        .ok()
        .and_then(|v| bool::from_str(v.as_str()).ok())
        .unwrap_or(false)
    {
        return true;
    }

    let Ok(in_meta) = metadata(path) else {
        return true;
    };
    let Ok(out_meta) = metadata(out_path) else {
        return true;
    };

    !out_path.is_file() || in_meta.modified().and_then(|m| Ok(out_meta.modified()? < m)).unwrap_or(true)
}

fn main() {
    println!("Build script running in {:?}.", current_dir().unwrap().canonicalize().unwrap());

    if Command::new("blender").arg("--help").output().is_err() {
        panic!("Failed to find 'blender' command. Please install Blender and add it to PATH.")
    }

    let mut job_counter = 0;
    let resources = Path::new("resources/");

    // .svg -> .blend runs (parallel)
    let mut svg_handles = Vec::new();
    let svgo_warned = Arc::new(AtomicBool::new(false));
    for svg_path in load_recursively(resources, OsStr::new("svg")) {
        let out_path = svg_path.with_extension("blend");

        if file_check(&svg_path, &out_path) {
            let svgo_warned = svgo_warned.clone();

            svg_handles.push(thread::spawn(move || {
                {
                    if let Ok(child) = Command::new("svgo")
                        .arg(&svg_path)
                        .arg("-o")
                        .arg(&svg_path)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                    {
                        let output = child.wait_with_output().unwrap();
                        println!("{}", str::from_utf8(&output.stdout).unwrap());

                        if !output.status.success() {
                            panic!("(SVGO) Process produced status code {}\n{}", output.status, String::from_utf8(output.stderr).unwrap());
                        }
                    } else if svgo_warned.load(Ordering::Acquire) {
                        eprintln!("Couldn't optimize SVGs with SVGO. You should install SVGO as a command-line tool on your system and add it to PATH.");
                        eprintln!("SVGO: https://github.com/svg/svgo/");
                        eprintln!();
                        svgo_warned.store(true, Ordering::Release);
                    }
                }

                let output = Command::new("blender")
                    .args(["--background", "--python-exit-code", "1", "--python", "build_script/python/export_svg.py", "--"])
                    .arg(&svg_path)
                    .arg(&out_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap()
                    .wait_with_output()
                    .unwrap();
                println!("{}", str::from_utf8(&output.stdout).unwrap());

                if !output.status.success() {
                    panic!("Process produced status code {}\n{}", output.status, String::from_utf8(output.stderr).unwrap())
                }
            }));
        }
    }
    for handle in svg_handles {
        if let Err(err) = handle.join() {
            panic!("Error with converting SVG files to Blender files: {err:?}.");
        }
        job_counter += 1;
    }

    // .blend -> .glb runs (parallel)
    let mut model_handles = Vec::new();
    for blend_path in load_recursively(resources, OsStr::new("blend")) {
        let out_path = blend_path.with_extension("glb");

        if file_check(&blend_path, &out_path) {
            model_handles.push(thread::spawn(move || {
                let output = Command::new("blender")
                    .arg(&blend_path)
                    .args([
                        "--background",
                        "--python-exit-code",
                        "1",
                        "--python",
                        "build_script/python/export_blender.py",
                        "--",
                    ])
                    .arg(&out_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap()
                    .wait_with_output()
                    .unwrap();
                println!("{}", str::from_utf8(&output.stdout).unwrap());

                if !output.status.success() {
                    panic!(
                        "Process bad status code: {}\n{}",
                        output.status,
                        String::from_utf8(output.stderr).unwrap()
                    )
                }
            }));
        }
    }

    for handle in model_handles {
        if let Err(err) = handle.join() {
            panic!("Error with converting Blender files to glB files: {err:?}.");
        }
        job_counter += 1;
    }

    let job_status = if job_counter == 0 {
        "No jobs done.".to_string()
    } else {
        format!("Jobs done: {}.", job_counter)
    };
    println!("Build script finished! ({})", job_status);
}
