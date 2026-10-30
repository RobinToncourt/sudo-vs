#![deny(clippy::unwrap_used)]

mod game;

use futures_timer::Delay;

use std::{fmt, time::Duration};

use iced::{
    Alignment, Color, Element, Length, Subscription, Task, Theme,
    futures::{FutureExt, SinkExt, StreamExt, channel::mpsc, select},
    stream,
    widget::{
        Column, Row, button, button::Style, column as col, container, radio, row, space, text,
        text_input,
    },
};

use log::debug;

use matchbox_socket::{PeerId, PeerState, WebRtcSocket};

use serde::{Deserialize, Serialize};

use crate::game::{Game, GameOutcome, Number, NumberPlacingOutcome, Player};

const CELL_SIZE: f32 = 40.0;
const CELL_SPACING: f32 = 2.0;
const SQUARE_SPACING: f32 = 10.0;

struct GameState {
    is_playing_first: bool,
    opponent_peer_id: PeerId,
    opponent_nickname: String,
    game: Game,
    selected_number: Option<Number>,
    sender: mpsc::Sender<Command>,
}

enum State {
    SelectingLobby {
        /// To show info to the user.
        info: String,
    },
    Connecting {
        opponent_peer_id: Option<PeerId>,
        /// To send command to the `WebRtcSocket` to execute, i.e. communicate online.
        sender: Option<mpsc::Sender<Command>>,
    },
    Game(GameState),
}
impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::SelectingLobby { .. } => "SelectingLobby",
                Self::Connecting { .. } => "Connecting",
                Self::Game { .. } => "Game",
            }
        )
    }
}

struct App {
    nickname: String,
    lobby_name: String,
    state: State,
}
impl Default for App {
    fn default() -> Self {
        Self {
            nickname: String::from("Bob"),
            lobby_name: String::from("test"),
            state: State::SelectingLobby {
                info: String::new(),
            },
        }
        // Self {
        //     nickname: String::from("Bob"),
        //     lobby_name: String::from("test"),
        //     state: State::Game(GameState {
        //         opponent_nickname: String::from("Patrick"),
        //         game: Game::new(),
        //         selected_number: Some(Number::One),
        //         sender: mpsc::channel(100).0,
        //     }),
        // }
    }
}

/// App level messages.
#[derive(Clone)]
enum Message {
    WebSocketEvent(WebSocketEvent),
    InputNickname(String),
    InputLobbyName(String),
    JoinLobby,
    CancelConnection,
    RadioNumberSelected(Number),
    SudokuButtonClicked(usize, usize),
}
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Message::WebSocketEvent(_) => "WebSocketEvent",
                Message::InputNickname(_) => "InputNickname",
                Message::InputLobbyName(_) => "InputLobbyName",
                Message::JoinLobby => "JoinLobby",
                Message::CancelConnection => "CancelConnection",
                Message::RadioNumberSelected(_) => "RadioNumberSelected",
                Message::SudokuButtonClicked(_, _) => "SudokuButtonClicked",
            }
        )
    }
}

/// Events that happens at the `WebRtcSocket` level.
#[derive(Debug, Clone)]
enum WebSocketEvent {
    Ready(mpsc::Sender<Command>),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    Message(PeerId, Vec<u8>),
    Closed,
}

/// Commands to send to the `WebRtcSocket`.
enum Command {
    Send(PeerId, Vec<u8>),
    Broadcast(Vec<u8>),
}

/// The data that will be exchange between players that represent their action.
#[derive(Deserialize, Serialize)]
enum PlayerAction {
    Nickname(String),
    NumberPlacement {
        row: usize,
        column: usize,
        number: Number,
    },
}

fn main() -> iced::Result {
    init_logger();
    iced::application(App::default, update, view)
        .subscription(subscription)
        .run()
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match (&mut app.state, message) {
        /*
         * SelectingLobby.
         */
        (State::SelectingLobby { .. }, Message::JoinLobby) => {
            app.state = State::Connecting {
                opponent_peer_id: None,
                sender: None,
            };
        }
        (State::SelectingLobby { .. }, Message::InputNickname(new_nickname)) => {
            app.nickname = new_nickname;
        }
        (State::SelectingLobby { info, .. }, Message::InputLobbyName(new_lobby_name)) => {
            app.lobby_name = new_lobby_name;
            if !app.lobby_name.chars().all(char::is_alphanumeric) {
                info.clear();
                info.push_str("Lobby name should be alphanumeric");
            }
        }
        /*
         * Connecting.
         */
        (State::Connecting { .. }, Message::CancelConnection) => {
            app.state = State::SelectingLobby {
                info: String::new(),
            };
        }
        (State::Connecting { sender, .. }, Message::WebSocketEvent(WebSocketEvent::Ready(s))) => {
            *sender = Some(s);
        }
        (
            State::Connecting {
                sender,
                opponent_peer_id,
            },
            Message::WebSocketEvent(WebSocketEvent::PeerConnected(peer)),
        ) => {
            if let Some(sender) = sender {
                let player_action = PlayerAction::Nickname(app.nickname.clone());
                match postcard::to_allocvec(&player_action) {
                    Ok(bytes) => {
                        if let Err(e) = sender.try_send(Command::Send(peer, bytes)) {
                            debug!("Failed to queue move: {e}");
                        }
                        *opponent_peer_id = Some(peer);
                    }
                    Err(e) => debug!("Failed to serialize nickname: {e}"),
                }
            } else {
                debug!("No sender to send nickname");
                app.state = State::SelectingLobby {
                    info: "An error occurred while connecting to peer.".into(),
                };
            }
        }
        (
            State::Connecting {
                opponent_peer_id, ..
            },
            Message::WebSocketEvent(WebSocketEvent::PeerDisconnected(peer_id)),
        ) => {
            let Some(opponent_peer_id) = opponent_peer_id else {
                app.state = State::SelectingLobby {
                    info: String::new(),
                };
                return Task::none();
            };

            if *opponent_peer_id == peer_id {
                app.state = State::SelectingLobby {
                    info: String::new(),
                };
            }
        }
        (
            State::Connecting {
                sender,
                opponent_peer_id,
            },
            Message::WebSocketEvent(WebSocketEvent::Message(packet_peer_id, packet)),
        ) => {
            let Some(peer_id) = opponent_peer_id else {
                debug!("Received message but no peer_id registered");
                return Task::none();
            };

            if *peer_id != packet_peer_id {
                debug!(
                    "Received message from another peer. Expected {peer_id}, got {packet_peer_id}"
                );
                return Task::none();
            }

            match postcard::from_bytes(&packet) {
                Ok(PlayerAction::Nickname(opponent_nickname)) => {
                    debug!("Nickname: {opponent_nickname}");
                    let Some(sender) = sender.take() else {
                        debug!("No sender");
                        app.state = State::SelectingLobby {
                            info: "An error occurred while connecting to peer.".into(),
                        };
                        return Task::none();
                    };
                    if let Some(opponent_peer_id) = opponent_peer_id.take() {
                        let is_playing_first = app.nickname < opponent_nickname;
                        app.state = State::Game(GameState {
                            is_playing_first,
                            opponent_peer_id,
                            opponent_nickname,
                            game: Game::new(),
                            selected_number: None,
                            sender,
                        });
                    } else {
                        debug!("Should not happen");
                    }
                }
                Ok(_) => debug!("Ignoring action while connecting"),
                Err(e) => debug!("Malformed packet: {e}"),
            }
        }
        (State::Connecting { .. }, Message::WebSocketEvent(WebSocketEvent::Closed)) => {
            debug!("Connection closed");
            app.state = State::SelectingLobby {
                info: "Connection closed".into(),
            };
        }
        /*
         * Game.
         */
        (State::Game(g), Message::RadioNumberSelected(n)) => g.selected_number = Some(n),
        (State::Game(g), Message::SudokuButtonClicked(row, column)) => {
            if let Some(number) = g.selected_number {
                let player_action = PlayerAction::NumberPlacement {
                    row,
                    column,
                    number,
                };
                match postcard::to_allocvec(&player_action) {
                    Ok(bytes) => {
                        if let Err(e) = g.sender.try_send(Command::Broadcast(bytes)) {
                            debug!("Failed to queue move: {e}");
                        } else {
                            // TODO: handle actual player.
                            game_place_number(g, row, column, number);
                            // TODO: send to opponent.
                        }
                    }
                    Err(e) => debug!("Failed to serialize move: {e}"),
                }
            } else {
                debug!("No number selected");
            }
        }
        (State::Game(g), Message::WebSocketEvent(WebSocketEvent::PeerDisconnected(peer_id))) => {
            debug!("Peer disconnected: {peer_id}");
            if g.opponent_peer_id == peer_id {
                debug!("Opponent disconnected");
                app.state = State::SelectingLobby {
                    info: "Opponent disconnected".into(),
                };
            }
        }
        (State::Game(g), Message::WebSocketEvent(WebSocketEvent::Message(peer_id, bytes))) => {
            if g.opponent_peer_id != peer_id {
                debug!(
                    "Received message from another peer. Expected {}, got {peer_id}",
                    g.opponent_peer_id
                );
                return Task::none();
            }

            match postcard::from_bytes(&bytes) {
                Ok(PlayerAction::NumberPlacement {
                    row,
                    column,
                    number,
                }) => {
                    // TODO: handle actual player.
                    game_place_number(g, row, column, number);
                }
                Ok(_) => debug!("Ignoring nickname while in game"),
                Err(e) => debug!("Malformed packet: {e}"),
            }
        }
        (State::Game(_g), Message::WebSocketEvent(WebSocketEvent::Closed)) => {
            debug!("Connection closed");
            app.state = State::SelectingLobby {
                info: "Connection closed".into(),
            };
        }
        /*
         * Invalid state/message pair.
         */
        (state, msg) => debug!("ignore message({msg}) in current state({state})"),
    }
    Task::none()
}

fn game_place_number(g: &mut GameState, row: usize, column: usize, number: Number) {
    let outcome = g.game.place_number(row, column, number);
    match outcome {
        NumberPlacingOutcome::Ok => debug!("Valid position"),
        NumberPlacingOutcome::CellNotEmpty => {
            // Non-empty cell are not clickable, should not happen.
            debug!("Cell is not empty");
        }
        NumberPlacingOutcome::GameOver(reason) => match reason {
            GameOutcome::Draw => {
                debug!("Draw");
                // TODO: how to handle app state changes ?
            }
            GameOutcome::NumberAlreadyInSquare => debug!("already in square, player lost"),
            GameOutcome::NumberAlreadyInRow => debug!("already in row, player lost"),
            GameOutcome::NumberAlreadyInColumn => debug!("already in column, player lost"),
            GameOutcome::NoValidPositions => debug!("unsupported yet"),
        },
    }
}

fn subscription(app: &App) -> Subscription<Message> {
    match &app.state {
        State::SelectingLobby { .. } => Subscription::none(),
        State::Connecting { .. } | State::Game(_) => connect(format!(
            "ws://localhost:3536/", // {}/?players=2
                                    // app.lobby_name()
        ))
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
                        Command::Send(peer, data) => {
                            debug!("Sending data to {peer}");
                            socket.channel_mut(0).send(data.into(), peer);
                        }
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
                            for (peer, state) in changes {
                                let _ = out.send(match state {
                                    PeerState::Connected => WebSocketEvent::PeerConnected(peer),
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
    match &app.state {
        State::SelectingLobby { info, .. } => {
            let nickname = &app.nickname;
            let lobby_name = &app.lobby_name;
            view_selecting_lobby(info, nickname, lobby_name)
        }
        State::Connecting { .. } => view_connecting(),
        State::Game(g) => view_game(&app.nickname, g),
    }
}

fn view_selecting_lobby<'a>(
    info: &'a str,
    nickname: &'a str,
    lobby_name: &'a str,
) -> Element<'a, Message> {
    container(
        col![
            text(info),
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
        col![player_turn(nickname, g), show_grid(g), number_selection(g)]
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
            g.game.playing()
        )
    ))
    .into()
}

fn player_to_name<'a>(
    nickname: &'a str,
    opponent_nickname: &'a str,
    is_playing_first: bool,
    player: &Player,
) -> &'a str {
    match (is_playing_first, player) {
        (true, Player::Player1) | (false, Player::Player2) => nickname,
        (false, Player::Player1) | (true, Player::Player2) => opponent_nickname,
    }
}

fn show_grid(g: &GameState) -> Element<'_, Message> {
    let grid = g.game.grid();

    Column::with_children((0..3).map(|square_y| {
        Row::with_children((0..3).map(|square_x| show_square(grid, square_x, square_y)))
            .spacing(SQUARE_SPACING)
            .into()
    }))
    .spacing(SQUARE_SPACING)
    .into()
}

fn show_square(
    grid: &[[Option<Number>; 9]; 9],
    square_x: usize,
    square_y: usize,
) -> Element<'_, Message> {
    Column::with_children((0..3).map(|dy| {
        Row::with_children((0..3).map(|dx| {
            let x = square_x * 3 + dx;
            let y = square_y * 3 + dy;

            match grid[x][y] {
                Some(number) => text(number.to_string())
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .size(30.0)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into(),
                None => button("")
                    .on_press(Message::SudokuButtonClicked(x, y))
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .style(style_button)
                    .into(),
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
