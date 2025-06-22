use crate::*;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MainTabSelection(PeerId);

impl MainTabSelection {
    pub fn peer_id(&self) -> PeerId {
        self.0
    }
}

impl MainTabSelection {
    fn go_left<A: RaftApplication>(
        &mut self,
        details_tab_selection: &mut DetailsTabSelection,
        simulation: &Simulation<A>,
    ) {
        let number_of_tabs = simulation.number_of_peers();

        let tab_index = self.tab_index();
        let new_tab_index = if tab_index == 0 { number_of_tabs - 1 } else { tab_index - 1 };

        self.set_from_tab_index(details_tab_selection, simulation, new_tab_index);
    }

    fn go_right<A: RaftApplication>(
        &mut self,
        details_tab_selection: &mut DetailsTabSelection,
        simulation: &Simulation<A>,
    ) {
        let number_of_tabs = simulation.number_of_peers();

        let tab_index = self.tab_index();
        let new_tab_index = (tab_index + 1) % number_of_tabs;

        self.set_from_tab_index(details_tab_selection, simulation, new_tab_index);
    }

    fn set_from_tab_index<A: RaftApplication>(
        &mut self,
        details_tab_selection: &mut DetailsTabSelection,
        simulation: &Simulation<A>,
        new_tab_index: usize,
    ) {
        let number_of_tabs = simulation.number_of_peers();
        if new_tab_index >= number_of_tabs {
            return;
        }

        let new_peer_id = PeerId(new_tab_index + 1);
        details_tab_selection.reset(simulation.peer(new_peer_id));

        *self = MainTabSelection(PeerId(new_tab_index + 1));
    }
}

impl MainTabSelection {
    pub fn tab_index(&self) -> usize {
        self.0 .0 - 1
    }
}

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum DetailsTabSelection {
    Log { selected: Option<usize> },
    Machine { vertical_scroll: usize, horizontal_scroll: usize },
    Snapshot { machine_vertical_scroll: usize, machine_horizontal_scroll: usize },
}

impl DetailsTabSelection {
    fn last_log<A: RaftApplication>(peer: &Peer<A>) -> Self {
        DetailsTabSelection::Log {
            selected: if peer.log().is_empty() { None } else { Some(peer.log().len() - 1) },
        }
    }

    fn reset<A: RaftApplication>(&mut self, new_peer: &Peer<A>) {
        match self {
            DetailsTabSelection::Log { .. } => {
                *self = DetailsTabSelection::last_log(new_peer);
            },
            DetailsTabSelection::Machine { vertical_scroll, horizontal_scroll } => {
                *vertical_scroll = 0;
                *horizontal_scroll = 0;
            },
            DetailsTabSelection::Snapshot {
                machine_vertical_scroll,
                machine_horizontal_scroll,
            } => {
                *machine_vertical_scroll = 0;
                *machine_horizontal_scroll = 0;
            },
        }
    }

    fn go_up<A: RaftApplication>(&mut self, simulation: &Simulation<A>, peer_id: PeerId) {
        match self {
            DetailsTabSelection::Log { selected } => {
                if let Some(selected) = selected {
                    if *selected == 0 {
                        let peer = simulation.peer(peer_id);
                        *selected = peer.log().len() - 1;
                    } else {
                        *selected = selected.saturating_sub(1);
                    }
                }
            },
            DetailsTabSelection::Machine { vertical_scroll, .. } => {
                *vertical_scroll = vertical_scroll.saturating_sub(1);
            },
            DetailsTabSelection::Snapshot { machine_vertical_scroll, .. } => {
                *machine_vertical_scroll = machine_vertical_scroll.saturating_sub(1);
            },
        }
    }

    fn go_down<A: RaftApplication>(&mut self, simulation: &Simulation<A>, peer_id: PeerId) {
        match self {
            DetailsTabSelection::Log { selected } => {
                if let Some(selected) = selected {
                    let peer = simulation.peer(peer_id);
                    if *selected == peer.log().len() - 1 {
                        *selected = 0;
                    } else {
                        *selected += 1;
                    }
                }
            },
            DetailsTabSelection::Machine { vertical_scroll, .. } => {
                *vertical_scroll += 1;
            },
            DetailsTabSelection::Snapshot { machine_vertical_scroll, .. } => {
                *machine_vertical_scroll += 1;
            },
        }
    }

    fn go_left(&mut self) {
        match self {
            DetailsTabSelection::Log { .. } => {},
            DetailsTabSelection::Machine { horizontal_scroll, .. } => {
                *horizontal_scroll = horizontal_scroll.saturating_sub(1);
            },
            DetailsTabSelection::Snapshot { machine_horizontal_scroll, .. } => {
                *machine_horizontal_scroll = machine_horizontal_scroll.saturating_sub(1);
            },
        }
    }

    fn go_right(&mut self) {
        match self {
            DetailsTabSelection::Log { .. } => {},
            DetailsTabSelection::Machine { horizontal_scroll, .. } => {
                *horizontal_scroll += 1;
            },
            DetailsTabSelection::Snapshot { machine_horizontal_scroll, .. } => {
                *machine_horizontal_scroll += 1;
            },
        }
    }
}

pub struct InfoWidget {
    pub(crate) main_tabs: Vec<String>,
    pub(crate) main_tab_selection: MainTabSelection,

    pub(crate) details_tabs: Vec<String>,
    pub(crate) details_tab_selection: DetailsTabSelection,
}

impl InfoWidget {
    pub fn new<A: RaftApplication>(simulation: &Simulation<A>) -> Self {
        Self {
            main_tabs: (1..=simulation.number_of_peers())
                .map(|peer_id| format!("Peer {peer_id}"))
                .collect(),
            main_tab_selection: MainTabSelection(PeerId(1)),

            details_tabs: vec!["Log".to_owned(), "Machine".to_owned(), "Snapshot".to_owned()],
            details_tab_selection: DetailsTabSelection::last_log(simulation.peer(PeerId(1))),
        }
    }
}

impl InfoWidget {
    pub fn process_event<A: RaftApplication>(&mut self, event: &Event, simulation: &Simulation<A>) {
        if let Event::Key(event) = event {
            match event.code {
                Key::Char('w') | Key::Char('W')
                    if event.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.details_tab_selection.go_up(simulation, self.main_tab_selection.peer_id());
                },
                Key::Char('a') | Key::Char('A')
                    if event.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.details_tab_selection.go_left();
                },
                Key::Char('s') | Key::Char('S')
                    if event.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.details_tab_selection
                        .go_down(simulation, self.main_tab_selection.peer_id());
                },
                Key::Char('d') | Key::Char('D')
                    if event.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.details_tab_selection.go_right();
                },

                Key::Left => {
                    self.main_tab_selection.go_left(&mut self.details_tab_selection, simulation);
                },
                Key::Right => {
                    self.main_tab_selection.go_right(&mut self.details_tab_selection, simulation);
                },

                Key::Tab => {
                    match self.details_tab_selection {
                        DetailsTabSelection::Log { .. } => {
                            self.details_tab_selection = DetailsTabSelection::Machine {
                                vertical_scroll: 0,
                                horizontal_scroll: 0,
                            };
                        },
                        DetailsTabSelection::Machine { .. } => {
                            self.details_tab_selection = DetailsTabSelection::Snapshot {
                                machine_vertical_scroll: 0,
                                machine_horizontal_scroll: 0,
                            };
                        },
                        DetailsTabSelection::Snapshot { .. } => {
                            self.details_tab_selection = DetailsTabSelection::last_log(
                                simulation.peer(self.main_tab_selection.peer_id()),
                            );
                        },
                    }
                },

                Key::Char(n @ '1'..='9') => {
                    let index = (n as usize) - ('1' as usize);
                    self.main_tab_selection.set_from_tab_index(
                        &mut self.details_tab_selection,
                        simulation,
                        index,
                    );
                },

                _ => {},
            }
        }
    }
}

impl InfoWidget {
    pub fn renderer<'debugger, A: RaftApplication>(
        &'debugger mut self,
        simulation: &'debugger Simulation<A>,
    ) -> InfoWidgetRenderer<'debugger, A> {
        InfoWidgetRenderer { info_widget: self, simulation }
    }
}

pub struct InfoWidgetRenderer<'debugger, A: RaftApplication> {
    info_widget: &'debugger mut InfoWidget,
    simulation: &'debugger Simulation<A>,
}

impl<'debugger, A: RaftApplication> Widget for &mut InfoWidgetRenderer<'debugger, A> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let block =
            Block::bordered().padding(Padding::new(1, 1, 1, 1)).border_type(BorderType::Rounded);

        let inner_area = block.inner(area);
        match self.info_widget.main_tab_selection {
            MainTabSelection(peer_id) => {
                let mut widget = PeerWidget {
                    info_widget: self.info_widget,
                    simulation: self.simulation,
                    peer_id,
                };
                widget.render(inner_area, buffer);
            },
        }

        block.render(area, buffer);
        Tabs::new(self.info_widget.main_tabs.iter().map(AsRef::as_ref))
            .highlight_style(Style::default().green().bold())
            .select(self.info_widget.main_tab_selection.tab_index())
            .divider(symbols::DOT)
            .padding(" ", " ")
            .render(area.offset(Offset { x: 1, y: 0 }), buffer);
    }
}

pub struct PeerWidget<'debugger, A: RaftApplication> {
    info_widget: &'debugger mut InfoWidget,
    simulation: &'debugger Simulation<A>,
    peer_id: PeerId,
}

impl<'debugger, A: RaftApplication> Widget for &mut PeerWidget<'debugger, A> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let peer = self.simulation.peer(self.peer_id);

        let [others_area, _, role_area] =
            Layout::horizontal([Constraint::Fill(75), Constraint::Length(1), Constraint::Fill(25)])
                .areas(area);
        let [state_area, _, details_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Length(1), Constraint::Fill(100)])
                .areas(others_area);

        let mut state_widget = StateWidget { peer };
        state_widget.render(state_area, buffer);

        let mut details_widget = DetailsWidget { info_widget: self.info_widget, peer };
        details_widget.render(details_area, buffer);

        let mut role_widget = RoleWidget { role: peer.role() };
        role_widget.render(role_area, buffer);
    }
}

pub struct StateWidget<'debugger, A: RaftApplication> {
    peer: &'debugger Peer<A>,
}

impl<'debugger, A: RaftApplication> Widget for &mut StateWidget<'debugger, A> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let [current_term_area, voted_for_area, commit_index_area, last_applied_area] =
            Layout::horizontal([
                Constraint::Length(18),
                Constraint::Length(15),
                Constraint::Length(18),
                Constraint::Length(18),
            ])
            .flex(Flex::Center)
            .areas(area);

        Paragraph::new(format!("{}", self.peer.current_term()))
            .alignment(Alignment::Center)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Current Term ")
                    .title_alignment(Alignment::Center)
                    .title_style(Style::default().fg(Color::Blue)),
            )
            .render(current_term_area, buffer);

        Paragraph::new(match self.peer.voted_for() {
            None => "None".to_owned(),
            Some(voted_for) => {
                format!("Peer {voted_for}")
            },
        })
        .alignment(Alignment::Center)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Voted For ")
                .title_alignment(Alignment::Center)
                .title_style(Style::default().fg(Color::Blue)),
        )
        .render(voted_for_area, buffer);

        Paragraph::new(format!("{}", self.peer.commit_index()))
            .alignment(Alignment::Center)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Commit Index ")
                    .title_alignment(Alignment::Center)
                    .title_style(Style::default().fg(Color::Blue)),
            )
            .render(commit_index_area, buffer);

        Paragraph::new(format!("{}", self.peer.last_applied()))
            .alignment(Alignment::Center)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Last Applied ")
                    .title_alignment(Alignment::Center)
                    .title_style(Style::default().fg(Color::Blue)),
            )
            .render(last_applied_area, buffer);
    }
}

pub struct DetailsWidget<'debugger, A: RaftApplication> {
    info_widget: &'debugger mut InfoWidget,
    peer: &'debugger Peer<A>,
}

impl<'debugger, A: RaftApplication> Widget for &mut DetailsWidget<'debugger, A> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let [area] = Layout::horizontal([Constraint::Length(69)]).flex(Flex::Center).areas(area);

        let block = Block::bordered().padding(Padding::top(1)).borders(Borders::TOP);

        let inner_area = block.inner(area);
        match &mut self.info_widget.details_tab_selection {
            DetailsTabSelection::Log { selected } => {
                let entries = self.peer.log().iter().map(|entry| {
                    let is_applied = entry.index() <= self.peer.last_applied();
                    let is_committed = entry.index() <= self.peer.commit_index();
                    let spans = vec![
                        Span::styled(
                            format!("[{}] ", entry.index()),
                            if is_applied {
                                Style::default().green()
                            } else if is_committed {
                                Style::default().yellow()
                            } else {
                                Style::default().red()
                            },
                        ),
                        Span::styled(format!("({}) ", entry.term()), Style::default().cyan()),
                        Span::raw(format!("{:?}", entry.command())),
                    ];
                    Line::from(spans)
                });
                let entry_list = List::new(entries).highlight_symbol(" • ").block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Entries ")
                        .title_alignment(Alignment::Center)
                        .title_style(Style::default().fg(Color::Blue)),
                );
                let mut entry_list_state = ListState::default().with_selected(*selected);
                let mut vertical_scroll_state =
                    ScrollbarState::new(entry_list.len()).position(selected.unwrap_or(0));

                StatefulWidget::render(entry_list, inner_area, buffer, &mut entry_list_state);
                StatefulWidget::render(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(ScrollbarSet {
                        track: "│",
                        thumb: "║",
                        begin: "╮",
                        end: "╯",
                    }),
                    inner_area,
                    buffer,
                    &mut vertical_scroll_state,
                );
            },
            DetailsTabSelection::Machine { vertical_scroll, horizontal_scroll } => {
                let machine = format!("{:#?}", self.peer.machine());
                let mut scroll_widget = ScrollWidget {
                    block: Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Machine ")
                        .title_alignment(Alignment::Center)
                        .title_style(Style::default().fg(Color::Blue)),
                    content: &machine,
                    vertical_scroll,
                    horizontal_scroll,
                };
                scroll_widget.render(inner_area, buffer);
            },
            DetailsTabSelection::Snapshot {
                machine_vertical_scroll,
                machine_horizontal_scroll,
            } => {
                let [last_included_area, machine_area] =
                    Layout::vertical([Constraint::Length(3), Constraint::Fill(100)])
                        .areas(inner_area);

                let [last_included_index_area, last_included_term_area] =
                    Layout::horizontal([Constraint::Length(34), Constraint::Length(35)])
                        .flex(Flex::Center)
                        .areas(last_included_area);

                let [machine_area] = Layout::horizontal([Constraint::Length(69)])
                    .flex(Flex::Center)
                    .areas(machine_area);

                Paragraph::new(format!("{}", self.peer.snapshot().last_included_index()))
                    .alignment(Alignment::Center)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Last Included Index ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(last_included_index_area, buffer);

                Paragraph::new(format!("{}", self.peer.snapshot().last_included_term()))
                    .alignment(Alignment::Center)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Last Included Term ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(last_included_term_area, buffer);

                let machine = format!("{:#?}", self.peer.snapshot().machine());
                let mut scroll_widget = ScrollWidget {
                    block: Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Machine ")
                        .title_alignment(Alignment::Center)
                        .title_style(Style::default().fg(Color::Blue)),
                    content: &machine,
                    vertical_scroll: machine_vertical_scroll,
                    horizontal_scroll: machine_horizontal_scroll,
                };
                scroll_widget.render(machine_area, buffer);
            },
        }

        block.render(area, buffer);
        Tabs::new(self.info_widget.details_tabs.iter().map(AsRef::as_ref))
            .highlight_style(Style::default().red())
            .select(match self.info_widget.details_tab_selection {
                DetailsTabSelection::Log { .. } => 0,
                DetailsTabSelection::Machine { .. } => 1,
                DetailsTabSelection::Snapshot { .. } => 2,
            })
            .divider("|")
            .padding(" ", " ")
            .render(area, buffer);
    }
}

pub struct RoleWidget<'debugger, A: RaftApplication> {
    role: &'debugger Role<A>,
}

impl<'debugger, A: RaftApplication> Widget for &mut RoleWidget<'debugger, A> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let block = Block::new()
            .borders(Borders::LEFT)
            .padding(Padding::left(1))
            .border_type(BorderType::Rounded);

        let inner_area = block.inner(area);
        match self.role {
            Role::Follower(follower_state) => {
                let [role_area, leader_area] =
                    Layout::vertical([Constraint::Length(3), Constraint::Length(3)])
                        .areas(inner_area);

                let [role_area] = Layout::horizontal([Constraint::Length(20)])
                    .flex(Flex::Center)
                    .areas(role_area);
                let [leader_area] = Layout::horizontal([Constraint::Length(20)])
                    .flex(Flex::Center)
                    .areas(leader_area);

                Paragraph::new("Follower")
                    .alignment(Alignment::Center)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Role ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(role_area, buffer);

                Paragraph::new(match follower_state.leader_id() {
                    Some(leader_id) => format!("Peer {leader_id}"),
                    None => "None".to_owned(),
                })
                .alignment(Alignment::Center)
                .block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .title(" Leader ")
                        .title_alignment(Alignment::Center)
                        .title_style(Style::default().fg(Color::Blue)),
                )
                .render(leader_area, buffer);
            },
            Role::Candidate(candidate_state) => {
                let [role_area, votes_granted_area] =
                    Layout::vertical([Constraint::Length(3), Constraint::Length(3)])
                        .areas(inner_area);

                let [role_area] = Layout::horizontal([Constraint::Length(20)])
                    .flex(Flex::Center)
                    .areas(role_area);
                let [votes_granted_area] = Layout::horizontal([Constraint::Length(20)])
                    .flex(Flex::Center)
                    .areas(votes_granted_area);

                Paragraph::new("Candidate")
                    .alignment(Alignment::Center)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Role ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(role_area, buffer);

                Paragraph::new(candidate_state.votes_granted().to_string())
                    .alignment(Alignment::Center)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Votes Granted ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(votes_granted_area, buffer);
            },
            Role::Leader(leader_state) => {
                let next_index_lines = leader_state
                    .next_index()
                    .iter()
                    .map(|(peer_id, next_index)| format!("Peer {peer_id} -> {next_index}"))
                    .collect::<Vec<_>>();
                let match_index_lines = leader_state
                    .match_index()
                    .iter()
                    .map(|(peer_id, next_index)| format!("Peer {peer_id} -> {next_index}"))
                    .collect::<Vec<_>>();

                let [role_area, next_index_area, match_index_area] = Layout::vertical([
                    Constraint::Length(3),
                    Constraint::Length((next_index_lines.len() + 2) as u16),
                    Constraint::Length((match_index_lines.len() + 2) as u16),
                ])
                .areas(inner_area);

                let [role_area] = Layout::horizontal([Constraint::Length(20)])
                    .flex(Flex::Center)
                    .areas(role_area);
                let [next_index_area] = Layout::horizontal([Constraint::Length(20)])
                    .flex(Flex::Center)
                    .areas(next_index_area);
                let [match_index_area] = Layout::horizontal([Constraint::Length(20)])
                    .flex(Flex::Center)
                    .areas(match_index_area);

                Paragraph::new("Leader")
                    .alignment(Alignment::Center)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title(" Role ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(role_area, buffer);

                Paragraph::new(next_index_lines.join("\n"))
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .padding(Padding::left(1))
                            .title(" Next Index ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(next_index_area, buffer);

                Paragraph::new(match_index_lines.join("\n"))
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .padding(Padding::left(1))
                            .title(" Match Index ")
                            .title_alignment(Alignment::Center)
                            .title_style(Style::default().fg(Color::Blue)),
                    )
                    .render(match_index_area, buffer);
            },
        }

        block.render(area, buffer);
    }
}
