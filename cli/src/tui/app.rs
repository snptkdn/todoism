use ratatui::widgets::TableState;
use todoism_core::{FileTaskRepository, Task, TaskRepository};

pub struct App {
    pub tasks: Vec<Task>,
    pub state: TableState,
}

impl App {
    pub fn new() -> App {
        let repo = FileTaskRepository::new(None).expect("Failed to initialize repository");
        let tasks = repo.list().unwrap_or_default();
        let mut state = TableState::default();
        if !tasks.is_empty() {
            state.select(Some(0));
        }
        App { tasks, state }
    }

    pub fn next(&mut self) {
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
}
