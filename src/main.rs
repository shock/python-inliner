use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("Usage: {} <input_file> <directory> <output_file>", args[0]);
        return;
    }

    let input_file = &args[1];
    let directory = &args[2];
    let output_file = &args[3];

    let contents = fs::read_to_string(input_file).unwrap();
    let inlined_contents = inline_imports(contents, directory);
    fs::write(output_file, inlined_contents).unwrap();
}

fn inline_imports(contents: String, directory: &str) -> String {
    let import_regex = Regex::new(r"import\s+([a-zA-Z0-9_\.]+)").unwrap();
    let mut inlined_contents = contents.clone();

    for cap in import_regex.captures_iter(&contents) {
        let import_path = cap[1].to_string();
        let import_path_parts: Vec<&str> = import_path.split('.').collect();
        let mut file_path = PathBuf::from(directory);

        for part in import_path_parts {
            file_path.push(part);
        }

        file_path.set_extension("py");

        if file_path.exists() {
            let file_contents = fs::read_to_string(file_path).unwrap();
            let inlined_file_contents = inline_imports(file_contents, directory);
            inlined_contents = inlined_contents.replace(&format!("import {}", import_path), &inlined_file_contents);
        }
    }

    inlined_contents
}
