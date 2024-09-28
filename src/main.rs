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
    let mut fs = RealFileSystem::new(current_dir);
    run(opt, &mut fs)
}

fn run<FS: FileSystem>(opt: Opt, fs: &mut FS) -> Result<(), Box<dyn Error>> {
    // get the input_file as a fully qualified path
    let input_file = fs.canonicalize(&opt.input_file)?;

    // get the working directory from the input file path
    let working_dir = input_file.parent().unwrap();

    let mut content = inline_imports(fs, &working_dir, &opt.input_file, &opt.modules_name, &mut HashSet::new(), &opt)?;
    if opt.release {
        content = post_process_imports(&content);
    }
    fs.write(&opt.output_file, content)?;
    println!("Inlined content written to {:?}", opt.output_file);
    Ok(())
}


fn inline_imports<FS: FileSystem>(fs: &mut FS, workding_dir: &Path, file: &Path, modules_name: &str, processed: &mut HashSet<PathBuf>, opt: &Opt) -> Result<String, Box<dyn Error>> {
    let content = fs.read_to_string(file)?;
    let import_regex = Regex::new(&format!(r"(?m)^([ \t]*)from\s+{}(\S*)\s+import\s+.+$", regex::escape(modules_name)))?;
    // let import_regex = Regex::new(&format!(r"(?m)^([ \t]*)from\s+{}(\S*)\s+import\s+.+$", regex::escape(modules_name)))?;

    let mut result = String::new();
    let mut last_end = 0;
    let captures = import_regex.captures_iter(&content);
    for cap in captures {
        println!("capture: {:?}", &cap[0]);
        let indent = &cap[1];
        let submodule = &cap[2];
        let start = cap.get(0).unwrap().start();
        let mut end = cap.get(0).unwrap().end();
        result.push_str(&content[last_end..start]);

        let module_path = workding_dir.join(modules_name.replace(".", "/") + &submodule.replace(".", "/") + ".py");
        if fs.exists(&module_path).unwrap() {
            println!("working_dir: {:?}, module_path: {:?}", workding_dir, module_path.to_path_buf());
            println!("processed before: {:?}", processed);
            if processed.insert(module_path.to_path_buf()) {
                let inlined_content = inline_imports(fs, workding_dir, &module_path, modules_name, processed, opt)?;

                if !opt.release {
                    result.push_str(&format!("{indent}# ↓↓↓ inlined module: {}{}\n", modules_name, submodule));
                }
                result.push_str(&indent);
                result.push_str(&inlined_content.replace("\n", &format!("\n{indent}")));
                // result.push_str("\n");
                if !opt.release {
                    result.push_str(&format!("\n{indent}# ↑↑↑ inlined module: {}{}", modules_name, submodule));
                }
            } else {
                println!("WARNING: already inlined {}.  Skipping...", module_path.display());
                if !opt.release {
                    result.push_str(&format!("{indent}# →→ {}{} ←← already inlined", modules_name, submodule));
                } else {
                    end += 1;  // remove the newline from the end of the import statement
                }
            }
            println!("processed after: {:?}", processed);
        } else {
            result.push_str(&content[start..end]);
        }

        last_end = end;
    }

    result.push_str(&content[last_end..]);
    // processed.remove(file);
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
    result.push('\n');
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::virtual_filesystem::VirtualFileSystem;

    const MAIN_PY_CONTENT: &str = r#"#!/usr/bin/env python3
from modules.module1 import func1

def main():
    from modules.module1 import func2
    print('Hello')

if __name__ == '__main__':
    main()
"#;

    const MODULE1_PY_CONTENT: &str = r#"def func1():
    print('Function 1')
"#;

    const INLINED_CONTENT: &str = r#"#!/usr/bin/env python3
# ↓↓↓ inlined module: modules.module1
def func1():
    print('Function 1')

# ↑↑↑ inlined module: modules.module1

def main():
    # →→ modules.module1 ←← already inlined
    print('Hello')

if __name__ == '__main__':
    main()
"#;

    #[test]
    fn test_inline_imports_simple() {
        let mut mock_fs = VirtualFileSystem::new();
        mock_fs.mkdir_p("/test/modules").unwrap();
        mock_fs.write("/test/main.py", MAIN_PY_CONTENT).unwrap();
        mock_fs.write("/test/modules/module1.py", MODULE1_PY_CONTENT).unwrap();

        let opt = Opt {
            input_file: PathBuf::from("/test/main.py"),
            output_file: PathBuf::from("/test/main_inlined.py"),
            modules_name: "".to_string(),
            release: false,
        };
        run(
            opt,
            &mut mock_fs,
        ).unwrap();

        let result = mock_fs.read_to_string("/test/main_inlined.py").unwrap();
        assert_eq!(result, INLINED_CONTENT);
    }

    #[test]
    fn test_post_process_imports() {
        let input = r#"#!/usr/bin/env python3
import sys
from os import path

def main():
    print('Hello')

import re

if __name__ == '__main__':
    main()
"#;

        let expected = r#"#!/usr/bin/env python3

from os import path
import re
import sys

def main():
    print('Hello')


if __name__ == '__main__':
    main()
"#;

        assert_eq!(post_process_imports(input), expected);
    }
}


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
