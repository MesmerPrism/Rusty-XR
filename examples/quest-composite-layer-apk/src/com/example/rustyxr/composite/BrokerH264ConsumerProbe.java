package com.example.rustyxr.composite;

import android.graphics.SurfaceTexture;
import android.graphics.ImageFormat;
import android.hardware.HardwareBuffer;
import android.media.Image;
import android.media.ImageReader;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaFormat;
import android.opengl.EGL14;
import android.opengl.EGLConfig;
import android.opengl.EGLContext;
import android.opengl.EGLDisplay;
import android.opengl.EGLSurface;
import android.opengl.GLES11Ext;
import android.opengl.GLES20;
import android.os.Build;
import android.os.Bundle;
import android.os.SystemClock;
import android.util.Base64;
import android.util.Log;
import android.view.Surface;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.EOFException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Random;
import java.util.zip.CRC32;

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
    private static final String SOURCE_MODE_BROKER_CAMERA = "broker-camera";
    private static final String SOURCE_MODE_BROKER_SYNTHETIC = "broker-synthetic";
    private static final String SOURCE_MODE_EXISTING_STREAM = "existing-stream";
    private static final String DEFAULT_SYNTHETIC_PATTERN = "diagnostic-grid";
    private static final String DEFAULT_SYNTHETIC_PROJECTION_PROFILE = "head-anchored-virtual-camera";
    private static final String STEREO_PAIRING_TIMESTAMP_NEAREST = "timestamp-nearest";
    private static final String STEREO_PAIRING_FRAME_ORDER = "frame-order";
    private static final long STEREO_REPLAY_DELIVERY_INTERVAL_NS = 33_333_333L;
    private static final long STEREO_REPLAY_DELIVERY_MAX_SLEEP_MS = 50L;
    private static final long STEREO_PAIR_MAX_DELTA_NS = 25_000_000L;
    private static final long STEREO_FRAME_SET_MAX_HOLD_NS = 75_000_000L;
    private static final long STEREO_FRAME_SET_STALE_NS = 250_000_000L;
    private static final int DEFAULT_LIVE_STEREO_PENDING_QUEUE_LIMIT = 8;
    private static final int MAX_LIVE_STEREO_PENDING_QUEUE_LIMIT = 16;
    private static final int MAX_PACKET_BYTES = 1024 * 1024;
    private static final int MAX_STREAM_HEADER_METADATA_BYTES = 256 * 1024;
    private static final int MAX_STREAM_PACKETS = 2400;
    private static final int DEQUEUE_TIMEOUT_US = 10000;
    private static final int SURFACE_FRAME_WAIT_MS = 250;
    private static final int HARDWARE_BUFFER_WAIT_MS = 250;
    private static final int HARDWARE_BUFFER_READER_MAX_IMAGES = 4;
    private static final long LIVE_PROGRESS_LOG_INTERVAL_NS = 1_000_000_000L;

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
        final int frameRateHz;
        final int commandTimeoutMs;
        final int streamTimeoutMs;
        final int decodeTimeoutMs;
        final String decodeOutputMode;
        final boolean stereo;
        final boolean liveStream;
        final String sourceMode;
        final boolean startBrokerCameraStream;
        final boolean startBrokerSyntheticStream;
        final String syntheticPattern;
        final String syntheticProjectionProfile;
        final boolean liveDecode;
        final boolean byteIdentityProbe;
        final String stereoPairingMode;
        final int liveStereoPendingQueueLimit;
        final String projectionMetadataJson;
        final String leftProjectionMetadataJson;
        final String rightProjectionMetadataJson;
        final String projectionMetadataBase64;
        final String leftProjectionMetadataBase64;
        final String rightProjectionMetadataBase64;

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
            int frameRateHz,
            int commandTimeoutMs,
            int streamTimeoutMs,
            int decodeTimeoutMs,
            String decodeOutputMode,
            boolean stereo,
            boolean liveStream,
            String sourceMode,
            String syntheticPattern,
            String syntheticProjectionProfile,
            boolean liveDecode,
            boolean byteIdentityProbe,
            String stereoPairingMode,
            int liveStereoPendingQueueLimit,
            String projectionMetadataJson,
            String leftProjectionMetadataJson,
            String rightProjectionMetadataJson,
            String projectionMetadataBase64,
            String leftProjectionMetadataBase64,
            String rightProjectionMetadataBase64) {
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
            this.frameRateHz = frameRateHz;
            this.commandTimeoutMs = commandTimeoutMs;
            this.streamTimeoutMs = streamTimeoutMs;
            this.decodeTimeoutMs = decodeTimeoutMs;
            this.decodeOutputMode = normalizeDecodeOutputMode(decodeOutputMode);
            this.stereo = stereo;
            this.liveStream = liveStream;
            this.sourceMode = normalizeSourceMode(sourceMode);
            this.startBrokerCameraStream = SOURCE_MODE_BROKER_CAMERA.equals(this.sourceMode);
            this.startBrokerSyntheticStream = SOURCE_MODE_BROKER_SYNTHETIC.equals(this.sourceMode);
            this.syntheticPattern = normalizeSyntheticPattern(syntheticPattern);
            this.syntheticProjectionProfile = normalizeSyntheticProjectionProfile(syntheticProjectionProfile);
            this.liveDecode = liveDecode;
            this.byteIdentityProbe = byteIdentityProbe;
            this.stereoPairingMode = normalizeStereoPairingMode(stereoPairingMode);
            this.liveStereoPendingQueueLimit = clampInt(
                liveStereoPendingQueueLimit,
                2,
                MAX_LIVE_STEREO_PENDING_QUEUE_LIMIT);
            this.projectionMetadataJson = projectionMetadataJson != null ? projectionMetadataJson : "";
            this.leftProjectionMetadataJson = leftProjectionMetadataJson != null ? leftProjectionMetadataJson : "";
            this.rightProjectionMetadataJson = rightProjectionMetadataJson != null ? rightProjectionMetadataJson : "";
            this.projectionMetadataBase64 = projectionMetadataBase64 != null ? projectionMetadataBase64 : "";
            this.leftProjectionMetadataBase64 =
                leftProjectionMetadataBase64 != null ? leftProjectionMetadataBase64 : "";
            this.rightProjectionMetadataBase64 =
                rightProjectionMetadataBase64 != null ? rightProjectionMetadataBase64 : "";
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
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 consumer config: host=%s brokerPort=%d leftStreamPort=%d rightStreamPort=%d stereo=%s liveStream=%s liveDecode=%s sourceMode=%s startBroker=%s captureMs=%d maxPackets=%d bitrateBps=%d frameRateHz=%d streamTimeoutMs=%d decodeTimeoutMs=%d leftCameraId=%s rightCameraId=%s metadataChars=%d leftMetadataChars=%d rightMetadataChars=%d metadataBase64Chars=%d leftMetadataBase64Chars=%d rightMetadataBase64Chars=%d",
                config.brokerHost,
                config.brokerPort,
                config.streamPort,
                config.rightStreamPort,
                config.stereo,
                config.liveStream,
                config.liveDecode,
                config.sourceMode,
                config.startBrokerCameraStream || config.startBrokerSyntheticStream,
                config.captureMs,
                config.maxPackets,
                config.bitrateBps,
                config.frameRateHz,
                config.streamTimeoutMs,
                config.decodeTimeoutMs,
                config.leftCameraId,
                config.rightCameraId,
                config.projectionMetadataJson.length(),
                config.leftProjectionMetadataJson.length(),
                config.rightProjectionMetadataJson.length(),
                config.projectionMetadataBase64.length(),
                config.leftProjectionMetadataBase64.length(),
                config.rightProjectionMetadataBase64.length()));
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
            report.put("frame_rate_hz", config.frameRateHz);
            report.put("stereo_requested", config.stereo);
            report.put("live_stream_requested", config.liveStream);
            report.put("source_mode", config.sourceMode);
            report.put("broker_camera_stream_start_requested", config.startBrokerCameraStream);
            report.put("broker_synthetic_stream_start_requested", config.startBrokerSyntheticStream);
            report.put("synthetic_pattern", config.syntheticPattern);
            report.put("synthetic_projection_profile", config.syntheticProjectionProfile);
            report.put("decode_output_mode", config.decodeOutputMode);
            report.put("live_decode_requested", config.liveDecode);
            report.put("byte_identity_probe_requested", config.byteIdentityProbe);
            report.put("stereo_pairing_mode_requested", config.stereoPairingMode);
            report.put("stereo_live_pending_queue_limit", config.liveStereoPendingQueueLimit);
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
        StartCommandResult startCommand = startOrUseExistingStream("mono", config.cameraId, config.streamPort);
        putStartCommandReport(report, "", startCommand);
        if (!startCommand.ack.optBoolean("accepted", false)) {
            throw new IllegalStateException("Broker rejected app-camera H.264 stream command.");
        }

        StreamResult stream = receiveStream("mono", config.streamPort);
        DecodeResult decode = decodePackets(
            stream,
            config.decodeTimeoutMs,
            config.decodeOutputMode,
            preferredStreamProjectionMetadata(stream, startCommand.streamProjectionMetadata),
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
        report.put("stereo_pairing_mode", "timestamp-nearest");
        report.put("stereo_pair_max_delta_target_ns", STEREO_PAIR_MAX_DELTA_NS);
        if (!DECODE_OUTPUT_HARDWARE_BUFFER.equals(config.decodeOutputMode)) {
            throw new IllegalStateException("Broker H.264 stereo probe requires hardware-buffer decode output.");
        }

        StartCommandResult leftStart = startOrUseExistingStream("left", config.leftCameraId, config.streamPort);
        putStartCommandReport(report, "left", leftStart);
        if (!leftStart.ack.optBoolean("accepted", false)) {
            throw new IllegalStateException("Broker rejected left app-camera H.264 stream command.");
        }

        StartCommandResult rightStart = startOrUseExistingStream("right", config.rightCameraId, config.rightStreamPort);
        putStartCommandReport(report, "right", rightStart);
        if (!rightStart.ack.optBoolean("accepted", false)) {
            throw new IllegalStateException("Broker rejected right app-camera H.264 stream command.");
        }

        if (shouldUseLiveStereoDecode()) {
            runLiveStereoProbe(report, startedElapsedNs, leftStart, rightStart);
            return;
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
                preferredStreamProjectionMetadata(leftStream, leftStart.streamProjectionMetadata),
                config.leftCameraId,
                "left",
                true);
            rightDecode = decodePackets(
                rightStream,
                config.decodeTimeoutMs,
                config.decodeOutputMode,
                preferredStreamProjectionMetadata(rightStream, rightStart.streamProjectionMetadata),
                config.rightCameraId,
                "right",
                true);
            boolean paceStereoDelivery = shouldPaceStereoDelivery();
            StereoPairResult pair = deliverStereoPairs(
                leftDecode.collectedHardwareBufferFrames,
                rightDecode.collectedHardwareBufferFrames,
                paceStereoDelivery);
            long completedElapsedNs = SystemClock.elapsedRealtimeNanos();
            putStreamDecodeReport(report, "left", leftStream, leftDecode);
            putStreamDecodeReport(report, "right", rightStream, rightDecode);
            report.put("stereo_pair_count", pair.pairCount);
            report.put("stereo_pair_native_accepted_count", pair.nativeAcceptedCount);
            report.put("stereo_pair_native_rejected_count", pair.nativeRejectedCount);
            report.put("stereo_pair_delivery_paced", pair.deliveryPaced);
            report.put("stereo_pair_delivery_duration_ns", pair.deliveryDurationNs);
            report.put("stereo_pair_delta_total_ns", pair.deltaTotalNs);
            report.put("stereo_pair_delta_avg_ns", pair.pairCount > 0 ? pair.deltaTotalNs / pair.pairCount : 0L);
            report.put("stereo_pair_delta_max_ns", pair.deltaMaxNs);
            report.put("stereo_pair_delta_over_target_count", pair.deltaOverTargetCount);
            putStereoFrameSetGateReport(report, pair);
            putStageTimingReport(report, "", "stereo_pair_native_bridge", pair.nativeBridgeTiming);
            report.put("stereo_left_right_resolution_match", pair.resolutionMismatchCount == 0);
            report.put("stereo_resolution_mismatch_count", pair.resolutionMismatchCount);
            DecodeResult leftByteIdentity = null;
            DecodeResult rightByteIdentity = null;
            if (config.byteIdentityProbe) {
                try {
                    leftByteIdentity = decodePackets(
                        leftStream,
                        config.decodeTimeoutMs,
                        DECODE_OUTPUT_BYTE_BUFFER,
                        preferredStreamProjectionMetadata(leftStream, leftStart.streamProjectionMetadata),
                        config.leftCameraId,
                        "left",
                        false);
                    rightByteIdentity = decodePackets(
                        rightStream,
                        config.decodeTimeoutMs,
                        DECODE_OUTPUT_BYTE_BUFFER,
                        preferredStreamProjectionMetadata(rightStream, rightStart.streamProjectionMetadata),
                        config.rightCameraId,
                        "right",
                        false);
                    putByteIdentityReport(report, "left", leftByteIdentity);
                    putByteIdentityReport(report, "right", rightByteIdentity);
                    Log.i(TAG, String.format(
                        Locale.US,
                        "Rusty XR broker H.264 decoded byte identity: leftFrames=%d leftUniqueCrc32=%d leftAdjacentEqual=%d leftAllIdentical=%s leftFirstCrc32=%d leftLastCrc32=%d rightFrames=%d rightUniqueCrc32=%d rightAdjacentEqual=%d rightAllIdentical=%s rightFirstCrc32=%d rightLastCrc32=%d",
                        leftByteIdentity.outputFrameHashCount,
                        leftByteIdentity.outputFrameHashUniqueCount,
                        leftByteIdentity.outputFrameHashAdjacentEqualCount,
                        outputFramesAllIdentical(leftByteIdentity),
                        leftByteIdentity.firstOutputFrameCrc32,
                        leftByteIdentity.lastOutputFrameCrc32,
                        rightByteIdentity.outputFrameHashCount,
                        rightByteIdentity.outputFrameHashUniqueCount,
                        rightByteIdentity.outputFrameHashAdjacentEqualCount,
                        outputFramesAllIdentical(rightByteIdentity),
                        rightByteIdentity.firstOutputFrameCrc32,
                        rightByteIdentity.lastOutputFrameCrc32));
                } catch (Exception identityError) {
                    report.put("byte_identity_probe_error", identityError.getClass().getSimpleName() + ": " + safeMessage(identityError));
                    Log.w(TAG, "Rusty XR broker H.264 byte identity probe failed", identityError);
                }
            }
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
                "Rusty XR broker H.264 stereo summary: succeeded=%s liveStream=%s leftCameraId=%s rightCameraId=%s left=%dx%d right=%dx%d leftPackets=%d rightPackets=%d leftPayloadBytes=%d rightPayloadBytes=%d leftEncodedPacketHz=%.3f rightEncodedPacketHz=%.3f leftSourcePacketHz=%.3f rightSourcePacketHz=%.3f leftWirePacketHz=%.3f rightWirePacketHz=%.3f leftDecodedFrames=%d rightDecodedFrames=%d leftDecodedFrameHz=%.3f rightDecodedFrameHz=%.3f pairCount=%d nativeAccepted=%d nativeRejected=%d pairDeliveryPaced=%s pairDeliveryDurationNs=%d pairDeltaAvgNs=%d pairDeltaMaxNs=%d nativeBridgeAvgNs=%d nativeBridgeMaxNs=%d metadataReadyLeft=%s metadataReadyRight=%s poseSourceLeft=%s poseSourceRight=%s totalDurationNs=%d",
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
                pair.deliveryPaced,
                pair.deliveryDurationNs,
                pair.pairCount > 0 ? pair.deltaTotalNs / pair.pairCount : 0L,
                pair.deltaMaxNs,
                pair.nativeBridgeTiming.averageNs(),
                pair.nativeBridgeTiming.maxNs,
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

    private void runLiveStereoProbe(
        JSONObject report,
        long startedElapsedNs,
        StartCommandResult leftStart,
        StartCommandResult rightStart) throws Exception {
        report.put("live_decode_path", true);
        report.put("stereo_pairing_mode", "live-" + config.stereoPairingMode);
        report.put("stereo_pair_max_delta_target_ns", STEREO_PAIR_MAX_DELTA_NS);
        LiveStereoPairer pairer = new LiveStereoPairer(
            config.stereoPairingMode,
            config.liveStereoPendingQueueLimit);
        LiveDecodeStreamTask leftDecode = new LiveDecodeStreamTask(
            "left",
            config.streamPort,
            config.leftCameraId,
            leftStart.streamProjectionMetadata,
            pairer,
            startedElapsedNs);
        LiveDecodeStreamTask rightDecode = new LiveDecodeStreamTask(
            "right",
            config.rightStreamPort,
            config.rightCameraId,
            rightStart.streamProjectionMetadata,
            pairer,
            startedElapsedNs);
        Log.i(TAG, String.format(
            Locale.US,
            "Rusty XR broker H.264 live stereo starting decode tasks: leftPort=%d rightPort=%d leftMetadataReady=%s rightMetadataReady=%s",
            config.streamPort,
            config.rightStreamPort,
            leftStart.streamProjectionMetadata != null &&
                leftStart.streamProjectionMetadata.optBoolean("projectionMetadataReady", false),
            rightStart.streamProjectionMetadata != null &&
                rightStart.streamProjectionMetadata.optBoolean("projectionMetadataReady", false)));
        leftDecode.start();
        rightDecode.start();
        LiveDecodeResult leftResult = null;
        LiveDecodeResult rightResult = null;
        try {
            leftResult = leftDecode.awaitResult();
            rightResult = rightDecode.awaitResult();
            pairer.closePendingFrames();
            StereoPairResult pair = pairer.snapshot();
            long completedElapsedNs = SystemClock.elapsedRealtimeNanos();
            putLiveDecodeReport(report, "left", leftResult);
            putLiveDecodeReport(report, "right", rightResult);
            report.put("stereo_pair_count", pair.pairCount);
            report.put("stereo_pair_native_accepted_count", pair.nativeAcceptedCount);
            report.put("stereo_pair_native_rejected_count", pair.nativeRejectedCount);
            report.put("stereo_pair_delivery_paced", false);
            report.put("stereo_pair_delivery_duration_ns", pair.deliveryDurationNs);
            report.put("stereo_pair_delta_total_ns", pair.deltaTotalNs);
            report.put("stereo_pair_delta_avg_ns", pair.pairCount > 0 ? pair.deltaTotalNs / pair.pairCount : 0L);
            report.put("stereo_pair_delta_max_ns", pair.deltaMaxNs);
            report.put("stereo_pair_delta_over_target_count", pair.deltaOverTargetCount);
            putStereoFrameSetGateReport(report, pair);
            putStageTimingReport(report, "", "stereo_pair_native_bridge", pair.nativeBridgeTiming);
            report.put("stereo_left_right_resolution_match", pair.resolutionMismatchCount == 0);
            report.put("stereo_resolution_mismatch_count", pair.resolutionMismatchCount);
            report.put("stereo_live_pair_queue_drop_count", pair.queueDropCount);
            report.put("total_duration_ns", Math.max(0L, completedElapsedNs - startedElapsedNs));
            report.put("succeeded", pair.nativeAcceptedCount > 0);
            if (leftResult.lastError.length() > 0) {
                report.put("left_last_error", leftResult.lastError);
            }
            if (rightResult.lastError.length() > 0) {
                report.put("right_last_error", rightResult.lastError);
            }
            if (pair.nativeAcceptedCount == 0 && pair.pairCount == 0) {
                report.put("last_error", "Live decoded hardware-buffer frames were not paired from both streams.");
            } else if (pair.nativeAcceptedCount == 0) {
                report.put("last_error", "Native live stereo hardware-buffer bridge rejected every decoded pair.");
            }
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 live stereo summary: succeeded=%s liveStream=%s leftCameraId=%s rightCameraId=%s left=%dx%d right=%dx%d leftPackets=%d rightPackets=%d leftPayloadBytes=%d rightPayloadBytes=%d leftWirePacketHz=%.3f rightWirePacketHz=%.3f leftDecodedFrames=%d rightDecodedFrames=%d leftDecodedFrameHz=%.3f rightDecodedFrameHz=%.3f pairCount=%d nativeAccepted=%d nativeRejected=%d queueDrops=%d pairDeltaAvgNs=%d pairDeltaMaxNs=%d nativeBridgeAvgNs=%d nativeBridgeMaxNs=%d metadataReadyLeft=%s metadataReadyRight=%s totalDurationNs=%d",
                pair.nativeAcceptedCount > 0,
                config.liveStream,
                leftStart.streamProjectionMetadata != null
                    ? leftStart.streamProjectionMetadata.optString("cameraId", "")
                    : config.leftCameraId,
                rightStart.streamProjectionMetadata != null
                    ? rightStart.streamProjectionMetadata.optString("cameraId", "")
                    : config.rightCameraId,
                leftResult.width,
                leftResult.height,
                rightResult.width,
                rightResult.height,
                leftResult.packetCount,
                rightResult.packetCount,
                leftResult.payloadBytes,
                rightResult.payloadBytes,
                rateHzFromNs(leftResult.packetCount, Math.max(0L, leftResult.lastPacketReceiveElapsedNs - leftResult.firstPacketReceiveElapsedNs)),
                rateHzFromNs(rightResult.packetCount, Math.max(0L, rightResult.lastPacketReceiveElapsedNs - rightResult.firstPacketReceiveElapsedNs)),
                leftResult.decodedFrameCount,
                rightResult.decodedFrameCount,
                rateHzFromNs(leftResult.decodedFrameCount, Math.max(0L, leftResult.decodeEndElapsedNs - leftResult.decodeStartElapsedNs)),
                rateHzFromNs(rightResult.decodedFrameCount, Math.max(0L, rightResult.decodeEndElapsedNs - rightResult.decodeStartElapsedNs)),
                pair.pairCount,
                pair.nativeAcceptedCount,
                pair.nativeRejectedCount,
                pair.queueDropCount,
                pair.pairCount > 0 ? pair.deltaTotalNs / pair.pairCount : 0L,
                pair.deltaMaxNs,
                pair.nativeBridgeTiming.averageNs(),
                pair.nativeBridgeTiming.maxNs,
                leftStart.streamProjectionMetadata != null &&
                    leftStart.streamProjectionMetadata.optBoolean("projectionMetadataReady", false),
                rightStart.streamProjectionMetadata != null &&
                    rightStart.streamProjectionMetadata.optBoolean("projectionMetadataReady", false),
                Math.max(0L, completedElapsedNs - startedElapsedNs)));
        } finally {
            pairer.closePendingFrames();
        }
    }

    private StartCommandResult startOrUseExistingStream(String label, String cameraId, int streamPort) throws Exception {
        Log.i(TAG, String.format(
            Locale.US,
            "Rusty XR broker H.264 stream setup: label=%s cameraId=%s streamPort=%d startBroker=%s sourceMode=%s",
            label,
            cameraId,
            streamPort,
            config.startBrokerCameraStream || config.startBrokerSyntheticStream,
            config.sourceMode));
        if (config.startBrokerCameraStream) {
            return sendStartCommand(label, cameraId, streamPort);
        }
        if (config.startBrokerSyntheticStream) {
            return sendStartCommand(label, cameraId, streamPort);
        }

        JSONObject streamProjectionMetadata = configuredExistingStreamProjectionMetadata(label, cameraId);
        JSONObject brokerStatus = null;
        if (streamProjectionMetadata == null) {
            brokerStatus = fetchBrokerStatusOrNull();
            streamProjectionMetadata = buildExistingStreamProjectionMetadata(brokerStatus, cameraId);
        }
        JSONObject projectionProfile = brokerStatus != null ? brokerStatus.optJSONObject("projectionProfile") : null;
        JSONObject ack = new JSONObject();
        ack.put("type", "command_ack");
        ack.put("schema", "rusty.xr.broker.command_ack.v1");
        ack.put("request_id", "composite-h264-existing-stream-" + label + "-" + System.currentTimeMillis());
        ack.put("command", "camera_provider.start_app_camera_h264_stream");
        ack.put("accepted", true);
        ack.put("message", "using_existing_h264_stream");
        JSONObject result = new JSONObject();
        result.put("source_mode", config.sourceMode);
        result.put("stream_port", streamPort);
        result.put("camera_id", cameraId != null ? cameraId : "");
        result.put("projection_metadata_attached", streamProjectionMetadata != null);
        if (streamProjectionMetadata != null) {
            result.put("projection_metadata", streamProjectionMetadata);
        }
        if (projectionProfile != null) {
            result.put("projection_profile", projectionProfile);
        }
        ack.put("result", result);
        Log.i(TAG, String.format(
            Locale.US,
            "Rusty XR broker H.264 using existing stream: label=%s streamPort=%d metadataAttached=%s metadataReady=%s metadataCameraId=%s projectionProfileAttached=%s",
            label,
            streamPort,
            streamProjectionMetadata != null,
            streamProjectionMetadata != null &&
                streamProjectionMetadata.optBoolean("projectionMetadataReady", false),
            streamProjectionMetadata != null ? streamProjectionMetadata.optString("cameraId", "") : "",
            projectionProfile != null));
        return new StartCommandResult(ack, streamProjectionMetadata, projectionProfile);
    }

    private JSONObject configuredExistingStreamProjectionMetadata(String label, String cameraId) {
        String json = "";
        String encoded = "";
        if ("left".equals(label)) {
            json = config.leftProjectionMetadataJson;
            encoded = config.leftProjectionMetadataBase64;
        } else if ("right".equals(label)) {
            json = config.rightProjectionMetadataJson;
            encoded = config.rightProjectionMetadataBase64;
        }
        if (json == null || json.length() == 0) {
            json = config.projectionMetadataJson;
        }
        if (encoded == null || encoded.length() == 0) {
            encoded = config.projectionMetadataBase64;
        }
        if (json == null || json.trim().length() == 0) {
            json = decodeProjectionMetadataBase64(encoded, label);
        }
        if (json == null || json.trim().length() == 0) {
            return null;
        }
        try {
            JSONObject metadata = new JSONObject(json);
            if (cameraId != null && cameraId.length() > 0 && !metadata.has("cameraId")) {
                metadata.put("cameraId", cameraId);
            }
            metadata.put("source", metadata.optString("source", "broker_existing_h264_stream_launch_extra"));
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 parsed existing projection metadata: label=%s cameraId=%s ready=%s hasIntrinsics=%s hasExtrinsics=%s jsonChars=%d",
                label,
                metadata.optString("cameraId", ""),
                metadata.optBoolean("projectionMetadataReady", false),
                metadata.has("intrinsics"),
                metadata.has("extrinsics"),
                json.length()));
            return metadata;
        } catch (Exception ex) {
            Log.w(TAG, "Could not parse existing H.264 stream projection metadata launch extra for " +
                label + ": " + safeMessage(ex));
            return null;
        }
    }

    private String decodeProjectionMetadataBase64(String encoded, String label) {
        if (encoded == null || encoded.trim().length() == 0) {
            return "";
        }
        try {
            byte[] bytes = Base64.decode(encoded.trim(), Base64.DEFAULT);
            return new String(bytes, StandardCharsets.UTF_8);
        } catch (Exception ex) {
            Log.w(TAG, "Could not decode existing H.264 stream projection metadata launch extra for " +
                label + ": " + safeMessage(ex));
            return "";
        }
    }

    private JSONObject fetchBrokerStatusOrNull() {
        Socket socket = null;
        try {
            socket = new Socket();
            socket.connect(new InetSocketAddress(config.brokerHost, config.brokerPort), config.commandTimeoutMs);
            socket.setSoTimeout(config.commandTimeoutMs);
            OutputStream output = socket.getOutputStream();
            String request =
                "GET /status HTTP/1.1\r\n" +
                "Host: " + config.brokerHost + ":" + config.brokerPort + "\r\n" +
                "Connection: close\r\n" +
                "\r\n";
            output.write(request.getBytes(StandardCharsets.US_ASCII));
            output.flush();

            ByteArrayOutputStream response = new ByteArrayOutputStream(32 * 1024);
            InputStream input = socket.getInputStream();
            byte[] buffer = new byte[4096];
            int read;
            while ((read = input.read(buffer)) >= 0) {
                if (read > 0) {
                    response.write(buffer, 0, read);
                }
            }
            String text = new String(response.toByteArray(), StandardCharsets.UTF_8);
            int bodyStart = text.indexOf("\r\n\r\n");
            if (bodyStart < 0) {
                return null;
            }
            return new JSONObject(text.substring(bodyStart + 4));
        } catch (Exception ex) {
            Log.w(TAG, "Could not fetch broker status for existing H.264 stream metadata: " + safeMessage(ex));
            return null;
        } finally {
            closeQuietly(socket);
        }
    }

    private JSONObject buildExistingStreamProjectionMetadata(JSONObject brokerStatus, String cameraId) {
        if (brokerStatus == null) {
            return null;
        }
        try {
            JSONObject candidate = findProjectionCandidate(brokerStatus, cameraId);
            if (candidate == null) {
                return null;
            }
            JSONArray calibration = candidate.optJSONArray("lens_intrinsic_calibration");
            JSONArray translation = candidate.optJSONArray("lens_pose_translation_m");
            JSONArray rotation = candidate.optJSONArray("lens_pose_rotation_xyzw");
            JSONObject intrinsicsDomain = projectionCandidateDomain(candidate);
            boolean hasIntrinsics = calibration != null && calibration.length() >= 4 && intrinsicsDomain != null;
            boolean hasPose = translation != null && translation.length() >= 3 && rotation != null && rotation.length() >= 4;

            JSONObject metadata = new JSONObject();
            String metadataCameraId = candidate.optString(
                "camera_id",
                cameraId != null && cameraId.length() > 0 ? cameraId : "broker-h264");
            metadata.put("schema", "rusty.xr.camera_projection.stream_source_metadata.v1");
            metadata.put("source", "broker_existing_h264_stream");
            metadata.put("sourceLabel", "Broker existing H.264 source " + metadataCameraId);
            metadata.put("cameraId", metadataCameraId);
            metadata.put("selectionScore", candidate.optLong("selection_score", 0L));
            metadata.put("deliveredWidth", config.preferredWidth);
            metadata.put("deliveredHeight", config.preferredHeight);
            metadata.put("lensFacing", normalizeLensFacing(candidate.optString("lens_facing", "unknown")));
            metadata.put("lensFacingRank", lensFacingRank(candidate.optString("lens_facing", "")));
            if (candidate.has("sensor_orientation_degrees")) {
                metadata.put("sensorOrientationDegrees", candidate.optInt("sensor_orientation_degrees"));
            }
            if (hasIntrinsics) {
                JSONObject intrinsics = new JSONObject();
                intrinsics.put("fx", calibration.optDouble(0, 0.0));
                intrinsics.put("fy", calibration.optDouble(1, 0.0));
                intrinsics.put("cx", calibration.optDouble(2, 0.0));
                intrinsics.put("cy", calibration.optDouble(3, 0.0));
                intrinsics.put("skew", calibration.optDouble(4, 0.0));
                metadata.put("intrinsics", intrinsics);
                metadata.put("intrinsicsDomain", intrinsicsDomain);
                metadata.put("activeArrayDomain", new JSONObject(intrinsicsDomain.toString()));
                metadata.put("sensorPixelDomain", new JSONObject(intrinsicsDomain.toString()));
            }
            metadata.put("missingIntrinsics", !hasIntrinsics);
            metadata.put("missingPose", !hasPose);
            metadata.put("poseSource", hasPose ? "platform" : "missing");
            metadata.put(
                "poseCoordinateConvention",
                hasPose
                    ? "android-camera2-lens-pose-reference-from-camera"
                    : "broker-decoded-h264-image-space");
            metadata.put(
                "lensPoseReferenceLabel",
                lensPoseReferenceLabel(candidate.optString("lens_pose_reference", "")));
            if (hasPose) {
                JSONObject extrinsics = new JSONObject();
                extrinsics.put("px", translation.optDouble(0, 0.0));
                extrinsics.put("py", translation.optDouble(1, 0.0));
                extrinsics.put("pz", translation.optDouble(2, 0.0));
                extrinsics.put("qx", rotation.optDouble(0, 0.0));
                extrinsics.put("qy", rotation.optDouble(1, 0.0));
                extrinsics.put("qz", rotation.optDouble(2, 0.0));
                extrinsics.put("qw", rotation.optDouble(3, 1.0));
                metadata.put("extrinsics", extrinsics);
            }
            metadata.put("projectionMetadataReady", hasIntrinsics && hasPose);
            return metadata;
        } catch (Exception ex) {
            Log.w(TAG, "Could not build existing H.264 stream projection metadata: " + safeMessage(ex));
            return null;
        }
    }

    private static JSONObject findProjectionCandidate(JSONObject brokerStatus, String cameraId) throws Exception {
        JSONObject profile = brokerStatus.optJSONObject("projectionProfile");
        JSONObject candidate = findProjectionCandidate(profile != null ? profile.optJSONArray("source_candidates") : null, cameraId);
        if (candidate != null) {
            return candidate;
        }
        JSONObject cameraProvider = brokerStatus.optJSONObject("cameraProvider");
        return findProjectionCandidate(cameraProvider != null ? cameraProvider.optJSONArray("source_candidates") : null, cameraId);
    }

    private static JSONObject findProjectionCandidate(JSONArray candidates, String cameraId) throws Exception {
        if (candidates == null || candidates.length() == 0) {
            return null;
        }
        JSONObject firstReady = null;
        for (int i = 0; i < candidates.length(); i++) {
            JSONObject candidate = candidates.getJSONObject(i);
            if (cameraId != null &&
                    cameraId.length() > 0 &&
                    cameraId.equals(candidate.optString("camera_id", ""))) {
                return candidate;
            }
            if (firstReady == null &&
                    candidate.optBoolean("has_lens_pose", false) &&
                    candidate.optBoolean("has_intrinsics", false)) {
                firstReady = candidate;
            }
        }
        return firstReady;
    }

    private static JSONObject projectionCandidateDomain(JSONObject candidate) throws Exception {
        JSONObject domainSize = candidate.optJSONObject("max_private_size");
        if (domainSize == null) {
            domainSize = candidate.optJSONObject("max_yuv_420_888_size");
        }
        int width = domainSize != null ? domainSize.optInt("width", 0) : 0;
        int height = domainSize != null ? domainSize.optInt("height", 0) : 0;
        if (width <= 0 || height <= 0) {
            return null;
        }
        JSONObject domain = new JSONObject();
        domain.put("kind", "activeArray");
        domain.put("width", width);
        domain.put("height", height);
        return domain;
    }

    private static String normalizeLensFacing(String value) {
        return value != null && value.length() > 0 ? value.toLowerCase(Locale.US) : "unknown";
    }

    private static int lensFacingRank(String value) {
        String normalized = normalizeLensFacing(value);
        if ("back".equals(normalized)) {
            return 3;
        }
        if ("external".equals(normalized)) {
            return 2;
        }
        if ("front".equals(normalized)) {
            return 1;
        }
        return 0;
    }

    private static String lensPoseReferenceLabel(String value) {
        if ("0".equals(value)) {
            return "UNDEFINED";
        }
        if ("1".equals(value)) {
            return "GYROSCOPE";
        }
        if ("2".equals(value)) {
            return "PRIMARY_CAMERA";
        }
        return value != null && value.length() > 0 ? value : "unknown";
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
        params.put("frame_rate_hz", config.frameRateHz);
        params.put("live_stream", config.liveStream);
        if (cameraId != null && cameraId.length() > 0) {
            params.put("camera_id", cameraId);
        }
        if (config.startBrokerSyntheticStream) {
            params.put("source_mode", "synthetic_surface");
            params.put("synthetic_pattern", config.syntheticPattern);
            params.put("synthetic_projection_profile", config.syntheticProjectionProfile);
        }

        JSONObject command = new JSONObject();
        command.put("type", "command");
        command.put("schema", "rusty.xr.broker.command.v1");
        command.put("request_id", "composite-h264-consumer-" + label + "-" + System.currentTimeMillis());
        command.put(
            "command",
            config.startBrokerSyntheticStream
                ? "media.start_synthetic_h264_stream"
                : "camera_provider.start_app_camera_h264_stream");
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
        JSONObject result = ack != null ? ack.optJSONObject("result") : null;
        JSONObject streamStart = result != null ? result.optJSONObject("stream_start") : null;
        report.put(reportKey(prefix, "broker_command_accepted"), ack.optBoolean("accepted", false));
        report.put(reportKey(prefix, "broker_command_message"), ack.optString("message", ""));
        if (streamStart != null) {
            report.put(reportKey(prefix, "camera_source_id"), streamStart.optString("camera_source_id", ""));
            report.put(reportKey(prefix, "source_api_path"), streamStart.optString("source_api_path", ""));
            report.put(reportKey(prefix, "camera_permission_state"), streamStart.optString("camera_permission_state", ""));
            report.put(reportKey(prefix, "headset_camera_permission_state"), streamStart.optString("headset_camera_permission_state", ""));
            report.put(reportKey(prefix, "selected_camera_id"), streamStart.optString("selected_camera_id", ""));
            report.put(reportKey(prefix, "selected_width"), streamStart.optInt("selected_width", 0));
            report.put(reportKey(prefix, "selected_height"), streamStart.optInt("selected_height", 0));
            report.put(reportKey(prefix, "selected_fps_min_hz"), streamStart.optInt("selected_fps_min_hz", 0));
            report.put(reportKey(prefix, "selected_fps_max_hz"), streamStart.optInt("selected_fps_max_hz", 0));
            report.put(reportKey(prefix, "selected_reason"), streamStart.optString("selected_reason", ""));
            report.put(reportKey(prefix, "stream_min_frame_duration_ns"), streamStart.optLong("stream_min_frame_duration_ns", 0L));
            report.put(reportKey(prefix, "timestamp_domain"), streamStart.optString("timestamp_domain", ""));
            JSONObject capabilities = streamStart.optJSONObject("camera_source_capabilities");
            report.put(reportKey(prefix, "camera_source_capabilities_attached"), capabilities != null);
            if (capabilities != null) {
                report.put(
                    reportKey(prefix, "supported_private_size_count"),
                    capabilities.optJSONArray("supported_private_sizes") != null
                        ? capabilities.optJSONArray("supported_private_sizes").length()
                        : 0);
                report.put(
                    reportKey(prefix, "supported_yuv_size_count"),
                    capabilities.optJSONArray("supported_yuv_sizes") != null
                        ? capabilities.optJSONArray("supported_yuv_sizes").length()
                        : 0);
                report.put(
                    reportKey(prefix, "supported_fps_range_count"),
                    capabilities.optJSONArray("supported_fps_ranges") != null
                        ? capabilities.optJSONArray("supported_fps_ranges").length()
                        : 0);
            }
        }
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
        report.put(reportKey(prefix, "stream_header_metadata_bytes"), stream.headerMetadataBytes);
        report.put(
            reportKey(prefix, "stream_header_projection_metadata_attached"),
            stream.headerProjectionMetadata != null);
        if (stream.headerProjectionMetadata != null) {
            report.put(
                reportKey(prefix, "stream_header_projection_metadata_camera_id"),
                stream.headerProjectionMetadata.optString("cameraId", ""));
            report.put(
                reportKey(prefix, "stream_header_projection_metadata_ready"),
                stream.headerProjectionMetadata.optBoolean("projectionMetadataReady", false));
        }
        report.put(reportKey(prefix, "stream_packet_count"), stream.packets.size());
        report.put(reportKey(prefix, "stream_codec_config_packet_count"), codecConfigPacketCount(stream.packets));
        report.put(reportKey(prefix, "stream_keyframe_count"), keyFrameCount(stream.packets));
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
        report.put(reportKey(prefix, "decoder_low_latency_feature_supported"), decode.lowLatencyFeatureSupported);
        report.put(reportKey(prefix, "decoder_low_latency_config_requested"), decode.lowLatencyConfigRequested);
        report.put(reportKey(prefix, "decoder_low_latency_parameter_succeeded"), decode.lowLatencyParameterSucceeded);
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
        putStageTimingReport(report, prefix, "hardware_buffer_await_image", decode.hardwareBufferAwaitImageTiming);
        putStageTimingReport(report, prefix, "hardware_buffer_get_buffer", decode.hardwareBufferGetBufferTiming);
        putStageTimingReport(report, prefix, "hardware_buffer_native_bridge", decode.hardwareBufferNativeBridgeTiming);
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

    private static void putLiveDecodeReport(JSONObject report, String prefix, LiveDecodeResult result) throws Exception {
        report.put(reportKey(prefix, "stream_schema_version"), result.schemaVersion);
        report.put(reportKey(prefix, "stream_codec_id"), result.codecId);
        report.put(reportKey(prefix, "stream_width"), result.width);
        report.put(reportKey(prefix, "stream_height"), result.height);
        report.put(reportKey(prefix, "stream_declared_packet_count"), result.declaredPacketCount);
        report.put(reportKey(prefix, "stream_header_metadata_bytes"), result.headerMetadataBytes);
        report.put(
            reportKey(prefix, "stream_header_projection_metadata_attached"),
            result.headerProjectionMetadata != null);
        if (result.headerProjectionMetadata != null) {
            report.put(
                reportKey(prefix, "stream_header_projection_metadata_camera_id"),
                result.headerProjectionMetadata.optString("cameraId", ""));
            report.put(
                reportKey(prefix, "stream_header_projection_metadata_ready"),
                result.headerProjectionMetadata.optBoolean("projectionMetadataReady", false));
        }
        report.put(reportKey(prefix, "session_projection_metadata_source"), result.sessionProjectionMetadataSource);
        report.put(reportKey(prefix, "stream_packet_count"), result.packetCount);
        report.put(reportKey(prefix, "stream_ended_by_eof"), result.streamEndedByEof);
        report.put(reportKey(prefix, "stream_missing_declared_packet_count"), result.streamMissingDeclaredPacketCount);
        report.put(reportKey(prefix, "stream_payload_bytes"), result.payloadBytes);
        report.put(reportKey(prefix, "stream_receive_duration_ns"), Math.max(0L, result.receiveEndElapsedNs - result.receiveStartElapsedNs));
        report.put(
            reportKey(prefix, "stream_payload_bitrate_bps"),
            bitrateBpsFromNs(result.payloadBytes, Math.max(0L, result.lastPacketReceiveElapsedNs - result.firstPacketReceiveElapsedNs)));
        report.put(
            reportKey(prefix, "stream_wire_packet_rate_hz"),
            rateHzFromNs(result.packetCount, Math.max(0L, result.lastPacketReceiveElapsedNs - result.firstPacketReceiveElapsedNs)));
        report.put(
            reportKey(prefix, "stream_source_packet_rate_hz"),
            rateHzFromNs(result.packetCount, Math.max(0L, result.lastSourceElapsedNs - result.firstSourceElapsedNs)));
        report.put(reportKey(prefix, "stream_first_source_elapsed_ns"), result.firstSourceElapsedNs);
        report.put(reportKey(prefix, "stream_last_source_elapsed_ns"), result.lastSourceElapsedNs);
        report.put(reportKey(prefix, "decode_succeeded"), result.decodedFrameCount > 0);
        report.put(reportKey(prefix, "decoder_name"), result.decoderName);
        report.put(reportKey(prefix, "decoder_low_latency_feature_supported"), result.lowLatencyFeatureSupported);
        report.put(reportKey(prefix, "decoder_low_latency_config_requested"), result.lowLatencyConfigRequested);
        report.put(reportKey(prefix, "decoder_low_latency_parameter_succeeded"), result.lowLatencyParameterSucceeded);
        report.put(reportKey(prefix, "csd_sps_found"), result.spsBytes > 0);
        report.put(reportKey(prefix, "csd_pps_found"), result.ppsBytes > 0);
        report.put(reportKey(prefix, "input_buffer_count"), result.inputBufferCount);
        report.put(reportKey(prefix, "input_bytes"), result.inputBytes);
        report.put(reportKey(prefix, "input_eos_queued"), result.inputEosQueued);
        report.put(reportKey(prefix, "output_format_changes"), result.outputFormatChanges);
        report.put(reportKey(prefix, "output_buffer_count"), result.outputBufferCount);
        report.put(reportKey(prefix, "decoded_frame_count"), result.decodedFrameCount);
        report.put(
            reportKey(prefix, "decoded_frame_rate_hz"),
            rateHzFromNs(result.decodedFrameCount, Math.max(0L, result.decodeEndElapsedNs - result.decodeStartElapsedNs)));
        report.put(reportKey(prefix, "surface_release_count"), result.surfaceReleaseCount);
        report.put(reportKey(prefix, "hardware_buffer_target_created"), result.hardwareBufferTargetCreated);
        report.put(reportKey(prefix, "hardware_buffer_reader_width"), result.hardwareBufferReaderWidth);
        report.put(reportKey(prefix, "hardware_buffer_reader_height"), result.hardwareBufferReaderHeight);
        report.put(reportKey(prefix, "hardware_buffer_image_count"), result.hardwareBufferImageCount);
        report.put(reportKey(prefix, "hardware_buffer_delivered_count"), result.hardwareBufferDeliveredCount);
        report.put(reportKey(prefix, "hardware_buffer_missing_count"), result.hardwareBufferMissingCount);
        putStageTimingReport(report, prefix, "hardware_buffer_await_image", result.hardwareBufferAwaitImageTiming);
        putStageTimingReport(report, prefix, "hardware_buffer_get_buffer", result.hardwareBufferGetBufferTiming);
        putStageTimingReport(report, prefix, "hardware_buffer_native_bridge", result.hardwareBufferNativeBridgeTiming);
        report.put(reportKey(prefix, "hardware_buffer_format"), result.lastHardwareBufferFormat);
        report.put(reportKey(prefix, "hardware_buffer_usage"), result.lastHardwareBufferUsage);
        report.put(reportKey(prefix, "hardware_buffer_layers"), result.lastHardwareBufferLayers);
        report.put(reportKey(prefix, "hardware_buffer_id"), result.lastHardwareBufferId);
        report.put(reportKey(prefix, "output_eos_seen"), result.outputEosSeen);
        report.put(reportKey(prefix, "output_format_mime"), result.outputMime);
        report.put(reportKey(prefix, "output_format_width"), result.outputWidth);
        report.put(reportKey(prefix, "output_format_height"), result.outputHeight);
        report.put(reportKey(prefix, "decode_duration_ns"), Math.max(0L, result.decodeEndElapsedNs - result.decodeStartElapsedNs));
    }

    private void logLiveProgress(
        String label,
        LiveDecodeResult result,
        LiveStereoPairer pairer,
        long startedElapsedNs,
        boolean force) {
        long nowNs = SystemClock.elapsedRealtimeNanos();
        if (!force &&
                result.lastProgressLogElapsedNs > 0L &&
                nowNs - result.lastProgressLogElapsedNs < LIVE_PROGRESS_LOG_INTERVAL_NS) {
            return;
        }
        result.lastProgressLogElapsedNs = nowNs;
        try {
            JSONObject progress = new JSONObject();
            progress.put("schema", "rusty.xr.composite.broker_h264_consumer_probe.v1");
            progress.put("source", "composite_app_broker_h264_consumer");
            progress.put("event", "progress");
            progress.put("progress_label", label);
            progress.put("broker_host", config.brokerHost);
            progress.put("broker_port", config.brokerPort);
            progress.put("stream_port", config.streamPort);
            progress.put("right_stream_port", config.rightStreamPort);
            progress.put("preferred_width", config.preferredWidth);
            progress.put("preferred_height", config.preferredHeight);
            progress.put("capture_ms", config.captureMs);
            progress.put("max_packets", config.maxPackets);
            progress.put("bitrate_bps", config.bitrateBps);
            progress.put("frame_rate_hz", config.frameRateHz);
            progress.put("stereo_requested", config.stereo);
            progress.put("live_stream_requested", config.liveStream);
            progress.put("source_mode", config.sourceMode);
            progress.put("broker_synthetic_stream_start_requested", config.startBrokerSyntheticStream);
            progress.put("synthetic_pattern", config.syntheticPattern);
            progress.put("decode_output_mode", config.decodeOutputMode);
            progress.put("live_decode_requested", config.liveDecode);
            progress.put("stereo_pairing_mode_requested", config.stereoPairingMode);
            progress.put("live_decode_path", true);
            progress.put("total_duration_ns", Math.max(0L, nowNs - startedElapsedNs));
            putLiveDecodeReport(progress, label, result);
            long receiveWindowNs = result.receiveEndElapsedNs > result.receiveStartElapsedNs
                ? result.receiveEndElapsedNs - result.receiveStartElapsedNs
                : nowNs - result.receiveStartElapsedNs;
            long decodeWindowNs = result.decodeEndElapsedNs > result.decodeStartElapsedNs
                ? result.decodeEndElapsedNs - result.decodeStartElapsedNs
                : nowNs - result.decodeStartElapsedNs;
            progress.put(reportKey(label, "stream_receive_duration_ns"), Math.max(0L, receiveWindowNs));
            progress.put(
                reportKey(label, "stream_wire_packet_rate_hz"),
                rateHzFromNs(result.packetCount, Math.max(0L, result.lastPacketReceiveElapsedNs - result.firstPacketReceiveElapsedNs)));
            progress.put(
                reportKey(label, "stream_source_packet_rate_hz"),
                rateHzFromNs(result.packetCount, Math.max(0L, result.lastSourceElapsedNs - result.firstSourceElapsedNs)));
            progress.put(reportKey(label, "decode_duration_ns"), Math.max(0L, decodeWindowNs));
            progress.put(
                reportKey(label, "decoded_frame_rate_hz"),
                rateHzFromNs(result.decodedFrameCount, Math.max(0L, decodeWindowNs)));

            StereoPairResult pair = pairer.snapshot();
            progress.put("stereo_pair_count", pair.pairCount);
            progress.put("stereo_pair_native_accepted_count", pair.nativeAcceptedCount);
            progress.put("stereo_pair_native_rejected_count", pair.nativeRejectedCount);
            progress.put("stereo_live_pair_queue_drop_count", pair.queueDropCount);
            progress.put("stereo_pair_delta_avg_ns", pair.pairCount > 0 ? pair.deltaTotalNs / pair.pairCount : 0L);
            progress.put("stereo_pair_delta_max_ns", pair.deltaMaxNs);
            putStereoFrameSetGateReport(progress, pair);
            putStageTimingReport(progress, "", "stereo_pair_native_bridge", pair.nativeBridgeTiming);
            progress.put("succeeded", result.decodedFrameCount > 0 || pair.nativeAcceptedCount > 0);
            Log.i(TAG, "Rusty XR broker H.264 consumer probe: " + progress.toString());
        } catch (Exception ex) {
            Log.w(TAG, "Could not log broker H.264 live progress: " + safeMessage(ex));
        }
    }

    private static void putByteIdentityReport(JSONObject report, String prefix, DecodeResult decode) throws Exception {
        report.put(reportKey(prefix, "byte_identity_decode_output_mode"), decode.decodeOutputMode);
        report.put(reportKey(prefix, "byte_identity_decoder_name"), decode.decoderName);
        report.put(reportKey(prefix, "byte_identity_decoded_frame_count"), decode.decodedFrameCount);
        report.put(reportKey(prefix, "byte_identity_output_frame_crc32_count"), decode.outputFrameHashCount);
        report.put(reportKey(prefix, "byte_identity_output_frame_unique_crc32_count"), decode.outputFrameHashUniqueCount);
        report.put(reportKey(prefix, "byte_identity_output_frame_adjacent_equal_count"), decode.outputFrameHashAdjacentEqualCount);
        report.put(reportKey(prefix, "byte_identity_output_frames_all_identical"), outputFramesAllIdentical(decode));
        report.put(reportKey(prefix, "byte_identity_first_output_frame_crc32"), decode.firstOutputFrameCrc32);
        report.put(reportKey(prefix, "byte_identity_last_output_frame_crc32"), decode.lastOutputFrameCrc32);
        report.put(reportKey(prefix, "byte_identity_output_bytes"), decode.outputBytes);
        report.put(reportKey(prefix, "byte_identity_output_format_mime"), decode.outputMime);
        report.put(reportKey(prefix, "byte_identity_output_format_width"), decode.outputWidth);
        report.put(reportKey(prefix, "byte_identity_output_format_height"), decode.outputHeight);
        report.put(reportKey(prefix, "byte_identity_output_eos_seen"), decode.outputEosSeen);
        report.put(reportKey(prefix, "byte_identity_decode_duration_ns"), Math.max(0L, decode.decodeEndElapsedNs - decode.decodeStartElapsedNs));
        if (decode.lastError.length() > 0) {
            report.put(reportKey(prefix, "byte_identity_last_error"), decode.lastError);
        }
    }

    private static void putStageTimingReport(
        JSONObject report,
        String prefix,
        String key,
        StageTiming timing) throws Exception {
        report.put(reportKey(prefix, key + "_count"), timing.count);
        report.put(reportKey(prefix, key + "_avg_ns"), timing.averageNs());
        report.put(reportKey(prefix, key + "_max_ns"), timing.maxNs);
    }

    private static void putStereoFrameSetGateReport(JSONObject report, StereoPairResult pair) throws Exception {
        report.put("stereo_frame_set_gate_policy", "latest-valid-complete-set");
        report.put("stereo_frame_set_join_window_ns", STEREO_PAIR_MAX_DELTA_NS);
        report.put("stereo_frame_set_max_hold_ns", STEREO_FRAME_SET_MAX_HOLD_NS);
        report.put("stereo_frame_set_stale_ns", STEREO_FRAME_SET_STALE_NS);
        report.put("stereo_frame_set_commit_count", pair.frameSetCommitCount);
        report.put("stereo_frame_set_drop_count", pair.frameSetDropCount);
        report.put("stereo_frame_set_queue_limit_drop_count", pair.frameSetQueueLimitDropCount);
        report.put("stereo_frame_set_stale_drop_count", pair.frameSetStaleDropCount);
        report.put("stereo_frame_set_skew_drop_count", pair.frameSetSkewDropCount);
        report.put("stereo_frame_set_wait_count", pair.frameSetWaitCount);
        report.put("stereo_frame_set_latest_queue_age_ns", pair.lastFrameSetQueueAgeNs);
        report.put("stereo_frame_set_latest_skew_ns", pair.lastFrameSetSkewNs);
        report.put("stereo_frame_set_latest_left_timestamp_ns", pair.lastFrameSetLeftTimestampNs);
        report.put("stereo_frame_set_latest_right_timestamp_ns", pair.lastFrameSetRightTimestampNs);
    }

    private static boolean outputFramesAllIdentical(DecodeResult decode) {
        return decode.outputFrameHashCount > 1 && decode.outputFrameHashUniqueCount == 1;
    }

    private static JSONObject preferredStreamProjectionMetadata(
        StreamResult stream,
        JSONObject startCommandProjectionMetadata) {
        if (stream != null && stream.headerProjectionMetadata != null) {
            return stream.headerProjectionMetadata;
        }
        return startCommandProjectionMetadata;
    }

    private boolean shouldUseLiveStereoDecode() {
        return config.liveStream &&
            config.liveDecode &&
            DECODE_OUTPUT_HARDWARE_BUFFER.equals(config.decodeOutputMode);
    }

    private boolean shouldPaceStereoDelivery() {
        return SOURCE_MODE_EXISTING_STREAM.equals(config.sourceMode) ||
            ((SOURCE_MODE_BROKER_CAMERA.equals(config.sourceMode) ||
                SOURCE_MODE_BROKER_SYNTHETIC.equals(config.sourceMode)) &&
                config.liveStream);
    }

    private static StereoPairResult deliverStereoPairs(
        List<DecodedHardwareBufferFrame> leftFrames,
        List<DecodedHardwareBufferFrame> rightFrames,
        boolean paceDelivery) {
        StereoPairResult result = new StereoPairResult();
        result.deliveryPaced = paceDelivery;
        long deliveryStartNs = SystemClock.elapsedRealtimeNanos();
        List<DecodedHardwareBufferFrame> remainingRight =
            new ArrayList<DecodedHardwareBufferFrame>(rightFrames);
        for (int i = 0; i < leftFrames.size() && !remainingRight.isEmpty(); i++) {
            int pairIndex = result.pairCount;
            if (paceDelivery && pairIndex > 0) {
                paceStereoPairDelivery(deliveryStartNs, pairIndex);
            }
            DecodedHardwareBufferFrame left = leftFrames.get(i);
            DecodedHardwareBufferFrame right = removeNearestFrame(left, remainingRight);
            long deltaNs = Math.abs(left.timestampNs - right.timestampNs);
            result.pairCount++;
            result.frameSetCommitCount++;
            result.lastFrameSetQueueAgeNs = 0L;
            result.lastFrameSetSkewNs = deltaNs;
            result.lastFrameSetLeftTimestampNs = left.timestampNs;
            result.lastFrameSetRightTimestampNs = right.timestampNs;
            result.deltaTotalNs += deltaNs;
            result.deltaMaxNs = Math.max(result.deltaMaxNs, deltaNs);
            if (deltaNs > STEREO_PAIR_MAX_DELTA_NS) {
                result.deltaOverTargetCount++;
            }
            if (left.width != right.width || left.height != right.height) {
                result.resolutionMismatchCount++;
            }
            boolean accepted = false;
            long nativeBridgeStartedNs = SystemClock.elapsedRealtimeNanos();
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
                    pairIndex);
            } catch (RuntimeException error) {
                Log.w(TAG, "Could not deliver broker H.264 decoded stereo hardware-buffer pair", error);
            } finally {
                result.nativeBridgeTiming.record(SystemClock.elapsedRealtimeNanos() - nativeBridgeStartedNs);
            }
            if (accepted) {
                result.nativeAcceptedCount++;
            } else {
                result.nativeRejectedCount++;
            }
        }
        result.deliveryDurationNs = Math.max(0L, SystemClock.elapsedRealtimeNanos() - deliveryStartNs);
        return result;
    }

    private static DecodedHardwareBufferFrame removeNearestFrame(
        DecodedHardwareBufferFrame target,
        List<DecodedHardwareBufferFrame> candidates) {
        int bestIndex = 0;
        long bestDeltaNs = Long.MAX_VALUE;
        for (int i = 0; i < candidates.size(); i++) {
            long deltaNs = Math.abs(target.timestampNs - candidates.get(i).timestampNs);
            if (deltaNs < bestDeltaNs) {
                bestDeltaNs = deltaNs;
                bestIndex = i;
            }
        }
        return candidates.remove(bestIndex);
    }

    private static void paceStereoPairDelivery(long deliveryStartNs, int pairIndex) {
        long targetNs = deliveryStartNs + pairIndex * STEREO_REPLAY_DELIVERY_INTERVAL_NS;
        while (true) {
            long remainingNs = targetNs - SystemClock.elapsedRealtimeNanos();
            if (remainingNs <= 0L) {
                return;
            }
            long sleepMs = Math.min(
                STEREO_REPLAY_DELIVERY_MAX_SLEEP_MS,
                Math.max(1L, remainingNs / 1_000_000L));
            SystemClock.sleep(sleepMs);
        }
    }

    private static void closeFrames(List<DecodedHardwareBufferFrame> frames) {
        for (int i = 0; i < frames.size(); i++) {
            frames.get(i).close();
        }
        frames.clear();
    }

    private static int codecConfigPacketCount(List<Packet> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if ((packets.get(i).flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0) {
                count++;
            }
        }
        return count;
    }

    private static int keyFrameCount(List<Packet> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if ((packets.get(i).flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0) {
                count++;
            }
        }
        return count;
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

    private static long bitrateBpsFromNs(long bytes, long windowNs) {
        return windowNs > 0L ? (long) ((bytes * 8_000_000_000.0) / windowNs) : 0L;
    }

    private static void recordOutputFrameHash(DecodeResult result, long crc32) {
        if (result.outputFrameHashCount == 0) {
            result.firstOutputFrameCrc32 = crc32;
        } else if (result.lastOutputFrameCrc32 == crc32) {
            result.outputFrameHashAdjacentEqualCount++;
        }
        result.lastOutputFrameCrc32 = crc32;
        result.outputFrameHashCount++;
        result.outputFrameHashes.add(Long.valueOf(crc32));
        result.outputFrameHashUniqueCount = result.outputFrameHashes.size();
    }

    private static long crc32Buffer(ByteBuffer buffer, int offset, int size) {
        CRC32 crc32 = new CRC32();
        ByteBuffer copy = buffer.duplicate();
        int start = Math.max(0, offset);
        int end = Math.min(copy.capacity(), start + Math.max(0, size));
        if (start >= end) {
            return crc32.getValue();
        }
        copy.position(start);
        copy.limit(end);
        byte[] chunk = new byte[Math.min(16384, end - start)];
        while (copy.hasRemaining()) {
            int count = Math.min(copy.remaining(), chunk.length);
            copy.get(chunk, 0, count);
            crc32.update(chunk, 0, count);
        }
        return crc32.getValue();
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

    private static final class LiveStereoPairer {
        private final ArrayDeque<DecodedHardwareBufferFrame> leftFrames =
            new ArrayDeque<DecodedHardwareBufferFrame>();
        private final ArrayDeque<DecodedHardwareBufferFrame> rightFrames =
            new ArrayDeque<DecodedHardwareBufferFrame>();
        private final StereoPairResult result = new StereoPairResult();
        private final String pairingMode;
        private final int queueLimit;
        private long deliveryStartNs;
        private long deliveryEndNs;

        LiveStereoPairer(String pairingMode, int queueLimit) {
            this.pairingMode = normalizeStereoPairingMode(pairingMode);
            this.queueLimit = clampInt(queueLimit, 2, MAX_LIVE_STEREO_PENDING_QUEUE_LIMIT);
        }

        synchronized void offer(String eye, DecodedHardwareBufferFrame frame) {
            ArrayDeque<DecodedHardwareBufferFrame> queue = "right".equals(eye) ? rightFrames : leftFrames;
            queue.addLast(frame);
            while (queue.size() > queueLimit) {
                dropFrame(queue, queue.removeFirst(), "queue-limit");
            }
            deliverAvailablePairs();
        }

        synchronized void closePendingFrames() {
            closeQueue(leftFrames);
            closeQueue(rightFrames);
        }

        synchronized StereoPairResult snapshot() {
            StereoPairResult snapshot = new StereoPairResult();
            snapshot.pairCount = result.pairCount;
            snapshot.nativeAcceptedCount = result.nativeAcceptedCount;
            snapshot.nativeRejectedCount = result.nativeRejectedCount;
            snapshot.deliveryPaced = result.deliveryPaced;
            snapshot.deliveryDurationNs = deliveryEndNs > deliveryStartNs
                ? deliveryEndNs - deliveryStartNs
                : result.deliveryDurationNs;
            snapshot.resolutionMismatchCount = result.resolutionMismatchCount;
            snapshot.deltaTotalNs = result.deltaTotalNs;
            snapshot.deltaMaxNs = result.deltaMaxNs;
            snapshot.deltaOverTargetCount = result.deltaOverTargetCount;
            snapshot.queueDropCount = result.queueDropCount;
            snapshot.frameSetCommitCount = result.frameSetCommitCount;
            snapshot.frameSetDropCount = result.frameSetDropCount;
            snapshot.frameSetQueueLimitDropCount = result.frameSetQueueLimitDropCount;
            snapshot.frameSetStaleDropCount = result.frameSetStaleDropCount;
            snapshot.frameSetSkewDropCount = result.frameSetSkewDropCount;
            snapshot.frameSetWaitCount = result.frameSetWaitCount;
            snapshot.lastFrameSetQueueAgeNs = result.lastFrameSetQueueAgeNs;
            snapshot.lastFrameSetSkewNs = result.lastFrameSetSkewNs;
            snapshot.lastFrameSetLeftTimestampNs = result.lastFrameSetLeftTimestampNs;
            snapshot.lastFrameSetRightTimestampNs = result.lastFrameSetRightTimestampNs;
            snapshot.nativeBridgeTiming.copyFrom(result.nativeBridgeTiming);
            return snapshot;
        }

        private void deliverAvailablePairs() {
            while (true) {
                long nowNs = SystemClock.elapsedRealtimeNanos();
                dropStaleFrames(nowNs);
                if (leftFrames.isEmpty() || rightFrames.isEmpty()) {
                    return;
                }
                DecodedHardwareBufferFrame left = null;
                DecodedHardwareBufferFrame right = null;
                if (STEREO_PAIRING_FRAME_ORDER.equals(pairingMode)) {
                    left = leftFrames.peekFirst();
                    right = rightFrames.peekFirst();
                } else {
                    long bestDeltaNs = Long.MAX_VALUE;
                    for (DecodedHardwareBufferFrame leftCandidate : leftFrames) {
                        for (DecodedHardwareBufferFrame rightCandidate : rightFrames) {
                            long deltaNs = Math.abs(leftCandidate.timestampNs - rightCandidate.timestampNs);
                            if (deltaNs < bestDeltaNs) {
                                bestDeltaNs = deltaNs;
                                left = leftCandidate;
                                right = rightCandidate;
                            }
                        }
                    }
                }
                if (left == null || right == null) {
                    return;
                }
                long deltaNs = Math.abs(left.timestampNs - right.timestampNs);
                if (deltaNs > STEREO_PAIR_MAX_DELTA_NS) {
                    if (shouldHoldForBetterFrame(left, right, nowNs)) {
                        result.frameSetWaitCount++;
                        return;
                    }
                    dropOlderSkewFrame(left, right);
                    continue;
                }
                leftFrames.remove(left);
                rightFrames.remove(right);
                deliverPair(left, right, nowNs);
            }
        }

        private boolean shouldHoldForBetterFrame(
            DecodedHardwareBufferFrame left,
            DecodedHardwareBufferFrame right,
            long nowNs) {
            if (leftFrames.size() > 1 || rightFrames.size() > 1) {
                return false;
            }
            long oldestAgeNs = Math.max(
                Math.max(0L, nowNs - left.retainedElapsedNs),
                Math.max(0L, nowNs - right.retainedElapsedNs));
            return oldestAgeNs < STEREO_FRAME_SET_MAX_HOLD_NS;
        }

        private void dropStaleFrames(long nowNs) {
            dropStaleFrames(leftFrames, nowNs);
            dropStaleFrames(rightFrames, nowNs);
        }

        private void dropStaleFrames(ArrayDeque<DecodedHardwareBufferFrame> queue, long nowNs) {
            while (!queue.isEmpty()) {
                DecodedHardwareBufferFrame frame = queue.peekFirst();
                long ageNs = Math.max(0L, nowNs - frame.retainedElapsedNs);
                if (ageNs <= STEREO_FRAME_SET_STALE_NS) {
                    return;
                }
                dropFrame(queue, queue.removeFirst(), "stale");
            }
        }

        private void dropOlderSkewFrame(DecodedHardwareBufferFrame left, DecodedHardwareBufferFrame right) {
            if (left.timestampNs <= right.timestampNs) {
                dropFrame(leftFrames, left, "skew");
            } else {
                dropFrame(rightFrames, right, "skew");
            }
        }

        private void dropFrame(
            ArrayDeque<DecodedHardwareBufferFrame> queue,
            DecodedHardwareBufferFrame frame,
            String reason) {
            queue.remove(frame);
            frame.close();
            result.frameSetDropCount++;
            if ("queue-limit".equals(reason)) {
                result.queueDropCount++;
                result.frameSetQueueLimitDropCount++;
            } else if ("stale".equals(reason)) {
                result.frameSetStaleDropCount++;
            } else if ("skew".equals(reason)) {
                result.frameSetSkewDropCount++;
            }
        }

        private void deliverPair(DecodedHardwareBufferFrame left, DecodedHardwareBufferFrame right, long nowNs) {
            if (deliveryStartNs == 0L) {
                deliveryStartNs = SystemClock.elapsedRealtimeNanos();
            }
            long deltaNs = Math.abs(left.timestampNs - right.timestampNs);
            long pairIndex = result.pairCount;
            result.pairCount++;
            result.frameSetCommitCount++;
            result.lastFrameSetQueueAgeNs = Math.max(
                0L,
                nowNs - Math.max(left.retainedElapsedNs, right.retainedElapsedNs));
            result.lastFrameSetSkewNs = deltaNs;
            result.lastFrameSetLeftTimestampNs = left.timestampNs;
            result.lastFrameSetRightTimestampNs = right.timestampNs;
            result.deltaTotalNs += deltaNs;
            result.deltaMaxNs = Math.max(result.deltaMaxNs, deltaNs);
            if (deltaNs > STEREO_PAIR_MAX_DELTA_NS) {
                result.deltaOverTargetCount++;
            }
            if (left.width != right.width || left.height != right.height) {
                result.resolutionMismatchCount++;
            }
            boolean accepted = false;
            long nativeBridgeStartedNs = SystemClock.elapsedRealtimeNanos();
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
                    pairIndex);
            } catch (RuntimeException error) {
                Log.w(TAG, "Could not deliver live broker H.264 decoded stereo hardware-buffer pair", error);
            } finally {
                result.nativeBridgeTiming.record(SystemClock.elapsedRealtimeNanos() - nativeBridgeStartedNs);
                left.close();
                right.close();
                deliveryEndNs = SystemClock.elapsedRealtimeNanos();
                result.deliveryDurationNs = deliveryEndNs - deliveryStartNs;
            }
            if (accepted) {
                result.nativeAcceptedCount++;
            } else {
                result.nativeRejectedCount++;
            }
        }

        private static void closeQueue(ArrayDeque<DecodedHardwareBufferFrame> queue) {
            while (!queue.isEmpty()) {
                queue.removeFirst().close();
            }
        }
    }

    private final class LiveDecodeStreamTask implements Runnable {
        private final String label;
        private final int streamPort;
        private final String cameraId;
        private final JSONObject streamProjectionMetadata;
        private final LiveStereoPairer pairer;
        private final long startedElapsedNs;
        private final Thread thread;
        private LiveDecodeResult result;
        private Exception error;

        LiveDecodeStreamTask(
            String label,
            int streamPort,
            String cameraId,
            JSONObject streamProjectionMetadata,
            LiveStereoPairer pairer,
            long startedElapsedNs) {
            this.label = label;
            this.streamPort = streamPort;
            this.cameraId = cameraId;
            this.streamProjectionMetadata = streamProjectionMetadata;
            this.pairer = pairer;
            this.startedElapsedNs = startedElapsedNs;
            this.thread = new Thread(this, "RustyXrBrokerH264LiveDecode-" + label);
        }

        void start() {
            Log.i(TAG, "Rusty XR broker H.264 live decode thread start requested: " + label);
            thread.start();
        }

        @Override
        public void run() {
            try {
                Log.i(TAG, String.format(
                    Locale.US,
                    "Rusty XR broker H.264 live decode thread running: label=%s target=%s:%d cameraId=%s metadataReady=%s",
                    label,
                    config.brokerHost,
                    streamPort,
                    cameraId,
                    streamProjectionMetadata != null &&
                        streamProjectionMetadata.optBoolean("projectionMetadataReady", false)));
                result = decodeLiveStream(label, streamPort, cameraId, streamProjectionMetadata, pairer, startedElapsedNs);
                Log.i(TAG, String.format(
                    Locale.US,
                    "Rusty XR broker H.264 live decode thread completed: label=%s packets=%d decodedFrames=%d nativeAcceptedPending=%d",
                    label,
                    result.packetCount,
                    result.decodedFrameCount,
                    pairer.snapshot().nativeAcceptedCount));
            } catch (Exception ex) {
                error = ex;
                Log.w(TAG, "Rusty XR broker H.264 live decode thread failed: " + label + ": " + safeMessage(ex), ex);
            }
        }

        LiveDecodeResult awaitResult() throws Exception {
            long waitMs = (long) config.streamTimeoutMs + config.decodeTimeoutMs + config.captureMs + 2000L;
            thread.join(waitMs);
            if (thread.isAlive()) {
                throw new IllegalStateException("Timed out waiting for " + label + " broker H.264 live decode task.");
            }
            if (error != null) {
                throw error;
            }
            if (result == null) {
                throw new IllegalStateException("Broker H.264 " + label + " live decode produced no result.");
            }
            return result;
        }
    }

    private LiveDecodeResult decodeLiveStream(
        String label,
        int streamPort,
        String cameraId,
        JSONObject streamProjectionMetadata,
        LiveStereoPairer pairer,
        long startedElapsedNs) throws Exception {
        Log.i(TAG, String.format(
            Locale.US,
            "Rusty XR broker H.264 live decode connecting: label=%s target=%s:%d cameraId=%s",
            label,
            config.brokerHost,
            streamPort,
            cameraId));
        Socket socket = connectWithRetry(config.brokerHost, streamPort, config.streamTimeoutMs, label);
        Log.i(TAG, "Rusty XR broker H.264 live decode connected: " + label + " target=" +
            config.brokerHost + ":" + streamPort);
        if ("right".equals(label)) {
            rightStreamSocket = socket;
        } else {
            streamSocket = socket;
        }
        socket.setSoTimeout(config.streamTimeoutMs);
        DataInputStream input = new DataInputStream(socket.getInputStream());
        LiveDecodeResult result = new LiveDecodeResult();
        DecodeHardwareBufferTarget hardwareBufferTarget = null;
        MediaCodec decoder = null;
        try {
            byte[] magicBytes = new byte[8];
            input.readFully(magicBytes);
            String magic = new String(magicBytes, StandardCharsets.US_ASCII);
            if (!STREAM_MAGIC.equals(magic)) {
                throw new IllegalStateException("Unexpected stream magic: " + magic);
            }

            result.schemaVersion = input.readInt();
            result.codecId = input.readInt();
            result.width = input.readInt();
            result.height = input.readInt();
            result.declaredPacketCount = input.readInt();
            result.headerMetadataBytes = input.readInt();
            if (result.schemaVersion < 1 || result.schemaVersion > 3) {
                throw new IllegalStateException("Unsupported broker stream schema version: " + result.schemaVersion);
            }
            result.headerProjectionMetadata = readStreamHeaderProjectionMetadata(
                input,
                result.schemaVersion,
                result.headerMetadataBytes,
                label);
            if (result.codecId != CODEC_H264) {
                throw new IllegalStateException("Broker stream codec is not H.264: " + result.codecId);
            }
            if (result.declaredPacketCount < 0 || result.declaredPacketCount > MAX_STREAM_PACKETS) {
                throw new IllegalStateException("Broker stream packet count is out of range: " + result.declaredPacketCount);
            }
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 live stream header: label=%s schema=%d codec=%d width=%d height=%d declaredPackets=%d",
                label,
                result.schemaVersion,
                result.codecId,
                result.width,
                result.height,
                result.declaredPacketCount));
            JSONObject effectiveStreamProjectionMetadata = result.headerProjectionMetadata != null
                ? result.headerProjectionMetadata
                : streamProjectionMetadata;
            result.sessionProjectionMetadataSource = result.headerProjectionMetadata != null
                ? "stream-header"
                : (streamProjectionMetadata != null ? "start-command" : "none");
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 live projection metadata source: label=%s source=%s headerAttached=%s ready=%s cameraId=%s",
                label,
                result.sessionProjectionMetadataSource,
                result.headerProjectionMetadata != null,
                effectiveStreamProjectionMetadata != null &&
                    effectiveStreamProjectionMetadata.optBoolean("projectionMetadataReady", false),
                effectiveStreamProjectionMetadata != null
                    ? effectiveStreamProjectionMetadata.optString("cameraId", "")
                    : ""));

            boolean unboundedStream = result.declaredPacketCount == 0;
            result.receiveStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            List<Packet> pendingPackets = new ArrayList<Packet>();
            while (running &&
                (unboundedStream || pendingPackets.size() < result.declaredPacketCount) &&
                pendingPackets.size() < 8) {
                Packet packet;
                try {
                    packet = readPacket(input, result.schemaVersion);
                } catch (EOFException eof) {
                    markLiveStreamEndedByEof(result);
                    break;
                }
                recordLivePacket(result, packet);
                pendingPackets.add(packet);
                if (findNalUnit(pendingPackets, 7) != null && findNalUnit(pendingPackets, 8) != null) {
                    break;
                }
            }
            if (pendingPackets.isEmpty()) {
                throw new IllegalStateException("Broker H.264 " + label + " live stream ended before any packets were received.");
            }
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 live stream primed: label=%s pendingPackets=%d packetCount=%d sps=%s pps=%s",
                label,
                pendingPackets.size(),
                result.packetCount,
                findNalUnit(pendingPackets, 7) != null,
                findNalUnit(pendingPackets, 8) != null));

            NalUnit sps = findNalUnit(pendingPackets, 7);
            NalUnit pps = findNalUnit(pendingPackets, 8);
            result.spsBytes = sps != null ? sps.bytes.length : 0;
            result.ppsBytes = pps != null ? pps.bytes.length : 0;
            MediaFormat format = MediaFormat.createVideoFormat("video/avc", result.width, result.height);
            if (sps != null) {
                format.setByteBuffer("csd-0", ByteBuffer.wrap(sps.bytes));
            }
            if (pps != null) {
                format.setByteBuffer("csd-1", ByteBuffer.wrap(pps.bytes));
            }

            hardwareBufferTarget = DecodeHardwareBufferTarget.create(result.width, result.height);
            result.hardwareBufferTargetCreated = true;
            result.hardwareBufferReaderWidth = result.width;
            result.hardwareBufferReaderHeight = result.height;
            decoder = MediaCodec.createDecoderByType("video/avc");
            result.decoderName = decoder.getName();
            result.lowLatencyFeatureSupported = decoderLowLatencySupported(decoder);
            result.lowLatencyConfigRequested = true;
            format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1);
            decoder.configure(format, hardwareBufferTarget.surface(), null, 0);
            decoder.start();
            result.lowLatencyParameterSucceeded = requestDecoderLowLatency(decoder);
            result.decodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            Log.i(TAG, String.format(
                Locale.US,
                "Rusty XR broker H.264 live decoder started: label=%s decoder=%s lowLatencySupported=%s lowLatencyRequested=%s",
                label,
                result.decoderName,
                result.lowLatencyFeatureSupported,
                result.lowLatencyParameterSucceeded));

            MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
            int nextPending = 0;
            long deadlineNs = unboundedStream || config.captureMs <= 0
                ? Long.MAX_VALUE
                : SystemClock.elapsedRealtimeNanos() +
                    ((long) config.streamTimeoutMs + config.decodeTimeoutMs + config.captureMs) * 1_000_000L;
            while (running && !result.outputEosSeen && SystemClock.elapsedRealtimeNanos() < deadlineNs) {
                if (!result.inputEosQueued) {
                    int inputIndex = decoder.dequeueInputBuffer(DEQUEUE_TIMEOUT_US);
                    if (inputIndex >= 0) {
                        if (nextPending < pendingPackets.size()) {
                            Packet packet = pendingPackets.get(nextPending++);
                            queueLivePacket(decoder, inputIndex, packet, result);
                        } else if (!result.streamEndedByEof && (unboundedStream || result.packetCount < result.declaredPacketCount)) {
                            Packet packet;
                            try {
                                packet = readPacket(input, result.schemaVersion);
                            } catch (EOFException eof) {
                                markLiveStreamEndedByEof(result);
                                queueLiveEos(decoder, inputIndex, result);
                                continue;
                            }
                            recordLivePacket(result, packet);
                            queueLivePacket(decoder, inputIndex, packet, result);
                        } else {
                            queueLiveEos(decoder, inputIndex, result);
                        }
                    }
                }

                int outputIndex = decoder.dequeueOutputBuffer(info, DEQUEUE_TIMEOUT_US);
                if (outputIndex == MediaCodec.INFO_TRY_AGAIN_LATER) {
                    continue;
                }
                if (outputIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    result.outputFormatChanges++;
                    applyLiveOutputFormat(result, decoder.getOutputFormat());
                    continue;
                }
                if (outputIndex < 0) {
                    continue;
                }

                boolean codecConfig = (info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0;
                boolean eos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
                if (!codecConfig && !eos) {
                    result.outputBufferCount++;
                    decoder.releaseOutputBuffer(outputIndex, true);
                    result.surfaceReleaseCount++;
                    List<DecodedHardwareBufferFrame> frame = new ArrayList<DecodedHardwareBufferFrame>(1);
                    DecodeHardwareBufferTarget.DeliverResult deliver = hardwareBufferTarget.awaitAndRetainFrame(
                        HARDWARE_BUFFER_WAIT_MS,
                        cameraId,
                        label,
                        info.presentationTimeUs,
                        sourceElapsedNsForLivePts(result, info.presentationTimeUs),
                        effectiveStreamProjectionMetadata,
                        frame);
                    recordHardwareBufferTiming(result, deliver);
                    result.hardwareBufferImageCount = hardwareBufferTarget.imageCount();
                    result.hardwareBufferDeliveredCount = hardwareBufferTarget.deliveredCount();
                    result.hardwareBufferMissingCount = hardwareBufferTarget.missingBufferCount();
                    if (deliver.delivered && frame.size() > 0) {
                        DecodedHardwareBufferFrame decodedFrame = frame.remove(0);
                        result.decodedFrameCount++;
                        result.lastHardwareBufferFormat = deliver.format;
                        result.lastHardwareBufferUsage = deliver.usage;
                        result.lastHardwareBufferLayers = deliver.layers;
                        result.lastHardwareBufferId = deliver.bufferId;
                        pairer.offer(label, decodedFrame);
                        logLiveProgress(label, result, pairer, startedElapsedNs, false);
                    }
                    closeFrames(frame);
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
                applyLiveOutputFormat(result, decoder.getOutputFormat());
            }
            result.receiveEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            logLiveProgress(label, result, pairer, startedElapsedNs, true);
        } finally {
            result.decodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (decoder != null) {
                try {
                    decoder.stop();
                } catch (Exception ignored) {
                }
                decoder.release();
            }
            if (hardwareBufferTarget != null) {
                hardwareBufferTarget.release();
            }
            closeQuietly(socket);
            if ("right".equals(label)) {
                rightStreamSocket = null;
            } else {
                streamSocket = null;
            }
        }
        return result;
    }

    private static void queueLivePacket(MediaCodec decoder, int inputIndex, Packet packet, LiveDecodeResult result) throws Exception {
        queuePacket(decoder, inputIndex, packet);
        result.inputBufferCount++;
        result.inputBytes += packet.payload.length;
        result.lastPresentationTimeUs = packet.ptsUs;
    }

    private static void queueLiveEos(MediaCodec decoder, int inputIndex, LiveDecodeResult result) {
        decoder.queueInputBuffer(
            inputIndex,
            0,
            0,
            result.lastPresentationTimeUs,
            MediaCodec.BUFFER_FLAG_END_OF_STREAM);
        result.inputEosQueued = true;
    }

    private static void markLiveStreamEndedByEof(LiveDecodeResult result) {
        result.streamEndedByEof = true;
        result.streamMissingDeclaredPacketCount = result.declaredPacketCount > 0
            ? Math.max(0, result.declaredPacketCount - result.packetCount)
            : 0;
    }

    private static void recordLivePacket(LiveDecodeResult result, Packet packet) {
        result.packetCount++;
        result.payloadBytes += packet.payload.length;
        if (result.firstPacketReceiveElapsedNs == 0L) {
            result.firstPacketReceiveElapsedNs = packet.receiveElapsedNs;
        }
        result.lastPacketReceiveElapsedNs = packet.receiveElapsedNs;
        if (packet.sourceElapsedNs > 0L) {
            if (result.firstSourceElapsedNs == 0L) {
                result.firstSourceElapsedNs = packet.sourceElapsedNs;
            }
            result.lastSourceElapsedNs = packet.sourceElapsedNs;
            result.sourceElapsedByPts.put(Long.valueOf(packet.ptsUs), Long.valueOf(packet.sourceElapsedNs));
        }
    }

    private static long sourceElapsedNsForLivePts(LiveDecodeResult result, long ptsUs) {
        Long exact = result.sourceElapsedByPts.get(Long.valueOf(ptsUs));
        if (exact != null) {
            return exact.longValue();
        }
        return result.lastSourceElapsedNs;
    }

    private Packet readPacket(DataInputStream input, int schemaVersion) throws Exception {
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
        return new Packet(
            ptsUs,
            flags,
            sourceElapsedNs,
            sourceUnixNs,
            SystemClock.elapsedRealtimeNanos(),
            payload);
    }

    private JSONObject readStreamHeaderProjectionMetadata(
        DataInputStream input,
        int schemaVersion,
        int metadataBytes,
        String label) throws Exception {
        if (schemaVersion < 3 || metadataBytes <= 0) {
            return null;
        }
        if (metadataBytes > MAX_STREAM_HEADER_METADATA_BYTES) {
            throw new IllegalStateException("Broker stream header metadata is too large: " + metadataBytes);
        }
        byte[] payload = new byte[metadataBytes];
        input.readFully(payload);
        String json = new String(payload, StandardCharsets.UTF_8);
        JSONObject metadata = new JSONObject(json);
        Log.i(TAG, String.format(
            Locale.US,
            "Rusty XR broker H.264 stream header projection metadata: label=%s bytes=%d cameraId=%s ready=%s source=%s",
            label,
            metadataBytes,
            metadata.optString("cameraId", ""),
            metadata.optBoolean("projectionMetadataReady", false),
            metadata.optString("source", "")));
        return metadata;
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
        int headerMetadataBytes = input.readInt();
        if (schemaVersion < 1 || schemaVersion > 3) {
            throw new IllegalStateException("Unsupported broker stream schema version: " + schemaVersion);
        }
        JSONObject headerProjectionMetadata = readStreamHeaderProjectionMetadata(
            input,
            schemaVersion,
            headerMetadataBytes,
            label);
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
            Packet packet = readPacket(input, schemaVersion);
            long packetReceiveElapsedNs = packet.receiveElapsedNs;
            if (firstPacketReceiveElapsedNs == 0L) {
                firstPacketReceiveElapsedNs = packetReceiveElapsedNs;
            }
            lastPacketReceiveElapsedNs = packetReceiveElapsedNs;
            if (packet.sourceElapsedNs > 0L) {
                if (firstSourceElapsedNs == 0L) {
                    firstSourceElapsedNs = packet.sourceElapsedNs;
                }
                lastSourceElapsedNs = packet.sourceElapsedNs;
            }
            packets.add(packet);
            payloadBytes += packet.payload.length;
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
            headerMetadataBytes,
            headerProjectionMetadata,
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
            result.lowLatencyFeatureSupported = decoderLowLatencySupported(decoder);
            result.lowLatencyConfigRequested = true;
            format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1);
            decoder.configure(format, decoderSurface, null, 0);
            decoder.start();
            result.lowLatencyParameterSucceeded = requestDecoderLowLatency(decoder);
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
                        recordHardwareBufferTiming(result, deliver);
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
                            ByteBuffer outputBuffer = decoder.getOutputBuffer(outputIndex);
                            if (outputBuffer != null) {
                                recordOutputFrameHash(
                                    result,
                                    crc32Buffer(outputBuffer, info.offset, info.size));
                            }
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

    private static String normalizeSourceMode(String value) {
        if (value == null || value.trim().length() == 0) {
            return SOURCE_MODE_BROKER_CAMERA;
        }
        String normalized = value.trim().toLowerCase(Locale.US).replace('_', '-');
        if ("existing".equals(normalized) ||
            "existing-stream".equals(normalized) ||
            "remote".equals(normalized) ||
            "proxied".equals(normalized) ||
            "proxy".equals(normalized) ||
            "proxy-stream".equals(normalized) ||
            "incoming".equals(normalized) ||
            "incoming-stream".equals(normalized)) {
            return SOURCE_MODE_EXISTING_STREAM;
        }
        if ("synthetic".equals(normalized) ||
            "broker-synthetic".equals(normalized) ||
            "synthetic-stream".equals(normalized) ||
            "diagnostic".equals(normalized) ||
            "diagnostic-stream".equals(normalized)) {
            return SOURCE_MODE_BROKER_SYNTHETIC;
        }
        return SOURCE_MODE_BROKER_CAMERA;
    }

    private static String normalizeSyntheticPattern(String value) {
        if (value == null || value.trim().length() == 0) {
            return DEFAULT_SYNTHETIC_PATTERN;
        }
        String normalized = value.trim().toLowerCase(Locale.US).replace('_', '-');
        if ("checker".equals(normalized) || "checkerboard".equals(normalized)) {
            return "checkerboard";
        }
        if ("ramp".equals(normalized) || "luma".equals(normalized) || "luma-ramp".equals(normalized)) {
            return "luma-ramp";
        }
        if ("motion".equals(normalized) || "motion-bar".equals(normalized)) {
            return "motion-bar";
        }
        return DEFAULT_SYNTHETIC_PATTERN;
    }

    private static String normalizeSyntheticProjectionProfile(String value) {
        if (value == null || value.trim().length() == 0) {
            return DEFAULT_SYNTHETIC_PROJECTION_PROFILE;
        }
        String normalized = value.trim().toLowerCase(Locale.US).replace('_', '-');
        if ("camera-matched".equals(normalized) || "camera-matched-synthetic".equals(normalized)) {
            return "camera-matched";
        }
        if ("full-frame".equals(normalized) ||
            "full-frame-diagnostic".equals(normalized) ||
            "projection-space-diagnostic".equals(normalized)) {
            return "full-frame-diagnostic";
        }
        if (DEFAULT_SYNTHETIC_PROJECTION_PROFILE.equals(normalized)) {
            return DEFAULT_SYNTHETIC_PROJECTION_PROFILE;
        }
        return DEFAULT_SYNTHETIC_PROJECTION_PROFILE;
    }

    private static String normalizeStereoPairingMode(String value) {
        if (value == null || value.trim().length() == 0) {
            return STEREO_PAIRING_TIMESTAMP_NEAREST;
        }
        String normalized = value.trim().toLowerCase(Locale.US).replace('_', '-');
        if ("frame".equals(normalized) ||
            "frame-order".equals(normalized) ||
            "frame-index".equals(normalized) ||
            "ordered".equals(normalized) ||
            "fifo".equals(normalized)) {
            return STEREO_PAIRING_FRAME_ORDER;
        }
        return STEREO_PAIRING_TIMESTAMP_NEAREST;
    }

    private static int clampInt(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
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
            long awaitImageStartedNs = SystemClock.elapsedRealtimeNanos();
            Image image = null;
            while (SystemClock.elapsedRealtime() < deadline) {
                try {
                    image = reader.acquireNextImage();
                } catch (IllegalStateException error) {
                    return DeliverResult.notDelivered(
                        SystemClock.elapsedRealtimeNanos() - awaitImageStartedNs,
                        0L,
                        0L);
                }
                if (image != null) {
                    break;
                }
                SystemClock.sleep(5);
            }
            if (image == null) {
                return DeliverResult.notDelivered(
                    SystemClock.elapsedRealtimeNanos() - awaitImageStartedNs,
                    0L,
                    0L);
            }
            long awaitImageNs = SystemClock.elapsedRealtimeNanos() - awaitImageStartedNs;

            HardwareBuffer buffer = null;
            long getBufferNs = 0L;
            long nativeBridgeNs = 0L;
            try {
                imageCount++;
                long getBufferStartedNs = SystemClock.elapsedRealtimeNanos();
                buffer = image.getHardwareBuffer();
                getBufferNs = SystemClock.elapsedRealtimeNanos() - getBufferStartedNs;
                if (buffer == null) {
                    missingBufferCount++;
                    return DeliverResult.notDelivered(awaitImageNs, getBufferNs, nativeBridgeNs);
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
                long nativeBridgeStartedNs = SystemClock.elapsedRealtimeNanos();
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
                nativeBridgeNs = SystemClock.elapsedRealtimeNanos() - nativeBridgeStartedNs;
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
                    bufferId,
                    awaitImageNs,
                    getBufferNs,
                    nativeBridgeNs);
            } catch (RuntimeException error) {
                nativeRejectedCount++;
                Log.w(TAG, "Could not deliver broker H.264 decoded hardware buffer", error);
                return DeliverResult.notDelivered(awaitImageNs, getBufferNs, nativeBridgeNs);
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
            long awaitImageStartedNs = SystemClock.elapsedRealtimeNanos();
            Image image = null;
            while (SystemClock.elapsedRealtime() < deadline) {
                try {
                    image = reader.acquireNextImage();
                } catch (IllegalStateException error) {
                    return DeliverResult.notDelivered(
                        SystemClock.elapsedRealtimeNanos() - awaitImageStartedNs,
                        0L,
                        0L);
                }
                if (image != null) {
                    break;
                }
                SystemClock.sleep(5);
            }
            if (image == null) {
                return DeliverResult.notDelivered(
                    SystemClock.elapsedRealtimeNanos() - awaitImageStartedNs,
                    0L,
                    0L);
            }
            long awaitImageNs = SystemClock.elapsedRealtimeNanos() - awaitImageStartedNs;

            HardwareBuffer buffer = null;
            long getBufferNs = 0L;
            try {
                imageCount++;
                long getBufferStartedNs = SystemClock.elapsedRealtimeNanos();
                buffer = image.getHardwareBuffer();
                getBufferNs = SystemClock.elapsedRealtimeNanos() - getBufferStartedNs;
                if (buffer == null) {
                    missingBufferCount++;
                    return DeliverResult.notDelivered(awaitImageNs, getBufferNs, 0L);
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
                    SystemClock.elapsedRealtimeNanos(),
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
                    bufferId,
                    awaitImageNs,
                    getBufferNs,
                    0L);
            } catch (RuntimeException error) {
                Log.w(TAG, "Could not retain broker H.264 decoded hardware buffer", error);
                if (buffer != null) {
                    try {
                        buffer.close();
                    } catch (RuntimeException ignored) {
                    }
                }
                return DeliverResult.notDelivered(awaitImageNs, getBufferNs, 0L);
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
            final long awaitImageNs;
            final long getBufferNs;
            final long nativeBridgeNs;

            DeliverResult(
                boolean delivered,
                boolean accepted,
                int format,
                long usage,
                int layers,
                long bufferId,
                long awaitImageNs,
                long getBufferNs,
                long nativeBridgeNs) {
                this.delivered = delivered;
                this.accepted = accepted;
                this.format = format;
                this.usage = usage;
                this.layers = layers;
                this.bufferId = bufferId;
                this.awaitImageNs = awaitImageNs;
                this.getBufferNs = getBufferNs;
                this.nativeBridgeNs = nativeBridgeNs;
            }

            static DeliverResult notDelivered() {
                return notDelivered(0L, 0L, 0L);
            }

            static DeliverResult notDelivered(long awaitImageNs, long getBufferNs, long nativeBridgeNs) {
                return new DeliverResult(false, false, 0, 0L, 0, 0L, awaitImageNs, getBufferNs, nativeBridgeNs);
            }
        }
    }

    private static void recordHardwareBufferTiming(
        LiveDecodeResult result,
        DecodeHardwareBufferTarget.DeliverResult deliver) {
        recordHardwareBufferTiming(
            result.hardwareBufferAwaitImageTiming,
            result.hardwareBufferGetBufferTiming,
            result.hardwareBufferNativeBridgeTiming,
            deliver);
    }

    private static void recordHardwareBufferTiming(
        DecodeResult result,
        DecodeHardwareBufferTarget.DeliverResult deliver) {
        recordHardwareBufferTiming(
            result.hardwareBufferAwaitImageTiming,
            result.hardwareBufferGetBufferTiming,
            result.hardwareBufferNativeBridgeTiming,
            deliver);
    }

    private static void recordHardwareBufferTiming(
        StageTiming awaitImageTiming,
        StageTiming getBufferTiming,
        StageTiming nativeBridgeTiming,
        DecodeHardwareBufferTarget.DeliverResult deliver) {
        if (deliver.awaitImageNs > 0L) {
            awaitImageTiming.record(deliver.awaitImageNs);
        }
        if (deliver.getBufferNs > 0L) {
            getBufferTiming.record(deliver.getBufferNs);
        }
        if (deliver.nativeBridgeNs > 0L) {
            nativeBridgeTiming.record(deliver.nativeBridgeNs);
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

            metadata.put(
                "source",
                hasStreamProjectionMetadata
                    ? streamProjectionMetadata.optString("source", "broker_app.h264_stream")
                    : "broker_app.h264_stream");
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
            copyOptionalStreamString(streamProjectionMetadata, metadata, "syntheticPattern");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "syntheticSideMarker");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "syntheticProjectionProfile");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "projectionGeometryProfile");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "syntheticProjectionProfileRequested");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "syntheticProjectionProfileFallbackReason");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "syntheticGeometryReferenceCameraId");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "syntheticGeometryReferenceSource");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "rasterOrientationSchema");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "orientationKind");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "rasterOrientation");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "rasterOrigin");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "rasterYAxis");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "uprightMarker");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "orientationMetadataSource");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "stimulusOrientationSchema");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "stimulusRasterOrientation");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "stimulusOrigin");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "stimulusYAxis");
            copyOptionalStreamString(streamProjectionMetadata, metadata, "stimulusUprightMarker");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentGeometrySchema");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentKind");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentWidth");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentHeight");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentAspectRatio");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "desiredDisplayAspectRatio");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "desiredProjectionAspectRatio");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentCoordinateSpace");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentOrigin");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentXAxis");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentYAxis");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentUvRect");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentMappingIntent");
            copyOptionalStreamValue(streamProjectionMetadata, metadata, "contentGeometryMetadataSource");
            if (hasStreamProjectionMetadata && streamProjectionMetadata.has("diagnosticSource")) {
                metadata.put("diagnosticSource", streamProjectionMetadata.optBoolean("diagnosticSource", false));
            }
            if (hasStreamProjectionMetadata && streamProjectionMetadata.has("stimulusOrientationDefault")) {
                metadata.put(
                    "stimulusOrientationDefault",
                    streamProjectionMetadata.optBoolean("stimulusOrientationDefault", false));
            }
            if (hasStreamProjectionMetadata && streamProjectionMetadata.has("orientationDefault")) {
                metadata.put(
                    "orientationDefault",
                    streamProjectionMetadata.optBoolean("orientationDefault", false));
            }
            if (hasStreamProjectionMetadata && streamProjectionMetadata.has("contentGeometryDefault")) {
                metadata.put(
                    "contentGeometryDefault",
                    streamProjectionMetadata.optBoolean("contentGeometryDefault", false));
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

    private static void copyOptionalStreamString(
        JSONObject source,
        JSONObject target,
        String key) throws Exception {
        if (source != null && source.has(key)) {
            target.put(key, source.optString(key, ""));
        }
    }

    private static void copyOptionalStreamValue(
        JSONObject source,
        JSONObject target,
        String key) throws Exception {
        if (source != null && source.has(key)) {
            target.put(key, source.get(key));
        }
    }

    private Socket connectWithRetry(String host, int port, int timeoutMs, String label) throws Exception {
        long deadline = SystemClock.elapsedRealtimeNanos() + timeoutMs * 1_000_000L;
        Exception lastError = null;
        int attempts = 0;
        Log.i(TAG, String.format(
            Locale.US,
            "Rusty XR broker H.264 connect retry begin: label=%s target=%s:%d timeoutMs=%d",
            label,
            host,
            port,
            timeoutMs));
        while (running && SystemClock.elapsedRealtimeNanos() < deadline) {
            Socket socket = new Socket();
            try {
                attempts++;
                socket.connect(new InetSocketAddress(host, port), 500);
                socket.setTcpNoDelay(true);
                Log.i(TAG, String.format(
                    Locale.US,
                    "Rusty XR broker H.264 connect retry succeeded: label=%s target=%s:%d attempts=%d",
                    label,
                    host,
                    port,
                    attempts));
                return socket;
            } catch (Exception ex) {
                lastError = ex;
                closeQuietly(socket);
                Thread.sleep(50);
            }
        }

        Log.w(TAG, String.format(
            Locale.US,
            "Rusty XR broker H.264 connect retry exhausted: label=%s target=%s:%d attempts=%d lastError=%s",
            label,
            host,
            port,
            attempts,
            lastError != null ? safeMessage(lastError) : ""));
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

    private static void applyLiveOutputFormat(LiveDecodeResult result, MediaFormat format) {
        result.outputMime = mediaFormatString(format, MediaFormat.KEY_MIME, "video/raw");
        result.outputWidth = mediaFormatInt(format, MediaFormat.KEY_WIDTH, result.width);
        result.outputHeight = mediaFormatInt(format, MediaFormat.KEY_HEIGHT, result.height);
    }

    private static boolean decoderLowLatencySupported(MediaCodec decoder) {
        try {
            MediaCodecInfo.CodecCapabilities capabilities =
                decoder.getCodecInfo().getCapabilitiesForType("video/avc");
            return capabilities.isFeatureSupported(MediaCodecInfo.CodecCapabilities.FEATURE_LowLatency);
        } catch (Exception ignored) {
            return false;
        }
    }

    private static boolean requestDecoderLowLatency(MediaCodec decoder) {
        Bundle params = new Bundle();
        params.putInt(MediaCodec.PARAMETER_KEY_LOW_LATENCY, 1);
        try {
            decoder.setParameters(params);
            return true;
        } catch (Exception ignored) {
            return false;
        }
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
        final int headerMetadataBytes;
        final JSONObject headerProjectionMetadata;
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
            int headerMetadataBytes,
            JSONObject headerProjectionMetadata,
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
            this.headerMetadataBytes = headerMetadataBytes;
            this.headerProjectionMetadata = headerProjectionMetadata;
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
        final long retainedElapsedNs;
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
            long retainedElapsedNs,
            String metadataJson,
            HardwareBuffer buffer,
            int format,
            long usage,
            int layers,
            long bufferId) {
            this.width = width;
            this.height = height;
            this.timestampNs = timestampNs;
            this.retainedElapsedNs = retainedElapsedNs;
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
        boolean deliveryPaced;
        long deliveryDurationNs;
        int resolutionMismatchCount;
        long deltaTotalNs;
        long deltaMaxNs;
        int deltaOverTargetCount;
        int queueDropCount;
        int frameSetCommitCount;
        int frameSetDropCount;
        int frameSetQueueLimitDropCount;
        int frameSetStaleDropCount;
        int frameSetSkewDropCount;
        int frameSetWaitCount;
        long lastFrameSetQueueAgeNs;
        long lastFrameSetSkewNs;
        long lastFrameSetLeftTimestampNs;
        long lastFrameSetRightTimestampNs;
        final StageTiming nativeBridgeTiming = new StageTiming();
    }

    private static final class LiveDecodeResult {
        int schemaVersion;
        int codecId;
        int width;
        int height;
        int declaredPacketCount;
        int headerMetadataBytes;
        JSONObject headerProjectionMetadata;
        String sessionProjectionMetadataSource = "none";
        int packetCount;
        boolean streamEndedByEof;
        int streamMissingDeclaredPacketCount;
        long payloadBytes;
        long receiveStartElapsedNs;
        long receiveEndElapsedNs;
        long firstPacketReceiveElapsedNs;
        long lastPacketReceiveElapsedNs;
        long firstSourceElapsedNs;
        long lastSourceElapsedNs;
        final HashMap<Long, Long> sourceElapsedByPts = new HashMap<Long, Long>();
        String decoderName = "";
        String outputMime = "";
        boolean lowLatencyFeatureSupported;
        boolean lowLatencyConfigRequested;
        boolean lowLatencyParameterSucceeded;
        int outputWidth;
        int outputHeight;
        int spsBytes;
        int ppsBytes;
        int inputBufferCount;
        long inputBytes;
        boolean inputEosQueued;
        long lastPresentationTimeUs;
        int outputFormatChanges;
        int outputBufferCount;
        int decodedFrameCount;
        boolean outputEosSeen;
        int surfaceReleaseCount;
        boolean hardwareBufferTargetCreated;
        int hardwareBufferReaderWidth;
        int hardwareBufferReaderHeight;
        int hardwareBufferImageCount;
        int hardwareBufferDeliveredCount;
        int hardwareBufferMissingCount;
        final StageTiming hardwareBufferAwaitImageTiming = new StageTiming();
        final StageTiming hardwareBufferGetBufferTiming = new StageTiming();
        final StageTiming hardwareBufferNativeBridgeTiming = new StageTiming();
        int lastHardwareBufferFormat;
        long lastHardwareBufferUsage;
        int lastHardwareBufferLayers;
        long lastHardwareBufferId;
        long decodeStartElapsedNs;
        long decodeEndElapsedNs;
        long lastProgressLogElapsedNs;
        String lastError = "";
    }

    private static final class DecodeResult {
        String decodeOutputMode = "";
        String decoderName = "";
        String outputMime = "";
        boolean lowLatencyFeatureSupported;
        boolean lowLatencyConfigRequested;
        boolean lowLatencyParameterSucceeded;
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
        int outputFrameHashCount;
        int outputFrameHashUniqueCount;
        int outputFrameHashAdjacentEqualCount;
        long firstOutputFrameCrc32 = -1L;
        long lastOutputFrameCrc32 = -1L;
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
        final StageTiming hardwareBufferAwaitImageTiming = new StageTiming();
        final StageTiming hardwareBufferGetBufferTiming = new StageTiming();
        final StageTiming hardwareBufferNativeBridgeTiming = new StageTiming();
        int lastHardwareBufferFormat;
        long lastHardwareBufferUsage;
        int lastHardwareBufferLayers;
        long lastHardwareBufferId;
        int captureWindowMs;
        long decodeStartElapsedNs;
        long decodeEndElapsedNs;
        final List<DecodedHardwareBufferFrame> collectedHardwareBufferFrames =
            new ArrayList<DecodedHardwareBufferFrame>();
        final HashSet<Long> outputFrameHashes = new HashSet<Long>();
        String lastError = "";
    }

    private static final class StageTiming {
        long count;
        long totalNs;
        long maxNs;

        void record(long elapsedNs) {
            if (elapsedNs < 0L) {
                return;
            }
            count++;
            totalNs += elapsedNs;
            if (elapsedNs > maxNs) {
                maxNs = elapsedNs;
            }
        }

        long averageNs() {
            return count > 0L ? totalNs / count : 0L;
        }

        void copyFrom(StageTiming other) {
            count = other.count;
            totalNs = other.totalNs;
            maxNs = other.maxNs;
        }
    }
}
