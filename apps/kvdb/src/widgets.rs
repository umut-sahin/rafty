use {
    crate::storage::Storage,
    crossterm::event::{
        Event,
        KeyCode as Key,
    },
    rafty_debugger::*,
    rafty_kvdb::{
        Command,
        KeyValueDatabase,
        Query,
    },
    ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{
            Color,
            Style,
            Stylize,
        },
        symbols::scrollbar::Set as ScrollbarSet,
        text::{
            Line,
            Span,
        },
        widgets::{
            Block,
            BorderType,
            Borders,
            List,
            ListState,
            Padding,
            Paragraph,
            Scrollbar,
            ScrollbarOrientation,
            ScrollbarState,
            StatefulWidget,
            Widget,
        },
    },
};


pub enum CommandSelectionWidget {
    SelectingCommand { selection: usize },

    EnteringKeyToInsert { key: String },
    EnteringKeyToUpsert { key: String },
    EnteringKeyToClear { key: String },

    EnteringValueToInsert { key: String, value: String },
    EnteringValueToUpsert { key: String, value: String },

    Finalized { command: Command },
}

impl CommandSelectionWidget {
    const COMMANDS: &'static [&'static str] = &["Insert", "Upsert", "Clear"];
}

impl Default for CommandSelectionWidget {
    fn default() -> Self {
        CommandSelectionWidget::SelectingCommand { selection: 0 }
    }
}

impl CommandWidget<KeyValueDatabase<Storage>> for CommandSelectionWidget {
    fn on_user_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match self {
                CommandSelectionWidget::SelectingCommand { selection } => {
                    match event.code {
                        Key::Enter => {
                            match CommandSelectionWidget::COMMANDS[*selection] {
                                "Insert" => {
                                    *self = CommandSelectionWidget::EnteringKeyToInsert {
                                        key: "".to_string(),
                                    };
                                },
                                "Upsert" => {
                                    *self = CommandSelectionWidget::EnteringKeyToUpsert {
                                        key: "".to_string(),
                                    };
                                },
                                "Clear" => {
                                    *self = CommandSelectionWidget::EnteringKeyToClear {
                                        key: "".to_string(),
                                    };
                                },
                                _ => unreachable!(),
                            }
                        },

                        Key::Up => {
                            if *selection == 0 {
                                *selection = CommandSelectionWidget::COMMANDS.len() - 1;
                            } else {
                                *selection -= 1;
                            }
                        },
                        Key::Down => {
                            if *selection == CommandSelectionWidget::COMMANDS.len() - 1 {
                                *selection = 0;
                            } else {
                                *selection += 1;
                            }
                        },

                        Key::Char('1') => {
                            *self =
                                CommandSelectionWidget::EnteringKeyToInsert { key: "".to_string() };
                        },
                        Key::Char('2') => {
                            *self =
                                CommandSelectionWidget::EnteringKeyToUpsert { key: "".to_string() };
                        },
                        Key::Char('3') => {
                            *self =
                                CommandSelectionWidget::EnteringKeyToClear { key: "".to_string() };
                        },

                        _ => {},
                    }
                },

                CommandSelectionWidget::EnteringKeyToInsert { key: input }
                | CommandSelectionWidget::EnteringKeyToUpsert { key: input }
                | CommandSelectionWidget::EnteringKeyToClear { key: input }
                | CommandSelectionWidget::EnteringValueToInsert { value: input, .. }
                | CommandSelectionWidget::EnteringValueToUpsert { value: input, .. } => {
                    match event.code {
                        Key::Char(char) => {
                            input.push(char);
                        },
                        Key::Backspace => {
                            input.pop();
                        },
                        Key::Enter => {
                            *self = match self {
                                CommandSelectionWidget::EnteringKeyToInsert { key } => {
                                    CommandSelectionWidget::EnteringValueToInsert {
                                        key: std::mem::take(key),
                                        value: String::new(),
                                    }
                                },
                                CommandSelectionWidget::EnteringKeyToUpsert { key } => {
                                    CommandSelectionWidget::EnteringValueToUpsert {
                                        key: std::mem::take(key),
                                        value: String::new(),
                                    }
                                },
                                CommandSelectionWidget::EnteringKeyToClear { key } => {
                                    CommandSelectionWidget::Finalized {
                                        command: Command::Clear { key: std::mem::take(key) },
                                    }
                                },

                                CommandSelectionWidget::EnteringValueToInsert { key, value } => {
                                    CommandSelectionWidget::Finalized {
                                        command: Command::Insert {
                                            key: std::mem::take(key),
                                            value: std::mem::take(value),
                                        },
                                    }
                                },
                                CommandSelectionWidget::EnteringValueToUpsert { key, value } => {
                                    CommandSelectionWidget::Finalized {
                                        command: Command::Upsert {
                                            key: std::mem::take(key),
                                            value: std::mem::take(value),
                                        },
                                    }
                                },

                                _ => unreachable!(),
                            };
                        },

                        _ => {},
                    }
                },

                CommandSelectionWidget::Finalized { .. } => unreachable!(),
            }
        }
    }

    fn back(&self) -> Option<Self> {
        match self {
            CommandSelectionWidget::SelectingCommand { .. } => None,

            CommandSelectionWidget::EnteringKeyToInsert { .. } => {
                Some(CommandSelectionWidget::SelectingCommand { selection: 0 })
            },
            CommandSelectionWidget::EnteringKeyToUpsert { .. } => {
                Some(CommandSelectionWidget::SelectingCommand { selection: 1 })
            },
            CommandSelectionWidget::EnteringKeyToClear { .. } => {
                Some(CommandSelectionWidget::SelectingCommand { selection: 2 })
            },

            CommandSelectionWidget::EnteringValueToInsert { key, .. } => {
                Some(CommandSelectionWidget::EnteringKeyToInsert { key: key.clone() })
            },
            CommandSelectionWidget::EnteringValueToUpsert { key, .. } => {
                Some(CommandSelectionWidget::EnteringKeyToUpsert { key: key.clone() })
            },

            CommandSelectionWidget::Finalized { .. } => unreachable!(),
        }
    }

    fn renderer(&self) -> impl Widget {
        CommandSelectionWidgetRendered { widget: self }
    }

    fn finalize(&mut self) -> Option<Command> {
        if let CommandSelectionWidget::Finalized { command } = self {
            Some(std::mem::replace(command, Command::NoOp))
        } else {
            None
        }
    }
}

struct CommandSelectionWidgetRendered<'debugger> {
    widget: &'debugger CommandSelectionWidget,
}

impl<'debugger> Widget for CommandSelectionWidgetRendered<'debugger> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        match self.widget {
            CommandSelectionWidget::SelectingCommand { selection } => {
                let action_list = List::new(
                    CommandSelectionWidget::COMMANDS.iter().enumerate().map(|(index, command)| {
                        let mut style = Style::default();
                        if index == *selection {
                            style = style.reversed();
                        }

                        let spans = vec![
                            Span::styled(format!("<{}> ", index + 1), Style::default().magenta()),
                            Span::styled(*command, style),
                        ];
                        Line::from(spans)
                    }),
                )
                .block(
                    Block::bordered()
                        .borders(Borders::ALL)
                        .padding(Padding::left(1))
                        .title(" Sending Command... ")
                        .title_style(Style::default().fg(Color::Green))
                        .border_type(BorderType::Rounded),
                );
                let mut action_list_state = ListState::default().with_selected(Some(*selection));
                let mut vertical_scroll_state =
                    ScrollbarState::new(action_list.len()).position(*selection);

                StatefulWidget::render(action_list, area, buffer, &mut action_list_state);
                StatefulWidget::render(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(ScrollbarSet {
                        track: "│",
                        thumb: "║",
                        begin: "╮",
                        end: "╯",
                    }),
                    area,
                    buffer,
                    &mut vertical_scroll_state,
                );
            },

            CommandSelectionWidget::EnteringKeyToInsert { key: content }
            | CommandSelectionWidget::EnteringKeyToUpsert { key: content }
            | CommandSelectionWidget::EnteringKeyToClear { key: content }
            | CommandSelectionWidget::EnteringValueToInsert { value: content, .. }
            | CommandSelectionWidget::EnteringValueToUpsert { value: content, .. } => {
                Paragraph::new({
                    let spans = vec![
                        Span::styled(
                            match self.widget {
                                CommandSelectionWidget::EnteringKeyToInsert { .. }
                                | CommandSelectionWidget::EnteringKeyToUpsert { .. }
                                | CommandSelectionWidget::EnteringKeyToClear { .. } => "Key: ",

                                CommandSelectionWidget::EnteringValueToInsert { .. }
                                | CommandSelectionWidget::EnteringValueToUpsert { .. } => "Value: ",

                                _ => unreachable!(),
                            },
                            Style::default().magenta(),
                        ),
                        Span::raw(content.as_str()),
                        Span::raw("█"),
                    ];
                    Line::from(spans)
                })
                .block(
                    Block::bordered()
                        .borders(Borders::ALL)
                        .padding(Padding::left(1))
                        .title(match self.widget {
                            CommandSelectionWidget::EnteringKeyToInsert { .. }
                            | CommandSelectionWidget::EnteringValueToInsert { .. } => {
                                " Commanding Insert... "
                            },
                            CommandSelectionWidget::EnteringKeyToUpsert { .. }
                            | CommandSelectionWidget::EnteringValueToUpsert { .. } => {
                                " Commanding Upsert... "
                            },
                            CommandSelectionWidget::EnteringKeyToClear { .. } => {
                                " Commanding Clear... "
                            },
                            _ => unreachable!(),
                        })
                        .title_style(Style::default().fg(Color::Green))
                        .border_type(BorderType::Rounded),
                )
                .render(area, buffer);
            },

            CommandSelectionWidget::Finalized { .. } => unreachable!(),
        }
    }
}


pub enum QuerySelectionWidget {
    SelectingQuery { selection: usize },

    EnteringKeyToEntry { key: String },

    Finalized { query: Query },
}

impl QuerySelectionWidget {
    const QUERIES: &'static [&'static str] = &["Length", "Entry"];
}

impl Default for QuerySelectionWidget {
    fn default() -> Self {
        QuerySelectionWidget::SelectingQuery { selection: 0 }
    }
}

impl QueryWidget<KeyValueDatabase<Storage>> for QuerySelectionWidget {
    fn on_user_event(&mut self, event: Event) {
        if let Event::Key(event) = event {
            match self {
                QuerySelectionWidget::SelectingQuery { selection } => {
                    match event.code {
                        Key::Enter => {
                            match QuerySelectionWidget::QUERIES[*selection] {
                                "Length" => {
                                    *self =
                                        QuerySelectionWidget::Finalized { query: Query::Length };
                                },
                                "Entry" => {
                                    *self = QuerySelectionWidget::EnteringKeyToEntry {
                                        key: "".to_string(),
                                    };
                                },
                                _ => unreachable!(),
                            }
                        },

                        Key::Up => {
                            if *selection == 0 {
                                *selection = QuerySelectionWidget::QUERIES.len() - 1;
                            } else {
                                *selection -= 1;
                            }
                        },
                        Key::Down => {
                            if *selection == QuerySelectionWidget::QUERIES.len() - 1 {
                                *selection = 0;
                            } else {
                                *selection += 1;
                            }
                        },

                        Key::Char('1') => {
                            *self = QuerySelectionWidget::Finalized { query: Query::Length };
                        },
                        Key::Char('2') => {
                            *self =
                                QuerySelectionWidget::EnteringKeyToEntry { key: "".to_string() };
                        },

                        _ => {},
                    }
                },

                QuerySelectionWidget::EnteringKeyToEntry { key } => {
                    match event.code {
                        Key::Char(char) => {
                            key.push(char);
                        },
                        Key::Backspace => {
                            key.pop();
                        },
                        Key::Enter => {
                            *self = QuerySelectionWidget::Finalized {
                                query: Query::Entry { key: std::mem::take(key) },
                            };
                        },

                        _ => {},
                    }
                },

                QuerySelectionWidget::Finalized { .. } => unreachable!(),
            }
        }
    }

    fn back(&self) -> Option<Self> {
        match self {
            QuerySelectionWidget::SelectingQuery { .. } => None,

            QuerySelectionWidget::EnteringKeyToEntry { .. } => {
                Some(QuerySelectionWidget::SelectingQuery { selection: 1 })
            },

            QuerySelectionWidget::Finalized { .. } => unreachable!(),
        }
    }

    fn renderer(&self) -> impl Widget {
        QuerySelectionWidgetRendered { widget: self }
    }

    fn finalize(&mut self) -> Option<Query> {
        if let QuerySelectionWidget::Finalized { query } = self {
            Some(std::mem::replace(query, Query::Length))
        } else {
            None
        }
    }
}

struct QuerySelectionWidgetRendered<'debugger> {
    widget: &'debugger QuerySelectionWidget,
}

impl<'debugger> Widget for QuerySelectionWidgetRendered<'debugger> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        match self.widget {
            QuerySelectionWidget::SelectingQuery { selection } => {
                let action_list = List::new(QuerySelectionWidget::QUERIES.iter().enumerate().map(
                    |(index, command)| {
                        let mut style = Style::default();
                        if index == *selection {
                            style = style.reversed();
                        }

                        let spans = vec![
                            Span::styled(format!("<{}> ", index + 1), Style::default().magenta()),
                            Span::styled(*command, style),
                        ];
                        Line::from(spans)
                    },
                ))
                .block(
                    Block::bordered()
                        .borders(Borders::ALL)
                        .padding(Padding::left(1))
                        .title(" Querying... ")
                        .title_style(Style::default().fg(Color::Green))
                        .border_type(BorderType::Rounded),
                );
                let mut action_list_state = ListState::default().with_selected(Some(*selection));
                let mut vertical_scroll_state =
                    ScrollbarState::new(action_list.len()).position(*selection);

                StatefulWidget::render(action_list, area, buffer, &mut action_list_state);
                StatefulWidget::render(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(ScrollbarSet {
                        track: "│",
                        thumb: "║",
                        begin: "╮",
                        end: "╯",
                    }),
                    area,
                    buffer,
                    &mut vertical_scroll_state,
                );
            },

            QuerySelectionWidget::EnteringKeyToEntry { key } => {
                Paragraph::new({
                    let spans = vec![
                        Span::styled("Key: ", Style::default().magenta()),
                        Span::raw(key.as_str()),
                        Span::raw("█"),
                    ];
                    Line::from(spans)
                })
                .block(
                    Block::bordered()
                        .borders(Borders::ALL)
                        .padding(Padding::left(1))
                        .title(" Querying Entry... ")
                        .title_style(Style::default().fg(Color::Green))
                        .border_type(BorderType::Rounded),
                )
                .render(area, buffer);
            },

            QuerySelectionWidget::Finalized { .. } => unreachable!(),
        }
    }
}
