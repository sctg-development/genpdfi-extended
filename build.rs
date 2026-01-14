use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Directory containing the helper web project
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    // Web helper is now located at the repository root (`./mermaid_pool`). Keep a fallback
    // to the historical `examples/mermaid_pool` path for backwards compatibility.
    let mut web_dir = manifest_dir.join("mermaid_pool");

    // If the web dir doesn't exist at the new location, fall back to the old one; if
    // neither exist, skip the web build.
    if !web_dir.exists() {
        let fallback = manifest_dir.join("examples/mermaid_pool");
        if fallback.exists() {
            println!("cargo:warning=Using legacy helper path {}", fallback.display());
            // Use fallback for the remainder of the build script
            web_dir = fallback;
        } else {
            println!("cargo:warning=No web helper dir at {} or at {} -- skipping web build", web_dir.display(), fallback.display());
            return Ok(());
        }
    }

    // Collect files we care about and emit rerun-if-changed for each
    let mut latest_src_mtime = None::<std::time::SystemTime>;
    visit_files(&web_dir, &mut |p| {
        // skip build outputs and node modules
        if p.components().any(|c| c.as_os_str() == "node_modules") {
            return Ok(());
        }
        if p.components().any(|c| c.as_os_str() == "dist") {
            return Ok(());
        }
        if p.is_file() {
            println!("cargo:rerun-if-changed={}", p.display());
            if let Ok(meta) = fs::metadata(p) {
                if let Ok(m) = meta.modified() {
                    latest_src_mtime = Some(match latest_src_mtime {
                        Some(prev) => std::cmp::max(prev, m),
                        None => m,
                    });
                }
            }
        }
        Ok(())
    })?;

    // Output file to check
    let dist_index = web_dir.join("dist").join("index.html");
    let need_build = match fs::metadata(&dist_index) {
        Ok(meta) => {
            if let Ok(dist_mtime) = meta.modified() {
                match latest_src_mtime {
                    Some(src_mtime) => src_mtime > dist_mtime,
                    None => false,
                }
            } else {
                true
            }
        }
        Err(_) => true,
    };

    if need_build {
        println!("cargo:warning=Web sources changed (or dist missing) -> running npm ci && npm run build in {}", web_dir.display());

        // Check if npm is available
        match Command::new("npm").arg("--version").output() {
            Ok(o) if o.status.success() => {
                // run npm ci
                let status = Command::new("npm").arg("ci").current_dir(&web_dir).status()?;
                if !status.success() {
                    return Err(format!("`npm ci` failed in {}", web_dir.display()).into());
                }

                // run npm run build
                let status = Command::new("npm").arg("run").arg("build").current_dir(&web_dir).status()?;
                if !status.success() {
                    return Err(format!("`npm run build` failed in {}", web_dir.display()).into());
                }

                println!("cargo:warning=Web helper build completed.");
            }
            _ => {
                println!("cargo:warning=npm not found in PATH; skipping web build (install Node/npm or run the helper build manually in {})", web_dir.display());
            }
        }
    } else {
        println!("cargo:warning=Web helper appears up-to-date; skipping web build");
    }

    Ok(())
}

fn visit_files(dir: &Path, cb: &mut dyn FnMut(&Path) -> io::Result<()>) -> io::Result<()> {
    if dir.is_file() {
        cb(dir)?;
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            if name == "node_modules" || name == "dist" || name == ".git" {
                continue;
            }
            visit_files(&path, cb)?;
        } else {
            cb(&path)?;
        }
    }
    Ok(())
}