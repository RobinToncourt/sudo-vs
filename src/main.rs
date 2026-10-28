#![allow(dead_code, unused)]

mod game;

use futures_timer::Delay;

use std::{fmt, time::Duration};

use iced::{
    Alignment, Color, Element, Length, Subscription, Task, Theme,
    futures::{FutureExt, SinkExt, StreamExt, channel::mpsc, select},
    stream,
    widget::{
        Column, Row, button, button::Style, column as col, container, radio, row, text, text_input,
    },
};

use log::debug;

use matchbox_socket::{PeerId, PeerState, WebRtcSocket};

use serde::{Deserialize, Serialize};

use crate::game::{Game, GameOutcome, Number, NumberPlacingOutcome, Player};

const CELL_SIZE: f32 = 60.0;

/*
 * `lobby_name` is required after the WebRtcSocket connected to keep Iced's Subscription running.
 */
enum App {
    SelectingLobby {
        /// To print info to the user.
        info: String,
        nickname: String,
        lobby_name: String,
    },
    Connecting {
        nickname: String,
        opponent_nickname: Option<String>,
        lobby_name: String,
        /// To send command to the `WebRtcSocket` to execute, i.e. communicate online.
        sender: Option<mpsc::Sender<Command>>,
    },
    Game {
        nickname: String,
        opponent_nickname: String,
        lobby_name: String,
        game: Game,
        selected_number: Option<Number>,
        sender: mpsc::Sender<Command>,
    },
}
impl Default for App {
    fn default() -> Self {
        Self::SelectingLobby {
            info: String::new(),
            nickname: String::new(),
            lobby_name: String::new(),
        }
        // Self::Game {
        //     nickname: String::new(),
        //     opponent_nickname: String::new(),
        //     game: Game::new(),
        //     selected_number: Some(Number::One),
        // }
    }
}
impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                App::SelectingLobby { .. } => "SelectingLobby",
                App::Connecting { .. } => "Connecting",
                App::Game { .. } => "Game",
            }
        )
    }
}
impl App {
    /*
     * Getters.
     */
    fn nickname(&self) -> &String {
        let Self::SelectingLobby { nickname, .. } = self else {
            panic!("Wrong state, expected [SelectingLobby] got [{self}]");
        };
        nickname
    }
    fn nickname_mut(&mut self) -> &mut String {
        let Self::SelectingLobby { nickname, .. } = self else {
            panic!("Wrong state, expected [SelectingLobby] got [{self}]");
        };
        nickname
    }
    fn lobby_name(&self) -> &String {
        match self {
            App::SelectingLobby { lobby_name, .. }
            | App::Connecting { lobby_name, .. }
            | App::Game { lobby_name, .. } => lobby_name,
        }
    }
    fn lobby_name_mut(&mut self) -> &mut String {
        match self {
            App::SelectingLobby { lobby_name, .. }
            | App::Connecting { lobby_name, .. }
            | App::Game { lobby_name, .. } => lobby_name,
        }
    }
    fn get_sender(&mut self) -> &mut mpsc::Sender<Command> {
        match self {
            App::SelectingLobby { .. } => {
                panic!("Wrong state, expected [Connecting/Game] got [{self}]")
            }
            App::Connecting { sender, .. } => sender.as_mut().unwrap(),
            App::Game { sender, .. } => sender,
        }
    }
    fn set_sender(&mut self, sender: mpsc::Sender<Command>) {
        if let Self::Connecting {
            sender: opt_sender, ..
        } = self
        {
            *opt_sender = Some(sender);
        }
    }
    #[allow(clippy::ref_option)]
    fn selected_number(&self) -> &Option<Number> {
        let Self::Game {
            selected_number, ..
        } = self
        else {
            panic!("Wrong state, expected [Game] got [{self}]");
        };
        selected_number
    }
    fn selected_number_mut(&mut self) -> &mut Option<Number> {
        let Self::Game {
            selected_number, ..
        } = self
        else {
            panic!("Wrong state, expected [Game] got [{self}]");
        };
        selected_number
    }
    fn game(&self) -> &Game {
        let Self::Game { game, .. } = self else {
            panic!("Wrong state, expected [Game] got [{self}]");
        };
        game
    }
    fn game_mut(&mut self) -> &mut Game {
        let Self::Game { game, .. } = self else {
            panic!("Wrong state, expected [Game] got [{self}]");
        };
        game
    }

    /*
     * State transition.
     */
    /// Change state from `SelectingLobby` to `Connecting`.
    fn transition_to_connecting(&mut self) {
        let Self::SelectingLobby {
            nickname,
            lobby_name,
            ..
        } = self
        else {
            panic!("Wrong state, expected [SelectingLobby] got [{self}]");
        };

        *self = Self::Connecting {
            nickname: nickname.clone(),
            opponent_nickname: None,
            lobby_name: lobby_name.clone(),
            sender: None,
        };
    }
    /// Change state from `Connecting` to `Game`.
    fn transition_to_game(&mut self) {
        let Self::Connecting {
            nickname,
            opponent_nickname,
            lobby_name,
            sender,
        } = self
        else {
            panic!("Wrong state, expected [Connecting] got [{self}]");
        };
        *self = Self::Game {
            nickname: nickname.clone(),
            opponent_nickname: opponent_nickname
                .clone()
                .expect("opponent nickname should be set"),
            lobby_name: lobby_name.clone(),
            game: Game::new(),
            selected_number: None,
            sender: sender.take().expect("the sender should be set"),
        }
    }
    /// Change other states to `SelectingLobby`.
    fn transition_to_selecting_lobby(&mut self, info: String) {
        let (nickname, lobby_name) = match self {
            App::SelectingLobby { .. } => panic!("Wrong state, expected not [SelectingLobby]."),
            App::Connecting {
                nickname,
                lobby_name,
                ..
            }
            | App::Game {
                nickname,
                lobby_name,
                ..
            } => (nickname, lobby_name),
        };

        *self = Self::SelectingLobby {
            nickname: nickname.clone(),
            lobby_name: lobby_name.clone(),
            info,
        }
    }

    fn can_join_lobby(&self) -> bool {
        let is_all_alphanumeric = self.lobby_name().chars().all(char::is_alphanumeric);
        !self.nickname().is_empty() && !self.lobby_name().is_empty() && is_all_alphanumeric
    }
}

/// App level messages.
#[derive(Clone)]
enum Message {
    WebSocketEvent(WebSocketEvent),
    InputNickname(String),
    InputLobbyName(String),
    JoinLobby,
    RadioNumberSelected(Number),
    SudokuButtonClicked(usize, usize),
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
struct PlayerAction {
    row: usize,
    column: usize,
    number: Number,
}

fn main() -> iced::Result {
    println!("Hello, sudo-vs!");
    init_logger();

    iced::application(App::default, update, view).run()
}

#[allow(clippy::needless_pass_by_value)]
fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::WebSocketEvent(event) => match event {
            WebSocketEvent::Ready(sender) => app.set_sender(sender),
            WebSocketEvent::PeerConnected(peer) => {}
            WebSocketEvent::PeerDisconnected(peer) => {}
            WebSocketEvent::Message(peer, packet) => {}
            WebSocketEvent::Closed => {}
        },
        Message::InputNickname(nickname) => *app.nickname_mut() = nickname,
        Message::InputLobbyName(lobby_name) => *app.lobby_name_mut() = lobby_name,
        Message::JoinLobby => {
            if app.can_join_lobby() {
                app.transition_to_connecting();
                debug!("Join lobby");
            }
        }
        Message::RadioNumberSelected(number) => *app.selected_number_mut() = Some(number),
        Message::SudokuButtonClicked(row, column) => {
            if let Some(selected_number) = *app.selected_number() {
                let outcome = app.game_mut().place_number(row, column, selected_number);
                match outcome {
                    NumberPlacingOutcome::Ok => debug!("Valid position"),
                    NumberPlacingOutcome::CellNotEmpty => {
                        debug!("Cell is not empty");
                    }
                    NumberPlacingOutcome::GameOver(reason) => match reason {
                        GameOutcome::Draw => debug!("Draw"),
                        GameOutcome::NumberAlreadyInSquare => debug!("already in square"),
                        GameOutcome::NumberAlreadyInRow => debug!("already in row"),
                        GameOutcome::NumberAlreadyInColumn => debug!("already in column"),
                        GameOutcome::NoValidPositions => debug!("unsupported yet"),
                    },
                }
            } else {
                debug!("No number selected");
            }
        }
    }
    Task::none()
}

fn subscription(app: &App) -> Subscription<Message> {
    connect(format!(
        "ws://localhost:3536/{}/?players=2",
        app.lobby_name()
    ))
    .map(Message::WebSocketEvent)
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
                        Command::Send(peer, data) =>
                            socket.channel_mut(0).send(data.into(), peer),
                        Command::Broadcast(data) => {
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
            std::future::pending::<()>().await; // a finished subscription stream can cause trouble; park it
        })
    })
}

fn view(app: &App) -> Element<'_, Message> {
    match app {
        App::SelectingLobby { .. } => view_selecting_lobby(app),
        App::Connecting { .. } => view_connecting(app),
        App::Game { .. } => view_game(app),
    }
}

fn view_selecting_lobby(app: &App) -> Element<'_, Message> {
    container(col![
        row![
            text("Nickname: "),
            text_input("Enter your nickname", app.nickname()).on_input(Message::InputNickname),
        ],
        row![
            text("Lobby: "),
            text_input("Lobby name", app.lobby_name()).on_input(Message::InputLobbyName),
        ],
        button("Join").on_press(Message::JoinLobby)
    ])
    .into()
}

fn view_connecting(_app: &App) -> Element<'_, Message> {
    container(text("Connecting.../Waiting for someone to connect").size(120))
        .center(Length::Fill)
        .into()
}

fn view_game(app: &App) -> Element<'_, Message> {
    container(
        col![player_turn(app), show_grid(app), number_selection(app),]
            .align_x(Alignment::Center)
            .spacing(10),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn player_turn(app: &App) -> Element<'_, Message> {
    text(format!("{}'s turn", player_to_name(app.game().playing()))).into()
}

fn player_to_name(player: &Player) -> &'static str {
    match player {
        Player::Player1 => "Bob",
        Player::Player2 => "Patrick",
    }
}

fn show_grid(app: &App) -> Element<'_, Message> {
    let grid = app.game().grid();

    let mut rows = Vec::with_capacity(grid.len());
    for (x, outer) in grid.iter().enumerate() {
        let mut row: Row<'_, Message> = Row::new();
        for (y, inner) in outer.iter().enumerate() {
            if let Some(number) = inner {
                let t = text(number.to_string())
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .size(30.0)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center);
                row = row.push(t);
            } else {
                let b = button("")
                    .on_press(Message::SudokuButtonClicked(x, y))
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .style(style_button);
                row = row.push(b);
            }
        }
        rows.push(row.spacing(1).into());
    }

    Column::from_iter(rows).spacing(1).into()
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

fn number_selection(app: &App) -> Element<'_, Message> {
    let one = number_to_radio(app, Number::One);
    let two = number_to_radio(app, Number::Two);
    let three = number_to_radio(app, Number::Three);
    let four = number_to_radio(app, Number::Four);
    let five = number_to_radio(app, Number::Five);
    let six = number_to_radio(app, Number::Six);
    let seven = number_to_radio(app, Number::Seven);
    let eight = number_to_radio(app, Number::Eight);
    let nine = number_to_radio(app, Number::Nine);
    row![one, two, three, four, five, six, seven, eight, nine]
        .spacing(10)
        .into()
}

fn number_to_radio(app: &App, number: Number) -> Element<'_, Message> {
    col![
        text(format!("{number}  ")),
        radio(
            "",
            number,
            *app.selected_number(),
            Message::RadioNumberSelected,
        ),
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
