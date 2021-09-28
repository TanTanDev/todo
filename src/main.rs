use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use termion::event::Key;
use termion::raw::IntoRawMode;
use tui::backend::Backend;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::text::{Span, Text};
use tui::widgets::{Block, Borders};
use tui::widgets::{List, ListItem, Paragraph};
use tui::Terminal;

use chrono::prelude::*;
mod events;
use events::*;

// the tasks for today
// saved to file
#[derive(Serialize, Deserialize, Debug)]
struct Today {
    tasks: Vec<Task>,
    date: Option<chrono::DateTime<Local>>,
}

impl Default for Today {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            date: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct WeekdayTask {
    tasks: Vec<String>,
    day_info: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct WeekdayTasks {
    tasks: HashMap<Weekday, WeekdayTask>,
}

impl Default for WeekdayTasks {
    fn default() -> Self {
        let tasks: HashMap<Weekday, WeekdayTask> = [
            (
                Weekday::Mon,
                WeekdayTask {
                    tasks: ["check mail".to_string()].into(),
                    day_info: "business day".to_string(),
                },
            ),
            (
                Weekday::Fri,
                WeekdayTask {
                    tasks: ["cleanup mail".to_string()].into(),
                    day_info: "wrap up day".to_string(),
                },
            ),
        ]
        .iter()
        .cloned()
        .collect();
        Self { tasks }
    }
}

#[derive(PartialEq)]
enum AppMode {
    Edit,
    Insert,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
enum Status {
    Todo,
    Done,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {
    status: Status,
    info: String,
}

impl Task {
    fn into_list_item(&self) -> ListItem {
        let box_token: &str = match self.status {
            Status::Todo => "☐",
            Status::Done => "☑",
        };

        let span = Span::raw(format!("{} {}", box_token, self.info));
        ListItem::new(span)
    }
}

fn save_today(tasks: &Vec<Task>, path: &str) {
    let local: DateTime<Local> = Local::now();
    let today = Today {
        tasks: tasks.to_vec(),
        date: Some(local),
    };
    let serialized = serde_json::to_string(&today).unwrap();
    let save_result = std::fs::write(path, serialized);
    println!("tried saving file {}, result: {:?}", path, save_result);
}

fn main() -> Result<(), io::Error> {
    let mut working_path = std::env::current_exe().unwrap();
    // get rid of application name
    working_path.pop();
    let daily_path = format!(
        "{}/{}",
        working_path.to_str().unwrap(),
        "daily_occuring.json"
    );
    let today_path = format!("{}/{}", working_path.to_str().unwrap(), "today.json");
    println!("files path: {:?}", today_path);

    let file_result = std::fs::read_to_string(&daily_path);
    let weekday_tasks = match file_result {
        Ok(file_string) => {
            let daily_occuring = serde_json::from_str(&file_string).expect("corrupt file");
            daily_occuring
        }
        Err(_file_error) => {
            // return default daily occuring data
            let default_weekday_tasks = WeekdayTasks::default();
            let serialized = serde_json::to_string(&default_weekday_tasks).unwrap();
            let save_result = std::fs::write(&daily_path, serialized);
            println!("saved to daily_occuring? {:?}", save_result);
            default_weekday_tasks
        }
    };
    let local: DateTime<Local> = Local::now();
    let weekday = local.date().weekday();
    let mut tasks = Vec::<Task>::new();
    if let Some(day_tasks) = weekday_tasks.tasks.get(&weekday) {
        for day_task in day_tasks.tasks.iter() {
            tasks.push(Task {
                status: Status::Todo,
                info: day_task.clone(),
            });
        }
    }
    // check if we have a save for today
    let today_file_result = std::fs::read_to_string(&today_path);
    if let Ok(today_string) = today_file_result {
        if let Ok(today) = serde_json::from_str::<Today>(&today_string) {
            if let Some(today_date) = today.date {
                if today_date.weekday() == weekday {
                    for saved_task in today.tasks.into_iter() {
                        for loaded_task in tasks.iter_mut() {
                            if loaded_task.info == saved_task.info {
                                loaded_task.status = saved_task.status;
                            }
                        }
                        if !tasks.iter().any(|t| t.info == saved_task.info) {
                            tasks.push(saved_task);
                        }
                    }
                }
            }
        }
    }

    let stdout = io::stdout().into_raw_mode()?;
    let mut backend = TermionBackend::new(stdout);
    let _ = backend.clear();
    let mut terminal = Terminal::new(backend)?;

    let mut selected: i32 = 0;
    let events = Events::new();
    let mut app_mode = AppMode::Edit;
    let mut input_string = String::new();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let items: Vec<ListItem> = tasks
                .iter()
                .enumerate()
                .map(|(index, task)| {
                    let mut list_item = task.into_list_item();
                    // modify style if selected
                    if index == selected as usize {
                        list_item = ListItem::style(list_item, Style::default().bg(Color::Magenta));
                    }

                    list_item
                })
                .collect();

            let title = match weekday_tasks.tasks.get(&weekday) {
                Some(task_data) => Paragraph::new(Text::raw(format!(
                    "{}: {}",
                    weekday,
                    task_data.day_info.clone()
                ))),
                None => Paragraph::new(Text::raw(format!(
                    "no daily occuring task for {:?}",
                    weekday
                ))),
            };
            f.render_widget(title, chunks[0]);

            let list = List::new(items);
            f.render_widget(list, chunks[1]);

            if AppMode::Insert == app_mode {
                let input = Paragraph::new(Text::raw(input_string.as_str()))
                    .block(Block::default().borders(Borders::ALL).title("new task"));
                f.render_widget(input, chunks[2]);
                f.set_cursor(
                    // Put cursor past the end of the input text
                    chunks[2].x + input_string.len() as u16 + 1,
                    // Move one line down, from the border to the input line
                    chunks[2].y + 1,
                )
            }
        })?;

        for event in events.next() {
            if let Event::Input(input) = event {
                match app_mode {
                    AppMode::Edit => {
                        match input {
                            Key::Char('j') => {
                                selected += 1;
                                if selected as usize >= tasks.len() {
                                    selected = 0;
                                }
                            }
                            Key::Char('k') => {
                                selected -= 1;
                                if selected < 0 {
                                    selected = (tasks.len() - 1) as i32;
                                }
                            }
                            Key::Char('l') => {
                                // modify the current selected task
                                let mut task = tasks.get_mut(selected as usize).unwrap();
                                task.status = Status::Done;
                            }
                            Key::Char('h') => {
                                // modify the current selected task
                                let mut task = tasks.get_mut(selected as usize).unwrap();
                                task.status = Status::Todo;
                            }
                            // enter insert mode
                            Key::Char('i') => {
                                // modify the current selected task
                                app_mode = AppMode::Insert;
                            }
                            Key::Char('x') => {
                                // remove entry
                                if selected >= 0 && selected < tasks.len() as i32 {
                                    tasks.remove(selected as usize);
                                    if selected as usize >= tasks.len() {
                                        selected = tasks.len() as i32 - 1;
                                    }
                                }
                            }
                            Key::Char('q') => {
                                save_today(&tasks, &today_path);
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                    AppMode::Insert => {
                        match input {
                            Key::Esc => {
                                app_mode = AppMode::Edit;
                                input_string.clear();
                            }
                            Key::Char('\n') => {
                                // submit
                                app_mode = AppMode::Edit;
                                tasks.push(Task {
                                    status: Status::Todo,
                                    info: input_string.drain(..).collect(),
                                });
                            }
                            Key::Backspace => {
                                input_string.pop();
                            }
                            Key::Char(c) => {
                                input_string.push(c);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
