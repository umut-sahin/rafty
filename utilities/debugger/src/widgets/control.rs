use crate::*;

const FOLLOWER_ACTIONS: &[Action] = &[
    Action::TriggerElectionTimeout,
    Action::ApplyCommittedEntries,
    Action::SendCommand,
    Action::SendQuery,
];

const CANDIDATE_ACTIONS: &[Action] = &[
    Action::TriggerElectionTimeout,
    Action::ApplyCommittedEntries,
    Action::SendCommand,
    Action::SendQuery,
];

const LEADER_ACTIONS: &[Action] = &[
    Action::TriggerHeartbeatTimeout,
    Action::ApplyCommittedEntries,
    Action::SendCommand,
    Action::SendQuery,
];

#[derive(Clone, Copy)]
enum Action {
    TriggerElectionTimeout,
    TriggerHeartbeatTimeout,
    ApplyCommittedEntries,
    SendCommand,
    SendQuery,
}

impl Action {
    pub fn list_view(&self) -> &'static str {
        match self {
            Action::TriggerElectionTimeout => "Trigger Election Timeout",
            Action::TriggerHeartbeatTimeout => "Trigger Heartbeat Timeout",
            Action::ApplyCommittedEntries => "Apply Committed Entries",
            Action::SendCommand => "Send Command",
            Action::SendQuery => "Query",
        }
    }
}

enum OperationSelection {
    Action { selected: usize, actions: &'static [Action] },
    Transmit { selected: usize },
}

impl OperationSelection {
    fn go_up<A: RaftApplication>(&mut self, info_widget: &InfoWidget, simulation: &Simulation<A>) {
        let peer = simulation.peer(info_widget.main_tab_selection.peer_id());
        match self {
            OperationSelection::Action { selected, actions } => {
                if *selected > 0 {
                    *selected -= 1;
                } else {
                    let transmit_count = peer.buffered_client_transmits().len()
                        + peer.buffered_peer_transmits().len();
                    if transmit_count == 0 {
                        *selected = actions.len() - 1;
                    } else {
                        *self = OperationSelection::Transmit { selected: transmit_count - 1 };
                    }
                }
            },
            OperationSelection::Transmit { selected } => {
                if *selected == 0 {
                    let actions = match peer.role() {
                        Role::Follower(_) => FOLLOWER_ACTIONS,
                        Role::Candidate(_) => CANDIDATE_ACTIONS,
                        Role::Leader(_) => LEADER_ACTIONS,
                    };
                    let selected = actions.len() - 1;

                    *self = OperationSelection::Action { selected, actions };
                } else {
                    *selected -= 1;
                }
            },
        }
    }

    fn go_down<A: RaftApplication>(
        &mut self,
        info_widget: &InfoWidget,
        simulation: &Simulation<A>,
    ) {
        let peer = simulation.peer(info_widget.main_tab_selection.peer_id());
        let transmit_count =
            peer.buffered_client_transmits().len() + peer.buffered_peer_transmits().len();
        match self {
            OperationSelection::Action { selected, actions } => {
                if *selected < actions.len() - 1 {
                    *selected += 1;
                } else if transmit_count == 0 {
                    *selected = 0;
                } else {
                    *self = OperationSelection::Transmit { selected: 0 };
                }
            },
            OperationSelection::Transmit { selected } => {
                if *selected < transmit_count - 1 {
                    *selected += 1;
                } else {
                    let actions = match peer.role() {
                        Role::Follower(_) => FOLLOWER_ACTIONS,
                        Role::Candidate(_) => CANDIDATE_ACTIONS,
                        Role::Leader(_) => LEADER_ACTIONS,
                    };
                    let selected = 0;

                    *self = OperationSelection::Action { selected, actions };
                }
            },
        }
    }

    fn trigger<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>>(
        &mut self,
        simulation: &mut Simulation<A>,
        peer_id: PeerId,
        debugger_state: &mut DebuggerState<A, CW, QW>,
    ) {
        match self {
            OperationSelection::Action { actions, selected } => {
                match &actions[*selected] {
                    Action::TriggerElectionTimeout => {
                        log::info!("<$> Triggering election timeout of peer {}", peer_id);
                        if let Err(error) =
                            simulation.perform(SimulationAction::TimeoutElection { peer_id })
                        {
                            log::error!("<$> {:?}", error)
                        }
                    },
                    Action::TriggerHeartbeatTimeout => {
                        log::info!("<$> Triggering heartbeat timeout of peer {}", peer_id);
                        if let Err(error) =
                            simulation.perform(SimulationAction::TimeoutHeartbeat { peer_id })
                        {
                            log::error!("<$> {:?}", error)
                        }
                    },

                    Action::ApplyCommittedEntries => {
                        log::info!(
                            "<$> Applying committed entries of peer {} to its machine",
                            peer_id,
                        );
                        if let Err(error) = simulation
                            .perform(SimulationAction::ApplyCommitted { peer_id: Some(peer_id) })
                        {
                            log::error!("<$> {:?}", error)
                        }
                    },

                    Action::SendCommand => {
                        *debugger_state = DebuggerState::SelectingClient {
                            next_debugger_state: NextDebuggerState::SpecifyingCommand,
                            selection: 0,
                        };
                    },
                    Action::SendQuery => {
                        *debugger_state = DebuggerState::SelectingClient {
                            next_debugger_state: NextDebuggerState::SpecifyingQuery,
                            selection: 0,
                        };
                    },
                }
            },
            OperationSelection::Transmit { selected } => {
                let peer = simulation.peer(peer_id);

                let new_transmit_count = peer.buffered_client_transmits().len()
                    + peer.buffered_peer_transmits().len()
                    - 1;

                let is_client_transmit = *selected < peer.buffered_client_transmits().len();
                let mut client_id = None;

                let action = if is_client_transmit {
                    let transmits = peer.buffered_client_transmits();
                    let transmit = transmits.get(*selected).unwrap();

                    assert!(transmit.message().is_reply());

                    let replied_client_id = transmit.client_id();
                    let request_id = transmit.request_id();

                    log::info!(
                        "<$> Transmitting reply of request #{} from peer {} to client {}",
                        request_id,
                        peer_id,
                        replied_client_id,
                    );

                    client_id = Some(replied_client_id);
                    SimulationAction::TransmitClientReply {
                        peer_id,
                        replied_client_id_and_request_id: (replied_client_id, request_id),
                    }
                } else {
                    let selected = *selected - peer.buffered_client_transmits().len();

                    let transmits = peer.buffered_peer_transmits();
                    let transmit = transmits.get(selected).unwrap();

                    if transmit.message().is_request() {
                        let replied_peer_id = transmit.peer_id();
                        let request_id = transmit.request_id();
                        log::info!(
                            "<$> Transmitting request #{} from peer {} to peer {}",
                            request_id,
                            peer_id,
                            replied_peer_id,
                        );
                        SimulationAction::TransmitPeerRequest { peer_id, request_id }
                    } else {
                        let replied_peer_id = transmit.peer_id();
                        let request_id = transmit.request_id();
                        log::info!(
                            "<$> Transmitting reply of request #{} from peer {} to peer {}",
                            request_id,
                            peer_id,
                            replied_peer_id,
                        );
                        SimulationAction::TransmitPeerReply {
                            peer_id,
                            replied_peer_id_and_request_id: (replied_peer_id, request_id),
                        }
                    }
                };

                let new_operation_selection = if new_transmit_count > 0 {
                    if *selected == new_transmit_count {
                        OperationSelection::Transmit { selected: *selected - 1 }
                    } else {
                        OperationSelection::Transmit { selected: *selected }
                    }
                } else {
                    let actions = match peer.role() {
                        Role::Follower(_) => FOLLOWER_ACTIONS,
                        Role::Candidate(_) => CANDIDATE_ACTIONS,
                        Role::Leader(_) => LEADER_ACTIONS,
                    };
                    OperationSelection::Action { selected: actions.len() - 1, actions }
                };

                if let Err(error) = simulation.perform(action) {
                    log::error!("<$> {:?}", error)
                } else {
                    *self = new_operation_selection;
                }

                if is_client_transmit {
                    let client_id = client_id.unwrap();
                    let client = simulation.client(client_id);

                    let mut buffered_transmits = client.buffered_client_transmits().iter();
                    if let Some(transmit) = buffered_transmits.next() {
                        assert!(buffered_transmits.next().is_none());
                        let request_id = transmit.request_id();
                        if let Err(error) =
                            simulation.perform(SimulationAction::TransmitClientRequest {
                                client_id,
                                request_id,
                            })
                        {
                            log::error!("<$> {:?}", error)
                        }
                    }
                }
            },
        }
    }
}

pub struct ControlWidget {
    operation_selection: OperationSelection,
    previous_main_tab_selection: MainTabSelection,

    message_vertical_scroll: usize,
    message_horizontal_scroll: usize,
}

impl ControlWidget {
    pub fn new(info_widget: &InfoWidget) -> Self {
        Self {
            operation_selection: OperationSelection::Action {
                actions: FOLLOWER_ACTIONS,
                selected: 0,
            },
            previous_main_tab_selection: info_widget.main_tab_selection,

            message_vertical_scroll: 0,
            message_horizontal_scroll: 0,
        }
    }
}

impl ControlWidget {
    pub fn process_event<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>>(
        &mut self,
        event: &Event,
        info_widget: &InfoWidget,
        simulation: &mut Simulation<A>,
        debugger_state: &mut DebuggerState<A, CW, QW>,
    ) {
        if self.previous_main_tab_selection != info_widget.main_tab_selection {
            self.previous_main_tab_selection = info_widget.main_tab_selection;

            let peer = simulation.peer(info_widget.main_tab_selection.peer_id());
            let actions = match peer.role() {
                Role::Follower(_) => FOLLOWER_ACTIONS,
                Role::Candidate(_) => CANDIDATE_ACTIONS,
                Role::Leader(_) => LEADER_ACTIONS,
            };

            self.operation_selection = OperationSelection::Action { selected: 0, actions };

            self.message_vertical_scroll = 0;
            self.message_horizontal_scroll = 0;
        }
        if let Event::Key(event) = event {
            match event.code {
                Key::Up => {
                    self.operation_selection.go_up(info_widget, simulation);
                },
                Key::Down => {
                    self.operation_selection.go_down(info_widget, simulation);
                },

                Key::Char('w') | Key::Char('W') if event.modifiers.contains(KeyModifiers::ALT) => {
                    self.message_vertical_scroll = self.message_vertical_scroll.saturating_sub(1);
                },
                Key::Char('a') | Key::Char('A') if event.modifiers.contains(KeyModifiers::ALT) => {
                    self.message_horizontal_scroll =
                        self.message_horizontal_scroll.saturating_sub(1);
                },
                Key::Char('s') | Key::Char('S') if event.modifiers.contains(KeyModifiers::ALT) => {
                    self.message_vertical_scroll += 1;
                },
                Key::Char('d') | Key::Char('D') if event.modifiers.contains(KeyModifiers::ALT) => {
                    self.message_horizontal_scroll += 1;
                },

                Key::F(n @ 1..=4) => {
                    let peer_id = info_widget.main_tab_selection.peer_id();
                    let peer = simulation.peer(peer_id);

                    let actions = match peer.role() {
                        Role::Follower(_) => FOLLOWER_ACTIONS,
                        Role::Candidate(_) => CANDIDATE_ACTIONS,
                        Role::Leader(_) => LEADER_ACTIONS,
                    };

                    self.operation_selection =
                        OperationSelection::Action { selected: (n as usize) - 1, actions };
                },

                Key::Char(n @ 'a'..='z') if event.modifiers.is_empty() => {
                    let peer_id = info_widget.main_tab_selection.peer_id();
                    let peer = simulation.peer(peer_id);

                    let selected = (n as usize) - ('a' as usize);
                    if selected
                        < peer.buffered_client_transmits().len()
                            + peer.buffered_peer_transmits().len()
                    {
                        self.operation_selection = OperationSelection::Transmit { selected };
                    }
                },
                Key::Char(n @ 'A'..='Z') if event.modifiers == KeyModifiers::SHIFT => {
                    let peer_id = info_widget.main_tab_selection.peer_id();
                    let peer = simulation.peer(peer_id);

                    let selected = (n as usize) - ('A' as usize);
                    if selected
                        < peer.buffered_client_transmits().len()
                            + peer.buffered_peer_transmits().len()
                    {
                        self.operation_selection = OperationSelection::Transmit { selected };
                    }
                },

                Key::Enter => {
                    self.operation_selection.trigger(
                        simulation,
                        info_widget.main_tab_selection.peer_id(),
                        debugger_state,
                    );
                },
                Key::Delete => {
                    if let OperationSelection::Transmit { selected } = &self.operation_selection {
                        let peer_id = info_widget.main_tab_selection.peer_id();
                        let peer = simulation.peer(peer_id);

                        let new_transmit_count = peer.buffered_client_transmits().len()
                            + peer.buffered_peer_transmits().len()
                            - 1;

                        let is_client_transmit = *selected < peer.buffered_client_transmits().len();
                        let action = if is_client_transmit {
                            let transmits = peer.buffered_client_transmits();
                            let transmit = transmits.get(*selected).unwrap();

                            assert!(transmit.message().is_reply());

                            let replied_client_id = transmit.client_id();
                            let request_id = transmit.request_id();

                            log::info!(
                                "<$> Dropping reply of request #{} from peer {} to client {}",
                                request_id,
                                peer_id,
                                replied_client_id,
                            );
                            SimulationAction::DropClientReply {
                                peer_id,
                                replied_client_id_and_request_id: (replied_client_id, request_id),
                            }
                        } else {
                            let selected = *selected - peer.buffered_client_transmits().len();

                            let transmits = peer.buffered_peer_transmits();
                            let transmit = transmits.get(selected).unwrap();

                            if transmit.message().is_request() {
                                let replied_peer_id = transmit.peer_id();
                                let request_id = transmit.request_id();
                                log::info!(
                                    "<$> Dropping request #{} from peer {} to peer {}",
                                    request_id,
                                    peer_id,
                                    replied_peer_id,
                                );
                                SimulationAction::DropPeerRequest { peer_id, request_id }
                            } else {
                                let replied_peer_id = transmit.peer_id();
                                let request_id = transmit.request_id();
                                log::info!(
                                    "<$> Transmitting reply of request #{} from peer {} to peer {}",
                                    request_id,
                                    peer_id,
                                    replied_peer_id,
                                );
                                SimulationAction::DropPeerReply {
                                    peer_id,
                                    replied_peer_id_and_request_id: (replied_peer_id, request_id),
                                }
                            }
                        };

                        let new_operation_selection = if new_transmit_count > 0 {
                            if *selected == new_transmit_count {
                                OperationSelection::Transmit { selected: *selected - 1 }
                            } else {
                                OperationSelection::Transmit { selected: *selected }
                            }
                        } else {
                            let actions = match peer.role() {
                                Role::Follower(_) => FOLLOWER_ACTIONS,
                                Role::Candidate(_) => CANDIDATE_ACTIONS,
                                Role::Leader(_) => LEADER_ACTIONS,
                            };
                            OperationSelection::Action { selected: actions.len() - 1, actions }
                        };

                        if let Err(error) = simulation.perform(action) {
                            log::error!("<$> {:?}", error)
                        } else {
                            self.operation_selection = new_operation_selection;
                        }
                    }
                },

                _ => {},
            }
        }
    }
}

impl ControlWidget {
    pub fn renderer<'debugger, A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>>(
        &'debugger mut self,
        debugger_state: &'debugger DebuggerState<A, CW, QW>,
        info_widget: &'debugger InfoWidget,
        simulation: &'debugger Simulation<A>,
    ) -> ControlWidgetRenderer<'debugger, A, CW, QW> {
        ControlWidgetRenderer { debugger_state, info_widget, control_widget: self, simulation }
    }
}

pub struct ControlWidgetRenderer<
    'debugger,
    A: RaftApplication,
    CW: CommandWidget<A>,
    QW: QueryWidget<A>,
> {
    debugger_state: &'debugger DebuggerState<A, CW, QW>,
    info_widget: &'debugger InfoWidget,
    control_widget: &'debugger mut ControlWidget,
    simulation: &'debugger Simulation<A>,
}

impl<'debugger, A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> Widget
    for &mut ControlWidgetRenderer<'debugger, A, CW, QW>
{
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let peer = self.simulation.peer(self.info_widget.main_tab_selection.peer_id());

        let [action_area, transmit_area, message_area] =
            Layout::vertical([Constraint::Length(6), Constraint::Length(8), Constraint::Fill(100)])
                .areas(area);

        {
            let mut action_widget = ActionWidget {
                simulation: self.simulation,
                debugger_state: self.debugger_state,
                control_widget: self.control_widget,
                peer,
            };
            action_widget.render(action_area, buffer);
        }
        {
            let selected = match self.control_widget.operation_selection {
                OperationSelection::Action { .. } => None,
                OperationSelection::Transmit { selected } => Some(selected),
            };

            let transmits = peer
                .buffered_client_transmits()
                .iter()
                .map(|transmit| {
                    match transmit.message() {
                        ClientMessage::CommandRequest(_) | ClientMessage::QueryRequest(_) => {
                            unreachable!()
                        },

                        ClientMessage::CommandReply(_) => {
                            format!(
                                "(CommandReply) #{} of Client {}",
                                transmit.request_id(),
                                transmit.client_id(),
                            )
                        },
                        ClientMessage::QueryReply(_) => {
                            format!(
                                "(QueryReply) #{} of Client {}",
                                transmit.request_id(),
                                transmit.client_id(),
                            )
                        },
                    }
                })
                .chain(peer.buffered_peer_transmits().iter().map(|transmit| {
                    match transmit.message() {
                        PeerMessage::RequestVoteRequest(_) => {
                            format!(
                                "(RequestVoteRequest) #{} to Peer {}",
                                transmit.request_id(),
                                transmit.peer_id(),
                            )
                        },
                        PeerMessage::RequestVoteReply(_) => {
                            format!(
                                "(RequestVoteReply) #{} of Peer {}",
                                transmit.request_id(),
                                transmit.peer_id(),
                            )
                        },
                        PeerMessage::AppendEntriesRequest(_) => {
                            format!(
                                "(AppendEntriesRequest) #{} to Peer {}",
                                transmit.request_id(),
                                transmit.peer_id(),
                            )
                        },
                        PeerMessage::AppendEntriesReply(_) => {
                            format!(
                                "(AppendEntriesReply) #{} of Peer {}",
                                transmit.request_id(),
                                transmit.peer_id(),
                            )
                        },
                    }
                }))
                .enumerate()
                .map(|(i, display)| {
                    let mut style = Style::default();
                    if selected == Some(i) {
                        style = style.reversed();
                    }

                    let shortcut = ('a' as usize) + i;
                    let spans = vec![
                        Span::styled(
                            if shortcut < 'z' as usize {
                                format!("<{}> ", ((shortcut as u8) as char).to_ascii_uppercase())
                            } else {
                                "<-> ".to_owned()
                            },
                            Style::default().yellow(),
                        ),
                        Span::styled(display, style),
                    ];
                    Line::from(spans)
                });
            let transmit_list = List::new(transmits).block(
                Block::bordered()
                    .borders(Borders::ALL)
                    .padding(Padding::left(1))
                    .title(" Awaiting Transmits ")
                    .title_style(Style::default().fg(Color::Green))
                    .border_type(BorderType::Rounded),
            );
            let mut transmit_list_state =
                ListState::default().with_selected(match self.control_widget.operation_selection {
                    OperationSelection::Action { .. } => None,
                    OperationSelection::Transmit { selected } => Some(selected),
                });
            let mut vertical_scroll_state =
                ScrollbarState::new(transmit_list.len()).position(selected.unwrap_or(0));

            StatefulWidget::render(transmit_list, transmit_area, buffer, &mut transmit_list_state);
            StatefulWidget::render(
                Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(ScrollbarSet {
                    track: "│",
                    thumb: "║",
                    begin: "╮",
                    end: "╯",
                }),
                transmit_area,
                buffer,
                &mut vertical_scroll_state,
            );
        }
        {
            let message = match &self.control_widget.operation_selection {
                OperationSelection::Action { .. } => "".to_owned(),
                OperationSelection::Transmit { selected } => {
                    let is_client_transmit = *selected < peer.buffered_client_transmits().len();
                    if is_client_transmit {
                        let transmits = peer.buffered_client_transmits();
                        let transmit = transmits.get(*selected).unwrap();

                        match transmit.message() {
                            ClientMessage::CommandRequest(_) | ClientMessage::QueryRequest(_) => {
                                unreachable!()
                            },
                            ClientMessage::CommandReply(message) => format!("{message:#?}"),
                            ClientMessage::QueryReply(message) => format!("{message:#?}"),
                        }
                    } else {
                        let selected = *selected - peer.buffered_client_transmits().len();

                        let transmits = peer.buffered_peer_transmits();
                        let transmit = transmits.get(selected).unwrap();

                        match transmit.message() {
                            PeerMessage::RequestVoteRequest(message) => format!("{message:#?}"),
                            PeerMessage::RequestVoteReply(message) => format!("{message:#?}"),
                            PeerMessage::AppendEntriesRequest(message) => {
                                format!("{message:#?}")
                            },
                            PeerMessage::AppendEntriesReply(message) => format!("{message:#?}"),
                        }
                    }
                },
            };

            let mut scroll_widget = ScrollWidget {
                block: Block::bordered()
                    .borders(Borders::ALL)
                    .padding(Padding::left(1))
                    .title(" Message ")
                    .title_style(Style::default().fg(Color::Green))
                    .border_type(BorderType::Rounded),
                content: &message,
                vertical_scroll: &mut self.control_widget.message_vertical_scroll,
                horizontal_scroll: &mut self.control_widget.message_horizontal_scroll,
            };
            scroll_widget.render(message_area, buffer);
        }
    }
}

pub struct ActionWidget<'debugger, A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> {
    simulation: &'debugger Simulation<A>,
    debugger_state: &'debugger DebuggerState<A, CW, QW>,
    control_widget: &'debugger mut ControlWidget,
    peer: &'debugger Peer<A>,
}

impl<'debugger, A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> Widget
    for &mut ActionWidget<'debugger, A, CW, QW>
{
    fn render(self, area: Rect, buffer: &mut Buffer) {
        match self.debugger_state {
            DebuggerState::Phantom(_) => {},
            DebuggerState::Exiting => {},
            DebuggerState::Debugging => {
                let actions = match self.peer.role() {
                    Role::Follower(_) => FOLLOWER_ACTIONS,
                    Role::Candidate(_) => CANDIDATE_ACTIONS,
                    Role::Leader(_) => LEADER_ACTIONS,
                };
                let selected = match self.control_widget.operation_selection {
                    OperationSelection::Action { selected, .. } => Some(selected),
                    OperationSelection::Transmit { .. } => None,
                };

                let action_list = List::new(actions.iter().enumerate().map(|(i, action)| {
                    let mut style = Style::default();
                    if selected == Some(i) {
                        style = style.reversed();
                    }

                    let spans = vec![
                        Span::styled(format!("<F{}> ", i + 1), Style::default().yellow()),
                        Span::styled(action.list_view(), style),
                    ];
                    Line::from(spans)
                }))
                .block(
                    Block::bordered()
                        .borders(Borders::ALL)
                        .padding(Padding::left(1))
                        .title(" Actions ")
                        .title_style(Style::default().fg(Color::Green))
                        .border_type(BorderType::Rounded),
                );
                let mut action_list_state = ListState::default().with_selected(selected);
                let mut vertical_scroll_state = ScrollbarState::new(action_list.len())
                    .position(selected.unwrap_or(actions.len() - 1));

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
            DebuggerState::SelectingClient { selection, next_debugger_state } => {
                let action_list =
                    List::new((1..=self.simulation.number_of_clients()).map(|client_id| {
                        let mut style = Style::default();
                        if *selection == client_id - 1 {
                            style = style.reversed();
                        }

                        let spans = vec![
                            Span::styled(format!("<{client_id}> "), Style::default().magenta()),
                            Span::styled(format!("Via Client {client_id}"), style),
                        ];
                        Line::from(spans)
                    }))
                    .block(
                        Block::bordered()
                            .borders(Borders::ALL)
                            .padding(Padding::left(1))
                            .title(match next_debugger_state {
                                NextDebuggerState::SpecifyingCommand => " Sending Command... ",
                                NextDebuggerState::SpecifyingQuery => " Querying... ",
                            })
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
            DebuggerState::SpecifyingCommand { input_widget, .. } => {
                input_widget.renderer().render(area, buffer);
            },
            DebuggerState::SpecifyingQuery { input_widget, .. } => {
                input_widget.renderer().render(area, buffer);
            },
        }
    }
}
