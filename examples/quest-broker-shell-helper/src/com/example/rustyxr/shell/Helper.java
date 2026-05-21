package com.example.rustyxr.shell;

import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.ImageFormat;
import android.graphics.Paint;
import android.graphics.Rect;
import android.graphics.YuvImage;
import android.content.AttributionSource;
import android.content.Context;
import android.content.ContextWrapper;
import android.content.pm.PackageManager;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
import android.media.MediaCodec;
import android.media.MediaFormat;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Process;
import android.os.SystemClock;
import android.util.Size;
import android.view.Surface;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.EOFException;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class Helper {
    private static final String VERSION = "0.1.0-public-proof";
    private static final String COMMAND_SCHEMA = "rusty.xr.broker.command.v1";
    private static final String EVENTS_PATH = "/rustyxr/v1/events";
    private static final String CLIENT_ID = "rusty-xr-adb-shell-helper";
    private static final String SYNTHETIC_BINARY_MAGIC = "RXYRVID1";
    private static final int SYNTHETIC_BINARY_SCHEMA_VERSION = 1;
    private static final int SYNTHETIC_BINARY_CODEC_H264 = 1;
    private static final int SYNTHETIC_VIDEO_WIDTH = 1280;
    private static final int SYNTHETIC_VIDEO_HEIGHT = 720;
    private static final int SYNTHETIC_VIDEO_FRAME_RATE_HZ = 30;
    private static final int SYNTHETIC_BINARY_DEFAULT_PORT = 8877;
    private static final int SYNTHETIC_BINARY_DEFAULT_PACKET_COUNT = 3;
    private static final int SYNTHETIC_BINARY_DEFAULT_PACKET_BYTES = 1024;
    private static final int SYNTHETIC_BINARY_MAX_PACKET_COUNT = 30;
    private static final int SYNTHETIC_BINARY_MAX_PACKET_BYTES = 65536;
    private static final int MEDIACODEC_DEFAULT_WIDTH = 640;
    private static final int MEDIACODEC_DEFAULT_HEIGHT = 360;
    private static final int MEDIACODEC_DEFAULT_FRAMES = 8;
    private static final int MEDIACODEC_DEFAULT_BITRATE_BPS = 1000000;
    private static final int MEDIACODEC_MAX_FRAMES = 60;
    private static final int BINARY_STREAM_MAX_PACKET_BYTES = 1024 * 1024;
    private static final int SCREENRECORD_DEFAULT_SECONDS = 1;
    private static final int SCREENRECORD_MAX_SECONDS = 3;
    private static final int SCREENRECORD_DEFAULT_PACKET_BYTES = 16384;
    private static final int SCREENRECORD_MIN_PACKET_BYTES = 4096;
    private static final int CAMERA_DUMPSYS_MAX_BYTES = 256 * 1024;
    private static final int CAMERA_DUMPSYS_TIMEOUT_SECONDS = 5;
    private static final int CAMERA_PROBE_MAX_DEVICES = 12;
    private static final int CAMERA_PROBE_MAX_STREAM_CONFIGS_PER_DEVICE = 64;
    private static final int CAMERA_PROBE_MAX_FPS_ROWS_PER_DEVICE = 16;
    private static final int CAMERA_OPEN_PROBE_MAX_CAMERA_IDS = 8;
    private static final int CAMERA_OPEN_PROBE_OPEN_TIMEOUT_MS = 3000;
    private static final int CAMERA_OPEN_PROBE_SESSION_TIMEOUT_MS = 3000;
    private static final int CAMERA_OPEN_PROBE_CAPTURE_TIMEOUT_MS = 4000;
    private static final int CAMERA_OPEN_PROBE_MAX_DIMENSION = 640;
    private static final String CAMERA_FRAME_CAPTURE_DEFAULT_DIR =
        "/data/local/tmp/rusty-xr-camera-frame-capture";
    private static final int CAMERA_FRAME_CAPTURE_DEFAULT_JPEG_QUALITY = 95;
    private static final String PROXIMITY_WATCHDOG_STOP_FILE =
        "/data/local/tmp/rusty-xr-proximity-watchdog.stop";
    private static final String PROXIMITY_WATCHDOG_LOG_FILE =
        "/data/local/tmp/rusty-xr-proximity-watchdog.log";
    private static final String FOCUS_GUARDIAN_STOP_FILE =
        "/data/local/tmp/rusty-xr-focus-guardian.stop";
    private static final String FOCUS_GUARDIAN_LOG_FILE =
        "/data/local/tmp/rusty-xr-focus-guardian.log";
    private static final String VR_POWER_MANAGER_PROX_CLOSE_ACTION =
        "com.oculus.vrpowermanager.prox_close";
    private static final int VR_POWER_MANAGER_DUMPSYS_MAX_BYTES = 128 * 1024;
    private static final int VR_POWER_MANAGER_COMMAND_TIMEOUT_SECONDS = 5;
    private static final int POWER_DUMPSYS_MAX_BYTES = 128 * 1024;
    private static final int POWER_COMMAND_TIMEOUT_SECONDS = 5;
    private static final int PROXIMITY_WATCHDOG_DEFAULT_DURATION_MS = 28_800_000;
    private static final int PROXIMITY_WATCHDOG_DEFAULT_HOLD_MS = 28_800_000;
    private static final int PROXIMITY_WATCHDOG_DEFAULT_INTERVAL_MS = 5_000;
    private static final int PROXIMITY_WATCHDOG_MIN_INTERVAL_MS = 1_000;
    private static final int PROXIMITY_WATCHDOG_MAX_INTERVAL_MS = 60_000;
    private static final int FOCUS_GUARDIAN_DEFAULT_DURATION_MS = 28_800_000;
    private static final int FOCUS_GUARDIAN_DEFAULT_INTERVAL_MS = 1_000;
    private static final int FOCUS_GUARDIAN_MIN_INTERVAL_MS = 250;
    private static final int FOCUS_GUARDIAN_MAX_INTERVAL_MS = 10_000;
    private static final int FOCUS_GUARDIAN_DEFAULT_COOLDOWN_MS = 1_500;
    private static final int FOCUS_DUMPSYS_MAX_BYTES = 192 * 1024;
    private static final int FOCUS_DUMPSYS_TIMEOUT_SECONDS = 3;
    private static final int FOCUS_GUARDIAN_PENDING_RECOVERY_MS = 20_000;
    private static final int FOCUS_GUARDIAN_PENDING_RECOVERY_HOLD_MS = 2_000;
    private static final int FOCUS_GUARDIAN_MAX_PENDING_RECOVERY_ATTEMPTS = 12;
    private static final int FOCUS_GUARDIAN_DEFAULT_LAUNCH_GUARD_TIMEOUT_MS = 20_000;
    private static final int FOCUS_GUARDIAN_MIN_LAUNCH_GUARD_TIMEOUT_MS = 5_000;
    private static final int FOCUS_GUARDIAN_MAX_LAUNCH_GUARD_TIMEOUT_MS = 120_000;
    private static final int FOCUS_GUARDIAN_TOGGLE_TRANSITION_GRACE_MS = 5_000;
    private static final String DEFAULT_BROKER_PACKAGE = "com.example.rustyxr.broker";
    private static final String DEFAULT_BROKER_ACTIVITY = "com.example.rustyxr.broker.MainActivity";
    private static final String ANDROID_MAIN_ACTION = "android.intent.action.MAIN";
    private static final String ANDROID_LAUNCHER_CATEGORY = "android.intent.category.LAUNCHER";
    private static final String OCULUS_VR_CATEGORY = "com.oculus.intent.category.VR";
    private static final Pattern VR_POWER_MANAGER_VIRTUAL_STATE_PATTERN =
        Pattern.compile("Virtual proximity state:\\s*(\\S+)");
    private static final Pattern VR_POWER_MANAGER_HEADSET_STATE_PATTERN =
        Pattern.compile("^\\s*State:\\s*(.+)$", Pattern.MULTILINE);
    private static final Pattern POWER_WAKEFULNESS_PATTERN =
        Pattern.compile("mWakefulness=(\\S+)");
    private static final Pattern POWER_STAY_ON_PATTERN =
        Pattern.compile("mStayOn=(true|false)");
    private static final Pattern POWER_DISPLAY_STATE_PATTERN =
        Pattern.compile("Display Power:\\s*state=(\\S+)");
    private static final Pattern CURRENT_FOCUS_COMPONENT_PATTERN =
        Pattern.compile("mCurrentFocus=.*?\\s([A-Za-z0-9_.$]+)/(\\S+)");
    private static final Pattern FOCUSED_APP_COMPONENT_PATTERN =
        Pattern.compile("mFocusedApp=.*?\\s([A-Za-z0-9_.$]+)/(\\S+)");
    private static final Pattern WINDOW_HEADER_COMPONENT_PATTERN =
        Pattern.compile("\\bu\\d+\\s+([A-Za-z0-9_.$]+)/(\\S+?)\\}:");
    private static final Pattern PACKAGE_COMPONENT_PATTERN =
        Pattern.compile("^([A-Za-z0-9_.$]+)/(\\S+)$");

    private Helper() {
    }

    public static void main(String[] args) throws Exception {
        Options options = Options.parse(args);
        int uid = Process.myUid();
        String uidLabel = uid == 2000 ? "shell" : "uid:" + uid;
        JSONObject report = buildReport(uidLabel, options);
        JSONObject ack;
        if (options.noBrokerReport) {
            ack = new JSONObject();
            ack.put("type", "local_report");
            ack.put("status", "skipped_broker_report");
            ack.put("report", report);
        } else {
            ack = sendBrokerCommand(options.host, options.port, "shell_helper.report_status", report);
        }
        System.out.println("Rusty XR shell helper version=" + VERSION + " uid=" + uidLabel);
        System.out.println(ack.toString(2));
        if (options.connected && options.syntheticVideoSamples > 0) {
            emitSyntheticVideoMetadata(options, false);
        }
        if (options.connected && options.emitSyntheticVideoBinary) {
            emitSyntheticVideoMetadata(options, true);
            emitSyntheticVideoBinary(options);
        }
        if (options.connected && options.emitMediaCodecSyntheticVideo) {
            emitMediaCodecSyntheticVideo(options);
        }
        if (options.connected && options.emitScreenrecordVideo) {
            emitScreenrecordVideo(options);
        }
        if (options.stopProximityWatchdog) {
            requestProximityWatchdogStop();
        }
        if (options.stopFocusGuardian) {
            requestFocusGuardianStop();
        }
        if (options.connected && options.proximityWatchdog && options.focusGuardian) {
            final Options threadOptions = options;
            final String threadUidLabel = uidLabel;
            Thread proximityThread = new Thread(new Runnable() {
                @Override
                public void run() {
                    try {
                        runProximityWatchdog(threadOptions, threadUidLabel);
                    } catch (Exception ex) {
                        System.out.println("Proximity watchdog failed: " + exceptionSummary(ex));
                    }
                }
            }, "RustyXrProximityWatchdog");
            proximityThread.start();
            runFocusGuardian(options, uidLabel);
        } else if (options.connected && options.proximityWatchdog) {
            runProximityWatchdog(options, uidLabel);
        } else if (options.connected && options.focusGuardian) {
            runFocusGuardian(options, uidLabel);
        }
    }

    private static JSONObject buildReport(String uidLabel, Options options) throws Exception {
        JSONObject report = new JSONObject();
        report.put("connected", options.connected);
        report.put("helper_version", VERSION);
        report.put("uid", uidLabel);
        JSONArray capabilities = new JSONArray();
        capabilities.put("shell.uid.report");
        capabilities.put("shell.display.list.planned");
        capabilities.put("shell.camera.list.planned");
        capabilities.put("shell.encoded_stream.planned");
        if (options.probeCodecs) {
            capabilities.put("shell.codec.query");
        }
        if (options.probeCameras) {
            capabilities.put("shell.camera.dumpsys_metadata");
        }
        if (options.probeCameraOpen) {
            capabilities.put("shell.camera.camera2_open_capture_probe");
        }
        if (options.captureCameraFrame) {
            capabilities.put("shell.camera.camera2_yuv_frame_persist");
            capabilities.put("shell.camera.camera2_yuv_nv21_sidecar");
            capabilities.put("shell.camera.camera2_yuv_jpeg_preview");
        }
        if (options.syntheticVideoSamples > 0 || options.emitSyntheticVideoBinary) {
            capabilities.put("shell.synthetic_encoded_metadata.emit");
        }
        if (options.emitSyntheticVideoBinary) {
            capabilities.put("shell.synthetic_encoded_binary.emit");
        }
        if (options.emitMediaCodecSyntheticVideo) {
            capabilities.put("shell.mediacodec.synthetic_surface_encode");
        }
        if (options.emitScreenrecordVideo) {
            capabilities.put("shell.screenrecord.h264_capture");
        }
        if (options.proximityWatchdog || options.stopProximityWatchdog) {
            capabilities.put("shell.proximity_watchdog.v1");
            capabilities.put("shell.proximity_watchdog.stop_file");
            capabilities.put("shell.proximity_watchdog.until_stopped");
            capabilities.put("shell.proximity_watchdog.stay_awake");
        }
        if (options.focusGuardian || options.stopFocusGuardian) {
            capabilities.put("shell.focus_guardian.v1");
            capabilities.put("shell.focus_guardian.stop_file");
            capabilities.put("shell.focus_guardian.setprop_whitelist.debug_rustyxr");
            capabilities.put("shell.focus_guardian.launch_target_guard.v1");
        }
        report.put("capabilities", capabilities);
        JSONArray activeStreams = new JSONArray();
        if (options.connected && (
                options.syntheticVideoSamples > 0 ||
                options.emitSyntheticVideoBinary ||
                options.emitMediaCodecSyntheticVideo ||
                options.emitScreenrecordVideo)) {
            activeStreams.put("video_lab.encoded_stream_manifest");
            activeStreams.put("video_lab.encoded_sample_metadata");
        }
        if (options.connected && (
                options.emitSyntheticVideoBinary ||
                options.emitMediaCodecSyntheticVideo ||
                options.emitScreenrecordVideo)) {
            activeStreams.put("video_lab.encoded_binary_payload");
        }
        if (options.connected && options.proximityWatchdog) {
            activeStreams.put("shell_helper.status");
        }
        if (options.connected && options.focusGuardian) {
            activeStreams.put("shell_helper.status");
        }
        report.put("active_streams", activeStreams);
        report.put("last_error", "");
        if (options.probeCodecs ||
                options.probeCameras ||
                options.probeCameraOpen ||
                options.captureCameraFrame ||
                options.proximityWatchdog ||
                options.stopProximityWatchdog ||
                options.focusGuardian ||
                options.stopFocusGuardian) {
            JSONObject diagnostics = new JSONObject();
            if (options.probeCodecs) {
                diagnostics.put("codec_probe", buildCodecProbe());
            }
            if (options.probeCameras) {
                diagnostics.put("camera_probe", buildCameraProbe());
            }
            if (options.probeCameraOpen || options.captureCameraFrame) {
                diagnostics.put(
                    "camera_open_probe",
                    buildCameraOpenProbe(
                        options.cameraOpenId,
                        options.captureCameraFrame,
                        options.cameraFrameOutputDir,
                        options.cameraFrameJpegQuality));
            }
            if (options.proximityWatchdog || options.stopProximityWatchdog) {
                diagnostics.put(
                    "proximity_watchdog",
                    buildProximityWatchdogStatus(
                        options,
                        options.connected && options.proximityWatchdog,
                        "initial_report",
                        "",
                        "",
                        "",
                        "",
                        false,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        ""));
            }
            if (options.focusGuardian || options.stopFocusGuardian) {
                diagnostics.put(
                    "focus_guardian",
                    buildFocusGuardianStatus(
                        options,
                        options.connected && options.focusGuardian,
                        options.focusGuardianMode,
                        "",
                        "",
                        "",
                        "initial_report",
                        0,
                        0,
                        0L,
                        options.focusTargetPackage,
                        options.focusTargetActivity,
                        false,
                        Math.max(0L, options.focusGuardianDurationMs),
                        ""));
            }
            report.put("diagnostics", diagnostics);
        }
        return report;
    }

    private static void emitSyntheticVideoMetadata(Options options, boolean binaryPayload) throws Exception {
        String sessionId = binaryPayload
            ? "shell-helper-binary-" + System.currentTimeMillis()
            : "shell-helper-synthetic-" + System.currentTimeMillis();
        String payloadTransport = binaryPayload
            ? "adb_forwarded_tcp_binary"
            : "pending_binary";
        int sampleCount = binaryPayload ? options.syntheticVideoPackets : options.syntheticVideoSamples;
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "shell_helper.synthetic_encoded_h264");
        manifest.put("session_id", sessionId);
        manifest.put("source", "adb_shell_helper_synthetic");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", payloadTransport);
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "surface");
        manifest.put("width", SYNTHETIC_VIDEO_WIDTH);
        manifest.put("height", SYNTHETIC_VIDEO_HEIGHT);
        manifest.put("frame_rate_hz", SYNTHETIC_VIDEO_FRAME_RATE_HZ);
        manifest.put("bitrate_bps", 4000000);
        if (binaryPayload) {
            JSONObject endpoint = new JSONObject();
            endpoint.put("host", "127.0.0.1");
            endpoint.put("device_port", options.syntheticVideoBinaryPort);
            endpoint.put("framing", "rusty.xr.video_lab.binary_stream.v1");
            endpoint.put("magic", SYNTHETIC_BINARY_MAGIC);
            manifest.put("binary_endpoint", endpoint);
        }
        JSONObject manifestAck = sendBrokerCommand(
            options.host,
            options.port,
            "video_lab.register_encoded_stream_manifest",
            manifest);
        System.out.println(manifestAck.toString(2));

        for (int i = 0; i < sampleCount; i++) {
            JSONObject sample = new JSONObject();
            long sequenceId = System.currentTimeMillis() * 1000L + i;
            sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
            sample.put("stream_id", "shell_helper.synthetic_encoded_h264");
            sample.put("session_id", sessionId);
            sample.put("sequence_id", sequenceId);
            sample.put("source", "adb_shell_helper_synthetic");
            sample.put("transport", "metadata_only");
            sample.put("payload_transport", payloadTransport);
            sample.put("mime_type", "video/avc");
            sample.put("codec", "h264");
            sample.put("encoded_size_bytes", binaryPayload ? options.syntheticVideoPacketBytes : 0);
            sample.put("key_frame", i == 0);
            sample.put("pts_us", i * 33333L);
            sample.put("dts_us", i * 33333L);
            sample.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
            sample.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            JSONObject sampleAck = sendBrokerCommand(
                options.host,
                options.port,
                "video_lab.record_encoded_sample_metadata",
                sample);
            System.out.println(sampleAck.toString(2));
        }
    }

    private static void emitSyntheticVideoBinary(Options options) throws Exception {
        ServerSocket server = new ServerSocket(
            options.syntheticVideoBinaryPort,
            1,
            InetAddress.getByName("127.0.0.1"));
        try {
            server.setSoTimeout(10000);
            System.out.println(String.format(
                Locale.ROOT,
                "Synthetic binary video stream listening on 127.0.0.1:%d packets=%d packet_bytes=%d",
                options.syntheticVideoBinaryPort,
                options.syntheticVideoPackets,
                options.syntheticVideoPacketBytes));
            Socket client = server.accept();
            try {
                client.setTcpNoDelay(true);
                OutputStream output = client.getOutputStream();
                writeSyntheticBinaryStream(output, options);
                output.flush();
            } finally {
                client.close();
            }
        } finally {
            server.close();
        }
    }

    private static void writeSyntheticBinaryStream(OutputStream output, Options options) throws Exception {
        output.write(SYNTHETIC_BINARY_MAGIC.getBytes(StandardCharsets.US_ASCII));
        writeU32(output, SYNTHETIC_BINARY_SCHEMA_VERSION);
        writeU32(output, SYNTHETIC_BINARY_CODEC_H264);
        writeU32(output, SYNTHETIC_VIDEO_WIDTH);
        writeU32(output, SYNTHETIC_VIDEO_HEIGHT);
        writeU32(output, options.syntheticVideoPackets);
        writeU32(output, options.syntheticVideoPacketBytes);

        byte[] payload = new byte[options.syntheticVideoPacketBytes];
        for (int packetIndex = 0; packetIndex < options.syntheticVideoPackets; packetIndex++) {
            long ptsUs = packetIndex * 33333L;
            int flags = packetIndex == 0 ? 1 : 0;
            writeU64(output, ptsUs);
            writeU32(output, flags);
            writeU32(output, payload.length);
            for (int i = 0; i < payload.length; i++) {
                payload[i] = (byte) ((packetIndex + i) & 0xff);
            }
            output.write(payload);
        }
    }

    private static void emitMediaCodecSyntheticVideo(Options options) throws Exception {
        long encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        List<EncodedPacket> packets = encodeSyntheticSurfacePackets(options);
        long encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
        emitMediaCodecSyntheticMetadata(options, packets);
        StreamWriteStats writeStats = emitEncodedPacketStream(options, packets, "MediaCodec synthetic");
        emitVideoLabMetricSample(
            options,
            "shell_helper.mediacodec_synthetic_h264",
            "adb_shell_helper_mediacodec_synthetic_surface",
            options.encodedVideoWidth,
            options.encodedVideoHeight,
            packets,
            encodeStartElapsedNs,
            encodeEndElapsedNs,
            writeStats);
    }

    private static void emitScreenrecordVideo(Options options) throws Exception {
        long captureStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        List<EncodedPacket> packets = captureScreenrecordPackets(options);
        long captureEndElapsedNs = SystemClock.elapsedRealtimeNanos();
        emitScreenrecordMetadata(options, packets);
        StreamWriteStats writeStats = emitEncodedPacketStream(options, packets, "screenrecord display");
        emitVideoLabMetricSample(
            options,
            "shell_helper.screenrecord_h264",
            "adb_shell_helper_screenrecord_display",
            options.encodedVideoWidth,
            options.encodedVideoHeight,
            packets,
            captureStartElapsedNs,
            captureEndElapsedNs,
            writeStats);
    }

    private static List<EncodedPacket> captureScreenrecordPackets(Options options) throws Exception {
        int packetBytes = Math.max(SCREENRECORD_MIN_PACKET_BYTES, options.syntheticVideoPacketBytes);
        int maxBytes = Math.min(
            BINARY_STREAM_MAX_PACKET_BYTES,
            packetBytes * options.syntheticVideoPackets);
        byte[] h264 = captureScreenrecordBytes(options, maxBytes);
        if (h264.length == 0) {
            throw new IllegalStateException("screenrecord produced no H.264 bytes");
        }

        List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        int offset = 0;
        int packetIndex = 0;
        while (offset < h264.length && packetIndex < options.syntheticVideoPackets) {
            int size = Math.min(packetBytes, h264.length - offset);
            byte[] payload = new byte[size];
            System.arraycopy(h264, offset, payload, 0, size);
            int flags = packetIndex == 0 ? MediaCodec.BUFFER_FLAG_KEY_FRAME : 0;
            packets.add(new EncodedPacket(packetIndex * 33333L, flags, payload));
            offset += size;
            packetIndex++;
        }
        if (offset < h264.length) {
            throw new IllegalStateException("screenrecord capture exceeded bounded packet budget");
        }
        return packets;
    }

    private static byte[] captureScreenrecordBytes(Options options, int maxBytes) throws Exception {
        ProcessBuilder builder = new ProcessBuilder(
            "screenrecord",
            "--output-format=h264",
            "--size",
            options.encodedVideoWidth + "x" + options.encodedVideoHeight,
            "--bit-rate",
            Integer.toString(options.encodedVideoBitrateBps),
            "--time-limit",
            Integer.toString(options.screenrecordTimeLimitSeconds),
            "-");
        builder.redirectError(ProcessBuilder.Redirect.to(new File("/dev/null")));
        java.lang.Process process = builder.start();
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        byte[] buffer = new byte[8192];
        try {
            InputStream input = process.getInputStream();
            while (true) {
                int read = input.read(buffer);
                if (read < 0) {
                    break;
                }
                if (output.size() + read > maxBytes) {
                    process.destroy();
                    throw new IllegalStateException("screenrecord output exceeded " + maxBytes + " bytes");
                }
                output.write(buffer, 0, read);
            }
            int exitCode = process.waitFor();
            if (exitCode != 0) {
                throw new IllegalStateException("screenrecord exited with code " + exitCode);
            }
            return output.toByteArray();
        } finally {
            process.destroy();
        }
    }

    private static List<EncodedPacket> encodeSyntheticSurfacePackets(Options options) throws Exception {
        MediaCodec encoder = MediaCodec.createEncoderByType("video/avc");
        Surface surface = null;
        try {
            MediaFormat format = MediaFormat.createVideoFormat(
                "video/avc",
                options.encodedVideoWidth,
                options.encodedVideoHeight);
            format.setInteger(
                MediaFormat.KEY_COLOR_FORMAT,
                MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
            format.setInteger(MediaFormat.KEY_BIT_RATE, options.encodedVideoBitrateBps);
            format.setInteger(MediaFormat.KEY_FRAME_RATE, SYNTHETIC_VIDEO_FRAME_RATE_HZ);
            format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);
            encoder.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            surface = encoder.createInputSurface();
            encoder.start();

            List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
            for (int frame = 0; frame < options.encodedVideoFrames; frame++) {
                drawSyntheticEncoderFrame(surface, frame, options);
                drainEncoder(encoder, packets, false);
                Thread.sleep(1000 / SYNTHETIC_VIDEO_FRAME_RATE_HZ);
            }
            encoder.signalEndOfInputStream();
            drainEncoder(encoder, packets, true);
            if (packets.size() == 0) {
                throw new IllegalStateException("MediaCodec produced no encoded packets");
            }
            return packets;
        } finally {
            if (surface != null) {
                surface.release();
            }
            try {
                encoder.stop();
            } catch (Exception ignored) {
            }
            encoder.release();
        }
    }

    private static void drawSyntheticEncoderFrame(Surface surface, int frame, Options options) throws Exception {
        Canvas canvas = surface.lockCanvas(null);
        try {
            Paint paint = new Paint();
            int base = (frame * 31) & 0xff;
            canvas.drawColor(Color.rgb(base, (base + 80) & 0xff, (base + 160) & 0xff));

            int width = options.encodedVideoWidth;
            int height = options.encodedVideoHeight;
            int barWidth = Math.max(16, width / 8);
            int x = (frame * Math.max(1, width / Math.max(1, options.encodedVideoFrames))) % width;
            paint.setColor(Color.WHITE);
            canvas.drawRect(new Rect(x, 0, Math.min(width, x + barWidth), height), paint);
            paint.setColor(Color.rgb(20, 20, 20));
            canvas.drawRect(new Rect(0, height - 56, width, height), paint);
            paint.setColor(Color.GREEN);
            int markerWidth = Math.max(8, width / Math.max(4, options.encodedVideoFrames));
            canvas.drawRect(new Rect(24, height - 42, 24 + markerWidth + frame * 4, height - 18), paint);
        } finally {
            surface.unlockCanvasAndPost(canvas);
        }
    }

    private static void drainEncoder(
            MediaCodec encoder,
            List<EncodedPacket> packets,
            boolean endOfStream) throws Exception {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        int emptyPolls = 0;
        while (true) {
            int status = encoder.dequeueOutputBuffer(info, 10000);
            if (status == MediaCodec.INFO_TRY_AGAIN_LATER) {
                if (!endOfStream || emptyPolls++ > 50) {
                    break;
                }
                continue;
            }
            if (status == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                continue;
            }
            if (status < 0) {
                continue;
            }

            ByteBuffer outputBuffer = encoder.getOutputBuffer(status);
            if (outputBuffer != null && info.size > 0) {
                if (info.size > BINARY_STREAM_MAX_PACKET_BYTES) {
                    throw new IllegalStateException("Encoded packet too large: " + info.size);
                }
                byte[] payload = new byte[info.size];
                outputBuffer.position(info.offset);
                outputBuffer.limit(info.offset + info.size);
                outputBuffer.get(payload);
                packets.add(new EncodedPacket(info.presentationTimeUs, info.flags, payload));
            }
            boolean reachedEos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
            encoder.releaseOutputBuffer(status, false);
            if (reachedEos) {
                break;
            }
        }
    }

    private static void emitMediaCodecSyntheticMetadata(
            Options options,
            List<EncodedPacket> packets) throws Exception {
        String sessionId = "shell-helper-mediacodec-" + System.currentTimeMillis();
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "shell_helper.mediacodec_synthetic_h264");
        manifest.put("session_id", sessionId);
        manifest.put("source", "adb_shell_helper_mediacodec_synthetic_surface");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "adb_forwarded_tcp_binary");
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "surface");
        manifest.put("width", options.encodedVideoWidth);
        manifest.put("height", options.encodedVideoHeight);
        manifest.put("frame_rate_hz", SYNTHETIC_VIDEO_FRAME_RATE_HZ);
        manifest.put("bitrate_bps", options.encodedVideoBitrateBps);
        manifest.put("source_kind", "synthetic_surface_mediacodec");
        JSONObject endpoint = new JSONObject();
        endpoint.put("host", "127.0.0.1");
        endpoint.put("device_port", options.syntheticVideoBinaryPort);
        endpoint.put("framing", "rusty.xr.video_lab.binary_stream.v1");
        endpoint.put("magic", SYNTHETIC_BINARY_MAGIC);
        manifest.put("binary_endpoint", endpoint);
        JSONObject manifestAck = sendBrokerCommand(
            options.host,
            options.port,
            "video_lab.register_encoded_stream_manifest",
            manifest);
        System.out.println(manifestAck.toString(2));

        for (int i = 0; i < packets.size(); i++) {
            EncodedPacket packet = packets.get(i);
            JSONObject sample = new JSONObject();
            long sequenceId = System.currentTimeMillis() * 1000L + i;
            sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
            sample.put("stream_id", "shell_helper.mediacodec_synthetic_h264");
            sample.put("session_id", sessionId);
            sample.put("sequence_id", sequenceId);
            sample.put("source", "adb_shell_helper_mediacodec_synthetic_surface");
            sample.put("transport", "metadata_only");
            sample.put("payload_transport", "adb_forwarded_tcp_binary");
            sample.put("mime_type", "video/avc");
            sample.put("codec", "h264");
            sample.put("encoded_size_bytes", packet.payload.length);
            sample.put("key_frame", (packet.flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0);
            sample.put("codec_config", (packet.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0);
            sample.put("pts_us", packet.ptsUs);
            sample.put("dts_us", packet.ptsUs);
            sample.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
            sample.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            JSONObject sampleAck = sendBrokerCommand(
                options.host,
                options.port,
                "video_lab.record_encoded_sample_metadata",
                sample);
            System.out.println(sampleAck.toString(2));
        }
    }

    private static void emitScreenrecordMetadata(
            Options options,
            List<EncodedPacket> packets) throws Exception {
        String sessionId = "shell-helper-screenrecord-" + System.currentTimeMillis();
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "shell_helper.screenrecord_h264");
        manifest.put("session_id", sessionId);
        manifest.put("source", "adb_shell_helper_screenrecord_display");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "adb_forwarded_tcp_binary");
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "surface");
        manifest.put("width", options.encodedVideoWidth);
        manifest.put("height", options.encodedVideoHeight);
        manifest.put("frame_rate_hz", SYNTHETIC_VIDEO_FRAME_RATE_HZ);
        manifest.put("bitrate_bps", options.encodedVideoBitrateBps);
        manifest.put("source_kind", "shell_screenrecord_display");
        manifest.put("screenrecord_time_limit_seconds", options.screenrecordTimeLimitSeconds);
        JSONObject endpoint = new JSONObject();
        endpoint.put("host", "127.0.0.1");
        endpoint.put("device_port", options.syntheticVideoBinaryPort);
        endpoint.put("framing", "rusty.xr.video_lab.binary_stream.v1");
        endpoint.put("magic", SYNTHETIC_BINARY_MAGIC);
        manifest.put("binary_endpoint", endpoint);
        JSONObject manifestAck = sendBrokerCommand(
            options.host,
            options.port,
            "video_lab.register_encoded_stream_manifest",
            manifest);
        System.out.println(manifestAck.toString(2));

        for (int i = 0; i < packets.size(); i++) {
            EncodedPacket packet = packets.get(i);
            JSONObject sample = new JSONObject();
            long sequenceId = System.currentTimeMillis() * 1000L + i;
            sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
            sample.put("stream_id", "shell_helper.screenrecord_h264");
            sample.put("session_id", sessionId);
            sample.put("sequence_id", sequenceId);
            sample.put("source", "adb_shell_helper_screenrecord_display");
            sample.put("transport", "metadata_only");
            sample.put("payload_transport", "adb_forwarded_tcp_binary");
            sample.put("mime_type", "video/avc");
            sample.put("codec", "h264");
            sample.put("encoded_size_bytes", packet.payload.length);
            sample.put("key_frame", (packet.flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0);
            sample.put("codec_config", false);
            sample.put("pts_us", packet.ptsUs);
            sample.put("dts_us", packet.ptsUs);
            sample.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
            sample.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            JSONObject sampleAck = sendBrokerCommand(
                options.host,
                options.port,
                "video_lab.record_encoded_sample_metadata",
                sample);
            System.out.println(sampleAck.toString(2));
        }
    }

    private static StreamWriteStats emitEncodedPacketStream(
            Options options,
            List<EncodedPacket> packets,
            String label) throws Exception {
        ServerSocket server = new ServerSocket(
            options.syntheticVideoBinaryPort,
            1,
            InetAddress.getByName("127.0.0.1"));
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        try {
            server.setSoTimeout(10000);
            System.out.println(String.format(
                Locale.ROOT,
                "%s binary video stream listening on 127.0.0.1:%d packets=%d",
                label,
                options.syntheticVideoBinaryPort,
                packets.size()));
            Socket client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            try {
                client.setTcpNoDelay(true);
                OutputStream output = client.getOutputStream();
                writeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                writeEncodedPacketStream(output, options, packets);
                output.flush();
                writeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            } finally {
                client.close();
            }
        } finally {
            server.close();
        }
        return new StreamWriteStats(
            listenStartElapsedNs,
            acceptElapsedNs,
            writeStartElapsedNs,
            writeEndElapsedNs);
    }

    private static void writeEncodedPacketStream(
            OutputStream output,
            Options options,
            List<EncodedPacket> packets) throws Exception {
        output.write(SYNTHETIC_BINARY_MAGIC.getBytes(StandardCharsets.US_ASCII));
        writeU32(output, SYNTHETIC_BINARY_SCHEMA_VERSION);
        writeU32(output, SYNTHETIC_BINARY_CODEC_H264);
        writeU32(output, options.encodedVideoWidth);
        writeU32(output, options.encodedVideoHeight);
        writeU32(output, packets.size());
        writeU32(output, 0);

        for (EncodedPacket packet : packets) {
            writeU64(output, packet.ptsUs);
            writeU32(output, packet.flags);
            writeU32(output, packet.payload.length);
            output.write(packet.payload);
        }
    }

    private static void emitVideoLabMetricSample(
            Options options,
            String streamId,
            String source,
            int width,
            int height,
            List<EncodedPacket> packets,
            long encodeStartElapsedNs,
            long encodeEndElapsedNs,
            StreamWriteStats writeStats) throws Exception {
        long totalPayloadBytes = 0L;
        for (EncodedPacket packet : packets) {
            totalPayloadBytes += packet.payload.length;
        }

        JSONObject metric = new JSONObject();
        metric.put("schema", "rusty.xr.video_lab.metric_sample.v1");
        metric.put("stream_id", streamId);
        metric.put("source", source);
        metric.put("transport", "metadata_only");
        metric.put("payload_transport", "adb_forwarded_tcp_binary");
        metric.put("codec", "h264");
        metric.put("sequence_id", System.currentTimeMillis() * 1000L);
        metric.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
        metric.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        metric.put("helper_encode_start_elapsed_ns", encodeStartElapsedNs);
        metric.put("helper_encode_end_elapsed_ns", encodeEndElapsedNs);
        metric.put("helper_encode_duration_ns", Math.max(0L, encodeEndElapsedNs - encodeStartElapsedNs));
        metric.put("helper_binary_listen_start_elapsed_ns", writeStats.listenStartElapsedNs);
        metric.put("helper_binary_accept_elapsed_ns", writeStats.acceptElapsedNs);
        metric.put("helper_binary_write_start_elapsed_ns", writeStats.writeStartElapsedNs);
        metric.put("helper_binary_write_end_elapsed_ns", writeStats.writeEndElapsedNs);
        metric.put("helper_binary_write_duration_ns", Math.max(0L, writeStats.writeEndElapsedNs - writeStats.writeStartElapsedNs));
        metric.put("packet_count", packets.size());
        metric.put("payload_size_bytes", totalPayloadBytes);
        metric.put("dropped_frames", 0);
        metric.put("stale_frames", 0);
        metric.put("queue_depth", 0);
        metric.put("width", width);
        metric.put("height", height);
        JSONObject metricAck = sendBrokerCommand(
            options.host,
            options.port,
            "video_lab.record_metric_sample",
            metric);
        System.out.println(metricAck.toString(2));
    }

    private static JSONObject buildCodecProbe() throws Exception {
        JSONObject probe = new JSONObject();
        probe.put("schema", "rusty.xr.shell_helper.codec_probe.v1");
        probe.put("queried_mime_types", new JSONArray()
            .put("video/avc")
            .put("video/hevc")
            .put("video/av01"));

        JSONArray codecs = new JSONArray();
        int encoderCount = 0;
        int decoderCount = 0;
        int surfaceCapableCount = 0;
        MediaCodecInfo[] infos = new MediaCodecList(MediaCodecList.ALL_CODECS).getCodecInfos();
        for (MediaCodecInfo info : infos) {
            String[] supportedTypes = info.getSupportedTypes();
            for (String type : supportedTypes) {
                if (!isVideoCodecTypeOfInterest(type)) {
                    continue;
                }

                JSONObject codec = new JSONObject();
                codec.put("name", info.getName());
                codec.put("canonical_name", info.getCanonicalName());
                codec.put("mime_type", type);
                codec.put("encoder", info.isEncoder());
                if (info.isEncoder()) {
                    encoderCount++;
                } else {
                    decoderCount++;
                }

                try {
                    MediaCodecInfo.CodecCapabilities capabilities = info.getCapabilitiesForType(type);
                    boolean surfaceCapable = containsColorFormat(
                        capabilities.colorFormats,
                        MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
                    codec.put("surface_color_format", surfaceCapable);
                    if (surfaceCapable) {
                        surfaceCapableCount++;
                    }
                    codec.put("color_formats", intArrayJson(capabilities.colorFormats, 16));
                    codec.put("profile_levels", profileLevelsJson(capabilities.profileLevels, 12));
                } catch (Exception ex) {
                    codec.put("capability_error", ex.getClass().getSimpleName() + ": " + ex.getMessage());
                }
                codecs.put(codec);
            }
        }

        probe.put("codec_count", codecs.length());
        probe.put("encoder_count", encoderCount);
        probe.put("decoder_count", decoderCount);
        probe.put("surface_capable_count", surfaceCapableCount);
        probe.put("codecs", codecs);
        return probe;
    }

    private static boolean isVideoCodecTypeOfInterest(String type) {
        return "video/avc".equals(type) ||
            "video/hevc".equals(type) ||
            "video/av01".equals(type);
    }

    private static JSONObject buildCameraOpenProbe(
            String requestedCameraId,
            boolean persistCameraFrame,
            String cameraFrameOutputDir,
            int cameraFrameJpegQuality) throws Exception {
        JSONObject probe = new JSONObject();
        probe.put("schema", "rusty.xr.shell_helper.camera_open_probe.v1");
        probe.put("source", "Camera2 CameraManager from adb shell app_process");
        probe.put("captured_time_unix_ms", System.currentTimeMillis());
        probe.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        probe.put("max_camera_ids", CAMERA_OPEN_PROBE_MAX_CAMERA_IDS);
        probe.put("open_timeout_ms", CAMERA_OPEN_PROBE_OPEN_TIMEOUT_MS);
        probe.put("session_timeout_ms", CAMERA_OPEN_PROBE_SESSION_TIMEOUT_MS);
        probe.put("capture_timeout_ms", CAMERA_OPEN_PROBE_CAPTURE_TIMEOUT_MS);
        probe.put("capture_format", "YUV_420_888");
        probe.put("capture_max_dimension", CAMERA_OPEN_PROBE_MAX_DIMENSION);
        probe.put("persist_camera_frame", persistCameraFrame);
        if (persistCameraFrame) {
            probe.put("camera_frame_output_dir", cameraFrameOutputDir);
            probe.put("camera_frame_jpeg_quality", cameraFrameJpegQuality);
        }
        if (requestedCameraId != null && requestedCameraId.trim().length() > 0) {
            probe.put("requested_camera_id", requestedCameraId.trim());
        }

        HandlerThread thread = new HandlerThread("RustyXrCameraOpenProbe");
        thread.start();
        try {
            CameraManagerCreateResult managerCreateResult = createCameraManagerReflectively();
            CameraManager manager = managerCreateResult.manager;
            probe.put("manager_state", "created");
            probe.put("camera_manager_constructor", managerCreateResult.constructorSignature);
            probe.put("camera_manager_constructor_strategy", managerCreateResult.strategy);
            probe.put("camera_manager_constructors", managerCreateResult.constructorSignatures);
            String[] cameraIds = manager.getCameraIdList();
            probe.put("camera_id_count", cameraIds.length);
            probe.put("camera_ids", stringArrayJson(cameraIds, CAMERA_OPEN_PROBE_MAX_CAMERA_IDS));
            String[] targetCameraIds = targetCameraIds(cameraIds, requestedCameraId);
            probe.put("target_camera_ids", stringArrayJson(targetCameraIds, CAMERA_OPEN_PROBE_MAX_CAMERA_IDS));

            Handler handler = new Handler(thread.getLooper());
            JSONArray attempts = new JSONArray();
            int openSuccessCount = 0;
            int captureSuccessCount = 0;
            int persistedFrameCount = 0;
            for (int i = 0; i < targetCameraIds.length && i < CAMERA_OPEN_PROBE_MAX_CAMERA_IDS; i++) {
                JSONObject attempt = probeSingleCameraOpenCapture(
                    manager,
                    targetCameraIds[i],
                    handler,
                    persistCameraFrame,
                    cameraFrameOutputDir,
                    cameraFrameJpegQuality);
                attempts.put(attempt);
                if (attempt.optBoolean("open_succeeded", false)) {
                    openSuccessCount++;
                }
                if (attempt.optBoolean("capture_succeeded", false)) {
                    captureSuccessCount++;
                }
                if (attempt.has("persisted_frame")) {
                    persistedFrameCount++;
                }
            }
            probe.put("attempted_count", attempts.length());
            probe.put("open_success_count", openSuccessCount);
            probe.put("capture_success_count", captureSuccessCount);
            if (persistCameraFrame) {
                probe.put("persisted_frame_count", persistedFrameCount);
            }
            probe.put("attempts", attempts);
        } catch (Exception ex) {
            probe.put("manager_state", "failed");
            probe.put("error", exceptionSummary(ex));
        } finally {
            thread.quitSafely();
            thread.join(1000);
        }
        return probe;
    }

    private static CameraManagerCreateResult createCameraManagerReflectively() throws Exception {
        Constructor<?>[] constructors = CameraManager.class.getDeclaredConstructors();
        JSONArray constructorSignatures = new JSONArray();
        Exception lastError = null;
        for (int i = 0; i < constructors.length; i++) {
            Constructor<?> constructor = constructors[i];
            constructorSignatures.put(constructorSignature(constructor));
            Object[] args = cameraManagerConstructorArgs(constructor.getParameterTypes());
            if (args == null) {
                continue;
            }
            try {
                constructor.setAccessible(true);
                Object instance = constructor.newInstance(args);
                if (instance instanceof CameraManager) {
                    return new CameraManagerCreateResult(
                        (CameraManager) instance,
                        constructorSignature(constructor),
                        args.length == 0 ? "no_arg_reflection" : "shell_context_reflection",
                        constructorSignatures);
                }
            } catch (Exception ex) {
                lastError = ex;
            }
        }

        String message = "No supported CameraManager constructor";
        if (lastError != null) {
            message += "; last_error=" + exceptionSummary(lastError);
        }
        message += "; constructors=" + constructorSignatures.toString();
        throw new NoSuchMethodException(message);
    }

    private static Object[] cameraManagerConstructorArgs(Class<?>[] parameterTypes) {
        if (parameterTypes.length == 0) {
            return new Object[0];
        }
        if (parameterTypes.length == 1 && Context.class.isAssignableFrom(parameterTypes[0])) {
            return new Object[] { new ShellContext() };
        }
        return null;
    }

    private static String constructorSignature(Constructor<?> constructor) {
        StringBuilder builder = new StringBuilder();
        builder.append("CameraManager(");
        Class<?>[] parameterTypes = constructor.getParameterTypes();
        for (int i = 0; i < parameterTypes.length; i++) {
            if (i > 0) {
                builder.append(",");
            }
            builder.append(parameterTypes[i].getName());
        }
        builder.append(")");
        return builder.toString();
    }

    private static String[] targetCameraIds(String[] cameraIds, String requestedCameraId) {
        if (requestedCameraId != null && requestedCameraId.trim().length() > 0) {
            return new String[] { requestedCameraId.trim() };
        }
        int count = Math.min(cameraIds.length, CAMERA_OPEN_PROBE_MAX_CAMERA_IDS);
        String[] targets = new String[count];
        System.arraycopy(cameraIds, 0, targets, 0, count);
        return targets;
    }

    private static JSONObject probeSingleCameraOpenCapture(
            CameraManager manager,
            String cameraId,
            Handler handler,
            boolean persistCameraFrame,
            String cameraFrameOutputDir,
            int cameraFrameJpegQuality) throws Exception {
        JSONObject result = new JSONObject();
        result.put("camera_id", cameraId);
        result.put("open_succeeded", false);
        result.put("capture_succeeded", false);
        result.put("open_state", "not_started");
        result.put("capture_state", "not_started");

        final CountDownLatch openLatch = new CountDownLatch(1);
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final String[] openErrorRef = new String[1];
        final int[] openErrorCodeRef = new int[] { Integer.MIN_VALUE };
        long openStartNs = SystemClock.elapsedRealtimeNanos();
        try {
            manager.openCamera(cameraId, new CameraDevice.StateCallback() {
                @Override
                public void onOpened(CameraDevice device) {
                    deviceRef[0] = device;
                    openLatch.countDown();
                }

                @Override
                public void onDisconnected(CameraDevice device) {
                    openErrorRef[0] = "disconnected";
                    if (device != null) {
                        device.close();
                    }
                    openLatch.countDown();
                }

                @Override
                public void onError(CameraDevice device, int error) {
                    openErrorCodeRef[0] = error;
                    openErrorRef[0] = "camera_error_" + error;
                    if (device != null) {
                        device.close();
                    }
                    openLatch.countDown();
                }
            }, handler);
        } catch (SecurityException ex) {
            result.put("open_state", "security_exception");
            result.put("open_error", exceptionSummary(ex));
            return result;
        } catch (CameraAccessException ex) {
            result.put("open_state", "camera_access_exception");
            result.put("open_error", exceptionSummary(ex));
            return result;
        } catch (RuntimeException ex) {
            result.put("open_state", "runtime_exception");
            result.put("open_error", exceptionSummary(ex));
            return result;
        }

        boolean openFinished = openLatch.await(CAMERA_OPEN_PROBE_OPEN_TIMEOUT_MS, TimeUnit.MILLISECONDS);
        result.put("open_elapsed_ms", nanosToMillis(SystemClock.elapsedRealtimeNanos() - openStartNs));
        if (!openFinished) {
            result.put("open_state", "timeout");
            closeQuietly(deviceRef[0]);
            return result;
        }
        if (deviceRef[0] == null) {
            result.put("open_state", openErrorRef[0] != null ? openErrorRef[0] : "failed");
            if (openErrorCodeRef[0] != Integer.MIN_VALUE) {
                result.put("open_error_code", openErrorCodeRef[0]);
            }
            return result;
        }

        result.put("open_succeeded", true);
        result.put("open_state", "opened");

        ImageReader reader = null;
        CameraCaptureSession session = null;
        try {
            Size size = chooseYuvProbeSize(manager, cameraId);
            if (size == null) {
                result.put("capture_state", "no_yuv_420_888_output_size");
                return result;
            }
            result.put("capture_size", sizeJson(size));
            reader = ImageReader.newInstance(size.getWidth(), size.getHeight(), ImageFormat.YUV_420_888, 2);
            final CountDownLatch imageLatch = new CountDownLatch(1);
            final int[] imageCount = new int[] { 0 };
            final int[] imageWidth = new int[] { 0 };
            final int[] imageHeight = new int[] { 0 };
            final long[] captureStartNs = new long[] { 0L };
            final long[] firstImageElapsedNs = new long[] { 0L };
            final String[] imageErrorRef = new String[1];
            final JSONObject[] persistedFrameRef = new JSONObject[1];
            final String[] persistedFrameErrorRef = new String[1];
            reader.setOnImageAvailableListener(new ImageReader.OnImageAvailableListener() {
                @Override
                public void onImageAvailable(ImageReader imageReader) {
                    Image image = null;
                    try {
                        image = imageReader.acquireNextImage();
                        if (image != null) {
                            imageCount[0]++;
                            imageWidth[0] = image.getWidth();
                            imageHeight[0] = image.getHeight();
                            firstImageElapsedNs[0] = SystemClock.elapsedRealtimeNanos() - captureStartNs[0];
                            if (persistCameraFrame) {
                                try {
                                    persistedFrameRef[0] = persistCapturedYuvImage(
                                        image,
                                        cameraId,
                                        cameraFrameOutputDir,
                                        cameraFrameJpegQuality);
                                } catch (Exception ex) {
                                    persistedFrameErrorRef[0] = exceptionSummary(ex);
                                }
                            }
                        }
                    } catch (RuntimeException ex) {
                        imageErrorRef[0] = exceptionSummary(ex);
                    } finally {
                        if (image != null) {
                            image.close();
                        }
                        imageLatch.countDown();
                    }
                }
            }, handler);

            final CountDownLatch sessionLatch = new CountDownLatch(1);
            final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
            final String[] sessionErrorRef = new String[1];
            deviceRef[0].createCaptureSession(
                Arrays.asList(reader.getSurface()),
                new CameraCaptureSession.StateCallback() {
                    @Override
                    public void onConfigured(CameraCaptureSession configuredSession) {
                        sessionRef[0] = configuredSession;
                        sessionLatch.countDown();
                    }

                    @Override
                    public void onConfigureFailed(CameraCaptureSession failedSession) {
                        sessionErrorRef[0] = "configure_failed";
                        sessionLatch.countDown();
                    }
                },
                handler);

            boolean sessionFinished = sessionLatch.await(CAMERA_OPEN_PROBE_SESSION_TIMEOUT_MS, TimeUnit.MILLISECONDS);
            if (!sessionFinished) {
                result.put("capture_state", "session_timeout");
                return result;
            }
            session = sessionRef[0];
            if (session == null) {
                result.put("capture_state", sessionErrorRef[0] != null ? sessionErrorRef[0] : "session_failed");
                return result;
            }

            Surface surface = reader.getSurface();
            CaptureRequest.Builder builder = deviceRef[0].createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
            builder.addTarget(surface);
            captureStartNs[0] = SystemClock.elapsedRealtimeNanos();
            int sequenceId = session.capture(builder.build(), null, handler);
            result.put("capture_sequence_id", sequenceId);
            boolean imageFinished = imageLatch.await(CAMERA_OPEN_PROBE_CAPTURE_TIMEOUT_MS, TimeUnit.MILLISECONDS);
            if (!imageFinished) {
                result.put("capture_state", "image_timeout");
                return result;
            }
            if (imageCount[0] <= 0) {
                result.put("capture_state", "image_unavailable");
                if (imageErrorRef[0] != null) {
                    result.put("capture_error", imageErrorRef[0]);
                }
                return result;
            }
            result.put("capture_succeeded", true);
            result.put("capture_state", "captured");
            result.put("captured_image_count", imageCount[0]);
            result.put("captured_width", imageWidth[0]);
            result.put("captured_height", imageHeight[0]);
            result.put("first_image_elapsed_ms", nanosToMillis(firstImageElapsedNs[0]));
            if (persistedFrameRef[0] != null) {
                result.put("persisted_frame", persistedFrameRef[0]);
            }
            if (persistedFrameErrorRef[0] != null) {
                result.put("persisted_frame_error", persistedFrameErrorRef[0]);
            }
        } catch (CameraAccessException ex) {
            result.put("capture_state", "camera_access_exception");
            result.put("capture_error", exceptionSummary(ex));
        } catch (SecurityException ex) {
            result.put("capture_state", "security_exception");
            result.put("capture_error", exceptionSummary(ex));
        } catch (RuntimeException ex) {
            result.put("capture_state", "runtime_exception");
            result.put("capture_error", exceptionSummary(ex));
        } finally {
            if (session != null) {
                session.close();
            }
            if (reader != null) {
                reader.close();
            }
            closeQuietly(deviceRef[0]);
        }
        return result;
    }

    private static JSONObject persistCapturedYuvImage(
            Image image,
            String cameraId,
            String cameraFrameOutputDir,
            int cameraFrameJpegQuality) throws Exception {
        if (image.getFormat() != ImageFormat.YUV_420_888) {
            throw new IllegalArgumentException("Expected YUV_420_888 image, got format=" + image.getFormat());
        }
        File outputDir = new File(cameraFrameOutputDir);
        if (!outputDir.exists() && !outputDir.mkdirs()) {
            throw new IllegalStateException("Could not create camera frame output dir: " + outputDir.getAbsolutePath());
        }
        if (!outputDir.isDirectory()) {
            throw new IllegalStateException("Camera frame output path is not a directory: " + outputDir.getAbsolutePath());
        }

        String baseName = "camera-" + safeFileToken(cameraId) +
            "-" + System.currentTimeMillis() +
            "-" + SystemClock.elapsedRealtimeNanos();
        File nv21File = new File(outputDir, baseName + ".nv21");
        File jpegFile = new File(outputDir, baseName + ".jpg");
        File metadataFile = new File(outputDir, baseName + ".json");

        byte[] nv21 = yuv420ImageToNv21(image);
        writeBytes(nv21File, nv21);

        ByteArrayOutputStream jpegBytes = new ByteArrayOutputStream();
        YuvImage yuvImage = new YuvImage(nv21, ImageFormat.NV21, image.getWidth(), image.getHeight(), null);
        boolean jpegWritten = yuvImage.compressToJpeg(
            new Rect(0, 0, image.getWidth(), image.getHeight()),
            cameraFrameJpegQuality,
            jpegBytes);
        if (!jpegWritten) {
            throw new IllegalStateException("YuvImage.compressToJpeg returned false");
        }
        writeBytes(jpegFile, jpegBytes.toByteArray());

        JSONObject record = new JSONObject();
        record.put("schema", "rusty.xr.shell_helper.camera_yuv_frame_capture.v1");
        record.put("source", "Camera2 YUV_420_888 one-frame capture from adb shell app_process");
        record.put("camera_id", cameraId);
        record.put("captured_time_unix_ms", System.currentTimeMillis());
        record.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        record.put("image_timestamp_ns", image.getTimestamp());
        record.put("width", image.getWidth());
        record.put("height", image.getHeight());
        record.put("raw_format", "YUV_420_888");
        record.put("raw_layout", "nv21_packed_from_yuv_420_888_planes");
        record.put("nv21_path", nv21File.getAbsolutePath());
        record.put("nv21_bytes", nv21File.length());
        record.put("jpeg_preview_path", jpegFile.getAbsolutePath());
        record.put("jpeg_preview_bytes", jpegFile.length());
        record.put("jpeg_quality", cameraFrameJpegQuality);
        record.put("planes", imagePlaneMetadataJson(image));
        record.put("metadata_path", metadataFile.getAbsolutePath());
        writeText(metadataFile, record.toString(2) + "\n");
        return record;
    }

    private static byte[] yuv420ImageToNv21(Image image) {
        int width = image.getWidth();
        int height = image.getHeight();
        Image.Plane[] planes = image.getPlanes();
        if (planes == null || planes.length < 3) {
            throw new IllegalArgumentException("YUV_420_888 image did not expose three planes");
        }
        byte[] nv21 = new byte[width * height * 3 / 2];
        copyPlaneToPackedOutput(planes[0], width, height, nv21, 0, 1);
        copyPlaneToPackedOutput(planes[2], width / 2, height / 2, nv21, width * height, 2);
        copyPlaneToPackedOutput(planes[1], width / 2, height / 2, nv21, width * height + 1, 2);
        return nv21;
    }

    private static void copyPlaneToPackedOutput(
            Image.Plane plane,
            int width,
            int height,
            byte[] output,
            int outputOffset,
            int outputPixelStride) {
        ByteBuffer buffer = plane.getBuffer().duplicate();
        int rowStride = plane.getRowStride();
        int pixelStride = plane.getPixelStride();
        int outputRowStride = width * outputPixelStride;
        int limit = buffer.limit();
        for (int row = 0; row < height; row++) {
            int inputRowOffset = row * rowStride;
            int outputRowOffset = outputOffset + row * outputRowStride;
            for (int col = 0; col < width; col++) {
                int inputIndex = inputRowOffset + col * pixelStride;
                int outputIndex = outputRowOffset + col * outputPixelStride;
                if (inputIndex < limit && outputIndex < output.length) {
                    output[outputIndex] = buffer.get(inputIndex);
                }
            }
        }
    }

    private static JSONArray imagePlaneMetadataJson(Image image) throws Exception {
        JSONArray planes = new JSONArray();
        Image.Plane[] imagePlanes = image.getPlanes();
        for (int i = 0; i < imagePlanes.length; i++) {
            Image.Plane plane = imagePlanes[i];
            JSONObject item = new JSONObject();
            item.put("index", i);
            item.put("row_stride", plane.getRowStride());
            item.put("pixel_stride", plane.getPixelStride());
            item.put("buffer_remaining", plane.getBuffer().remaining());
            planes.put(item);
        }
        return planes;
    }

    private static String safeFileToken(String value) {
        String input = value != null ? value : "unknown";
        StringBuilder builder = new StringBuilder();
        for (int i = 0; i < input.length(); i++) {
            char ch = input.charAt(i);
            if ((ch >= 'A' && ch <= 'Z') ||
                    (ch >= 'a' && ch <= 'z') ||
                    (ch >= '0' && ch <= '9') ||
                    ch == '-' ||
                    ch == '_') {
                builder.append(ch);
            } else {
                builder.append('_');
            }
        }
        return builder.length() > 0 ? builder.toString() : "unknown";
    }

    private static void writeBytes(File file, byte[] bytes) throws Exception {
        FileOutputStream stream = new FileOutputStream(file, false);
        try {
            stream.write(bytes);
            stream.flush();
        } finally {
            stream.close();
        }
    }

    private static void writeText(File file, String text) throws Exception {
        writeBytes(file, text.getBytes(StandardCharsets.UTF_8));
    }

    private static Size chooseYuvProbeSize(CameraManager manager, String cameraId) throws CameraAccessException {
        CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
        StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
        if (map == null) {
            return null;
        }
        Size[] sizes = map.getOutputSizes(ImageFormat.YUV_420_888);
        if (sizes == null || sizes.length == 0) {
            return null;
        }

        Size bestWithinLimit = null;
        Size smallest = null;
        for (Size size : sizes) {
            if (smallest == null || area(size) < area(smallest)) {
                smallest = size;
            }
            if (size.getWidth() <= CAMERA_OPEN_PROBE_MAX_DIMENSION &&
                    size.getHeight() <= CAMERA_OPEN_PROBE_MAX_DIMENSION &&
                    (bestWithinLimit == null || area(size) < area(bestWithinLimit))) {
                bestWithinLimit = size;
            }
        }
        return bestWithinLimit != null ? bestWithinLimit : smallest;
    }

    private static long area(Size size) {
        return (long) size.getWidth() * (long) size.getHeight();
    }

    private static JSONObject sizeJson(Size size) throws Exception {
        JSONObject json = new JSONObject();
        json.put("width", size.getWidth());
        json.put("height", size.getHeight());
        return json;
    }

    private static long nanosToMillis(long nanos) {
        return TimeUnit.NANOSECONDS.toMillis(Math.max(0L, nanos));
    }

    private static void closeQuietly(CameraDevice device) {
        if (device != null) {
            try {
                device.close();
            } catch (RuntimeException ignored) {
            }
        }
    }

    private static String exceptionSummary(Throwable ex) {
        String message = ex.getMessage();
        String base = (message == null || message.length() == 0)
            ? ex.getClass().getSimpleName()
            : ex.getClass().getSimpleName() + ": " + message;
        Throwable cause = ex.getCause();
        if (cause != null && cause != ex) {
            return base + "; cause=" + exceptionSummary(cause);
        }
        return base;
    }

    private static JSONObject buildCameraProbe() throws Exception {
        CommandCapture capture = runBoundedCommand(
            new String[] { "dumpsys", "media.camera" },
            CAMERA_DUMPSYS_MAX_BYTES,
            CAMERA_DUMPSYS_TIMEOUT_SECONDS);
        JSONObject probe = new JSONObject();
        probe.put("schema", "rusty.xr.shell_helper.camera_probe.v1");
        probe.put("source", "dumpsys media.camera");
        probe.put("captured_time_unix_ms", System.currentTimeMillis());
        probe.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        probe.put("exit_code", capture.exitCode);
        probe.put("timed_out", capture.timedOut);
        probe.put("raw_output_bytes", capture.outputBytes);
        probe.put("raw_output_truncated", capture.truncated);
        if (capture.exitCode != 0 || capture.timedOut) {
            probe.put("error", capture.error);
        }
        parseCameraDumpsys(capture.output, probe);
        return probe;
    }

    private static CommandCapture runBoundedCommand(String[] command, int maxBytes, int timeoutSeconds) throws Exception {
        ProcessBuilder builder = new ProcessBuilder(command);
        builder.redirectErrorStream(true);
        java.lang.Process process = builder.start();
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        byte[] buffer = new byte[8192];
        boolean truncated = false;
        try {
            InputStream input = process.getInputStream();
            while (true) {
                int available = input.available();
                if (available <= 0) {
                    if (!process.isAlive()) {
                        break;
                    }
                    Thread.sleep(10);
                    continue;
                }

                int read = input.read(buffer, 0, Math.min(buffer.length, available));
                if (read < 0) {
                    break;
                }
                if (output.size() + read > maxBytes) {
                    int allowed = Math.max(0, maxBytes - output.size());
                    if (allowed > 0) {
                        output.write(buffer, 0, allowed);
                    }
                    truncated = true;
                    process.destroy();
                    break;
                }
                output.write(buffer, 0, read);
            }

            boolean finished = process.waitFor(timeoutSeconds, TimeUnit.SECONDS);
            boolean timedOut = !finished;
            if (timedOut) {
                process.destroy();
            }
            int exitCode = timedOut ? -1 : process.exitValue();
            return new CommandCapture(
                new String(output.toByteArray(), StandardCharsets.UTF_8),
                output.size(),
                truncated,
                timedOut,
                exitCode,
                "");
        } finally {
            process.destroy();
        }
    }

    private static void requestProximityWatchdogStop() throws Exception {
        File stopFile = new File(PROXIMITY_WATCHDOG_STOP_FILE);
        File parent = stopFile.getParentFile();
        if (parent != null && !parent.exists()) {
            parent.mkdirs();
        }
        stopFile.createNewFile();
        System.out.println("Requested proximity watchdog stop via " + PROXIMITY_WATCHDOG_STOP_FILE);
    }

    private static void runProximityWatchdog(Options options, String uidLabel) throws Exception {
        File stopFile = new File(PROXIMITY_WATCHDOG_STOP_FILE);
        if (stopFile.exists() && !stopFile.delete()) {
            System.out.println("Could not clear stale proximity watchdog stop file: " + PROXIMITY_WATCHDOG_STOP_FILE);
        }

        long startedElapsedMs = SystemClock.elapsedRealtime();
        long deadlineElapsedMs = startedElapsedMs + options.proximityWatchdogDurationMs;
        int reapplyCount = 0;
        int readFailureCount = 0;
        int broadcastFailureCount = 0;
        int powerReadFailureCount = 0;
        int stayAwakeApplyCount = 0;
        int wakeApplyCount = 0;
        String lastVirtualState = "";
        String lastHeadsetState = "";
        String lastWakefulness = "";
        String lastDisplayPowerState = "";
        boolean lastStayOn = false;
        String lastAction = "started";
        String lastError = "";
        sendProximityWatchdogHeartbeat(
            options,
            uidLabel,
            true,
            lastAction,
            lastVirtualState,
            lastHeadsetState,
            lastWakefulness,
            lastDisplayPowerState,
            lastStayOn,
            reapplyCount,
            readFailureCount,
            broadcastFailureCount,
            powerReadFailureCount,
            stayAwakeApplyCount,
            wakeApplyCount,
            Math.max(0L, deadlineElapsedMs - SystemClock.elapsedRealtime()),
            lastError);

        while (options.proximityWatchdogUntilStopped || SystemClock.elapsedRealtime() < deadlineElapsedMs) {
            if (stopFile.exists()) {
                lastAction = "stopped";
                break;
            }

            lastAction = "";
            lastError = "";
            if (options.proximityWatchdogEnsureStayAwake) {
                PowerReadback power = readPowerState();
                lastWakefulness = power.wakefulness;
                lastDisplayPowerState = power.displayPowerState;
                lastStayOn = power.stayOn;
                if (!power.available) {
                    powerReadFailureCount++;
                    lastAction = "power_read_failed";
                    lastError = power.error;
                } else {
                    if (!power.stayOn) {
                        CommandCapture stayOn = enableStayAwake();
                        if (stayOn.exitCode == 0 && !stayOn.timedOut) {
                            stayAwakeApplyCount++;
                            lastAction = "reapplied_stay_awake";
                            lastError = "";
                        } else {
                            powerReadFailureCount++;
                            lastAction = "stay_awake_apply_failed";
                            lastError = stayOn.timedOut
                                ? "svc power stayon timed out"
                                : "svc power stayon exit=" + stayOn.exitCode + " " + stayOn.output.trim();
                        }
                    }
                    if (shouldWakeDevice(power)) {
                        CommandCapture wake = wakeDevice();
                        if (wake.exitCode == 0 && !wake.timedOut) {
                            wakeApplyCount++;
                            lastAction = "reapplied_wake";
                            lastError = "";
                        } else {
                            powerReadFailureCount++;
                            lastAction = "wake_apply_failed";
                            lastError = wake.timedOut
                                ? "input keyevent wakeup timed out"
                                : "input keyevent wakeup exit=" + wake.exitCode + " " + wake.output.trim();
                        }
                    }
                }
            }

            ProximityReadback readback = readProximityState();
            lastVirtualState = readback.virtualState;
            lastHeadsetState = readback.headsetState;
            if (!readback.available) {
                readFailureCount++;
                lastAction = "proximity_read_failed";
                lastError = readback.error;
            } else if ("CLOSE".equalsIgnoreCase(readback.virtualState)) {
                if (lastAction.length() == 0 || "started".equals(lastAction)) {
                    lastAction = "observed_close";
                    lastError = "";
                }
            } else {
                CommandCapture broadcast = broadcastProxClose(options.proximityWatchdogHoldDurationMs);
                if (broadcast.exitCode == 0 && !broadcast.timedOut) {
                    reapplyCount++;
                    lastAction = "reapplied_prox_close";
                    lastError = "";
                } else {
                    broadcastFailureCount++;
                    lastAction = "broadcast_failed";
                    lastError = broadcast.timedOut
                        ? "prox_close broadcast timed out"
                        : "prox_close broadcast exit=" + broadcast.exitCode + " " + broadcast.output.trim();
                }
            }

            sendProximityWatchdogHeartbeat(
                options,
                uidLabel,
                true,
                lastAction,
                lastVirtualState,
                lastHeadsetState,
                lastWakefulness,
                lastDisplayPowerState,
                lastStayOn,
                reapplyCount,
                readFailureCount,
                broadcastFailureCount,
                powerReadFailureCount,
                stayAwakeApplyCount,
                wakeApplyCount,
                Math.max(0L, deadlineElapsedMs - SystemClock.elapsedRealtime()),
                lastError);

            long remainingMs = options.proximityWatchdogUntilStopped
                ? options.proximityWatchdogIntervalMs
                : deadlineElapsedMs - SystemClock.elapsedRealtime();
            if (remainingMs <= 0L) {
                break;
            }
            Thread.sleep(Math.min(options.proximityWatchdogIntervalMs, remainingMs));
        }

        sendProximityWatchdogHeartbeat(
            options,
            uidLabel,
            false,
            lastAction,
            lastVirtualState,
            lastHeadsetState,
            lastWakefulness,
            lastDisplayPowerState,
            lastStayOn,
            reapplyCount,
            readFailureCount,
            broadcastFailureCount,
            powerReadFailureCount,
            stayAwakeApplyCount,
            wakeApplyCount,
            0L,
            lastError);
    }

    private static void sendProximityWatchdogHeartbeat(
            Options options,
            String uidLabel,
            boolean active,
            String lastAction,
            String virtualState,
            String headsetState,
            String wakefulness,
            String displayPowerState,
            boolean stayOn,
            int reapplyCount,
            int readFailureCount,
            int broadcastFailureCount,
            int powerReadFailureCount,
            int stayAwakeApplyCount,
            int wakeApplyCount,
            long remainingMs,
            String lastError) throws Exception {
        JSONObject report = new JSONObject();
        report.put("connected", active);
        report.put("helper_version", VERSION);
        report.put("uid", uidLabel);
        JSONArray capabilities = new JSONArray();
        capabilities.put("shell.uid.report");
        capabilities.put("shell.proximity_watchdog.v1");
        capabilities.put("shell.proximity_watchdog.stop_file");
        capabilities.put("shell.proximity_watchdog.until_stopped");
        capabilities.put("shell.proximity_watchdog.stay_awake");
        report.put("capabilities", capabilities);
        JSONArray activeStreams = new JSONArray();
        if (active) {
            activeStreams.put("shell_helper.status");
        }
        report.put("active_streams", activeStreams);
        JSONObject diagnostics = new JSONObject();
        diagnostics.put(
            "proximity_watchdog",
            buildProximityWatchdogStatus(
                options,
                active,
                lastAction,
                virtualState,
                headsetState,
                wakefulness,
                displayPowerState,
                stayOn,
                reapplyCount,
                readFailureCount,
                broadcastFailureCount,
                powerReadFailureCount,
                stayAwakeApplyCount,
                wakeApplyCount,
                remainingMs,
                lastError));
        report.put("diagnostics", diagnostics);
        report.put("last_error", lastError != null ? lastError : "");
        try {
            sendBrokerCommand(options.host, options.port, "shell_helper.report_status", report);
        } catch (Exception ex) {
            System.out.println("Proximity watchdog heartbeat report failed: " + exceptionSummary(ex));
        }
    }

    private static JSONObject buildProximityWatchdogStatus(
            Options options,
            boolean active,
            String lastAction,
            String virtualState,
            String headsetState,
            String wakefulness,
            String displayPowerState,
            boolean stayOn,
            int reapplyCount,
            int readFailureCount,
            int broadcastFailureCount,
            int powerReadFailureCount,
            int stayAwakeApplyCount,
            int wakeApplyCount,
            long remainingMs,
            String lastError) throws Exception {
        JSONObject status = new JSONObject();
        status.put("schema", "rusty.xr.shell_helper.proximity_watchdog.v1");
        status.put("enabled", options.proximityWatchdog);
        status.put("active", active);
        status.put("stop_requested", options.stopProximityWatchdog);
        status.put("until_stopped", options.proximityWatchdogUntilStopped);
        status.put("ensure_stay_awake", options.proximityWatchdogEnsureStayAwake);
        status.put("duration_ms", options.proximityWatchdogDurationMs);
        status.put("hold_duration_ms", options.proximityWatchdogHoldDurationMs);
        status.put("interval_ms", options.proximityWatchdogIntervalMs);
        status.put("remaining_ms", options.proximityWatchdogUntilStopped ? -1L : Math.max(0L, remainingMs));
        status.put("last_action", lastAction != null ? lastAction : "");
        status.put("virtual_proximity_state", virtualState != null ? virtualState : "");
        status.put("headset_state", headsetState != null ? headsetState : "");
        status.put("wakefulness", wakefulness != null ? wakefulness : "");
        status.put("display_power_state", displayPowerState != null ? displayPowerState : "");
        status.put("stay_on", stayOn);
        status.put("reapply_count", reapplyCount);
        status.put("read_failure_count", readFailureCount);
        status.put("broadcast_failure_count", broadcastFailureCount);
        status.put("power_read_failure_count", powerReadFailureCount);
        status.put("stay_awake_apply_count", stayAwakeApplyCount);
        status.put("wake_apply_count", wakeApplyCount);
        status.put("stop_file", PROXIMITY_WATCHDOG_STOP_FILE);
        status.put("log_file", PROXIMITY_WATCHDOG_LOG_FILE);
        status.put("non_interference_rule", "only_reapply_proximity_when_virtual_state_is_not_close");
        status.put("last_error", lastError != null ? lastError : "");
        return status;
    }

    private static void requestFocusGuardianStop() throws Exception {
        File stopFile = new File(FOCUS_GUARDIAN_STOP_FILE);
        File parent = stopFile.getParentFile();
        if (parent != null && !parent.exists()) {
            parent.mkdirs();
        }
        stopFile.createNewFile();
        System.out.println("Requested focus guardian stop via " + FOCUS_GUARDIAN_STOP_FILE);
    }

    private static void runFocusGuardian(Options options, String uidLabel) throws Exception {
        File stopFile = new File(FOCUS_GUARDIAN_STOP_FILE);
        if (stopFile.exists() && !stopFile.delete()) {
            System.out.println("Could not clear stale focus guardian stop file: " + FOCUS_GUARDIAN_STOP_FILE);
        }

        long startedElapsedMs = SystemClock.elapsedRealtime();
        long deadlineElapsedMs = startedElapsedMs + options.focusGuardianDurationMs;
        long lastLaunchElapsedMs = 0L;
        long appliedRevision = -1L;
        int recoveryCount = 0;
        int propertyApplyCount = 0;
        String desiredSide = options.focusGuardianDesiredFocus;
        String activeSide = "";
        String mode = options.focusGuardianMode;
        String targetPackage = options.focusTargetPackage;
        String targetActivity = options.focusTargetActivity;
        String brokerPackage = options.focusBrokerPackage;
        String brokerActivity = options.focusBrokerActivity;
        String foregroundPackage = "";
        String foregroundActivity = "";
        String lastAction = "started";
        String lastError = "";
        String pendingRecoverySide = "";
        long pendingRecoveryDeadlineMs = 0L;
        long pendingRecoveryObservedSinceMs = 0L;
        long lastControlChangeElapsedMs = startedElapsedMs;
        long lastObservedSideElapsedMs = 0L;
        int pendingRecoveryAttempts = 0;
        long launchGuardRevision = -1L;
        long launchGuardStartedMs = 0L;
        boolean launchGuardLaunched = false;
        boolean launchGuardRecovered = false;
        boolean launchGuardTargetReady = false;
        long launchGuardTargetReadyMs = 0L;
        int launchGuardTimeoutMs = FOCUS_GUARDIAN_DEFAULT_LAUNCH_GUARD_TIMEOUT_MS;
        boolean launchGuardPreviewTimeoutEnabled = false;

        sendFocusGuardianHeartbeat(
            options,
            targetPackage,
            targetActivity,
            launchGuardPreviewTimeoutEnabled,
            uidLabel,
            true,
            mode,
            activeSide,
            foregroundPackage,
            foregroundActivity,
            lastAction,
            recoveryCount,
            propertyApplyCount,
            appliedRevision,
            Math.max(0L, deadlineElapsedMs - SystemClock.elapsedRealtime()),
            lastError);

        while (SystemClock.elapsedRealtime() < deadlineElapsedMs) {
            if (stopFile.exists()) {
                lastAction = "stopped";
                break;
            }

            JSONObject control = null;
            try {
                control = fetchExperimentControl(options);
            } catch (Exception ex) {
                lastError = "control poll failed: " + exceptionSummary(ex);
            }

            if (control != null) {
                String nextMode = control.optString("mode", mode);
                boolean enabled = control.optBoolean("enabled", !"off".equals(nextMode));
                if (!enabled) {
                    nextMode = "off";
                }
                String nextDesiredSide = control.optString("desired_focus", desiredSide);
                String nextTargetPackage = control.optString("target_package", targetPackage);
                String nextTargetActivity = control.optString("target_activity", targetActivity);
                String nextBrokerPackage = control.optString("broker_package", brokerPackage);
                String nextBrokerActivity = control.optString("broker_activity", brokerActivity);

                long revision = control.optLong("revision", appliedRevision);
                boolean identityChanged = !nextTargetPackage.equals(targetPackage) ||
                    !nextTargetActivity.equals(targetActivity) ||
                    !nextBrokerPackage.equals(brokerPackage) ||
                    !nextBrokerActivity.equals(brokerActivity);
                boolean nextLaunchGuardPreviewTimeoutEnabled =
                    control.optBoolean("launch_guard_preview_timeout_enabled", launchGuardPreviewTimeoutEnabled);
                boolean controlChanged = revision != appliedRevision ||
                    !nextMode.equals(mode) ||
                    !nextDesiredSide.equals(desiredSide) ||
                    identityChanged ||
                    nextLaunchGuardPreviewTimeoutEnabled != launchGuardPreviewTimeoutEnabled;
                mode = nextMode;
                desiredSide = nextDesiredSide;
                targetPackage = nextTargetPackage;
                targetActivity = nextTargetActivity;
                brokerPackage = nextBrokerPackage;
                brokerActivity = nextBrokerActivity;
                launchGuardPreviewTimeoutEnabled = nextLaunchGuardPreviewTimeoutEnabled;
                if (controlChanged) {
                    lastControlChangeElapsedMs = SystemClock.elapsedRealtime();
                    pendingRecoverySide = "";
                    pendingRecoveryAttempts = 0;
                    pendingRecoveryDeadlineMs = 0L;
                    pendingRecoveryObservedSinceMs = 0L;
                    if (identityChanged) {
                        activeSide = "";
                        lastObservedSideElapsedMs = 0L;
                    }
                }
                launchGuardTimeoutMs = clampInt(
                    control.optInt("launch_guard_timeout_ms", FOCUS_GUARDIAN_DEFAULT_LAUNCH_GUARD_TIMEOUT_MS),
                    FOCUS_GUARDIAN_MIN_LAUNCH_GUARD_TIMEOUT_MS,
                    FOCUS_GUARDIAN_MAX_LAUNCH_GUARD_TIMEOUT_MS);
                if ("launch_target_guard".equals(mode)) {
                    if (revision != launchGuardRevision) {
                        launchGuardRevision = revision;
                        launchGuardStartedMs = SystemClock.elapsedRealtime();
                        launchGuardLaunched = false;
                        launchGuardRecovered = false;
                        launchGuardTargetReady = false;
                        launchGuardTargetReadyMs = 0L;
                        pendingRecoverySide = "";
                        pendingRecoveryAttempts = 0;
                        pendingRecoveryDeadlineMs = 0L;
                        pendingRecoveryObservedSinceMs = 0L;
                        lastAction = "launch_guard_armed";
                    }
                } else {
                    launchGuardRevision = -1L;
                    launchGuardStartedMs = 0L;
                    launchGuardLaunched = false;
                    launchGuardRecovered = false;
                    launchGuardTargetReady = false;
                    launchGuardTargetReadyMs = 0L;
                }
                if (revision != appliedRevision) {
                    try {
                        int applied = applyExperimentPropertyWrites(control);
                        propertyApplyCount += applied;
                        appliedRevision = revision;
                        lastAction = "applied_control";
                        lastError = "";
                    } catch (Exception ex) {
                        lastAction = "apply_control_failed";
                        lastError = exceptionSummary(ex);
                    }
                }
            }

            ForegroundReadback foreground = readForegroundState();
            long nowElapsedMs = SystemClock.elapsedRealtime();
            foregroundPackage = foreground.packageName;
            foregroundActivity = foreground.activityName;
            boolean targetFocused = foreground.available && isSamePackage(foreground.packageName, targetPackage);
            boolean brokerFocused = foreground.available && isSamePackage(foreground.packageName, brokerPackage);
            if (!foreground.available) {
                lastError = foreground.error;
            } else if (targetFocused) {
                activeSide = "target";
                lastObservedSideElapsedMs = nowElapsedMs;
                if ("launch_target_guard".equals(mode) && launchGuardLaunched) {
                    launchGuardTargetReady = true;
                    if (launchGuardTargetReadyMs == 0L) {
                        launchGuardTargetReadyMs = nowElapsedMs;
                    }
                }
                lastAction = "observed_target";
                lastError = "";
            } else if (brokerFocused) {
                activeSide = "broker";
                lastObservedSideElapsedMs = nowElapsedMs;
                lastAction = "observed_broker";
                lastError = "";
            }

            String launchSide = "";
            if (pendingRecoverySide.length() > 0) {
                boolean pendingSatisfied = ("target".equals(pendingRecoverySide) && targetFocused) ||
                    ("broker".equals(pendingRecoverySide) && brokerFocused);
                if (pendingSatisfied) {
                    if (pendingRecoveryObservedSinceMs == 0L) {
                        pendingRecoveryObservedSinceMs = nowElapsedMs;
                    }
                } else {
                    pendingRecoveryObservedSinceMs = 0L;
                }
                if ((pendingSatisfied &&
                        nowElapsedMs - pendingRecoveryObservedSinceMs >= FOCUS_GUARDIAN_PENDING_RECOVERY_HOLD_MS) ||
                    nowElapsedMs > pendingRecoveryDeadlineMs ||
                    pendingRecoveryAttempts >= FOCUS_GUARDIAN_MAX_PENDING_RECOVERY_ATTEMPTS) {
                    pendingRecoverySide = "";
                    pendingRecoveryAttempts = 0;
                    pendingRecoveryDeadlineMs = 0L;
                    pendingRecoveryObservedSinceMs = 0L;
                }
            }

            boolean launchGuardMode = "launch_target_guard".equals(mode);
            if (launchGuardMode && foreground.available && !foreground.protectedSystemFlow) {
                if (launchGuardRecovered) {
                    if (brokerFocused) {
                        try {
                            configureExperimentMode(options, "off", "broker");
                            lastAction = "launch_guard_recovered_broker_confirmed";
                            lastError = "";
                        } catch (Exception ex) {
                            lastAction = "launch_guard_disable_failed";
                            lastError = exceptionSummary(ex);
                        }
                    } else {
                        launchSide = "broker";
                        lastAction = "launch_guard_retrying_broker";
                    }
                } else if (launchGuardPreviewTimeoutEnabled &&
                        launchGuardLaunched &&
                        launchGuardTargetReady &&
                        launchGuardTargetReadyMs > 0L &&
                        nowElapsedMs - launchGuardTargetReadyMs >= launchGuardTimeoutMs) {
                    CommandCapture stop = forceStopPackage(targetPackage);
                    CommandCapture launch = launchBrokerConsole(options, brokerPackage, brokerActivity);
                    lastLaunchElapsedMs = nowElapsedMs;
                    launchGuardRecovered = true;
                    pendingRecoverySide = "broker";
                    pendingRecoveryAttempts++;
                    pendingRecoveryDeadlineMs = nowElapsedMs + FOCUS_GUARDIAN_PENDING_RECOVERY_MS;
                    pendingRecoveryObservedSinceMs = 0L;
                    recoveryCount++;
                    if (launch.exitCode == 0 && !launch.timedOut) {
                        lastAction = stop.exitCode == 0 && !stop.timedOut
                            ? "launch_guard_preview_returned_broker"
                            : "launch_guard_preview_returned_broker_stop_failed";
                        if (lastError.length() == 0 && (stop.exitCode != 0 || stop.timedOut)) {
                            lastError = stop.timedOut
                                ? "target force-stop timed out"
                                : "target force-stop exit=" + stop.exitCode + " " + stop.output.trim();
                        }
                    } else {
                        lastAction = "launch_guard_preview_return_broker_failed";
                        lastError = launch.timedOut
                            ? "broker launch timed out"
                            : "broker launch exit=" + launch.exitCode + " " + launch.output.trim();
                    }
                } else if (launchGuardLaunched &&
                        launchGuardTargetReady &&
                        "target".equals(activeSide) &&
                        !targetFocused &&
                        !brokerFocused &&
                        (foreground.metaHome || foreground.visibleMetaMenuOverlay)) {
                    CommandCapture launch = launchBrokerConsole(options, brokerPackage, brokerActivity);
                    lastLaunchElapsedMs = nowElapsedMs;
                    launchGuardRecovered = true;
                    pendingRecoverySide = "broker";
                    pendingRecoveryAttempts++;
                    pendingRecoveryDeadlineMs = nowElapsedMs + FOCUS_GUARDIAN_PENDING_RECOVERY_MS;
                    pendingRecoveryObservedSinceMs = 0L;
                    recoveryCount++;
                    if (launch.exitCode == 0 && !launch.timedOut) {
                        lastAction = "launch_guard_menu_recovered_broker";
                    } else {
                        lastAction = "launch_guard_menu_recover_broker_failed";
                        lastError = launch.timedOut
                            ? "broker launch timed out"
                            : "broker launch exit=" + launch.exitCode + " " + launch.output.trim();
                    }
                } else if (!launchGuardRecovered &&
                        !launchGuardTargetReady &&
                        launchGuardStartedMs > 0L &&
                        nowElapsedMs - launchGuardStartedMs >= launchGuardTimeoutMs) {
                    CommandCapture stop = forceStopPackage(targetPackage);
                    CommandCapture launch = launchBrokerConsole(options, brokerPackage, brokerActivity);
                    lastLaunchElapsedMs = nowElapsedMs;
                    launchGuardRecovered = true;
                    pendingRecoverySide = "broker";
                    pendingRecoveryAttempts++;
                    pendingRecoveryDeadlineMs = nowElapsedMs + FOCUS_GUARDIAN_PENDING_RECOVERY_MS;
                    pendingRecoveryObservedSinceMs = 0L;
                    recoveryCount++;
                    if (launch.exitCode == 0 && !launch.timedOut) {
                        lastAction = stop.exitCode == 0 && !stop.timedOut
                            ? "launch_guard_recovered_broker"
                            : "launch_guard_recovered_broker_stop_failed";
                        if (lastError.length() == 0 && (stop.exitCode != 0 || stop.timedOut)) {
                            lastError = stop.timedOut
                                ? "target force-stop timed out"
                                : "target force-stop exit=" + stop.exitCode + " " + stop.output.trim();
                        }
                    } else {
                        lastAction = "launch_guard_recover_broker_failed";
                        lastError = launch.timedOut
                            ? "broker launch timed out"
                            : "broker launch exit=" + launch.exitCode + " " + launch.output.trim();
                    }
                } else if (!launchGuardLaunched) {
                    launchSide = "target";
                    lastAction = "launch_guard_launching_target";
                } else if (launchGuardLaunched && targetFocused) {
                    launchGuardTargetReady = true;
                    if (launchGuardTargetReadyMs == 0L) {
                        launchGuardTargetReadyMs = nowElapsedMs;
                    }
                    lastAction = "launch_guard_observed_target";
                }
            } else if (!"off".equals(mode) && !"observe".equals(mode) && foreground.available && !foreground.protectedSystemFlow) {
                boolean otherForeground = !targetFocused && !brokerFocused;
                boolean menuInterrupted = otherForeground &&
                    (foreground.metaHome || foreground.visibleMetaMenuOverlay);
                boolean launchTransition = otherForeground &&
                    (pendingRecoverySide.length() > 0 ||
                    nowElapsedMs - lastLaunchElapsedMs < FOCUS_GUARDIAN_TOGGLE_TRANSITION_GRACE_MS ||
                    nowElapsedMs - lastControlChangeElapsedMs < FOCUS_GUARDIAN_TOGGLE_TRANSITION_GRACE_MS);
                if (pendingRecoverySide.length() > 0) {
                    launchSide = pendingRecoverySide;
                } else if ("recover_target".equals(mode) || "strict".equals(mode)) {
                    if (!targetFocused) {
                        launchSide = "target";
                    }
                } else if ("recover_broker".equals(mode)) {
                    if (!brokerFocused) {
                        launchSide = "broker";
                    }
                } else if ("toggle_broker_target".equals(mode) && menuInterrupted) {
                    if (activeSide.length() > 0 && lastObservedSideElapsedMs > 0L && !launchTransition) {
                        launchSide = "broker".equals(activeSide) ? "target" : "broker";
                    } else {
                        lastAction = "ignored_toggle_transition";
                        lastError = "";
                    }
                } else if ("toggle_broker_target".equals(mode) && otherForeground) {
                    lastAction = "observed_other_foreground";
                    lastError = "";
                }
            }

            int cooldownMs = "strict".equals(mode)
                ? Math.max(250, options.focusGuardianCooldownMs / 2)
                : options.focusGuardianCooldownMs;
            if (launchSide.length() > 0 && nowElapsedMs - lastLaunchElapsedMs >= cooldownMs) {
                CommandCapture preLaunchStop = null;
                if (launchGuardMode && "target".equals(launchSide)) {
                    preLaunchStop = forceStopPackage(targetPackage);
                    Thread.sleep(250L);
                }
                CommandCapture launch = "broker".equals(launchSide)
                    ? launchBrokerConsole(options, brokerPackage, brokerActivity)
                    : launchComponent(targetPackage, targetActivity);
                lastLaunchElapsedMs = nowElapsedMs;
                if (launch.exitCode == 0 && !launch.timedOut) {
                    pendingRecoverySide = launchSide;
                    pendingRecoveryAttempts++;
                    pendingRecoveryDeadlineMs = nowElapsedMs + FOCUS_GUARDIAN_PENDING_RECOVERY_MS;
                    pendingRecoveryObservedSinceMs = 0L;
                    recoveryCount++;
                    if (launchGuardMode && "target".equals(launchSide)) {
                        launchGuardLaunched = true;
                        launchGuardStartedMs = nowElapsedMs;
                        launchGuardTargetReady = false;
                        launchGuardTargetReadyMs = 0L;
                        lastAction = "launch_guard_launched_target";
                    } else {
                        lastAction = "launched_" + launchSide;
                    }
                    if (preLaunchStop != null && (preLaunchStop.exitCode != 0 || preLaunchStop.timedOut)) {
                        lastError = preLaunchStop.timedOut
                            ? "target pre-launch force-stop timed out"
                            : "target pre-launch force-stop exit=" + preLaunchStop.exitCode + " " +
                                preLaunchStop.output.trim();
                    } else {
                        lastError = "";
                    }
                } else {
                    lastAction = "launch_failed_" + launchSide;
                    lastError = launch.timedOut
                        ? "am start timed out"
                        : "am start exit=" + launch.exitCode + " " + launch.output.trim();
                }
            } else if (foreground.protectedSystemFlow) {
                lastAction = "observed_protected_system_flow";
            }

            sendFocusGuardianHeartbeat(
                options,
                targetPackage,
                targetActivity,
                launchGuardPreviewTimeoutEnabled,
                uidLabel,
                true,
                mode,
                activeSide,
                foregroundPackage,
                foregroundActivity,
                lastAction,
                recoveryCount,
                propertyApplyCount,
                appliedRevision,
                Math.max(0L, deadlineElapsedMs - SystemClock.elapsedRealtime()),
                lastError);

            long remainingMs = deadlineElapsedMs - SystemClock.elapsedRealtime();
            if (remainingMs <= 0L) {
                break;
            }
            Thread.sleep(Math.min(options.focusGuardianIntervalMs, remainingMs));
        }

        sendFocusGuardianHeartbeat(
            options,
            targetPackage,
            targetActivity,
            launchGuardPreviewTimeoutEnabled,
            uidLabel,
            false,
            mode,
            activeSide,
            foregroundPackage,
            foregroundActivity,
            lastAction,
            recoveryCount,
            propertyApplyCount,
            appliedRevision,
            0L,
            lastError);
    }

    private static JSONObject fetchExperimentControl(Options options) throws Exception {
        JSONObject ack = sendBrokerCommand(options.host, options.port, "experiment.get_control", new JSONObject());
        JSONObject result = ack.optJSONObject("result");
        return result != null ? result.optJSONObject("control") : null;
    }

    private static int applyExperimentPropertyWrites(JSONObject control) throws Exception {
        JSONArray writes = control.optJSONArray("property_writes");
        if (writes == null) {
            return 0;
        }

        int applied = 0;
        for (int i = 0; i < writes.length(); i++) {
            JSONObject write = writes.optJSONObject(i);
            if (write == null) {
                continue;
            }
            String name = write.optString("name", "");
            String value = write.optString("value", "");
            if (!isAllowedRuntimeProperty(name) || value.length() == 0) {
                continue;
            }
            CommandCapture capture = runBoundedCommand(
                new String[] { "setprop", name, value },
                4096,
                3);
            if (capture.exitCode != 0 || capture.timedOut) {
                throw new IllegalStateException("setprop failed for " + name + ": " + capture.output.trim());
            }
            applied++;
        }
        return applied;
    }

    private static boolean isAllowedRuntimeProperty(String name) {
        return name != null && name.startsWith("debug.rustyxr.") && name.length() <= 92;
    }

    private static void sendFocusGuardianHeartbeat(
            Options options,
            String targetPackage,
            String targetActivity,
            boolean launchGuardPreviewTimeoutEnabled,
            String uidLabel,
            boolean active,
            String mode,
            String activeSide,
            String foregroundPackage,
            String foregroundActivity,
            String lastAction,
            int recoveryCount,
            int propertyApplyCount,
            long appliedRevision,
            long remainingMs,
            String lastError) throws Exception {
        JSONObject report = new JSONObject();
        report.put("connected", active);
        report.put("helper_version", VERSION);
        report.put("uid", uidLabel);
        JSONArray capabilities = new JSONArray();
        capabilities.put("shell.uid.report");
        capabilities.put("shell.focus_guardian.v1");
        capabilities.put("shell.focus_guardian.stop_file");
        capabilities.put("shell.focus_guardian.setprop_whitelist.debug_rustyxr");
        capabilities.put("shell.focus_guardian.launch_target_guard.v1");
        report.put("capabilities", capabilities);
        JSONArray activeStreams = new JSONArray();
        if (active) {
            activeStreams.put("shell_helper.status");
        }
        report.put("active_streams", activeStreams);
        JSONObject status = buildFocusGuardianStatus(
            options,
            active,
            mode,
            activeSide,
            foregroundPackage,
            foregroundActivity,
            lastAction,
            recoveryCount,
            propertyApplyCount,
            appliedRevision,
            targetPackage,
            targetActivity,
            launchGuardPreviewTimeoutEnabled,
            remainingMs,
            lastError);
        JSONObject diagnostics = new JSONObject();
        diagnostics.put("focus_guardian", status);
        report.put("diagnostics", diagnostics);
        report.put("last_error", lastError != null ? lastError : "");
        try {
            sendBrokerCommand(options.host, options.port, "shell_helper.report_status", report);
            sendBrokerCommand(options.host, options.port, "experiment.report_status", status);
        } catch (Exception ex) {
            System.out.println("Focus guardian heartbeat report failed: " + exceptionSummary(ex));
        }
    }

    private static JSONObject buildFocusGuardianStatus(
            Options options,
            boolean active,
            String mode,
            String activeSide,
            String foregroundPackage,
            String foregroundActivity,
            String lastAction,
            int recoveryCount,
            int propertyApplyCount,
            long appliedRevision,
            String targetPackage,
            String targetActivity,
            boolean launchGuardPreviewTimeoutEnabled,
            long remainingMs,
            String lastError) throws Exception {
        JSONObject status = new JSONObject();
        status.put("schema", "rusty.xr.shell_helper.focus_guardian.v1");
        status.put("enabled", options.focusGuardian);
        status.put("active", active);
        status.put("mode", mode != null ? mode : "");
        status.put("active_side", activeSide != null ? activeSide : "");
        status.put("foreground_package", foregroundPackage != null ? foregroundPackage : "");
        status.put("foreground_activity", foregroundActivity != null ? foregroundActivity : "");
        status.put("last_action", lastAction != null ? lastAction : "");
        status.put("recovery_count", recoveryCount);
        status.put("property_apply_count", propertyApplyCount);
        status.put("applied_revision", appliedRevision);
        status.put("duration_ms", options.focusGuardianDurationMs);
        status.put("interval_ms", options.focusGuardianIntervalMs);
        status.put("cooldown_ms", options.focusGuardianCooldownMs);
        status.put("remaining_ms", Math.max(0L, remainingMs));
        status.put("target_package", targetPackage != null ? targetPackage : "");
        status.put("target_activity", targetActivity != null ? targetActivity : "");
        status.put("launch_guard_preview_timeout_enabled", launchGuardPreviewTimeoutEnabled);
        status.put("broker_package", options.focusBrokerPackage);
        status.put("broker_activity", options.focusBrokerActivity);
        status.put("stop_file", FOCUS_GUARDIAN_STOP_FILE);
        status.put("log_file", FOCUS_GUARDIAN_LOG_FILE);
        status.put("toggle_transition_grace_ms", FOCUS_GUARDIAN_TOGGLE_TRANSITION_GRACE_MS);
        status.put("non_interference_rule", "actual_foreground_side_only_skip_protected_flows_and_launch_transitions");
        status.put("last_error", lastError != null ? lastError : "");
        return status;
    }

    private static ForegroundReadback readForegroundState() {
        try {
            CommandCapture capture = runBoundedCommand(
                new String[] { "dumpsys", "window", "windows" },
                FOCUS_DUMPSYS_MAX_BYTES,
                FOCUS_DUMPSYS_TIMEOUT_SECONDS);
            if (capture.exitCode != 0 || capture.timedOut) {
                String error = capture.timedOut
                    ? "dumpsys window timed out"
                    : "dumpsys window exit=" + capture.exitCode + " " + capture.output.trim();
                return new ForegroundReadback(false, "", "", false, false, false, error);
            }

            String output = capture.output != null ? capture.output : "";
            String packageName = "";
            String activityName = "";
            Matcher current = CURRENT_FOCUS_COMPONENT_PATTERN.matcher(output);
            if (current.find()) {
                packageName = current.group(1);
                activityName = cleanActivityName(current.group(2));
            } else {
                Matcher focused = FOCUSED_APP_COMPONENT_PATTERN.matcher(output);
                if (focused.find()) {
                    packageName = focused.group(1);
                    activityName = cleanActivityName(focused.group(2));
                } else {
                    String[] visibleComponent = findVisibleWindowComponent(output);
                    packageName = visibleComponent[0];
                    activityName = visibleComponent[1];
                }
            }

            String foregroundToken = (packageName + "/" + activityName).toLowerCase(Locale.ROOT);
            boolean metaHome = foregroundToken.contains("com.oculus.vrshell") ||
                foregroundToken.contains("com.oculus.panelapp") ||
                foregroundToken.contains("homeactivity") ||
                foregroundToken.contains("quickactions");
            boolean visibleMetaMenuOverlay = hasVisibleMetaMenuOverlay(output);
            boolean protectedFlow = foregroundToken.contains("permissioncontroller") ||
                foregroundToken.contains("packageinstaller") ||
                foregroundToken.contains("com.oculus.guardian");
            return new ForegroundReadback(true, packageName, activityName, metaHome, visibleMetaMenuOverlay, protectedFlow, "");
        } catch (Exception ex) {
            return new ForegroundReadback(false, "", "", false, false, false, exceptionSummary(ex));
        }
    }

    private static boolean hasVisibleMetaMenuOverlay(String output) {
        if (output == null || output.length() == 0) {
            return false;
        }
        String[] blocks = output.split("(?m)^\\s*Window #");
        for (String block : blocks) {
            String lower = block.toLowerCase(Locale.ROOT);
            if (!lower.contains("isvisible=true") ||
                    !lower.contains("surface: shown=true") ||
                    !lower.contains("com.oculus")) {
                continue;
            }
            if (lower.contains("system_bar_wayfinder_menu") ||
                    lower.contains("quickactions")) {
                return true;
            }
        }
        return false;
    }

    private static String[] findVisibleWindowComponent(String output) {
        if (output == null || output.length() == 0) {
            return new String[] { "", "" };
        }
        String[] blocks = output.split("(?m)^\\s*Window #");
        for (String block : blocks) {
            if (!block.contains("isVisible=true")) {
                continue;
            }
            int lineEnd = block.indexOf('\n');
            String header = lineEnd >= 0 ? block.substring(0, lineEnd) : block;
            Matcher matcher = WINDOW_HEADER_COMPONENT_PATTERN.matcher(header);
            if (matcher.find()) {
                return new String[] {
                    matcher.group(1),
                    cleanActivityName(matcher.group(2))
                };
            }
        }
        return new String[] { "", "" };
    }

    private static CommandCapture launchComponent(String packageName, String activityName) {
        try {
            packageName = packageName != null ? packageName.trim() : "";
            activityName = activityName != null ? activityName.trim() : "";
            if (packageName.length() == 0) {
                return new CommandCapture("", 0, false, false, 2, "missing package");
            }
            if (activityName.length() == 0) {
                CommandCapture vrLaunch = launchResolvedPackageWithCategory(packageName, OCULUS_VR_CATEGORY);
                if (isSuccessfulLaunch(vrLaunch)) {
                    return vrLaunch;
                }
                CommandCapture vrPackageLaunch = launchPackageWithCategory(packageName, OCULUS_VR_CATEGORY);
                if (isSuccessfulLaunch(vrPackageLaunch)) {
                    return vrPackageLaunch;
                }
                return launchLauncherPackage(packageName);
            }

            if (isVrLikeActivity(activityName)) {
                CommandCapture vrLaunch = launchExplicitWithCategory(packageName, activityName, OCULUS_VR_CATEGORY);
                if (isSuccessfulLaunch(vrLaunch)) {
                    return vrLaunch;
                }
            }
            return launchExplicitWithCategory(packageName, activityName, ANDROID_LAUNCHER_CATEGORY);
        } catch (Exception ex) {
            return new CommandCapture("", 0, false, false, 1, exceptionSummary(ex));
        }
    }

    private static CommandCapture launchResolvedPackageWithCategory(
            String packageName,
            String category) throws Exception {
        CommandCapture resolve = runBoundedCommand(
            new String[] {
                "cmd",
                "package",
                "resolve-activity",
                "--brief",
                "-a",
                ANDROID_MAIN_ACTION,
                "-c",
                category,
                "-p",
                packageName
            },
            16 * 1024,
            5);
        if (!isSuccessfulLaunch(resolve)) {
            return resolve;
        }
        String componentName = resolvedComponentName(resolve.output, packageName);
        if (componentName.length() == 0) {
            return new CommandCapture(
                resolve.output,
                resolve.outputBytes,
                resolve.truncated,
                resolve.timedOut,
                3,
                "resolve-activity did not return a launch component");
        }
        return launchComponentNameWithCategory(componentName, category);
    }

    private static CommandCapture launchPackageWithCategory(String packageName, String category) throws Exception {
        return runBoundedCommand(
            new String[] {
                "am",
                "start",
                "-W",
                "-a",
                ANDROID_MAIN_ACTION,
                "-c",
                category,
                "-p",
                packageName
            },
            16 * 1024,
            6);
    }

    private static CommandCapture launchComponentNameWithCategory(String componentName, String category) throws Exception {
        return runBoundedCommand(
            new String[] {
                "am",
                "start",
                "-W",
                "-a",
                ANDROID_MAIN_ACTION,
                "-c",
                category,
                "-n",
                componentName
            },
            16 * 1024,
            6);
    }

    private static CommandCapture launchExplicitWithCategory(
            String packageName,
            String activityName,
            String category) throws Exception {
        String className = activityName.startsWith(".") ? packageName + activityName : activityName;
        return runBoundedCommand(
            new String[] {
                "am",
                "start",
                "-W",
                "-a",
                ANDROID_MAIN_ACTION,
                "-c",
                category,
                "-n",
                packageName + "/" + className
            },
            16 * 1024,
            6);
    }

    private static CommandCapture launchLauncherPackage(String packageName) throws Exception {
        return runBoundedCommand(
            new String[] {
                "monkey",
                "-p",
                packageName,
                "-c",
                ANDROID_LAUNCHER_CATEGORY,
                "1"
            },
            16 * 1024,
            6);
    }

    private static boolean isSuccessfulLaunch(CommandCapture launch) {
        if (launch == null || launch.exitCode != 0 || launch.timedOut) {
            return false;
        }
        String output = launch.output != null ? launch.output.toLowerCase(Locale.ROOT) : "";
        return !output.contains("error:") && !output.contains("unable to resolve");
    }

    private static boolean isVrLikeActivity(String activityName) {
        if (activityName == null) {
            return false;
        }
        String normalized = activityName.toLowerCase(Locale.ROOT);
        return normalized.contains("xr") || normalized.contains("vr");
    }

    private static String resolvedComponentName(String output, String expectedPackageName) {
        if (output == null || output.length() == 0) {
            return "";
        }
        String[] lines = output.split("\\r?\\n");
        for (int i = lines.length - 1; i >= 0; i--) {
            String line = lines[i] != null ? lines[i].trim() : "";
            if (line.length() == 0 || line.indexOf('/') < 0) {
                continue;
            }
            Matcher matcher = PACKAGE_COMPONENT_PATTERN.matcher(line);
            if (!matcher.matches()) {
                continue;
            }
            String packageName = matcher.group(1);
            String className = matcher.group(2);
            if (!packageName.equals(expectedPackageName) || className.length() == 0) {
                continue;
            }
            return packageName + "/" + className;
        }
        return "";
    }

    private static CommandCapture launchLauncherPackageOrComponent(String packageName, String fallbackActivityName) {
        CommandCapture launch = launchComponent(packageName, "");
        if (launch.exitCode == 0 && !launch.timedOut) {
            return launch;
        }
        if (fallbackActivityName == null || fallbackActivityName.trim().length() == 0) {
            return launch;
        }
        return launchComponent(packageName, fallbackActivityName);
    }

    private static CommandCapture launchBrokerConsole(
            Options options,
            String brokerPackage,
            String fallbackActivityName) {
        try {
            JSONObject ack = sendBrokerCommand(options.host, options.port, "open_ui", new JSONObject());
            String output = ack != null ? ack.toString() : "";
            if (ack != null && ack.optBoolean("accepted", false)) {
                return new CommandCapture(output, output.length(), false, false, 0, "");
            }
        } catch (Exception ignored) {
            // Fall through to the shell-side launcher path.
        }
        return launchLauncherPackageOrComponent(brokerPackage, fallbackActivityName);
    }

    private static CommandCapture forceStopPackage(String packageName) {
        try {
            packageName = packageName != null ? packageName.trim() : "";
            if (packageName.length() == 0) {
                return new CommandCapture("", 0, false, false, 2, "missing package");
            }
            return runBoundedCommand(
                new String[] { "am", "force-stop", packageName },
                8 * 1024,
                4);
        } catch (Exception ex) {
            return new CommandCapture("", 0, false, false, 1, exceptionSummary(ex));
        }
    }

    private static void configureExperimentMode(Options options, String mode, String desiredFocus) throws Exception {
        JSONObject params = new JSONObject();
        params.put("mode", mode != null ? mode : "off");
        if (desiredFocus != null && desiredFocus.length() > 0) {
            params.put("desired_focus", desiredFocus);
        }
        sendBrokerCommand(options.host, options.port, "experiment.configure", params);
    }

    private static boolean isSamePackage(String left, String right) {
        return left != null && right != null && left.trim().length() > 0 && left.trim().equals(right.trim());
    }

    private static int clampInt(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static String cleanActivityName(String value) {
        if (value == null) {
            return "";
        }
        String cleaned = value.trim();
        int end = cleaned.length();
        for (int i = 0; i < cleaned.length(); i++) {
            char ch = cleaned.charAt(i);
            if (Character.isWhitespace(ch) || ch == '}') {
                end = i;
                break;
            }
        }
        return cleaned.substring(0, end);
    }

    private static ProximityReadback readProximityState() {
        try {
            CommandCapture capture = runBoundedCommand(
                new String[] { "dumpsys", "vrpowermanager" },
                VR_POWER_MANAGER_DUMPSYS_MAX_BYTES,
                VR_POWER_MANAGER_COMMAND_TIMEOUT_SECONDS);
            if (capture.exitCode != 0 || capture.timedOut) {
                String error = capture.timedOut
                    ? "dumpsys vrpowermanager timed out"
                    : "dumpsys vrpowermanager exit=" + capture.exitCode + " " + capture.output.trim();
                return new ProximityReadback(false, "", "", error);
            }

            String virtualState = matchFirst(VR_POWER_MANAGER_VIRTUAL_STATE_PATTERN, capture.output);
            String headsetState = matchFirst(VR_POWER_MANAGER_HEADSET_STATE_PATTERN, capture.output);
            if (virtualState.length() == 0) {
                return new ProximityReadback(false, "", headsetState, "Virtual proximity state missing from vrpowermanager");
            }
            return new ProximityReadback(true, virtualState, headsetState, "");
        } catch (Exception ex) {
            return new ProximityReadback(false, "", "", exceptionSummary(ex));
        }
    }

    private static PowerReadback readPowerState() {
        try {
            CommandCapture capture = runBoundedCommand(
                new String[] { "dumpsys", "power" },
                POWER_DUMPSYS_MAX_BYTES,
                POWER_COMMAND_TIMEOUT_SECONDS);
            if (capture.exitCode != 0 || capture.timedOut) {
                String error = capture.timedOut
                    ? "dumpsys power timed out"
                    : "dumpsys power exit=" + capture.exitCode + " " + capture.output.trim();
                return new PowerReadback(false, "", false, "", error);
            }

            String wakefulness = matchFirst(POWER_WAKEFULNESS_PATTERN, capture.output);
            String stayOn = matchFirst(POWER_STAY_ON_PATTERN, capture.output);
            String displayPowerState = matchFirst(POWER_DISPLAY_STATE_PATTERN, capture.output);
            if (wakefulness.length() == 0 && stayOn.length() == 0 && displayPowerState.length() == 0) {
                return new PowerReadback(false, "", false, "", "Power state missing from dumpsys power");
            }
            return new PowerReadback(
                true,
                wakefulness,
                "true".equalsIgnoreCase(stayOn),
                displayPowerState,
                "");
        } catch (Exception ex) {
            return new PowerReadback(false, "", false, "", exceptionSummary(ex));
        }
    }

    private static boolean shouldWakeDevice(PowerReadback power) {
        if (!power.available) {
            return false;
        }
        return (power.wakefulness.length() > 0 && !"Awake".equalsIgnoreCase(power.wakefulness)) ||
            (power.displayPowerState.length() > 0 &&
                !"ON".equalsIgnoreCase(power.displayPowerState) &&
                !"ON_SUSPEND".equalsIgnoreCase(power.displayPowerState));
    }

    private static CommandCapture enableStayAwake() throws Exception {
        return runBoundedCommand(
            new String[] { "svc", "power", "stayon", "true" },
            16 * 1024,
            POWER_COMMAND_TIMEOUT_SECONDS);
    }

    private static CommandCapture wakeDevice() throws Exception {
        return runBoundedCommand(
            new String[] { "input", "keyevent", "224" },
            16 * 1024,
            POWER_COMMAND_TIMEOUT_SECONDS);
    }

    private static CommandCapture broadcastProxClose(long durationMs) throws Exception {
        return runBoundedCommand(
            new String[] {
                "am",
                "broadcast",
                "-a",
                VR_POWER_MANAGER_PROX_CLOSE_ACTION,
                "--ei",
                "duration",
                Long.toString(Math.max(0L, durationMs))
            },
            16 * 1024,
            VR_POWER_MANAGER_COMMAND_TIMEOUT_SECONDS);
    }

    private static String matchFirst(Pattern pattern, String text) {
        Matcher matcher = pattern.matcher(text != null ? text : "");
        return matcher.find() ? matcher.group(1).trim() : "";
    }

    private static void parseCameraDumpsys(String text, JSONObject probe) throws Exception {
        String[] lines = text.split("\\r?\\n");
        JSONArray api1Mappings = new JSONArray();
        JSONArray dynamicCameraIds = new JSONArray();
        JSONArray devices = new JSONArray();
        JSONObject currentDevice = null;
        String pendingArrayKey = null;
        JSONArray pendingArray = null;
        String pendingArrayKind = "";

        Pattern countPattern = Pattern.compile("^Number of all camera devices:\\s*(\\d+)");
        Pattern api1CountPattern = Pattern.compile("^Number of camera devices visible to API1:\\s*(\\d+)");
        Pattern publicApi1CountPattern = Pattern.compile("^Number of public camera devices visible to API1:\\s*(\\d+)");
        Pattern mappingPattern = Pattern.compile("^Device\\s+(\\d+)\\s+maps to\\s+\"([^\"]+)\"");
        Pattern staticDevicePattern = Pattern.compile("== Camera HAL device\\s+([^\\s]+)\\s+\\(([^)]*)\\) static information: ==");
        Pattern dynamicDevicePattern = Pattern.compile("== Camera device\\s+([^\\s]+)\\s+dynamic info: ==");

        for (int lineIndex = 0; lineIndex < lines.length; lineIndex++) {
            String rawLine = lines[lineIndex];
            String line = rawLine.trim();
            if (line.length() == 0) {
                continue;
            }

            if (pendingArray != null) {
                if (line.startsWith("[")) {
                    if ("fps".equals(pendingArrayKind)) {
                        JSONArray row = intRowFromBracketLine(line);
                        if (row.length() > 0 && pendingArray.length() < CAMERA_PROBE_MAX_FPS_ROWS_PER_DEVICE) {
                            pendingArray.put(row);
                        }
                    } else if ("stream_config".equals(pendingArrayKind)) {
                        JSONObject row = streamConfigFromBracketLine(line);
                        if (row != null && pendingArray.length() < CAMERA_PROBE_MAX_STREAM_CONFIGS_PER_DEVICE) {
                            pendingArray.put(row);
                        }
                    } else {
                        JSONArray row = intRowFromBracketLine(line);
                        if (row.length() > 0) {
                            pendingArray.put(row);
                        }
                    }
                    continue;
                }

                if (currentDevice != null) {
                    currentDevice.put(pendingArrayKey, pendingArray);
                }
                pendingArray = null;
                pendingArrayKey = null;
                pendingArrayKind = "";
            }

            Matcher countMatcher = countPattern.matcher(line);
            if (countMatcher.find()) {
                probe.put("camera_count", Integer.parseInt(countMatcher.group(1)));
                continue;
            }
            Matcher api1CountMatcher = api1CountPattern.matcher(line);
            if (api1CountMatcher.find()) {
                probe.put("api1_visible_count", Integer.parseInt(api1CountMatcher.group(1)));
                continue;
            }
            Matcher publicApi1CountMatcher = publicApi1CountPattern.matcher(line);
            if (publicApi1CountMatcher.find()) {
                probe.put("public_api1_visible_count", Integer.parseInt(publicApi1CountMatcher.group(1)));
                continue;
            }
            Matcher mappingMatcher = mappingPattern.matcher(line);
            if (mappingMatcher.find()) {
                JSONObject mapping = new JSONObject();
                mapping.put("api1_device_index", Integer.parseInt(mappingMatcher.group(1)));
                mapping.put("camera_id", mappingMatcher.group(2));
                api1Mappings.put(mapping);
                continue;
            }
            Matcher dynamicDeviceMatcher = dynamicDevicePattern.matcher(line);
            if (dynamicDeviceMatcher.find()) {
                dynamicCameraIds.put(dynamicDeviceMatcher.group(1));
                continue;
            }
            Matcher staticDeviceMatcher = staticDevicePattern.matcher(line);
            if (staticDeviceMatcher.find()) {
                if (currentDevice != null && devices.length() < CAMERA_PROBE_MAX_DEVICES) {
                    devices.put(currentDevice);
                }
                currentDevice = new JSONObject();
                String halName = staticDeviceMatcher.group(1);
                currentDevice.put("hal_device", halName);
                currentDevice.put("camera_id", cameraIdFromHalName(halName));
                currentDevice.put("hal_version", staticDeviceMatcher.group(2));
                continue;
            }

            if (currentDevice == null) {
                continue;
            }

            if (line.startsWith("Resource cost:")) {
                currentDevice.put("resource_cost", parseFirstInt(line));
            } else if (line.startsWith("Conflicting devices:")) {
                currentDevice.put("conflicting_devices", line.substring("Conflicting devices:".length()).trim());
            } else if (line.startsWith("Has a flash unit:")) {
                currentDevice.put("has_flash", line.toLowerCase(Locale.ROOT).contains("true"));
            } else if (line.startsWith("Facing:")) {
                currentDevice.put("api1_facing", line.substring("Facing:".length()).trim());
            } else if (line.startsWith("Orientation:")) {
                currentDevice.put("api1_orientation", parseFirstInt(line));
            } else if (line.startsWith("android.control.aeAvailableTargetFpsRanges")) {
                pendingArrayKey = "ae_available_target_fps_rows";
                pendingArray = new JSONArray();
                pendingArrayKind = "fps";
            } else if (line.startsWith("android.scaler.availableStreamConfigurations")) {
                pendingArrayKey = "stream_configurations";
                pendingArray = new JSONArray();
                pendingArrayKind = "stream_config";
            } else if (line.startsWith("android.lens.facing")) {
                currentDevice.put("lens_facing", nextValueString(lines, lineIndex));
            } else if (line.startsWith("android.lens.poseReference")) {
                currentDevice.put("lens_pose_reference", nextValueString(lines, lineIndex));
            } else if (line.startsWith("android.lens.poseRotation")) {
                currentDevice.put("lens_pose_rotation_xyzw", floatArrayAfterLine(lines, lineIndex, 4));
            } else if (line.startsWith("android.lens.poseTranslation")) {
                currentDevice.put("lens_pose_translation_m", floatArrayAfterLine(lines, lineIndex, 3));
            } else if (line.startsWith("android.lens.intrinsicCalibration")) {
                currentDevice.put("lens_intrinsic_calibration", floatArrayAfterLine(lines, lineIndex, 5));
            } else if (line.startsWith("android.sensor.info.physicalSize")) {
                currentDevice.put("sensor_physical_size", floatArrayAfterLine(lines, lineIndex, 2));
            } else if (line.startsWith("android.sensor.info.pixelArraySize")) {
                currentDevice.put("sensor_pixel_array_size", intArrayAfterLine(lines, lineIndex, 4));
            } else if (line.startsWith("android.sensor.info.activeArraySize")) {
                currentDevice.put("sensor_active_array_size", intArrayAfterLine(lines, lineIndex, 4));
            } else if (line.startsWith("android.info.supportedHardwareLevel")) {
                currentDevice.put("supported_hardware_level", nextValueString(lines, lineIndex));
            }
        }

        if (pendingArray != null && currentDevice != null) {
            currentDevice.put(pendingArrayKey, pendingArray);
        }
        if (currentDevice != null && devices.length() < CAMERA_PROBE_MAX_DEVICES) {
            devices.put(currentDevice);
        }
        probe.put("api1_mappings", api1Mappings);
        probe.put("dynamic_camera_ids", dynamicCameraIds);
        probe.put("devices", devices);
        probe.put("parsed_device_count", devices.length());
    }

    private static String cameraIdFromHalName(String halName) {
        int slash = halName.lastIndexOf('/');
        return slash >= 0 && slash + 1 < halName.length() ? halName.substring(slash + 1) : halName;
    }

    private static JSONArray intRowFromBracketLine(String line) {
        JSONArray array = new JSONArray();
        String inside = bracketContents(line);
        if (inside.length() == 0) {
            return array;
        }
        String[] parts = inside.split("\\s+");
        for (String part : parts) {
            if (part.length() == 0 || !part.matches("-?\\d+")) {
                continue;
            }
            array.put(Integer.parseInt(part));
        }
        return array;
    }

    private static JSONObject streamConfigFromBracketLine(String line) throws Exception {
        String inside = bracketContents(line);
        if (inside.length() == 0) {
            return null;
        }
        String[] parts = inside.split("\\s+");
        if (parts.length < 4) {
            return null;
        }
        JSONObject config = new JSONObject();
        config.put("format", Integer.parseInt(parts[0]));
        config.put("format_name", imageFormatName(Integer.parseInt(parts[0])));
        config.put("width", Integer.parseInt(parts[1]));
        config.put("height", Integer.parseInt(parts[2]));
        config.put("direction", parts[3]);
        return config;
    }

    private static String imageFormatName(int format) {
        if (format == 34) {
            return "PRIVATE";
        }
        if (format == 35) {
            return "YUV_420_888";
        }
        if (format == 33) {
            return "BLOB";
        }
        return "format_" + format;
    }

    private static String bracketContents(String line) {
        int start = line.indexOf('[');
        int end = line.indexOf(']', start + 1);
        if (start < 0 || end <= start) {
            return "";
        }
        return line.substring(start + 1, end).trim();
    }

    private static int parseFirstInt(String line) {
        Matcher matcher = Pattern.compile("-?\\d+").matcher(line);
        return matcher.find() ? Integer.parseInt(matcher.group()) : 0;
    }

    private static String nextValueString(String[] lines, int index) {
        if (index < 0 || index + 1 >= lines.length) {
            return "";
        }
        String next = lines[index + 1].trim();
        String inside = bracketContents(next);
        return inside.length() == 0 ? next : inside;
    }

    private static JSONArray floatArrayAfterLine(String[] lines, int index, int limit) throws Exception {
        JSONArray array = new JSONArray();
        if (index < 0 || index + 1 >= lines.length) {
            return array;
        }
        String inside = bracketContents(lines[index + 1].trim());
        if (inside.length() == 0) {
            return array;
        }
        String[] parts = inside.split("\\s+");
        for (int i = 0; i < parts.length && i < limit; i++) {
            if (parts[i].length() == 0) {
                continue;
            }
            try {
                array.put(Double.parseDouble(parts[i]));
            } catch (NumberFormatException ignored) {
            }
        }
        return array;
    }

    private static JSONArray intArrayAfterLine(String[] lines, int index, int limit) {
        JSONArray array = new JSONArray();
        if (index < 0 || index + 1 >= lines.length) {
            return array;
        }
        String inside = bracketContents(lines[index + 1].trim());
        if (inside.length() == 0) {
            return array;
        }
        String[] parts = inside.split("\\s+");
        for (int i = 0; i < parts.length && i < limit; i++) {
            if (parts[i].length() == 0 || !parts[i].matches("-?\\d+")) {
                continue;
            }
            array.put(Integer.parseInt(parts[i]));
        }
        return array;
    }

    private static boolean containsColorFormat(int[] values, int target) {
        if (values == null) {
            return false;
        }
        for (int value : values) {
            if (value == target) {
                return true;
            }
        }
        return false;
    }

    private static JSONArray intArrayJson(int[] values, int limit) {
        JSONArray array = new JSONArray();
        if (values == null) {
            return array;
        }
        for (int i = 0; i < values.length && i < limit; i++) {
            array.put(values[i]);
        }
        return array;
    }

    private static JSONArray stringArrayJson(String[] values, int limit) {
        JSONArray array = new JSONArray();
        if (values == null) {
            return array;
        }
        for (int i = 0; i < values.length && i < limit; i++) {
            array.put(values[i]);
        }
        return array;
    }

    private static JSONArray profileLevelsJson(MediaCodecInfo.CodecProfileLevel[] levels, int limit) throws Exception {
        JSONArray array = new JSONArray();
        if (levels == null) {
            return array;
        }
        for (int i = 0; i < levels.length && i < limit; i++) {
            JSONObject item = new JSONObject();
            item.put("profile", levels[i].profile);
            item.put("level", levels[i].level);
            array.put(item);
        }
        return array;
    }

    private static JSONObject sendBrokerCommand(String host, int port, String command, JSONObject params) throws Exception {
        Socket socket = new Socket(host, port);
        try {
            socket.setSoTimeout(5000);
            InputStream input = socket.getInputStream();
            OutputStream output = socket.getOutputStream();
            writeHandshake(output, host, port);
            String headers = readHeaders(input);
            if (!headers.startsWith("HTTP/1.1 101")) {
                throw new IllegalStateException("Broker did not accept WebSocket upgrade: " + headers);
            }

            readTextFrame(input); // Initial broker status frame.
            JSONObject message = new JSONObject();
            message.put("type", "command");
            message.put("schema", COMMAND_SCHEMA);
            message.put("request_id", "shell-helper-" + System.currentTimeMillis());
            message.put("client_id", CLIENT_ID);
            message.put("app_label", "Rusty XR ADB Shell Helper");
            message.put("app_version", VERSION);
            message.put("command", command);
            message.put("params", params);
            writeClientTextFrame(output, message.toString());
            return new JSONObject(readTextFrame(input));
        } finally {
            socket.close();
        }
    }

    private static void writeHandshake(OutputStream output, String host, int port) throws Exception {
        String request =
            "GET " + EVENTS_PATH + " HTTP/1.1\r\n" +
            "Host: " + host + ":" + port + "\r\n" +
            "Upgrade: websocket\r\n" +
            "Connection: Upgrade\r\n" +
            "Sec-WebSocket-Key: ZHVtbXlfa2V5XzEyMzQ1Ng==\r\n" +
            "Sec-WebSocket-Version: 13\r\n" +
            "\r\n";
        output.write(request.getBytes(StandardCharsets.US_ASCII));
        output.flush();
    }

    private static String readHeaders(InputStream input) throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        int previous3 = -1;
        int previous2 = -1;
        int previous1 = -1;
        while (true) {
            int next = input.read();
            if (next < 0) {
                throw new EOFException("EOF while reading HTTP headers");
            }
            bytes.write(next);
            if (previous3 == '\r' && previous2 == '\n' && previous1 == '\r' && next == '\n') {
                break;
            }
            if (bytes.size() > 16 * 1024) {
                throw new IllegalStateException("HTTP header block too large");
            }
            previous3 = previous2;
            previous2 = previous1;
            previous1 = next;
        }
        return bytes.toString("US-ASCII");
    }

    private static String readTextFrame(InputStream input) throws Exception {
        int first = input.read();
        int second = input.read();
        if (first < 0 || second < 0) {
            throw new EOFException("EOF while reading WebSocket frame header");
        }
        int opcode = first & 0x0f;
        if (opcode == 8) {
            throw new EOFException("Broker closed WebSocket");
        }
        if (opcode != 1) {
            throw new IllegalStateException("Expected text frame, opcode=" + opcode);
        }

        boolean masked = (second & 0x80) != 0;
        long length = second & 0x7f;
        if (length == 126) {
            length = (readByte(input) << 8) | readByte(input);
        } else if (length == 127) {
            length = 0;
            for (int i = 0; i < 8; i++) {
                length = (length << 8) | readByte(input);
            }
        }
        if (length > 1024 * 1024) {
            throw new IllegalStateException("WebSocket frame too large: " + length);
        }

        byte[] mask = new byte[4];
        if (masked) {
            readFully(input, mask);
        }
        byte[] payload = new byte[(int) length];
        readFully(input, payload);
        if (masked) {
            for (int i = 0; i < payload.length; i++) {
                payload[i] = (byte) (payload[i] ^ mask[i % mask.length]);
            }
        }
        return new String(payload, StandardCharsets.UTF_8);
    }

    private static void writeClientTextFrame(OutputStream output, String text) throws Exception {
        byte[] payload = text.getBytes(StandardCharsets.UTF_8);
        ByteArrayOutputStream frame = new ByteArrayOutputStream(payload.length + 16);
        frame.write(0x81);
        if (payload.length <= 125) {
            frame.write(0x80 | payload.length);
        } else if (payload.length <= 0xffff) {
            frame.write(0x80 | 126);
            frame.write((payload.length >> 8) & 0xff);
            frame.write(payload.length & 0xff);
        } else {
            frame.write(0x80 | 127);
            long length = payload.length;
            for (int i = 7; i >= 0; i--) {
                frame.write((int) ((length >> (i * 8)) & 0xff));
            }
        }

        byte[] mask = new byte[] { 0x12, 0x34, 0x56, 0x78 };
        frame.write(mask);
        for (int i = 0; i < payload.length; i++) {
            frame.write(payload[i] ^ mask[i % mask.length]);
        }
        output.write(frame.toByteArray());
        output.flush();
    }

    private static void writeU32(OutputStream output, int value) throws Exception {
        output.write((value >> 24) & 0xff);
        output.write((value >> 16) & 0xff);
        output.write((value >> 8) & 0xff);
        output.write(value & 0xff);
    }

    private static void writeU64(OutputStream output, long value) throws Exception {
        for (int i = 7; i >= 0; i--) {
            output.write((int) ((value >> (i * 8)) & 0xff));
        }
    }

    private static int readByte(InputStream input) throws Exception {
        int value = input.read();
        if (value < 0) {
            throw new EOFException("EOF while reading byte");
        }
        return value & 0xff;
    }

    private static void readFully(InputStream input, byte[] buffer) throws Exception {
        int offset = 0;
        while (offset < buffer.length) {
            int count = input.read(buffer, offset, buffer.length - offset);
            if (count < 0) {
                throw new EOFException("EOF while reading payload");
            }
            offset += count;
        }
    }

    private static final class EncodedPacket {
        final long ptsUs;
        final int flags;
        final byte[] payload;

        EncodedPacket(long ptsUs, int flags, byte[] payload) {
            this.ptsUs = ptsUs;
            this.flags = flags;
            this.payload = payload;
        }
    }

    private static final class CameraManagerCreateResult {
        final CameraManager manager;
        final String constructorSignature;
        final String strategy;
        final JSONArray constructorSignatures;

        CameraManagerCreateResult(
                CameraManager manager,
                String constructorSignature,
                String strategy,
                JSONArray constructorSignatures) {
            this.manager = manager;
            this.constructorSignature = constructorSignature;
            this.strategy = strategy;
            this.constructorSignatures = constructorSignatures;
        }
    }

    private static final class ShellContext extends ContextWrapper {
        ShellContext() {
            super(systemContextOrNull());
        }

        @Override
        public String getPackageName() {
            return "com.android.shell";
        }

        @Override
        public String getOpPackageName() {
            return "com.android.shell";
        }

        @Override
        public Context getApplicationContext() {
            return this;
        }

        @Override
        public PackageManager getPackageManager() {
            Context base = getBaseContext();
            if (base != null) {
                return base.getPackageManager();
            }
            return super.getPackageManager();
        }

        @Override
        public Object getSystemService(String name) {
            return null;
        }

        @Override
        public String getSystemServiceName(Class<?> serviceClass) {
            return null;
        }

        @Override
        public int checkSelfPermission(String permission) {
            return PackageManager.PERMISSION_GRANTED;
        }

        @Override
        public int checkPermission(String permission, int pid, int uid) {
            return PackageManager.PERMISSION_GRANTED;
        }

        @Override
        public int checkCallingPermission(String permission) {
            return PackageManager.PERMISSION_GRANTED;
        }

        @Override
        public int checkCallingOrSelfPermission(String permission) {
            return PackageManager.PERMISSION_GRANTED;
        }

        @Override
        public AttributionSource getAttributionSource() {
            return AttributionSource.myAttributionSource();
        }

        private static Context systemContextOrNull() {
            try {
                Class<?> activityThreadClass = Class.forName("android.app.ActivityThread");
                Method currentActivityThread = activityThreadClass.getDeclaredMethod("currentActivityThread");
                currentActivityThread.setAccessible(true);
                Object activityThread = currentActivityThread.invoke(null);
                if (activityThread == null) {
                    Method systemMain = activityThreadClass.getDeclaredMethod("systemMain");
                    systemMain.setAccessible(true);
                    activityThread = systemMain.invoke(null);
                }
                if (activityThread == null) {
                    return null;
                }
                Method getSystemContext = activityThreadClass.getDeclaredMethod("getSystemContext");
                getSystemContext.setAccessible(true);
                Object context = getSystemContext.invoke(activityThread);
                if (context instanceof Context) {
                    return (Context) context;
                }
                Class<?> contextImplClass = Class.forName("android.app.ContextImpl");
                Method createSystemContext = contextImplClass.getDeclaredMethod(
                    "createSystemContext",
                    activityThreadClass);
                createSystemContext.setAccessible(true);
                Object createdContext = createSystemContext.invoke(null, activityThread);
                return createdContext instanceof Context ? (Context) createdContext : null;
            } catch (Exception ignored) {
                return null;
            }
        }
    }

    private static final class CommandCapture {
        final String output;
        final int outputBytes;
        final boolean truncated;
        final boolean timedOut;
        final int exitCode;
        final String error;

        CommandCapture(
                String output,
                int outputBytes,
                boolean truncated,
                boolean timedOut,
                int exitCode,
                String error) {
            this.output = output;
            this.outputBytes = outputBytes;
            this.truncated = truncated;
            this.timedOut = timedOut;
            this.exitCode = exitCode;
            this.error = error;
        }
    }

    private static final class StreamWriteStats {
        final long listenStartElapsedNs;
        final long acceptElapsedNs;
        final long writeStartElapsedNs;
        final long writeEndElapsedNs;

        StreamWriteStats(
                long listenStartElapsedNs,
                long acceptElapsedNs,
                long writeStartElapsedNs,
                long writeEndElapsedNs) {
            this.listenStartElapsedNs = listenStartElapsedNs;
            this.acceptElapsedNs = acceptElapsedNs;
            this.writeStartElapsedNs = writeStartElapsedNs;
            this.writeEndElapsedNs = writeEndElapsedNs;
        }
    }

    private static final class ProximityReadback {
        final boolean available;
        final String virtualState;
        final String headsetState;
        final String error;

        ProximityReadback(boolean available, String virtualState, String headsetState, String error) {
            this.available = available;
            this.virtualState = virtualState != null ? virtualState : "";
            this.headsetState = headsetState != null ? headsetState : "";
            this.error = error != null ? error : "";
        }
    }

    private static final class PowerReadback {
        final boolean available;
        final String wakefulness;
        final boolean stayOn;
        final String displayPowerState;
        final String error;

        PowerReadback(
                boolean available,
                String wakefulness,
                boolean stayOn,
                String displayPowerState,
                String error) {
            this.available = available;
            this.wakefulness = wakefulness != null ? wakefulness : "";
            this.stayOn = stayOn;
            this.displayPowerState = displayPowerState != null ? displayPowerState : "";
            this.error = error != null ? error : "";
        }
    }

    private static final class ForegroundReadback {
        final boolean available;
        final String packageName;
        final String activityName;
        final boolean metaHome;
        final boolean visibleMetaMenuOverlay;
        final boolean protectedSystemFlow;
        final String error;

        ForegroundReadback(
                boolean available,
                String packageName,
                String activityName,
                boolean metaHome,
                boolean visibleMetaMenuOverlay,
                boolean protectedSystemFlow,
                String error) {
            this.available = available;
            this.packageName = packageName != null ? packageName : "";
            this.activityName = activityName != null ? activityName : "";
            this.metaHome = metaHome;
            this.visibleMetaMenuOverlay = visibleMetaMenuOverlay;
            this.protectedSystemFlow = protectedSystemFlow;
            this.error = error != null ? error : "";
        }
    }

    private static final class Options {
        final String host;
        final int port;
        final boolean connected;
        final boolean noBrokerReport;
        final boolean probeCodecs;
        final boolean probeCameras;
        final boolean probeCameraOpen;
        final String cameraOpenId;
        final boolean captureCameraFrame;
        final String cameraFrameOutputDir;
        final int cameraFrameJpegQuality;
        final int syntheticVideoSamples;
        final boolean emitSyntheticVideoBinary;
        final int syntheticVideoBinaryPort;
        final int syntheticVideoPackets;
        final int syntheticVideoPacketBytes;
        final boolean emitMediaCodecSyntheticVideo;
        final boolean emitScreenrecordVideo;
        final int encodedVideoFrames;
        final int encodedVideoWidth;
        final int encodedVideoHeight;
        final int encodedVideoBitrateBps;
        final int screenrecordTimeLimitSeconds;
        final boolean proximityWatchdog;
        final boolean stopProximityWatchdog;
        final boolean proximityWatchdogUntilStopped;
        final boolean proximityWatchdogEnsureStayAwake;
        final int proximityWatchdogDurationMs;
        final int proximityWatchdogHoldDurationMs;
        final int proximityWatchdogIntervalMs;
        final boolean focusGuardian;
        final boolean stopFocusGuardian;
        final String focusGuardianMode;
        final String focusGuardianDesiredFocus;
        final String focusTargetPackage;
        final String focusTargetActivity;
        final String focusBrokerPackage;
        final String focusBrokerActivity;
        final int focusGuardianDurationMs;
        final int focusGuardianIntervalMs;
        final int focusGuardianCooldownMs;

        Options(
                String host,
                int port,
                boolean connected,
                boolean noBrokerReport,
                boolean probeCodecs,
                boolean probeCameras,
                boolean probeCameraOpen,
                String cameraOpenId,
                boolean captureCameraFrame,
                String cameraFrameOutputDir,
                int cameraFrameJpegQuality,
                int syntheticVideoSamples,
                boolean emitSyntheticVideoBinary,
                int syntheticVideoBinaryPort,
                int syntheticVideoPackets,
                int syntheticVideoPacketBytes,
                boolean emitMediaCodecSyntheticVideo,
                boolean emitScreenrecordVideo,
                int encodedVideoFrames,
                int encodedVideoWidth,
                int encodedVideoHeight,
                int encodedVideoBitrateBps,
                int screenrecordTimeLimitSeconds,
                boolean proximityWatchdog,
                boolean stopProximityWatchdog,
                boolean proximityWatchdogUntilStopped,
                boolean proximityWatchdogEnsureStayAwake,
                int proximityWatchdogDurationMs,
                int proximityWatchdogHoldDurationMs,
                int proximityWatchdogIntervalMs,
                boolean focusGuardian,
                boolean stopFocusGuardian,
                String focusGuardianMode,
                String focusGuardianDesiredFocus,
                String focusTargetPackage,
                String focusTargetActivity,
                String focusBrokerPackage,
                String focusBrokerActivity,
                int focusGuardianDurationMs,
                int focusGuardianIntervalMs,
                int focusGuardianCooldownMs) {
            this.host = host;
            this.port = port;
            this.connected = connected;
            this.noBrokerReport = noBrokerReport;
            this.probeCodecs = probeCodecs;
            this.probeCameras = probeCameras;
            this.probeCameraOpen = probeCameraOpen;
            this.cameraOpenId = cameraOpenId;
            this.captureCameraFrame = captureCameraFrame;
            this.cameraFrameOutputDir = cameraFrameOutputDir != null && cameraFrameOutputDir.length() > 0
                ? cameraFrameOutputDir
                : CAMERA_FRAME_CAPTURE_DEFAULT_DIR;
            this.cameraFrameJpegQuality = cameraFrameJpegQuality;
            this.syntheticVideoSamples = syntheticVideoSamples;
            this.emitSyntheticVideoBinary = emitSyntheticVideoBinary;
            this.syntheticVideoBinaryPort = syntheticVideoBinaryPort;
            this.syntheticVideoPackets = syntheticVideoPackets;
            this.syntheticVideoPacketBytes = syntheticVideoPacketBytes;
            this.emitMediaCodecSyntheticVideo = emitMediaCodecSyntheticVideo;
            this.emitScreenrecordVideo = emitScreenrecordVideo;
            this.encodedVideoFrames = encodedVideoFrames;
            this.encodedVideoWidth = encodedVideoWidth;
            this.encodedVideoHeight = encodedVideoHeight;
            this.encodedVideoBitrateBps = encodedVideoBitrateBps;
            this.screenrecordTimeLimitSeconds = screenrecordTimeLimitSeconds;
            this.proximityWatchdog = proximityWatchdog;
            this.stopProximityWatchdog = stopProximityWatchdog;
            this.proximityWatchdogUntilStopped = proximityWatchdogUntilStopped;
            this.proximityWatchdogEnsureStayAwake = proximityWatchdogEnsureStayAwake;
            this.proximityWatchdogDurationMs = proximityWatchdogDurationMs;
            this.proximityWatchdogHoldDurationMs = proximityWatchdogHoldDurationMs;
            this.proximityWatchdogIntervalMs = proximityWatchdogIntervalMs;
            this.focusGuardian = focusGuardian;
            this.stopFocusGuardian = stopFocusGuardian;
            this.focusGuardianMode = focusGuardianMode != null ? focusGuardianMode : "observe";
            this.focusGuardianDesiredFocus = focusGuardianDesiredFocus != null ? focusGuardianDesiredFocus : "broker";
            this.focusTargetPackage = focusTargetPackage != null ? focusTargetPackage : "";
            this.focusTargetActivity = focusTargetActivity != null ? focusTargetActivity : "";
            this.focusBrokerPackage = focusBrokerPackage != null && focusBrokerPackage.length() > 0
                ? focusBrokerPackage
                : DEFAULT_BROKER_PACKAGE;
            this.focusBrokerActivity = focusBrokerActivity != null && focusBrokerActivity.length() > 0
                ? focusBrokerActivity
                : DEFAULT_BROKER_ACTIVITY;
            this.focusGuardianDurationMs = focusGuardianDurationMs;
            this.focusGuardianIntervalMs = focusGuardianIntervalMs;
            this.focusGuardianCooldownMs = focusGuardianCooldownMs;
        }

        static Options parse(String[] args) {
            String host = "127.0.0.1";
            int port = 8765;
            boolean connected = true;
            boolean noBrokerReport = false;
            boolean probeCodecs = false;
            boolean probeCameras = false;
            boolean probeCameraOpen = false;
            String cameraOpenId = "";
            boolean captureCameraFrame = false;
            String cameraFrameOutputDir = CAMERA_FRAME_CAPTURE_DEFAULT_DIR;
            int cameraFrameJpegQuality = CAMERA_FRAME_CAPTURE_DEFAULT_JPEG_QUALITY;
            int syntheticVideoSamples = 0;
            boolean emitSyntheticVideoBinary = false;
            int syntheticVideoBinaryPort = SYNTHETIC_BINARY_DEFAULT_PORT;
            int syntheticVideoPackets = SYNTHETIC_BINARY_DEFAULT_PACKET_COUNT;
            int syntheticVideoPacketBytes = SYNTHETIC_BINARY_DEFAULT_PACKET_BYTES;
            boolean emitMediaCodecSyntheticVideo = false;
            boolean emitScreenrecordVideo = false;
            int encodedVideoFrames = MEDIACODEC_DEFAULT_FRAMES;
            int encodedVideoWidth = MEDIACODEC_DEFAULT_WIDTH;
            int encodedVideoHeight = MEDIACODEC_DEFAULT_HEIGHT;
            int encodedVideoBitrateBps = MEDIACODEC_DEFAULT_BITRATE_BPS;
            int screenrecordTimeLimitSeconds = SCREENRECORD_DEFAULT_SECONDS;
            boolean proximityWatchdog = false;
            boolean stopProximityWatchdog = false;
            boolean proximityWatchdogUntilStopped = false;
            boolean proximityWatchdogEnsureStayAwake = false;
            int proximityWatchdogDurationMs = PROXIMITY_WATCHDOG_DEFAULT_DURATION_MS;
            int proximityWatchdogHoldDurationMs = PROXIMITY_WATCHDOG_DEFAULT_HOLD_MS;
            int proximityWatchdogIntervalMs = PROXIMITY_WATCHDOG_DEFAULT_INTERVAL_MS;
            boolean focusGuardian = false;
            boolean stopFocusGuardian = false;
            String focusGuardianMode = "observe";
            String focusGuardianDesiredFocus = "broker";
            String focusTargetPackage = "";
            String focusTargetActivity = "";
            String focusBrokerPackage = DEFAULT_BROKER_PACKAGE;
            String focusBrokerActivity = DEFAULT_BROKER_ACTIVITY;
            int focusGuardianDurationMs = FOCUS_GUARDIAN_DEFAULT_DURATION_MS;
            int focusGuardianIntervalMs = FOCUS_GUARDIAN_DEFAULT_INTERVAL_MS;
            int focusGuardianCooldownMs = FOCUS_GUARDIAN_DEFAULT_COOLDOWN_MS;
            for (int i = 0; i < args.length; i++) {
                String arg = args[i];
                if ("--broker-host".equals(arg) && i + 1 < args.length) {
                    host = args[++i];
                } else if ("--broker-port".equals(arg) && i + 1 < args.length) {
                    port = Integer.parseInt(args[++i]);
                } else if ("--disconnect".equals(arg)) {
                    connected = false;
                } else if ("--no-broker-report".equals(arg)) {
                    connected = false;
                    noBrokerReport = true;
                } else if ("--probe-codecs".equals(arg)) {
                    probeCodecs = true;
                } else if ("--probe-cameras".equals(arg)) {
                    probeCameras = true;
                } else if ("--probe-camera-open".equals(arg)) {
                    probeCameraOpen = true;
                } else if ("--camera-open-id".equals(arg) && i + 1 < args.length) {
                    cameraOpenId = args[++i];
                } else if ("--capture-camera-frame".equals(arg)) {
                    captureCameraFrame = true;
                    probeCameraOpen = true;
                } else if ("--camera-frame-output-dir".equals(arg) && i + 1 < args.length) {
                    cameraFrameOutputDir = args[++i];
                } else if ("--camera-frame-jpeg-quality".equals(arg) && i + 1 < args.length) {
                    cameraFrameJpegQuality = parsePositiveBounded(
                        "--camera-frame-jpeg-quality",
                        args[++i],
                        1,
                        100);
                } else if ("--emit-synthetic-video-metadata".equals(arg)) {
                    syntheticVideoSamples = Math.max(syntheticVideoSamples, 3);
                } else if ("--synthetic-video-samples".equals(arg) && i + 1 < args.length) {
                    syntheticVideoSamples = parseSyntheticSampleCount(args[++i]);
                } else if ("--emit-synthetic-video-binary".equals(arg)) {
                    emitSyntheticVideoBinary = true;
                } else if ("--binary-video-port".equals(arg) && i + 1 < args.length) {
                    syntheticVideoBinaryPort = parsePort("--binary-video-port", args[++i]);
                } else if ("--binary-video-packets".equals(arg) && i + 1 < args.length) {
                    syntheticVideoPackets = parseSyntheticPacketCount(args[++i]);
                } else if ("--binary-video-packet-bytes".equals(arg) && i + 1 < args.length) {
                    syntheticVideoPacketBytes = parseSyntheticPacketBytes(args[++i]);
                } else if ("--emit-mediacodec-synthetic-video".equals(arg)) {
                    emitMediaCodecSyntheticVideo = true;
                } else if ("--emit-screenrecord-video".equals(arg)) {
                    emitScreenrecordVideo = true;
                } else if ("--encoded-video-frames".equals(arg) && i + 1 < args.length) {
                    encodedVideoFrames = parseEncodedFrameCount(args[++i]);
                } else if ("--encoded-video-width".equals(arg) && i + 1 < args.length) {
                    encodedVideoWidth = parsePositiveBounded("--encoded-video-width", args[++i], 16, 4096);
                } else if ("--encoded-video-height".equals(arg) && i + 1 < args.length) {
                    encodedVideoHeight = parsePositiveBounded("--encoded-video-height", args[++i], 16, 4096);
                } else if ("--encoded-video-bitrate".equals(arg) && i + 1 < args.length) {
                    encodedVideoBitrateBps = parsePositiveBounded("--encoded-video-bitrate", args[++i], 1000, 100000000);
                } else if ("--screenrecord-time-limit".equals(arg) && i + 1 < args.length) {
                    screenrecordTimeLimitSeconds = parsePositiveBounded(
                        "--screenrecord-time-limit",
                        args[++i],
                        1,
                        SCREENRECORD_MAX_SECONDS);
                } else if ("--proximity-watchdog".equals(arg)) {
                    proximityWatchdog = true;
                } else if ("--stop-proximity-watchdog".equals(arg)) {
                    stopProximityWatchdog = true;
                } else if ("--proximity-watchdog-until-stopped".equals(arg)) {
                    proximityWatchdogUntilStopped = true;
                } else if ("--proximity-watchdog-ensure-stay-awake".equals(arg)) {
                    proximityWatchdogEnsureStayAwake = true;
                } else if ("--proximity-watchdog-duration-ms".equals(arg) && i + 1 < args.length) {
                    proximityWatchdogDurationMs = parsePositiveBounded(
                        "--proximity-watchdog-duration-ms",
                        args[++i],
                        PROXIMITY_WATCHDOG_MIN_INTERVAL_MS,
                        Integer.MAX_VALUE);
                } else if ("--proximity-watchdog-hold-duration-ms".equals(arg) && i + 1 < args.length) {
                    proximityWatchdogHoldDurationMs = parsePositiveBounded(
                        "--proximity-watchdog-hold-duration-ms",
                        args[++i],
                        PROXIMITY_WATCHDOG_MIN_INTERVAL_MS,
                        Integer.MAX_VALUE);
                } else if ("--proximity-watchdog-interval-ms".equals(arg) && i + 1 < args.length) {
                    proximityWatchdogIntervalMs = parsePositiveBounded(
                        "--proximity-watchdog-interval-ms",
                        args[++i],
                        PROXIMITY_WATCHDOG_MIN_INTERVAL_MS,
                        PROXIMITY_WATCHDOG_MAX_INTERVAL_MS);
                } else if ("--focus-guardian".equals(arg)) {
                    focusGuardian = true;
                } else if ("--stop-focus-guardian".equals(arg)) {
                    stopFocusGuardian = true;
                } else if ("--focus-guardian-mode".equals(arg) && i + 1 < args.length) {
                    focusGuardianMode = normalizeFocusGuardianMode(args[++i]);
                } else if ("--focus-guardian-desired-focus".equals(arg) && i + 1 < args.length) {
                    focusGuardianDesiredFocus = normalizeFocusSide(args[++i]);
                } else if ("--focus-target-package".equals(arg) && i + 1 < args.length) {
                    focusTargetPackage = args[++i];
                } else if ("--focus-target-activity".equals(arg) && i + 1 < args.length) {
                    focusTargetActivity = args[++i];
                } else if ("--focus-broker-package".equals(arg) && i + 1 < args.length) {
                    focusBrokerPackage = args[++i];
                } else if ("--focus-broker-activity".equals(arg) && i + 1 < args.length) {
                    focusBrokerActivity = args[++i];
                } else if ("--focus-guardian-duration-ms".equals(arg) && i + 1 < args.length) {
                    focusGuardianDurationMs = parsePositiveBounded(
                        "--focus-guardian-duration-ms",
                        args[++i],
                        FOCUS_GUARDIAN_MIN_INTERVAL_MS,
                        Integer.MAX_VALUE);
                } else if ("--focus-guardian-interval-ms".equals(arg) && i + 1 < args.length) {
                    focusGuardianIntervalMs = parsePositiveBounded(
                        "--focus-guardian-interval-ms",
                        args[++i],
                        FOCUS_GUARDIAN_MIN_INTERVAL_MS,
                        FOCUS_GUARDIAN_MAX_INTERVAL_MS);
                } else if ("--focus-guardian-cooldown-ms".equals(arg) && i + 1 < args.length) {
                    focusGuardianCooldownMs = parsePositiveBounded(
                        "--focus-guardian-cooldown-ms",
                        args[++i],
                        FOCUS_GUARDIAN_MIN_INTERVAL_MS,
                        FOCUS_GUARDIAN_MAX_INTERVAL_MS);
                } else if ("--help".equals(arg) || "-h".equals(arg)) {
                    printHelpAndExit();
                } else {
                    throw new IllegalArgumentException(String.format(Locale.ROOT, "Unknown argument: %s", arg));
                }
            }
            if (emitScreenrecordVideo && syntheticVideoPackets == SYNTHETIC_BINARY_DEFAULT_PACKET_COUNT) {
                syntheticVideoPackets = SYNTHETIC_BINARY_MAX_PACKET_COUNT;
            }
            if (emitScreenrecordVideo && syntheticVideoPacketBytes == SYNTHETIC_BINARY_DEFAULT_PACKET_BYTES) {
                syntheticVideoPacketBytes = SCREENRECORD_DEFAULT_PACKET_BYTES;
            }
            return new Options(
                host,
                port,
                connected,
                noBrokerReport,
                probeCodecs,
                probeCameras,
                probeCameraOpen,
                cameraOpenId,
                captureCameraFrame,
                cameraFrameOutputDir,
                cameraFrameJpegQuality,
                syntheticVideoSamples,
                emitSyntheticVideoBinary,
                syntheticVideoBinaryPort,
                syntheticVideoPackets,
                syntheticVideoPacketBytes,
                emitMediaCodecSyntheticVideo,
                emitScreenrecordVideo,
                encodedVideoFrames,
                encodedVideoWidth,
                encodedVideoHeight,
                encodedVideoBitrateBps,
                screenrecordTimeLimitSeconds,
                proximityWatchdog,
                stopProximityWatchdog,
                proximityWatchdogUntilStopped,
                proximityWatchdogEnsureStayAwake,
                proximityWatchdogDurationMs,
                proximityWatchdogHoldDurationMs,
                proximityWatchdogIntervalMs,
                focusGuardian,
                stopFocusGuardian,
                focusGuardianMode,
                focusGuardianDesiredFocus,
                focusTargetPackage,
                focusTargetActivity,
                focusBrokerPackage,
                focusBrokerActivity,
                focusGuardianDurationMs,
                focusGuardianIntervalMs,
                focusGuardianCooldownMs);
        }

        private static int parseSyntheticSampleCount(String value) {
            int count = Integer.parseInt(value);
            if (count < 0) {
                throw new IllegalArgumentException("--synthetic-video-samples must be non-negative");
            }
            if (count > 30) {
                throw new IllegalArgumentException("--synthetic-video-samples is bounded to 30");
            }
            return count;
        }

        private static int parseSyntheticPacketCount(String value) {
            int count = Integer.parseInt(value);
            if (count <= 0) {
                throw new IllegalArgumentException("--binary-video-packets must be positive");
            }
            if (count > SYNTHETIC_BINARY_MAX_PACKET_COUNT) {
                throw new IllegalArgumentException("--binary-video-packets is bounded to 30");
            }
            return count;
        }

        private static int parseSyntheticPacketBytes(String value) {
            int byteCount = Integer.parseInt(value);
            if (byteCount <= 0) {
                throw new IllegalArgumentException("--binary-video-packet-bytes must be positive");
            }
            if (byteCount > SYNTHETIC_BINARY_MAX_PACKET_BYTES) {
                throw new IllegalArgumentException("--binary-video-packet-bytes is bounded to 65536");
            }
            return byteCount;
        }

        private static int parseEncodedFrameCount(String value) {
            int count = Integer.parseInt(value);
            if (count <= 0) {
                throw new IllegalArgumentException("--encoded-video-frames must be positive");
            }
            if (count > MEDIACODEC_MAX_FRAMES) {
                throw new IllegalArgumentException("--encoded-video-frames is bounded to 60");
            }
            return count;
        }

        private static int parsePositiveBounded(String name, String value, int min, int max) {
            int parsed = Integer.parseInt(value);
            if (parsed < min || parsed > max) {
                throw new IllegalArgumentException(name + " must be between " + min + " and " + max);
            }
            return parsed;
        }

        private static int parsePort(String name, String value) {
            int parsed = Integer.parseInt(value);
            if (parsed <= 0 || parsed > 65535) {
                throw new IllegalArgumentException(name + " must be between 1 and 65535");
            }
            return parsed;
        }

        private static String normalizeFocusGuardianMode(String value) {
            String normalized = value != null ? value.trim().toLowerCase(Locale.ROOT) : "";
            if ("off".equals(normalized) ||
                "observe".equals(normalized) ||
                "recover_target".equals(normalized) ||
                "recover_broker".equals(normalized) ||
                "toggle_broker_target".equals(normalized) ||
                "launch_target_guard".equals(normalized) ||
                "strict".equals(normalized)) {
                return normalized;
            }
            throw new IllegalArgumentException("--focus-guardian-mode is invalid: " + value);
        }

        private static String normalizeFocusSide(String value) {
            String normalized = value != null ? value.trim().toLowerCase(Locale.ROOT) : "";
            if ("target".equals(normalized) || "broker".equals(normalized)) {
                return normalized;
            }
            throw new IllegalArgumentException("--focus-guardian-desired-focus must be target or broker");
        }

        private static void printHelpAndExit() {
            System.out.println("Rusty XR broker shell helper");
            System.out.println("  --broker-host <host>  default 127.0.0.1");
            System.out.println("  --broker-port <port>  default 8765");
            System.out.println("  --disconnect          report connected=false");
            System.out.println("  --no-broker-report    print local report without opening the broker WebSocket");
            System.out.println("  --probe-codecs        report bounded MediaCodec H.264/H.265/AV1 capabilities");
            System.out.println("  --probe-cameras       report bounded shell-visible camera metadata from dumpsys");
            System.out.println("  --probe-camera-open   attempt bounded shell Camera2 open plus one YUV capture");
            System.out.println("  --camera-open-id <id> restrict --probe-camera-open to one Camera2 id");
            System.out.println("  --capture-camera-frame");
            System.out.println("                        persist the captured Camera2 YUV frame as NV21 plus a JPEG preview");
            System.out.println("  --camera-frame-output-dir <path>");
            System.out.println("                        device-local output directory; default " + CAMERA_FRAME_CAPTURE_DEFAULT_DIR);
            System.out.println("  --camera-frame-jpeg-quality <1-100>");
            System.out.println("                        JPEG preview quality; default " + CAMERA_FRAME_CAPTURE_DEFAULT_JPEG_QUALITY);
            System.out.println("  --emit-synthetic-video-metadata");
            System.out.println("                        register a metadata-only H.264 stream and 3 sample metadata events");
            System.out.println("  --synthetic-video-samples <count>");
            System.out.println("                        bounded sample metadata count; max 30");
            System.out.println("  --emit-synthetic-video-binary");
            System.out.println("                        emit bounded synthetic encoded packets over localhost TCP");
            System.out.println("  --binary-video-port <port>");
            System.out.println("                        device-local TCP port for the synthetic binary stream; default 8877");
            System.out.println("  --binary-video-packets <count>");
            System.out.println("                        bounded binary packet count; max 30");
            System.out.println("  --binary-video-packet-bytes <bytes>");
            System.out.println("                        bounded binary packet payload size; max 65536");
            System.out.println("  --emit-mediacodec-synthetic-video");
            System.out.println("                        encode a tiny synthetic Surface source with MediaCodec");
            System.out.println("  --emit-screenrecord-video");
            System.out.println("                        capture display H.264 through shell screenrecord stdout");
            System.out.println("  --encoded-video-frames <count>");
            System.out.println("                        bounded MediaCodec input frame count; max 60");
            System.out.println("  --encoded-video-width <pixels>");
            System.out.println("                        encoded video width; default 640");
            System.out.println("  --encoded-video-height <pixels>");
            System.out.println("                        encoded video height; default 360");
            System.out.println("  --encoded-video-bitrate <bps>");
            System.out.println("                        MediaCodec target bitrate; default 1000000");
            System.out.println("  --screenrecord-time-limit <seconds>");
            System.out.println("                        screenrecord capture time; max 3, default 1");
            System.out.println("  --proximity-watchdog");
            System.out.println("                        keep a shell-side proximity hold watchdog active");
            System.out.println("  --stop-proximity-watchdog");
            System.out.println("                        request any active shell-side proximity watchdog to stop");
            System.out.println("  --proximity-watchdog-until-stopped");
            System.out.println("                        run until --stop-proximity-watchdog creates the stop file");
            System.out.println("  --proximity-watchdog-ensure-stay-awake");
            System.out.println("                        also reapply svc power stayon and send KEYCODE_WAKEUP when needed");
            System.out.println("  --proximity-watchdog-duration-ms <ms>");
            System.out.println("                        watchdog process lifetime; default 28800000");
            System.out.println("  --proximity-watchdog-hold-duration-ms <ms>");
            System.out.println("                        prox_close hold duration when reapplying; default 28800000");
            System.out.println("  --proximity-watchdog-interval-ms <ms>");
            System.out.println("                        readback interval; 1000-60000, default 5000");
            System.out.println("  --focus-guardian");
            System.out.println("                        run reactive focus recovery and tuning-property apply loop");
            System.out.println("  --stop-focus-guardian");
            System.out.println("                        request any active focus guardian to stop");
            System.out.println("  --focus-guardian-mode <mode>");
            System.out.println("                        off, observe, recover_target, recover_broker, toggle_broker_target, launch_target_guard, strict");
            System.out.println("  --focus-guardian-desired-focus <target|broker>");
            System.out.println("                        fallback side when no active side has been observed");
            System.out.println("  --focus-target-package <package>");
            System.out.println("                        optional initial target package; broker UI can update it");
            System.out.println("  --focus-target-activity <activity>");
            System.out.println("                        optional initial target activity");
            System.out.println("  --focus-broker-package <package>");
            System.out.println("                        broker package; default com.example.rustyxr.broker");
            System.out.println("  --focus-broker-activity <activity>");
            System.out.println("                        broker activity; default com.example.rustyxr.broker.MainActivity");
            System.out.println("  --focus-guardian-duration-ms <ms>");
            System.out.println("                        focus guardian process lifetime; default 28800000");
            System.out.println("  --focus-guardian-interval-ms <ms>");
            System.out.println("                        foreground poll interval; 250-10000, default 1000");
            System.out.println("  --focus-guardian-cooldown-ms <ms>");
            System.out.println("                        minimum relaunch spacing; 250-10000, default 1500");
            System.exit(0);
        }
    }
}
