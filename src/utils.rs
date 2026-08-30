use std::path::Path;
use walkdir::WalkDir;

pub fn find_file_path_from_file_name(file_name: &str, src_path: &Path) -> Vec<String> {
    let mut results = vec![];
    for entry in WalkDir::new(src_path) {
        let entry = entry.unwrap();

        if entry.file_type().is_file() && entry.file_name() == file_name {
            results.push(
                entry
                    .path()
                    .strip_prefix(src_path)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
        }
    }
    results
}
