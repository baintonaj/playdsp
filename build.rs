use std::path::PathBuf;
use cc;
const PATH_TO_CPP_DSP: &str = "/src/external_processing/cpp_process_audio.cpp";

fn get_documents_playdsp_path() -> Option<PathBuf> {
    let home_dir = home::home_dir()?;
    Some(home_dir.join("Documents").join("playdsp"))
}

fn main() {

    let user_path =  get_documents_playdsp_path().unwrap().to_str().unwrap().to_owned();
    let external_processing_path = user_path.clone() + PATH_TO_CPP_DSP;

    println!("cargo:rerun-if-changed=build.rs");
    println!("{}", "cargo:rerun-if-changed=".to_owned() + &*external_processing_path);

    cc::Build::new()
        .cpp(true)
        .file(external_processing_path)
        .flag_if_supported("-O3")
        .compile("cpp_process_audio");
}