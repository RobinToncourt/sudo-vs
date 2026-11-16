use std::time::{
    Duration,
    Instant, //
};

use iced::{
    Task,
    futures::channel::mpsc, //
};

use log::debug;

use matchbox_socket::PeerId;

use crate::clock::Clock;
use crate::{
    App,
    Command,
    GameOverInfo,
    GameState,
    LobbyInfo,
    Message,
    PROTOCOL_VERSION,
    PlayerAction,
    State,
    WebSocketEvent,
    clock::TIMEOUT_GRACE, //
    game::{
        Game,
        Outcome,
        Placement,
        Position, //
    },
    is_opponent_turn,
    is_player_turn,
};

struct Step {
    next: Option<State>,
    task: Task<Message>,
}

impl Step {
    fn stay() -> Self {
        Self {
            next: None,
            task: Task::none(),
        }
    }
    fn go(state: State) -> Self {
        Self {
            next: Some(state),
            task: Task::none(),
        }
    }
    fn run(task: Task<Message>) -> Self {
        Self { next: None, task }
    }
}

fn ignored(message: &Message, state: &str) -> Step {
    debug!("Ignore message({message}) in current state({state}).");
    Step::stay()
}

fn lobby(info: LobbyInfo) -> State {
    State::SelectingLobby { info }
}

fn broadcast(sender: &mut mpsc::Sender<Command>, action: &PlayerAction) -> bool {
    match postcard::to_allocvec(action) {
        Ok(bytes) => match sender.try_send(Command::Broadcast(bytes)) {
            Ok(()) => true,
            Err(e) => {
                debug!("Failed to queue action: {e}");
                false
            }
        },
        Err(e) => {
            debug!("Failed to serialize action: {e}");
            false
        }
    }
}

impl GameState {
    fn game_over(&self, info: GameOverInfo, last_placement: Option<Position>) -> State {
        State::GameOver {
            info,
            is_playing_first: self.is_playing_first,
            game: self.game.clone(),
            last_placement,
            _sender: self.sender.clone(),
        }
    }
}

pub fn update(app: &mut App, message: Message) -> Task<Message> {
    let step = match message {
        Message::ThemeExpand => {
            app.theme_expanded = !app.theme_expanded;
            Step::stay()
        }
        Message::SelectedTheme(theme) => {
            app.theme = theme;
            Step::stay()
        }
        Message::LocalizationExpand => {
            app.localization_expanded = !app.localization_expanded;
            Step::stay()
        }
        Message::SelectedLocalization(localization) => {
            app.localization = localization;
            Step::stay()
        }
        message => update_state(app, message),
    };

    if let Some(state) = step.next {
        debug!("{state}");
        app.state = state;
    }
    step.task
}

fn update_state(app: &mut App, message: Message) -> Step {
    match &mut app.state {
        State::SelectingLobby { .. } => update_selecting_lobby(app, message),
        State::WaitingForSocket => update_waiting_for_socket(message),
        State::WaitingForPeer { sender } => update_waiting_for_peer(sender, &app.nickname, message),
        State::Handshaking {
            sender,
            me_peer_id,
            opponent_peer_id,
        } => update_handshaking(sender, *me_peer_id, *opponent_peer_id, message),
        State::Game(g) => update_game(g, message),
        State::GameOver { .. } => update_game_over(message),
    }
}

fn update_selecting_lobby(app: &mut App, message: Message) -> Step {
    match message {
        Message::InputNickname(v) => app.nickname = v,
        Message::InputLobbyName(v) => app.lobby_name = v,
        Message::InputSignalingServer(v) => app.signaling_server_url = v,
        Message::JoinLobby => return Step::go(State::WaitingForSocket),
        other => return ignored(&other, "SelectingLobby"),
    }
    Step::stay()
}

fn update_waiting_for_socket(message: Message) -> Step {
    match message {
        Message::CancelConnection => Step::go(lobby(LobbyInfo::None)),
        Message::WebSocketEvent(WebSocketEvent::Ready(sender)) => {
            Step::go(State::WaitingForPeer { sender })
        }
        other => ignored(&other, "WaitingForSocket"),
    }
}

fn update_waiting_for_peer(
    sender: &mut mpsc::Sender<Command>,
    nickname: &str,
    message: Message,
) -> Step {
    match message {
        Message::CancelConnection => Step::go(lobby(LobbyInfo::None)),
        Message::WebSocketEvent(WebSocketEvent::PeerConnected { me, peer }) => {
            let action = PlayerAction::Hello {
                nickname: nickname.to_owned(),
                protocol: PROTOCOL_VERSION,
            };
            if broadcast(sender, &action) {
                Step::go(State::Handshaking {
                    sender: sender.clone(),
                    me_peer_id: me,
                    opponent_peer_id: peer,
                })
            } else {
                Step::go(lobby(LobbyInfo::AnErrorOccurred))
            }
        }
        other => ignored(&other, "WaitingForPeer"),
    }
}

fn update_handshaking(
    sender: &mpsc::Sender<Command>,
    me: PeerId,
    opponent: PeerId,
    message: Message,
) -> Step {
    match message {
        Message::CancelConnection => Step::go(lobby(LobbyInfo::None)),
        Message::WebSocketEvent(WebSocketEvent::PeerDisconnected(peer)) => {
            if peer == opponent {
                Step::go(lobby(LobbyInfo::PeerDisconnectedDuringConnection))
            } else {
                debug!("{peer} disconnected");
                Step::stay()
            }
        }
        Message::WebSocketEvent(WebSocketEvent::Closed) => {
            Step::go(lobby(LobbyInfo::ConnectionClosed))
        }
        Message::WebSocketEvent(WebSocketEvent::Message(from, packet)) => {
            if from != opponent {
                debug!("Received message from another peer. Expected {opponent}, got {from}");
                return Step::stay();
            }
            match postcard::from_bytes(&packet) {
                Ok(PlayerAction::Hello {
                    nickname: opponent_nickname,
                    protocol,
                }) => {
                    if protocol == PROTOCOL_VERSION {
                        let now = Instant::now();
                        Step::go(State::Game(GameState {
                            is_playing_first: me > opponent,
                            opponent_peer_id: opponent,
                            opponent_nickname,
                            now,
                            clock: Clock::new(now),
                            game: Game::default(),
                            last_placement: None,
                            selected_number: None,
                            sender: sender.clone(),
                        }))
                    } else {
                        Step::go(lobby(LobbyInfo::IncompatibleVersion))
                    }
                }
                Ok(_) => {
                    debug!("Ignoring action while connecting");
                    Step::stay()
                }
                Err(e) => {
                    debug!("Malformed packet: {e}");
                    Step::stay()
                }
            }
        }
        other => ignored(&other, "Handshaking"),
    }
}

fn update_game(g: &mut GameState, message: Message) -> Step {
    match message {
        Message::WebSocketEvent(WebSocketEvent::PeerDisconnected(peer)) => {
            if peer == g.opponent_peer_id {
                Step::go(g.game_over(GameOverInfo::OpponentDisconnected, None))
            } else {
                debug!("{peer} disconnected");
                Step::stay()
            }
        }
        Message::WebSocketEvent(WebSocketEvent::Closed) => {
            Step::go(g.game_over(GameOverInfo::ConnectionClosed, None))
        }
        Message::WebSocketEvent(WebSocketEvent::Message(from, packet)) => {
            game_on_packet(g, from, &packet)
        }
        Message::RadioNumberSelected(n) => {
            g.selected_number = Some(n);
            Step::stay()
        }
        Message::SudokuButtonClicked(position) => game_on_click(g, position),
        Message::Tick(now) => game_on_tick(g, now),
        Message::Forfeit => {
            if broadcast(&mut g.sender, &PlayerAction::Forfeit) {
                Step::go(g.game_over(GameOverInfo::PlayerForfeit, None))
            } else {
                Step::stay()
            }
        }
        Message::GameOver => Step::go(g.game_over(GameOverInfo::Finished, g.last_placement)),
        other => ignored(&other, "Game"),
    }
}

fn game_on_packet(g: &mut GameState, from: PeerId, packet: &[u8]) -> Step {
    if from != g.opponent_peer_id {
        debug!(
            "Received message from another peer. Expected {}, got {from}",
            g.opponent_peer_id
        );
        return Step::stay();
    }
    match postcard::from_bytes(packet) {
        Ok(PlayerAction::NumberPlacement {
            ply,
            placement,
            remaining_ms,
        }) => {
            if !is_opponent_turn(g.is_playing_first, g.game.active_player())
                || usize::from(ply) != g.game.ply()
            {
                return Step::go(g.game_over(GameOverInfo::Desynchronized, g.last_placement));
            }
            let Ok(outcome) = g.game.place_number(placement) else {
                return Step::go(g.game_over(GameOverInfo::Desynchronized, g.last_placement));
            };
            g.last_placement = Some(placement.position());
            g.clock
                .opponent_moved(Duration::from_millis(remaining_ms.into()), Instant::now());
            match outcome {
                Outcome::Finished(_) => Step::run(Task::done(Message::GameOver)),
                Outcome::Continue => Step::stay(),
            }
        }
        Ok(PlayerAction::Forfeit) => {
            Step::go(g.game_over(GameOverInfo::OpponentForfeit, g.last_placement))
        }
        Ok(PlayerAction::Timeout) => {
            Step::go(g.game_over(GameOverInfo::OpponentTimeout, g.last_placement))
        }
        Ok(_) => {
            debug!("Ignoring action while in game.");
            Step::stay()
        }
        Err(e) => {
            debug!("Malformed packet: {e}");
            Step::stay()
        }
    }
}

fn game_on_click(g: &mut GameState, position: Position) -> Step {
    let Some(number) = g.selected_number else {
        return Step::stay();
    };
    if !is_player_turn(g.is_playing_first, g.game.active_player()) {
        return Step::stay();
    }
    let placement = Placement::new(position, number);
    let ply = g.game.ply();
    let Ok(outcome) = g.game.place_number(placement) else {
        return Step::stay();
    };

    let remaining = g.clock.i_moved(Instant::now());
    let action = PlayerAction::NumberPlacement {
        ply: ply as u16,
        placement,
        remaining_ms: remaining.as_millis() as u32,
    };
    if !broadcast(&mut g.sender, &action) {
        // We applied a move the opponent never got: states have diverged.
        return Step::go(g.game_over(GameOverInfo::ConnectionClosed, g.last_placement));
    }
    g.last_placement = Some(position);
    match outcome {
        Outcome::Finished(_) => Step::run(Task::done(Message::GameOver)),
        Outcome::Continue => Step::stay(),
    }
}

fn game_on_tick(g: &mut GameState, now: Instant) -> Step {
    g.now = now;
    let my_turn = is_player_turn(g.is_playing_first, g.game.active_player());

    if my_turn && g.clock.mine(true, now).is_zero() {
        // I'm the authority on my clock: announce it, then end.
        let _ = broadcast(&mut g.sender, &PlayerAction::Timeout);
        return Step::go(g.game_over(GameOverInfo::PlayerTimeout, g.last_placement));
    }
    // Normally the opponent announces its own timeout. This is the fallback
    // for a frozen or throttled client.
    if !my_turn
        && g.clock.theirs(false, now).is_zero()
        && g.clock.elapsed(now) > g.clock.theirs + TIMEOUT_GRACE
    {
        return Step::go(g.game_over(GameOverInfo::OpponentTimeout, g.last_placement));
    }
    Step::stay()
}

fn update_game_over(message: Message) -> Step {
    match message {
        Message::ReturnToLobby => Step::go(lobby(LobbyInfo::None)),
        other => ignored(&other, "GameOver"),
    }
}
