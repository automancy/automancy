use std::env::current_dir;
use std::ffi::OsStr;
use std::fs::metadata;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str;
use std::str::FromStr;
use std::thread;

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

    !out_path.is_file()
        || in_meta
            .modified()
            .and_then(|m| Ok(out_meta.modified()? < m))
            .unwrap_or(true)
}

fn main() {
    println!(
        "Build script running in {:?}",
        current_dir().unwrap().canonicalize().unwrap()
    );

    if Command::new("blender").arg("--help").output().is_err() {
        println!("\n\n==============");
        println!("Failed to find blender command: please add blender to your PATH.");
        println!("==============\n\n");

        panic!()
    }

    let resources = Path::new("resources/");

    // SVG blender runs (parallel)
    let mut svg_handles = Vec::new();
    for svg_path in load_recursively(resources, OsStr::new("svg")) {
        let out_path = svg_path.with_extension("blend");

        if file_check(&svg_path, &out_path) {
            svg_handles.push(thread::spawn(move || {
                let output = Command::new("blender")
                    .args([
                        "--background",
                        "--python-exit-code",
                        "1",
                        "--python",
                        "scripts/export_svg.py",
                        "--",
                    ])
                    .arg(svg_path)
                    .arg(out_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("blender processing couldn't start")
                    .wait_with_output()
                    .unwrap();
                println!("{}", str::from_utf8(&output.stdout).unwrap());

                if !output.status.success() {
                    panic!("Process bad status code: {}", output.status)
                }
            }));
        }
    }
    for handle in svg_handles {
        if let Err(e) = handle.join() {
            panic!("Issue with svg to blend: {e:?}");
        }
    }

    // Model blender runs (parallel)
    let mut model_handles = Vec::new();
    for blend_path in load_recursively(resources, OsStr::new("blend")) {
        let out_path = blend_path.with_extension("glb");

        if file_check(&blend_path, &out_path) {
            model_handles.push(thread::spawn(move || {
                let output = Command::new("blender")
                    .arg(blend_path)
                    .args([
                        "--background",
                        "--python-exit-code",
                        "1",
                        "--python",
                        "scripts/export_blender.py",
                        "--",
                    ])
                    .arg(out_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("blender processing couldn't start")
                    .wait_with_output()
                    .unwrap();
                println!("{}", str::from_utf8(&output.stdout).unwrap());

                if !output.status.success() {
                    panic!("Process bad status code: {}", output.status)
                }
            }));
        }
    }

    for handle in model_handles {
        if let Err(e) = handle.join() {
            panic!("Issue with blend to gltf: {e:?}");
        }
    }
}
