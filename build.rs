use embed_manifest::manifest::DpiAwareness;
use embed_manifest::{embed_manifest, new_manifest};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let manifest = new_manifest("grammy").dpi_awareness(DpiAwareness::Unaware);
        embed_manifest(manifest).expect("unable to embed manifest file");

        let icon_path = Path::new("assets/icon.ico");
        if !icon_path.exists() {
            let png_path = Path::new("assets/icon.png");
            if png_path.exists() {
                let img = image::open(png_path).expect("Failed to open icon.png");
                // Save as ico
                if let Err(e) = img.save(icon_path) {
                    println!("cargo:warning=Failed to generate icon.ico: {}", e);
                }
            }
        }

        if icon_path.exists() {
            let rc_path = Path::new("assets/icon.rc");
            if !rc_path.exists() {
                let _ = fs::write(rc_path, r#"icon_id ICON "icon.ico""#);
            }
            embed_resource::compile(rc_path, embed_resource::NONE);
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.png");
}
