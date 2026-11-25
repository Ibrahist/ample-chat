use ample_chat_proto::UserSummary;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::client::DisplayMessage;

pub struct App {
    pub username: String,
    pub messages: Vec<DisplayMessage>,
    pub users: Vec<UserSummary>,
    pub input: String,
    pub cursor_pos: usize,
    pub status: String,
    pub connected: bool,
    pub is_tls: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(username: impl Into<String>, is_tls: bool) -> Self {
        Self {
            username: username.into(),
            messages: Vec::new(),
            users: Vec::new(),
            input: String::new(),
            cursor_pos: 0,
            status: "Connecting...".to_string(),
            connected: false,
            is_tls,
            should_quit: false,
        }
    }

    /// Handles keyboard input. Returns `Some(message_to_send)` when Enter is pressed with text.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        // Handle Ctrl+C or Esc to quit
        if key.code == KeyCode::Esc
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            self.should_quit = true;
            return None;
        }

        match key.code {
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                None
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
                None
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.input.len() {
                    self.input.remove(self.cursor_pos);
                }
                None
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                None
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
                None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                None
            }
            KeyCode::End => {
                self.cursor_pos = self.input.len();
                None
            }
            KeyCode::Enter => {
                let trimmed = self.input.trim().to_string();
                if !trimmed.is_empty() {
                    self.input.clear();
                    self.cursor_pos = 0;
                    Some(trimmed)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn add_message(&mut self, msg: DisplayMessage) {
        self.messages.push(msg);
    }

    pub fn set_user_list(&mut self, users: Vec<UserSummary>) {
        self.users = users;
    }

    pub fn add_user(&mut self, user: UserSummary) {
        if !self.users.iter().any(|u| u.user_id == user.user_id) {
            self.users.push(user);
        }
    }

    pub fn remove_user(&mut self, user_id: &str) {
        self.users.retain(|u| u.user_id != user_id);
    }
}
