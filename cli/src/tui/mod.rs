pub mod app;
pub mod ui;

use std::io;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::tui::app::{App, InputMode};



pub fn run() -> Result<()> {

    // Setup terminal

    enable_raw_mode()?;

    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;



    // Create app state

    let mut app = App::new();

    let res = run_app(&mut terminal, &mut app);



    // Restore terminal

    disable_raw_mode()?;

    execute!(

        terminal.backend_mut(),

        LeaveAlternateScreen,

        DisableMouseCapture

    )?;

    terminal.show_cursor()?;



    if let Err(err) = res {

        println!("{:?}", err);

    }



    Ok(())

}



fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {

    loop {

        terminal.draw(|f| ui::draw(f, app))

            .map_err(|e| io::Error::other(e.to_string()))?;



                if event::poll(std::time::Duration::from_millis(250))? {



                    if let Event::Key(key) = event::read()? {



                        match app.input_mode {



                            InputMode::Normal => {



                                match key.code {



                                    KeyCode::Char('q') => return Ok(()),



                                    KeyCode::Down | KeyCode::Char('j') => app.next(),



                                    KeyCode::Up | KeyCode::Char('k') => app.previous(),



                                    KeyCode::Char(' ') | KeyCode::Enter => app.toggle_status(),



                                    KeyCode::Char('d') | KeyCode::Delete => app.delete_task(),



                                    KeyCode::Char('a') => app.enter_add_mode(),



                                    KeyCode::Char('m') => app.enter_modify_mode(),



                                    _ => {}



                                }



                            },



                            InputMode::Adding | InputMode::Modifying => {



                                match key.code {



                                    KeyCode::Enter => app.submit_command(),



                                    KeyCode::Esc => app.exit_input_mode(),



                                    KeyCode::Char(c) => app.input_char(c),



                                    KeyCode::Backspace => app.delete_char(),



                                    KeyCode::Left => app.move_cursor_left(),



                                    KeyCode::Right => app.move_cursor_right(),



                                    _ => {}



                                }



                            }



                        }



                    }



                }



        

    }

}
