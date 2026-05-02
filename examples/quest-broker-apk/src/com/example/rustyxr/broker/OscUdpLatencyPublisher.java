package com.example.rustyxr.broker;

import android.util.Log;

import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.InetAddress;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicLong;

final class OscUdpLatencyPublisher implements LatencyPublisher {
    private static final int MAX_OSC_DATAGRAM_BYTES = 8192;

    private final String host;
    private final int port;
    private final String address;
    private final DatagramSocket socket;
    private final InetAddress targetAddress;
    private final AtomicLong sentPackets = new AtomicLong();
    private final AtomicLong lastPacketBytes = new AtomicLong();
    private final AtomicLong lastSendUnixNs = new AtomicLong();
    private volatile String lastError = "";

    private OscUdpLatencyPublisher(BrokerRuntimeConfig config) throws Exception {
        if (config.oscHost.length() == 0) {
            throw new IllegalArgumentException("OSC enabled but no " + BrokerRuntimeConfig.EXTRA_OSC_HOST + " was supplied");
        }

        if (config.oscPort <= 0 || config.oscPort > 65535) {
            throw new IllegalArgumentException("OSC port out of range: " + config.oscPort);
        }

        if (!config.oscAddress.startsWith("/")) {
            throw new IllegalArgumentException("OSC address must start with '/': " + config.oscAddress);
        }

        host = config.oscHost;
        port = config.oscPort;
        address = config.oscAddress;
        targetAddress = InetAddress.getByName(host);
        socket = new DatagramSocket();
        Log.i(BrokerService.TAG, "OSC UDP publisher ready: " + host + ":" + port + " " + address);
    }

    static OscUdpLatencyPublisher createOrNull(BrokerRuntimeConfig config) {
        if (config == null || !config.oscEnabled) {
            return null;
        }

        try {
            return new OscUdpLatencyPublisher(config);
        } catch (Exception ex) {
            Log.w(BrokerService.TAG, "OSC UDP publisher unavailable: " + ex.getMessage(), ex);
            return null;
        }
    }

    @Override
    public String mode() {
        return "osc-udp";
    }

    @Override
    public boolean isLslAvailable() {
        return false;
    }

    @Override
    public boolean isOscAvailable() {
        return socket != null && !socket.isClosed();
    }

    @Override
    public String blocker() {
        return lastError;
    }

    @Override
    public JSONObject oscStatus() throws Exception {
        JSONObject status = new JSONObject();
        status.put("enabled", isOscAvailable());
        status.put("transport", "udp");
        status.put("host", host);
        status.put("port", port);
        status.put("address", address);
        status.put("sentPackets", sentPackets.get());
        status.put("lastPacketBytes", lastPacketBytes.get());
        status.put("lastSendUnixNs", lastSendUnixNs.get());
        if (lastError.length() > 0) {
            status.put("lastError", lastError);
        }
        return status;
    }

    @Override
    public void publish(JSONObject payload) {
        if (!isOscAvailable() || payload == null) {
            return;
        }

        try {
            byte[] packetBytes = encodeMessage(address, payload.toString());
            if (packetBytes.length > MAX_OSC_DATAGRAM_BYTES) {
                lastError = "OSC packet exceeds " + MAX_OSC_DATAGRAM_BYTES + " bytes: " + packetBytes.length;
                Log.w(BrokerService.TAG, lastError);
                return;
            }

            DatagramPacket packet = new DatagramPacket(packetBytes, packetBytes.length, targetAddress, port);
            socket.send(packet);
            lastError = "";
            sentPackets.incrementAndGet();
            lastPacketBytes.set(packetBytes.length);
            lastSendUnixNs.set(System.currentTimeMillis() * 1_000_000L);
        } catch (Exception ex) {
            lastError = ex.getClass().getSimpleName() + ": " + ex.getMessage();
            Log.w(BrokerService.TAG, "OSC UDP publish failed: " + lastError);
        }
    }

    @Override
    public void close() {
        if (socket != null) {
            socket.close();
        }
    }

    private static byte[] encodeMessage(String address, String json) throws Exception {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        writePaddedString(output, address);
        writePaddedString(output, ",s");
        writePaddedString(output, json);
        return output.toByteArray();
    }

    private static void writePaddedString(ByteArrayOutputStream output, String value) throws Exception {
        byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
        output.write(bytes);
        output.write(0);
        int unpadded = bytes.length + 1;
        int padding = (4 - (unpadded % 4)) % 4;
        for (int i = 0; i < padding; i++) {
            output.write(0);
        }
    }
}
