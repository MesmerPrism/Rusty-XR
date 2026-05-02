use std::{env, process, time::Duration};

use rusty_xr_osc::{
    send_packet_to, OscArgument, OscEndpointConfig, OscMessage, OscPacket, OscStreamRole,
    OscUdpSocket,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "send" => send(&args[1..]),
        "listen" => listen(&args[1..]),
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn send(args: &[String]) -> Result<(), String> {
    let to = option_value(args, "--to").unwrap_or("127.0.0.1:9000");
    let address = option_value(args, "--address").unwrap_or("/rusty-xr/probe");
    let mut message = OscMessage::new(address).map_err(|error| error.to_string())?;
    for value in option_values(args, "--arg") {
        message = message.with_argument(parse_arg(value)?);
    }
    if message.arguments.is_empty() {
        message = message.with_argument(OscArgument::String("hello".to_string()));
    }

    let packet = OscPacket::Message(message);
    let bytes = send_packet_to(&packet, to).map_err(|error| error.to_string())?;
    println!("sent {bytes} OSC bytes to {to}");
    Ok(())
}

fn listen(args: &[String]) -> Result<(), String> {
    let bind = option_value(args, "--bind").unwrap_or("0.0.0.0:9000");
    let count = option_value(args, "--count")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let socket = OscUdpSocket::bind(
        OscEndpointConfig::new(bind)
            .with_role(OscStreamRole::Probe)
            .with_max_packet_bytes(8192),
    )
    .map_err(|error| error.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| error.to_string())?;
    println!(
        "listening for {count} OSC packet(s) on {}",
        socket.local_addr().map_err(|error| error.to_string())?
    );
    for _ in 0..count {
        let received = socket.recv_packet().map_err(|error| error.to_string())?;
        println!("{received:#?}");
    }
    Ok(())
}

fn parse_arg(raw: &str) -> Result<OscArgument, String> {
    let (kind, value) = raw
        .split_once(':')
        .ok_or_else(|| format!("argument '{raw}' must use kind:value"))?;
    match kind {
        "int" | "i" => value
            .parse::<i32>()
            .map(OscArgument::Int)
            .map_err(|error| format!("invalid int argument '{raw}': {error}")),
        "float" | "f" => value
            .parse::<f32>()
            .map(OscArgument::Float)
            .map_err(|error| format!("invalid float argument '{raw}': {error}")),
        "string" | "s" => Ok(OscArgument::String(value.to_string())),
        "bool" | "b" => value
            .parse::<bool>()
            .map(OscArgument::Bool)
            .map_err(|error| format!("invalid bool argument '{raw}': {error}")),
        "nil" => Ok(OscArgument::Nil),
        "impulse" => Ok(OscArgument::Impulse),
        _ => Err(format!("unsupported OSC probe argument kind '{kind}'")),
    }
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn option_values<'a>(args: &'a [String], name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    args.windows(2)
        .filter(move |pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn print_help() {
    println!(
        "Usage:\n  osc_udp_probe send --to 127.0.0.1:9000 --address /rusty-xr/probe --arg string:hello\n  osc_udp_probe listen --bind 0.0.0.0:9000 --count 1"
    );
}
