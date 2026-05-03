package com.example.rustyxr.broker;

import android.util.Log;

import org.json.JSONObject;

import java.io.Closeable;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;

final class OscIngressServer implements Closeable {
    private static final int MAX_PACKET_BYTES = 8192;

    private final int port;
    private final String acceptedAddress;
    private final BrokerState state;
    private final LocalBrokerServer brokerServer;
    private volatile boolean running;
    private DatagramSocket socket;
    private Thread receiveThread;
    private volatile String lastPeer = "";
    private volatile String lastError = "";
    private volatile float lastValue;
    private volatile long lastReceiveUnixNs;
    private volatile long lastSequenceId;

    private OscIngressServer(BrokerRuntimeConfig config, BrokerState state, LocalBrokerServer brokerServer) {
        this.port = config.oscIngressPort;
        this.acceptedAddress = config.oscIngressAddress;
        this.state = state;
        this.brokerServer = brokerServer;
    }

    static OscIngressServer createOrNull(BrokerRuntimeConfig config, BrokerState state, LocalBrokerServer brokerServer) {
        if (config == null || !config.oscIngressEnabled) {
            return null;
        }

        if (config.oscIngressPort <= 0 || config.oscIngressPort > 65535) {
            Log.w(BrokerService.TAG, "OSC ingress port out of range: " + config.oscIngressPort);
            return null;
        }

        if (config.oscIngressAddress == null || !config.oscIngressAddress.startsWith("/")) {
            Log.w(BrokerService.TAG, "OSC ingress address must start with '/': " + config.oscIngressAddress);
            return null;
        }

        return new OscIngressServer(config, state, brokerServer);
    }

    boolean isRunning() {
        return running;
    }

    int port() {
        return port;
    }

    String acceptedAddress() {
        return acceptedAddress;
    }

    String streamId() {
        return "osc:" + acceptedAddress;
    }

    void start() throws Exception {
        if (running) {
            return;
        }

        socket = new DatagramSocket(null);
        socket.setReuseAddress(true);
        socket.bind(new InetSocketAddress(InetAddress.getByName("0.0.0.0"), port));
        running = true;
        receiveThread = new Thread(new Runnable() {
            @Override
            public void run() {
                receiveLoop();
            }
        }, "RustyXrOscIngress");
        receiveThread.start();
        Log.i(BrokerService.TAG, "OSC ingress listening on 0.0.0.0:" + port + " address=" + acceptedAddress);
    }

    @Override
    public void close() {
        running = false;
        if (socket != null) {
            socket.close();
            socket = null;
        }
    }

    JSONObject toStatusJson() throws Exception {
        JSONObject status = new JSONObject();
        status.put("enabled", running);
        status.put("transport", "udp");
        status.put("port", port);
        status.put("address", acceptedAddress);
        status.put("packets", state.oscIngressPackets.get());
        status.put("rejectedPackets", state.oscIngressRejectedPackets.get());
        status.put("broadcasts", state.oscIngressBroadcasts.get());
        status.put("lastPeer", lastPeer);
        status.put("lastValue", lastValue);
        status.put("lastReceiveUnixNs", lastReceiveUnixNs);
        status.put("lastSequenceId", lastSequenceId);
        if (lastError.length() > 0) {
            status.put("lastError", lastError);
        }
        return status;
    }

    private void receiveLoop() {
        byte[] buffer = new byte[MAX_PACKET_BYTES];
        while (running) {
            DatagramPacket datagram = new DatagramPacket(buffer, buffer.length);
            try {
                DatagramSocket activeSocket = socket;
                if (activeSocket == null) {
                    return;
                }

                activeSocket.receive(datagram);
                handleDatagram(datagram);
            } catch (Exception ex) {
                if (!running) {
                    return;
                }

                lastError = ex.getClass().getSimpleName() + ": " + ex.getMessage();
                state.oscIngressRejectedPackets.incrementAndGet();
                Log.w(BrokerService.TAG, "OSC ingress receive failed: " + lastError);
            }
        }
    }

    private void handleDatagram(DatagramPacket datagram) throws Exception {
        OscMessage message = decodeOscMessage(datagram.getData(), datagram.getOffset(), datagram.getLength());
        String peer = datagram.getAddress().getHostAddress() + ":" + datagram.getPort();
        if (!acceptedAddress.equals(message.address)) {
            lastError = "unexpected OSC address: " + message.address;
            state.oscIngressRejectedPackets.incrementAndGet();
            Log.w(BrokerService.TAG, "OSC ingress rejected address=" + message.address + " peer=" + peer);
            return;
        }

        if (message.argumentCount <= 0) {
            lastError = "OSC drive packet has no argument";
            state.oscIngressRejectedPackets.incrementAndGet();
            return;
        }

        float value = clamp01(message.firstValue);
        long sequence = state.oscIngressPackets.incrementAndGet();
        long receiveUnixNs = System.currentTimeMillis() * 1_000_000L;
        lastPeer = peer;
        lastValue = value;
        lastReceiveUnixNs = receiveUnixNs;
        lastSequenceId = sequence;
        lastError = "";

        JSONObject event = new JSONObject();
        event.put("type", "osc_drive");
        event.put("schema", "rusty.xr.osc.drive.v1");
        event.put("address", message.address);
        event.put("value", value);
        event.put("sequence_id", sequence);
        event.put("peer", peer);
        event.put("broker_receive_time_unix_ns", receiveUnixNs);
        event.put("argument_type", String.valueOf(message.firstTypeTag));
        int legacyBroadcasts = brokerServer.broadcastText(event.toString());

        JSONObject payload = new JSONObject();
        payload.put("address", message.address);
        payload.put("value01", value);
        payload.put("peer", peer);
        payload.put("argument_type", String.valueOf(message.firstTypeTag));
        int streamBroadcasts = brokerServer.broadcastStreamEvent(
            streamId(),
            sequence,
            receiveUnixNs,
            payload);
        int broadcasts = legacyBroadcasts + streamBroadcasts;
        state.oscIngressBroadcasts.addAndGet(broadcasts);

        if (sequence == 1 || sequence % 30 == 0) {
            Log.i(BrokerService.TAG, "OSC ingress drive seq=" + sequence + " value=" + value + " broadcasts=" + broadcasts);
        }
    }

    private static OscMessage decodeOscMessage(byte[] data, int offset, int length) throws Exception {
        int limit = offset + length;
        ReadStringResult address = readPaddedString(data, offset, limit);
        if (!address.value.startsWith("/")) {
            throw new IllegalArgumentException("invalid OSC address: " + address.value);
        }

        ReadStringResult typeTags = readPaddedString(data, address.nextOffset, limit);
        if (!typeTags.value.startsWith(",")) {
            throw new IllegalArgumentException("invalid OSC type tags: " + typeTags.value);
        }

        int cursor = typeTags.nextOffset;
        int argumentCount = 0;
        float firstValue = 0f;
        char firstTypeTag = '\0';
        String tags = typeTags.value.substring(1);
        for (int i = 0; i < tags.length(); i++) {
            char tag = tags.charAt(i);
            float parsedValue;
            if (tag == 'f') {
                require(data, cursor, limit, 4);
                int bits = ByteBuffer.wrap(data, cursor, 4).order(ByteOrder.BIG_ENDIAN).getInt();
                parsedValue = Float.intBitsToFloat(bits);
                cursor += 4;
            } else if (tag == 'i') {
                require(data, cursor, limit, 4);
                parsedValue = (float) ByteBuffer.wrap(data, cursor, 4).order(ByteOrder.BIG_ENDIAN).getInt();
                cursor += 4;
            } else if (tag == 's') {
                ReadStringResult stringArg = readPaddedString(data, cursor, limit);
                parsedValue = Float.parseFloat(stringArg.value.trim());
                cursor = stringArg.nextOffset;
            } else if (tag == 'T' || tag == 'F') {
                parsedValue = tag == 'T' ? 1f : 0f;
            } else {
                throw new IllegalArgumentException("unsupported OSC type tag: " + tag);
            }

            if (argumentCount == 0) {
                firstValue = parsedValue;
                firstTypeTag = tag;
            }
            argumentCount++;
        }

        if (cursor != limit) {
            throw new IllegalArgumentException("OSC packet has trailing bytes: " + (limit - cursor));
        }

        return new OscMessage(address.value, firstValue, firstTypeTag, argumentCount);
    }

    private static ReadStringResult readPaddedString(byte[] data, int offset, int limit) {
        if (offset >= limit) {
            throw new IllegalArgumentException("unexpected end of OSC packet");
        }

        int cursor = offset;
        while (cursor < limit && data[cursor] != 0) {
            cursor++;
        }

        if (cursor >= limit) {
            throw new IllegalArgumentException("OSC string missing null terminator");
        }

        String value = new String(data, offset, cursor - offset, StandardCharsets.UTF_8);
        int nextOffset = offset + paddedLength(cursor - offset + 1);
        require(data, offset, limit, nextOffset - offset);
        return new ReadStringResult(value, nextOffset);
    }

    private static int paddedLength(int value) {
        return value + ((4 - (value % 4)) % 4);
    }

    private static void require(byte[] data, int offset, int limit, int length) {
        if (data == null || offset < 0 || length < 0 || offset + length > limit) {
            throw new IllegalArgumentException("unexpected end of OSC packet");
        }
    }

    private static float clamp01(float value) {
        if (Float.isNaN(value) || Float.isInfinite(value)) {
            return 0f;
        }
        if (value < 0f) {
            return 0f;
        }
        if (value > 1f) {
            return 1f;
        }
        return value;
    }

    private static final class ReadStringResult {
        final String value;
        final int nextOffset;

        ReadStringResult(String value, int nextOffset) {
            this.value = value;
            this.nextOffset = nextOffset;
        }
    }

    private static final class OscMessage {
        final String address;
        final float firstValue;
        final char firstTypeTag;
        final int argumentCount;

        OscMessage(String address, float firstValue, char firstTypeTag, int argumentCount) {
            this.address = address;
            this.firstValue = firstValue;
            this.firstTypeTag = firstTypeTag;
            this.argumentCount = argumentCount;
        }
    }
}
