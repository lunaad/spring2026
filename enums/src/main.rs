use std::io::{self, Write};
use std::process::Command;

// 1. Define the enum
enum FileOperation {
    List(String),
    Display(String),
    Create(String, String),
    Remove(String),
    Pwd,
}

// 3. Perform operation using Command::new()
fn perform_operation(operation: FileOperation) {
    match operation {
        FileOperation::List(path) => {
            let result = Command::new("ls")
                .arg(&path)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                }
                _ => eprintln!("Failed to list directory."),
            }
        }

        FileOperation::Display(path) => {
            let result = Command::new("cat")
                .arg(&path)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                }
                _ => eprintln!("Failed to display file."),
            }
        }

        FileOperation::Create(path, content) => {
            let command = format!("echo '{}' > {}", content, path);

            let result = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    println!("File '{}' created successfully.", path);
                }
                _ => eprintln!("Failed to create file."),
            }
        }

        FileOperation::Remove(path) => {
            let result = Command::new("rm")
                .arg(&path)
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    println!("File '{}' removed successfully.", path);
                }
                _ => eprintln!("Failed to remove file."),
            }
        }

        FileOperation::Pwd => {
            let result = Command::new("pwd").output();

            match result {
                Ok(output) if output.status.success() => {
                    println!(
                        "Current working directory: {}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                }
                _ => eprintln!("Failed to get current directory."),
            }
        }
    }
}

// Helper function to read input
fn get_input(prompt: &str) -> String {
    let mut input = String::new();
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    input.trim().to_string()
}

// 2. Main menu loop
fn main() {
    println!("Welcome to the File Operations Program!");

    loop {
        println!("\nFile Operations Menu:");
        println!("1. List files in a directory");
        println!("2. Display file contents");
        println!("3. Create a new file");
        println!("4. Remove a file");
        println!("5. Print working directory");
        println!("0. Exit");

        let choice = get_input("Enter your choice (0-5): ");

        match choice.as_str() {
            "1" => {
                let path = get_input("Enter directory path: ");
                perform_operation(FileOperation::List(path));
            }

            "2" => {
                let path = get_input("Enter file path: ");
                perform_operation(FileOperation::Display(path));
            }

            "3" => {
                let path = get_input("Enter file path: ");
                let content = get_input("Enter content: ");
                perform_operation(FileOperation::Create(path, content));
            }

            "4" => {
                let path = get_input("Enter file path: ");
                perform_operation(FileOperation::Remove(path));
            }

            "5" => {
                perform_operation(FileOperation::Pwd);
            }

            "0" => {
                println!("Goodbye!");
                break;
            }

            _ => {
                println!("Invalid option. Please try again.");
            }
        }
    }
}