use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Look for .env in the workspace root first (g:/dev/calendo/.env), then fall
    // back to src-tauri/.env so users who place the file inside src-tauri/ also work.
    let candidates = [
        manifest.parent().map(|p| p.join(".env")),
        Some(manifest.join(".env")),
    ];

    for candidate in candidates.iter().flatten() {
        println!("cargo:rerun-if-changed={}", candidate.display());
        if candidate.exists() {
            let _ = dotenvy::from_path(candidate);
        }
    }

    for var in ["MS_CLIENT_ID", "GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET"] {
        println!("cargo:rerun-if-env-changed={var}");
        if let Ok(val) = std::env::var(var) {
            // Defensive trim: stray whitespace or trailing newlines from `.env` files
            // would otherwise be sent verbatim to the OAuth endpoint and cause errors
            // like "OAuth client was not found".
            let trimmed = val.trim();
            println!("cargo:rustc-env={var}={trimmed}");
        }
    }

    tauri_build::build();
}
