use std::collections::HashSet;
use std::fs as fs;
use std::path::{Path, PathBuf};
use std::error::Error;
use regex::Regex;
use structopt::StructOpt;
mod modules {
    pub mod file_system;
    pub mod virtual_filesystem;
}

use modules::file_system::RealFileSystem;
use modules::file_system::FileSystem;


#[cfg(test)]
mod test_utils;

#[derive(StructOpt, Debug)]
#[structopt(name = "python-inliner", about = "Python File Inliner - https://github.com/shock/python-inliner")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,

    #[structopt(parse(from_os_str))]
    output_file: PathBuf,

    #[structopt(help = "Name of the module to be inlined", default_value = "")]
    modules_name: String,

    #[structopt(long, short = "r", help = "Suppress comments in the output, and consolidate imports", takes_value = false)]
    release: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    // get current working directory
    let current_dir = fs::canonicalize(".")?;
    let fs = RealFileSystem::new(current_dir);
    run(opt, fs)
}

fn run<FS: FileSystem>(opt: Opt, mut fs: FS) -> Result<(), Box<dyn Error>> {
    // get the input_file as a fully qualified path
    let input_file = fs.canonicalize(&opt.input_file)?;

    // get the working directory from the input file path
    let working_dir = input_file.parent().unwrap();

    let mut content = inline_imports(&mut fs, &working_dir, &opt.input_file, &opt.modules_name, &mut HashSet::new(), &opt)?;
    if opt.release {
        content = post_process_imports(&content);
    }
    fs.write(&opt.output_file, content)?;
    println!("Inlined content written to {:?}", opt.output_file);
    Ok(())
}


fn inline_imports<FS: FileSystem>(fs: &mut FS, workding_dir: &Path, file: &Path, modules_name: &str, processed: &mut HashSet<PathBuf>, opt: &Opt) -> Result<String, Box<dyn Error>> {
    if !processed.insert(file.to_path_buf()) {
        println!("WARNING: already inlined {}.  Skipping...", file.display());
        return Ok(String::new());
    }

    let content = fs.read_to_string(file)?;
    let import_regex = Regex::new(&format!(r"(?m)^([ \t]*)from\s+{}(\S*)\s+import\s+.+$", regex::escape(modules_name)))?;

    let mut result = String::new();
    let mut last_end = 0;

    for cap in import_regex.captures_iter(&content) {
        let indent = &cap[1];
        let submodule = &cap[2];
        let start = cap.get(0).unwrap().start();
        let end = cap.get(0).unwrap().end();
        result.push_str(&content[last_end..start]);

        let module_path = workding_dir.join(modules_name.replace(".", "/") + &submodule.replace(".", "/") + ".py");
        if module_path.exists() {
            let inlined_content = inline_imports(fs, workding_dir, &module_path, modules_name, processed, opt)?;
            if !opt.release {
                result.push_str(&format!("{indent}# ↓↓↓ inlined module: {}{}\n", modules_name, submodule));
            }
            result.push_str(&indent);
            result.push_str(&inlined_content.replace("\n", &format!("\n{indent}")));
            // result.push_str("\n");
            if !opt.release {
                result.push_str(&format!("\n{indent}# ↑↑↑ inlined module: {}{}\n", modules_name, submodule));
            }
        } else {
            result.push_str(&content[start..end]);
            // result.push('\n');
        }

        last_end = end;
    }

    result.push_str(&content[last_end..]);
    processed.remove(file);
    Ok(result)
}

fn post_process_imports(content: &str) -> String {
    let mut imports = HashSet::new();
    let mut header_content = Vec::new();
    let mut other_content = Vec::new();
    let import_regex = Regex::new(r"(?m)^\s*(import|from)\s+").unwrap();
    let shebang_regex = Regex::new(r"^#!").unwrap();

    let mut lines = content.lines().collect::<Vec<&str>>();

    if let Some(first_line) = lines.first() {
        if shebang_regex.is_match(first_line) {
            header_content.push(first_line.to_string());
            header_content.push("\n".to_string());
            lines.remove(0);
        }
    }

    for line in lines {
        if import_regex.is_match(line) {
            imports.insert(line.trim_start().to_string());
        } else {
            other_content.push(line.to_string());
        }
    }

    let mut result = String::new();
    result.push_str(&header_content.join("\n"));
    let mut imports_vec: Vec<String> = imports.into_iter().collect();
    imports_vec.sort();
    result.push_str(&imports_vec.join("\n"));
    result.push('\n');
    result.push_str(&other_content.join("\n"));
    result
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::test_utils::{MockFileSystem, setup_test_env};
//     use crate::modules::file_system::RealFileSystem;
//     use crate::modules::virtual_filesystem::VirtualFileSystem;
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::test_utils::{MockFileSystem, setup_test_env};

//     fn mock_inline_imports(
//         mock_fs: &MockFileSystem,
//         working_dir: &Path,
//         file: &Path,
//         modules_name: &str,
//         processed: &mut HashSet<PathBuf>,
//         opt: &Opt,
//     ) -> Result<String, Box<dyn Error>> {
//         let content = mock_fs.read_file(file).ok_or("File not found")?;
//         let import_regex = Regex::new(&format!(r"(?m)^([ \t]*)from\s+{}(\S*)\s+import\s+.+$", regex::escape(modules_name)))?;

//         let mut result = String::new();
//         let mut last_end = 0;

//         for cap in import_regex.captures_iter(&content) {
//             let indent = &cap[1];
//             let submodule = &cap[2];
//             let start = cap.get(0).unwrap().start();
//             let end = cap.get(0).unwrap().end();
//             result.push_str(&content[last_end..start]);

//             let module_path = working_dir.join(modules_name.replace(".", "/") + &submodule.replace(".", "/") + ".py");
//             if mock_fs.file_exists(&module_path) {
//                 let inlined_content = mock_inline_imports(mock_fs, working_dir, &module_path, modules_name, processed, opt)?;
//                 if !opt.release {
//                     result.push_str(&format!("{indent}# ↓↓↓ inlined module: {}{}\n", modules_name, submodule));
//                 }
//                 result.push_str(&indent);
//                 result.push_str(&inlined_content.replace("\n", &format!("\n{indent}")));
//                 if !opt.release {
//                     result.push_str(&format!("\n{indent}# ↑↑↑ inlined module: {}{}\n", modules_name, submodule));
//                 }
//             } else {
//                 result.push_str(&content[start..end]);
//             }

//             last_end = end;
//         }

//         result.push_str(&content[last_end..]);
//         processed.remove(file);
//         Ok(result)
//     }

//     #[test]
//     fn test_inline_imports_simple() {
//         let mock_fs = setup_test_env();
//         mock_fs.add_file("/test/main.py", "from modules import module1\n\nprint('Hello')");
//         mock_fs.add_file("/test/modules/module1.py", "def func1():\n    print('Function 1')");

//         let opt = Opt {
//             input_file: PathBuf::from("/test/main.py"),
//             output_file: PathBuf::from("/test/main_inlined.py"),
//             modules_name: "modules".to_string(),
//             release: false,
//         };

//         let result = mock_inline_imports(
//             &mock_fs,
//             Path::new("/test"),
//             Path::new("/test/main.py"),
//             "modules",
//             &mut HashSet::new(),
//             &opt,
//         ).unwrap();

//         assert_eq!(result, "# ↓↓↓ inlined module: modules\ndef func1():\n    print('Function 1')\n# ↑↑↑ inlined module: modules\n\nprint('Hello')");
//     }

//     #[test]
//     fn test_post_process_imports() {
//         let input = r#"#!/usr/bin/env python3
// import sys
// from os import path
// import re

// def main():
//     print('Hello')

// if __name__ == '__main__':
//     main()
// "#;

//         let expected = r#"#!/usr/bin/env python3
// from os import path
// import re
// import sys

// def main():
//     print('Hello')

// if __name__ == '__main__':
//     main()
// "#;

//         assert_eq!(post_process_imports(input), expected);
//     }
// }
