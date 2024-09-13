use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::error::Error;
use regex::Regex;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "pyroll", about = "Python File Inliner")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input_file: PathBuf,

    #[structopt(parse(from_os_str))]
    output_file: PathBuf,

    #[structopt(help = "Name of the module to be inlined", default_value = "")]
    modules_name: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    // get the input_file as a fully qualified path
    let input_file = fs::canonicalize(&opt.input_file)?;
    println!("Input file: {:?}", input_file);


    // get the working directory from the input file path
    let working_dir = input_file.parent().unwrap();
    println!("Working directory: {:?}", working_dir);

    let content = inline_imports(&working_dir, &opt.input_file, &opt.modules_name, &mut HashSet::new())?;
    fs::write(&opt.output_file, content)?;
    println!("Inlined content written to {:?}", opt.output_file);
    Ok(())
}

fn inline_imports(workding_dir: &Path, file: &Path, modules_name: &str, processed: &mut HashSet<PathBuf>) -> Result<String, Box<dyn Error>> {
    if !processed.insert(file.to_path_buf()) {
        return Err("Circular import detected".into());
    }

    let content = fs::read_to_string(file)?;
    let import_regex = Regex::new(&format!(r"(?m)^(\s*)from\s+{}(\S*)\s+import\s+.+$", regex::escape(modules_name)))?;

    let mut result = String::new();
    let mut last_end = 0;

    for cap in import_regex.captures_iter(&content) {
        let indent = &cap[1];
        let submodule = &cap[2];
        let start = cap.get(0).unwrap().start();
        let end = cap.get(0).unwrap().end();
        println!("indent: {indent}, submodule: {submodule}, start: {start}, end: {end}");
        result.push_str(&content[last_end..start]);

        let module_path = workding_dir.join(modules_name.replace(".", "/") + &submodule.replace(".", "/") + ".py");
        println!("module_path: {:?}", module_path);
        if module_path.exists() {
            println!("module_path exists");
            let inlined_content = inline_imports(workding_dir, &module_path, modules_name, processed)?;
            result.push_str(&format!("{indent}# Inlined module: {}{}\n", modules_name, submodule));
            result.push_str(&indent);
            result.push_str(&inlined_content.replace("\n", &format!("\n{indent}")));
            result.push_str("\n");
        } else {
            result.push_str(&content[start..end]);
            result.push('\n');
        }

        last_end = end;
    }

    result.push_str(&content[last_end..]);
    processed.remove(file);
    Ok(result)
}
