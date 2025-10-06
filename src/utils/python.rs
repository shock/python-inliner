use std::process::{Command, Stdio};
use std::str;

// Create a custom error type
#[derive(Debug)]
pub struct CommandError(String);

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CommandError {}

pub fn get_python_sys_path() -> Result<Vec<String>, CommandError> {
    // Launch the Python subprocess
    let output = Command::new("python3") // or "python" depending on your setup
        .arg("-c") // Use the -c option to run the following command
        .arg("import sys; print('\\n'.join(sys.path))") // Correctly escape the newline character
        .stdout(Stdio::piped()) // Capture standard output
        .stderr(Stdio::piped()) // Capture standard error
        .output(); // Execute the command and capture the output

    if let Err(e) = output {
        return Err(CommandError(format!("Command failed with error: {}", e)));
    }
    // Check if the command was successful
    let output = output.unwrap();
    if !output.status.success() {
        // Capture stdout and stderr
        let stdout_str = str::from_utf8(&output.stdout).unwrap_or("<invalid utf-8>");
        let stderr_str = str::from_utf8(&output.stderr).unwrap_or("<invalid utf-8>");

        eprintln!("Error: Command failed with status: {}", output.status);
        eprintln!("stdout: {}", stdout_str);
        eprintln!("stderr: {}", stderr_str);

        return Err(CommandError(format!(
            "Command failed with status: {}",
            output.status
        )));
    }

    // Convert the output to a String
    let output_str = str::from_utf8(&output.stdout);

    match output_str {
        Ok(output_str) => {
            // Split the output into lines and collect into a Vec<String>
            let sys_path: Vec<String> = output_str.lines().map(String::from).collect();
            Ok(sys_path)
        },
        Err(e) => {
            return Err(CommandError(format!("Error converting output to string: {}", e)));
        }
    }
}
