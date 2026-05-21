use rusty_xr_broker_model::{
    BrokerPayloadKind, BrokerStreamDirection, BrokerTransportEndpoint, BrokerZeroMqBindMode,
    BrokerZeroMqBridgeManifest, BrokerZeroMqPattern, BROKER_LATENCY_SAMPLE_SCHEMA,
    STREAM_LATENCY_SAMPLE,
};
use rusty_xr_zmq::{
    spawn_pub_sub_receiver, ZmqMessageInbox, ZmqPubSubReceiverConfig, ZmqReceivedMessage,
    ZmqReceiverStatus,
};
use std::{
    error::Error,
    io,
    net::TcpListener,
    time::{Duration, Instant},
};
use zeromq::{PubSocket, Socket, SocketSend, ZmqMessage};

const TOPIC_PREFIX: &str = "rustyxr.loopback";
const SAMPLE_COUNT: usize = 5;

fn main() -> Result<(), Box<dyn Error>> {
    let port = reserve_loopback_port()?;
    let endpoint = format!("tcp://127.0.0.1:{port}");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(run_loopback(port, endpoint))
}

async fn run_loopback(port: u16, endpoint: String) -> Result<(), Box<dyn Error>> {
    let mut publisher = PubSocket::new();
    publisher.bind(&endpoint).await?;

    let manifest = BrokerZeroMqBridgeManifest::new(
        "rusty-xr-zmq-loopback",
        BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", port),
        BrokerZeroMqPattern::PubSub,
        BrokerStreamDirection::ProducerToConsumer,
        BrokerPayloadKind::Json,
        BROKER_LATENCY_SAMPLE_SCHEMA,
    )
    .with_bind_mode(BrokerZeroMqBindMode::Connect)
    .with_stream_id(STREAM_LATENCY_SAMPLE)
    .with_topic_prefix(TOPIC_PREFIX)
    .with_max_message_bytes(4096)
    .with_high_water_mark(16)
    .with_consent_data_category("synthetic")
    .with_note("pure Rust ZeroMQ loopback example");

    let config = ZmqPubSubReceiverConfig::try_from_manifest(&manifest)?.with_receive_timeout_ms(10);
    let receiver = spawn_pub_sub_receiver(config)?;
    wait_for_connected(receiver.inbox(), Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;

    for sequence_number in 0..SAMPLE_COUNT {
        let payload = format!(
            "{TOPIC_PREFIX} {{\"sequence_number\":{sequence_number},\"source\":\"synthetic\"}}"
        );
        publisher.send(ZmqMessage::from(payload)).await?;
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let messages = drain_until(receiver.inbox(), SAMPLE_COUNT, Duration::from_secs(2)).await?;
    let snapshot = receiver.inbox().snapshot();
    let close_errors = publisher.close().await;
    receiver.shutdown();

    if !close_errors.is_empty() {
        return Err(io_error(
            io::ErrorKind::Other,
            format!("failed to close ZeroMQ publisher cleanly: {close_errors:?}"),
        ));
    }

    println!("ZeroMQ loopback endpoint: {endpoint}");
    println!(
        "received={} drained={} dropped={} decode_errors={}",
        snapshot.received_count,
        snapshot.drained_count,
        snapshot.dropped_count,
        snapshot.decode_error_count
    );
    for message in messages {
        println!(
            "{} {}",
            message.sequence_number,
            message.utf8_text.as_deref().unwrap_or("<non-utf8>")
        );
    }

    Ok(())
}

async fn wait_for_connected(
    inbox: &ZmqMessageInbox,
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    let start = Instant::now();
    loop {
        let snapshot = inbox.snapshot();
        match snapshot.status {
            ZmqReceiverStatus::Connected => return Ok(()),
            ZmqReceiverStatus::Fault => {
                return Err(io_error(
                    io::ErrorKind::Other,
                    format!("ZeroMQ receiver fault: {:?}", snapshot.fault),
                ));
            }
            ZmqReceiverStatus::Starting | ZmqReceiverStatus::Stopped => {}
        }
        if start.elapsed() >= timeout {
            return Err(io_error(
                io::ErrorKind::TimedOut,
                "timed out waiting for ZeroMQ receiver connection",
            ));
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn drain_until(
    inbox: &ZmqMessageInbox,
    expected_count: usize,
    timeout: Duration,
) -> Result<Vec<ZmqReceivedMessage>, Box<dyn Error>> {
    let start = Instant::now();
    let mut messages = Vec::with_capacity(expected_count);
    loop {
        messages.extend(inbox.drain_messages());
        if messages.len() >= expected_count {
            return Ok(messages);
        }

        let snapshot = inbox.snapshot();
        if snapshot.status == ZmqReceiverStatus::Fault {
            return Err(io_error(
                io::ErrorKind::Other,
                format!("ZeroMQ receiver fault: {:?}", snapshot.fault),
            ));
        }
        if start.elapsed() >= timeout {
            return Err(io_error(
                io::ErrorKind::TimedOut,
                format!(
                    "timed out waiting for ZeroMQ loopback messages: got {}, expected {expected_count}",
                    messages.len()
                ),
            ));
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn reserve_loopback_port() -> io::Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn io_error(kind: io::ErrorKind, message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(kind, message.into()))
}
