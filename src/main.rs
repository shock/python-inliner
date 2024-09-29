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
mod utils {
    pub mod python;
}

use modules::file_system::RealFileSystem;
use modules::file_system::FileSystem;
use utils::python::get_python_sys_path;

#[derive(StructOpt, Debug)]
#[structopt(name = "python-inliner", about = "Python File Inliner - https://github.com/shock/python-inliner")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,

    #[structopt(parse(from_os_str))]
    output_file: PathBuf,

    #[structopt(help = "comma-separated list module names to be inlined", default_value = "")]
    module_names: String,

    #[structopt(long, short = "r", help = "Suppress comments in the output, and consolidate imports", takes_value = false)]
    release: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let python_sys_path = get_python_sys_path()?;
    // map the python_sys_path to a vector of Path objects
    let python_sys_path: Vec<PathBuf> = python_sys_path.into_iter().map(|p| PathBuf::from(p)).collect();

    // get current working directory
    let current_dir = fs::canonicalize(".")?;
    let mut fs = RealFileSystem::new(current_dir);

    // filter out the non-directories from python_sys_path using the fs.is_dir() method
    let mut python_sys_path = python_sys_path.into_iter().filter(|p|
        match fs.is_dir(p) {
            Ok(true) => true,
            _ => false
        }
    ).collect::<Vec<PathBuf>>();
    handle_editable_installs(&mut fs, &mut python_sys_path)?;
    // if the environment flag is set, print the PYTHONPATH and exit
    if !opt.release {
        println!("PYTHONPATH: {:?}", python_sys_path);
        // return Ok(())
    }
    run(opt, &mut fs, &python_sys_path)
}

fn run<FS: FileSystem>(opt: Opt, fs: &mut FS, python_sys_path: &Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    // get the input_file as a fully qualified path
    let input_file = fs.canonicalize(&opt.input_file)?;

    // get the working directory from the input file path
    let working_dir = input_file.parent().unwrap();
    let mut python_sys_path = python_sys_path.clone();
    python_sys_path.insert(0, working_dir.to_path_buf());

    // split the module names into a vector
    let mut module_names: Vec<String> = opt.module_names.split(",").map(|s| s.trim().to_string()).collect::<Vec<String>>();
    // insert a '.' at the beginning of the module names to match the current script's directory
    module_names.insert(0, ".".to_string());

    // rejoin the module names into a single string using a pipe character for the regex group
    let module_names = module_names.join("|");

    let mut content = inline_imports(fs, &python_sys_path, &opt.input_file, &module_names, &mut HashSet::new(), &opt)?;
    if opt.release {
        content = post_process_imports(&content);
    }
    fs.write(&opt.output_file, content)?;
    println!("Inlined content written to {:?}", opt.output_file);
    Ok(())
}

use serde_json::Value;

fn handle_editable_installs<FS: FileSystem>(fs: &mut FS, python_sys_path: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    let site_packages_paths: Vec<PathBuf> = python_sys_path
        .iter()
        .filter(|path| path.to_string_lossy().contains("site-packages"))
        .cloned()
        .collect();

    for path in site_packages_paths {
        println!("path: {:?}", path);
        if fs.is_dir(&path)? {
            println!("is_dir");
            for entry in fs.read_dir(&path)? {
                let entry_path = entry;
                if entry_path.is_dir() && entry_path.file_name().unwrap().to_string_lossy().ends_with(".dist-info") {
                    let direct_url_path = entry_path.join("direct_url.json");
                    if fs.exists(&direct_url_path)? {
                        let content = fs.read_to_string(&direct_url_path)?;
                        let json: Value = serde_json::from_str(&content)?;

                        if let Some(url) = json.get("url").and_then(Value::as_str) {
                            if let Some(dir_info) = json.get("dir_info") {
                                if let Some(true) = dir_info.get("editable").and_then(Value::as_bool) {
                                    if url.starts_with("file://") {
                                        let package_path = PathBuf::from(url.trim_start_matches("file://"));
                                        if fs.is_dir(&package_path)? && !python_sys_path.contains(&package_path) {
                                            python_sys_path.push(package_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn inline_imports<FS: FileSystem>(fs: &mut FS, python_sys_path: &Vec<PathBuf>, file: &Path, module_names: &str, processed: &mut HashSet<PathBuf>, opt: &Opt) -> Result<String, Box<dyn Error>> {
    let content = fs.read_to_string(file)?;
    let import_regex = Regex::new(&format!(r"(?m)^([ \t]*)from\s+((?:{})\S*)\s+import\s+(.+)$", module_names))?;
    let parent_dir = file.parent().unwrap();
    let mut result = String::new();
    let mut last_end = 0;
    let captures = import_regex.captures_iter(&content);
    for cap in captures {
        let indent = &cap[1];
        let submodule = &cap[2];
        #[allow(unused)]
        let imports = &cap[3];  // TODO: handle specific imports?  non-trivial
        let start = cap.get(0).unwrap().start();
        let mut end = cap.get(0).unwrap().end();
        result.push_str(&content[last_end..start]);

        let mut module_paths = Vec::new();
        if submodule.starts_with(".") {
            let module_path = parent_dir.join(submodule.trim_start_matches('.').replace(".", "/"));
            module_paths.push(module_path);
        } else {
            for path in python_sys_path {
                let module_path = path.join(submodule.replace(".", "/"));
                module_paths.push(module_path);
            }
        }

        let mut found = false;
        for module_path in module_paths {
            let init_path = module_path.join("__init__.py");
            let module_file_path = module_path.with_extension("py");

            if fs.exists(&init_path).unwrap() {
                // It's a package, process __init__.py
                found = true;
                if processed.insert(init_path.to_path_buf()) {
                    let init_content = inline_imports(fs, python_sys_path, &init_path, module_names, processed, opt)?;
                    if !opt.release {
                        result.push_str(&format!("{indent}# ↓↓↓ inlined package: {}\n", submodule));
                    }
                    result.push_str(&indent);
                    result.push_str(&init_content.replace("\n", &format!("\n{indent}")));
                    if !opt.release {
                        result.push_str(&format!("\n{indent}# ↑↑↑ inlined package: {}\n", submodule));
                    }
                } else {
                    println!("WARNING: package {} has already been inlined. Skipping...", init_path.display());
                    if !opt.release {
                        result.push_str(&format!("{indent}# →→ {} ←← package already inlined\n", submodule));
                    } else {
                        end += 1;  // remove the newline from the end of the import statement
                    }
                }
            } else if fs.exists(&module_file_path).unwrap() {
                // It's a module file
                found = true;
                if processed.insert(module_file_path.to_path_buf()) {
                    let module_content = inline_imports(fs, python_sys_path, &module_file_path, module_names, processed, opt)?;
                    if !opt.release {
                        result.push_str(&format!("{indent}# ↓↓↓ inlined submodule: {}\n", submodule));
                    }
                    result.push_str(&indent);
                    result.push_str(&module_content.replace("\n", &format!("\n{indent}")));
                    if !opt.release {
                        result.push_str(&format!("\n{indent}# ↑↑↑ inlined submodule: {}", submodule));
                    }
                } else {
                    println!("WARNING: module {} has already been inlined. Skipping...", module_file_path.display());
                    if !opt.release {
                        result.push_str(&format!("{indent}# →→ {} ←← module already inlined", submodule));
                    } else {
                        end += 1;  // remove the newline from the end of the import statement
                    }
                }
            }
            if found {
                break;
            }
        }
        if !found {
            println!("Could not find module {:?}", submodule);
            result.push_str(&content[start..end]);
        }
        last_end = end;
    }

    result.push_str(&content[last_end..]);
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
# ↓↓↓ inlined submodule: modules.module1
def func1():
    print('Function 1')

# ↑↑↑ inlined submodule: modules.module1

def main():
    # →→ modules.module1 ←← module already inlined
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
            module_names: "".to_string(),
            release: false,
        };
        let mut python_sys_path = Vec::new();
        python_sys_path.push(PathBuf::from("/test/modules"));
        run(
            opt,
            &mut mock_fs,
            &python_sys_path,
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
