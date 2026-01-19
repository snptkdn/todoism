mod tui;

use clap::Parser;
use todoism_core::{greet, Task, FileTaskRepository, TaskRepository, parse_args, expand_key, parse_human_date, Priority, sort_tasks, SortStrategy};
use anyhow::{Result};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "todoism")]
#[command(about = "A robust CLI task manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
    /// List all tasks
    List,
    /// Open the Terminal User Interface
    Tui,
}

fn parse_priority_str(pri_str: &str) -> Priority {
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
        Some(Commands::Greet) => {
            println!("{}", greet());
        },
        Some(Commands::Add { args }) => {
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
                .map(|p| parse_priority_str(p))
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
        Some(Commands::List) => {
            let mut tasks = repo.list()?;
            if tasks.is_empty() {
                println!("No tasks found.");
            } else {
                // Apply sorting (Urgency by default)
                let strategy = SortStrategy::Urgency;
                sort_tasks(&mut tasks, strategy);

                println!("{:<8} {:<8} {:<10} {:<12} {:<10} {:<20}", "ID", "Score", "Priority", "Due", "Project", "Description");
                println!("{:-<8} {:-<8} {:-<10} {:-<12} {:-<10} {:-<20}", "", "", "", "", "", "");
                
                for task in tasks {
                    let id_str = task.id.to_string();
                    let short_id = if id_str.len() > 8 { &id_str[..8] } else { &id_str }; 
                    let pri = format!("{:?}", task.priority);
                    let due = task.due.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());
                    let project = task.project.clone().unwrap_or_else(|| "-".to_string());
                    let score = task.score(strategy);
                    
                    println!("{:<8} {:<8.1} {:<10} {:<12} {:<10} {}", 
                        short_id,
                        score, 
                        pri, 
                        due, 
                        project, 
                        task.name
                    );
                }
            }
        },
        Some(Commands::Tui) => {
            tui::run()?;
        },
        None => {
            tui::run()?;
        }
    }
    Ok(())
}
