use cc;
const PATH_TO_CPP_DSP: &str = "../dsp/src/processing/cpp_process_audio.cpp";
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("{}", "cargo:rerun-if-changed=".to_owned() + PATH_TO_CPP_DSP);

    cc::Build::new()
        .cpp(true)
        .file(PATH_TO_CPP_DSP)
        .flag_if_supported("-O3")
        .compile("cpp_process_audio");
}