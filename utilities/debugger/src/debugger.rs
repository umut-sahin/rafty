use {
    crate::*,
    std::marker::PhantomData,
};

pub(crate) enum NextDebuggerState {
    SpecifyingCommand,
    SpecifyingQuery,
}

pub(crate) enum DebuggerState<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> {
    Exiting,
    Debugging,
    SelectingClient {
        next_debugger_state: NextDebuggerState,
        selection: usize,
    },
    SpecifyingCommand {
        client_id: ClientId,
        input_widget: CW,
    },
    SpecifyingQuery {
        client_id: ClientId,
        input_widget: QW,
    },

    #[allow(unused)]
    Phantom(PhantomData<A>),
}

pub trait CommandWidget<A: RaftApplication>: Default {
    fn on_user_event(&mut self, event: Event);

    fn back(&self) -> Option<Self>;

    fn renderer(&self) -> impl Widget;

    fn finalize(&mut self) -> Option<A::Command>;
}

pub trait QueryWidget<A: RaftApplication>: Default {
    fn on_user_event(&mut self, event: Event);

    fn back(&self) -> Option<Self>;

    fn renderer(&self) -> impl Widget;

    fn finalize(&mut self) -> Option<A::Query>;
}

/// A TUI debugger for [RaftApplication]s.
pub struct Debugger<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> {
    state: DebuggerState<A, CW, QW>,
    simulation: Simulation<A>,
    logs_widget: LogsWidget,
    info_widget: InfoWidget,
    control_widget: ControlWidget,
}

impl<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> Debugger<A, CW, QW> {
    /// Creates a debugger for a simulation.
    pub fn new(simulation: Simulation<A>) -> anyhow::Result<Self> {
        if simulation.number_of_peers() == 0 {
            return Err(anyhow::anyhow!("Debugger cannot be initialized with no peers"));
        }
        if simulation.number_of_clients() == 0 {
            return Err(anyhow::anyhow!("Debugger cannot be initialized with no clients"));
        }

        let logs_widget = LogsWidget::default();
        let info_widget = InfoWidget::new(&simulation);
        let control_widget = ControlWidget::new(&info_widget);

        Ok(Debugger {
            state: DebuggerState::Debugging,
            simulation,
            logs_widget,
            info_widget,
            control_widget,
        })
    }
}

impl<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> Debugger<A, CW, QW> {
    /// Starts the debugging session.
    pub fn start(mut self) -> anyhow::Result<()> {
        tui_logger::init_logger(LevelFilter::Trace)?;
        tui_logger::set_default_level(LevelFilter::Trace);

        crossterm::terminal::enable_raw_mode().context("Failed to enable raw mode")?;
        crossterm::execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
            .context("Failed to setup the terminal")?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend).context("Failed to create the terminal")?;

        terminal.clear().context("Failed to clear the terminal")?;
        let (event_sender, event_receiver) = mpsc::channel::<DebuggerEvent>();

        thread::spawn({
            let event_sender = event_sender.clone();
            move || {
                while let Ok(event) = crossterm::event::read() {
                    if event_sender.send(DebuggerEvent::UserEvent(event)).is_err() {
                        break;
                    }
                }
            }
        });
        thread::spawn({
            let event_sender = event_sender.clone();
            move || {
                while event_sender.send(DebuggerEvent::Redraw).is_ok() {
                    thread::sleep(Duration::from_millis(16));
                }
            }
        });

        self.run(&mut terminal, event_receiver)?;

        terminal.clear().context("Failed to clear the terminal")?;

        crossterm::terminal::disable_raw_mode().context("Failed to teardown the terminal")?;
        crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
            .context("Failed to leave alternate screen")?;

        Ok(())
    }
}

impl<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> Debugger<A, CW, QW> {
    fn run(
        &mut self,
        terminal: &mut Terminal<impl Backend>,
        event_receiver: mpsc::Receiver<DebuggerEvent>,
    ) -> anyhow::Result<()> {
        for event in event_receiver {
            match event {
                DebuggerEvent::UserEvent(event) => self.on_user_event(event),
                DebuggerEvent::Redraw => self.on_redraw(terminal)?,
            }
            if matches!(self.state, DebuggerState::Exiting) {
                break;
            }
        }
        Ok(())
    }

    fn on_user_event(&mut self, event: Event) {
        #[allow(clippy::collapsible_if)]
        if let Event::Key(event) = event {
            if event.code == Key::Esc {
                match &mut self.state {
                    DebuggerState::Phantom(_) => {},
                    DebuggerState::Exiting => {},

                    DebuggerState::Debugging => {
                        self.state = DebuggerState::Exiting;
                    },

                    DebuggerState::SelectingClient { .. } => {
                        self.state = DebuggerState::Debugging;
                    },

                    DebuggerState::SpecifyingCommand { client_id, input_widget } => {
                        match input_widget.back() {
                            Some(new_input_widget) => {
                                *input_widget = new_input_widget;
                            },
                            None => {
                                self.state = DebuggerState::SelectingClient {
                                    next_debugger_state: NextDebuggerState::SpecifyingCommand,
                                    selection: client_id.0 - 1,
                                }
                            },
                        }
                    },
                    DebuggerState::SpecifyingQuery { client_id, input_widget } => {
                        match input_widget.back() {
                            Some(new_input_widget) => {
                                *input_widget = new_input_widget;
                            },
                            None => {
                                self.state = DebuggerState::SelectingClient {
                                    next_debugger_state: NextDebuggerState::SpecifyingQuery,
                                    selection: client_id.0 - 1,
                                }
                            },
                        }
                    },
                }
                return;
            }
        }
        match &mut self.state {
            DebuggerState::Phantom(_) => {},
            DebuggerState::Exiting => {},

            DebuggerState::Debugging => {
                self.logs_widget.process_event(&event);
                self.info_widget.process_event(&event, &self.simulation);
                self.control_widget.process_event(
                    &event,
                    &self.info_widget,
                    &mut self.simulation,
                    &mut self.state,
                );
            },

            DebuggerState::SelectingClient { next_debugger_state, selection } => {
                if let Event::Key(event) = event {
                    match event.code {
                        Key::Char(n @ '1'..'9') => {
                            let index = (n as usize) - ('1' as usize);
                            if index < self.simulation.number_of_clients() {
                                let client_id = ClientId(index + 1);
                                match next_debugger_state {
                                    NextDebuggerState::SpecifyingCommand => {
                                        self.state = DebuggerState::SpecifyingCommand {
                                            client_id,
                                            input_widget: Default::default(),
                                        };
                                    },
                                    NextDebuggerState::SpecifyingQuery => {
                                        self.state = DebuggerState::SpecifyingQuery {
                                            client_id,
                                            input_widget: Default::default(),
                                        };
                                    },
                                }
                            }
                        },
                        Key::Enter => {
                            let client_id = ClientId(*selection + 1);
                            match next_debugger_state {
                                NextDebuggerState::SpecifyingCommand => {
                                    self.state = DebuggerState::SpecifyingCommand {
                                        client_id,
                                        input_widget: Default::default(),
                                    };
                                },
                                NextDebuggerState::SpecifyingQuery => {
                                    self.state = DebuggerState::SpecifyingQuery {
                                        client_id,
                                        input_widget: Default::default(),
                                    };
                                },
                            }
                        },

                        Key::Up => {
                            if *selection == 0 {
                                *selection = self.simulation.number_of_clients() - 1;
                            } else {
                                *selection -= 1;
                            }
                        },
                        Key::Down => {
                            if *selection == self.simulation.number_of_clients() - 1 {
                                *selection = 0;
                            } else {
                                *selection += 1;
                            }
                        },

                        _ => {},
                    }
                }
            },
            DebuggerState::SpecifyingCommand { client_id, input_widget } => {
                input_widget.on_user_event(event);
                if let Some(command) = input_widget.finalize() {
                    let peer_id = self.info_widget.main_tab_selection.peer_id();
                    log::info!(
                        "<$> Sending `{:?}` command to peer {} via client {}",
                        command,
                        peer_id,
                        client_id,
                    );

                    let action = SimulationAction::SendCommand {
                        peer_id: Some(peer_id),
                        client_id: *client_id,
                        command,
                    };
                    if let Err(error) = self.simulation.perform(action) {
                        log::error!("<$> {:?}", error)
                    }

                    let client = self.simulation.client(*client_id);
                    let mut buffered_transmits = client.buffered_client_transmits().iter();
                    if let Some(transmit) = buffered_transmits.next() {
                        assert!(buffered_transmits.next().is_none());
                        let request_id = transmit.request_id();
                        if let Err(error) =
                            self.simulation.perform(SimulationAction::TransmitClientRequest {
                                client_id: *client_id,
                                request_id,
                            })
                        {
                            log::error!("<$> {:?}", error)
                        }
                    }

                    self.state = DebuggerState::Debugging;
                }
            },
            DebuggerState::SpecifyingQuery { client_id, input_widget } => {
                input_widget.on_user_event(event);
                if let Some(query) = input_widget.finalize() {
                    let peer_id = self.info_widget.main_tab_selection.peer_id();
                    log::info!(
                        "<$> Sending `{:?}` query to peer {} via client {}",
                        query,
                        peer_id,
                        client_id,
                    );

                    let action = SimulationAction::SendQuery {
                        peer_id: Some(peer_id),
                        client_id: *client_id,
                        query,
                    };
                    if let Err(error) = self.simulation.perform(action) {
                        log::error!("<$> {:?}", error)
                    }

                    let client = self.simulation.client(*client_id);
                    let mut buffered_transmits = client.buffered_client_transmits().iter();
                    if let Some(transmit) = buffered_transmits.next() {
                        assert!(buffered_transmits.next().is_none());
                        let request_id = transmit.request_id();
                        if let Err(error) =
                            self.simulation.perform(SimulationAction::TransmitClientRequest {
                                client_id: *client_id,
                                request_id,
                            })
                        {
                            log::error!("<$> {:?}", error)
                        }
                    }

                    self.state = DebuggerState::Debugging;
                }
            },
        }
    }

    fn on_redraw(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        terminal.draw(|frame| {
            frame.render_widget(&mut *self, frame.area());
        })?;
        Ok(())
    }
}

impl<A: RaftApplication, CW: CommandWidget<A>, QW: QueryWidget<A>> Widget
    for &mut Debugger<A, CW, QW>
{
    fn render(self, area: Rect, buffer: &mut Buffer) {
        let [debugger_area, log_area] =
            Layout::vertical([Constraint::Fill(50), Constraint::Fill(50)]).areas(area);

        let [info_area, control_area] =
            Layout::horizontal([Constraint::Fill(70), Constraint::Fill(30)]).areas(debugger_area);

        self.logs_widget.render(log_area, buffer);
        self.info_widget.renderer(&self.simulation).render(info_area, buffer);
        self.control_widget
            .renderer(&self.state, &self.info_widget, &self.simulation)
            .render(control_area, buffer);
    }
}
