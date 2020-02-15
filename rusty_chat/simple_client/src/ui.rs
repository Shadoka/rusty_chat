use console::{Term, Style};
use std::sync::{Arc, Mutex};

pub struct UI {
    console: Term,
    write_index: usize,
    max_row: usize
}

pub fn create_ui() -> UI {
    let crate_term = Term::stdout();
    let (rows, _) = crate_term.size();
    UI{console: crate_term, write_index: 0, max_row: rows as usize}
}

pub fn write_sys_message(term: &Arc<Mutex<UI>>, message: &str) {
    let mut term_locked = term.lock().unwrap();
    term_locked.write_string_to_console(message, Style::new().green());
}

pub fn write_err_message(term: &Arc<Mutex<UI>>, message: &str) {
    let mut term_locked = term.lock().unwrap();
    term_locked.write_string_to_console(message, Style::new().red());
}

pub fn write_info_message(term: &Arc<Mutex<UI>>, message: &str) {
    let mut term_locked = term.lock().unwrap();
    term_locked.write_string_to_console(message, Style::new().yellow());
}

pub fn reset_input_line(arc_term: &Arc<Mutex<UI>>) {
    let term = arc_term.lock().unwrap();
    term.reset_current_line();
}

pub fn read_line(arc_term: &Arc<Mutex<UI>>, prompt_text: &str) -> String {
    let term = arc_term.lock().unwrap();
    term.read_line(prompt_text)
}

impl UI {
    pub fn clear_screen_and_reset_cursor(&self) {
        self.console.clear_screen().unwrap();
        self.console.move_cursor_to(0, self.max_row).unwrap();
    }

    pub fn write_string_to_console(&mut self, message: &str, style: Style) {
        self.console.move_cursor_to(0, self.write_index).unwrap();
        self.console.write_line(format!("{}", style.apply_to(message)).as_str()).unwrap();
        self.write_index = self.write_index + 1;
        if self.write_index >= self.max_row {
            self.console.clear_line().unwrap();
            self.console.move_cursor_to(0, self.write_index + 1).unwrap();
        } else {
            self.console.move_cursor_to(0, self.max_row).unwrap();
            self.console.clear_line().unwrap();
        }
    }

    pub fn reset_current_line(&self) {
        self.console.clear_line().unwrap();
    }

    pub fn read_line(&self, prompt_text: &str) -> String {
        let mut read = self.console.read_line_initial_text(prompt_text).unwrap();
        let input = read.split_off(prompt_text.len());
        self.console.clear_line().unwrap();
        input
    }
}
