use console::{Term, Style};
use std::sync::Mutex;

pub struct UI {
    console: Term,
    write_index: usize,
    max_row: usize,
    position: Mutex<usize>
}

pub fn create_ui() -> UI {
    let crate_term = Term::stdout();
    let (rows, _) = crate_term.size();
    let pos = Mutex::new(0 as usize);
    UI{console: crate_term, write_index: 0, max_row: rows as usize, position: pos}
}

impl UI {
    pub fn clear_screen_and_reset_cursor(&self) {
        self.console.clear_screen().unwrap();
        self.console.move_cursor_to(0, self.max_row).unwrap();
    }

    pub fn move_to_input_pos(&self) {
        self.console.move_cursor_to(0, self.max_row).unwrap();
    }

    pub fn update_title(&self, suffix: &str) {
        self.console.set_title(format!("Rusty Chat - Chatting with {}", suffix));
    }

    fn write_string_to_console(&mut self, message: &str, style: Style) {
        // not entirely sure i still need that lock...maybe in chat rooms
        let lock = self.position.lock().unwrap();
        self.console.move_cursor_to(0, self.write_index).unwrap();
        self.console.write_line(format!("{}", style.apply_to(message)).as_str()).unwrap();
        // is that in regular win10 cmd needed?
        self.write_index = self.write_index + 1;
        if self.write_index >= self.max_row {
            self.console.clear_line().unwrap();
            self.console.move_cursor_to(0, self.write_index + 1).unwrap();
        } else {
            self.console.move_cursor_to(0, self.max_row).unwrap();
            self.console.clear_line().unwrap();
        }
    }

    pub fn reset_input_line(&self) {
        self.console.move_cursor_to(0, self.write_index - 1).unwrap();
        self.console.clear_line().unwrap();
        self.move_to_input_pos(); // TODO: Wrong if we are beyond initial max_row
    }

    pub fn read_line(&self) -> String {
        self.console.read_line().unwrap()
    }

    pub fn write_sys_message(&mut self, message: &str) {
        self.write_string_to_console(message, Style::new().green());
    }

    pub fn write_info_message(&mut self, message: &str) {
        self.write_string_to_console(message, Style::new().yellow());
    }

    pub fn write_err_message(&mut self, message: &str) {
        self.write_string_to_console(message, Style::new().red());
    }
}
