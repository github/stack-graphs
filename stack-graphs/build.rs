extern crate cbindgen;

fn main() {
    generate_c_bindings();
}

fn generate_c_bindings() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();

    let config = cbindgen::Config::from_root_or_default(&crate_dir);
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate C bindings")
        .write_to_file(&format!("include/{}.h", crate_name));
}
