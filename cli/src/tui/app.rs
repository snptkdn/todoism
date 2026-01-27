use ratatui::widgets::TableState;
use todoism_core::{FileTaskRepository, FileDailyLogRepository, Task, TaskDto, parse_args, expand_key, parse_human_date, Priority};
use todoism_core::{TaskService, DailyLogService, SortStrategy};
use todoism_core::usecase::daily_plan::{DailyPlanUseCase, DailyPlanStats};
use std::collections::HashMap;
use chrono::Local;
use uuid::Uuid;

pub enum InputMode {
    Normal,
    Adding,
    Modifying,
    MeetingHoursPrompt,
    CompleteWithEffort,
}

pub struct App {
    pub service: TaskService<FileTaskRepository>,
    pub daily_log_service: DailyLogService<FileDailyLogRepository>,
    pub tasks: Vec<TaskDto>,
    pub state: TableState,
    pub input: String,
    pub input_mode: InputMode,
    pub cursor_position: usize,
    pub task_id_for_prompt: Option<Uuid>,
    
    // Capacity Stats
    pub daily_stats: DailyPlanStats,
}

impl App {
    pub fn new() -> App {
        let repo = FileTaskRepository::new(None).expect("Failed to initialize repository");
        let service = TaskService::new(repo);
        
        let log_repo = FileDailyLogRepository::new(None).expect("Failed to initialize log repository");
        let daily_log_service = DailyLogService::new(log_repo);
        
        let mut input_mode = InputMode::Normal;
        let today = Local::now().date_naive();
        
        // Check log existence for prompt
        if let Ok(has_log) = daily_log_service.has_log(today) {
             if !has_log {
                 input_mode = InputMode::MeetingHoursPrompt;
             }
        }
        
        // Fetch all tasks first
        let mut all_tasks = service.get_sorted_tasks(SortStrategy::Urgency).unwrap_or_default();
        
        // Apply Daily Plan Logic (Mutates tasks to add fit info)
        let usecase = DailyPlanUseCase::new(&daily_log_service);
        let daily_stats = usecase.apply_daily_plan(&mut all_tasks).unwrap_or_default();

        // Filter for display
        let tasks: Vec<TaskDto> = all_tasks.into_iter()
            .filter(|t| t.status != "Completed" && t.status != "Deleted")
            .collect();

        let mut state = TableState::default();
        if !tasks.is_empty() {
            state.select(Some(0));
        }
        App { 
            service,
            daily_log_service,
            tasks, 
            state,
            input: String::new(),
            input_mode,
            cursor_position: 0,
            task_id_for_prompt: None,
            daily_stats,
        }
    }

    pub fn next(&mut self) {
        if self.tasks.is_empty() { return; }
        
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.tasks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.tasks.is_empty() { return; }

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tasks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn toggle_status(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(task) = self.tasks.get(i) {
                // Since only Pending tasks are shown, we are completing it.
                // If we ever show Completed tasks, we should check task.status here.
                
                self.input_mode = InputMode::CompleteWithEffort;
                self.task_id_for_prompt = Some(task.id);
                
                if let Some(est) = &task.estimate {
                    self.input = est.clone();
                    self.cursor_position = self.input.len();
                } else {
                    self.input.clear();
                    self.cursor_position = 0;
                }
            }
        }
    }

    pub fn delete_task(&mut self) {
        if let Some(i) = self.state.selected() {
            if let Some(task) = self.tasks.get(i) {
                let _ = self.service.delete_task(&task.id);
            }
            // Instead of manually removing, just reload to be safe and consistent with sorting
            self.reload_tasks();
            
            // Adjust selection after reload
            if self.tasks.is_empty() {
                self.state.select(None);
            } else if i >= self.tasks.len() {
                self.state.select(Some(self.tasks.len() - 1));
            } else {
                self.state.select(Some(i));
            }
        }
    }

    fn reload_tasks(&mut self) {
        if let Ok(mut all_tasks) = self.service.get_sorted_tasks(SortStrategy::Urgency) {
             let usecase = DailyPlanUseCase::new(&self.daily_log_service);
             if let Ok(stats) = usecase.apply_daily_plan(&mut all_tasks) {
                 self.daily_stats = stats;
             }
             
             self.tasks = all_tasks.into_iter()
                .filter(|t| t.status != "Completed" && t.status != "Deleted")
                .collect();
        }
    }

    pub fn enter_add_mode(&mut self) {
        self.input_mode = InputMode::Adding;
        self.input.clear();
        self.cursor_position = 0;
    }

    pub fn enter_modify_mode(&mut self) {
        if self.state.selected().is_some() {
            self.input_mode = InputMode::Modifying;
            self.input.clear();
            self.cursor_position = 0;
        }
    }

    pub fn exit_input_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn input_char(&mut self, c: char) {
        let byte_index = self.input.chars().take(self.cursor_position).map(|c| c.len_utf8()).sum();
        self.input.insert(byte_index, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let byte_index: usize = self.input.chars().take(self.cursor_position - 1).map(|c| c.len_utf8()).sum();
            self.input.remove(byte_index);
            self.cursor_position -= 1;
        }
    }
    
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.chars().count() {
            self.cursor_position += 1;
        }
    }

    pub fn submit_command(&mut self) {
        if self.input.trim().is_empty() {
            self.exit_input_mode();
            return;
        }

        match self.input_mode {
            InputMode::Adding => self.submit_add(),
            InputMode::Modifying => self.submit_modify(),
            InputMode::MeetingHoursPrompt => self.submit_meeting_hours(),
            InputMode::CompleteWithEffort => self.submit_complete_with_effort(),
            InputMode::Normal => {},
        }

        self.input.clear();
        self.cursor_position = 0;
        self.exit_input_mode();
    }

    fn submit_add(&mut self) {
        let args: Vec<String> = self.input.split_whitespace().map(|s| s.to_string()).collect();
        let parsed = parse_args(&args);
        
        if parsed.name.is_empty() { return; }

        let known_keys = vec!["due", "project", "priority", "description", "estimate"];
        let mut normalized_metadata = HashMap::new();
        
        for (key, value) in parsed.metadata {
            if let Ok(full_key) = expand_key(&key, &known_keys) {
                normalized_metadata.insert(full_key, value);
            }
        }

        let due = normalized_metadata.get("due").and_then(|d| parse_human_date(d).ok());
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

        if let Ok(_) = self.service.create_task(new_task) {
             self.reload_tasks();
             if !self.tasks.is_empty() {
                 self.state.select(Some(0));
             }
        }
    }

    fn submit_modify(&mut self) {
        if let Some(i) = self.state.selected() {
             let args: Vec<String> = self.input.split_whitespace().map(|s| s.to_string()).collect();
             let parsed = parse_args(&args);
             
             let known_keys = vec!["due", "project", "priority", "description", "estimate"];
             
             if let Some(task_dto) = self.tasks.get(i) {
                 // Fetch the full entity to modify
                 if let Ok(mut task) = self.service.get_task(&task_dto.id) {
                     if !parsed.name.is_empty() {
                         task.name = parsed.name;
                     }
                     
                     for (key, value) in parsed.metadata {
                        if let Ok(full_key) = expand_key(&key, &known_keys) {
                            match full_key.as_str() {
                                "due" => {
                                    if let Ok(d) = parse_human_date(&value) {
                                        task.due = Some(d);
                                    }
                                },
                                "project" => task.project = Some(value),
                                "priority" => task.priority = parse_priority_str(&value),
                                "description" => task.description = Some(value),
                                "estimate" => task.estimate = Some(value),
                                _ => {}
                            }
                        }
                     }
                     let _ = self.service.update_task(&task);
                 }
             }
             self.reload_tasks();
        }
    }

    fn submit_complete_with_effort(&mut self) {
        if let Some(id) = self.task_id_for_prompt {
            let effort = self.input.trim().to_string();
            // Even if empty, we might want to allow it? 
            // The prompt defaults to estimate. If user clears it, maybe it means 0?
            // Let's pass whatever string they gave.
            let _ = self.service.complete_task_with_effort(&id, effort);
            self.task_id_for_prompt = None;
            self.reload_tasks();
        }
    }

    fn submit_meeting_hours(&mut self) {
        if let Ok(hours) = self.input.trim().parse::<f64>() {
            let today = Local::now().date_naive();
            let _ = self.daily_log_service.add_log(today, hours);
            self.input_mode = InputMode::Normal;
        } else {
             // Invalid input, maybe clear or keep for correction. 
             // For now, let's just clear and stay in mode or maybe provide visual feedback (not implemented in this step).
             // Let's assume user might retry. 
             // If input is empty/invalid, we could default to 0.0 or force them to type correct number.
             if self.input.trim() == "0" || self.input.trim().is_empty() {
                  let today = Local::now().date_naive();
                 let _ = self.daily_log_service.add_log(today, 0.0);
                 self.input_mode = InputMode::Normal;
             }
        }
    }
}

fn parse_priority_str(s: &str) -> Priority {
    match s.to_lowercase().as_str() {
        "h" | "high" => Priority::High,
        "m" | "medium" => Priority::Medium,
        "l" | "low" => Priority::Low,
        _ => Priority::Medium,
    }
}
