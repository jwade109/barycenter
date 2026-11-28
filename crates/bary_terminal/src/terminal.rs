use std::collections::{BTreeMap, VecDeque};

use bary_input::InputState;

#[derive(Debug)]
pub enum ParseError {
    BadKey,
    BadValue,
    WrongArgumentCount,
    CommandNotFound,
    NotImplemented,
}

pub type ArgsMap = BTreeMap<String, String>;

pub struct Command<T> {
    pub entrypoint: String,
    pub params: Vec<String>,
    pub func: Box<dyn Fn(&ArgsMap) -> Result<T, ParseError>>,
}

impl<T> Command<T> {
    pub fn new(
        entrypoint: impl Into<String>,
        params: Vec<&'static str>,
        f: impl Fn(&ArgsMap) -> Result<T, ParseError> + 'static,
    ) -> Self {
        Self {
            entrypoint: entrypoint.into(),
            params: params.iter().map(|s| s.to_string()).collect(),
            func: Box::new(f),
        }
    }

    pub fn to_suggestion(&self) -> String {
        let mut ret = self.entrypoint.clone();
        for param in &self.params {
            ret += &format!(" [{}]", param);
        }
        ret
    }

    pub fn parse_partial_args(&self, args: &[String]) -> Vec<(Option<String>, Option<String>)> {
        let mut ret = Vec::new();

        for i in 0..self.params.len().max(args.len()) {
            let p = self.params.get(i);
            let a = args.get(i);
            ret.push((p.cloned(), a.cloned()));
        }
        ret
    }

    pub fn parse_complete_args(&self, args: &[String]) -> Option<ArgsMap> {
        let mut ret = ArgsMap::new();
        for (i, param) in self.params.iter().enumerate() {
            if param.starts_with("?") {
                if let Some(arg) = args.get(i) {
                    ret.insert(param.clone(), arg.clone());
                }
            } else {
                let arg = args.get(i)?;
                ret.insert(param.clone(), arg.clone());
            };
        }
        Some(ret)
    }

    pub fn parse(&self, args: &[String]) -> Result<T, ParseError> {
        let args = self
            .parse_complete_args(args)
            .ok_or(ParseError::WrongArgumentCount)?;
        (self.func)(&args)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Terminal,
    Command,
}

pub struct Terminal<T: std::fmt::Debug> {
    contents: String,
    is_active: bool,
    lines: VecDeque<(String, LogLevel)>,
    history_index: Option<usize>,
    command_history: VecDeque<String>,
    suggest_text: String,
    commands: Vec<Command<T>>,
    log_level: LogLevel,
    font_size: u16,
}

impl<T: std::fmt::Debug> Terminal<T> {
    pub fn new() -> Self {
        let splash_text = vec![
            "#   .    .    . .  ..    ",
            "# .  >#]   .           . ",
            "# .  .     .  . >#]   . .",
            "#         ___ ___ ___    ",
            "#  .    >[0%0|0%0|000]   ",
            "#            .    . ..   ",
            "#    >|=@=|<        .    ",
            "#  .    .      .      .  ",
            "#   .  .  .       >#]   .",
            "#",
            "# Welcome to the Barycenter Shell (bsh).",
            "# This provides a relatively rich interface",
            "# for interacting with the game via console commands.",
            "#",
            "# Run `help` for help.",
            "#",
            "# Run `clear` to clear all printouts.",
            "#",
            "# Hit ESC to leave this command prompt.",
            "#",
            "# Press CTRL+C to exit this application.",
            "#",
        ];

        Self {
            contents: String::new(),
            is_active: false,
            lines: splash_text
                .iter()
                .rev()
                .map(|s| (s.to_string(), LogLevel::Info))
                .collect(),
            history_index: None,
            command_history: vec!["help".into(), "client.req.blob.all".into()].into(),
            suggest_text: String::new(),
            commands: Vec::new(),
            log_level: LogLevel::Info,
            font_size: 30,
        }
    }

    pub fn register_commands(&mut self, commands: Vec<Command<T>>) {
        self.commands.extend(commands);
    }

    pub fn register_command(&mut self, cmd: Command<T>) {
        self.commands.push(cmd);
    }

    pub fn commands(&self) -> impl Iterator<Item = &Command<T>> {
        self.commands.iter()
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn font_size(&self) -> u16 {
        self.font_size
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn lines(&self) -> impl Iterator<Item = &(String, LogLevel)> {
        self.lines.iter()
    }

    pub fn zoom_out(&mut self) {
        if self.font_size > 7 {
            self.font_size -= 2;
        }
    }

    pub fn zoom_in(&mut self) {
        self.font_size += 2;
    }

    pub fn on_arrow_up(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        if let Some(idx) = &mut self.history_index {
            if *idx + 1 < self.command_history.len() {
                *idx += 1;
            }
        } else {
            self.history_index = Some(0);
        }

        if let Some(h) = self
            .history_index
            .map(|i| self.command_history.get(i))
            .unwrap_or_default()
        {
            self.contents = h.clone();
        }

        self.update_suggest_text();
    }

    pub fn on_arrow_down(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        if let Some(idx) = &mut self.history_index {
            if *idx > 0 {
                *idx -= 1;
            } else {
                self.history_index = None;
            }
        }

        if let Some(h) = self
            .history_index
            .and_then(|i| self.command_history.get(i))
        {
            self.contents = h.clone();
        } else {
            self.contents.clear();
        }

        if let Some(h) = self
            .history_index
            .map(|i| self.command_history.get(i))
            .unwrap_or_default()
        {
            self.contents = h.clone();
        }

        self.update_suggest_text();
    }

    pub fn handle_input(&mut self, input: &InputState) -> Vec<T> {
        let mut ret = Vec::new();

        let left_ctrl = input.is_key_pressed(rdev::Key::ControlLeft);

        if input.just_pressed_debounced(rdev::Key::Equal) && left_ctrl {
            self.zoom_in();
        }

        if input.just_pressed_debounced(rdev::Key::Minus) && left_ctrl {
            self.zoom_out();
        }

        for event in input.events() {
            if let rdev::EventType::KeyPress(k) = &event.event_type {
                match k {
                    rdev::Key::Backspace => self.on_backspace(),
                    rdev::Key::Return => {
                        if let Some(event) = self.on_enter() {
                            ret.push(event);
                        }
                    }
                    rdev::Key::BackQuote => self.focus(),
                    rdev::Key::Escape => self.dismiss(),
                    rdev::Key::Tab => self.on_tab_complete(),
                    rdev::Key::UpArrow => self.on_arrow_up(),
                    rdev::Key::DownArrow => self.on_arrow_down(),
                    _ => {
                        if let Some(n) = &event.name
                            && n.is_ascii() {
                                self.append(n);
                            }
                    }
                }
            }

            if let rdev::EventType::Wheel {
                delta_x: _,
                delta_y,
            } = &event.event_type
            {
                if left_ctrl && *delta_y > 0 {
                    self.zoom_in();
                }
                if left_ctrl && *delta_y < 0 {
                    self.zoom_out();
                }
            }
        }

        ret
    }

    pub fn toggle_debug_mode(&mut self) {
        self.log_level = match self.log_level {
            LogLevel::Debug => LogLevel::Info,
            LogLevel::Info => LogLevel::Debug,
            _ => self.log_level,
        };
        self.log_terminal(format!("Set log level to {:?}", self.log_level));
    }

    pub fn on_backspace(&mut self) {
        if !self.is_active {
            return;
        }
        self.contents.pop();
        self.update_suggest_text();
    }

    fn on_enter(&mut self) -> Option<T> {
        if !self.is_active {
            return None;
        }
        if self.contents.is_empty() {
            return None;
        }

        if self.command_history.front() != Some(&self.contents) {
            self.command_history.push_front(self.contents.clone());
        }

        self.history_index = None;

        self.push_log("bsh > ".to_string() + &self.contents, LogLevel::Command);

        let mut ret = None;

        match self.find_best_command(true) {
            Ok(cmd) => match cmd.parse(&self.get_args()) {
                Ok(action) => {
                    ret = Some(action);
                }
                Err(e) => {
                    self.log_error(format!("Failed to parse: {e:?}"));
                }
            },
            Err(s) => {
                self.log_error(format!("Bad command: \"{}\"", s));
            }
        }

        self.contents.clear();
        self.update_suggest_text();

        ret
    }

    pub fn append(&mut self, s: &str) {
        if !self.is_active {
            return;
        }
        self.contents += s;
        self.update_suggest_text();
    }

    pub fn on_tab_complete(&mut self) {
        let tokens = shellwords::split(&self.contents).unwrap_or_default();
        let command = tokens.first().cloned().unwrap_or_default();
        if command.is_empty() {
            return;
        }

        let possible_commands: Vec<_> = self
            .commands
            .iter()
            .filter(|c| c.entrypoint.starts_with(&command))
            .map(|c| c.entrypoint.clone())
            .collect();

        if possible_commands.is_empty() {
            return;
        }

        if possible_commands.len() == 1 {
            self.contents = possible_commands[0].clone();
            return;
        }

        let longest_common_prefix = |a: &str, b: &str| -> usize {
            let mut len = 0;
            for (a, b) in a.chars().zip(b.chars()) {
                if a != b {
                    break;
                } else {
                    len += 1;
                }
            }
            len
        };

        let mut len = longest_common_prefix(&possible_commands[0], &possible_commands[1]);

        for cmds in possible_commands.windows(2) {
            let l = longest_common_prefix(&cmds[0], &cmds[1]);
            len = len.min(l);
        }

        self.contents = possible_commands[0][..len].to_string();
    }

    pub fn focus(&mut self) {
        self.is_active = true;
        self.update_suggest_text();
    }

    pub fn is_focused(&self) -> bool {
        self.is_active
    }

    pub fn dismiss(&mut self) {
        self.contents.clear();
        self.is_active = false;
    }

    fn find_best_command(&self, exact_match: bool) -> Result<&Command<T>, String> {
        if self.contents.is_empty() {
            return Err("".to_string());
        }
        if let Ok(s) = shellwords::split(&self.contents) {
            let Some(ep) = s.into_iter().nth(0) else {
                return Err("".to_string());
            };
            // TODO(feature) edit distance?
            for cmd in &self.commands {
                if (!exact_match && cmd.entrypoint.starts_with(&ep))
                    || (exact_match && cmd.entrypoint == ep)
                {
                    return Ok(cmd);
                }
            }
            return Err(ep);
        }
        Err("".to_string())
    }

    fn get_args(&self) -> Vec<String> {
        let mut tokens = shellwords::split(&self.contents).unwrap_or_default();
        if !tokens.is_empty() {
            tokens.remove(0);
        }
        tokens
    }

    pub fn update_suggest_text(&mut self) {
        let tokens = shellwords::split(&self.contents).unwrap_or_default();
        let command = tokens.first().cloned().unwrap_or_default();

        // find commands that can start with current text
        let cmds: Vec<String> = self
            .commands
            .iter()
            .filter(|c| c.entrypoint.starts_with(&command))
            .map(|c| c.entrypoint.clone())
            .collect();

        let only_one_command = cmds.len() == 1;
        let cmds = cmds.join(" ");

        if let Ok(cmd) = self.find_best_command(false)
            && only_one_command
        {
            self.suggest_text = cmd.to_suggestion();
        } else {
            self.suggest_text = cmds;
        }
    }

    fn push_log(&mut self, s: impl Into<String>, level: LogLevel) {
        if self.log_level > level {
            return;
        }
        let s = s.into();
        self.lines.push_front((s, level));
        if self.lines.len() > 1000 {
            self.lines.pop_back();
        }
    }

    pub fn log_debug(&mut self, s: impl Into<String>) {
        self.push_log(s, LogLevel::Debug);
    }

    pub fn log_info(&mut self, s: impl Into<String>) {
        self.push_log(s, LogLevel::Info);
    }

    pub fn log_warn(&mut self, s: impl Into<String>) {
        self.push_log(s, LogLevel::Warning);
    }

    pub fn log_error(&mut self, s: impl Into<String>) {
        self.push_log(s, LogLevel::Error);
    }

    pub fn log_terminal(&mut self, s: impl Into<String>) {
        self.push_log(s, LogLevel::Terminal);
    }

    pub fn display_text(&self) -> Vec<(char, bool)> {
        let mut ret = Vec::new();
        let args = self.get_args();
        if !args.is_empty()
            && let Ok(cmd) = self.find_best_command(false)
        {
            for c in cmd.entrypoint.chars() {
                ret.push((c, true));
            }
            let args = cmd.parse_partial_args(&args);
            for (param, arg) in args {
                ret.push((' ', false));
                let param = param.unwrap_or("???".to_string());
                let arg = arg.unwrap_or("_".to_string());
                for c in param.chars() {
                    ret.push((c, false));
                }
                ret.push((':', false));
                ret.push((' ', false));
                for c in arg.chars() {
                    ret.push((c, true));
                }
            }
        } else {
            for i in 0..(self.contents.len().max(self.suggest_text.len())) {
                if let Some(c) = self.contents.chars().nth(i) {
                    ret.push((c, true));
                } else if let Some(c) = self.suggest_text.chars().nth(i) {
                    ret.push((c, false));
                }
            }
        }
        ret
    }

    pub fn print_help_command(&mut self) {
        let mut current_line = String::new();
        let mut lines: Vec<String> = Vec::new();
        for cmd in self.commands() {
            // lines.push(cmd.entrypoint.clone());
            current_line += &format!("{}   ", cmd.entrypoint);
            if current_line.len() > 80 {
                lines.push(std::mem::take(&mut current_line));
            }
        }
        for line in lines {
            self.log_info(line);
        }
    }
}

impl<T: std::fmt::Debug> Default for Terminal<T> {
    fn default() -> Self {
        Self::new()
    }
}
