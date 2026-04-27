use std::path::PathBuf;

fn main() {
    let env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join(".env"));

    if let Some(path) = env_path {
        if path.exists() {
            let _ = dotenvy::from_path(&path);
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    for var in ["MS_CLIENT_ID", "GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"] {
        println!("cargo:rerun-if-env-changed={var}");
        if let Ok(val) = std::env::var(var) {
            println!("cargo:rustc-env={var}={val}");
        }
    }

    tauri_build::build();
}
