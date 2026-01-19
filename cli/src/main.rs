mod tui;

use clap::Parser;
use todoism_core::{greet, Task, FileTaskRepository, TaskRepository, parse_args, expand_key, parse_human_date, Priority, Status, parse_duration};
use todoism_core::service::task_service::{TaskService, SortStrategy, calculate_score};
use anyhow::{Result};
use std::collections::HashMap;
use itertools::Itertools;
use tabled::{Table, settings::Style};
use chrono::{Datelike, Duration, Local};
use crossterm::style::Stylize;

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
    /// Show completed task history
    History,
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
    let repo = FileTaskRepository::new(None)?;
    let service = TaskService::new(repo.clone());

    // Define known keys for expansion
    let known_keys = vec!["due", "project", "priority", "description", "estimate"];

    let cli = Cli::parse();

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

            let created_task = service.create_task(new_task)?;
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
            let strategy = SortStrategy::Urgency;
            let tasks = service.get_sorted_tasks(strategy)?;
            
            if tasks.is_empty() {
                println!("No tasks found.");
            } else {
                println!("{:<8} {:<8} {:<10} {:<12} {:<10} {:<20}", "ID", "Score", "Priority", "Due", "Project", "Description");
                println!("{:-<8} {:-<8} {:-<10} {:-<12} {:-<10} {:-<20}", "", "", "", "", "", "");
                
                for task in tasks {
                    let id_str = task.id.to_string();
                    let short_id = if id_str.len() > 8 { &id_str[..8] } else { &id_str }; 
                    let pri = format!("{:?}", task.priority);
                    let due = task.due.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());
                    let project = task.project.clone().unwrap_or_else(|| "-".to_string());
                    // TaskDto now has the score directly
                    let score = task.score;
                    
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
        Some(Commands::History) => {
             run_history(&repo)?;
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

fn run_history(repo: &FileTaskRepository) -> Result<()> {
    let tasks = repo.list()?;
    let mut completed_tasks: Vec<_> = tasks.into_iter()
        .filter(|t| t.status == Status::Completed && t.completed_at.is_some())
        .collect();

    if completed_tasks.is_empty() {
        println!("No completed tasks found in history.");
        return Ok(());
    }

    // Sort by completion time desc (Recent first)
    completed_tasks.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));

    // Group by Week
    let weeks = completed_tasks.into_iter().chunk_by(|t| {
        let date = t.completed_at.unwrap().with_timezone(&Local);
        date.iso_week()
    });

    for (week, week_tasks) in &weeks {
        let week_tasks_vec: Vec<_> = week_tasks.collect();
        let week_total = sum_estimates(&week_tasks_vec);

        println!("\n{} {} {}",
            format!("Week {}", week.week()).bold().cyan(),
            format!("({})", week.year()).dim(),
            format!("Total: {}", format_duration(week_total)).yellow()
        );

        // Group by Day
        let days = week_tasks_vec.into_iter().chunk_by(|t| {
            t.completed_at.unwrap().with_timezone(&Local).date_naive()
        });

        for (day, day_tasks) in &days {
            let day_tasks_vec: Vec<_> = day_tasks.collect();
            let day_total = sum_estimates(&day_tasks_vec);

            println!("  {} {}",
                format!("{}", day.format("%Y-%m-%d (%a)")).bold(),
                format!("Total: {}", format_duration(day_total)).yellow()
            );

            let table_rows: Vec<HistoryRow> = day_tasks_vec.iter().map(HistoryRow::from).collect();
            let mut table = Table::new(table_rows);
            table.with(Style::modern());

            let table_str = table.to_string();
            for line in table_str.lines() {
                println!("    {}", line);
            }
            println!();
        }
    }
    Ok(())
}

fn sum_estimates(tasks: &[Task]) -> Duration {
    tasks.iter().fold(Duration::zero(), |acc, t| {
        if let Some(est) = &t.estimate {
            if let Ok(d) = parse_duration(est) {
                return acc + d;
            }
        }
        acc
    })
}

fn format_duration(d: Duration) -> String {
    let hours = d.num_hours();
    let minutes = d.num_minutes() % 60;
    if hours > 0 {
        if minutes > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", minutes)
    }
}

#[derive(tabled::Tabled)]
struct HistoryRow {
    #[tabled(rename = "Time")]
    time: String,
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Project")]
    project: String,
    #[tabled(rename = "Task")]
    name: String,
    #[tabled(rename = "Est")]
    estimate: String,
}

impl From<&Task> for HistoryRow {
    fn from(task: &Task) -> Self {
        let id_str = task.id.to_string();
        let short_id = if id_str.len() > 8 { &id_str[..8] } else { &id_str };

        let time_str = task.completed_at
            .map(|dt| dt.with_timezone(&Local).format("%H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());

        Self {
            time: time_str,
            id: short_id.to_string(),
            project: task.project.clone().unwrap_or_default(),
            name: task.name.clone(),
            estimate: task.estimate.clone().unwrap_or_default(),
        }
    }
}
