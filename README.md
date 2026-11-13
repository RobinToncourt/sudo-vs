# Sudo-vs

A two-player, peer-to-peer duel played on a Sudoku grid, written in Rust and running in the browser (WebAssembly).

Instead of solving a puzzle together, you and your opponent take turns filling the **same empty grid**. The first player to break a Sudoku rule loses.

**Play online:** <https://robintoncourt.github.io/sudo-vs/>

## Table of contents

- [Game mechanics](#game-mechanics)
- [Peer-to-peer: play with people you trust](#peer-to-peer-play-with-people-you-trust)
- [Getting started](#getting-started)
- [Project layout](#project-layout)
- [TODO](#todo)
- [Use of AI](#use-of-ai)
- [License](#license)

## Game mechanics

### A reminder of Sudoku's rules

A Sudoku grid is a 9×9 board split into nine 3×3 boxes. Each cell holds a number from 1 to 9, and the same number may appear **only once** in:

- each **row** (rank),
- each **column** (file),
- each **3×3 box**.

In classic Sudoku you start from a partially filled grid and complete it. In Sudo-vs the grid **starts empty** and the two players fill it together.

### Sudo-vs rules

1. Two players share one 9×9 grid, initially empty.
2. Players alternate turns. On your turn you pick a number (1–9) and an **empty** cell, and place it.
3. If the number you place already appears in the same row, column or 3×3 box, you **break a Sudoku rule and lose immediately**. The conflicting cell and your last move are highlighted on the end screen.
4. If the grid gets completely filled without any violation, the game is a **draw**.
5. You can **forfeit** at any time; your opponent wins.
6. A player who is disconnected or whose connection closes ends the game.

The strategy is to place numbers that are still legal while steering the game towards a position where your opponent has no safe move left.

Which player starts is decided automatically when the connection is established (by comparing the two peers' IDs).

### Notation

Cells are written `rank-file`, with ranks `1`–`9` and files `A`–`I`. A move is `rank-file-number`: `5-C-7` means "put a 7 in rank 5, file C". A game history is a list of moves separated by `;`, for example `5-C-7;5-D-3;1-A-9;`.

### Playing

1. Enter a **nickname** and a **lobby name**. Both players must enter the **same lobby name**.
2. Join the lobby and wait for your opponent.
3. On your turn, select a number with the radio buttons, then click a cell.
4. The last placed number is highlighted so you can follow the opponent's moves.

The interface is available in **English** and **French**, with a selectable theme.

## Peer-to-peer: play with people you trust

Sudo-vs uses **WebRTC** through [matchbox](https://github.com/johanhelsing/matchbox). A small **signaling server** only introduces the two players to each other; once connected, the game messages travel **directly between the two browsers**, with no game server in the middle.

This is great for latency and for not needing a game backend, but it has a privacy consequence you should be aware of:

> **A direct connection means your opponent's machine learns your IP address (and you learn theirs).**

To establish the connection, WebRTC peers exchange network candidates that contain IP addresses. A malicious opponent can read them with standard browser or network tools. An IP address can be used to infer an approximate location (typically the city or region), and in some cases more.

Some practical advice:

- **Only play with people you trust**, or at least be aware of who is in your lobby.
- **Choose a lobby name that is hard to guess** and share it only with your opponent. Anyone who knows the lobby name can try to join it.
- If you are concerned, **use a VPN** (or a similar tool) so that the address your opponent sees is not your home address.
- The **signaling server operator** can also see connection metadata (IP addresses, lobby names). The default server is a public one; you can run your own (see below) and point the game to it from the lobby screen.
- There is **no authentication and no anti-cheat**. The game logic runs on both clients; nothing prevents a modified client from misbehaving.

## Getting started

### Requirements

- A recent stable Rust toolchain with the WebAssembly target:
  ```sh
  rustup target add wasm32-unknown-unknown
  ```
- [Trunk](https://trunkrs.dev/) to build and serve the web app:
  ```sh
  cargo install trunk
  ```

### Run locally
```sh
cargo run
```
or
```sh
trunk serve
```

Then open the printed address in **two browser tabs or windows** (or two machines), enter the same lobby name in both, and play.

### Build for release

```sh
trunk build --release
```

The output is written to `dist/`. A GitHub Actions workflow (`.github/workflows/pages.yml`) builds the project with Trunk and deploys it to GitHub Pages on every push to `main`.

### Signaling server

The game needs a matchbox-compatible signaling server. The lobby screen has a **signaling server URL** field, where `<lobby_name>` is replaced by the lobby name you entered:

```
wss://web-socket-signaling-server.onrender.com/<lobby_name>
```

That is the default. If it is hosted on a free tier, the first connection after a period of inactivity may take a while because the server has to wake up.

To run your own server locally:

```sh
cargo install matchbox_server
matchbox_server
```

and use `ws://localhost:3536/<lobby_name>` as the signaling server URL.

### Tests

The game rules live in a UI-independent module and are covered by unit tests:

```sh
cargo test
```

## Project layout

| Path                          | Role                                                                                                                                                       |
|-------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `src/game.rs`                 | Pure game logic: grid, ranks/files, placements, history, rule checking, results. No UI, no networking.                                                     |
| `src/main.rs`                 | [Iced](https://iced.rs/) application: state machine (lobby → connection → handshake → game → game over), localization, and the WebRTC socket subscription. |
| `src/clock.rs`                | Application clock.                                                                                                                                         |
| `src/update.rs`               | Application update.                                                                                                                                        |
| `src/view.rs`                 | Application view.                                                                                                                                          |
| `index.html`                  | Trunk entry point.                                                                                                                                         |
| `.github/workflows/pages.yml` | Build and deployment to GitHub Pages.                                                                                                                      |

Main dependencies: [`iced`](https://iced.rs/) / `iced_aw` (GUI), [`matchbox_socket`](https://github.com/johanhelsing/matchbox) (WebRTC), [`postcard`](https://github.com/jamesmunns/postcard) + `serde` (network serialization), `futures-timer` (timers that work on WebAssembly).

## TODO

### Features

- [ ] **Record forfeits in the game history.** The history only stores number placements today; extend its entries and its text format so a forfeit appears as a move.
- [ ] **Download the game history.** Add a "save history" button on the game over screen that exports the history as a file.
- [ ] **Replay a game from a history file.** Load a history file and step through the game move by move.
- [ ] **AI opponent.** Let a player face the computer instead of a human.

### Game logic

- [ ] **Detect when a player has no safe move left** (`src/game.rs`). `LossReason::NoValidPositions` already exists and is handled by the UI, but the engine never produces it: the game currently continues until someone actually places a conflicting number.

### Interface

- [ ] Show a message when the lobby name is wrong or invalid (`src/main.rs`).
- [ ] Add a toggle to hide or reveal the signaling server field (`src/main.rs`).

### Code cleanup

- [ ] Build the number selector from `Number::All` in a loop instead of nine handwritten lines (`src/main.rs`).

### Ideas

- [ ] **Reconnection** by resending the move history to a player who dropped.

## Use of AI

An AI assistant(Claude) was used as a coding assistant during the development of this project (design discussions, debugging help, refactoring suggestions and code review). The author remains responsible for the final code and design decisions.

## License

Released under the [MIT License](LICENSE). Copyright (c) 2026 Robin Toncourt.