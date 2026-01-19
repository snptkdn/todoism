use clap::Parser;
use todoism_core::{greet, Task, FileTaskRepository, TaskRepository, parse_args, expand_key, parse_human_date, Priority};
use anyhow::{Result};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "todoism")]
#[command(about = "A robust CLI task manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Greet the user (test command)
    Greet,
    /// Add a new task (usage: add "Task Name" due:2025-01-01 project:Work pri:H)
    Add {
        /// Task details including name and metadata (key:value)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn parse_priority(pri_str: &str) -> Priority {
    match pri_str.to_lowercase().as_str() {
        "h" | "high" => Priority::High,
        "m" | "medium" | "med" => Priority::Medium,
        "l" | "low" => Priority::Low,
        _ => Priority::Medium,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let repo = FileTaskRepository::new(None)?;

    // Define known keys for expansion
    let known_keys = vec!["due", "project", "priority", "description", "estimate"];

    match cli.command {
        Commands::Greet => {
            println!("{}", greet());
        },
        Commands::Add { args } => {
            if args.is_empty() {
                println!("Error: Task name is required.");
                return Ok(());
            }

            let parsed = parse_args(&args);
            
            if parsed.name.is_empty() {
                 println!("Error: Task name is required.");
                 return Ok(());
            }

            // Normalize metadata keys
            let mut normalized_metadata = HashMap::new();
            for (key, value) in parsed.metadata {
                match expand_key(&key, &known_keys) {
                    Ok(full_key) => {
                        normalized_metadata.insert(full_key, value);
                    },
                    Err(e) => {
                         println!("Warning: {}", e);
                    }
                }
            }

            let due = if let Some(d) = normalized_metadata.get("due") {
                match parse_human_date(d) {
                    Ok(dt) => Some(dt),
                    Err(e) => {
                        println!("Warning: Invalid due date '{}': {}", d, e);
                        None
                    }
                }
            } else {
                None
            };

            let project = normalized_metadata.get("project").cloned();
            let priority = normalized_metadata.get("priority")
                .map(|p| parse_priority(p))
                .unwrap_or_default();
            let description = normalized_metadata.get("description").cloned();
            let estimate = normalized_metadata.get("estimate").cloned();

            let mut new_task = Task::new(parsed.name, due);
            new_task.project = project;
            new_task.priority = priority;
            new_task.description = description;
            new_task.estimate = estimate;

            let created_task = repo.create(new_task)?;
            println!("Task added: {} (ID: {})", created_task.name, created_task.id);
            if let Some(d) = created_task.due {
                println!("  Due: {}", d);
            }
            if let Some(p) = created_task.project {
                println!("  Project: {}", p);
            }
            println!("  Priority: {:?}", created_task.priority);
        },
    }
    Ok(())
}
