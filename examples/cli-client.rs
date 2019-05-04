/*
 * A networking library for the multiplayer game, Conwayste.
 *
 * Copyright (C) 2018-2019 The Conwayste Developers
 *
 * This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU General Public License as published by the Free
 * Software Foundation, either version 3 of the License, or (at your option)
 * any later version.
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of  MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate netwayste;
extern crate env_logger;
extern crate futures;
extern crate tokio_core;
#[macro_use] extern crate log;

use std::io::{self, Read, Write};
use std::process;
use std::str::FromStr;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::channel as std_channel;
use std::thread;

use chrono::Local;
use log::LevelFilter;
use netwayste::{net::NetwaysteEvent, client::{ClientNetState, CLIENT_VERSION}};
use futures::sync::mpsc;

#[derive(PartialEq, Debug, Clone)]
enum UserInput {
    Command{cmd: String, args: Vec<String>},
    Chat(String),
}

// At this point we should only have command or chat message to work with
fn parse_stdin(mut input: String) -> UserInput {
    if input.get(0..1) == Some("/") {
        // this is a command
        input.remove(0);  // remove initial slash

        let mut words: Vec<String> = input.split_whitespace().map(|w| w.to_owned()).collect();

        let command = if words.len() > 0 {
            words.remove(0).to_lowercase()
        } else {
            "".to_owned()
        };

        UserInput::Command{cmd: command, args: words}
    } else {
            UserInput::Chat(input)
    }
}

// Our helper method which will read data from stdin and send it along the
// sender provided. This is blocking so should be on separate thread.
fn read_stdin(channel_to_netwayste: mpsc::UnboundedSender<NetwaysteEvent>) {
    let mut stdin = io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf) {
            Err(_) |
            Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        let string = String::from_utf8(buf).unwrap();
        let string = String::from_str(string.trim()).unwrap();
        if !string.is_empty() && string != "" {
            let user_input = parse_stdin(string);
            let event = handle_user_input_event(user_input);
            match channel_to_netwayste.unbounded_send(event) {
                Ok(_) => {},
                Err(e) => error!("Could not send event to netwayste thread: {}", e)
            }
        }
    }
}

fn print_help() {
    info!("");
    info!("/help                  - print this text");
    info!("/connect <player_name> - connect to server");
    info!("/disconnect            - disconnect from server");
    info!("/list                  - list rooms when in lobby, or players when in game");
    info!("/new <room_name>       - create a new room (when not in game)");
    info!("/join <room_name>      - join a room (when not in game)");
    info!("/leave                 - leave a room (when in game)");
    info!("/part                  - alias of leave");
    info!("/quit                  - exit the program");
    info!("...or just type text to chat!");
}

fn build_command_request_action(cmd: String, args: Vec<String>) -> NetwaysteEvent {
    let mut new_event: NetwaysteEvent = NetwaysteEvent::None;
    // keep these in sync with print_help function
    match cmd.as_str() {
        "help" | "?" | "h" => {
            print_help();
        }
        "connect" | "c" => {
            if args.len() == 1 {
                new_event = NetwaysteEvent::Connect(args[0].clone(), CLIENT_VERSION.to_owned());
            } else { error!("Expected client name as the sole argument (no spaces allowed)."); }
        }
        "disconnect" | "d" => {
            if args.len() == 0 {
                new_event = NetwaysteEvent::Disconnect;
            } else { debug!("Command failed: Expected no arguments to disconnect"); }
        }
        "list" | "l" => {
            if args.len() == 0 {
                new_event = NetwaysteEvent::List;
            } else { debug!("Command failed: Expected no arguments to list"); }
        }
        "new" | "n" => {
            if args.len() == 1 {
                new_event = NetwaysteEvent::NewRoom(args[0].clone());
            } else { debug!("Command failed: Expected name of room (no spaces allowed)"); }
        }
        "join" | "j" => {
            if args.len() == 1 {
                new_event = NetwaysteEvent::JoinRoom(args[0].clone());
            } else { debug!("Command failed: Expected room name only (no spaces allowed)"); }
        }
        "part" | "leave" => {
            if args.len() == 0 {
                new_event = NetwaysteEvent::LeaveRoom;
            } else { debug!("Command failed: Expected no arguments to leave"); }
        }
        "quit" | "q" | "exit" => {
            trace!("Peace out!");
            new_event = NetwaysteEvent::Disconnect;
        }
        _ => {
            debug!("Command not recognized: {}", cmd);
        }
    }
    return new_event;
}

fn handle_user_input_event(user_input: UserInput) -> NetwaysteEvent {
    match user_input {
        UserInput::Chat(string) => {
            NetwaysteEvent::ChatMessage(string)
        }
        UserInput::Command{cmd, args} => {
            build_command_request_action(cmd, args)
        }
    }
}

fn main() {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                "{} [{:5}] - {}",
                Local::now().format("%a %Y-%m-%d %H:%M:%S%.6f"),
                record.level(),
                record.args(),
            )
        })
        .filter(None, LevelFilter::Trace)
        .filter(Some("futures"), LevelFilter::Off)
        .filter(Some("tokio_core"), LevelFilter::Off)
        .filter(Some("tokio_reactor"), LevelFilter::Off)
        .filter(Some("conway"), LevelFilter::Off)
        .filter(Some("ggez"), LevelFilter::Off)
        .filter(Some("gfx_device_gl"), LevelFilter::Off)
        .filter(Some("netwayste"), LevelFilter::Info)   //Ignore Trace events can be noisy, keep all others
        .init();

    let (ggez_client_request, nw_client_request) = mpsc::unbounded::<NetwaysteEvent>();
    let (nw_server_response, ggez_server_response) = std_channel::<NetwaysteEvent>();
    thread::spawn(move || {
        ClientNetState::start_network(nw_server_response, nw_client_request);
    });

    thread::spawn(move || {
        read_stdin(ggez_client_request);
    });

    info!("Type /help for more info...");

    loop {
        match ggez_server_response.try_recv() {
            Ok(response_code) => {
                println!("{:?}", response_code);
            }
            Err(TryRecvError::Empty) => {
                // Nothing to do in the empty case
            }
            Err(e) => {
                println!("Got error from ResponseCode stream from netwayste thread: {:?}", e);
                process::exit(1);
            }
        }
    }
}
