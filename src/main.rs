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
        content = strip_docstrings(&content);
        content = strip_comments(&content);
        content = strip_blank_lines(&content);
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

    // Improved regex that validates actual import statements:
    // - "from module.name import something" - requires valid module name and 'import' keyword
    // - "import module.name" - requires valid module name after import
    // Module names must start with letter/underscore and contain word chars, dots, and underscores
    let import_regex = Regex::new(
        r"^\s*(?:from\s+[a-zA-Z_][\w.]*\s+import\s+|import\s+[a-zA-Z_][\w.,\s*]+)"
    ).unwrap();

    // Filter out JavaScript-style imports (import X from '...'), which Python never uses
    let js_import_filter = Regex::new(
        "^\\s*import\\s+[\\w.*]+\\s+from\\s+['\"]"
    ).unwrap();

    let shebang_regex = Regex::new(r"^#!").unwrap();
    let pep723_start_regex = Regex::new(r"^#\s*///").unwrap();

    let mut lines = content.lines().collect::<Vec<&str>>();

    if let Some(first_line) = lines.first() {
        if shebang_regex.is_match(first_line) {
            header_content.push(first_line.to_string());
            header_content.push("\n".to_string());
            lines.remove(0);
        }
    }

    // Check for and extract PEP 723 inline script metadata block
    if !lines.is_empty() {
        let first_line_after_shebang = lines[0].trim_start();
        if pep723_start_regex.is_match(first_line_after_shebang) {
            // Found PEP 723 start marker
            let mut idx = 0;

            while idx < lines.len() {
                let line = lines[idx];
                let trimmed = line.trim_start();

                if pep723_start_regex.is_match(trimmed) {
                    // Check if this is the end marker (just "# ///" or "#///" with nothing after)
                    let is_end_marker = trimmed == "# ///" || trimmed == "#///";
                    if is_end_marker && !header_content.is_empty() {
                        // End of PEP 723 block
                        header_content.push(line.to_string());
                        idx += 1;
                        break;
                    }
                }

                header_content.push(line.to_string());
                idx += 1;
            }

            // Remove the PEP 723 block from the remaining lines
            lines = lines[idx..].to_vec();
        }
    }

    for line in lines {
        if import_regex.is_match(line) && !js_import_filter.is_match(line) {
            imports.insert(line.trim_start().to_string());
        } else {
            other_content.push(line.to_string());
        }
    }

    let mut result = String::new();
    result.push_str(&header_content.join("\n"));
    let mut imports_vec: Vec<String> = imports.into_iter().collect();
    imports_vec.sort();

    // Check if header contains a PEP 723 block (looks for "# ///" marker)
    let has_pep723 = header_content.iter().any(|line| line.contains("# ///"));

    if !imports_vec.is_empty() {
        // Add extra blank line after header if it contains PEP 723 block
        if has_pep723 {
            result.push('\n');
        }
        result.push_str(&imports_vec.join("\n"));
        result.push('\n');
    } else if has_pep723 {
        // No imports but PEP 723 block exists - add blank line after it
        result.push('\n');
    }

    result.push_str(&other_content.join("\n"));
    result.push('\n');
    result
}

/// Strip docstrings from Python code.
/// Removes function and class docstrings (triple-quoted strings that are NOT assigned to variables).
/// Preserves variable assignments that use triple-quoted strings.
fn strip_docstrings(content: &str) -> String {
    // Patterns to check what comes before a triple-quoted string
    // Assignment pattern now handles: var=, self.attr=, obj.attr.nested=, etc.
    let assignment_pattern = Regex::new(r"^\s*[a-zA-Z_]\w*(\.[a-zA-Z_]\w*)*\s*=").unwrap();
    let import_pattern = Regex::new(r"^\s*(from|import)\s+").unwrap();
    let decorator_pattern = Regex::new(r"^\s*@").unwrap();

    let mut result = String::new();
    let mut last_pos = 0;
    let bytes = content.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        // Check for triple-quoted strings (""" or ''')
        if pos + 2 < bytes.len() {
            let is_triple_double = bytes[pos] == b'"' && bytes[pos + 1] == b'"' && bytes[pos + 2] == b'"';
            let is_triple_single = bytes[pos] == b'\'' && bytes[pos + 1] == b'\'' && bytes[pos + 2] == b'\'';

            if is_triple_double || is_triple_single {
                let quote_byte = bytes[pos];
                let start_pos = pos;

                // Make sure this is exactly 3 quotes, not 4+
                if pos + 3 < bytes.len() && bytes[pos + 3] == quote_byte {
                    // This is 4+ quotes, skip the first one and continue
                    pos += 1;
                    continue;
                }

                // Find the closing triple quote
                let mut end_pos = pos + 3;
                let mut found_closing = false;

                while end_pos + 2 < bytes.len() {
                    if bytes[end_pos] == quote_byte && bytes[end_pos + 1] == quote_byte && bytes[end_pos + 2] == quote_byte {
                        // Make sure it's exactly 3 quotes, not part of 4+
                        let has_fourth = end_pos + 3 < bytes.len() && bytes[end_pos + 3] == quote_byte;
                        if !has_fourth {
                            end_pos += 3;
                            found_closing = true;
                            break;
                        }
                    }
                    end_pos += 1;
                }

                if !found_closing {
                    // No closing quote found, treat as regular content
                    pos += 1;
                    continue;
                }

                // Check if this should be preserved
                let before_string = &content[..start_pos];
                let line_start = before_string.rfind('\n').map(|p| p + 1).unwrap_or(0);
                let line_before = &content[line_start..start_pos];

                let trimmed = line_before.trim_end();
                let is_f_string = trimmed.ends_with('f');

                let should_preserve = assignment_pattern.is_match(line_before)
                    || import_pattern.is_match(line_before)
                    || decorator_pattern.is_match(line_before)
                    || is_f_string;

                // Copy everything from last position to start of this string
                result.push_str(&content[last_pos..start_pos]);

                if should_preserve {
                    // Keep the triple-quoted string
                    result.push_str(&content[start_pos..end_pos]);
                }
                // else: skip it (it's a docstring) - just don't add it to result

                last_pos = end_pos;
                pos = end_pos;
                continue;
            }
        }

        pos += 1;
    }

    // Copy any remaining content
    result.push_str(&content[last_pos..]);

    result
}

fn strip_comments(content: &str) -> String {
    let shebang_regex = Regex::new(r"^#!").unwrap();
    let pep723_start_regex = Regex::new(r"^#\s*///").unwrap(); // Match # /// with optional text after

    let mut result = String::new();
    let mut lines = content.lines().enumerate().peekable();
    let mut in_multiline_string = None::<char>; // Track if we're inside a multi-line triple-quoted string
    let mut in_pep723_block = false; // Track if we're inside a PEP 723 metadata block

    while let Some((line_num, line)) = lines.next() {
        let trimmed = line.trim_start();

        // Preserve shebang line (only on first line)
        if line_num == 0 && shebang_regex.is_match(trimmed) {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Handle PEP 723 inline script metadata blocks
        if pep723_start_regex.is_match(trimmed) {
            // Check if this is the end marker (just "# ///" with nothing after, or only whitespace)
            let is_end_marker = trimmed == "# ///" || trimmed == "#///";
            if in_pep723_block && is_end_marker {
                // End of PEP 723 block
                in_pep723_block = false;
                result.push_str(line);
                if lines.peek().is_some() {
                    result.push('\n');
                }
                continue;
            } else if !in_pep723_block {
                // Start of PEP 723 block
                in_pep723_block = true;
                result.push_str(line);
                if lines.peek().is_some() {
                    result.push('\n');
                }
                continue;
            }
        }

        // Preserve all lines inside PEP 723 block (including comments)
        if in_pep723_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Find inline comment position (not inside strings)
        let mut in_string = in_multiline_string; // Start with multi-line state
        let mut chars = line.chars().peekable();
        let mut comment_pos = None;
        let mut i = 0;

        while let Some(&ch) = chars.peek() {
            let pos = i;
            i += ch.len_utf8();
            chars.next();

            // Check for triple quotes
            if ch == '"' || ch == '\'' {
                if let Some(&next1) = chars.peek() {
                    if next1 == ch {
                        chars.next();
                        if let Some(&next2) = chars.peek() {
                            if next2 == ch {
                                chars.next();
                                // Triple quote
                                if in_string == Some(ch) {
                                    in_string = None;
                                    in_multiline_string = None;
                                } else if in_string.is_none() {
                                    in_string = Some(ch);
                                    in_multiline_string = Some(ch);
                                }
                                continue;
                            }
                        }
                    }
                }

                // Single/double quote (only if not in multi-line string)
                if in_multiline_string.is_none() {
                    if in_string.is_none() {
                        in_string = Some(ch);
                    } else if in_string == Some(ch) {
                        in_string = None;
                    }
                }
            } else if ch == '#' && in_string.is_none() {
                // Found a comment outside a string
                comment_pos = Some(pos);
                break;
            }
        }

        // Add the line up to the comment (or whole line if no comment)
        // Skip whole-line comments (if comment starts at position 0 or only whitespace)
        if let Some(pos) = comment_pos {
            let before_comment = &line[..pos];
            if before_comment.trim().is_empty() {
                // This is a whole-line comment, skip it
            } else {
                // Inline comment, keep the part before it
                let trimmed_content = before_comment.trim_end();
                if !trimmed_content.is_empty() {
                    result.push_str(trimmed_content);
                    if lines.peek().is_some() {
                        result.push('\n');
                    }
                }
            }
        } else {
            if !line.trim().is_empty() {
                result.push_str(line);
                if lines.peek().is_some() {
                    result.push('\n');
                }
            }
        }
    }

    // Preserve final newline if original content ended with one
    if content.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Strip all blank lines from Python code.
/// Removes both single blank lines and multiple consecutive blank lines.
fn strip_blank_lines(content: &str) -> String {
    let mut result = String::new();
    let mut lines = content.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        result.push_str(line);

        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve final newline if original content ended with one
    if content.ends_with('\n') {
        result.push('\n');
    }

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
    fn test_javascript_import_filtering() {
        // This test verifies that JavaScript-style imports embedded in Python code
        // are not mistakenly detected as Python imports
        let input = r#"#!/usr/bin/env python3
import os
from sys import path

def generate_html(is_markdown):
    mermaid_script = ""
    if is_markdown:
        mermaid_script = """
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({ startOnLoad: true, theme: 'dark' });
    </script>"""
    return f"<html>{mermaid_script}</html>"

def main():
    import re

if __name__ == '__main__':
    main()
"#;

        let expected = r#"#!/usr/bin/env python3

from sys import path
import os
import re

def generate_html(is_markdown):
    mermaid_script = ""
    if is_markdown:
        mermaid_script = """
    <script type="module">
        import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.esm.min.mjs';
        mermaid.initialize({ startOnLoad: true, theme: 'dark' });
    </script>"""
    return f"<html>{mermaid_script}</html>"

def main():

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

    #[test]
    fn test_strip_docstrings_simple() {
        // Test basic function and class docstrings
        let input = r##""""Module docstring."""

def func():
    """Function docstring."""
    pass

class MyClass:
    """Class docstring."""
    pass
"##;

        // Note: strip_docstrings leaves blank lines (including indented ones) behind - that's OK!
        // strip_blank_lines() will clean them up in the full release mode flow
        let expected = "\n\ndef func():\n    \n    pass\n\nclass MyClass:\n    \n    pass\n";

        assert_eq!(strip_docstrings(input), expected);
    }

    #[test]
    fn test_strip_docstrings_preserves_variable_assignment() {
        // Test that variable assignments with triple quotes are preserved
        let input = r##""""Module docstring."""

MY_VAR = """This is assigned to a variable and should be preserved."""

def func():
    """Function docstring."""
    pass
"##;

        let expected = "\n\nMY_VAR = \"\"\"This is assigned to a variable and should be preserved.\"\"\"\n\ndef func():\n    \n    pass\n";

        assert_eq!(strip_docstrings(input), expected);
    }

    #[test]
    fn test_strip_docstrings_f_string_preserved() {
        // Test that f-strings with triple quotes are preserved
        let input = r##""""Module docstring."""

def func():
    """Function docstring."""
    some_var = f"""long
string {self.name} with interpolation
"""
    pass
"##;

        let expected = "\n\ndef func():\n    \n    some_var = f\"\"\"long\nstring {self.name} with interpolation\n\"\"\"\n    pass\n";

        assert_eq!(strip_docstrings(input), expected);
    }

    #[test]
    fn test_strip_docstrings_single_quotes() {
        // Test that single triple quotes are also removed as docstrings
        let input = r##""""Module docstring."""

def func():
    '''Function docstring with single quotes.'''
    pass

class MyClass:
    '''Class docstring with single quotes.'''
    pass
"##;

        let expected = "\n\ndef func():\n    \n    pass\n\nclass MyClass:\n    \n    pass\n";

        assert_eq!(strip_docstrings(input), expected);
    }

    #[test]
    fn test_strip_docstrings_preserves_attribute_assignment() {
        // Test that attribute assignments (self.attr, obj.attr) with triple quotes are preserved
        let input = r##""""Module docstring."""

class MyClass:
    def __init__(self):
        """Init docstring."""
        self.template = """
        This should be preserved.
        """
        pass
"##;

        let expected = "\n\nclass MyClass:\n    def __init__(self):\n        \n        self.template = \"\"\"\n        This should be preserved.\n        \"\"\"\n        pass\n";

        assert_eq!(strip_docstrings(input), expected);
    }

    #[test]
    fn test_strip_docstrings_no_docstrings() {
        // Test code without docstrings
        let input = r#"def func():
    pass

class MyClass:
    pass
"#;

        assert_eq!(strip_docstrings(input), input);
    }

    #[test]
    fn test_strip_comments_whole_line() {
        // Test removing whole-line comments
        let input = r#"#!/usr/bin/env python3
# This is a comment
import sys

# Another comment
def main():
    pass
"#;

        let expected = r#"#!/usr/bin/env python3
import sys
def main():
    pass
"#;

        assert_eq!(strip_comments(input), expected);
    }

    #[test]
    fn test_strip_comments_inline() {
        // Test removing inline comments
        let input = r#"#!/usr/bin/env python3
import sys  # This is an inline comment

def main():
    pass  # Another inline comment
"#;

        let expected = r#"#!/usr/bin/env python3
import sys
def main():
    pass
"#;

        assert_eq!(strip_comments(input), expected);
    }

    #[test]
    fn test_strip_comments_preserves_strings_with_hash() {
        // Test that comments inside strings are preserved
        let input = r#"def func():
    s = "This # is not a comment"
    s2 = 'This # is also not a comment'
    pass
"#;

        let expected = r#"def func():
    s = "This # is not a comment"
    s2 = 'This # is also not a comment'
    pass
"#;

        assert_eq!(strip_comments(input), expected);
    }

    #[test]
    fn test_strip_comments_preserves_triple_quoted_strings() {
        // Test that triple-quoted strings with # are preserved
        let input = r#"MY_VAR = """
This string contains # symbols that are not comments.
They should be preserved.
"""
"#;

        // # symbols inside triple-quoted strings should be preserved
        let expected = r#"MY_VAR = """
This string contains # symbols that are not comments.
They should be preserved.
"""
"#;

        assert_eq!(strip_comments(input), expected);
    }

    #[test]
    fn test_strip_comments_no_comments() {
        // Test code without comments
        let input = r#"#!/usr/bin/env python3
import sys

def main():
    pass
"#;

        let expected = r#"#!/usr/bin/env python3
import sys
def main():
    pass
"#;

        assert_eq!(strip_comments(input), expected);
    }

    #[test]
    fn test_strip_comments_preserves_pep723_block() {
        // Test that PEP 723 inline script metadata blocks are preserved
        let input = r#"#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "prompt-toolkit>=3.0.47",
#     "pydantic>=2.9.1",
# ]
# ///
# This comment should be removed
import sys

def main():
    pass  # This comment should also be removed
"#;

        let expected = r#"#!/usr/bin/env python3
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "prompt-toolkit>=3.0.47",
#     "pydantic>=2.9.1",
# ]
# ///
import sys
def main():
    pass
"#;

        assert_eq!(strip_comments(input), expected);
    }

    #[test]
    fn test_strip_blank_lines_single() {
        // Test removing single blank lines
        let input = r#"#!/usr/bin/env python3

import sys

def main():
    pass
"#;

        let expected = r#"#!/usr/bin/env python3
import sys
def main():
    pass
"#;

        assert_eq!(strip_blank_lines(input), expected);
    }

    #[test]
    fn test_strip_blank_lines_multiple() {
        // Test removing multiple consecutive blank lines
        let input = r#"#!/usr/bin/env python3


import sys


def main():


    pass
"#;

        let expected = r#"#!/usr/bin/env python3
import sys
def main():
    pass
"#;

        assert_eq!(strip_blank_lines(input), expected);
    }

    #[test]
    fn test_strip_blank_lines_no_blank_lines() {
        // Test code without blank lines
        let input = r#"#!/usr/bin/env python3
import sys
def main():
    pass
"#;

        assert_eq!(strip_blank_lines(input), input);
    }

    #[test]
    fn test_strip_blank_lines_whitespace_only() {
        // Test that lines with only whitespace are removed
        let input = r#"#!/usr/bin/env python3

import sys

    def main():
    	pass
"#;

        let expected = r#"#!/usr/bin/env python3
import sys
    def main():
    	pass
"#;

        assert_eq!(strip_blank_lines(input), expected);
    }

    #[test]
    fn test_release_mode_complete_flow() {
        // Integration test for complete release mode flow with docstrings, comments, and blank lines
        let mut mock_fs = VirtualFileSystem::new();
        mock_fs.mkdir_p("/test/mylib").unwrap();

        // Module with docstrings, comments, and blank lines
        let mylib_py = r##""""My library module."""

# This is a module-level comment
import sys


MY_VAR = """This should be preserved."""


class MyClass:
    """This is a class docstring - should be removed."""

    # This is a comment about __init__
    def __init__(self):
        """Initialize the class."""
        self.name = "MyClass"


def my_func():
    """This is a function docstring - should be removed."""
    # Inline comment
    return "Hello"


# Another module-level comment
"##;
        mock_fs.write("/test/mylib/mylib.py", mylib_py).unwrap();

        // Main file with various comments and docstrings
        let main_py = r##"#!/usr/bin/env python3
"""Main script for testing."""

# Import statement
from mylib.mylib import MyClass


def main():
    """Main entry point."""
    # Create instance
    obj = MyClass()
    print(obj.name)


if __name__ == '__main__':
    # Run main
    main()
"##;
        mock_fs.write("/test/main.py", main_py).unwrap();

        let input_file = PathBuf::from("/test/main.py");
        let output_file = PathBuf::from("/test/main_inlined.py");
        let module_names = "mylib".to_string();
        let release = true;
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

        // Expected: shebang preserved, all docstrings removed, all comments removed,
        // all blank lines removed, imports consolidated and sorted, mylib inlined
        let expected = r#"#!/usr/bin/env python3
import sys
MY_VAR = """This should be preserved."""
class MyClass:
    def __init__(self):
        self.name = "MyClass"
def my_func():
    return "Hello"
def main():
    obj = MyClass()
    print(obj.name)
if __name__ == '__main__':
    main()
"#;

        assert_eq!(result, expected, "\n\nExpected:\n{}\n\nGot:\n{}\n", expected, result);
    }

    #[test]
    fn test_release_mode_preserves_pep723_block() {
        // Integration test for release mode with PEP 723 inline script metadata block
        let mut mock_fs = VirtualFileSystem::new();
        mock_fs.mkdir_p("/test/mylib").unwrap();

        // Simple module
        let mylib_py = r#"def helper():
    return "Hello"
"#;
        mock_fs.write("/test/mylib/helper.py", mylib_py).unwrap();

        // Main file with PEP 723 block
        let main_py = r#"#!/usr/bin/env python
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "prompt-toolkit>=3.0.47",
#     "pydantic>=2.9.1",
# ]
# ///
"""Main script."""

from mylib.helper import helper


def main():
    # This comment should be removed
    result = helper()
    print(result)


if __name__ == '__main__':
    # Run main
    main()
"#;
        mock_fs.write("/test/main.py", main_py).unwrap();

        let input_file = PathBuf::from("/test/main.py");
        let output_file = PathBuf::from("/test/main_inlined.py");
        let module_names = "mylib".to_string();
        let release = true;
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

        // Expected: PEP 723 block preserved, shebang preserved, docstrings removed,
        // other comments removed, blank lines removed, mylib inlined
        let expected = r#"#!/usr/bin/env python
# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "prompt-toolkit>=3.0.47",
#     "pydantic>=2.9.1",
# ]
# ///
def helper():
    return "Hello"
def main():
    result = helper()
    print(result)
if __name__ == '__main__':
    main()
"#;

        assert_eq!(result, expected, "\n\nExpected:\n{}\n\nGot:\n{}\n", expected, result);
    }
}
