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

        // Check if this is a multi-line import (ends with opening parenthesis)
        let first_line = cap.get(0).unwrap().as_str();
        if first_line.trim_end().ends_with("(") {
            // Find the closing parenthesis
            let remaining = &content_to_process[end..];
            let mut paren_count = 1;  // We've seen the opening paren
            let mut chars_scanned = 0;

            for ch in remaining.chars() {
                chars_scanned += ch.len_utf8();
                if ch == '(' {
                    paren_count += 1;
                } else if ch == ')' {
                    paren_count -= 1;
                    if paren_count == 0 {
                        // Found the matching closing paren
                        end += chars_scanned;
                        // Skip past any newline immediately after the closing paren
                        if content_to_process[end..].starts_with('\n') {
                            end += 1;
                        } else if content_to_process[end..].starts_with("\r\n") {
                            end += 2;
                        }
                        break;
                    }
                }
            }
        } else {
            // Single-line import: skip past the newline after the import statement
            if content_to_process[end..].starts_with('\n') {
                end += 1;
            } else if content_to_process[end..].starts_with("\r\n") {
                end += 2;
            }
        }
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
                    // Add import context indentation to all lines of inlined content
                    for line in init_content.lines() {
                        if line.is_empty() {
                            // Preserve empty lines without indentation
                            result.push('\n');
                        } else {
                            result.push_str(indent);
                            result.push_str(line);
                            result.push('\n');
                        }
                    }
                    // Ensure trailing newline after inlined content to prevent concatenation
                    // (especially important in release mode where closing comments are omitted)
                    result.push('\n');
                    if !opt.release {
                        result.push_str(&format!("{indent}# ↑↑↑ inlined package: {}\n", submodule));
                    }
                } else {
                    if opt.verbose {
                        println!("WARNING: package {} has already been inlined. Skipping...", init_path.display());
                    }
                    if !opt.release {
                        result.push_str(&format!("{indent}# →→ {} ←← package already inlined\n", submodule));
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
                    // Add import context indentation to all lines of inlined content
                    for line in module_content.lines() {
                        if line.is_empty() {
                            // Preserve empty lines without indentation
                            result.push('\n');
                        } else {
                            result.push_str(indent);
                            result.push_str(line);
                            result.push('\n');
                        }
                    }
                    // Ensure trailing newline after inlined content to prevent concatenation
                    // (especially important in release mode where closing comments are omitted)
                    result.push('\n');
                    if !opt.release {
                        result.push_str(&format!("{indent}# ↑↑↑ inlined submodule: {}\n", submodule));
                    }
                } else {
                    if opt.verbose {
                        println!("WARNING: module {} has already been inlined. Skipping...", module_file_path.display());
                    }
                    if !opt.release {
                        result.push_str(&format!("{indent}# →→ {} ←← module already inlined\n", submodule));
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

    #[test]
    fn test_module_level_indentation_preservation() {
        // This test verifies that function-scoped imports correctly indent
        // the inlined content to match the import statement's indentation level
        let mut mock_fs = VirtualFileSystem::new();
        mock_fs.mkdir_p("/test/mylib").unwrap();

        // Module with module-level constants at indentation 0
        let environment_py = r#"import os

API_KEY = os.getenv("API_KEY") or "default-key"
ANOTHER_CONSTANT = "value"

def helper_function():
    return API_KEY
"#;
        mock_fs.write("/test/mylib/environment.py", environment_py).unwrap();

        // Main file that imports from an indented context (inside a function)
        let main_py = r#"def my_function():
    from mylib.environment import API_KEY
    return API_KEY

if __name__ == '__main__':
    print(my_function())
"#;
        mock_fs.write("/test/main.py", main_py).unwrap();

        let input_file = PathBuf::from("/test/main.py");
        let output_file = PathBuf::from("/test/main_inlined.py");
        let module_names = "mylib".to_string();
        let release = false;
        let verbose = false;

        let mut python_sys_path = Vec::new();
        python_sys_path.push(PathBuf::from("/test"));

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

        // The expected output should have inlined content indented to match
        // the import statement's indentation level (4 spaces in this case)
        let expected = r#"def my_function():
    # ↓↓↓ inlined submodule: mylib.environment
    import os

    API_KEY = os.getenv("API_KEY") or "default-key"
    ANOTHER_CONSTANT = "value"

    def helper_function():
        return API_KEY

    # ↑↑↑ inlined submodule: mylib.environment
    return API_KEY

if __name__ == '__main__':
    print(my_function())
"#;

        assert_eq!(result, expected, "\n\nExpected:\n{}\n\nGot:\n{}\n", expected, result);
    }

    #[test]
    fn test_multiline_import_removal() {
        // This test reproduces the bug where multi-line import statements
        // are not completely removed, leaving dangling import names
        let mut mock_fs = VirtualFileSystem::new();
        mock_fs.mkdir_p("/test/mylib").unwrap();

        // Module with some constants
        let environment_py = r#"import os

API_KEY = os.getenv("API_KEY") or "default-key"
ANOTHER_KEY = os.getenv("ANOTHER") or "other"
THIRD_KEY = "third"
"#;
        mock_fs.write("/test/mylib/environment.py", environment_py).unwrap();

        // Main file with multi-line import statement
        let main_py = r#"from mylib.environment import (
    API_KEY,
    ANOTHER_KEY,
    THIRD_KEY,
)

def my_function():
    return API_KEY

if __name__ == '__main__':
    print(my_function())
"#;
        mock_fs.write("/test/main.py", main_py).unwrap();

        let input_file = PathBuf::from("/test/main.py");
        let output_file = PathBuf::from("/test/main_inlined.py");
        let module_names = "mylib".to_string();
        let release = false;
        let verbose = false;

        let mut python_sys_path = Vec::new();
        python_sys_path.push(PathBuf::from("/test"));

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

        // The expected output should have the entire multi-line import replaced,
        // with NO dangling import names or parentheses
        let expected = r#"# ↓↓↓ inlined submodule: mylib.environment
import os

API_KEY = os.getenv("API_KEY") or "default-key"
ANOTHER_KEY = os.getenv("ANOTHER") or "other"
THIRD_KEY = "third"

# ↑↑↑ inlined submodule: mylib.environment

def my_function():
    return API_KEY

if __name__ == '__main__':
    print(my_function())
"#;

        assert_eq!(result, expected, "\n\nExpected:\n{}\n\nGot:\n{}\n", expected, result);
    }

    #[test]
    fn test_function_scoped_import_indentation() {
        // This test reproduces the bug where imports inside function bodies
        // cause inlined content to be at wrong indentation level (0 instead of function indent)
        let mut mock_fs = VirtualFileSystem::new();
        mock_fs.mkdir_p("/test/mylib").unwrap();

        // Module with module-level code (indentation 0 in source file)
        let llm_response_py = r#"from dataclasses import dataclass

@dataclass
class LLMResponse:
    """Response from LLM API."""
    content: str
    model: str

    def from_api_response(self, api_data):
        return LLMResponse(
            content=api_data.get("content", ""),
            model=api_data.get("model", "unknown")
        )
"#;
        mock_fs.write("/test/mylib/llm_response.py", llm_response_py).unwrap();

        // Main file with function-scoped imports (indented inside function body)
        let main_py = r#"def call_llm_light(prompt: str, temperature: float = 0.0):
    """Call LLM using light provider config."""
    from mylib.llm_response import LLMResponse

    payload = {
        "model": "test-model",
        "messages": [{"role": "user", "content": prompt}]
    }

    # Simulated API response
    api_data = {"content": "Hello, world!", "model": "test-model"}
    return LLMResponse.from_api_response(api_data)

if __name__ == '__main__':
    result = call_llm_light("Hello!")
    print(result)
"#;
        mock_fs.write("/test/main.py", main_py).unwrap();

        let input_file = PathBuf::from("/test/main.py");
        let output_file = PathBuf::from("/test/main_inlined.py");
        let module_names = "mylib".to_string();
        let release = false;
        let verbose = false;

        let mut python_sys_path = Vec::new();
        python_sys_path.push(PathBuf::from("/test"));

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

        // The expected output should have inlined content indented at the same level
        // as the import statement (4 spaces), NOT at module level (0 spaces)
        let expected = r#"def call_llm_light(prompt: str, temperature: float = 0.0):
    """Call LLM using light provider config."""
    # ↓↓↓ inlined submodule: mylib.llm_response
    from dataclasses import dataclass

    @dataclass
    class LLMResponse:
        """Response from LLM API."""
        content: str
        model: str

        def from_api_response(self, api_data):
            return LLMResponse(
                content=api_data.get("content", ""),
                model=api_data.get("model", "unknown")
            )

    # ↑↑↑ inlined submodule: mylib.llm_response

    payload = {
        "model": "test-model",
        "messages": [{"role": "user", "content": prompt}]
    }

    # Simulated API response
    api_data = {"content": "Hello, world!", "model": "test-model"}
    return LLMResponse.from_api_response(api_data)

if __name__ == '__main__':
    result = call_llm_light("Hello!")
    print(result)
"#;

        assert_eq!(result, expected, "\n\nExpected:\n{}\n\nGot:\n{}\n", expected, result);
    }

    #[test]
    #[ignore] // TODO: Implement __all__ statement filtering for inlined content
    fn test___all___statement_removal() {
        // This test reproduces the bug where __all__ statements from modules/packages
        // are inlined into functions, causing invalid Python syntax
        let mut mock_fs = VirtualFileSystem::new();
        mock_fs.mkdir_p("/test/mylib").unwrap();

        // Package __init__.py with __all__ statement
        let init_py = r#"""My library package."""

from .utils import helper_function

__all__ = ["helper_function"]
"#;
        mock_fs.write("/test/mylib/__init__.py", init_py).unwrap();

        // Utils module
        let utils_py = r#"def helper_function():
    """Helper function."""
    return "Hello, world!"
"#;
        mock_fs.write("/test/mylib/utils.py", utils_py).unwrap();

        // Main file with function-scoped import
        let main_py = r#"def process_data():
    """Process data using mylib."""
    from mylib import helper_function

    result = helper_function()
    return result.upper()

if __name__ == '__main__':
    print(process_data())
"#;
        mock_fs.write("/test/main.py", main_py).unwrap();

        let input_file = PathBuf::from("/test/main.py");
        let output_file = PathBuf::from("/test/main_inlined.py");
        let module_names = "mylib".to_string();
        let release = false;
        let verbose = false;

        let mut python_sys_path = Vec::new();
        python_sys_path.push(PathBuf::from("/test"));

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

        // The expected output should NOT include the __all__ statement
        // from mylib/__init__.py, as it's only meaningful at module level
        let expected = r#"def process_data():
    """Process data using mylib."""
    # ↓↓↓ inlined package: mylib
    """My library package."""

    # ↓↓↓ inlined submodule: .utils
    def helper_function():
        """Helper function."""
        return "Hello, world!"

    # ↑↑↑ inlined submodule: .utils

    # ↑↑↑ inlined package: mylib

    result = helper_function()
    return result.upper()

if __name__ == '__main__':
    print(process_data())
"#;

        assert_eq!(result, expected, "\n\nExpected:\n{}\n\nGot:\n{}\n", expected, result);
    }
}
