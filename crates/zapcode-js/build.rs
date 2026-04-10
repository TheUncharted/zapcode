extern crate napi_build;

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if let Ok(type_def_dir) = env::var("NAPI_TYPE_DEF_TMP_FOLDER") {
        let mut type_def_path = PathBuf::from(type_def_dir);
        type_def_path.push(env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME is not set"));

        if let Some(parent) = type_def_path.parent() {
            fs::create_dir_all(parent).expect("failed to create type definition temp directory");
        }

        println!(
            "cargo:rustc-env=TYPE_DEF_TMP_PATH={}",
            type_def_path.display()
        );
    }

    napi_build::setup();
}
