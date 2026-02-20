use std::path::PathBuf;
use std::sync::LazyLock;

pub(crate) const SOURCE_NAME: &str = "source";
pub(crate) const RESULT_NAME: &str = "result";
pub(crate) const PROCESSING_NAME: &str = "processing";
pub(crate) const CODE_FILE_PATH_NAME: &str = "code_file_path";
pub(crate) const AUDIO_FILE_PATH_NAME: &str = "audio_file_path";
pub(crate) const AUDIO_NAME: &str = "audio";

pub(crate) static PROGRAM_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from("..").join("audio").join("processing")
});
pub(crate) static RUST_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from("..").join("audio").join("processing").join("rust")
});
pub(crate) static CPP_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from("..").join("audio").join("processing").join("cpp")
});
pub(crate) static SOURCE_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from("..").join("audio").join("source")
});
pub(crate) static RESULT_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
    PathBuf::from("..").join("audio").join("result")
});
