use crate::*;

pub struct LogsWidget {
    pub logger_state: LoggerState,
}

impl LogsWidget {
    pub fn process_event(&mut self, event: &Event) {
        if let Event::Mouse(event) = event {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    self.logger_state.transition(LoggerEvent::PrevPageKey);
                },
                MouseEventKind::ScrollDown => {
                    self.logger_state.transition(LoggerEvent::NextPageKey);
                },

                _ => {},
            }
        }
    }
}

impl Default for LogsWidget {
    fn default() -> Self {
        let logger_state = LoggerState::new().set_default_display_level(LevelFilter::Debug);
        logger_state.transition(LoggerEvent::HideKey);
        Self { logger_state }
    }
}

impl Widget for &mut LogsWidget {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        LoggerWidget::default()
            .state(&self.logger_state)
            .formatter(Box::new(Formatter))
            .block(
                Block::default()
                    .title(" Logs ")
                    .title_style(Style::default().fg(Color::Green))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .render(area, buffer);
    }
}

struct Formatter;

impl LogFormatter for Formatter {
    fn min_width(&self) -> u16 {
        30
    }

    fn format(&self, width: usize, record: &ExtLogRecord) -> Vec<Line<'_>> {
        if record.msg().is_empty() {
            return Vec::new();
        }

        let mut spans = vec![Span::styled(
            record.timestamp.format("[%H:%M:%S%.3f] ").to_string(),
            match record.level {
                Level::Error => Style::default().fg(Color::Red),
                Level::Warn => Style::default().fg(Color::Yellow),
                Level::Info => Style::default().fg(Color::Green),
                Level::Debug => Style::default().fg(Color::Cyan),
                Level::Trace => Style::default().fg(Color::Magenta),
            },
        )];

        let mut prefix = String::new();
        let mut prefix_is_complete = false;

        let mut chars = record.msg().chars();

        let first_char = chars.next().unwrap();
        prefix.push(first_char);

        if matches!(first_char, '(' | '|' | '<') {
            for char in chars.by_ref() {
                prefix.push(char);
                if matches!(char, ')' | '|' | '>') {
                    prefix_is_complete = true;
                    break;
                }
            }
        }

        let message = if prefix_is_complete {
            let prefix_style = match prefix.chars().next().unwrap() {
                '(' => Style::default().fg(Color::Cyan),
                '|' => Style::default().fg(Color::Magenta),
                '<' => Style::default().fg(Color::Yellow),
                _ => unreachable!(),
            };
            spans.push(Span::styled(prefix, prefix_style));
            chars.as_str().to_owned()
        } else {
            prefix += chars.as_str();
            prefix
        };
        let header = spans.iter().map(|span| span.content.chars().count()).sum::<usize>() + 1;

        let splitter = Splitter {
            spans,
            width,
            header,
            message: message.chars(),
            word: String::new(),
            line: String::new(),
            first: true,
        };
        splitter.collect()
    }
}

struct Splitter<'a, 'b> {
    spans: Vec<Span<'a>>,
    width: usize,
    header: usize,
    message: Chars<'b>,
    word: String,
    line: String,
    first: bool,
}

impl<'a> Iterator for Splitter<'a, '_> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut budget = self.width - self.header;
        while budget > 0 {
            match self.message.next() {
                Some(' ') => {
                    self.line += &self.word;
                    self.line.push(' ');
                    self.word.clear();
                    budget -= 1;
                },
                Some('\n') => {
                    self.line += &self.word;
                    self.word.clear();
                    break;
                },
                Some(char) => {
                    self.word.push(char);
                    budget -= 1;
                },
                None => {
                    if self.word.is_empty() && self.line.is_empty() {
                        return None;
                    }

                    self.line += &self.word;
                    self.word.clear();

                    break;
                },
            }
        }

        if self.first {
            self.first = false;
            if !self.line.is_empty() {
                self.spans.push(Span::raw(std::mem::take(&mut self.line)));
            }
            return Some(Line::from(std::mem::take(&mut self.spans)));
        }

        self.spans.push(Span::raw(" ".repeat(self.header)));
        if !self.line.is_empty() {
            self.spans.push(Span::raw(std::mem::take(&mut self.line)));
        }
        Some(Line::from(std::mem::take(&mut self.spans)))
    }
}
