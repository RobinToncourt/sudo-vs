#![deny(clippy::unwrap_used)]

mod clock;
mod game;
mod update;
mod view;

use std::{
    fmt::{
        self,
        Display, //
    },
    time::{
        Duration,
        Instant, //
    },
};

use iced::{
    Color,
    Subscription,
    Theme,
    futures::{
        FutureExt,
        SinkExt,
        StreamExt,
        channel::mpsc,
        select, //
    },
    stream, //
};

use futures_timer::Delay;

use log::debug;

use matchbox_socket::{
    PeerId,
    PeerState,
    WebRtcSocket, //
};

use serde::{
    Deserialize,
    Serialize, //
};

use crate::{
    clock::Clock,
    game::{
        Game,
        Number,
        Placement,
        Player,
        Position, //
    },
    update::update,
    view::view,
};

const PROTOCOL_VERSION: u16 = 1;

const CELL_SIZE: f32 = 40.0;
const CELL_SPACING: f32 = 2.0;
const SQUARE_SPACING: f32 = 10.0;
const LAST_POSITION_COLOR: Color = Color::from_rgb(0.984_313, 0.749_019, 0.141_176);

#[derive(Debug, Clone, Copy)]
enum LobbyInfo {
    None,
    IncompatibleVersion,
    PeerDisconnectedDuringConnection,
    ConnectionClosed,
    AnErrorOccurred,
}
impl LobbyInfo {
    fn to_str(self, loc: Localization) -> &'static str {
        match self {
            Self::None => "",
            Self::IncompatibleVersion => loc.incompatible_version(),
            Self::PeerDisconnectedDuringConnection => loc.peer_disconnected_during_connection(),
            Self::ConnectionClosed => loc.connection_closed(),
            Self::AnErrorOccurred => loc.an_error_occurred(),
        }
    }
}

struct GameState {
    is_playing_first: bool,
    opponent_peer_id: PeerId,
    opponent_nickname: String,
    now: Instant,
    clock: Clock,
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
    PlayerTimeout,
    OpponentTimeout,
    Desynchronized,
    OpponentDisconnected,
    ConnectionClosed,
}
impl GameOverInfo {
    fn to_str(self, loc: Localization) -> &'static str {
        match self {
            Self::Finished => loc.game_finished(),
            Self::PlayerForfeit => loc.player_forfeit(),
            Self::OpponentForfeit => loc.opponent_forfeit(),
            Self::PlayerTimeout => loc.player_timeout(),
            Self::OpponentTimeout => loc.opponent_timeout(),
            Self::Desynchronized => loc.desynchronized(),
            Self::OpponentDisconnected => loc.opponent_disconnected(),
            Self::ConnectionClosed => loc.connection_closed(),
        }
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

#[derive(Debug, Clone, Copy)]
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

    fn nickname(self) -> &'static str {
        match self {
            Self::En => "Nickname: ",
            Self::Fr => "Pseudonyme: ",
        }
    }
    fn nickname_placeholder(self) -> &'static str {
        match self {
            Self::En => "Enter your nickname",
            Self::Fr => "Entrez votre pseudonyme",
        }
    }
    fn lobby(self) -> &'static str {
        match self {
            Self::En => "Lobby: ",
            Self::Fr => "Salon: ",
        }
    }
    fn lobby_placeholder(self) -> &'static str {
        match self {
            Self::En => "Lobby name",
            Self::Fr => "Nom du salon",
        }
    }
    fn incompatible_version(self) -> &'static str {
        match self {
            Self::En => "Incompatible version, update the game.",
            Self::Fr => "Version incompatible, mettez à jour le jeu.",
        }
    }
    fn peer_disconnected_during_connection(self) -> &'static str {
        match self {
            Self::En => "The peer disconnected during connection.",
            Self::Fr => "Le pair s'est déconnecté pendant la connexion.",
        }
    }
    #[allow(clippy::unused_self)]
    fn signaling_server_url(self) -> &'static str {
        "Signaling server URL: "
    }
    #[allow(clippy::unused_self)]
    fn signaling_server_url_placeholder(self) -> &'static str {
        "Signaling server URL"
    }
    fn join(self) -> &'static str {
        match self {
            Self::En => "Join",
            Self::Fr => "Joindre",
        }
    }
    fn waiting_connection(self) -> &'static str {
        match self {
            Self::En => "Connecting / Waiting for someone to connect...",
            Self::Fr => "Connexion / En attente de connexion...",
        }
    }
    fn cancel(self) -> &'static str {
        match self {
            Self::En => "Cancel",
            Self::Fr => "Annuler",
        }
    }
    fn forfeit(self) -> &'static str {
        match self {
            Self::En => "Forfeit",
            Self::Fr => "Abandonner",
        }
    }
    fn draw(self) -> &'static str {
        match self {
            Self::En => "Draw.",
            Self::Fr => "Égalité.",
        }
    }
    fn player_lost(self) -> &'static str {
        match self {
            Self::En => "You lost.",
            Self::Fr => "Vous avez perdu.",
        }
    }
    fn player_win(self) -> &'static str {
        match self {
            Self::En => "You won.",
            Self::Fr => "Vous avez gagné.",
        }
    }
    fn player_forfeit(self) -> &'static str {
        match self {
            Self::En => "You forfeited.",
            Self::Fr => "Vous avez abandonné.",
        }
    }
    fn opponent_forfeit(self) -> &'static str {
        match self {
            Self::En => "Your opponent forfeited.",
            Self::Fr => "Votre adversaire a abandonné.",
        }
    }
    // PlayerTimeout,
    // OpponentTimeout,
    // Desynchronized,
    fn player_timeout(self) -> &'static str {
        match self {
            Self::En => "You lost, timeout.",
            Self::Fr => "Perdu, plus de temps.",
        }
    }
    fn opponent_timeout(self) -> &'static str {
        match self {
            Self::En => "You won, opponent timeout.",
            Self::Fr => "Gagné, votre adversaire n'a plus de temps.",
        }
    }
    fn desynchronized(self) -> &'static str {
        match self {
            Self::En => "Error, clocks desynchronized.",
            Self::Fr => "Erreur, les horloges sont désynchronisées.",
        }
    }
    fn opponent_disconnected(self) -> &'static str {
        match self {
            Self::En => "Your opponent disconnected.",
            Self::Fr => "Votre adversaire s'est déconnecté.",
        }
    }
    fn connection_closed(self) -> &'static str {
        match self {
            Self::En => "The connection closed.",
            Self::Fr => "La connexion a été interrompu",
        }
    }
    fn game_finished(self) -> &'static str {
        match self {
            Self::En => "The game is finished.",
            Self::Fr => "La partie est terminé.",
        }
    }
    fn return_to_lobby(self) -> &'static str {
        match self {
            Self::En => "Return to lobby.",
            Self::Fr => "Retourner au salon.",
        }
    }
    fn an_error_occurred(self) -> &'static str {
        match self {
            Self::En => "An error occurred.",
            Self::Fr => "Une erreur est survenu.",
        }
    }
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
            nickname: String::new(),
            lobby_name: String::new(),
            signaling_server_url: String::from(
                "wss://web-socket-signaling-server.onrender.com/<lobby_name>",
            ), // ws://localhost:3536
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
    Tick(Instant),
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
                Self::SelectedLocalization(_) => "Selected Localization",
                Self::WebSocketEvent(_) => "WebSocketEvent",
                Self::InputNickname(_) => "InputNickname",
                Self::InputLobbyName(_) => "InputLobbyName",
                Self::InputSignalingServer(_) => "InputSignalingServer",
                Self::JoinLobby => "JoinLobby",
                Self::CancelConnection => "CancelConnection",
                Self::RadioNumberSelected(_) => "RadioNumberSelected",
                Self::SudokuButtonClicked(_) => "SudokuButtonClicked",
                Self::Tick(_) => "Tick",
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
    Hello {
        protocol: u16,
        nickname: String,
    },
    NumberPlacement {
        ply: u16,
        placement: Placement,
        remaining_ms: u32,
    },
    Forfeit,
    Timeout,
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

fn subscription(app: &App) -> Subscription<Message> {
    match &app.state {
        State::SelectingLobby { .. } => Subscription::none(),
        State::WaitingForSocket
        | State::WaitingForPeer { .. }
        | State::Handshaking { .. }
        | State::GameOver { .. } => {
            let room_url = app
                .signaling_server_url
                .replace("<lobby_name>", &app.lobby_name);
            connect(room_url).map(Message::WebSocketEvent)
        }
        State::Game(_) => {
            let room_url = app
                .signaling_server_url
                .replace("<lobby_name>", &app.lobby_name);
            Subscription::batch([connect(room_url).map(Message::WebSocketEvent), ticks()])
        }
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
                            for (peer, state) in changes {
                                let event = match state {
                                    PeerState::Connected => match socket.id() {
                                        Some(me) => WebSocketEvent::PeerConnected { me, peer },
                                        None => {
                                            debug!("Peer connected but local id is not assigned yet.");
                                            continue;
                                        }
                                    },
                                    PeerState::Disconnected => WebSocketEvent::PeerDisconnected(peer),
                                };
                                let _ = out.send(event).await;
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

fn ticks() -> Subscription<Message> {
    Subscription::run(|| {
        stream::channel(1, |mut out: mpsc::Sender<Message>| async move {
            loop {
                Delay::new(Duration::from_millis(200)).await;
                if out.send(Message::Tick(Instant::now())).await.is_err() {
                    break;
                }
            }
        })
    })
}

fn is_player_turn(is_playing_first: bool, player: Player) -> bool {
    match (is_playing_first, player) {
        (true, Player::Player1) | (false, Player::Player2) => true,
        (false, Player::Player1) | (true, Player::Player2) => false,
    }
}

fn is_opponent_turn(is_playing_first: bool, player: Player) -> bool {
    !is_player_turn(is_playing_first, player)
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
