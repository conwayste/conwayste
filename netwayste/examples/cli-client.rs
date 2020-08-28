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

#[macro_use]
extern crate log;
extern crate color_backtrace;
extern crate env_logger;
#[macro_use]
extern crate futures;
extern crate netwayste;
extern crate tokio;

use std::io::{self, Read, Write};
use std::str::FromStr;
use std::thread;

use chrono::Local;
use futures as Fut;
use log::LevelFilter;
use netwayste::{
    client::{ClientNetState, CLIENT_VERSION},
    net::NetwaysteEvent,
    utils::PingPong,
};
use Fut::{channel::mpsc, StreamExt};

#[derive(PartialEq, Debug, Clone)]
enum UserInput {
    Command { cmd: String, args: Vec<String> },
    Chat(String),
}

// At this point we should only have command or chat message to work with
fn parse_stdin(mut input: String) -> UserInput {
    if input.get(0..1) == Some("/") {
        // this is a command
        input.remove(0); // remove initial slash

        let mut words: Vec<String> = input.split_whitespace().map(|w| w.to_owned()).collect();

        let command = if words.len() > 0 {
            words.remove(0).to_lowercase()
        } else {
            "".to_owned()
        };

        UserInput::Command {
            cmd:  command,
            args: words,
        }
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
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        let string = String::from_utf8(buf).unwrap();
        let string = String::from_str(string.trim()).unwrap();
        if !string.is_empty() && string != "" {
            let user_input = parse_stdin(string);
            let event = handle_user_input_event(user_input);
            match channel_to_netwayste.unbounded_send(event) {
                Ok(_) => {}
                Err(e) => error!("Could not send event to netwayste thread: {}", e),
            }
        }
    }
}

fn print_help() {
    info!("");
    info!("/help                  - print this text");
    info!("/status                - get the server's status");
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
        "status" | "s" => {
            let ping = PingPong::ping();
            new_event = NetwaysteEvent::GetStatus(ping);
        }
        "connect" | "c" => {
            if args.len() == 1 {
                new_event = NetwaysteEvent::Connect(args[0].clone(), CLIENT_VERSION.to_owned());
            } else {
                error!("Expected client name as the sole argument (no spaces allowed).");
            }
        }
        "disconnect" | "d" => {
            if args.len() == 0 {
                new_event = NetwaysteEvent::Disconnect;
            } else {
                debug!("Command failed: Expected no arguments to disconnect");
            }
        }
        "list" | "l" => {
            if args.len() == 0 {
                new_event = NetwaysteEvent::List;
            } else {
                debug!("Command failed: Expected no arguments to list");
            }
        }
        "new" | "n" => {
            if args.len() == 1 {
                new_event = NetwaysteEvent::NewRoom(args[0].clone());
            } else {
                debug!("Command failed: Expected name of room (no spaces allowed)");
            }
        }
        "join" | "j" => {
            if args.len() == 1 {
                new_event = NetwaysteEvent::JoinRoom(args[0].clone());
            } else {
                debug!("Command failed: Expected room name only (no spaces allowed)");
            }
        }
        "part" | "leave" => {
            if args.len() == 0 {
                new_event = NetwaysteEvent::LeaveRoom;
            } else {
                debug!("Command failed: Expected no arguments to leave");
            }
        }
        "quit" | "q" | "exit" => {
            trace!("Peace out!");
            new_event = NetwaysteEvent::Disconnect;
        }
        _ => {
            debug!("Command not recognized: {}", cmd);
        }
    }
    new_event
}

fn handle_user_input_event(user_input: UserInput) -> NetwaysteEvent {
    match user_input {
        UserInput::Chat(string) => NetwaysteEvent::ChatMessage(string),
        UserInput::Command { cmd, args } => build_command_request_action(cmd, args),
    }
}

#[tokio::main]
async fn main() {
    color_backtrace::install();
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
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
        .filter(Some("netwayste"), LevelFilter::Info) //Ignore Trace events can be noisy, keep all others
        .init();

    let (ggez_client_request, nw_client_request) = mpsc::unbounded::<NetwaysteEvent>();
    let (nw_server_response, mut ggez_server_response) = mpsc::channel::<NetwaysteEvent>(5);

    tokio::spawn(async {
        match ClientNetState::start_network(nw_server_response, nw_client_request).await {
            Ok(()) => {}
            Err(e) => error!("Error during ClientNetState: {}", e),
        }
    });

    thread::spawn(move || {
        read_stdin(ggez_client_request);
    });

    info!("Type /help for more info...");

    loop {
        select! {
            response = ggez_server_response.next() => {
                if let Some(event) = response {
                    if let NetwaysteEvent::Status(_pkt, opt_latency) = event {
                        if let Some(latency_ms) = opt_latency {
                            println!("Average Latency: {}", latency_ms);
                        }
                    }
                }
            }
            complete => {
                // An empty channel returns None and falls into complete. Do nothing to keep
                // polling the future.
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stdin_input_has_no_leading_forward_slash() {
        let chat = parse_stdin("some text".to_owned());
        assert_eq!(chat, UserInput::Chat("some text".to_owned()));
    }

    #[test]
    fn parse_stdin_input_no_arguments() {
        let cmd = parse_stdin("/helpusobi".to_owned());
        assert_eq!(
            cmd,
            UserInput::Command {
                cmd:  "helpusobi".to_owned(),
                args: vec![],
            }
        );
    }

    #[test]
    fn parse_stdin_input_multiple_arguments() {
        let cmd = parse_stdin("/helpusobi 1".to_owned());
        assert_eq!(
            cmd,
            UserInput::Command {
                cmd:  "helpusobi".to_owned(),
                args: vec!["1".to_owned()],
            }
        );

        let cmd = parse_stdin("/helpusobi 1 you".to_owned());
        assert_eq!(
            cmd,
            UserInput::Command {
                cmd:  "helpusobi".to_owned(),
                args: vec!["1".to_owned(), "you".to_owned()],
            }
        );

        let cmd = parse_stdin("/helpusobi 1 you are our only hope".to_owned());
        assert_eq!(
            cmd,
            UserInput::Command {
                cmd:  "helpusobi".to_owned(),
                args: vec![
                    "1".to_owned(),
                    "you".to_owned(),
                    "are".to_owned(),
                    "our".to_owned(),
                    "only".to_owned(),
                    "hope".to_owned()
                ],
            }
        );
    }

    /* XXX testXXX
    #[test]
    fn build_command_request_action_unknown_command() {
        let command = UserInput::Command {
            cmd: "helpusobi".to_owned(),
            args: vec!["1".to_owned()],
        };

        match command {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn build_command_request_action_help_returns_no_action() {
        let command = UserInput::Command {
            cmd: "help".to_owned(),
            args: vec![],
        };

        match command {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn build_command_request_action_disconnect() {
        let command = UserInput::Command {
            cmd: "disconnect".to_owned(),
            args: vec![],
        };

        match command {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::Disconnect);
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn build_command_request_action_disconnect_with_args_returns_no_action() {
        let command = UserInput::Command {
            cmd: "disconnect".to_owned(),
            args: vec!["1".to_owned()],
        };

        match command {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn build_command_request_action_list_in_lobby() {
        let command = UserInput::Command {
            cmd: "list".to_owned(),
            args: vec![],
        };

        match command {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::ListRooms);
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn build_command_request_action_list_in_game() {
        let command = UserInput::Command {
            cmd: "list".to_owned(),
            args: vec![],
        };

        client_state.room = Some("some room".to_owned());
        match command {
            UserInput::Command { cmd, args } => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::ListPlayers);
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn build_command_request_action_leave_cases() {
        let command = UserInput::Command {
            cmd: "leave".to_owned(),
            args: vec![],
        };

        // Not in a room
        match command.clone() {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            }
            UserInput::Chat(_) => unreachable!(),
        }

        // Happy to leave
        client_state.room = Some("some room".to_owned());
        match command {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::LeaveRoom);
            }
            UserInput::Chat(_) => unreachable!(),
        }

        // Even though we're in a room, you cannot specify anything else
        let command = UserInput::Command {
            cmd: "leave".to_owned(),
            args: vec!["some room".to_owned()],
        };
        match command {
            UserInput::Command { cmd, args } => {
                let action = build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn build_command_request_action_join_cases() {
        let command = UserInput::Command {
            cmd: "join".to_owned(),
            args: vec![],
        };

        let mut client_state = create_client_net_state();
        // no room specified
        match command.clone() {
            UserInput::Command { cmd, args } => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            }
            UserInput::Chat(_) => unreachable!(),
        }

        // Already in game
        client_state.room = Some("some room".to_owned());
        match command {
            UserInput::Command { cmd, args } => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::None);
            }
            UserInput::Chat(_) => unreachable!(),
        }

        // Happily join one
        client_state.room = None;
        let command = UserInput::Command {
            cmd: "join".to_owned(),
            args: vec!["some room".to_owned()],
        };
        match command {
            UserInput::Command { cmd, args } => {
                let action = client_state.build_command_request_action(cmd, args);
                assert_eq!(action, RequestAction::JoinRoom("some room".to_owned()));
            }
            UserInput::Chat(_) => unreachable!(),
        }
    }
    */

    /* XXX testXXX
    #[test]
    fn handle_user_input_event_increment_sequence_number() {
        // There is a lot that _could_ be tested here but most of it is handled in the above test cases.
        let mut client_state = create_client_net_state();
        let (udp_tx, _) = mpsc::unbounded();
        let (exit_tx, _) = mpsc::unbounded();
        let user_input = UserInput::Chat("memes".to_owned());

        client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, user_input);
        assert_eq!(client_state.sequence, 1);

        let user_input = UserInput::Chat("and another one".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, user_input);
        assert_eq!(client_state.sequence, 2);
    }
    */

    /* XXX testXXX
    #[test]
    fn handle_incoming_event_basic_tx_rx_queueing() {
        let mut client_state = create_client_net_state();
        let (udp_tx, _) = mpsc::unbounded();
        let (exit_tx, _) = mpsc::unbounded();
        let connect_cmd = UserInput::Command {
            cmd: "connect".to_owned(),
            args: vec!["name".to_owned()],
        };
        let new_room_cmd = UserInput::Command {
            cmd: "new".to_owned(),
            args: vec!["room_name".to_owned()],
        };
        let join_room_cmd = UserInput::Command {
            cmd: "join".to_owned(),
            args: vec!["room_name".to_owned()],
        };
        let leave_room_cmd = UserInput::Command {
            cmd: "leave".to_owned(),
            args: vec![],
        };

        client_state.sequence = 0;
        client_state.response_sequence = 1;
        client_state.handle_user_input_event(&udp_tx, &exit_tx, connect_cmd); // Seq 0
        client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
        // dequeue connect since we don't actually want to process it later
        client_state.network.tx_packets.clear();
        client_state.handle_user_input_event(&udp_tx, &exit_tx, new_room_cmd); // Seq 1
        client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd); // Seq 2
        client_state.room = Some("room_name".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, leave_room_cmd); // Seq 3
        assert_eq!(client_state.sequence, 3);
        assert_eq!(client_state.response_sequence, 1);
        assert_eq!(client_state.network.tx_packets.len(), 3);
        assert_eq!(client_state.network.rx_packets.len(), 0);

        let room_response = Packet::Response {
            sequence: 1,
            request_ack: Some(1),
            code: ResponseCode::OK,
        };
        let join_response = Packet::Response {
            sequence: 2,
            request_ack: Some(2),
            code: ResponseCode::OK,
        };
        let leave_response = Packet::Response {
            sequence: 3,
            request_ack: Some(3),
            code: ResponseCode::OK,
        };

        client_state.handle_incoming_event(&udp_tx, Some(leave_response)); // 3 arrives
        assert_eq!(client_state.network.tx_packets.len(), 2);
        assert_eq!(client_state.network.rx_packets.len(), 1);
        client_state.handle_incoming_event(&udp_tx, Some(join_response)); // 2 arrives
        assert_eq!(client_state.network.tx_packets.len(), 1);
        assert_eq!(client_state.network.rx_packets.len(), 2);
        client_state.handle_incoming_event(&udp_tx, Some(room_response)); // 1 arrives
        assert_eq!(client_state.network.tx_packets.len(), 0);
        // RX should be cleared out because upon processing packet sequence '1', RX queue will be contiguous
        assert_eq!(client_state.network.rx_packets.len(), 0);
    }
    */

    /* XXX testXXX
        #[test]
        fn handle_incoming_event_basic_tx_rx_queueing_cannot_process_all_responses() {
            let mut client_state = create_client_net_state();
            let (udp_tx, _) = mpsc::unbounded();
            let (exit_tx, _) = mpsc::unbounded();
            let connect_cmd = UserInput::Command {
                cmd: "connect".to_owned(),
                args: vec!["name".to_owned()],
            };
            let new_room_cmd = UserInput::Command {
                cmd: "new".to_owned(),
                args: vec!["room_name".to_owned()],
            };
            let join_room_cmd = UserInput::Command {
                cmd: "join".to_owned(),
                args: vec!["room_name".to_owned()],
            };
            let leave_room_cmd = UserInput::Command {
                cmd: "leave".to_owned(),
                args: vec![],
            };

            client_state.sequence = 0;
            client_state.response_sequence = 1;
            client_state.handle_user_input_event(&udp_tx, &exit_tx, connect_cmd); // Seq 0
            client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
            // dequeue connect since we don't actually want to process it later
            client_state.network.tx_packets.clear();
            client_state.handle_user_input_event(&udp_tx, &exit_tx, new_room_cmd); // Seq 1
            client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd.clone()); // Seq 2
            client_state.room = Some("room_name".to_owned());
            client_state.handle_user_input_event(&udp_tx, &exit_tx, leave_room_cmd); // Seq 3
            client_state.room = None; // Temporarily set to None so we can process the next join
            client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd); // Seq 4
            client_state.room = Some("room_name".to_owned());
            assert_eq!(client_state.sequence, 4);
            assert_eq!(client_state.response_sequence, 1);
            assert_eq!(client_state.network.tx_packets.len(), 4);
            assert_eq!(client_state.network.rx_packets.len(), 0);

            let room_response = Packet::Response {
                sequence: 1,
                request_ack: Some(1),
                code: ResponseCode::OK,
            };
            let join_response = Packet::Response {
                sequence: 2,
                request_ack: Some(2),
                code: ResponseCode::OK,
            };
            let _leave_response = Packet::Response {
                sequence: 3,
                request_ack: Some(3),
                code: ResponseCode::OK,
            };
            let join2_response = Packet::Response {
                sequence: 4,
                request_ack: Some(4),
                code: ResponseCode::OK,
            };

            // The intent is that 3 never arrives
            client_state.handle_incoming_event(&udp_tx, Some(join2_response)); // 4 arrives
            assert_eq!(client_state.network.tx_packets.len(), 3);
            assert_eq!(client_state.network.rx_packets.len(), 1);
            client_state.handle_incoming_event(&udp_tx, Some(join_response)); // 2 arrives
            assert_eq!(client_state.network.tx_packets.len(), 2);
            assert_eq!(client_state.network.rx_packets.len(), 2);
            client_state.handle_incoming_event(&udp_tx, Some(room_response)); // 1 arrives
            assert_eq!(client_state.network.tx_packets.len(), 1);
            assert_eq!(client_state.network.rx_packets.len(), 1);
        }
    */

    /* XXX testXXX
    #[test]
    fn handle_incoming_event_basic_tx_rx_queueing_arrives_at_server_out_of_order() {
        let mut client_state = create_client_net_state();
        let (udp_tx, _) = mpsc::unbounded();
        let (exit_tx, _) = mpsc::unbounded();
        let connect_cmd = UserInput::Command {
            cmd: "connect".to_owned(),
            args: vec!["name".to_owned()],
        };
        let new_room_cmd = UserInput::Command {
            cmd: "new".to_owned(),
            args: vec!["room_name".to_owned()],
        };
        let join_room_cmd = UserInput::Command {
            cmd: "join".to_owned(),
            args: vec!["room_name".to_owned()],
        };
        let leave_room_cmd = UserInput::Command {
            cmd: "leave".to_owned(),
            args: vec![],
        };

        client_state.sequence = 0;
        client_state.response_sequence = 1;
        client_state.handle_user_input_event(&udp_tx, &exit_tx, connect_cmd); // Seq 0
        client_state.cookie = Some("ThisDoesNotReallyMatterAsLongAsItExists".to_owned());
        // dequeue connect since we don't actually want to process it later
        client_state.network.tx_packets.clear();
        client_state.handle_user_input_event(&udp_tx, &exit_tx, new_room_cmd); // Seq 1
        client_state.handle_user_input_event(&udp_tx, &exit_tx, join_room_cmd); // Seq 2
        client_state.room = Some("room_name".to_owned());
        client_state.handle_user_input_event(&udp_tx, &exit_tx, leave_room_cmd); // Seq 3
        assert_eq!(client_state.sequence, 3);
        assert_eq!(client_state.response_sequence, 1);
        assert_eq!(client_state.network.tx_packets.len(), 3);
        assert_eq!(client_state.network.rx_packets.len(), 0);

        // An out-of-order arrival at the server means the response packet's sequence number will not be 1:1 mapping
        // as in the first basic tested above. End result should be the same in both cases.
        let room_response = Packet::Response {
            sequence: 2,
            request_ack: Some(1),
            code: ResponseCode::OK,
        };
        let join_response = Packet::Response {
            sequence: 3,
            request_ack: Some(2),
            code: ResponseCode::OK,
        };
        let leave_response = Packet::Response {
            sequence: 1,
            request_ack: Some(3),
            code: ResponseCode::OK,
        };

        client_state.handle_incoming_event(&udp_tx, Some(leave_response)); // client 3 arrives, can process
        assert_eq!(client_state.network.tx_packets.len(), 2);
        assert_eq!(client_state.network.rx_packets.len(), 0);
        client_state.handle_incoming_event(&udp_tx, Some(join_response)); // client 2 arrives, cannot process
        assert_eq!(client_state.network.tx_packets.len(), 1);
        assert_eq!(client_state.network.rx_packets.len(), 1);
        client_state.handle_incoming_event(&udp_tx, Some(room_response)); // client 1 arrives, can process all
        assert_eq!(client_state.network.tx_packets.len(), 0);
        assert_eq!(client_state.network.rx_packets.len(), 0);
    }
    */
}
