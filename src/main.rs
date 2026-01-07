use std::collections::HashSet;
use std::fs as fs;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
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
#[structopt(name = "python-inliner", about = "Python File Inliner - https://github.com/shock/python-inliner", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Opt {
    #[structopt(parse(from_os_str))]
    input_file: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    output_file: Option<PathBuf>,

    #[structopt(help = "comma-separated list module names to be inlined", default_value = "")]
    module_names: String,

    #[structopt(long, short = "r", help = "Suppress comments in the output, and consolidate imports", takes_value = false)]
    release: bool,

    #[structopt(long, short = "v", help = "Print verbose debug information", takes_value = false)]
    verbose: bool,

    #[structopt(long, help = "Print version information and exit", takes_value = false)]
    version: bool,
}

fn get_current_year() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / (60 * 60 * 24 * 365) + 1970)
        .unwrap_or(2025)
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    if opt.version {
        let current_year = get_current_year();
        println!("python-inliner v{}", env!("CARGO_PKG_VERSION"));
        println!("Author: {}", env!("CARGO_PKG_AUTHORS"));
        println!("Copyright (c) {}", current_year);
        return Ok(());
    }

    // Check if required arguments are provided
    let input_file = opt.input_file.ok_or("Input file is required")?;
    let output_file = opt.output_file.ok_or("Output file is required")?;

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
    if opt.verbose {
        println!("PYTHONPATH: {:?}\n", python_sys_path);
    }
    run(input_file, output_file, opt.module_names, opt.release, opt.verbose, &mut fs, &python_sys_path)
}

fn run<FS: FileSystem>(input_file: PathBuf, output_file: PathBuf, module_names: String, release: bool, verbose: bool, fs: &mut FS, python_sys_path: &Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    // get the input_file as a fully qualified path
    let input_file = fs.canonicalize(&input_file)?;

    // get the working directory from the input file path
    let working_dir = input_file.parent().unwrap();
    let mut python_sys_path = python_sys_path.clone();
    python_sys_path.insert(0, working_dir.to_path_buf());

    // split the module names into a vector and filter out empty strings
    let mut module_names: Vec<String> = module_names.split(",").filter(|s| !s.is_empty()).map(|s| s.trim().to_string()).collect::<Vec<String>>();
    // insert a '.' at the beginning of the module names to match the current script's directory
    module_names.insert(0, "\\.".to_string());

    // rejoin the module names into a single string using a pipe character for the regex group
    let module_names = module_names.join("|");

    let opt = Opt {
        input_file: Some(input_file.clone()),
        output_file: Some(output_file.clone()),
        module_names: module_names.clone(),
        release,
        verbose,
        version: false,
    };

    let mut content = inline_imports(fs, &python_sys_path, &input_file, &module_names, &mut HashSet::new(), &opt)?;
    if release {
        content = post_process_imports(&content);
    }
    fs.write(&output_file, content)?;
    println!("Inlined content written to {:?}", output_file);
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
        // println!("path: {:?}", path);
        if fs.is_dir(&path)? {
            // println!("is_dir");
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

/// Find all TYPE_CHECKING block ranges in the content
/// Returns a vector of (start_pos, end_pos) tuples for each TYPE_CHECKING block
fn find_type_checking_blocks(content: &str) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let type_checking_regex = Regex::new(r"(?m)^([ \t]*)if\s+TYPE_CHECKING\s*:").unwrap();

    for cap in type_checking_regex.captures_iter(content) {
        let block_start = cap.get(0).unwrap().start();
        let indent = &cap[1];
        let indent_len = indent.len();

        // Find the end of this indented block
        // The block ends when we find a line with equal or lesser indentation (non-empty)
        let after_colon = cap.get(0).unwrap().end();
        let lines_after = &content[after_colon..];

        let mut block_end = after_colon;
        let mut found_content = false;

        for line in lines_after.lines() {
            let line_start = block_end;
            let line_len = line.len();

            // Skip empty lines (they're part of the block)
            if line.trim().is_empty() {
                block_end = line_start + line_len + 1; // +1 for newline
                continue;
            }

            // Check indentation of non-empty line
            let line_indent = line.len() - line.trim_start().len();

            if !found_content {
                // First non-empty line after if TYPE_CHECKING:
                if line_indent > indent_len {
                    found_content = true;
                    block_end = line_start + line_len + 1;
                } else {
                    // No indented content found, block is empty
                    break;
                }
            } else {
                // Subsequent lines
                if line_indent > indent_len {
                    // Still inside the block
                    block_end = line_start + line_len + 1;
                } else {
                    // End of block (dedent)
                    break;
                }
            }
        }

        blocks.push((block_start, block_end));
    }

    blocks
}

fn inline_imports<FS: FileSystem>(fs: &mut FS, python_sys_path: &Vec<PathBuf>, file: &Path, module_names: &str, processed: &mut HashSet<PathBuf>, opt: &Opt) -> Result<String, Box<dyn Error>> {
    let content = fs.read_to_string(file)?;

    // Find all TYPE_CHECKING blocks and strip them from the content
    // TYPE_CHECKING is always False at runtime, so these blocks are only for static type checkers
    let type_checking_blocks = find_type_checking_blocks(&content);

    let import_regex = Regex::new(&format!(r"(?m)^([ \t]*)from\s+((?:{})\S*)\s+import\s+(.+)$", module_names))?;
    // if opt.verbose {
    //     println!("Import regex: {}", import_regex);
    // }
    let parent_dir = file.parent().unwrap();
    let mut result = String::new();

    // First, skip over any TYPE_CHECKING blocks when copying content
    let mut current_pos = 0;
    for (block_start, block_end) in &type_checking_blocks {
        // Copy content before this TYPE_CHECKING block
        if current_pos < *block_start {
            result.push_str(&content[current_pos..*block_start]);
        }
        // Skip the TYPE_CHECKING block entirely (don't copy it)
        if opt.verbose {
            let block_content = &content[*block_start..*block_end];
            println!("Stripping TYPE_CHECKING block:\n{}", block_content.lines().take(3).collect::<Vec<_>>().join("\n"));
        }
        current_pos = *block_end;
    }
    // Copy any remaining content after the last TYPE_CHECKING block
    let content_after_blocks = if current_pos < content.len() {
        content[current_pos..].to_string()
    } else {
        String::new()
    };

    // Now process imports in the content (excluding TYPE_CHECKING blocks)
    let content_to_process = result.clone() + &content_after_blocks;
    result.clear();
    let mut last_end = 0;

    let captures = import_regex.captures_iter(&content_to_process);
    for cap in captures {
        // if opt.verbose {
        //     println!("Capture: {:?}", cap);
        // }
        let indent = &cap[1];
        let submodule = &cap[2];
        #[allow(unused)]
        let imports = &cap[3];  // TODO: handle specific imports?  non-trivial
        let start = cap.get(0).unwrap().start();
        let mut end = cap.get(0).unwrap().end();
        result.push_str(&content_to_process[last_end..start]);

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
        // if opt.verbose {
        //     println!("Module paths: {:?}", module_paths);
        // }
        let mut found = false;
        for module_path in module_paths {
            let init_path = module_path.join("__init__.py");
            let module_file_path = module_path.with_extension("py");

            if fs.exists(&init_path).unwrap() {
                // It's a package, process __init__.py
                found = true;
                if processed.insert(init_path.to_path_buf()) {
                    if opt.verbose {
                        println!("Inlining package {}", init_path.display());
                    }
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
                    if opt.verbose {
                        println!("WARNING: package {} has already been inlined. Skipping...", init_path.display());
                    }
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
                    if opt.verbose {
                        println!("Inlining module {}", module_file_path.display());
                    }
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
                    if opt.verbose {
                        println!("WARNING: module {} has already been inlined. Skipping...", module_file_path.display());
                    }
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
            if opt.verbose {
                println!("Could not find module {:?}", submodule);
            }
            result.push_str(&content_to_process[start..end]);
        }
        last_end = end;
    }

    result.push_str(&content_to_process[last_end..]);
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

        let input_file = PathBuf::from("/test/main.py");
        let output_file = PathBuf::from("/test/main_inlined.py");
        let module_names = "modules".to_string();
        let release = false;
        let verbose = false;

        let mut python_sys_path = Vec::new();
        python_sys_path.push(PathBuf::from("/test/modules"));
        run(
            input_file,
            output_file,
            module_names,
            release,
            verbose,
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
