#![deny(clippy::unwrap_used)]

mod game;

use std::{
    fmt::{self, Display},
    time::Duration,
};

use iced::{
    Alignment, Color, Element, Length, Subscription, Task, Theme,
    futures::{FutureExt, SinkExt, StreamExt, channel::mpsc, select},
    stream,
    widget::{
        Column, Row, button, button::Style, column as col, container, radio, row, space, text,
        text_input,
    },
};
use iced_aw::DropDown;

use futures_timer::Delay;
use iced::widget::scrollable;
use log::debug;

use matchbox_socket::{PeerId, PeerState, WebRtcSocket};

use serde::{Deserialize, Serialize};

use crate::game::{
    File, Game, GameResult, Grid, Number, Outcome, PlaceError, Placement, Player, Position, Rank,
};

const CELL_SIZE: f32 = 40.0;
const CELL_SPACING: f32 = 2.0;
const SQUARE_SPACING: f32 = 10.0;

#[derive(Debug, Clone)]
enum LobbyInfo {
    None,
    PeerDisconnectedDuringConnection,
    ConnectionClosed,
    AnErrorOccurred,
}
impl Display for LobbyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::None => "",
                Self::PeerDisconnectedDuringConnection =>
                    "The peer disconnected during connection.",
                Self::ConnectionClosed => "The connection was closed.",
                Self::AnErrorOccurred => "An error occurred.",
            }
        )
    }
}

struct GameState {
    is_playing_first: bool,
    opponent_peer_id: PeerId,
    opponent_nickname: String,
    game: Game,
    last_placement: Option<Position>,
    selected_number: Option<Number>,
    sender: mpsc::Sender<Command>,
}

#[derive(Debug, Clone, Copy)]
enum GameOverInfo {
    Finished,
    PlayerForfeit,
    OpponentForfeit,
    OpponentDisconnected,
    ConnectionClosed,
}
impl Display for GameOverInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Finished => "The game is finished.",
                Self::PlayerForfeit => "You forfeited.",
                Self::OpponentForfeit => "Your opponent forfeited.",
                Self::OpponentDisconnected => "Your opponent disconnected.",
                Self::ConnectionClosed => "The connection closed.",
            }
        )
    }
}

enum State {
    SelectingLobby {
        info: LobbyInfo,
    },
    WaitingForSocket,
    WaitingForPeer {
        sender: mpsc::Sender<Command>,
    },
    Handshaking {
        sender: mpsc::Sender<Command>,
        me_peer_id: PeerId,
        opponent_peer_id: PeerId,
    },
    Game(GameState),
    GameOver {
        info: GameOverInfo,
        is_playing_first: bool,
        game: Game,
        last_placement: Option<Position>,
        // Keep the sender so the last move is flush.
        _sender: mpsc::Sender<Command>,
    },
}
impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::SelectingLobby { .. } => "SelectingLobby",
                Self::WaitingForSocket => "WaitingForSocket",
                Self::WaitingForPeer { .. } => "WaitingForPeer",
                Self::Handshaking { .. } => "Handshaking",
                Self::Game { .. } => "Game",
                Self::GameOver { .. } => "GameOver",
            }
        )
    }
}

#[derive(Clone)]
enum Localization {
    En,
    Fr,
}
impl Display for Localization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Localization::En => "English",
                Localization::Fr => "Français",
            }
        )
    }
}
impl Localization {
    const ALL: &[Self] = &[Self::En, Self::Fr];
}

struct App {
    theme: Theme,
    theme_expanded: bool,
    localization: Localization,
    localization_expanded: bool,
    nickname: String,
    lobby_name: String,
    signaling_server_url: String,
    state: State,
}
impl Default for App {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            theme_expanded: false,
            localization: Localization::En,
            localization_expanded: false,
            nickname: String::from("Bob"),
            lobby_name: String::from("test"),
            signaling_server_url: String::from(
                // "wss://web-socket-signaling-server.onrender.com/<lobby_name>",
                "ws://localhost:3536/<lobby_name>",
            ),
            state: State::SelectingLobby {
                info: LobbyInfo::None,
            },
        }
    }
}

/// App level messages.
#[derive(Clone)]
enum Message {
    ThemeExpand,
    SelectedTheme(Theme),
    LocalizationExpand,
    SelectedLocalization(Localization),
    WebSocketEvent(WebSocketEvent),
    /// Input the nickname.
    InputNickname(String),
    /// Input the lobby name.
    InputLobbyName(String),
    /// Input the signaling server URL.
    InputSignalingServer(String),
    JoinLobby,
    CancelConnection,
    /// Selecting a radio button.
    RadioNumberSelected(Number),
    /// Clicking on a button of the grid.
    SudokuButtonClicked(Position),
    Forfeit,
    /// To transition from the `Game` state to the `GameOver` state.
    GameOver,
    ReturnToLobby,
}
impl Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ThemeExpand => "Theme Expand",
                Self::SelectedTheme(_) => "Selected Theme",
                Self::LocalizationExpand => "Localization Expand",
                Self::SelectedLocalization(_) => "Localization Expand",
                Self::WebSocketEvent(_) => "WebSocketEvent",
                Self::InputNickname(_) => "InputNickname",
                Self::InputLobbyName(_) => "InputLobbyName",
                Self::InputSignalingServer(_) => "InputSignalingServer",
                Self::JoinLobby => "JoinLobby",
                Self::CancelConnection => "CancelConnection",
                Self::RadioNumberSelected(_) => "RadioNumberSelected",
                Self::SudokuButtonClicked(_) => "SudokuButtonClicked",
                Self::Forfeit => "Forfeit",
                Self::GameOver => "GameOver",
                Self::ReturnToLobby => "ReturnToLobby",
            }
        )
    }
}

/// Events that happens at the `WebRtcSocket` level.
#[derive(Debug, Clone)]
enum WebSocketEvent {
    Ready(mpsc::Sender<Command>),
    PeerConnected { me: PeerId, peer: PeerId },
    PeerDisconnected(PeerId),
    Message(PeerId, Vec<u8>),
    Closed,
}

/// Commands to send to the `WebRtcSocket`.
enum Command {
    Broadcast(Vec<u8>),
}

/// The data that will be exchange between players, represents their action.
#[derive(Deserialize, Serialize)]
enum PlayerAction {
    Nickname(String),
    NumberPlacement(Placement),
    Forfeit,
}

fn main() -> iced::Result {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Debug).ok();
    }

    #[cfg(not(target_arch = "wasm32"))]
    init_logger();

    iced::application(App::default, update, view)
        .subscription(subscription)
        .theme(theme)
        .run()
}

fn theme(app: &App) -> Theme {
    app.theme.clone()
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match (&mut app.state, message) {
        /*
         * Theme.
         */
        (_, Message::ThemeExpand) => {
            app.theme_expanded = !app.theme_expanded;
        }
        (_, Message::SelectedTheme(theme)) => {
            app.theme = theme;
        }
        /*
         * Localization.
         */
        (_, Message::LocalizationExpand) => {
            app.localization_expanded = !app.localization_expanded;
        }
        (_, Message::SelectedLocalization(localization)) => {
            app.localization = localization;
        }
        /*
         * SelectingLobby.
         */
        (State::SelectingLobby { .. }, Message::InputNickname(new_nickname)) => {
            app.nickname = new_nickname
        }
        (State::SelectingLobby { .. }, Message::InputLobbyName(new_lobby_name)) => {
            app.lobby_name = new_lobby_name
        }
        (State::SelectingLobby { .. }, Message::InputSignalingServer(new_signaling_server)) => {
            app.signaling_server_url = new_signaling_server
        }
        (State::SelectingLobby { .. }, Message::JoinLobby) => {
            debug!("WaitingForSocket");
            app.state = State::WaitingForSocket
        }
        /*
         * WaitingForSocket.
         */
        (State::WaitingForSocket, Message::CancelConnection) => {
            debug!("SelectingLobby");
            app.state = State::SelectingLobby {
                info: LobbyInfo::None,
            }
        }
        (State::WaitingForSocket, Message::WebSocketEvent(WebSocketEvent::Ready(sender))) => {
            debug!("WaitingForPeer");
            app.state = State::WaitingForPeer { sender };
        }
        /*
         * WaitingForPeer.
         */
        (State::WaitingForPeer { .. }, Message::CancelConnection) => {
            debug!("SelectingLobby");
            app.state = State::SelectingLobby {
                info: LobbyInfo::None,
            }
        }
        (
            State::WaitingForPeer { sender },
            Message::WebSocketEvent(WebSocketEvent::PeerConnected { me, peer }),
        ) => {
            debug!("Sending nickname");
            match postcard::to_allocvec(&PlayerAction::Nickname(app.nickname.clone())) {
                Ok(bytes) => {
                    if let Err(e) = sender.try_send(Command::Broadcast(bytes)) {
                        debug!("Failed to queue nickname: {e}");
                        app.state = State::SelectingLobby {
                            info: LobbyInfo::AnErrorOccurred,
                        }
                    } else {
                        debug!("Nickname queued successfully");
                        debug!("Handshaking");
                        app.state = State::Handshaking {
                            sender: sender.clone(),
                            me_peer_id: me,
                            opponent_peer_id: peer,
                        };
                    }
                }
                Err(e) => {
                    debug!("Failed to serialize move: {e}");
                    app.state = State::SelectingLobby {
                        info: LobbyInfo::AnErrorOccurred,
                    }
                }
            }
        }
        /*
         * Handshaking.
         */
        (State::Handshaking { .. }, Message::CancelConnection) => {
            debug!("SelectingLobby");
            app.state = State::SelectingLobby {
                info: LobbyInfo::None,
            }
        }
        (
            State::Handshaking {
                opponent_peer_id, ..
            },
            Message::WebSocketEvent(WebSocketEvent::PeerDisconnected(peer_id)),
        ) => {
            if peer_id == *opponent_peer_id {
                debug!("Opponent disconnected.");
                debug!("SelectingLobby");
                app.state = State::SelectingLobby {
                    info: LobbyInfo::PeerDisconnectedDuringConnection,
                };
            } else {
                debug!("{peer_id} disconnected");
            }
        }
        (State::Handshaking { .. }, Message::WebSocketEvent(WebSocketEvent::Closed)) => {
            debug!("SelectingLobby");
            app.state = State::SelectingLobby {
                info: LobbyInfo::ConnectionClosed,
            }
        }
        (
            State::Handshaking {
                sender,
                me_peer_id,
                opponent_peer_id,
            },
            Message::WebSocketEvent(WebSocketEvent::Message(packet_peer_id, packet)),
        ) => {
            if packet_peer_id != *opponent_peer_id {
                debug!(
                    "Received message from another peer. Expected {opponent_peer_id}, got {packet_peer_id}"
                );
                return Task::none();
            }

            match postcard::from_bytes(&packet) {
                Ok(PlayerAction::Nickname(opponent_nickname)) => {
                    debug!("Game");
                    let is_playing_first = me_peer_id > opponent_peer_id;
                    app.state = State::Game(GameState {
                        is_playing_first,
                        opponent_peer_id: *opponent_peer_id,
                        opponent_nickname,
                        game: Game::default(),
                        last_placement: None,
                        selected_number: None,
                        sender: sender.clone(),
                    });
                }
                Ok(_) => debug!("Ignoring action while connecting"),
                Err(e) => debug!("Malformed packet: {e}"),
            }
        }
        /*
         * Game.
         */
        (State::Game(g), Message::WebSocketEvent(WebSocketEvent::PeerDisconnected(peer_id))) => {
            if peer_id == g.opponent_peer_id {
                debug!("GameOver");
                app.state = State::GameOver {
                    info: GameOverInfo::OpponentDisconnected,
                    is_playing_first: g.is_playing_first,
                    game: g.game.clone(),
                    last_placement: None,
                    _sender: g.sender.clone(),
                };
            } else {
                debug!("{peer_id} disconnected");
            }
        }
        (State::Game(g), Message::WebSocketEvent(WebSocketEvent::Closed)) => {
            debug!("GameOver");
            app.state = State::GameOver {
                info: GameOverInfo::ConnectionClosed,
                is_playing_first: g.is_playing_first,
                game: g.game.clone(),
                last_placement: None,
                _sender: g.sender.clone(),
            }
        }
        (
            State::Game(g),
            Message::WebSocketEvent(WebSocketEvent::Message(packet_peer_id, packet)),
        ) => {
            if packet_peer_id != g.opponent_peer_id {
                debug!(
                    "Received message from another peer. Expected {}, got {packet_peer_id}",
                    g.opponent_peer_id,
                );
                return Task::none();
            }

            match postcard::from_bytes(&packet) {
                Ok(PlayerAction::NumberPlacement(placement)) => {
                    g.last_placement = Some(placement.position());
                    return game_place_number(g, placement);
                }
                Ok(PlayerAction::Forfeit) => {
                    debug!("GameOver");
                    app.state = State::GameOver {
                        info: GameOverInfo::OpponentForfeit,
                        is_playing_first: g.is_playing_first,
                        game: g.game.clone(),
                        last_placement: g.last_placement,
                        _sender: g.sender.clone(),
                    }
                }
                Ok(_) => debug!("Ignoring action while in game."),
                Err(e) => debug!("Malformed packet: {e}"),
            }
        }
        (State::Game(g), Message::RadioNumberSelected(selected_number)) => {
            g.selected_number = Some(selected_number);
        }
        (State::Game(g), Message::SudokuButtonClicked(position)) => {
            if let Some(selected_number) = g.selected_number {
                let placement = Placement::new(position, selected_number);
                let player_action = PlayerAction::NumberPlacement(placement);
                match postcard::to_allocvec(&player_action) {
                    Ok(bytes) => {
                        if let Err(e) = g.sender.try_send(Command::Broadcast(bytes)) {
                            debug!("Failed to queue move: {e}");
                        } else {
                            g.last_placement = Some(placement.position());
                            return game_place_number(g, placement);
                        }
                    }
                    Err(e) => debug!("Failed to serialize move: {e}"),
                }
            } else {
                debug!("No selected number.");
            }
        }
        (State::Game(g), Message::Forfeit) => {
            let player_action = PlayerAction::Forfeit;
            match postcard::to_allocvec(&player_action) {
                Ok(bytes) => {
                    if let Err(e) = g.sender.try_send(Command::Broadcast(bytes)) {
                        debug!("Failed to queue move: {e}");
                    } else {
                        debug!("GameOver");
                        app.state = State::GameOver {
                            info: GameOverInfo::PlayerForfeit,
                            is_playing_first: g.is_playing_first,
                            game: g.game.clone(),
                            last_placement: None,
                            _sender: g.sender.clone(),
                        };
                    }
                }
                Err(e) => debug!("Failed to serialize move: {e}"),
            }
        }
        (State::Game(g), Message::GameOver) => {
            debug!("GameOver");
            app.state = State::GameOver {
                info: GameOverInfo::Finished,
                is_playing_first: g.is_playing_first,
                game: g.game.clone(),
                last_placement: g.last_placement,
                _sender: g.sender.clone(),
            };
        }
        /*
         * GameOver.
         */
        (State::GameOver { .. }, Message::ReturnToLobby) => {
            app.state = State::SelectingLobby {
                info: LobbyInfo::None,
            }
        }
        /*
         * Invalid state/message pair.
         */
        (state, msg) => debug!("Ignore message({msg}) in current state({state})."),
    }
    Task::none()
}

fn game_place_number(g: &mut GameState, placement: Placement) -> Task<Message> {
    let outcome = g.game.place_number(placement);
    match outcome {
        Ok(outcome) => match outcome {
            Outcome::Continue => Task::none(),
            Outcome::Finished(_) => Task::done(Message::GameOver),
        },
        Err(PlaceError::CellNotEmpty) => Task::none(),
        Err(PlaceError::GameAlreadyOver) => Task::none(),
    }
}

fn subscription(app: &App) -> Subscription<Message> {
    match &app.state {
        State::SelectingLobby { .. } => Subscription::none(),
        State::WaitingForSocket
        | State::WaitingForPeer { .. }
        | State::Handshaking { .. }
        | State::Game(_)
        | State::GameOver { .. } => {
            let room_url = app
                .signaling_server_url
                .replace("<lobby_name>", &app.lobby_name);
            connect(room_url)
        }
        .map(Message::WebSocketEvent),
    }
}

fn connect<S: Into<String>>(room_url: S) -> Subscription<WebSocketEvent> {
    let room_url = room_url.into();
    Subscription::run_with(room_url, |url| {
        let url = url.clone();
        stream::channel(100, |mut out: mpsc::Sender<WebSocketEvent>| async move {
            let (mut socket, loop_fut) = WebRtcSocket::new_reliable(url);
            let mut loop_fut = loop_fut.fuse();

            let (cmd_tx, mut cmd_rx) = mpsc::channel(100);
            let _ = out.send(WebSocketEvent::Ready(cmd_tx)).await;

            loop {
                select! {
                    _ = loop_fut => {
                        let _ = out.send(WebSocketEvent::Closed).await;
                        break;
                    }
                    cmd = cmd_rx.select_next_some() => match cmd {
                        Command::Broadcast(data) => {
                            debug!("Sending data to all peers.");
                            let peers: Vec<_> = socket.connected_peers().collect();
                            for p in peers {
                                socket.channel_mut(0).send(data.clone().into(), p);
                            }
                        }
                    },
                    () = Delay::new(Duration::from_millis(30)).fuse() => {
                        if let Ok(changes) = socket.try_update_peers() {
                            let socket_peer_id = socket.id().expect("CAN IT FAILS AT THIS POINT ?");
                            for (peer, state) in changes {
                                let _ = out.send(match state {
                                    PeerState::Connected => WebSocketEvent::PeerConnected {
                                        me: socket_peer_id,
                                        peer,
                                    },
                                    PeerState::Disconnected => WebSocketEvent::PeerDisconnected(peer),
                                }).await;
                            }
                        }
                        for (peer, packet) in socket.channel_mut(0).receive() {
                            let _ = out.send(WebSocketEvent::Message(peer, packet.into_vec())).await;
                        }
                    }
                }
            }
            std::future::pending::<()>().await;
        })
    })
}

fn view(app: &App) -> Element<'_, Message> {
    let top_bar = top_bar(app);

    let main_element = match &app.state {
        State::SelectingLobby { info, .. } => {
            let nickname = &app.nickname;
            let lobby_name = &app.lobby_name;
            let signaling_server_url = &app.signaling_server_url;
            view_selecting_lobby(info, nickname, lobby_name, signaling_server_url)
        }
        State::WaitingForSocket | State::WaitingForPeer { .. } | State::Handshaking { .. } => {
            view_connecting()
        }
        State::Game(g) => view_game(&app.nickname, g),
        State::GameOver {
            info,
            is_playing_first,
            game,
            last_placement,
            ..
        } => view_game_over(*info, *is_playing_first, game, *last_placement),
    };

    col![
        top_bar,
        main_element,
    ].into()
}

fn top_bar(app: &App) -> Element<'_, Message> {
    let theme_menu = theme_menu(app);
    let language_menu = language_menu(app);

    row![theme_menu, language_menu].into()
}

fn theme_menu(app: &App) -> Element<'_, Message> {
    let underlay = Row::new()
        .push(button(text(format!("{}", app.theme))).on_press(Message::ThemeExpand))
        .spacing(8)
        .align_y(Alignment::Center);
    let options = Column::with_children(Theme::ALL.iter().map(|theme| {
        Row::new()
            .push(button(text(theme.to_string())).on_press(Message::SelectedTheme(theme.clone())))
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }));
    let overlay = scrollable(container(options.padding([0, 4])).style(container::bordered_box));

    let theme_drop_down = DropDown::new(underlay, overlay, app.theme_expanded);
    theme_drop_down.into()
}

fn language_menu(app: &App) -> Element<'_, Message> {
    let underlay = Row::new()
        .push(button(text(format!("{}", app.localization))).on_press(Message::LocalizationExpand))
        .spacing(8)
        .align_y(Alignment::Center);
    let options = Column::with_children(Localization::ALL.iter().map(|localization| {
        Row::new()
            .push(button(text(localization.to_string())).on_press(Message::SelectedLocalization(localization.clone())))
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }));
    let overlay = scrollable(container(options.padding([0, 4])).style(container::bordered_box));

    let localization_drop_down = DropDown::new(underlay, overlay, app.localization_expanded);
    localization_drop_down.into()
}

fn view_selecting_lobby<'a>(
    info: &LobbyInfo,
    nickname: &'a str,
    lobby_name: &'a str,
    signaling_server_url: &'a str,
) -> Element<'a, Message> {
    container(
        col![
            // Info.
            text(info.to_string()),
            // Nickname.
            row![
                space().width(Length::Fill),
                text("Nickname: ")
                    .width(Length::Fixed(90.0))
                    .align_x(Alignment::End),
                text_input("Enter your nickname", nickname)
                    .on_input(Message::InputNickname)
                    .width(Length::Fill),
                space().width(Length::Fill),
            ]
            .align_y(Alignment::Center),
            // Lobby name.
            row![
                space().width(Length::Fill),
                text("Lobby: ")
                    .width(Length::Fixed(90.0))
                    .align_x(Alignment::End),
                text_input("Lobby name", lobby_name)
                    .on_input(Message::InputLobbyName)
                    .width(Length::Fill),
                space().width(Length::Fill),
            ]
            .align_y(Alignment::Center),
            // TODO: add text to signal wrong lobby name.
            // Signaling server url.
            // TODO: toggle hide/reveal the text_input.
            row![
                space().width(Length::Fill),
                text("Signaling server URL: ")
                    .width(Length::Fixed(90.0))
                    .align_x(Alignment::End),
                text_input("Signaling server URL", signaling_server_url)
                    .on_input(Message::InputSignalingServer)
                    .width(Length::Fill),
                space().width(Length::Fill),
            ]
            .align_y(Alignment::Center),
            // Join button.
            button("Join")
                .on_press_maybe(can_join_lobby(nickname, lobby_name).then_some(Message::JoinLobby))
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn can_join_lobby(nickname: &str, lobby_name: &str) -> bool {
    let is_all_alphanumeric = lobby_name.chars().all(char::is_alphanumeric);
    !nickname.is_empty() && !lobby_name.is_empty() && is_all_alphanumeric
}

fn view_connecting() -> Element<'static, Message> {
    container(
        col![
            text("Connecting / Waiting for someone to connect...").size(40),
            button("Cancel").on_press(Message::CancelConnection),
        ]
        .align_x(Alignment::Center),
    )
    .center(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn view_game<'a>(nickname: &'a str, g: &'a GameState) -> Element<'a, Message> {
    container(
        col![
            player_turn(nickname, g),
            show_grid(&g.game, GridInteraction::Interactive, g.last_placement),
            number_selection(g),
            button("Forfeit").on_press(Message::Forfeit),
        ]
        .align_x(Alignment::Center)
        .spacing(10),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn view_game_over(
    info: GameOverInfo,
    is_player_first: bool,
    g: &Game,
    last_placement: Option<Position>,
) -> Element<'_, Message> {
    let end_text = match info {
        GameOverInfo::Finished => match g.result() {
            Some(GameResult::Draw) => "Draw".to_string(),
            Some(GameResult::Loss { loser, .. }) => {
                if is_player_self(is_player_first, loser) {
                    "You lost.".to_string()
                } else {
                    "You won.".to_string()
                }
            }
            None => "Finished".to_string(),
        },
        _ => info.to_string(),
    };
    container(
        col![
            text(end_text),
            show_grid(g, GridInteraction::None, last_placement),
            // TODO: button to save history.
            button("Return to lobby").on_press(Message::ReturnToLobby),
        ]
        .align_x(Alignment::Center)
        .spacing(10),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn player_turn<'a>(nickname: &'a str, g: &GameState) -> Element<'a, Message> {
    text(format!(
        "{}'s turn",
        player_to_name(
            nickname,
            &g.opponent_nickname,
            g.is_playing_first,
            g.game.active_player()
        )
    ))
    .into()
}

fn player_to_name<'a>(
    nickname: &'a str,
    opponent_nickname: &'a str,
    is_playing_first: bool,
    player: Player,
) -> &'a str {
    if is_player_self(is_playing_first, player) {
        nickname
    } else {
        opponent_nickname
    }
}

fn is_player_self(is_playing_first: bool, player: Player) -> bool {
    match (is_playing_first, player) {
        (true, Player::Player1) | (false, Player::Player2) => true,
        (false, Player::Player1) | (true, Player::Player2) => false,
    }
}

fn is_player_opponent(is_playing_first: bool, player: Player) -> bool {
    !is_player_self(is_playing_first, player)
}

#[derive(Debug, Clone, Copy)]
enum GridInteraction {
    Interactive,
    None,
}

fn show_grid(
    g: &Game,
    grid_interaction: GridInteraction,
    highlight_position: Option<Position>,
) -> Element<'_, Message> {
    let grid = g.grid();

    Column::with_children([File::A, File::D, File::G].map(|file| {
        Row::with_children(
            [Rank::One, Rank::Four, Rank::Seven]
                .map(|rank| show_square(grid, rank, file, grid_interaction, highlight_position)),
        )
        .spacing(SQUARE_SPACING)
        .into()
    }))
    .spacing(SQUARE_SPACING)
    .into()
}

fn show_square(
    grid: &Grid,
    rank: Rank,
    file: File,
    grid_interaction: GridInteraction,
    highlight_position: Option<Position>,
) -> Element<'_, Message> {
    let ranks = rank.band();
    let files = file.stack();
    Column::with_children(files.iter().map(|&d_file| {
        Row::with_children(ranks.iter().map(|&d_rank| {
            let position = Position {
                rank: d_rank,
                file: d_file,
            };

            match grid.get(position) {
                Some(number) => {
                    let mut t = text(number.to_string())
                        .width(Length::Fixed(CELL_SIZE))
                        .height(Length::Fixed(CELL_SIZE))
                        .size(30.0)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center);

                    if let Some(highlight_position) = highlight_position
                        && highlight_position == position
                    {}

                    t.into()
                }
                None => {
                    let mut button = button("")
                        .width(Length::Fixed(CELL_SIZE))
                        .height(Length::Fixed(CELL_SIZE))
                        .style(style_button);

                    if let GridInteraction::Interactive = grid_interaction {
                        button = button.on_press(Message::SudokuButtonClicked(position));
                    }

                    button.into()
                }
            }
        }))
        .spacing(CELL_SPACING)
        .into()
    }))
    .spacing(CELL_SPACING)
    .into()
}

fn style_button(theme: &Theme, status: button::Status) -> Style {
    let mut style = button::primary(theme, status);

    match status {
        button::Status::Hovered => {
            style.background = Some(Color::from_rgb(0.2, 0.6, 0.9).into());
        }
        button::Status::Pressed => {
            style.background = Some(Color::from_rgb(0.1, 0.4, 0.7).into());
        }
        _ => {
            // Default state color
            style.background = Some(Color::from_rgb(0.1, 0.5, 0.8).into());
        }
    }
    style.text_color = Color::WHITE;

    style.border.radius = 0.0.into();

    style
}

fn number_selection(g: &GameState) -> Element<'_, Message> {
    // TODO: use Number::All in a for loop.
    let one = number_to_radio(g.selected_number, Number::One);
    let two = number_to_radio(g.selected_number, Number::Two);
    let three = number_to_radio(g.selected_number, Number::Three);
    let four = number_to_radio(g.selected_number, Number::Four);
    let five = number_to_radio(g.selected_number, Number::Five);
    let six = number_to_radio(g.selected_number, Number::Six);
    let seven = number_to_radio(g.selected_number, Number::Seven);
    let eight = number_to_radio(g.selected_number, Number::Eight);
    let nine = number_to_radio(g.selected_number, Number::Nine);
    row![one, two, three, four, five, six, seven, eight, nine]
        .spacing(10)
        .into()
}

fn number_to_radio(selected_number: Option<Number>, number: Number) -> Element<'static, Message> {
    col![
        text(format!("{number}  ")),
        radio("", number, selected_number, Message::RadioNumberSelected,),
    ]
    .align_x(Alignment::Center)
    .spacing(5)
    .into()
}

fn init_logger() {
    use tracing_subscriber::prelude::*;

    let log_level = format!("{}=debug", env!("CARGO_CRATE_NAME"));
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));
    let (filter, _handle) = tracing_subscriber::reload::Layer::new(filter);

    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    tracing_subscriber::Registry::default()
        .with(filter)
        .with(fmt_layer)
        .try_init()
        .expect("Failed to initialize logger");
}
