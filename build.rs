use std::{env, fs, io, path::Path};

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Tell cargo to rerun build script if anything inside `res/` changes
    println!("cargo:rerun-if-changed=res");

    let out_dir = env::var("OUT_DIR")?;
    let dest = Path::new(&out_dir).join("res");

    // Copy the project `res/` into OUT_DIR/res
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("res");
    if src.exists() {
        copy_dir_all(&src, &dest)?;
    }

    // Export the copied directory to the compiled crate as ASSETS_DIR
    // so code can use env!("ASSETS_DIR") to find files at runtime.
    println!("cargo:rustc-env=ASSETS_DIR={}", dest.display());

    Ok(())
}
