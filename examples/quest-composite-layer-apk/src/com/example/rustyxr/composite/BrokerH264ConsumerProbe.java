package com.example.rustyxr.composite;

import android.graphics.SurfaceTexture;
import android.graphics.ImageFormat;
import android.hardware.HardwareBuffer;
import android.media.Image;
import android.media.ImageReader;
import android.media.MediaCodec;
import android.media.MediaFormat;
import android.opengl.EGL14;
import android.opengl.EGLConfig;
import android.opengl.EGLContext;
import android.opengl.EGLDisplay;
import android.opengl.EGLSurface;
import android.opengl.GLES11Ext;
import android.opengl.GLES20;
import android.os.Build;
import android.os.SystemClock;
import android.util.Base64;
import android.util.Log;
import android.view.Surface;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Random;

final class BrokerH264ConsumerProbe implements Runnable {
    private static final String TAG = "RustyXrComposite";
    private static final String STREAM_MAGIC = "RXYRVID1";
    private static final int CODEC_H264 = 1;
    private static final int DEFAULT_COMMAND_TIMEOUT_MS = 10000;
    private static final int DEFAULT_STREAM_TIMEOUT_MS = 20000;
    private static final int DEFAULT_DECODE_TIMEOUT_MS = 5000;
    private static final String DECODE_OUTPUT_BYTE_BUFFER = "byte-buffer";
    private static final String DECODE_OUTPUT_SURFACE_TEXTURE = "surface-texture";
    private static final String DECODE_OUTPUT_HARDWARE_BUFFER = "hardware-buffer";
    private static final int MAX_PACKET_BYTES = 1024 * 1024;
    private static final int MAX_STREAM_PACKETS = 720;
    private static final int DEQUEUE_TIMEOUT_US = 10000;
    private static final int SURFACE_FRAME_WAIT_MS = 250;
    private static final int HARDWARE_BUFFER_WAIT_MS = 250;
    private static final int HARDWARE_BUFFER_READER_MAX_IMAGES = 4;

    interface Sink {
        void onBrokerH264ConsumerProbe(JSONObject report);
    }

    static final class Config {
        final String brokerHost;
        final int brokerPort;
        final int streamPort;
        final int rightStreamPort;
        final String cameraId;
        final String leftCameraId;
        final String rightCameraId;
        final int preferredWidth;
        final int preferredHeight;
        final int captureMs;
        final int maxPackets;
        final int bitrateBps;
        final int commandTimeoutMs;
        final int streamTimeoutMs;
        final int decodeTimeoutMs;
        final String decodeOutputMode;
        final boolean stereo;
        final boolean liveStream;

        Config(
            String brokerHost,
            int brokerPort,
            int streamPort,
            int rightStreamPort,
            String cameraId,
            String leftCameraId,
            String rightCameraId,
            int preferredWidth,
            int preferredHeight,
            int captureMs,
            int maxPackets,
            int bitrateBps,
            int commandTimeoutMs,
            int streamTimeoutMs,
            int decodeTimeoutMs,
            String decodeOutputMode,
            boolean stereo,
            boolean liveStream) {
            this.brokerHost = brokerHost;
            this.brokerPort = brokerPort;
            this.streamPort = streamPort;
            this.rightStreamPort = rightStreamPort;
            this.cameraId = cameraId != null ? cameraId : "";
            this.leftCameraId =
                leftCameraId != null && leftCameraId.length() > 0 ? leftCameraId : this.cameraId;
            this.rightCameraId = rightCameraId != null ? rightCameraId : "";
            this.preferredWidth = preferredWidth;
            this.preferredHeight = preferredHeight;
            this.captureMs = captureMs;
            this.maxPackets = maxPackets;
            this.bitrateBps = bitrateBps;
            this.commandTimeoutMs = commandTimeoutMs;
            this.streamTimeoutMs = streamTimeoutMs;
            this.decodeTimeoutMs = decodeTimeoutMs;
            this.decodeOutputMode = normalizeDecodeOutputMode(decodeOutputMode);
            this.stereo = stereo;
            this.liveStream = liveStream;
        }
    }

    private static final class StartCommandResult {
        final JSONObject ack;
        final JSONObject streamProjectionMetadata;
        final JSONObject projectionProfile;

        StartCommandResult(JSONObject ack, JSONObject streamProjectionMetadata, JSONObject projectionProfile) {
            this.ack = ack;
            this.streamProjectionMetadata = streamProjectionMetadata;
            this.projectionProfile = projectionProfile;
        }
    }

    private final Config config;
    private final Sink sink;
    private volatile boolean running = true;
    private volatile Socket brokerSocket;
    private volatile Socket streamSocket;
    private volatile Socket rightStreamSocket;

    private static native boolean nativeBrokerH264DecodedHardwareBufferFrame(
        int width,
        int height,
        long timestampNs,
        String metadataJson,
        HardwareBuffer buffer,
        int hardwareBufferFormat,
        long hardwareBufferUsage,
        int hardwareBufferLayers,
        long hardwareBufferId);

    private static native boolean nativeBrokerH264DecodedStereoHardwareBufferFrame(
        int leftWidth,
        int leftHeight,
        long leftTimestampNs,
        String leftMetadataJson,
        HardwareBuffer leftBuffer,
        int leftHardwareBufferFormat,
        long leftHardwareBufferUsage,
        int leftHardwareBufferLayers,
        long leftHardwareBufferId,
        int rightWidth,
        int rightHeight,
        long rightTimestampNs,
        String rightMetadataJson,
        HardwareBuffer rightBuffer,
        int rightHardwareBufferFormat,
        long rightHardwareBufferUsage,
        int rightHardwareBufferLayers,
        long rightHardwareBufferId,
        long pairDeltaNs,
        long pairIndex);

    private BrokerH264ConsumerProbe(Config config, Sink sink) {
        this.config = config;
        this.sink = sink;
    }

    static BrokerH264ConsumerProbe start(Config config, Sink sink) {
        BrokerH264ConsumerProbe probe = new BrokerH264ConsumerProbe(config, sink);
        Thread thread = new Thread(probe, "RustyXrBrokerH264Consumer");
        thread.start();
        return probe;
    }

    void stop() {
        running = false;
        closeQuietly(brokerSocket);
        closeQuietly(streamSocket);
        closeQuietly(rightStreamSocket);
    }

    @Override
    public void run() {
        long startedElapsedNs = SystemClock.elapsedRealtimeNanos();
        JSONObject report = new JSONObject();
        try {
            report.put("schema", "rusty.xr.composite.broker_h264_consumer_probe.v1");
            report.put("source", "composite_app_broker_h264_consumer");
            report.put("broker_host", config.brokerHost);
            report.put("broker_port", config.brokerPort);
            report.put("stream_port", config.streamPort);
            report.put("right_stream_port", config.rightStreamPort);
            report.put("camera_id", config.cameraId);
            report.put("left_camera_id", config.leftCameraId);
            report.put("right_camera_id", config.rightCameraId);
            report.put("preferred_width", config.preferredWidth);
            report.put("preferred_height", config.preferredHeight);
            report.put("capture_ms", config.captureMs);
            report.put("max_packets", config.maxPackets);
            report.put("bitrate_bps", config.bitrateBps);
            report.put("stereo_requested", config.stereo);
            report.put("live_stream_requested", config.liveStream);
            report.put("decode_output_mode", config.decodeOutputMode);
            report.put(
                "decode_surface_target_requested",
                DECODE_OUTPUT_SURFACE_TEXTURE.equals(config.decodeOutputMode));
            report.put(
                "decode_hardware_buffer_target_requested",
                DECODE_OUTPUT_HARDWARE_BUFFER.equals(config.decodeOutputMode));
            report.put("succeeded", false);

            if (config.stereo) {
                runStereoProbe(report, startedElapsedNs);
            } else {
                runMonoProbe(report, startedElapsedNs);
            }
        } catch (Exception ex) {
            try {
                report.put("last_error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
                report.put("succeeded", false);
                report.put("total_duration_ns", Math.max(0L, SystemClock.elapsedRealtimeNanos() - startedElapsedNs));
            } catch (Exception ignored) {
            }
        }

        Log.i(TAG, "Rusty XR broker H.264 consumer probe: " + report.toString());
        if (sink != null) {
            sink.onBrokerH264ConsumerProbe(report);
        }
    }

    private void runMonoProbe(JSONObject report, long startedElapsedNs) throws Exception {
        StartCommandResult startCommand = sendStartCommand("mono", config.cameraId, config.streamPort);
        putStartCommandReport(report, "", startCommand);
        if (!startCommand.ack.optBoolean("accepted", false)) {
            throw new IllegalStateException("Broker rejected app-camera H.264 stream command.");
        }

        StreamResult stream = receiveStream("mono", config.streamPort);
        DecodeResult decode = decodePackets(
            stream,
            config.decodeTimeoutMs,
            config.decodeOutputMode,
            startCommand.streamProjectionMetadata,
            config.cameraId,
            "",
            false);
        long completedElapsedNs = SystemClock.elapsedRealtimeNanos();
        putStreamDecodeReport(report, "", stream, decode);
        report.put("total_duration_ns", Math.max(0L, completedElapsedNs - startedElapsedNs));
        if (decode.lastError.length() > 0) {
            report.put("last_error", decode.lastError);
        }
        report.put("succeeded", decode.decodedFrameCount > 0);
    }

    private void runStereoProbe(JSONObject report, long startedElapsedNs) throws Exception {
        report.put("stereo_pairing_mode", "decoded-frame-index");
        if (!DECODE_OUTPUT_HARDWARE_BUFFER.equals(config.decodeOutputMode)) {
            throw new IllegalStateException("Broker H.264 stereo probe requires hardware-buffer decode output.");
        }

        StartCommandResult leftStart = sendStartCommand("left", config.leftCameraId, config.streamPort);
        putStartCommandReport(report, "left", leftStart);
        if (!leftStart.ack.optBoolean("accepted", false)) {
            throw new IllegalStateException("Broker rejected left app-camera H.264 stream command.");
        }

        StartCommandResult rightStart = sendStartCommand("right", config.rightCameraId, config.rightStreamPort);
        putStartCommandReport(report, "right", rightStart);
        if (!rightStart.ack.optBoolean("accepted", false)) {
            throw new IllegalStateException("Broker rejected right app-camera H.264 stream command.");
        }

        StreamReceiveTask leftReceive = new StreamReceiveTask("left", config.streamPort);
        StreamReceiveTask rightReceive = new StreamReceiveTask("right", config.rightStreamPort);
        leftReceive.start();
        rightReceive.start();
        StreamResult leftStream = leftReceive.awaitResult();
        StreamResult rightStream = rightReceive.awaitResult();
        DecodeResult leftDecode = null;
        DecodeResult rightDecode = null;
        try {
            leftDecode = decodePackets(
                leftStream,
                config.decodeTimeoutMs,
                config.decodeOutputMode,
                leftStart.streamProjectionMetadata,
                config.leftCameraId,
                "left",
                true);
            rightDecode = decodePackets(
                rightStream,
                config.decodeTimeoutMs,
                config.decodeOutputMode,
                rightStart.streamProjectionMetadata,
                config.rightCameraId,
                "right",
                true);
            StereoPairResult pair = deliverStereoPairs(
                leftDecode.collectedHardwareBufferFrames,
                rightDecode.collectedHardwareBufferFrames);
            long completedElapsedNs = SystemClock.elapsedRealtimeNanos();
            putStreamDecodeReport(report, "left", leftStream, leftDecode);
            putStreamDecodeReport(report, "right", rightStream, rightDecode);
            report.put("stereo_pair_count", pair.pairCount);
            report.put("stereo_pair_native_accepted_count", pair.nativeAcceptedCount);
            report.put("stereo_pair_native_rejected_count", pair.nativeRejectedCount);
            report.put("stereo_pair_delta_total_ns", pair.deltaTotalNs);
            report.put("stereo_pair_delta_avg_ns", pair.pairCount > 0 ? pair.deltaTotalNs / pair.pairCount : 0L);
            report.put("stereo_pair_delta_max_ns", pair.deltaMaxNs);
            report.put("stereo_left_right_resolution_match", pair.resolutionMismatchCount == 0);
            report.put("stereo_resolution_mismatch_count", pair.resolutionMismatchCount);
            report.put("total_duration_ns", Math.max(0L, completedElapsedNs - startedElapsedNs));
            report.put("succeeded", pair.nativeAcceptedCount > 0);
            if (leftDecode.lastError.length() > 0) {
                report.put("left_last_error", leftDecode.lastError);
            }
            if (rightDecode.lastError.length() > 0) {
                report.put("right_last_error", rightDecode.lastError);
            }
            if (pair.nativeAcceptedCount == 0 && pair.pairCount == 0) {
                report.put("last_error", "Decoded hardware-buffer frames were not available from both streams.");
            } else if (pair.nativeAcceptedCount == 0) {
                report.put("last_error", "Native stereo hardware-buffer bridge rejected every decoded pair.");
            }
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 stereo summary: succeeded=%s liveStream=%s leftCameraId=%s rightCameraId=%s left=%dx%d right=%dx%d leftPackets=%d rightPackets=%d leftPayloadBytes=%d rightPayloadBytes=%d leftEncodedPacketHz=%.3f rightEncodedPacketHz=%.3f leftSourcePacketHz=%.3f rightSourcePacketHz=%.3f leftWirePacketHz=%.3f rightWirePacketHz=%.3f leftDecodedFrames=%d rightDecodedFrames=%d leftDecodedFrameHz=%.3f rightDecodedFrameHz=%.3f pairCount=%d nativeAccepted=%d nativeRejected=%d pairDeltaAvgNs=%d pairDeltaMaxNs=%d metadataReadyLeft=%s metadataReadyRight=%s poseSourceLeft=%s poseSourceRight=%s totalDurationNs=%d",
                pair.nativeAcceptedCount > 0,
                config.liveStream,
                leftStart.streamProjectionMetadata != null
                    ? leftStart.streamProjectionMetadata.optString("cameraId", "")
                    : config.leftCameraId,
                rightStart.streamProjectionMetadata != null
                    ? rightStart.streamProjectionMetadata.optString("cameraId", "")
                    : config.rightCameraId,
                leftStream.width,
                leftStream.height,
                rightStream.width,
                rightStream.height,
                leftStream.packets.size(),
                rightStream.packets.size(),
                leftStream.payloadBytes,
                rightStream.payloadBytes,
                rateHz(leftStream.packets.size(), leftDecode.captureWindowMs),
                rateHz(rightStream.packets.size(), rightDecode.captureWindowMs),
                rateHzFromNs(leftStream.packets.size(), Math.max(0L, leftStream.lastSourceElapsedNs - leftStream.firstSourceElapsedNs)),
                rateHzFromNs(rightStream.packets.size(), Math.max(0L, rightStream.lastSourceElapsedNs - rightStream.firstSourceElapsedNs)),
                rateHzFromNs(leftStream.packets.size(), Math.max(0L, leftStream.lastPacketReceiveElapsedNs - leftStream.firstPacketReceiveElapsedNs)),
                rateHzFromNs(rightStream.packets.size(), Math.max(0L, rightStream.lastPacketReceiveElapsedNs - rightStream.firstPacketReceiveElapsedNs)),
                leftDecode.decodedFrameCount,
                rightDecode.decodedFrameCount,
                rateHz(leftDecode.decodedFrameCount, leftDecode.captureWindowMs),
                rateHz(rightDecode.decodedFrameCount, rightDecode.captureWindowMs),
                pair.pairCount,
                pair.nativeAcceptedCount,
                pair.nativeRejectedCount,
                pair.pairCount > 0 ? pair.deltaTotalNs / pair.pairCount : 0L,
                pair.deltaMaxNs,
                leftStart.streamProjectionMetadata != null &&
                    leftStart.streamProjectionMetadata.optBoolean("projectionMetadataReady", false),
                rightStart.streamProjectionMetadata != null &&
                    rightStart.streamProjectionMetadata.optBoolean("projectionMetadataReady", false),
                leftStart.streamProjectionMetadata != null
                    ? leftStart.streamProjectionMetadata.optString("poseSource", "")
                    : "",
                rightStart.streamProjectionMetadata != null
                    ? rightStart.streamProjectionMetadata.optString("poseSource", "")
                    : "",
                Math.max(0L, completedElapsedNs - startedElapsedNs)));
        } finally {
            if (leftDecode != null) {
                closeFrames(leftDecode.collectedHardwareBufferFrames);
            }
            if (rightDecode != null) {
                closeFrames(rightDecode.collectedHardwareBufferFrames);
            }
        }
    }

    private StartCommandResult sendStartCommand(String label, String cameraId, int streamPort) throws Exception {
        Socket socket = new Socket();
        brokerSocket = socket;
        socket.connect(new InetSocketAddress(config.brokerHost, config.brokerPort), config.commandTimeoutMs);
        socket.setSoTimeout(config.commandTimeoutMs);
        InputStream input = socket.getInputStream();
        OutputStream output = socket.getOutputStream();
        String key = Base64.encodeToString(
            ("rusty-xr-" + System.nanoTime()).getBytes(StandardCharsets.US_ASCII),
            Base64.NO_WRAP);
        String request =
            "GET /rustyxr/v1/events HTTP/1.1\r\n" +
            "Host: " + config.brokerHost + ":" + config.brokerPort + "\r\n" +
            "Upgrade: websocket\r\n" +
            "Connection: Upgrade\r\n" +
            "Sec-WebSocket-Version: 13\r\n" +
            "Sec-WebSocket-Key: " + key + "\r\n" +
            "\r\n";
        output.write(request.getBytes(StandardCharsets.US_ASCII));
        output.flush();
        String status = readHttpLine(input);
        if (status == null || !status.contains("101")) {
            throw new IllegalStateException("Broker WebSocket upgrade failed: " + status);
        }
        while (true) {
            String line = readHttpLine(input);
            if (line == null || line.length() == 0) {
                break;
            }
        }

        readWebSocketTextFrame(input);
        sendMaskedTextFrame(output, startCommandJson(label, cameraId, streamPort).toString());
        long deadline = SystemClock.elapsedRealtimeNanos() + config.commandTimeoutMs * 1_000_000L;
        while (running && SystemClock.elapsedRealtimeNanos() < deadline) {
            String text = readWebSocketTextFrame(input);
            if (text == null || text.length() == 0) {
                continue;
            }
            JSONObject message = new JSONObject(text);
            if ("command_ack".equals(message.optString("type", ""))) {
                closeQuietly(socket);
                brokerSocket = null;
                return new StartCommandResult(
                    message,
                    extractStreamProjectionMetadata(message),
                    extractProjectionProfile(message));
            }
        }

        closeQuietly(socket);
        brokerSocket = null;
        throw new IllegalStateException("Timed out waiting for broker command ack.");
    }

    private JSONObject startCommandJson(String label, String cameraId, int streamPort) throws Exception {
        JSONObject params = new JSONObject();
        params.put("device_port", streamPort);
        params.put("host_port", streamPort);
        params.put("preferred_width", config.preferredWidth);
        params.put("preferred_height", config.preferredHeight);
        params.put("capture_ms", config.captureMs);
        params.put("max_packets", config.maxPackets);
        params.put("bitrate_bps", config.bitrateBps);
        params.put("live_stream", config.liveStream);
        if (cameraId != null && cameraId.length() > 0) {
            params.put("camera_id", cameraId);
        }

        JSONObject command = new JSONObject();
        command.put("type", "command");
        command.put("schema", "rusty.xr.broker.command.v1");
        command.put("request_id", "composite-h264-consumer-" + label + "-" + System.currentTimeMillis());
        command.put("command", "camera_provider.start_app_camera_h264_stream");
        command.put("client_id", "rusty-xr-composite-h264-consumer-" + label);
        command.put("app_label", "Rusty XR Composite Layer APK");
        command.put("app_version", "source-example");
        command.put("params", params);
        return command;
    }

    private static JSONObject extractStreamProjectionMetadata(JSONObject ack) {
        JSONObject result = ack != null ? ack.optJSONObject("result") : null;
        JSONObject streamStart = result != null ? result.optJSONObject("stream_start") : null;
        return streamStart != null ? streamStart.optJSONObject("projection_metadata") : null;
    }

    private static JSONObject extractProjectionProfile(JSONObject ack) {
        JSONObject result = ack != null ? ack.optJSONObject("result") : null;
        return result != null ? result.optJSONObject("projection_profile") : null;
    }

    private static void putStartCommandReport(
        JSONObject report,
        String prefix,
        StartCommandResult startCommand) throws Exception {
        JSONObject ack = startCommand.ack;
        report.put(reportKey(prefix, "broker_command_accepted"), ack.optBoolean("accepted", false));
        report.put(reportKey(prefix, "broker_command_message"), ack.optString("message", ""));
        report.put(
            reportKey(prefix, "broker_projection_metadata_attached"),
            startCommand.streamProjectionMetadata != null);
        if (startCommand.streamProjectionMetadata != null) {
            report.put(
                reportKey(prefix, "broker_projection_metadata_camera_id"),
                startCommand.streamProjectionMetadata.optString("cameraId", ""));
            report.put(
                reportKey(prefix, "broker_projection_metadata_has_intrinsics"),
                !startCommand.streamProjectionMetadata.optBoolean("missingIntrinsics", true));
            report.put(
                reportKey(prefix, "broker_projection_metadata_has_pose"),
                !startCommand.streamProjectionMetadata.optBoolean("missingPose", true));
            report.put(
                reportKey(prefix, "broker_projection_metadata_pose_source"),
                startCommand.streamProjectionMetadata.optString("poseSource", ""));
            report.put(
                reportKey(prefix, "broker_projection_metadata_ready"),
                startCommand.streamProjectionMetadata.optBoolean("projectionMetadataReady", false));
        }
        if (startCommand.projectionProfile != null) {
            report.put(
                reportKey(prefix, "broker_projection_profile_id"),
                startCommand.projectionProfile.optString("profile_id", ""));
            report.put(
                reportKey(prefix, "broker_projection_profile_mapping"),
                startCommand.projectionProfile.optString("mapping", ""));
        }
    }

    private static void putStreamDecodeReport(
        JSONObject report,
        String prefix,
        StreamResult stream,
        DecodeResult decode) throws Exception {
        report.put(reportKey(prefix, "stream_schema_version"), stream.schemaVersion);
        report.put(reportKey(prefix, "stream_codec_id"), stream.codecId);
        report.put(reportKey(prefix, "stream_width"), stream.width);
        report.put(reportKey(prefix, "stream_height"), stream.height);
        report.put(reportKey(prefix, "stream_declared_packet_count"), stream.declaredPacketCount);
        report.put(reportKey(prefix, "stream_packet_count"), stream.packets.size());
        report.put(reportKey(prefix, "stream_payload_bytes"), stream.payloadBytes);
        report.put(reportKey(prefix, "encoded_packet_rate_hz"), rateHz(stream.packets.size(), decode.captureWindowMs));
        report.put(reportKey(prefix, "stream_payload_bitrate_bps"), bitrateBps(stream.payloadBytes, decode.captureWindowMs));
        report.put(reportKey(prefix, "stream_receive_duration_ns"), Math.max(0L, stream.receiveEndElapsedNs - stream.receiveStartElapsedNs));
        report.put(
            reportKey(prefix, "stream_wire_packet_rate_hz"),
            rateHzFromNs(stream.packets.size(), Math.max(0L, stream.lastPacketReceiveElapsedNs - stream.firstPacketReceiveElapsedNs)));
        report.put(
            reportKey(prefix, "stream_source_packet_rate_hz"),
            rateHzFromNs(stream.packets.size(), Math.max(0L, stream.lastSourceElapsedNs - stream.firstSourceElapsedNs)));
        report.put(reportKey(prefix, "stream_first_source_elapsed_ns"), stream.firstSourceElapsedNs);
        report.put(reportKey(prefix, "stream_last_source_elapsed_ns"), stream.lastSourceElapsedNs);
        report.put(reportKey(prefix, "decode_succeeded"), decode.decodedFrameCount > 0);
        report.put(reportKey(prefix, "decoder_name"), decode.decoderName);
        report.put(reportKey(prefix, "csd_sps_found"), decode.spsBytes > 0);
        report.put(reportKey(prefix, "csd_pps_found"), decode.ppsBytes > 0);
        report.put(reportKey(prefix, "input_buffer_count"), decode.inputBufferCount);
        report.put(reportKey(prefix, "input_bytes"), decode.inputBytes);
        report.put(reportKey(prefix, "input_eos_queued"), decode.inputEosQueued);
        report.put(reportKey(prefix, "output_format_changes"), decode.outputFormatChanges);
        report.put(reportKey(prefix, "output_buffer_count"), decode.outputBufferCount);
        report.put(reportKey(prefix, "decoded_frame_count"), decode.decodedFrameCount);
        report.put(reportKey(prefix, "decoded_frame_rate_hz"), rateHz(decode.decodedFrameCount, decode.captureWindowMs));
        report.put(reportKey(prefix, "decoder_output_bytes"), decode.outputBytes);
        report.put(reportKey(prefix, "surface_target_created"), decode.surfaceTargetCreated);
        report.put(reportKey(prefix, "egl_context_created"), decode.eglContextCreated);
        report.put(reportKey(prefix, "external_texture_created"), decode.externalTextureCreated);
        report.put(reportKey(prefix, "external_texture_id"), decode.externalTextureId);
        report.put(reportKey(prefix, "surface_release_count"), decode.surfaceReleaseCount);
        report.put(reportKey(prefix, "surface_frame_available_count"), decode.surfaceFrameAvailableCount);
        report.put(reportKey(prefix, "surface_texture_update_count"), decode.surfaceTextureUpdateCount);
        report.put(reportKey(prefix, "first_surface_frame_available_ns"), decode.firstSurfaceFrameAvailableNs);
        report.put(reportKey(prefix, "first_surface_texture_update_ns"), decode.firstSurfaceTextureUpdateNs);
        report.put(reportKey(prefix, "last_surface_texture_timestamp_ns"), decode.lastSurfaceTextureTimestampNs);
        if (decode.surfaceTextureTransform != null) {
            report.put(reportKey(prefix, "surface_texture_transform"), floatArrayJson(decode.surfaceTextureTransform));
        }
        report.put(reportKey(prefix, "hardware_buffer_target_created"), decode.hardwareBufferTargetCreated);
        report.put(reportKey(prefix, "hardware_buffer_reader_width"), decode.hardwareBufferReaderWidth);
        report.put(reportKey(prefix, "hardware_buffer_reader_height"), decode.hardwareBufferReaderHeight);
        report.put(reportKey(prefix, "hardware_buffer_image_count"), decode.hardwareBufferImageCount);
        report.put(reportKey(prefix, "hardware_buffer_delivered_count"), decode.hardwareBufferDeliveredCount);
        report.put(reportKey(prefix, "hardware_buffer_native_accepted_count"), decode.hardwareBufferNativeAcceptedCount);
        report.put(reportKey(prefix, "hardware_buffer_native_rejected_count"), decode.hardwareBufferNativeRejectedCount);
        report.put(reportKey(prefix, "hardware_buffer_missing_count"), decode.hardwareBufferMissingCount);
        report.put(reportKey(prefix, "hardware_buffer_format"), decode.lastHardwareBufferFormat);
        report.put(reportKey(prefix, "hardware_buffer_usage"), decode.lastHardwareBufferUsage);
        report.put(reportKey(prefix, "hardware_buffer_layers"), decode.lastHardwareBufferLayers);
        report.put(reportKey(prefix, "hardware_buffer_id"), decode.lastHardwareBufferId);
        report.put(reportKey(prefix, "output_eos_seen"), decode.outputEosSeen);
        report.put(reportKey(prefix, "output_format_mime"), decode.outputMime);
        report.put(reportKey(prefix, "output_format_width"), decode.outputWidth);
        report.put(reportKey(prefix, "output_format_height"), decode.outputHeight);
        report.put(reportKey(prefix, "decode_duration_ns"), Math.max(0L, decode.decodeEndElapsedNs - decode.decodeStartElapsedNs));
    }

    private static StereoPairResult deliverStereoPairs(
        List<DecodedHardwareBufferFrame> leftFrames,
        List<DecodedHardwareBufferFrame> rightFrames) {
        StereoPairResult result = new StereoPairResult();
        int pairCount = Math.min(leftFrames.size(), rightFrames.size());
        for (int i = 0; i < pairCount; i++) {
            DecodedHardwareBufferFrame left = leftFrames.get(i);
            DecodedHardwareBufferFrame right = rightFrames.get(i);
            long deltaNs = Math.abs(left.timestampNs - right.timestampNs);
            result.pairCount++;
            result.deltaTotalNs += deltaNs;
            result.deltaMaxNs = Math.max(result.deltaMaxNs, deltaNs);
            if (left.width != right.width || left.height != right.height) {
                result.resolutionMismatchCount++;
            }
            boolean accepted = false;
            try {
                accepted = nativeBrokerH264DecodedStereoHardwareBufferFrame(
                    left.width,
                    left.height,
                    left.timestampNs,
                    left.metadataJson,
                    left.buffer,
                    left.format,
                    left.usage,
                    left.layers,
                    left.bufferId,
                    right.width,
                    right.height,
                    right.timestampNs,
                    right.metadataJson,
                    right.buffer,
                    right.format,
                    right.usage,
                    right.layers,
                    right.bufferId,
                    deltaNs,
                    i);
            } catch (RuntimeException error) {
                Log.w(TAG, "Could not deliver broker H.264 decoded stereo hardware-buffer pair", error);
            }
            if (accepted) {
                result.nativeAcceptedCount++;
            } else {
                result.nativeRejectedCount++;
            }
        }
        return result;
    }

    private static void closeFrames(List<DecodedHardwareBufferFrame> frames) {
        for (int i = 0; i < frames.size(); i++) {
            frames.get(i).close();
        }
        frames.clear();
    }

    private static String reportKey(String prefix, String key) {
        return prefix != null && prefix.length() > 0 ? prefix + "_" + key : key;
    }

    private static double rateHz(int count, int windowMs) {
        return windowMs > 0 ? (count * 1000.0) / windowMs : 0.0;
    }

    private static double rateHzFromNs(int count, long windowNs) {
        return windowNs > 0L ? (count * 1_000_000_000.0) / windowNs : 0.0;
    }

    private static long bitrateBps(long bytes, int windowMs) {
        return windowMs > 0 ? (bytes * 8000L) / windowMs : 0L;
    }

    private int observedStreamWindowMs(StreamResult stream) {
        long windowNs = Math.max(0L, stream.lastSourceElapsedNs - stream.firstSourceElapsedNs);
        if (windowNs <= 0L) {
            windowNs = Math.max(0L, stream.lastPacketReceiveElapsedNs - stream.firstPacketReceiveElapsedNs);
        }
        if (windowNs <= 0L) {
            return Math.max(1, config.captureMs);
        }
        return Math.max(1, (int) ((windowNs + 999_999L) / 1_000_000L));
    }

    private final class StreamReceiveTask implements Runnable {
        private final String label;
        private final int streamPort;
        private final Thread thread;
        private StreamResult result;
        private Exception error;

        StreamReceiveTask(String label, int streamPort) {
            this.label = label;
            this.streamPort = streamPort;
            this.thread = new Thread(this, "RustyXrBrokerH264Receive-" + label);
        }

        void start() {
            thread.start();
        }

        @Override
        public void run() {
            try {
                result = receiveStream(label, streamPort);
            } catch (Exception ex) {
                error = ex;
            }
        }

        StreamResult awaitResult() throws Exception {
            thread.join(config.streamTimeoutMs + 1000L);
            if (thread.isAlive()) {
                throw new IllegalStateException("Timed out waiting for " + label + " broker H.264 stream receive task.");
            }
            if (error != null) {
                throw error;
            }
            if (result == null) {
                throw new IllegalStateException("Broker H.264 " + label + " stream receive produced no result.");
            }
            return result;
        }
    }

    private StreamResult receiveStream(String label, int streamPort) throws Exception {
        Socket socket = connectWithRetry(config.brokerHost, streamPort, config.streamTimeoutMs, label);
        if ("right".equals(label)) {
            rightStreamSocket = socket;
        } else {
            streamSocket = socket;
        }
        socket.setSoTimeout(config.streamTimeoutMs);
        DataInputStream input = new DataInputStream(socket.getInputStream());
        byte[] magicBytes = new byte[8];
        input.readFully(magicBytes);
        String magic = new String(magicBytes, StandardCharsets.US_ASCII);
        if (!STREAM_MAGIC.equals(magic)) {
            throw new IllegalStateException("Unexpected stream magic: " + magic);
        }

        int schemaVersion = input.readInt();
        int codecId = input.readInt();
        int width = input.readInt();
        int height = input.readInt();
        int packetCount = input.readInt();
        input.readInt();
        if (schemaVersion < 1 || schemaVersion > 2) {
            throw new IllegalStateException("Unsupported broker stream schema version: " + schemaVersion);
        }
        if (codecId != CODEC_H264) {
            throw new IllegalStateException("Broker stream codec is not H.264: " + codecId);
        }
        if (packetCount < 0 || packetCount > MAX_STREAM_PACKETS) {
            throw new IllegalStateException("Broker stream packet count is out of range: " + packetCount);
        }

        List<Packet> packets = new ArrayList<Packet>();
        long payloadBytes = 0L;
        long receiveStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long receiveEndElapsedNs = receiveStartElapsedNs;
        long firstPacketReceiveElapsedNs = 0L;
        long lastPacketReceiveElapsedNs = 0L;
        long firstSourceElapsedNs = 0L;
        long lastSourceElapsedNs = 0L;
        for (int i = 0; i < packetCount && running; i++) {
            long ptsUs = input.readLong();
            int flags = input.readInt();
            int size = input.readInt();
            if (size < 0 || size > MAX_PACKET_BYTES) {
                throw new IllegalStateException("Broker stream packet size is out of range: " + size);
            }
            long sourceElapsedNs = 0L;
            long sourceUnixNs = 0L;
            if (schemaVersion >= 2) {
                sourceElapsedNs = input.readLong();
                sourceUnixNs = input.readLong();
            }
            byte[] payload = new byte[size];
            input.readFully(payload);
            long packetReceiveElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (firstPacketReceiveElapsedNs == 0L) {
                firstPacketReceiveElapsedNs = packetReceiveElapsedNs;
            }
            lastPacketReceiveElapsedNs = packetReceiveElapsedNs;
            if (sourceElapsedNs > 0L) {
                if (firstSourceElapsedNs == 0L) {
                    firstSourceElapsedNs = sourceElapsedNs;
                }
                lastSourceElapsedNs = sourceElapsedNs;
            }
            packets.add(new Packet(ptsUs, flags, sourceElapsedNs, sourceUnixNs, packetReceiveElapsedNs, payload));
            payloadBytes += size;
        }
        receiveEndElapsedNs = SystemClock.elapsedRealtimeNanos();

        closeQuietly(socket);
        if ("right".equals(label)) {
            rightStreamSocket = null;
        } else {
            streamSocket = null;
        }
        return new StreamResult(
            schemaVersion,
            codecId,
            width,
            height,
            packetCount,
            packets,
            payloadBytes,
            receiveStartElapsedNs,
            receiveEndElapsedNs,
            firstPacketReceiveElapsedNs,
            lastPacketReceiveElapsedNs,
            firstSourceElapsedNs,
            lastSourceElapsedNs);
    }

    private DecodeResult decodePackets(
        StreamResult stream,
        int timeoutMs,
        String decodeOutputMode,
        JSONObject streamProjectionMetadata,
        String cameraId,
        String stereoEye,
        boolean collectHardwareBuffers) throws Exception {
        DecodeResult result = new DecodeResult();
        result.decodeOutputMode = decodeOutputMode;
        result.captureWindowMs = observedStreamWindowMs(stream);
        result.decodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        NalUnit sps = findNalUnit(stream.packets, 7);
        NalUnit pps = findNalUnit(stream.packets, 8);
        result.spsBytes = sps != null ? sps.bytes.length : 0;
        result.ppsBytes = pps != null ? pps.bytes.length : 0;
        boolean hasCompleteCsd = sps != null && pps != null;

        MediaFormat format = MediaFormat.createVideoFormat("video/avc", stream.width, stream.height);
        if (sps != null) {
            format.setByteBuffer("csd-0", ByteBuffer.wrap(sps.bytes));
        }
        if (pps != null) {
            format.setByteBuffer("csd-1", ByteBuffer.wrap(pps.bytes));
        }

        DecodeSurfaceTarget surfaceTarget = null;
        if (DECODE_OUTPUT_SURFACE_TEXTURE.equals(decodeOutputMode)) {
            surfaceTarget = DecodeSurfaceTarget.create(stream.width, stream.height);
            result.surfaceTargetCreated = true;
            result.eglContextCreated = surfaceTarget.eglContextCreated();
            result.externalTextureCreated = surfaceTarget.externalTextureCreated();
            result.externalTextureId = surfaceTarget.textureId();
        }
        DecodeHardwareBufferTarget hardwareBufferTarget = null;
        if (DECODE_OUTPUT_HARDWARE_BUFFER.equals(decodeOutputMode)) {
            hardwareBufferTarget = DecodeHardwareBufferTarget.create(stream.width, stream.height);
            result.hardwareBufferTargetCreated = true;
            result.hardwareBufferReaderWidth = stream.width;
            result.hardwareBufferReaderHeight = stream.height;
        }

        MediaCodec decoder = MediaCodec.createDecoderByType("video/avc");
        try {
            result.decoderName = decoder.getName();
            Surface decoderSurface = surfaceTarget != null
                ? surfaceTarget.surface()
                : (hardwareBufferTarget != null ? hardwareBufferTarget.surface() : null);
            decoder.configure(format, decoderSurface, null, 0);
            decoder.start();
            MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
            long deadline = SystemClock.elapsedRealtimeNanos() + timeoutMs * 1_000_000L;
            int nextInput = 0;
            while (running && !result.outputEosSeen && SystemClock.elapsedRealtimeNanos() < deadline) {
                if (!result.inputEosQueued) {
                    if (hasCompleteCsd) {
                        while (nextInput < stream.packets.size() &&
                            (stream.packets.get(nextInput).flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0) {
                            nextInput++;
                        }
                    }
                    int inputIndex = decoder.dequeueInputBuffer(DEQUEUE_TIMEOUT_US);
                    if (inputIndex >= 0) {
                        if (nextInput < stream.packets.size()) {
                            Packet packet = stream.packets.get(nextInput++);
                            queuePacket(decoder, inputIndex, packet);
                            result.inputBufferCount++;
                            result.inputBytes += packet.payload.length;
                        } else {
                            decoder.queueInputBuffer(
                                inputIndex,
                                0,
                                0,
                                lastPresentationTimeUs(stream.packets),
                                MediaCodec.BUFFER_FLAG_END_OF_STREAM);
                            result.inputEosQueued = true;
                        }
                    }
                }

                int outputIndex = decoder.dequeueOutputBuffer(info, DEQUEUE_TIMEOUT_US);
                if (outputIndex == MediaCodec.INFO_TRY_AGAIN_LATER) {
                    continue;
                }
                if (outputIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    result.outputFormatChanges++;
                    applyOutputFormat(result, decoder.getOutputFormat(), stream);
                    continue;
                }
                if (outputIndex < 0) {
                    continue;
                }

                boolean codecConfig = (info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0;
                boolean eos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
                if (!codecConfig) {
                    result.outputBufferCount++;
                    if (surfaceTarget != null && !eos) {
                        decoder.releaseOutputBuffer(outputIndex, true);
                        result.surfaceReleaseCount++;
                        if (surfaceTarget.awaitAndUpdateFrame(SURFACE_FRAME_WAIT_MS)) {
                            result.decodedFrameCount++;
                            result.surfaceFrameAvailableCount = surfaceTarget.frameAvailableCount();
                            result.surfaceTextureUpdateCount = surfaceTarget.textureUpdateCount();
                            result.firstSurfaceFrameAvailableNs = surfaceTarget.firstFrameAvailableNs();
                            result.firstSurfaceTextureUpdateNs = surfaceTarget.firstTextureUpdateNs();
                            result.lastSurfaceTextureTimestampNs = surfaceTarget.lastTextureTimestampNs();
                            result.surfaceTextureTransform = surfaceTarget.textureTransform();
                        }
                    } else if (hardwareBufferTarget != null && !eos) {
                        decoder.releaseOutputBuffer(outputIndex, true);
                        result.surfaceReleaseCount++;
                        DecodeHardwareBufferTarget.DeliverResult deliver;
                        if (collectHardwareBuffers) {
                            deliver = hardwareBufferTarget.awaitAndRetainFrame(
                                HARDWARE_BUFFER_WAIT_MS,
                                cameraId,
                                stereoEye,
                                info.presentationTimeUs,
                                sourceElapsedNsForPts(stream, info.presentationTimeUs),
                                streamProjectionMetadata,
                                result.collectedHardwareBufferFrames);
                        } else {
                            deliver = hardwareBufferTarget.awaitAndDeliverFrame(
                                HARDWARE_BUFFER_WAIT_MS,
                                cameraId,
                                info.presentationTimeUs,
                                sourceElapsedNsForPts(stream, info.presentationTimeUs),
                                streamProjectionMetadata);
                        }
                        result.hardwareBufferImageCount = hardwareBufferTarget.imageCount();
                        result.hardwareBufferDeliveredCount = hardwareBufferTarget.deliveredCount();
                        result.hardwareBufferNativeAcceptedCount = hardwareBufferTarget.nativeAcceptedCount();
                        result.hardwareBufferNativeRejectedCount = hardwareBufferTarget.nativeRejectedCount();
                        result.hardwareBufferMissingCount = hardwareBufferTarget.missingBufferCount();
                        if (deliver.delivered) {
                            result.decodedFrameCount++;
                            result.lastHardwareBufferFormat = deliver.format;
                            result.lastHardwareBufferUsage = deliver.usage;
                            result.lastHardwareBufferLayers = deliver.layers;
                            result.lastHardwareBufferId = deliver.bufferId;
                        }
                    } else {
                        if (info.size > 0 && surfaceTarget == null && hardwareBufferTarget == null) {
                            result.decodedFrameCount++;
                            result.outputBytes += info.size;
                        }
                        decoder.releaseOutputBuffer(outputIndex, false);
                    }
                } else {
                    decoder.releaseOutputBuffer(outputIndex, false);
                }
                if (eos) {
                    result.outputEosSeen = true;
                }
            }
            if (result.decodedFrameCount == 0) {
                result.lastError = result.outputEosSeen
                    ? "Decoder reached end-of-stream without output frames."
                    : "Timed out before a decoded output frame was produced.";
            }
            if (result.outputWidth == 0 || result.outputHeight == 0) {
                applyOutputFormat(result, decoder.getOutputFormat(), stream);
            }
        } finally {
            result.decodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            try {
                decoder.stop();
            } catch (Exception ignored) {
            }
            decoder.release();
            if (surfaceTarget != null) {
                result.surfaceFrameAvailableCount = surfaceTarget.frameAvailableCount();
                result.surfaceTextureUpdateCount = surfaceTarget.textureUpdateCount();
                result.firstSurfaceFrameAvailableNs = surfaceTarget.firstFrameAvailableNs();
                result.firstSurfaceTextureUpdateNs = surfaceTarget.firstTextureUpdateNs();
                result.lastSurfaceTextureTimestampNs = surfaceTarget.lastTextureTimestampNs();
                result.surfaceTextureTransform = surfaceTarget.textureTransform();
                surfaceTarget.release();
            }
            if (hardwareBufferTarget != null) {
                result.hardwareBufferImageCount = hardwareBufferTarget.imageCount();
                result.hardwareBufferDeliveredCount = hardwareBufferTarget.deliveredCount();
                result.hardwareBufferNativeAcceptedCount = hardwareBufferTarget.nativeAcceptedCount();
                result.hardwareBufferNativeRejectedCount = hardwareBufferTarget.nativeRejectedCount();
                result.hardwareBufferMissingCount = hardwareBufferTarget.missingBufferCount();
                hardwareBufferTarget.release();
            }
        }

        return result;
    }

    private static void queuePacket(MediaCodec decoder, int inputIndex, Packet packet) throws Exception {
        ByteBuffer inputBuffer = decoder.getInputBuffer(inputIndex);
        if (inputBuffer == null) {
            throw new IllegalStateException("Decoder input buffer is unavailable.");
        }
        if (packet.payload.length > inputBuffer.capacity()) {
            throw new IllegalStateException("Encoded packet exceeds decoder input capacity.");
        }
        inputBuffer.clear();
        inputBuffer.put(packet.payload);
        int flags = (packet.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0
            ? MediaCodec.BUFFER_FLAG_CODEC_CONFIG
            : 0;
        decoder.queueInputBuffer(inputIndex, 0, packet.payload.length, packet.ptsUs, flags);
    }

    private static String normalizeDecodeOutputMode(String value) {
        if (value == null || value.trim().length() == 0) {
            return DECODE_OUTPUT_SURFACE_TEXTURE;
        }
        String normalized = value.trim().toLowerCase(Locale.US).replace('_', '-');
        if ("surface".equals(normalized) ||
            "surface-texture".equals(normalized) ||
            "external-texture".equals(normalized) ||
            "external-oes".equals(normalized) ||
            "oes".equals(normalized)) {
            return DECODE_OUTPUT_SURFACE_TEXTURE;
        }
        if ("hardware-buffer".equals(normalized) ||
            "hardwarebuffer".equals(normalized) ||
            "ahardwarebuffer".equals(normalized) ||
            "image-reader".equals(normalized) ||
            "imagereader".equals(normalized) ||
            "vulkan-import".equals(normalized) ||
            "openxr-layer".equals(normalized)) {
            return DECODE_OUTPUT_HARDWARE_BUFFER;
        }
        if ("byte-buffer".equals(normalized) ||
            "bytebuffer".equals(normalized) ||
            "buffer".equals(normalized)) {
            return DECODE_OUTPUT_BYTE_BUFFER;
        }
        return DECODE_OUTPUT_SURFACE_TEXTURE;
    }

    private static JSONArray floatArrayJson(float[] values) throws Exception {
        JSONArray array = new JSONArray();
        for (int i = 0; i < values.length; i++) {
            array.put((double) values[i]);
        }
        return array;
    }

    private static final class DecodeSurfaceTarget implements SurfaceTexture.OnFrameAvailableListener {
        private final Object frameLock = new Object();
        private final EGLDisplay eglDisplay;
        private final EGLContext eglContext;
        private final EGLSurface eglSurface;
        private final int textureId;
        private final SurfaceTexture surfaceTexture;
        private final Surface surface;
        private final float[] textureTransform = new float[16];
        private int frameAvailableCount;
        private int consumedFrameCount;
        private int textureUpdateCount;
        private long firstFrameAvailableNs;
        private long firstTextureUpdateNs;
        private long lastTextureTimestampNs;

        private DecodeSurfaceTarget(
            EGLDisplay eglDisplay,
            EGLContext eglContext,
            EGLSurface eglSurface,
            int textureId,
            SurfaceTexture surfaceTexture,
            Surface surface) {
            this.eglDisplay = eglDisplay;
            this.eglContext = eglContext;
            this.eglSurface = eglSurface;
            this.textureId = textureId;
            this.surfaceTexture = surfaceTexture;
            this.surface = surface;
        }

        static DecodeSurfaceTarget create(int width, int height) throws Exception {
            EGLDisplay display = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY);
            if (display == EGL14.EGL_NO_DISPLAY) {
                throw new IllegalStateException("EGL display is unavailable.");
            }
            int[] version = new int[2];
            if (!EGL14.eglInitialize(display, version, 0, version, 1)) {
                throw new IllegalStateException("Could not initialize EGL display.");
            }

            int[] configAttributes = new int[] {
                EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT,
                EGL14.EGL_SURFACE_TYPE, EGL14.EGL_PBUFFER_BIT,
                EGL14.EGL_RED_SIZE, 8,
                EGL14.EGL_GREEN_SIZE, 8,
                EGL14.EGL_BLUE_SIZE, 8,
                EGL14.EGL_ALPHA_SIZE, 8,
                EGL14.EGL_NONE
            };
            EGLConfig[] configs = new EGLConfig[1];
            int[] configCount = new int[1];
            if (!EGL14.eglChooseConfig(display, configAttributes, 0, configs, 0, 1, configCount, 0) ||
                configCount[0] <= 0) {
                EGL14.eglTerminate(display);
                throw new IllegalStateException("Could not choose an EGL config for SurfaceTexture decode.");
            }

            int[] contextAttributes = new int[] {
                EGL14.EGL_CONTEXT_CLIENT_VERSION, 2,
                EGL14.EGL_NONE
            };
            EGLContext context = EGL14.eglCreateContext(
                display,
                configs[0],
                EGL14.EGL_NO_CONTEXT,
                contextAttributes,
                0);
            if (context == EGL14.EGL_NO_CONTEXT) {
                EGL14.eglTerminate(display);
                throw new IllegalStateException("Could not create an EGL context for SurfaceTexture decode.");
            }

            int[] surfaceAttributes = new int[] {
                EGL14.EGL_WIDTH, 1,
                EGL14.EGL_HEIGHT, 1,
                EGL14.EGL_NONE
            };
            EGLSurface pbuffer = EGL14.eglCreatePbufferSurface(display, configs[0], surfaceAttributes, 0);
            if (pbuffer == EGL14.EGL_NO_SURFACE) {
                EGL14.eglDestroyContext(display, context);
                EGL14.eglTerminate(display);
                throw new IllegalStateException("Could not create an EGL pbuffer for SurfaceTexture decode.");
            }
            if (!EGL14.eglMakeCurrent(display, pbuffer, pbuffer, context)) {
                EGL14.eglDestroySurface(display, pbuffer);
                EGL14.eglDestroyContext(display, context);
                EGL14.eglTerminate(display);
                throw new IllegalStateException("Could not make the SurfaceTexture EGL context current.");
            }

            int[] textureIds = new int[1];
            GLES20.glGenTextures(1, textureIds, 0);
            checkGl("glGenTextures");
            if (textureIds[0] <= 0) {
                throw new IllegalStateException("GL did not allocate an external texture name.");
            }
            GLES20.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, textureIds[0]);
            checkGl("glBindTexture external OES");
            GLES20.glTexParameteri(
                GLES11Ext.GL_TEXTURE_EXTERNAL_OES,
                GLES20.GL_TEXTURE_MIN_FILTER,
                GLES20.GL_LINEAR);
            GLES20.glTexParameteri(
                GLES11Ext.GL_TEXTURE_EXTERNAL_OES,
                GLES20.GL_TEXTURE_MAG_FILTER,
                GLES20.GL_LINEAR);
            GLES20.glTexParameteri(
                GLES11Ext.GL_TEXTURE_EXTERNAL_OES,
                GLES20.GL_TEXTURE_WRAP_S,
                GLES20.GL_CLAMP_TO_EDGE);
            GLES20.glTexParameteri(
                GLES11Ext.GL_TEXTURE_EXTERNAL_OES,
                GLES20.GL_TEXTURE_WRAP_T,
                GLES20.GL_CLAMP_TO_EDGE);
            checkGl("configure external OES texture");

            SurfaceTexture surfaceTexture = new SurfaceTexture(textureIds[0]);
            surfaceTexture.setDefaultBufferSize(Math.max(1, width), Math.max(1, height));
            Surface surface = new Surface(surfaceTexture);
            DecodeSurfaceTarget target = new DecodeSurfaceTarget(
                display,
                context,
                pbuffer,
                textureIds[0],
                surfaceTexture,
                surface);
            surfaceTexture.setOnFrameAvailableListener(target);
            return target;
        }

        Surface surface() {
            return surface;
        }

        boolean eglContextCreated() {
            return eglDisplay != EGL14.EGL_NO_DISPLAY && eglContext != EGL14.EGL_NO_CONTEXT;
        }

        boolean externalTextureCreated() {
            return textureId > 0;
        }

        int textureId() {
            return textureId;
        }

        boolean awaitAndUpdateFrame(int timeoutMs) {
            long deadline = SystemClock.elapsedRealtime() + Math.max(1, timeoutMs);
            synchronized (frameLock) {
                while (frameAvailableCount <= consumedFrameCount) {
                    long waitMs = deadline - SystemClock.elapsedRealtime();
                    if (waitMs <= 0) {
                        return false;
                    }
                    try {
                        frameLock.wait(waitMs);
                    } catch (InterruptedException error) {
                        Thread.currentThread().interrupt();
                        return false;
                    }
                }
                consumedFrameCount++;
            }

            try {
                if (!EGL14.eglMakeCurrent(eglDisplay, eglSurface, eglSurface, eglContext)) {
                    return false;
                }
                surfaceTexture.updateTexImage();
                surfaceTexture.getTransformMatrix(textureTransform);
                lastTextureTimestampNs = surfaceTexture.getTimestamp();
                textureUpdateCount++;
                if (firstTextureUpdateNs == 0L) {
                    firstTextureUpdateNs = SystemClock.elapsedRealtimeNanos();
                }
                return true;
            } catch (RuntimeException error) {
                Log.w(TAG, "Could not update broker H.264 SurfaceTexture frame", error);
                return false;
            }
        }

        int frameAvailableCount() {
            synchronized (frameLock) {
                return frameAvailableCount;
            }
        }

        int textureUpdateCount() {
            return textureUpdateCount;
        }

        long firstFrameAvailableNs() {
            synchronized (frameLock) {
                return firstFrameAvailableNs;
            }
        }

        long firstTextureUpdateNs() {
            return firstTextureUpdateNs;
        }

        long lastTextureTimestampNs() {
            return lastTextureTimestampNs;
        }

        float[] textureTransform() {
            float[] copy = new float[textureTransform.length];
            System.arraycopy(textureTransform, 0, copy, 0, textureTransform.length);
            return copy;
        }

        void release() {
            try {
                surface.release();
            } catch (RuntimeException ignored) {
            }
            try {
                surfaceTexture.release();
            } catch (RuntimeException ignored) {
            }
            try {
                EGL14.eglMakeCurrent(eglDisplay, eglSurface, eglSurface, eglContext);
                if (textureId > 0) {
                    GLES20.glDeleteTextures(1, new int[] { textureId }, 0);
                }
            } catch (RuntimeException ignored) {
            }
            EGL14.eglMakeCurrent(
                eglDisplay,
                EGL14.EGL_NO_SURFACE,
                EGL14.EGL_NO_SURFACE,
                EGL14.EGL_NO_CONTEXT);
            EGL14.eglDestroySurface(eglDisplay, eglSurface);
            EGL14.eglDestroyContext(eglDisplay, eglContext);
            EGL14.eglTerminate(eglDisplay);
        }

        @Override
        public void onFrameAvailable(SurfaceTexture ignored) {
            synchronized (frameLock) {
                frameAvailableCount++;
                if (firstFrameAvailableNs == 0L) {
                    firstFrameAvailableNs = SystemClock.elapsedRealtimeNanos();
                }
                frameLock.notifyAll();
            }
        }
    }

    private static void checkGl(String action) {
        int error = GLES20.glGetError();
        if (error != GLES20.GL_NO_ERROR) {
            throw new IllegalStateException(action + " failed with GL error 0x" + Integer.toHexString(error));
        }
    }

    private static final class DecodeHardwareBufferTarget {
        private final ImageReader reader;
        private final int width;
        private final int height;
        private int imageCount;
        private int deliveredCount;
        private int nativeAcceptedCount;
        private int nativeRejectedCount;
        private int missingBufferCount;

        private DecodeHardwareBufferTarget(ImageReader reader, int width, int height) {
            this.reader = reader;
            this.width = width;
            this.height = height;
        }

        static DecodeHardwareBufferTarget create(int width, int height) {
            ImageReader reader = ImageReader.newInstance(
                Math.max(1, width),
                Math.max(1, height),
                ImageFormat.PRIVATE,
                HARDWARE_BUFFER_READER_MAX_IMAGES);
            return new DecodeHardwareBufferTarget(reader, width, height);
        }

        Surface surface() {
            return reader.getSurface();
        }

        DeliverResult awaitAndDeliverFrame(
            int timeoutMs,
            String cameraId,
            long presentationTimeUs,
            long sourceElapsedNs,
            JSONObject streamProjectionMetadata) {
            long deadline = SystemClock.elapsedRealtime() + Math.max(1, timeoutMs);
            Image image = null;
            while (SystemClock.elapsedRealtime() < deadline) {
                try {
                    image = reader.acquireNextImage();
                } catch (IllegalStateException error) {
                    return DeliverResult.notDelivered();
                }
                if (image != null) {
                    break;
                }
                SystemClock.sleep(5);
            }
            if (image == null) {
                return DeliverResult.notDelivered();
            }

            HardwareBuffer buffer = null;
            try {
                imageCount++;
                buffer = image.getHardwareBuffer();
                if (buffer == null) {
                    missingBufferCount++;
                    return DeliverResult.notDelivered();
                }

                long bufferId = 0L;
                if (Build.VERSION.SDK_INT >= 34) {
                    bufferId = buffer.getId();
                }
                long timestampNs = image.getTimestamp();
                if (sourceElapsedNs > 0L) {
                    timestampNs = sourceElapsedNs;
                }
                if (timestampNs <= 0L && presentationTimeUs > 0L) {
                    timestampNs = presentationTimeUs * 1000L;
                }
                if (timestampNs <= 0L) {
                    timestampNs = SystemClock.elapsedRealtimeNanos();
                }
                String metadata = buildDecodedHardwareBufferMetadataJson(
                    cameraId,
                    image.getWidth() > 0 ? image.getWidth() : width,
                    image.getHeight() > 0 ? image.getHeight() : height,
                    timestampNs,
                    streamProjectionMetadata);
                boolean accepted = nativeBrokerH264DecodedHardwareBufferFrame(
                    image.getWidth() > 0 ? image.getWidth() : width,
                    image.getHeight() > 0 ? image.getHeight() : height,
                    timestampNs,
                    metadata,
                    buffer,
                    buffer.getFormat(),
                    buffer.getUsage(),
                    buffer.getLayers(),
                    bufferId);
                deliveredCount++;
                if (accepted) {
                    nativeAcceptedCount++;
                } else {
                    nativeRejectedCount++;
                }
                return new DeliverResult(
                    true,
                    accepted,
                    buffer.getFormat(),
                    buffer.getUsage(),
                    buffer.getLayers(),
                    bufferId);
            } catch (RuntimeException error) {
                nativeRejectedCount++;
                Log.w(TAG, "Could not deliver broker H.264 decoded hardware buffer", error);
                return DeliverResult.notDelivered();
            } finally {
                if (buffer != null) {
                    buffer.close();
                }
                image.close();
            }
        }

        DeliverResult awaitAndRetainFrame(
            int timeoutMs,
            String cameraId,
            String stereoEye,
            long presentationTimeUs,
            long sourceElapsedNs,
            JSONObject streamProjectionMetadata,
            List<DecodedHardwareBufferFrame> outFrames) {
            long deadline = SystemClock.elapsedRealtime() + Math.max(1, timeoutMs);
            Image image = null;
            while (SystemClock.elapsedRealtime() < deadline) {
                try {
                    image = reader.acquireNextImage();
                } catch (IllegalStateException error) {
                    return DeliverResult.notDelivered();
                }
                if (image != null) {
                    break;
                }
                SystemClock.sleep(5);
            }
            if (image == null) {
                return DeliverResult.notDelivered();
            }

            HardwareBuffer buffer = null;
            try {
                imageCount++;
                buffer = image.getHardwareBuffer();
                if (buffer == null) {
                    missingBufferCount++;
                    return DeliverResult.notDelivered();
                }

                long bufferId = 0L;
                if (Build.VERSION.SDK_INT >= 34) {
                    bufferId = buffer.getId();
                }
                long timestampNs = image.getTimestamp();
                if (sourceElapsedNs > 0L) {
                    timestampNs = sourceElapsedNs;
                }
                if (timestampNs <= 0L && presentationTimeUs > 0L) {
                    timestampNs = presentationTimeUs * 1000L;
                }
                if (timestampNs <= 0L) {
                    timestampNs = SystemClock.elapsedRealtimeNanos();
                }
                int frameWidth = image.getWidth() > 0 ? image.getWidth() : width;
                int frameHeight = image.getHeight() > 0 ? image.getHeight() : height;
                String metadata = buildDecodedHardwareBufferMetadataJson(
                    cameraId,
                    frameWidth,
                    frameHeight,
                    timestampNs,
                    streamProjectionMetadata,
                    "separate",
                    stereoEye,
                    "gpu-projected",
                    "gpu-projected",
                    false,
                    "broker decoded stereo H.264 hardware buffer with Camera2 projection metadata");
                outFrames.add(new DecodedHardwareBufferFrame(
                    frameWidth,
                    frameHeight,
                    timestampNs,
                    metadata,
                    buffer,
                    buffer.getFormat(),
                    buffer.getUsage(),
                    buffer.getLayers(),
                    bufferId));
                deliveredCount++;
                return new DeliverResult(
                    true,
                    true,
                    buffer.getFormat(),
                    buffer.getUsage(),
                    buffer.getLayers(),
                    bufferId);
            } catch (RuntimeException error) {
                Log.w(TAG, "Could not retain broker H.264 decoded hardware buffer", error);
                if (buffer != null) {
                    try {
                        buffer.close();
                    } catch (RuntimeException ignored) {
                    }
                }
                return DeliverResult.notDelivered();
            } finally {
                image.close();
            }
        }

        int imageCount() {
            return imageCount;
        }

        int deliveredCount() {
            return deliveredCount;
        }

        int nativeAcceptedCount() {
            return nativeAcceptedCount;
        }

        int nativeRejectedCount() {
            return nativeRejectedCount;
        }

        int missingBufferCount() {
            return missingBufferCount;
        }

        void release() {
            reader.close();
        }

        private static final class DeliverResult {
            final boolean delivered;
            final boolean accepted;
            final int format;
            final long usage;
            final int layers;
            final long bufferId;

            DeliverResult(boolean delivered, boolean accepted, int format, long usage, int layers, long bufferId) {
                this.delivered = delivered;
                this.accepted = accepted;
                this.format = format;
                this.usage = usage;
                this.layers = layers;
                this.bufferId = bufferId;
            }

            static DeliverResult notDelivered() {
                return new DeliverResult(false, false, 0, 0L, 0, 0L);
            }
        }
    }

    private static String buildDecodedHardwareBufferMetadataJson(
        String cameraId,
        int width,
        int height,
        long timestampNs,
        JSONObject streamProjectionMetadata) {
        return buildDecodedHardwareBufferMetadataJson(
            cameraId,
            width,
            height,
            timestampNs,
            streamProjectionMetadata,
            "mono",
            "",
            "gpu-buffer-probe",
            "gpu-buffer-probe",
            true,
            null);
    }

    private static String buildDecodedHardwareBufferMetadataJson(
        String cameraId,
        int width,
        int height,
        long timestampNs,
        JSONObject streamProjectionMetadata,
        String stereoLayout,
        String stereoEye,
        String requestedTier,
        String activeTier,
        boolean monoFallback,
        String fallbackReasonOverride) {
        JSONObject metadata = new JSONObject();
        try {
            boolean hasStreamProjectionMetadata = streamProjectionMetadata != null;
            String metadataCameraId = hasStreamProjectionMetadata
                ? streamProjectionMetadata.optString("cameraId", "")
                : "";
            JSONObject intrinsics = hasStreamProjectionMetadata
                ? streamProjectionMetadata.optJSONObject("intrinsics")
                : null;
            JSONObject intrinsicsDomain = hasStreamProjectionMetadata
                ? streamProjectionMetadata.optJSONObject("intrinsicsDomain")
                : null;
            JSONObject extrinsics = hasStreamProjectionMetadata
                ? streamProjectionMetadata.optJSONObject("extrinsics")
                : null;
            boolean hasIntrinsics = intrinsics != null && intrinsicsDomain != null;
            boolean hasPose = extrinsics != null &&
                !"missing".equals(streamProjectionMetadata.optString("poseSource", "missing")) &&
                !streamProjectionMetadata.optBoolean("missingPose", false);

            metadata.put("sourceLabel", "Broker H.264 decoded hardware buffer");
            metadata.put(
                "cameraId",
                metadataCameraId.length() > 0
                    ? metadataCameraId
                    : (cameraId != null && cameraId.length() > 0 ? cameraId : "broker-h264"));
            metadata.put(
                "lensFacing",
                hasStreamProjectionMetadata
                    ? streamProjectionMetadata.optString("lensFacing", "unknown")
                    : "unknown");
            metadata.put(
                "lensFacingRank",
                hasStreamProjectionMetadata ? streamProjectionMetadata.optInt("lensFacingRank", 0) : 0);
            metadata.put(
                "selectionScore",
                hasStreamProjectionMetadata ? streamProjectionMetadata.optLong("selectionScore", 0L) : 0L);
            metadata.put("deliveredWidth", width);
            metadata.put("deliveredHeight", height);
            metadata.put("timestampNs", timestampNs);
            if (hasStreamProjectionMetadata && streamProjectionMetadata.has("sensorOrientationDegrees")) {
                metadata.put("sensorOrientationDegrees", streamProjectionMetadata.optInt("sensorOrientationDegrees"));
            }
            metadata.put("stereoLayout", stereoLayout != null && stereoLayout.length() > 0 ? stereoLayout : "mono");
            metadata.put(
                "requestedStereoLayout",
                stereoLayout != null && stereoLayout.length() > 0 ? stereoLayout : "mono");
            if (stereoEye != null && stereoEye.length() > 0) {
                metadata.put("eye", stereoEye);
            }
            metadata.put("transport", "android-mediacodec-h264-decoder-hardware-buffer");
            metadata.put("requestedTier", requestedTier != null ? requestedTier : "gpu-buffer-probe");
            metadata.put("activeTier", activeTier != null ? activeTier : "gpu-buffer-probe");
            metadata.put("gpuImportRequested", true);
            metadata.put("missingIntrinsics", !hasIntrinsics);
            metadata.put("missingPose", !hasPose);
            metadata.put(
                "poseSource",
                hasStreamProjectionMetadata
                    ? streamProjectionMetadata.optString("poseSource", hasPose ? "platform" : "missing")
                    : "missing");
            metadata.put(
                "poseCoordinateConvention",
                hasStreamProjectionMetadata
                    ? streamProjectionMetadata.optString("poseCoordinateConvention", "broker-decoded-h264-image-space")
                    : "broker-decoded-h264-image-space");
            if (hasStreamProjectionMetadata && streamProjectionMetadata.has("lensPoseReferenceLabel")) {
                metadata.put("lensPoseReferenceLabel", streamProjectionMetadata.optString("lensPoseReferenceLabel"));
            }
            if (intrinsics != null) {
                metadata.put("intrinsics", new JSONObject(intrinsics.toString()));
            }
            if (intrinsicsDomain != null) {
                metadata.put("intrinsicsDomain", new JSONObject(intrinsicsDomain.toString()));
            }
            JSONObject activeArrayDomain = hasStreamProjectionMetadata
                ? streamProjectionMetadata.optJSONObject("activeArrayDomain")
                : null;
            if (activeArrayDomain != null) {
                metadata.put("activeArrayDomain", new JSONObject(activeArrayDomain.toString()));
            }
            JSONObject sensorPixelDomain = hasStreamProjectionMetadata
                ? streamProjectionMetadata.optJSONObject("sensorPixelDomain")
                : null;
            if (sensorPixelDomain != null) {
                metadata.put("sensorPixelDomain", new JSONObject(sensorPixelDomain.toString()));
            }
            if (extrinsics != null) {
                metadata.put("extrinsics", new JSONObject(extrinsics.toString()));
            }
            metadata.put("monoFallback", monoFallback);
            metadata.put(
                "fallbackReason",
                fallbackReasonOverride != null && fallbackReasonOverride.length() > 0
                    ? fallbackReasonOverride
                    : (hasIntrinsics && hasPose
                        ? "broker decoded H.264 hardware buffer with Camera2 projection metadata; mono diagnostic draw until stereo projection is selected"
                        : "broker decoded H.264 hardware buffer; projection metadata not yet attached"));
        } catch (Exception ignored) {
        }
        return metadata.toString();
    }

    private Socket connectWithRetry(String host, int port, int timeoutMs, String label) throws Exception {
        long deadline = SystemClock.elapsedRealtimeNanos() + timeoutMs * 1_000_000L;
        Exception lastError = null;
        while (running && SystemClock.elapsedRealtimeNanos() < deadline) {
            Socket socket = new Socket();
            try {
                socket.connect(new InetSocketAddress(host, port), 500);
                socket.setTcpNoDelay(true);
                return socket;
            } catch (Exception ex) {
                lastError = ex;
                closeQuietly(socket);
                Thread.sleep(50);
            }
        }

        throw new IllegalStateException(
            "Timed out connecting to broker H.264 " + label + " stream on port " + port + ": " +
                (lastError != null ? safeMessage(lastError) : ""));
    }

    private static void sendMaskedTextFrame(OutputStream output, String text) throws Exception {
        byte[] payload = text.getBytes(StandardCharsets.UTF_8);
        output.write(0x81);
        if (payload.length < 126) {
            output.write(0x80 | payload.length);
        } else if (payload.length <= 65535) {
            output.write(0x80 | 126);
            output.write((payload.length >>> 8) & 0xff);
            output.write(payload.length & 0xff);
        } else {
            output.write(0x80 | 127);
            long length = payload.length;
            for (int i = 7; i >= 0; i--) {
                output.write((int) ((length >>> (i * 8)) & 0xff));
            }
        }

        byte[] mask = new byte[4];
        new Random(System.nanoTime()).nextBytes(mask);
        output.write(mask);
        for (int i = 0; i < payload.length; i++) {
            output.write(payload[i] ^ mask[i % 4]);
        }
        output.flush();
    }

    private static String readWebSocketTextFrame(InputStream input) throws Exception {
        int first = input.read();
        if (first < 0) {
            return "";
        }
        int second = input.read();
        if (second < 0) {
            return "";
        }

        int opcode = first & 0x0f;
        boolean masked = (second & 0x80) != 0;
        long length = second & 0x7f;
        if (length == 126) {
            length = readUnsignedShort(input);
        } else if (length == 127) {
            length = readLong(input);
        }
        if (length < 0 || length > 1024 * 1024) {
            throw new IllegalStateException("Broker WebSocket frame is too large.");
        }

        byte[] mask = null;
        if (masked) {
            mask = readExact(input, 4);
        }
        byte[] payload = readExact(input, (int) length);
        if (mask != null) {
            for (int i = 0; i < payload.length; i++) {
                payload[i] = (byte) (payload[i] ^ mask[i % 4]);
            }
        }
        return opcode == 1 ? new String(payload, StandardCharsets.UTF_8) : "";
    }

    private static String readHttpLine(InputStream input) throws Exception {
        ByteArrayOutputStream buffer = new ByteArrayOutputStream();
        int previous = -1;
        while (true) {
            int value = input.read();
            if (value < 0) {
                break;
            }
            if (previous == '\r' && value == '\n') {
                break;
            }
            buffer.write(value);
            previous = value;
            if (buffer.size() > 8192) {
                throw new IllegalStateException("HTTP line exceeded 8192 bytes.");
            }
        }
        byte[] bytes = buffer.toByteArray();
        int length = bytes.length;
        if (length > 0 && bytes[length - 1] == '\r') {
            length--;
        }
        return new String(bytes, 0, length, StandardCharsets.US_ASCII);
    }

    private static int readUnsignedShort(InputStream input) throws Exception {
        int high = input.read();
        int low = input.read();
        if (high < 0 || low < 0) {
            throw new IllegalStateException("Truncated unsigned short.");
        }
        return ((high & 0xff) << 8) | (low & 0xff);
    }

    private static long readLong(InputStream input) throws Exception {
        long value = 0L;
        for (int i = 0; i < 8; i++) {
            int next = input.read();
            if (next < 0) {
                throw new IllegalStateException("Truncated long.");
            }
            value = (value << 8) | (next & 0xffL);
        }
        return value;
    }

    private static byte[] readExact(InputStream input, int length) throws Exception {
        byte[] bytes = new byte[length];
        int offset = 0;
        while (offset < length) {
            int read = input.read(bytes, offset, length - offset);
            if (read < 0) {
                throw new IllegalStateException("Truncated frame payload.");
            }
            offset += read;
        }
        return bytes;
    }

    private static NalUnit findNalUnit(List<Packet> packets, int nalType) {
        for (int i = 0; i < packets.size(); i++) {
            byte[] payload = packets.get(i).payload;
            int start = findStartCode(payload, 0);
            while (start >= 0) {
                int startCodeLength = startCodeLengthAt(payload, start);
                int nalStart = start + startCodeLength;
                if (nalStart >= payload.length) {
                    break;
                }
                int nextStart = findStartCode(payload, nalStart);
                int nalEnd = nextStart >= 0 ? nextStart : payload.length;
                if ((payload[nalStart] & 0x1f) == nalType) {
                    byte[] bytes = new byte[nalEnd - start];
                    System.arraycopy(payload, start, bytes, 0, bytes.length);
                    return new NalUnit(bytes);
                }
                start = nextStart;
            }
        }
        return null;
    }

    private static int findStartCode(byte[] data, int offset) {
        for (int i = Math.max(0, offset); i < data.length - 2; i++) {
            if (startCodeLengthAt(data, i) > 0) {
                return i;
            }
        }
        return -1;
    }

    private static int startCodeLengthAt(byte[] data, int offset) {
        if (offset + 4 <= data.length &&
            data[offset] == 0 &&
            data[offset + 1] == 0 &&
            data[offset + 2] == 0 &&
            data[offset + 3] == 1) {
            return 4;
        }
        if (offset + 3 <= data.length &&
            data[offset] == 0 &&
            data[offset + 1] == 0 &&
            data[offset + 2] == 1) {
            return 3;
        }
        return 0;
    }

    private static void applyOutputFormat(DecodeResult result, MediaFormat format, StreamResult stream) {
        result.outputMime = mediaFormatString(format, MediaFormat.KEY_MIME, "video/raw");
        result.outputWidth = mediaFormatInt(format, MediaFormat.KEY_WIDTH, stream.width);
        result.outputHeight = mediaFormatInt(format, MediaFormat.KEY_HEIGHT, stream.height);
    }

    private static int mediaFormatInt(MediaFormat format, String key, int fallback) {
        try {
            return format.containsKey(key) ? format.getInteger(key) : fallback;
        } catch (Exception ignored) {
            return fallback;
        }
    }

    private static String mediaFormatString(MediaFormat format, String key, String fallback) {
        try {
            String value = format.containsKey(key) ? format.getString(key) : fallback;
            return value != null ? value : fallback;
        } catch (Exception ignored) {
            return fallback;
        }
    }

    private static long lastPresentationTimeUs(List<Packet> packets) {
        return packets.size() > 0 ? packets.get(packets.size() - 1).ptsUs : 0L;
    }

    private static long sourceElapsedNsForPts(StreamResult stream, long ptsUs) {
        long fallback = 0L;
        long closest = 0L;
        long closestDelta = Long.MAX_VALUE;
        for (int i = 0; i < stream.packets.size(); i++) {
            Packet packet = stream.packets.get(i);
            if (packet.sourceElapsedNs > 0L) {
                if (fallback == 0L) {
                    fallback = packet.sourceElapsedNs;
                }
                if (packet.ptsUs == ptsUs) {
                    return packet.sourceElapsedNs;
                }
                long delta = Math.abs(packet.ptsUs - ptsUs);
                if (delta < closestDelta) {
                    closestDelta = delta;
                    closest = packet.sourceElapsedNs;
                }
            }
        }
        return closest > 0L ? closest : fallback;
    }

    private static void closeQuietly(Socket socket) {
        if (socket != null) {
            try {
                socket.close();
            } catch (Exception ignored) {
            }
        }
    }

    private static String safeMessage(Exception ex) {
        String message = ex.getMessage();
        return message != null ? message : "";
    }

    private static final class Packet {
        final long ptsUs;
        final int flags;
        final long sourceElapsedNs;
        final long sourceUnixNs;
        final long receiveElapsedNs;
        final byte[] payload;

        Packet(long ptsUs, int flags, long sourceElapsedNs, long sourceUnixNs, long receiveElapsedNs, byte[] payload) {
            this.ptsUs = ptsUs;
            this.flags = flags;
            this.sourceElapsedNs = sourceElapsedNs;
            this.sourceUnixNs = sourceUnixNs;
            this.receiveElapsedNs = receiveElapsedNs;
            this.payload = payload;
        }
    }

    private static final class StreamResult {
        final int schemaVersion;
        final int codecId;
        final int width;
        final int height;
        final int declaredPacketCount;
        final List<Packet> packets;
        final long payloadBytes;
        final long receiveStartElapsedNs;
        final long receiveEndElapsedNs;
        final long firstPacketReceiveElapsedNs;
        final long lastPacketReceiveElapsedNs;
        final long firstSourceElapsedNs;
        final long lastSourceElapsedNs;

        StreamResult(
            int schemaVersion,
            int codecId,
            int width,
            int height,
            int declaredPacketCount,
            List<Packet> packets,
            long payloadBytes,
            long receiveStartElapsedNs,
            long receiveEndElapsedNs,
            long firstPacketReceiveElapsedNs,
            long lastPacketReceiveElapsedNs,
            long firstSourceElapsedNs,
            long lastSourceElapsedNs) {
            this.schemaVersion = schemaVersion;
            this.codecId = codecId;
            this.width = width;
            this.height = height;
            this.declaredPacketCount = declaredPacketCount;
            this.packets = packets;
            this.payloadBytes = payloadBytes;
            this.receiveStartElapsedNs = receiveStartElapsedNs;
            this.receiveEndElapsedNs = receiveEndElapsedNs;
            this.firstPacketReceiveElapsedNs = firstPacketReceiveElapsedNs;
            this.lastPacketReceiveElapsedNs = lastPacketReceiveElapsedNs;
            this.firstSourceElapsedNs = firstSourceElapsedNs;
            this.lastSourceElapsedNs = lastSourceElapsedNs;
        }
    }

    private static final class NalUnit {
        final byte[] bytes;

        NalUnit(byte[] bytes) {
            this.bytes = bytes;
        }
    }

    private static final class DecodedHardwareBufferFrame {
        final int width;
        final int height;
        final long timestampNs;
        final String metadataJson;
        HardwareBuffer buffer;
        final int format;
        final long usage;
        final int layers;
        final long bufferId;

        DecodedHardwareBufferFrame(
            int width,
            int height,
            long timestampNs,
            String metadataJson,
            HardwareBuffer buffer,
            int format,
            long usage,
            int layers,
            long bufferId) {
            this.width = width;
            this.height = height;
            this.timestampNs = timestampNs;
            this.metadataJson = metadataJson;
            this.buffer = buffer;
            this.format = format;
            this.usage = usage;
            this.layers = layers;
            this.bufferId = bufferId;
        }

        void close() {
            if (buffer != null) {
                try {
                    buffer.close();
                } catch (RuntimeException ignored) {
                }
                buffer = null;
            }
        }
    }

    private static final class StereoPairResult {
        int pairCount;
        int nativeAcceptedCount;
        int nativeRejectedCount;
        int resolutionMismatchCount;
        long deltaTotalNs;
        long deltaMaxNs;
    }

    private static final class DecodeResult {
        String decodeOutputMode = "";
        String decoderName = "";
        String outputMime = "";
        int outputWidth;
        int outputHeight;
        int spsBytes;
        int ppsBytes;
        int inputBufferCount;
        long inputBytes;
        boolean inputEosQueued;
        int outputFormatChanges;
        int outputBufferCount;
        int decodedFrameCount;
        long outputBytes;
        boolean outputEosSeen;
        boolean surfaceTargetCreated;
        boolean eglContextCreated;
        boolean externalTextureCreated;
        int externalTextureId;
        int surfaceReleaseCount;
        int surfaceFrameAvailableCount;
        int surfaceTextureUpdateCount;
        long firstSurfaceFrameAvailableNs;
        long firstSurfaceTextureUpdateNs;
        long lastSurfaceTextureTimestampNs;
        float[] surfaceTextureTransform;
        boolean hardwareBufferTargetCreated;
        int hardwareBufferReaderWidth;
        int hardwareBufferReaderHeight;
        int hardwareBufferImageCount;
        int hardwareBufferDeliveredCount;
        int hardwareBufferNativeAcceptedCount;
        int hardwareBufferNativeRejectedCount;
        int hardwareBufferMissingCount;
        int lastHardwareBufferFormat;
        long lastHardwareBufferUsage;
        int lastHardwareBufferLayers;
        long lastHardwareBufferId;
        int captureWindowMs;
        long decodeStartElapsedNs;
        long decodeEndElapsedNs;
        final List<DecodedHardwareBufferFrame> collectedHardwareBufferFrames =
            new ArrayList<DecodedHardwareBufferFrame>();
        String lastError = "";
    }
}
