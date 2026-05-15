package com.example.rustyxr.broker;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Rect;
import android.graphics.ImageFormat;
import android.graphics.Paint;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CameraMetadata;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.media.MediaFormat;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import android.util.Base64;
import android.util.Range;
import android.util.Size;
import android.view.Surface;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.ArrayDeque;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

final class BrokerAppCameraH264StreamSession {
    private static final String STREAM_SCHEMA = "rusty.xr.video_lab.binary_stream.v1";
    private static final String STREAM_ID_CAMERA_H264 = "broker_app.camera_h264";
    private static final String STREAM_ID_SYNTHETIC_H264 = "broker_app.synthetic_h264";
    private static final String SOURCE_CAMERA_H264 = "broker_app_camera2_mediacodec_surface";
    private static final String SOURCE_SYNTHETIC_H264 = "broker_app_synthetic_mediacodec_surface";
    private static final String SOURCE_API_CAMERA2 = "AndroidCamera2";
    private static final String SOURCE_API_SYNTHETIC_SURFACE = "AndroidMediaCodecSyntheticSurface";
    private static final String SOURCE_MODE_CAMERA2 = "camera2";
    private static final String SOURCE_MODE_SYNTHETIC_SURFACE = "synthetic_surface";
    private static final String DEFAULT_SYNTHETIC_PATTERN = "diagnostic-grid";
    private static final String MAGIC = "RXYRVID1";
    private static final int SCHEMA_VERSION = 3;
    private static final int CODEC_H264 = 1;
    private static final int DEFAULT_PORT = 8879;
    private static final int DEFAULT_HOST_PORT = 18879;
    private static final int DEFAULT_WIDTH = 720;
    private static final int DEFAULT_HEIGHT = 480;
    private static final int DEFAULT_CAPTURE_MS = 900;
    private static final int MAX_CAPTURE_MS = 3000;
    private static final int MAX_LIVE_CAPTURE_MS = 60000;
    private static final int DEFAULT_MAX_PACKETS = 12;
    private static final int MAX_PACKETS = 30;
    private static final int MAX_LIVE_PACKETS = 2400;
    private static final int DEFAULT_BITRATE_BPS = 1_000_000;
    private static final int MIN_RUNTIME_BITRATE_BPS = 100_000;
    private static final int MAX_RUNTIME_BITRATE_BPS = 20_000_000;
    private static final int DEFAULT_FRAME_RATE_HZ = 30;
    private static final int MIN_FRAME_RATE_HZ = 1;
    private static final int MAX_FRAME_RATE_HZ = 120;
    private static final int OPEN_TIMEOUT_MS = 4000;
    private static final int SESSION_TIMEOUT_MS = 4000;
    private static final int DEFAULT_STREAM_ACCEPT_TIMEOUT_MS = 15000;
    private static final int MAX_STREAM_ACCEPT_TIMEOUT_MS = 120000;
    private static final float SYNTHETIC_PROJECTION_FOV_Y_DEGREES = 60.0f;
    private static final int ENCODER_DRAIN_TIMEOUT_US = 10000;
    private static final int BINARY_STREAM_MAX_PACKET_BYTES = 1024 * 1024;
    private static final int MAX_STREAM_HEADER_METADATA_BYTES = 256 * 1024;
    private static final int MAX_CODEC_CONFIG_PACKETS = 8;
    private static final int DEFAULT_LIVE_WRITER_QUEUE_DEPTH = 48;
    private static final int MAX_LIVE_WRITER_QUEUE_DEPTH = 512;
    private static final int MAX_SYNTHETIC_FRAME_COUNT = 2400;
    private static final int WRITER_QUEUE_POLL_MS = 100;
    private static final int WRITER_JOIN_TIMEOUT_MS = 5000;
    private static final String MIME_H264 = "video/avc";
    private static final Object ACTIVE_ENCODER_LOCK = new Object();
    private static ActiveEncoderControl activeEncoderControl = null;

    interface Sink {
        void registerManifest(JSONObject manifest) throws Exception;

        void recordSample(JSONObject sample) throws Exception;

        void recordMetric(JSONObject metric) throws Exception;
    }

    private BrokerAppCameraH264StreamSession() {
    }

    static JSONObject startSynthetic(Context context, JSONObject params, Sink sink) throws Exception {
        JSONObject syntheticParams = params != null ? new JSONObject(params.toString()) : new JSONObject();
        syntheticParams.put("source_mode", SOURCE_MODE_SYNTHETIC_SURFACE);
        return start(context, syntheticParams, sink);
    }

    static JSONObject start(Context context, JSONObject params, Sink sink) throws Exception {
        final Context appContext = context != null ? context.getApplicationContext() : null;
        final boolean syntheticSource = isSyntheticSource(params);
        final String sourceMode = syntheticSource ? SOURCE_MODE_SYNTHETIC_SURFACE : SOURCE_MODE_CAMERA2;
        final String streamId = syntheticSource ? STREAM_ID_SYNTHETIC_H264 : STREAM_ID_CAMERA_H264;
        final String source = syntheticSource ? SOURCE_SYNTHETIC_H264 : SOURCE_CAMERA_H264;
        final String sourceApiPath = syntheticSource ? SOURCE_API_SYNTHETIC_SURFACE : SOURCE_API_CAMERA2;
        final String syntheticPattern = normalizeSyntheticPattern(
            params != null ? params.optString("synthetic_pattern", DEFAULT_SYNTHETIC_PATTERN) : DEFAULT_SYNTHETIC_PATTERN);
        final String sessionId = normalizeSessionId(
            params != null ? params.optString("session_id", "") : "",
            syntheticSource ? "broker-synthetic-h264-" : "broker-app-camera-h264-");
        final int devicePort = clamp(params != null ? params.optInt("device_port", DEFAULT_PORT) : DEFAULT_PORT, 1, 65535);
        final int hostPort = clamp(params != null ? params.optInt("host_port", DEFAULT_HOST_PORT) : DEFAULT_HOST_PORT, 1, 65535);
        final int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_WIDTH) : DEFAULT_WIDTH, 16, 4096);
        final int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_HEIGHT) : DEFAULT_HEIGHT, 16, 4096);
        final boolean liveStream = params != null && params.optBoolean("live_stream", false);
        final int requestedCaptureMs = params != null ? params.optInt("capture_ms", DEFAULT_CAPTURE_MS) : DEFAULT_CAPTURE_MS;
        final int requestedMaxPackets = params != null ? params.optInt("max_packets", DEFAULT_MAX_PACKETS) : DEFAULT_MAX_PACKETS;
        final int captureMs = liveStream && requestedCaptureMs <= 0
            ? 0
            : clamp(
                requestedCaptureMs,
                100,
                liveStream ? MAX_LIVE_CAPTURE_MS : MAX_CAPTURE_MS);
        final int maxPackets = liveStream && requestedMaxPackets <= 0
            ? 0
            : clamp(
                requestedMaxPackets,
                1,
                liveStream ? MAX_LIVE_PACKETS : MAX_PACKETS);
        final int writerQueueDepth = clamp(
            params != null ? params.optInt("writer_queue_depth", DEFAULT_LIVE_WRITER_QUEUE_DEPTH) : DEFAULT_LIVE_WRITER_QUEUE_DEPTH,
            1,
            MAX_LIVE_WRITER_QUEUE_DEPTH);
        final int acceptTimeoutMs = clamp(
            params != null ? params.optInt("accept_timeout_ms", DEFAULT_STREAM_ACCEPT_TIMEOUT_MS) : DEFAULT_STREAM_ACCEPT_TIMEOUT_MS,
            100,
            MAX_STREAM_ACCEPT_TIMEOUT_MS);
        final int bitrateBps = clamp(params != null ? params.optInt("bitrate_bps", DEFAULT_BITRATE_BPS) : DEFAULT_BITRATE_BPS, 100_000, 20_000_000);
        final int frameRateHz = clamp(
            params != null ? params.optInt("frame_rate_hz", DEFAULT_FRAME_RATE_HZ) : DEFAULT_FRAME_RATE_HZ,
            MIN_FRAME_RATE_HZ,
            MAX_FRAME_RATE_HZ);
        final String requestedCameraId = params != null ? params.optString("camera_id", "").trim() : "";
        final boolean lanStreamEnabled = params != null && params.optBoolean("lan_stream_enabled", false);
        final String bindHost = normalizeBindHost(
            params != null ? params.optString("bind_host", "") : "",
            lanStreamEnabled);
        final String advertisedHost = normalizeAdvertisedHost(
            params != null ? params.optString("advertised_host", "") : "",
            bindHost);

        JSONObject endpoint = new JSONObject();
        endpoint.put("host", advertisedHost);
        endpoint.put("bind_host", bindHost);
        endpoint.put("lan_stream_enabled", lanStreamEnabled);
        endpoint.put("device_port", devicePort);
        endpoint.put("host_port", hostPort);
        endpoint.put("framing", STREAM_SCHEMA);
        endpoint.put("magic", MAGIC);
        endpoint.put("codec_id", CODEC_H264);
        endpoint.put("codec", "h264");
        endpoint.put("schema_version", SCHEMA_VERSION);
        endpoint.put("packet_header", "pts_us,flags,size,source_time_elapsed_ns,source_time_unix_ns");
        endpoint.put("header_metadata", "projection_metadata_json_utf8");
        endpoint.put("writer_queue_depth", writerQueueDepth);
        endpoint.put("accept_timeout_ms", acceptTimeoutMs);
        endpoint.put("stream_id", streamId);
        endpoint.put("source_mode", sourceMode);
        endpoint.put("source", source);
        endpoint.put("frame_rate_hz", frameRateHz);

        final String cameraPermissionState = cameraPermissionState(appContext);
        JSONObject start = new JSONObject();
        start.put(
            "schema",
            syntheticSource
                ? "rusty.xr.media.synthetic_h264_stream_start.v1"
                : "rusty.xr.camera_provider.app_camera_h264_stream_start.v1");
        start.put("session_id", sessionId);
        start.put("stream_id", streamId);
        start.put("source", source);
        start.put("source_kind", source);
        start.put("source_mode", sourceMode);
        start.put("source_api_path", sourceApiPath);
        start.put(
            "camera_source_id",
            syntheticSource
                ? "synthetic:" + syntheticPattern
                : (requestedCameraId.length() > 0 ? "camera2:" + requestedCameraId : "camera2:auto"));
        start.put("camera_permission_state", syntheticSource ? "NotRequired" : cameraPermissionState);
        start.put("headset_camera_permission_state", syntheticSource ? "NotRequired" : cameraPermissionState);
        start.put("state", "starting");
        start.put("camera_id", requestedCameraId);
        start.put("preferred_width", preferredWidth);
        start.put("preferred_height", preferredHeight);
        start.put("capture_ms", captureMs);
        start.put("max_packets", maxPackets);
        start.put("writer_queue_depth", writerQueueDepth);
        start.put("accept_timeout_ms", acceptTimeoutMs);
        start.put("bitrate_bps", bitrateBps);
        start.put("frame_rate_hz", frameRateHz);
        start.put("live_stream", liveStream);
        start.put("stream_mode", streamMode(liveStream, captureMs, maxPackets));
        start.put("binary_endpoint", endpoint);
        if (syntheticSource) {
            Size syntheticSize = new Size(preferredWidth, preferredHeight);
            start.put("selected_camera_id", "synthetic:" + syntheticPattern);
            start.put("selected_width", syntheticSize.getWidth());
            start.put("selected_height", syntheticSize.getHeight());
            start.put("selected_reason", "synthetic_diagnostic_source");
            start.put("selected_fps_min_hz", frameRateHz);
            start.put("selected_fps_max_hz", frameRateHz);
            start.put("timestamp_domain", "ElapsedRealtime");
            start.put("synthetic_pattern", syntheticPattern);
            start.put("projection_metadata", buildSyntheticProjectionMetadata(syntheticSize, syntheticPattern));
        }
        try {
            if (!syntheticSource &&
                    appContext != null &&
                    appContext.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
                CameraManager manager = (CameraManager) appContext.getSystemService(Context.CAMERA_SERVICE);
                if (manager != null) {
                    CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight, frameRateHz);
                    start.put("selected_camera_id", selection.cameraId);
                    start.put("selected_width", selection.size.getWidth());
                    start.put("selected_height", selection.size.getHeight());
                    start.put("selection_score", selection.score);
                    putCameraSourceSelectionFields(start, selection, cameraPermissionState);
                    start.put("camera_source_capabilities", buildCameraSourceCapabilities(selection, cameraPermissionState));
                    start.put("projection_metadata", buildProjectionMetadata(selection));
                }
            }
        } catch (Exception ex) {
            start.put("projection_metadata_error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
        }

        Thread thread = new Thread(new Runnable() {
            @Override
            public void run() {
                runSession(
                    appContext,
                    sink,
                    sessionId,
                    requestedCameraId,
                    devicePort,
                    bindHost,
                    endpoint,
                    preferredWidth,
                    preferredHeight,
                    captureMs,
                    maxPackets,
                    writerQueueDepth,
                    acceptTimeoutMs,
                    bitrateBps,
                    frameRateHz,
                    liveStream,
                    syntheticSource,
                    syntheticPattern);
            }
        }, "RustyXrAppCameraH264Stream");
        thread.start();
        return start;
    }

    static JSONObject requestKeyframe(JSONObject params) throws Exception {
        ActiveEncoderControl control = activeControlFor(params);
        JSONObject result = mediaControlResult("media.request_keyframe", params, control);
        if (control == null) {
            result.put("applied", false);
            result.put("reason", "no_active_live_h264_stream");
            return result;
        }

        synchronized (control) {
            Bundle bundle = new Bundle();
            bundle.putInt(MediaCodec.PARAMETER_KEY_REQUEST_SYNC_FRAME, 0);
            control.encoder.setParameters(bundle);
            control.keyframeRequestCount++;
            control.lastControlElapsedNs = SystemClock.elapsedRealtimeNanos();
            result.put("applied", true);
            result.put("keyframe_request_count", control.keyframeRequestCount);
            result.put("applied_elapsed_ns", control.lastControlElapsedNs);
            return result;
        }
    }

    static JSONObject setVideoBitrate(JSONObject params) throws Exception {
        int requestedBitrateBps = clamp(
            params != null ? params.optInt("bitrate_bps", DEFAULT_BITRATE_BPS) : DEFAULT_BITRATE_BPS,
            MIN_RUNTIME_BITRATE_BPS,
            MAX_RUNTIME_BITRATE_BPS);
        return applyVideoBitrate(params, requestedBitrateBps, "media.set_video_bitrate", "");
    }

    static JSONObject setQualityProfile(JSONObject params) throws Exception {
        String profile = params != null ? params.optString("quality_profile", "") : "";
        if (profile.trim().length() == 0 && params != null) {
            profile = params.optString("profile", "");
        }
        profile = profile.trim().toLowerCase();
        if (profile.length() == 0) {
            profile = "balanced";
        }
        int bitrateBps = qualityProfileBitrateBps(profile);
        JSONObject result = applyVideoBitrate(params, bitrateBps, "media.set_quality_profile", profile);
        if (result.optBoolean("applied", false) && (params == null || params.optBoolean("request_keyframe", true))) {
            try {
                JSONObject keyframe = requestKeyframe(params);
                result.put("keyframe_request", keyframe);
            } catch (Exception ex) {
                result.put("keyframe_request_error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
            }
        }
        return result;
    }

    static CaptureResult capturePacketsForProbe(Context context, JSONObject params) throws Exception {
        final Context appContext = context != null ? context.getApplicationContext() : null;
        final String sessionId = normalizeSessionId(
            params != null ? params.optString("session_id", "") : "",
            "broker-app-camera-h264-decode-");
        final int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_WIDTH) : DEFAULT_WIDTH, 16, 4096);
        final int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_HEIGHT) : DEFAULT_HEIGHT, 16, 4096);
        final int captureMs = clamp(params != null ? params.optInt("capture_ms", DEFAULT_CAPTURE_MS) : DEFAULT_CAPTURE_MS, 100, MAX_CAPTURE_MS);
        final int maxPackets = clamp(params != null ? params.optInt("max_packets", DEFAULT_MAX_PACKETS) : DEFAULT_MAX_PACKETS, 1, MAX_PACKETS);
        final int bitrateBps = clamp(params != null ? params.optInt("bitrate_bps", DEFAULT_BITRATE_BPS) : DEFAULT_BITRATE_BPS, 100_000, 20_000_000);
        final int frameRateHz = clamp(
            params != null ? params.optInt("frame_rate_hz", DEFAULT_FRAME_RATE_HZ) : DEFAULT_FRAME_RATE_HZ,
            MIN_FRAME_RATE_HZ,
            MAX_FRAME_RATE_HZ);
        final String requestedCameraId = params != null ? params.optString("camera_id", "").trim() : "";

        if (appContext == null) {
            throw new IllegalStateException("Broker app context is unavailable.");
        }
        if (appContext.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            throw new SecurityException("Broker app camera permission is not granted.");
        }

        CameraManager manager = (CameraManager) appContext.getSystemService(Context.CAMERA_SERVICE);
        if (manager == null) {
            throw new IllegalStateException("CameraManager is unavailable.");
        }

        CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight, frameRateHz);
        EncoderMetadata encoderMetadata = new EncoderMetadata();
        encoderMetadata.sensorTimestampSource = sensorTimestampSourceLabel(
            selection.characteristics.get(CameraCharacteristics.SENSOR_INFO_TIMESTAMP_SOURCE));
        long encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        List<EncodedPacket> packets = encodeCameraPackets(
            manager,
            selection.cameraId,
            selection.size,
            captureMs,
            maxPackets,
            bitrateBps,
            frameRateHz,
            encoderMetadata);
        long encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
        return new CaptureResult(
            sessionId,
            requestedCameraId,
            selection.cameraId,
            selection.size,
            captureMs,
            maxPackets,
            bitrateBps,
            frameRateHz,
            encodeStartElapsedNs,
            encodeEndElapsedNs,
            packets,
            encoderMetadata);
    }

    private static JSONObject applyVideoBitrate(
        JSONObject params,
        int bitrateBps,
        String command,
        String qualityProfile) throws Exception {
        ActiveEncoderControl control = activeControlFor(params);
        JSONObject result = mediaControlResult(command, params, control);
        result.put("requested_bitrate_bps", bitrateBps);
        if (qualityProfile != null && qualityProfile.length() > 0) {
            result.put("quality_profile", qualityProfile);
        }
        if (control == null) {
            result.put("applied", false);
            result.put("reason", "no_active_live_h264_stream");
            return result;
        }

        synchronized (control) {
            Bundle bundle = new Bundle();
            bundle.putInt(MediaCodec.PARAMETER_KEY_VIDEO_BITRATE, bitrateBps);
            control.encoder.setParameters(bundle);
            control.currentBitrateBps = bitrateBps;
            if (qualityProfile != null && qualityProfile.length() > 0) {
                control.qualityProfile = qualityProfile;
            }
            control.bitrateChangeCount++;
            control.lastControlElapsedNs = SystemClock.elapsedRealtimeNanos();
            result.put("applied", true);
            result.put("applied_bitrate_bps", control.currentBitrateBps);
            result.put("bitrate_change_count", control.bitrateChangeCount);
            result.put("applied_elapsed_ns", control.lastControlElapsedNs);
            return result;
        }
    }

    private static JSONObject mediaControlResult(
        String command,
        JSONObject params,
        ActiveEncoderControl control) throws Exception {
        JSONObject result = new JSONObject();
        result.put("schema", "rusty.xr.media_control.result.v1");
        result.put("command", command);
        result.put("requested_session_id", params != null ? params.optString("session_id", "") : "");
        result.put("requested_stream_id", params != null ? params.optString("stream_id", "") : "");
        result.put("active", control != null);
        if (control != null) {
            synchronized (control) {
                result.put("session_id", control.sessionId);
                result.put("stream_id", control.streamId);
                result.put("camera_id", control.cameraId);
                result.put("current_bitrate_bps", control.currentBitrateBps);
                result.put("quality_profile", control.qualityProfile);
                result.put("keyframe_request_count", control.keyframeRequestCount);
                result.put("bitrate_change_count", control.bitrateChangeCount);
                result.put("active_since_elapsed_ns", control.activeSinceElapsedNs);
                result.put("last_control_elapsed_ns", control.lastControlElapsedNs);
            }
        }
        return result;
    }

    private static ActiveEncoderControl activeControlFor(JSONObject params) {
        synchronized (ACTIVE_ENCODER_LOCK) {
            if (activeEncoderControl == null) {
                return null;
            }
            String requestedSessionId = params != null ? params.optString("session_id", "").trim() : "";
            if (requestedSessionId.length() > 0 && !requestedSessionId.equals(activeEncoderControl.sessionId)) {
                return null;
            }
            String requestedStreamId = params != null ? params.optString("stream_id", "").trim() : "";
            if (requestedStreamId.length() > 0 && !requestedStreamId.equals(activeEncoderControl.streamId)) {
                return null;
            }
            return activeEncoderControl;
        }
    }

    private static void registerActiveEncoder(ActiveEncoderControl control) {
        synchronized (ACTIVE_ENCODER_LOCK) {
            activeEncoderControl = control;
        }
    }

    private static void unregisterActiveEncoder(ActiveEncoderControl control) {
        synchronized (ACTIVE_ENCODER_LOCK) {
            if (activeEncoderControl == control) {
                activeEncoderControl = null;
            }
        }
    }

    private static int qualityProfileBitrateBps(String profile) {
        if ("low".equals(profile)) {
            return 600_000;
        }
        if ("high".equals(profile)) {
            return 2_000_000;
        }
        if ("ultra".equals(profile)) {
            return 4_000_000;
        }
        return DEFAULT_BITRATE_BPS;
    }

    private static void runSession(
        Context context,
        Sink sink,
        String sessionId,
        String requestedCameraId,
        int devicePort,
        String bindHost,
        JSONObject endpoint,
        int preferredWidth,
        int preferredHeight,
        int captureMs,
        int maxPackets,
        int writerQueueDepth,
        int acceptTimeoutMs,
        int bitrateBps,
        int frameRateHz,
        boolean liveStream,
        boolean syntheticSource,
        String syntheticPattern) {
        long encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long encodeEndElapsedNs = encodeStartElapsedNs;
        StreamWriteStats writeStats = new StreamWriteStats(0L, 0L, 0L, 0L);
        List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        EncoderMetadata encoderMetadata = new EncoderMetadata();
        String cameraId = requestedCameraId;
        Size size = null;
        CameraSelection selection = null;
        String lastError = "";
        try {
            CameraManager manager = null;
            JSONObject streamProjectionMetadata;
            if (syntheticSource) {
                syntheticPattern = normalizeSyntheticPattern(syntheticPattern);
                cameraId = "synthetic:" + syntheticPattern;
                size = new Size(preferredWidth, preferredHeight);
                streamProjectionMetadata = buildSyntheticProjectionMetadata(size, syntheticPattern);
                encoderMetadata.sensorTimestampSource = "synthetic_elapsed_realtime";
            } else {
                if (context == null) {
                    throw new IllegalStateException("Broker app context is unavailable.");
                }
                if (context.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
                    throw new SecurityException("Broker app camera permission is not granted.");
                }

                manager = (CameraManager) context.getSystemService(Context.CAMERA_SERVICE);
                if (manager == null) {
                    throw new IllegalStateException("CameraManager is unavailable.");
                }
                selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight, frameRateHz);
                cameraId = selection.cameraId;
                size = selection.size;
                streamProjectionMetadata = buildProjectionMetadata(selection);
                encoderMetadata.sensorTimestampSource = sensorTimestampSourceLabel(
                    selection.characteristics.get(CameraCharacteristics.SENSOR_INFO_TIMESTAMP_SOURCE));
            }
            registerManifest(
                sink,
                sessionId,
                cameraId,
                size,
                captureMs,
                maxPackets,
                bitrateBps,
                frameRateHz,
                liveStream,
                endpoint,
                selection,
                encoderMetadata,
                syntheticSource,
                syntheticPattern);
            encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (liveStream) {
                LiveStreamResult liveResult = syntheticSource
                    ? streamSyntheticPacketsLive(
                        size,
                        captureMs,
                        maxPackets,
                        writerQueueDepth,
                        acceptTimeoutMs,
                        bitrateBps,
                        frameRateHz,
                        devicePort,
                        bindHost,
                        sink,
                        sessionId,
                        endpoint,
                        cameraId,
                        streamProjectionMetadata,
                        encoderMetadata,
                        syntheticPattern)
                    : streamCameraPacketsLive(
                        manager,
                        cameraId,
                        size,
                        captureMs,
                        maxPackets,
                        writerQueueDepth,
                        acceptTimeoutMs,
                        bitrateBps,
                        frameRateHz,
                        devicePort,
                        bindHost,
                        sink,
                        sessionId,
                        endpoint,
                        selection,
                        streamProjectionMetadata,
                        encoderMetadata);
                packets = liveResult.packets;
                writeStats = liveResult.writeStats;
                encodeEndElapsedNs = liveResult.encodeEndElapsedNs;
            } else {
                packets = syntheticSource
                    ? encodeSyntheticPackets(size, captureMs, maxPackets, bitrateBps, frameRateHz, encoderMetadata, syntheticPattern)
                    : encodeCameraPackets(manager, cameraId, size, captureMs, maxPackets, bitrateBps, frameRateHz, encoderMetadata);
                encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
                registerManifest(
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    captureMs,
                    maxPackets,
                    bitrateBps,
                    frameRateHz,
                    liveStream,
                    endpoint,
                    selection,
                    encoderMetadata,
                    syntheticSource,
                    syntheticPattern);
                for (int i = 0; i < packets.size(); i++) {
                    recordSample(
                        sink,
                        sessionId,
                        cameraId,
                        size,
                        i,
                        packets.get(i),
                        false,
                        syntheticSource);
                }
                writeStats = writePackets(devicePort, bindHost, size, packets, streamProjectionMetadata);
            }
        } catch (Exception ex) {
            encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            lastError = ex.getClass().getSimpleName() + ": " + safeMessage(ex);
        } finally {
            try {
                recordMetric(
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    packets,
                    encodeStartElapsedNs,
                    encodeEndElapsedNs,
                    writeStats,
                    captureMs,
                    maxPackets,
                    frameRateHz,
                    liveStream,
                    selection,
                    encoderMetadata,
                    lastError,
                    syntheticSource,
                    syntheticPattern);
            } catch (Exception ignored) {
            }
        }
    }

    private static List<EncodedPacket> encodeCameraPackets(
        final CameraManager manager,
        final String cameraId,
        final Size size,
        final int captureMs,
        final int maxPackets,
        final int bitrateBps,
        final int frameRateHz,
        final EncoderMetadata encoderMetadata) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        HandlerThread thread = new HandlerThread("RustyXrAppCameraH264Capture");
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        EncoderSelection encoderSelection = selectH264Encoder(size, bitrateBps, frameRateHz);
        MediaCodec encoder = createH264Encoder(encoderSelection, encoderMetadata);
        Surface encoderSurface = null;
        CaptureTimingTracker captureTiming = new CaptureTimingTracker();
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        try {
            applyEncoderSelectionMetadata(encoderSelection, encoderMetadata, encoder);
            configureH264Encoder(encoder, size, bitrateBps, frameRateHz, encoderMetadata);
            encoderSurface = encoder.createInputSurface();
            encoder.start();
            requestSyncFrameOnStart(encoder, encoderMetadata);

            deviceRef[0] = openCamera(manager, cameraId, handler);
            sessionRef[0] = configureSession(deviceRef[0], encoderSurface, handler);
            CaptureRequest.Builder builder = createRecordRequest(deviceRef[0]);
            builder.addTarget(encoderSurface);
            applyCaptureRequestSelection(builder, new CameraSelection(
                cameraId,
                size,
                0L,
                manager.getCameraCharacteristics(cameraId),
                chooseFpsRange(manager.getCameraCharacteristics(cameraId), frameRateHz),
                streamMinFrameDurationNs(manager.getCameraCharacteristics(cameraId), size),
                "capture_probe_selection"));
            sessionRef[0].setRepeatingRequest(builder.build(), captureTiming, handler);

            long deadlineElapsedNs = SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L;
            while (SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs && videoPacketCount(packets) < maxPackets) {
                drainEncoder(encoder, packets, false, maxPackets, encoderMetadata);
                Thread.sleep(10);
            }
            try {
                sessionRef[0].stopRepeating();
            } catch (Exception ignored) {
            }
            encoder.signalEndOfInputStream();
            drainEncoder(encoder, packets, true, maxPackets, encoderMetadata);
            encoderMetadata.copyCaptureTiming(captureTiming);
            if (videoPacketCount(packets) == 0) {
                throw new IllegalStateException("MediaCodec produced no app-camera H.264 packets.");
            }
            return packets;
        } finally {
            closeQuietly(sessionRef[0]);
            closeQuietly(deviceRef[0]);
            if (encoderSurface != null) {
                encoderSurface.release();
            }
            try {
                encoder.stop();
            } catch (Exception ignored) {
            }
            encoder.release();
            thread.quitSafely();
        }
    }

    private static List<EncodedPacket> encodeSyntheticPackets(
        final Size size,
        final int captureMs,
        final int maxPackets,
        final int bitrateBps,
        final int frameRateHz,
        final EncoderMetadata encoderMetadata,
        final String syntheticPattern) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        final int effectiveMaxPackets = maxPackets > 0 ? maxPackets : MAX_SYNTHETIC_FRAME_COUNT;
        final int frameLimit = syntheticFrameLimit(captureMs, effectiveMaxPackets, frameRateHz);
        EncoderSelection encoderSelection = selectH264Encoder(size, bitrateBps, frameRateHz);
        MediaCodec encoder = createH264Encoder(encoderSelection, encoderMetadata);
        Surface encoderSurface = null;
        try {
            applyEncoderSelectionMetadata(encoderSelection, encoderMetadata, encoder);
            configureH264Encoder(encoder, size, bitrateBps, frameRateHz, encoderMetadata);
            encoderSurface = encoder.createInputSurface();
            encoder.start();
            requestSyncFrameOnStart(encoder, encoderMetadata);

            long deadlineElapsedNs = captureMs > 0
                ? SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L
                : Long.MAX_VALUE;
            int frameIndex = 0;
            while (frameIndex < frameLimit &&
                    SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs &&
                    videoPacketCount(packets) < effectiveMaxPackets) {
                long frameStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                drawSyntheticEncoderFrame(encoderSurface, frameIndex, size, syntheticPattern);
                drainEncoder(encoder, packets, false, effectiveMaxPackets, encoderMetadata);
                sleepUntilSyntheticFrameCadence(frameStartElapsedNs, frameRateHz);
                frameIndex++;
            }
            encoder.signalEndOfInputStream();
            drainEncoder(encoder, packets, true, effectiveMaxPackets, encoderMetadata);
            if (videoPacketCount(packets) == 0) {
                throw new IllegalStateException("MediaCodec produced no synthetic H.264 packets.");
            }
            return packets;
        } finally {
            if (encoderSurface != null) {
                encoderSurface.release();
            }
            try {
                encoder.stop();
            } catch (Exception ignored) {
            }
            encoder.release();
        }
    }

    private static LiveStreamResult streamCameraPacketsLive(
        final CameraManager manager,
        final String cameraId,
        final Size size,
        final int captureMs,
        final int maxPackets,
        final int writerQueueDepth,
        final int acceptTimeoutMs,
        final int bitrateBps,
        final int frameRateHz,
        final int devicePort,
        final String bindHost,
        final Sink sink,
        final String sessionId,
        final JSONObject endpoint,
        final CameraSelection selection,
        final JSONObject streamProjectionMetadata,
        final EncoderMetadata encoderMetadata) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        HandlerThread thread = new HandlerThread("RustyXrAppCameraH264LiveCapture");
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        EncoderSelection encoderSelection = selectH264Encoder(size, bitrateBps, frameRateHz);
        MediaCodec encoder = createH264Encoder(encoderSelection, encoderMetadata);
        Surface encoderSurface = null;
        CaptureTimingTracker captureTiming = new CaptureTimingTracker();
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        ServerSocket server = null;
        Socket client = null;
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        long encodeEndElapsedNs = listenStartElapsedNs;
        final LivePacketQueue packetQueue = new LivePacketQueue(writerQueueDepth);
        final LiveStreamWriter[] writerRef = new LiveStreamWriter[1];
        Thread writerThread = null;
        ActiveEncoderControl activeControl = null;
        try {
            server = new ServerSocket(devicePort, 1, InetAddress.getByName(bindHost));
            server.setSoTimeout(acceptTimeoutMs);
            client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            client.setTcpNoDelay(true);
            OutputStream output = client.getOutputStream();
            LiveStreamWriter writer = new LiveStreamWriter(
                output,
                size,
                maxPackets,
                packetQueue,
                packets,
                sink,
                sessionId,
                cameraId,
                streamProjectionMetadata,
                true,
                false);
            writerRef[0] = writer;
            writerThread = new Thread(writer, "RustyXrAppCameraH264Writer");
            writerThread.start();

            applyEncoderSelectionMetadata(encoderSelection, encoderMetadata, encoder);
            configureH264Encoder(encoder, size, bitrateBps, frameRateHz, encoderMetadata);
            encoderSurface = encoder.createInputSurface();
            encoder.start();
            requestSyncFrameOnStart(encoder, encoderMetadata);
            activeControl = new ActiveEncoderControl(
                sessionId,
                STREAM_ID_CAMERA_H264,
                cameraId,
                encoder,
                bitrateBps,
                "balanced");
            registerActiveEncoder(activeControl);

            deviceRef[0] = openCamera(manager, cameraId, handler);
            sessionRef[0] = configureSession(deviceRef[0], encoderSurface, handler);
            CaptureRequest.Builder builder = createRecordRequest(deviceRef[0]);
            builder.addTarget(encoderSurface);
            applyCaptureRequestSelection(builder, selection);
            sessionRef[0].setRepeatingRequest(builder.build(), captureTiming, handler);

            long deadlineElapsedNs = captureMs > 0
                ? SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L
                : Long.MAX_VALUE;
            while (SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs &&
                    (maxPackets <= 0 || packetQueue.acceptedPacketCount() < maxPackets) &&
                    (maxPackets <= 0 || writer.writtenPacketCount() < maxPackets) &&
                    !writer.hasError()) {
                drainEncoderToQueue(
                    encoder,
                    packetQueue,
                    false,
                    maxPackets,
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    captureMs,
                    bitrateBps,
                    frameRateHz,
                    endpoint,
                    selection,
                    encoderMetadata,
                    false,
                    "");
                Thread.sleep(5);
            }
            try {
                sessionRef[0].stopRepeating();
            } catch (Exception ignored) {
            }
            encoder.signalEndOfInputStream();
            drainEncoderToQueue(
                encoder,
                packetQueue,
                true,
                maxPackets,
                sink,
                sessionId,
                cameraId,
                size,
                captureMs,
                bitrateBps,
                frameRateHz,
                endpoint,
                selection,
                encoderMetadata,
                false,
                "");
            encoderMetadata.copyCaptureTiming(captureTiming);
            packetQueue.close();
            joinWriterThread(writerThread, client);
            if (writer.hasError() && packets.size() == 0) {
                throw writer.error();
            }
            writeStartElapsedNs = writer.writeStartElapsedNs();
            writeEndElapsedNs = writer.writeEndElapsedNs();
            encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (packets.size() == 0) {
                throw new IllegalStateException("MediaCodec produced no live app-camera H.264 packets.");
            }
            return new LiveStreamResult(
                packets,
                new StreamWriteStats(
                    listenStartElapsedNs,
                    acceptElapsedNs,
                    writeStartElapsedNs,
                    writeEndElapsedNs,
                    packetQueue,
                    writer),
                encodeEndElapsedNs);
        } finally {
            if (activeControl != null) {
                unregisterActiveEncoder(activeControl);
            }
            packetQueue.close();
            if (writerThread != null && writerThread.isAlive()) {
                closeQuietly(client);
                joinQuietly(writerThread, 250L);
            }
            closeQuietly(sessionRef[0]);
            closeQuietly(deviceRef[0]);
            if (encoderSurface != null) {
                encoderSurface.release();
            }
            try {
                encoder.stop();
            } catch (Exception ignored) {
            }
            encoder.release();
            closeQuietly(client);
            if (server != null) {
                try {
                    server.close();
                } catch (Exception ignored) {
                }
            }
            thread.quitSafely();
        }
    }

    private static LiveStreamResult streamSyntheticPacketsLive(
        final Size size,
        final int captureMs,
        final int maxPackets,
        final int writerQueueDepth,
        final int acceptTimeoutMs,
        final int bitrateBps,
        final int frameRateHz,
        final int devicePort,
        final String bindHost,
        final Sink sink,
        final String sessionId,
        final JSONObject endpoint,
        final String cameraId,
        final JSONObject streamProjectionMetadata,
        final EncoderMetadata encoderMetadata,
        final String syntheticPattern) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        EncoderSelection encoderSelection = selectH264Encoder(size, bitrateBps, frameRateHz);
        MediaCodec encoder = createH264Encoder(encoderSelection, encoderMetadata);
        Surface encoderSurface = null;
        ServerSocket server = null;
        Socket client = null;
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        long encodeEndElapsedNs = listenStartElapsedNs;
        final LivePacketQueue packetQueue = new LivePacketQueue(writerQueueDepth);
        final LiveStreamWriter[] writerRef = new LiveStreamWriter[1];
        Thread writerThread = null;
        ActiveEncoderControl activeControl = null;
        try {
            server = new ServerSocket(devicePort, 1, InetAddress.getByName(bindHost));
            server.setSoTimeout(acceptTimeoutMs);
            client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            client.setTcpNoDelay(true);
            OutputStream output = client.getOutputStream();
            LiveStreamWriter writer = new LiveStreamWriter(
                output,
                size,
                maxPackets,
                packetQueue,
                packets,
                sink,
                sessionId,
                cameraId,
                streamProjectionMetadata,
                true,
                true);
            writerRef[0] = writer;
            writerThread = new Thread(writer, "RustyXrSyntheticH264Writer");
            writerThread.start();

            applyEncoderSelectionMetadata(encoderSelection, encoderMetadata, encoder);
            configureH264Encoder(encoder, size, bitrateBps, frameRateHz, encoderMetadata);
            encoderSurface = encoder.createInputSurface();
            encoder.start();
            requestSyncFrameOnStart(encoder, encoderMetadata);
            activeControl = new ActiveEncoderControl(
                sessionId,
                STREAM_ID_SYNTHETIC_H264,
                cameraId,
                encoder,
                bitrateBps,
                "balanced");
            registerActiveEncoder(activeControl);

            long deadlineElapsedNs = captureMs > 0
                ? SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L
                : Long.MAX_VALUE;
            int frameIndex = 0;
            boolean boundedByFrameLimit = captureMs > 0 || maxPackets > 0;
            while (SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs &&
                    (maxPackets <= 0 || packetQueue.acceptedPacketCount() < maxPackets) &&
                    (maxPackets <= 0 || writer.writtenPacketCount() < maxPackets) &&
                    (!boundedByFrameLimit || frameIndex < MAX_SYNTHETIC_FRAME_COUNT) &&
                    !writer.hasError()) {
                long frameStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                drawSyntheticEncoderFrame(encoderSurface, frameIndex, size, syntheticPattern);
                drainEncoderToQueue(
                    encoder,
                    packetQueue,
                    false,
                    maxPackets,
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    captureMs,
                    bitrateBps,
                    frameRateHz,
                    endpoint,
                    null,
                    encoderMetadata,
                    true,
                    syntheticPattern);
                sleepUntilSyntheticFrameCadence(frameStartElapsedNs, frameRateHz);
                frameIndex++;
            }
            encoder.signalEndOfInputStream();
            drainEncoderToQueue(
                encoder,
                packetQueue,
                true,
                maxPackets,
                sink,
                sessionId,
                cameraId,
                size,
                captureMs,
                bitrateBps,
                frameRateHz,
                endpoint,
                null,
                encoderMetadata,
                true,
                syntheticPattern);
            packetQueue.close();
            joinWriterThread(writerThread, client);
            if (writer.hasError() && packets.size() == 0) {
                throw writer.error();
            }
            writeStartElapsedNs = writer.writeStartElapsedNs();
            writeEndElapsedNs = writer.writeEndElapsedNs();
            encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (packets.size() == 0) {
                throw new IllegalStateException("MediaCodec produced no live synthetic H.264 packets.");
            }
            return new LiveStreamResult(
                packets,
                new StreamWriteStats(
                    listenStartElapsedNs,
                    acceptElapsedNs,
                    writeStartElapsedNs,
                    writeEndElapsedNs,
                    packetQueue,
                    writer),
                encodeEndElapsedNs);
        } finally {
            if (activeControl != null) {
                unregisterActiveEncoder(activeControl);
            }
            packetQueue.close();
            if (writerThread != null && writerThread.isAlive()) {
                closeQuietly(client);
                joinQuietly(writerThread, 250L);
            }
            if (encoderSurface != null) {
                encoderSurface.release();
            }
            try {
                encoder.stop();
            } catch (Exception ignored) {
            }
            encoder.release();
            closeQuietly(client);
            if (server != null) {
                try {
                    server.close();
                } catch (Exception ignored) {
                }
            }
        }
    }

    private static CameraDevice openCamera(CameraManager manager, String cameraId, Handler handler) throws Exception {
        final CountDownLatch latch = new CountDownLatch(1);
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final Exception[] errorRef = new Exception[1];
        manager.openCamera(cameraId, new CameraDevice.StateCallback() {
            @Override
            public void onOpened(CameraDevice camera) {
                deviceRef[0] = camera;
                latch.countDown();
            }

            @Override
            public void onDisconnected(CameraDevice camera) {
                errorRef[0] = new IllegalStateException("Camera disconnected.");
                if (camera != null) {
                    camera.close();
                }
                latch.countDown();
            }

            @Override
            public void onError(CameraDevice camera, int error) {
                errorRef[0] = new CameraAccessException(CameraAccessException.CAMERA_ERROR, "Camera error " + error);
                if (camera != null) {
                    camera.close();
                }
                latch.countDown();
            }
        }, handler);
        if (!latch.await(OPEN_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
            throw new IllegalStateException("Timed out opening camera " + cameraId + ".");
        }
        if (deviceRef[0] == null) {
            throw errorRef[0] != null ? errorRef[0] : new IllegalStateException("Camera open failed.");
        }
        return deviceRef[0];
    }

    private static CameraCaptureSession configureSession(
        CameraDevice device,
        Surface surface,
        Handler handler) throws Exception {
        final CountDownLatch latch = new CountDownLatch(1);
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        final String[] errorRef = new String[1];
        device.createCaptureSession(
            Arrays.asList(surface),
            new CameraCaptureSession.StateCallback() {
                @Override
                public void onConfigured(CameraCaptureSession session) {
                    sessionRef[0] = session;
                    latch.countDown();
                }

                @Override
                public void onConfigureFailed(CameraCaptureSession session) {
                    errorRef[0] = "Camera capture session configure failed.";
                    latch.countDown();
                }
            },
            handler);
        if (!latch.await(SESSION_TIMEOUT_MS, TimeUnit.MILLISECONDS)) {
            throw new IllegalStateException("Timed out configuring camera capture session.");
        }
        if (sessionRef[0] == null) {
            throw new IllegalStateException(errorRef[0] != null ? errorRef[0] : "Camera capture session failed.");
        }
        return sessionRef[0];
    }

    private static CaptureRequest.Builder createRecordRequest(CameraDevice device) throws CameraAccessException {
        try {
            return device.createCaptureRequest(CameraDevice.TEMPLATE_RECORD);
        } catch (IllegalArgumentException ex) {
            return device.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
        }
    }

    private static void applyCaptureRequestSelection(
        CaptureRequest.Builder builder,
        CameraSelection selection) {
        if (builder == null || selection == null || selection.fpsRange == null) {
            return;
        }
        try {
            builder.set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, selection.fpsRange);
        } catch (Exception ignored) {
        }
    }

    private static EncoderSelection selectH264Encoder(Size size, int bitrateBps, int frameRateHz) {
        EncoderSelection best = null;
        try {
            MediaCodecList codecList = new MediaCodecList(MediaCodecList.ALL_CODECS);
            MediaCodecInfo[] infos = codecList.getCodecInfos();
            for (int i = 0; i < infos.length; i++) {
                MediaCodecInfo info = infos[i];
                if (info == null || !info.isEncoder() || !supportsType(info, MIME_H264)) {
                    continue;
                }
                EncoderSelection candidate = inspectH264Encoder(info, size, bitrateBps, frameRateHz);
                if (candidate != null && (best == null || candidate.score > best.score)) {
                    best = candidate;
                }
            }
        } catch (Exception ignored) {
            return null;
        }
        return best;
    }

    private static EncoderSelection inspectH264Encoder(MediaCodecInfo info, Size size, int bitrateBps, int frameRateHz) {
        try {
            MediaCodecInfo.CodecCapabilities capabilities = info.getCapabilitiesForType(MIME_H264);
            MediaCodecInfo.VideoCapabilities videoCapabilities = capabilities.getVideoCapabilities();
            MediaCodecInfo.EncoderCapabilities encoderCapabilities = capabilities.getEncoderCapabilities();
            boolean sizeAndRateSupported = videoCapabilities == null ||
                videoCapabilities.areSizeAndRateSupported(size.getWidth(), size.getHeight(), (double) frameRateHz);
            boolean sizeSupported = videoCapabilities == null ||
                videoCapabilities.isSizeSupported(size.getWidth(), size.getHeight());
            int widthAlignment = videoCapabilities != null ? videoCapabilities.getWidthAlignment() : 1;
            int heightAlignment = videoCapabilities != null ? videoCapabilities.getHeightAlignment() : 1;
            int bitrateLower = 0;
            int bitrateUpper = 0;
            boolean bitrateSupported = true;
            if (videoCapabilities != null) {
                Range<Integer> bitrateRange = videoCapabilities.getBitrateRange();
                bitrateLower = bitrateRange.getLower();
                bitrateUpper = bitrateRange.getUpper();
                bitrateSupported = bitrateBps >= bitrateLower && bitrateBps <= bitrateUpper;
            }
            boolean cbrSupported = encoderCapabilities != null &&
                encoderCapabilities.isBitrateModeSupported(MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR);
            boolean cbrFdSupported = encoderCapabilities != null &&
                encoderCapabilities.isBitrateModeSupported(MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR_FD);
            boolean vbrSupported = encoderCapabilities != null &&
                encoderCapabilities.isBitrateModeSupported(MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_VBR);
            boolean hardwareAccelerated = safeIsHardwareAccelerated(info);
            boolean softwareOnly = safeIsSoftwareOnly(info);
            long score = 0L;
            if (sizeAndRateSupported) {
                score += 1_000_000_000L;
            } else if (sizeSupported) {
                score += 500_000_000L;
            }
            if (bitrateSupported) {
                score += 100_000_000L;
            }
            if (hardwareAccelerated) {
                score += 10_000_000L;
            }
            if (softwareOnly) {
                score -= 10_000_000L;
            }
            if (cbrSupported) {
                score += 1_000_000L;
            }
            return new EncoderSelection(
                info.getName(),
                hardwareAccelerated,
                softwareOnly,
                sizeSupported,
                sizeAndRateSupported,
                bitrateSupported,
                widthAlignment,
                heightAlignment,
                bitrateLower,
                bitrateUpper,
                cbrSupported,
                cbrFdSupported,
                vbrSupported,
                score);
        } catch (Exception ignored) {
            return null;
        }
    }

    private static boolean supportsType(MediaCodecInfo info, String mimeType) {
        String[] types = info.getSupportedTypes();
        for (int i = 0; i < types.length; i++) {
            if (mimeType.equalsIgnoreCase(types[i])) {
                return true;
            }
        }
        return false;
    }

    private static MediaCodec createH264Encoder(
        EncoderSelection encoderSelection,
        EncoderMetadata encoderMetadata) throws Exception {
        if (encoderSelection != null && encoderSelection.codecName.length() > 0) {
            try {
                return MediaCodec.createByCodecName(encoderSelection.codecName);
            } catch (Exception ex) {
                encoderMetadata.encoderSelectionFallbackReason =
                    ex.getClass().getSimpleName() + ": " + safeMessage(ex);
            }
        }
        return MediaCodec.createEncoderByType(MIME_H264);
    }

    private static void applyEncoderSelectionMetadata(
        EncoderSelection encoderSelection,
        EncoderMetadata encoderMetadata,
        MediaCodec encoder) {
        encoderMetadata.encoderName = safeCodecName(encoder);
        if (encoderSelection == null) {
            encoderMetadata.encoderSelectionSource = "create_encoder_by_type";
            encoderMetadata.bitrateModeRequested = "default";
            encoderMetadata.bitrateModeApplied = "default";
            return;
        }
        encoderMetadata.encoderSelectionSource = "mediacodec_list";
        encoderMetadata.encoderSelectedName = encoderSelection.codecName;
        encoderMetadata.encoderHardwareAccelerated = encoderSelection.hardwareAccelerated;
        encoderMetadata.encoderSoftwareOnly = encoderSelection.softwareOnly;
        encoderMetadata.encoderSizeSupported = encoderSelection.sizeSupported;
        encoderMetadata.encoderSizeAndRateSupported = encoderSelection.sizeAndRateSupported;
        encoderMetadata.encoderBitrateSupported = encoderSelection.bitrateSupported;
        encoderMetadata.encoderWidthAlignment = encoderSelection.widthAlignment;
        encoderMetadata.encoderHeightAlignment = encoderSelection.heightAlignment;
        encoderMetadata.encoderBitrateLower = encoderSelection.bitrateLower;
        encoderMetadata.encoderBitrateUpper = encoderSelection.bitrateUpper;
        encoderMetadata.encoderCbrSupported = encoderSelection.cbrSupported;
        encoderMetadata.encoderCbrFdSupported = encoderSelection.cbrFdSupported;
        encoderMetadata.encoderVbrSupported = encoderSelection.vbrSupported;
        encoderMetadata.bitrateModeRequested = encoderSelection.cbrSupported ? "cbr" : "default";
    }

    private static void configureH264Encoder(
        MediaCodec encoder,
        Size size,
        int bitrateBps,
        int frameRateHz,
        EncoderMetadata encoderMetadata) throws Exception {
        encoderMetadata.prependHeadersToSyncFramesRequested = true;
        encoderMetadata.optionalLowLatencyHintsRequested = true;
        int bitrateMode = "cbr".equals(encoderMetadata.bitrateModeRequested)
            ? MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR
            : -1;
        try {
            encoder.configure(
                buildH264EncoderFormat(size, bitrateBps, frameRateHz, true, bitrateMode),
                null,
                null,
                MediaCodec.CONFIGURE_FLAG_ENCODE);
            encoderMetadata.prependHeadersToSyncFramesApplied = true;
            encoderMetadata.optionalLowLatencyHintsApplied = true;
            encoderMetadata.bitrateModeApplied = bitrateModeName(bitrateMode);
        } catch (Exception firstError) {
            encoderMetadata.prependHeadersToSyncFramesApplied = false;
            encoderMetadata.optionalLowLatencyHintsApplied = false;
            encoderMetadata.configureFallbackReason =
                firstError.getClass().getSimpleName() + ": " + safeMessage(firstError);
            try {
                encoder.reset();
            } catch (Exception ignored) {
            }
            try {
                encoder.configure(
                    buildH264EncoderFormat(size, bitrateBps, frameRateHz, false, bitrateMode),
                    null,
                    null,
                    MediaCodec.CONFIGURE_FLAG_ENCODE);
                encoderMetadata.bitrateModeApplied = bitrateModeName(bitrateMode);
            } catch (Exception secondError) {
                if (bitrateMode < 0) {
                    throw secondError;
                }
                encoderMetadata.bitrateModeApplied = "default";
                encoderMetadata.bitrateModeFallbackReason =
                    secondError.getClass().getSimpleName() + ": " + safeMessage(secondError);
                try {
                    encoder.reset();
                } catch (Exception ignored) {
                }
                encoder.configure(
                    buildH264EncoderFormat(size, bitrateBps, frameRateHz, false, -1),
                    null,
                    null,
                    MediaCodec.CONFIGURE_FLAG_ENCODE);
            }
        }
    }

    private static MediaFormat buildH264EncoderFormat(
        Size size,
        int bitrateBps,
        int frameRateHz,
        boolean includeOptionalHints,
        int bitrateMode) {
        MediaFormat format = MediaFormat.createVideoFormat(MIME_H264, size.getWidth(), size.getHeight());
        format.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
        format.setInteger(MediaFormat.KEY_BIT_RATE, bitrateBps);
        format.setInteger(MediaFormat.KEY_FRAME_RATE, frameRateHz);
        format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);
        if (bitrateMode >= 0) {
            format.setInteger(MediaFormat.KEY_BITRATE_MODE, bitrateMode);
        }
        if (includeOptionalHints) {
            format.setInteger(MediaFormat.KEY_PREPEND_HEADER_TO_SYNC_FRAMES, 1);
            format.setInteger(MediaFormat.KEY_PRIORITY, 0);
            format.setInteger(MediaFormat.KEY_LATENCY, 1);
            format.setInteger(MediaFormat.KEY_MAX_B_FRAMES, 0);
            format.setInteger(MediaFormat.KEY_OUTPUT_REORDER_DEPTH, 0);
            format.setFloat(MediaFormat.KEY_MAX_FPS_TO_ENCODER, (float) frameRateHz);
        }
        return format;
    }

    private static String bitrateModeName(int mode) {
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR) {
            return "cbr";
        }
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR_FD) {
            return "cbr_fd";
        }
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_VBR) {
            return "vbr";
        }
        if (mode == MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CQ) {
            return "cq";
        }
        return "default";
    }

    private static boolean safeIsHardwareAccelerated(MediaCodecInfo info) {
        try {
            return info.isHardwareAccelerated();
        } catch (Exception ignored) {
            return false;
        }
    }

    private static boolean safeIsSoftwareOnly(MediaCodecInfo info) {
        try {
            return info.isSoftwareOnly();
        } catch (Exception ignored) {
            return false;
        }
    }

    private static void requestSyncFrameOnStart(MediaCodec encoder, EncoderMetadata encoderMetadata) {
        encoderMetadata.syncFrameRequestOnStartRequested = true;
        Bundle params = new Bundle();
        params.putInt(MediaCodec.PARAMETER_KEY_REQUEST_SYNC_FRAME, 0);
        try {
            encoder.setParameters(params);
            encoderMetadata.syncFrameRequestOnStartSucceeded = true;
        } catch (Exception ex) {
            encoderMetadata.syncFrameRequestOnStartSucceeded = false;
            encoderMetadata.syncFrameRequestOnStartError =
                ex.getClass().getSimpleName() + ": " + safeMessage(ex);
        }
    }

    private static void drainEncoder(
        MediaCodec encoder,
        List<EncodedPacket> packets,
        boolean endOfStream,
        int maxVideoPackets,
        EncoderMetadata encoderMetadata) throws Exception {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        int emptyPolls = 0;
        while (videoPacketCount(packets) < maxVideoPackets) {
            int status = encoder.dequeueOutputBuffer(info, ENCODER_DRAIN_TIMEOUT_US);
            if (status == MediaCodec.INFO_TRY_AGAIN_LATER) {
                if (!endOfStream || emptyPolls++ > 50) {
                    break;
                }
                continue;
            }
            if (status == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                captureEncoderOutputFormat(encoder, encoderMetadata);
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
                EncodedPacket packet = new EncodedPacket(
                        info.presentationTimeUs,
                        info.flags,
                        payload,
                        SystemClock.elapsedRealtimeNanos(),
                        System.currentTimeMillis() * 1_000_000L);
                if (!packet.isCodecConfig() || codecConfigPacketCount(packets) < MAX_CODEC_CONFIG_PACKETS) {
                    packets.add(packet);
                }
            }
            boolean reachedEos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
            encoder.releaseOutputBuffer(status, false);
            if (reachedEos) {
                break;
            }
        }
    }

    private static void drainEncoderToQueue(
        MediaCodec encoder,
        LivePacketQueue packetQueue,
        boolean endOfStream,
        int maxPackets,
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int captureMs,
        int bitrateBps,
        int frameRateHz,
        JSONObject endpoint,
        CameraSelection selection,
        EncoderMetadata encoderMetadata,
        boolean syntheticSource,
        String syntheticPattern) throws Exception {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        int emptyPolls = 0;
        while (maxPackets <= 0 || packetQueue.acceptedPacketCount() < maxPackets) {
            int status = encoder.dequeueOutputBuffer(info, ENCODER_DRAIN_TIMEOUT_US);
            if (status == MediaCodec.INFO_TRY_AGAIN_LATER) {
                if (!endOfStream || emptyPolls++ > 50) {
                    break;
                }
                continue;
            }
            if (status == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                captureEncoderOutputFormat(encoder, encoderMetadata);
                registerManifest(
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    captureMs,
                    maxPackets,
                    bitrateBps,
                    frameRateHz,
                    true,
                    endpoint,
                    selection,
                    encoderMetadata,
                    syntheticSource,
                    syntheticPattern);
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
                EncodedPacket packet = new EncodedPacket(
                    info.presentationTimeUs,
                    info.flags,
                    payload,
                    SystemClock.elapsedRealtimeNanos(),
                    System.currentTimeMillis() * 1_000_000L);
                if (!packet.isCodecConfig() || packetQueue.codecConfigAcceptedCount() < MAX_CODEC_CONFIG_PACKETS) {
                    packetQueue.offer(packet);
                }
            }
            boolean reachedEos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
            encoder.releaseOutputBuffer(status, false);
            if (reachedEos) {
                break;
            }
        }
    }

    private static CameraSelection chooseCamera(
        CameraManager manager,
        String requestedCameraId,
        int preferredWidth,
        int preferredHeight,
        int frameRateHz) throws Exception {
        String[] ids = manager.getCameraIdList();
        CameraSelection best = null;
        for (int i = 0; i < ids.length; i++) {
            String id = ids[i];
            if (requestedCameraId != null && requestedCameraId.length() > 0 && !requestedCameraId.equals(id)) {
                continue;
            }
            CameraCharacteristics characteristics = manager.getCameraCharacteristics(id);
            Size size = chooseSize(characteristics, preferredWidth, preferredHeight);
            if (size == null) {
                continue;
            }
            Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
            boolean back = facing != null && facing == CameraCharacteristics.LENS_FACING_BACK;
            float[] translation = characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION);
            double translationX = translation != null && translation.length > 0 ? translation[0] : 0.0;
            long score = scoreCamera(back, translationX, size, preferredWidth, preferredHeight);
            if (best == null || score > best.score) {
                Range<Integer> fpsRange = chooseFpsRange(characteristics, frameRateHz);
                long streamMinFrameDurationNs = streamMinFrameDurationNs(characteristics, size);
                String selectionReason = requestedCameraId != null && requestedCameraId.length() > 0
                    ? "requested_camera_id_closest_preferred_private_size"
                    : "best_score_lens_pose_and_preferred_size";
                best = new CameraSelection(
                    id,
                    size,
                    score,
                    characteristics,
                    fpsRange,
                    streamMinFrameDurationNs,
                    selectionReason);
            }
        }
        if (best == null) {
            throw new IllegalStateException(requestedCameraId != null && requestedCameraId.length() > 0
                ? "Requested camera has no encoder-compatible output: " + requestedCameraId
                : "No app-visible encoder-compatible camera source found.");
        }
        return best;
    }

    private static Size chooseSize(CameraCharacteristics characteristics, int preferredWidth, int preferredHeight) {
        StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
        if (map == null) {
            return null;
        }
        Size[] sizes = map.getOutputSizes(ImageFormat.PRIVATE);
        if (sizes == null || sizes.length == 0) {
            sizes = map.getOutputSizes(Surface.class);
        }
        if (sizes == null || sizes.length == 0) {
            return null;
        }
        Size best = null;
        long bestScore = Long.MAX_VALUE;
        for (int i = 0; i < sizes.length; i++) {
            Size size = sizes[i];
            long area = (long) size.getWidth() * (long) size.getHeight();
            long preferredArea = (long) preferredWidth * (long) preferredHeight;
            long distance = Math.abs((long) size.getWidth() - preferredWidth) +
                Math.abs((long) size.getHeight() - preferredHeight);
            long underPreferred = area < preferredArea ? preferredArea - area : 0L;
            long score = distance * 10000L + underPreferred;
            if (score < bestScore) {
                bestScore = score;
                best = size;
            }
        }
        return best;
    }

    private static long scoreCamera(
        boolean back,
        double translationX,
        Size size,
        int preferredWidth,
        int preferredHeight) {
        long score = back ? 1_000_000_000L : 0L;
        score -= (long) (Math.abs(translationX) * 1_000_000.0);
        score -= Math.abs((long) size.getWidth() - preferredWidth) * 1000L;
        score -= Math.abs((long) size.getHeight() - preferredHeight) * 1000L;
        return score;
    }

    private static Range<Integer> chooseFpsRange(CameraCharacteristics characteristics, int frameRateHz) {
        Range<Integer>[] ranges = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES);
        if (ranges == null || ranges.length == 0) {
            return null;
        }
        Range<Integer> best = null;
        long bestScore = Long.MAX_VALUE;
        for (int i = 0; i < ranges.length; i++) {
            Range<Integer> range = ranges[i];
            if (range == null || range.getLower() == null || range.getUpper() == null) {
                continue;
            }
            int lower = range.getLower().intValue();
            int upper = range.getUpper().intValue();
            long containsPenalty = lower <= frameRateHz && upper >= frameRateHz ? 0L : 1_000_000L;
            long spanPenalty = Math.max(0, upper - lower);
            long targetPenalty = Math.abs(upper - frameRateHz) * 1000L + Math.abs(lower - frameRateHz);
            long score = containsPenalty + targetPenalty + spanPenalty;
            if (best == null || score < bestScore) {
                best = range;
                bestScore = score;
            }
        }
        return best;
    }

    private static long streamMinFrameDurationNs(CameraCharacteristics characteristics, Size size) {
        StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
        if (map == null || size == null) {
            return 0L;
        }
        try {
            long duration = map.getOutputMinFrameDuration(ImageFormat.PRIVATE, size);
            if (duration > 0L) {
                return duration;
            }
        } catch (Exception ignored) {
        }
        try {
            return map.getOutputMinFrameDuration(Surface.class, size);
        } catch (Exception ignored) {
            return 0L;
        }
    }

    private static void putCameraSourceSelectionFields(
        JSONObject target,
        CameraSelection selection,
        String cameraPermissionState) throws Exception {
        target.put("source_api_path", SOURCE_API_CAMERA2);
        target.put("camera_permission_state", cameraPermissionState);
        target.put("headset_camera_permission_state", cameraPermissionState);
        target.put("timestamp_domain", timestampDomainLabel(selection != null
            ? sensorTimestampSourceLabel(selection.characteristics.get(CameraCharacteristics.SENSOR_INFO_TIMESTAMP_SOURCE))
            : ""));
        if (selection == null) {
            return;
        }
        target.put("camera_source_id", "camera2:" + selection.cameraId);
        target.put("selected_camera_id", selection.cameraId);
        target.put("selected_width", selection.size.getWidth());
        target.put("selected_height", selection.size.getHeight());
        target.put("selected_reason", selection.selectionReason);
        target.put("selection_score", selection.score);
        if (selection.fpsRange != null) {
            target.put("selected_fps_min_hz", selection.fpsRange.getLower().intValue());
            target.put("selected_fps_max_hz", selection.fpsRange.getUpper().intValue());
        }
        target.put("stream_min_frame_duration_ns", selection.streamMinFrameDurationNs);
    }

    private static void putSourceSelectionFields(
        JSONObject target,
        CameraSelection selection,
        boolean syntheticSource,
        String syntheticPattern,
        Size size,
        int frameRateHz) throws Exception {
        if (!syntheticSource) {
            putCameraSourceSelectionFields(target, selection, "Granted");
            return;
        }

        String pattern = normalizeSyntheticPattern(syntheticPattern);
        target.put("source_api_path", SOURCE_API_SYNTHETIC_SURFACE);
        target.put("camera_permission_state", "NotRequired");
        target.put("headset_camera_permission_state", "NotRequired");
        target.put("timestamp_domain", "ElapsedRealtime");
        target.put("camera_source_id", "synthetic:" + pattern);
        target.put("selected_camera_id", "synthetic:" + pattern);
        target.put("selected_width", size != null ? size.getWidth() : 0);
        target.put("selected_height", size != null ? size.getHeight() : 0);
        target.put("selected_reason", "synthetic_diagnostic_source");
        target.put("selected_fps_min_hz", frameRateHz);
        target.put("selected_fps_max_hz", frameRateHz);
        target.put("synthetic_pattern", pattern);
    }

    private static JSONObject buildCameraSourceCapabilities(
        CameraSelection selection,
        String cameraPermissionState) throws Exception {
        JSONObject capabilities = new JSONObject();
        capabilities.put("schema", "rusty.xr.broker.camera_source_capabilities.v1");
        capabilities.put("source_id", "camera2:" + selection.cameraId);
        capabilities.put("source_api_path", "AndroidCamera2");
        capabilities.put("horizon_os_version_observed", JSONObject.NULL);
        capabilities.put("camera_permission_state", cameraPermissionState);
        capabilities.put("headset_camera_permission_state", cameraPermissionState);
        capabilities.put("camera_id", selection.cameraId);
        capabilities.put("physical_camera_ids", new JSONArray());
        capabilities.put("meta_vendor_camera_source", JSONObject.NULL);
        capabilities.put("meta_vendor_position", JSONObject.NULL);
        capabilities.put("supported_private_sizes", outputSizesJson(selection.characteristics, ImageFormat.PRIVATE));
        capabilities.put("supported_yuv_sizes", outputSizesJson(selection.characteristics, ImageFormat.YUV_420_888));
        capabilities.put("supported_fps_ranges", fpsRangesJson(selection.characteristics));
        capabilities.put("selected_size", sizeJson(selection.size));
        capabilities.put("selected_fps_range", selection.fpsRange != null ? fpsRangeJson(selection.fpsRange) : JSONObject.NULL);
        capabilities.put(
            "stream_min_frame_duration_ns",
            selection.streamMinFrameDurationNs > 0L ? selection.streamMinFrameDurationNs : JSONObject.NULL);
        capabilities.put(
            "timestamp_domain",
            timestampDomainLabel(sensorTimestampSourceLabel(
                selection.characteristics.get(CameraCharacteristics.SENSOR_INFO_TIMESTAMP_SOURCE))));
        capabilities.put("selected_reason", selection.selectionReason);
        return capabilities;
    }

    private static JSONArray outputSizesJson(CameraCharacteristics characteristics, int imageFormat) throws Exception {
        JSONArray array = new JSONArray();
        StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
        if (map == null) {
            return array;
        }
        Size[] sizes = map.getOutputSizes(imageFormat);
        if (sizes == null) {
            return array;
        }
        for (int i = 0; i < sizes.length; i++) {
            array.put(sizeJson(sizes[i]));
        }
        return array;
    }

    private static JSONArray fpsRangesJson(CameraCharacteristics characteristics) throws Exception {
        JSONArray array = new JSONArray();
        Range<Integer>[] ranges = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES);
        if (ranges == null) {
            return array;
        }
        for (int i = 0; i < ranges.length; i++) {
            if (ranges[i] != null) {
                array.put(fpsRangeJson(ranges[i]));
            }
        }
        return array;
    }

    private static JSONObject sizeJson(Size size) throws Exception {
        JSONObject json = new JSONObject();
        json.put("width", size != null ? size.getWidth() : 0);
        json.put("height", size != null ? size.getHeight() : 0);
        return json;
    }

    private static JSONObject fpsRangeJson(Range<Integer> range) throws Exception {
        JSONObject json = new JSONObject();
        json.put("min_hz", range.getLower().intValue());
        json.put("max_hz", range.getUpper().intValue());
        return json;
    }

    private static String cameraPermissionState(Context context) {
        if (context == null) {
            return "Unavailable";
        }
        return context.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED
            ? "Granted"
            : "Denied";
    }

    private static String timestampDomainLabel(String sensorTimestampSource) {
        return "REALTIME".equals(sensorTimestampSource) ? "ElapsedRealtime" : "Unknown";
    }

    private static JSONObject buildProjectionMetadata(CameraSelection selection) throws Exception {
        CameraCharacteristics characteristics = selection.characteristics;
        JSONObject metadata = new JSONObject();
        metadata.put("schema", "rusty.xr.camera_projection.stream_source_metadata.v1");
        metadata.put("source", "broker_app.camera2_h264_stream");
        metadata.put("sourceLabel", "Broker app Camera2 H.264 source " + selection.cameraId);
        metadata.put("cameraId", selection.cameraId);
        metadata.put("selectionScore", selection.score);
        metadata.put("deliveredWidth", selection.size.getWidth());
        metadata.put("deliveredHeight", selection.size.getHeight());

        Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
        metadata.put("lensFacing", lensFacingLabel(facing));
        metadata.put("lensFacingRank", lensFacingRank(facing));
        Integer sensorOrientation = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);
        if (sensorOrientation != null) {
            metadata.put("sensorOrientationDegrees", sensorOrientation.intValue());
        }

        Rect activeArray = characteristics.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE);
        Size sensorPixelArray = characteristics.get(CameraCharacteristics.SENSOR_INFO_PIXEL_ARRAY_SIZE);
        float[] calibration = characteristics.get(CameraCharacteristics.LENS_INTRINSIC_CALIBRATION);
        int intrinsicsWidth = activeArray != null ? activeArray.width() : (sensorPixelArray != null ? sensorPixelArray.getWidth() : 0);
        int intrinsicsHeight = activeArray != null ? activeArray.height() : (sensorPixelArray != null ? sensorPixelArray.getHeight() : 0);
        boolean hasIntrinsics = calibration != null && calibration.length >= 4 && intrinsicsWidth > 0 && intrinsicsHeight > 0;
        if (hasIntrinsics) {
            JSONObject intrinsics = new JSONObject();
            intrinsics.put("fx", calibration[0]);
            intrinsics.put("fy", calibration[1]);
            intrinsics.put("cx", calibration[2]);
            intrinsics.put("cy", calibration[3]);
            intrinsics.put("skew", calibration.length >= 5 ? calibration[4] : 0.0f);
            metadata.put("intrinsics", intrinsics);
            metadata.put(
                "intrinsicsDomain",
                pixelDomain(activeArray != null ? "activeArray" : "sensorPixelArray", intrinsicsWidth, intrinsicsHeight));
        }
        if (activeArray != null) {
            metadata.put("activeArrayDomain", pixelDomain("activeArray", activeArray.width(), activeArray.height()));
        }
        if (sensorPixelArray != null) {
            metadata.put("sensorPixelDomain", pixelDomain("sensorPixelArray", sensorPixelArray.getWidth(), sensorPixelArray.getHeight()));
        }

        float[] translation = characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION);
        float[] rotation = characteristics.get(CameraCharacteristics.LENS_POSE_ROTATION);
        Integer reference = characteristics.get(CameraCharacteristics.LENS_POSE_REFERENCE);
        float[] normalizedRotation = normalizeQuaternionOrNull(rotation);
        boolean hasPose = isFiniteArray(translation, 3) &&
            normalizedRotation != null &&
            isAcceptedLensPoseReference(reference);
        metadata.put("missingIntrinsics", !hasIntrinsics);
        metadata.put("missingPose", !hasPose);
        metadata.put("poseSource", hasPose ? "platform" : "missing");
        metadata.put(
            "poseCoordinateConvention",
            hasPose
                ? "android-camera2-lens-pose-reference-from-camera"
                : "broker-decoded-h264-image-space");
        metadata.put("lensPoseReferenceLabel", lensPoseReferenceLabel(reference));
        if (hasPose) {
            JSONObject extrinsics = new JSONObject();
            extrinsics.put("px", translation[0]);
            extrinsics.put("py", translation[1]);
            extrinsics.put("pz", translation[2]);
            extrinsics.put("qx", normalizedRotation[0]);
            extrinsics.put("qy", normalizedRotation[1]);
            extrinsics.put("qz", normalizedRotation[2]);
            extrinsics.put("qw", normalizedRotation[3]);
            metadata.put("extrinsics", extrinsics);
        }
        metadata.put("projectionMetadataReady", hasIntrinsics && hasPose);
        return metadata;
    }

    private static JSONObject buildSyntheticProjectionMetadata(Size size, String syntheticPattern) throws Exception {
        String pattern = normalizeSyntheticPattern(syntheticPattern);
        int width = size != null ? size.getWidth() : 0;
        int height = size != null ? size.getHeight() : 0;
        JSONObject metadata = new JSONObject();
        metadata.put("schema", "rusty.xr.camera_projection.stream_source_metadata.v1");
        metadata.put("source", "broker_app.synthetic_h264_stream");
        metadata.put("sourceLabel", "Broker synthetic H.264 diagnostic source");
        metadata.put("cameraId", "synthetic:" + pattern);
        metadata.put("selectionScore", 0L);
        metadata.put("deliveredWidth", width);
        metadata.put("deliveredHeight", height);
        metadata.put("lensFacing", "synthetic");
        metadata.put("lensFacingRank", 0);
        if (width > 0 && height > 0) {
            double fovRadians = Math.toRadians(SYNTHETIC_PROJECTION_FOV_Y_DEGREES);
            double focal = height / (2.0 * Math.tan(fovRadians * 0.5));
            JSONObject intrinsics = new JSONObject();
            intrinsics.put("fx", focal);
            intrinsics.put("fy", focal);
            intrinsics.put("cx", width * 0.5);
            intrinsics.put("cy", height * 0.5);
            intrinsics.put("skew", 0.0);
            metadata.put("intrinsics", intrinsics);
            metadata.put("intrinsicsDomain", pixelDomain("deliveredImage", width, height));
            metadata.put("sensorPixelDomain", pixelDomain("deliveredImage", width, height));

            JSONObject extrinsics = new JSONObject();
            extrinsics.put("px", 0.0);
            extrinsics.put("py", 0.0);
            extrinsics.put("pz", 0.0);
            extrinsics.put("qx", 1.0);
            extrinsics.put("qy", 0.0);
            extrinsics.put("qz", 0.0);
            extrinsics.put("qw", 0.0);
            metadata.put("extrinsics", extrinsics);
        }
        metadata.put("missingIntrinsics", width <= 0 || height <= 0);
        metadata.put("missingPose", width <= 0 || height <= 0);
        metadata.put("poseSource", width > 0 && height > 0 ? "estimated-profile" : "synthetic");
        metadata.put("poseCoordinateConvention", "broker-synthetic-head-anchored-preview");
        metadata.put("lensPoseReferenceLabel", "synthetic-head");
        metadata.put("projectionMetadataReady", width > 0 && height > 0);
        metadata.put("diagnosticSource", true);
        metadata.put("syntheticPattern", pattern);
        metadata.put("syntheticProjectionFovYDegrees", SYNTHETIC_PROJECTION_FOV_Y_DEGREES);
        return metadata;
    }

    private static void drawSyntheticEncoderFrame(
        Surface surface,
        int frameIndex,
        Size size,
        String syntheticPattern) throws Exception {
        Canvas canvas = surface.lockCanvas(null);
        try {
            String pattern = normalizeSyntheticPattern(syntheticPattern);
            Paint paint = new Paint();
            paint.setAntiAlias(false);
            int width = size.getWidth();
            int height = size.getHeight();
            if ("checkerboard".equals(pattern)) {
                drawSyntheticCheckerboard(canvas, paint, width, height);
            } else if ("luma-ramp".equals(pattern)) {
                drawSyntheticLumaRamp(canvas, paint, width, height);
            } else {
                drawSyntheticDiagnosticGrid(canvas, paint, width, height);
            }
            if ("motion-bar".equals(pattern)) {
                drawSyntheticMotionMarker(canvas, paint, width, height, frameIndex);
            }
        } finally {
            surface.unlockCanvasAndPost(canvas);
        }
    }

    private static void drawSyntheticDiagnosticGrid(Canvas canvas, Paint paint, int width, int height) {
        canvas.drawColor(Color.rgb(8, 8, 8));
        int barHeight = Math.max(24, height / 10);
        int[] colors = new int[] {
            Color.WHITE,
            Color.YELLOW,
            Color.CYAN,
            Color.GREEN,
            Color.MAGENTA,
            Color.RED,
            Color.BLUE,
            Color.BLACK
        };
        int barWidth = Math.max(1, width / colors.length);
        for (int i = 0; i < colors.length; i++) {
            paint.setColor(colors[i]);
            canvas.drawRect(new Rect(i * barWidth, 0, i == colors.length - 1 ? width : (i + 1) * barWidth, barHeight), paint);
        }

        int rampTop = barHeight + Math.max(8, height / 32);
        int rampHeight = Math.max(24, height / 6);
        for (int x = 0; x < width; x++) {
            int luma = width <= 1 ? 0 : (int) Math.round(255.0 * x / (double) (width - 1));
            paint.setColor(Color.rgb(luma, luma, luma));
            canvas.drawLine(x, rampTop, x, rampTop + rampHeight, paint);
        }

        int checkerTop = rampTop + rampHeight + Math.max(8, height / 32);
        int cell = Math.max(16, Math.min(width, height) / 12);
        drawSyntheticDiagnosticCheckerboard(canvas, paint, width, height, checkerTop, cell);
    }

    private static void drawSyntheticDiagnosticCheckerboard(
        Canvas canvas,
        Paint paint,
        int width,
        int height,
        int top,
        int cell) {
        for (int y = top; y < height; y += cell) {
            for (int x = 0; x < width; x += cell) {
                int cellX = x / cell;
                int cellY = (y - top) / cell;
                boolean high = ((cellX + cellY) & 1) == 0;
                paint.setColor(high ? Color.rgb(224, 224, 224) : Color.rgb(32, 32, 32));
                canvas.drawRect(new Rect(x, y, Math.min(width, x + cell), Math.min(height, y + cell)), paint);
            }
        }

        drawSyntheticThinLineOverlay(canvas, paint, 0, top, width, height, cell);
    }

    private static void drawSyntheticThinLineOverlay(
        Canvas canvas,
        Paint paint,
        int left,
        int top,
        int right,
        int bottom,
        int cell) {
        if (left >= right || top >= bottom || cell <= 1) {
            return;
        }

        paint.setColor(Color.rgb(255, 255, 255));
        int offset = Math.max(1, cell / 2);
        for (int x = left + offset; x < right; x += cell) {
            canvas.drawRect(new Rect(x, top, Math.min(right, x + 1), bottom), paint);
        }
        for (int y = top + offset; y < bottom; y += cell) {
            canvas.drawRect(new Rect(left, y, right, Math.min(bottom, y + 1)), paint);
        }
    }

    private static void drawSyntheticCheckerboard(Canvas canvas, Paint paint, int width, int height) {
        int cell = Math.max(16, Math.min(width, height) / 10);
        for (int y = 0; y < height; y += cell) {
            for (int x = 0; x < width; x += cell) {
                boolean high = (((x / cell) + (y / cell)) & 1) == 0;
                paint.setColor(high ? Color.WHITE : Color.BLACK);
                canvas.drawRect(new Rect(x, y, Math.min(width, x + cell), Math.min(height, y + cell)), paint);
            }
        }
    }

    private static void drawSyntheticLumaRamp(Canvas canvas, Paint paint, int width, int height) {
        for (int x = 0; x < width; x++) {
            int luma = width <= 1 ? 0 : (int) Math.round(255.0 * x / (double) (width - 1));
            paint.setColor(Color.rgb(luma, luma, luma));
            canvas.drawLine(x, 0, x, height, paint);
        }
    }

    private static void drawSyntheticMotionMarker(Canvas canvas, Paint paint, int width, int height, int frameIndex) {
        int markerWidth = Math.max(16, width / 12);
        int markerHeight = Math.max(16, height / 8);
        int travel = Math.max(1, width - markerWidth);
        int x = (frameIndex * Math.max(1, width / 30)) % travel;
        int y = Math.max(0, height - markerHeight - Math.max(8, height / 32));
        paint.setColor(Color.rgb(0, 255, 96));
        canvas.drawRect(new Rect(x, y, x + markerWidth, y + markerHeight), paint);
    }

    private static JSONObject pixelDomain(String kind, int width, int height) throws Exception {
        JSONObject domain = new JSONObject();
        domain.put("kind", kind);
        domain.put("width", width);
        domain.put("height", height);
        return domain;
    }

    private static String lensFacingLabel(Integer facing) {
        if (facing == null) {
            return "unknown";
        }
        int value = facing.intValue();
        if (value == CameraCharacteristics.LENS_FACING_BACK) {
            return "back";
        }
        if (value == CameraCharacteristics.LENS_FACING_FRONT) {
            return "front";
        }
        if (value == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return "external";
        }
        return "unknown";
    }

    private static int lensFacingRank(Integer facing) {
        if (facing == null) {
            return 0;
        }
        int value = facing.intValue();
        if (value == CameraCharacteristics.LENS_FACING_BACK) {
            return 3;
        }
        if (value == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return 2;
        }
        if (value == CameraCharacteristics.LENS_FACING_FRONT) {
            return 1;
        }
        return 0;
    }

    private static boolean isAcceptedLensPoseReference(Integer reference) {
        if (reference == null) {
            return false;
        }
        int value = reference.intValue();
        return value == CameraCharacteristics.LENS_POSE_REFERENCE_PRIMARY_CAMERA
            || value == CameraCharacteristics.LENS_POSE_REFERENCE_GYROSCOPE;
    }

    private static String lensPoseReferenceLabel(Integer reference) {
        if (reference == null) {
            return "missing";
        }
        int value = reference.intValue();
        if (value == CameraCharacteristics.LENS_POSE_REFERENCE_PRIMARY_CAMERA) {
            return "PRIMARY_CAMERA";
        }
        if (value == CameraCharacteristics.LENS_POSE_REFERENCE_GYROSCOPE) {
            return "GYROSCOPE";
        }
        if (value == CameraCharacteristics.LENS_POSE_REFERENCE_UNDEFINED) {
            return "UNDEFINED";
        }
        return "other";
    }

    private static String sensorTimestampSourceLabel(Integer source) {
        if (source == null) {
            return "missing";
        }
        int value = source.intValue();
        if (value == CameraMetadata.SENSOR_INFO_TIMESTAMP_SOURCE_REALTIME) {
            return "REALTIME";
        }
        if (value == CameraMetadata.SENSOR_INFO_TIMESTAMP_SOURCE_UNKNOWN) {
            return "UNKNOWN";
        }
        return "other";
    }

    private static boolean isFiniteArray(float[] values, int expectedLength) {
        if (values == null || values.length < expectedLength) {
            return false;
        }
        for (int i = 0; i < expectedLength; i++) {
            if (Float.isNaN(values[i]) || Float.isInfinite(values[i])) {
                return false;
            }
        }
        return true;
    }

    private static float[] normalizeQuaternionOrNull(float[] quaternion) {
        if (!isFiniteArray(quaternion, 4)) {
            return null;
        }
        double normSquared =
            quaternion[0] * quaternion[0] +
            quaternion[1] * quaternion[1] +
            quaternion[2] * quaternion[2] +
            quaternion[3] * quaternion[3];
        if (Double.isNaN(normSquared) || Double.isInfinite(normSquared) || normSquared <= 0.000001) {
            return null;
        }
        double invNorm = 1.0 / Math.sqrt(normSquared);
        return new float[] {
            (float) (quaternion[0] * invNorm),
            (float) (quaternion[1] * invNorm),
            (float) (quaternion[2] * invNorm),
            (float) (quaternion[3] * invNorm)
        };
    }

    private static StreamWriteStats writePackets(
        int devicePort,
        String bindHost,
        Size size,
        List<EncodedPacket> packets,
        JSONObject streamProjectionMetadata) throws Exception {
        if (packets.size() == 0) {
            throw new IllegalStateException("No H.264 packets were available to stream.");
        }
        ServerSocket server = new ServerSocket(devicePort, 1, InetAddress.getByName(bindHost));
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        try {
            server.setSoTimeout(15000);
            Socket client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            try {
                client.setTcpNoDelay(true);
                OutputStream output = client.getOutputStream();
                writeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                writeEncodedPacketStream(output, size, packets, streamProjectionMetadata);
                output.flush();
                writeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            } finally {
                client.close();
            }
        } finally {
            server.close();
        }
        return new StreamWriteStats(listenStartElapsedNs, acceptElapsedNs, writeStartElapsedNs, writeEndElapsedNs);
    }

    private static void writeEncodedPacketStream(
        OutputStream output,
        Size size,
        List<EncodedPacket> packets,
        JSONObject streamProjectionMetadata) throws Exception {
        writeStreamHeader(output, size, packets.size(), streamProjectionMetadata);
        for (int i = 0; i < packets.size(); i++) {
            writeEncodedPacket(output, packets.get(i));
        }
    }

    private static void writeStreamHeader(
        OutputStream output,
        Size size,
        int packetCount,
        JSONObject streamProjectionMetadata) throws Exception {
        byte[] metadataBytes = streamProjectionMetadata != null
            ? streamProjectionMetadata.toString().getBytes(StandardCharsets.UTF_8)
            : new byte[0];
        if (metadataBytes.length > MAX_STREAM_HEADER_METADATA_BYTES) {
            throw new IllegalStateException("Projection metadata header is too large: " + metadataBytes.length);
        }
        output.write(MAGIC.getBytes(StandardCharsets.US_ASCII));
        writeU32(output, SCHEMA_VERSION);
        writeU32(output, CODEC_H264);
        writeU32(output, size.getWidth());
        writeU32(output, size.getHeight());
        writeU32(output, packetCount);
        writeU32(output, metadataBytes.length);
        if (metadataBytes.length > 0) {
            output.write(metadataBytes);
        }
    }

    private static void writeEncodedPacket(OutputStream output, EncodedPacket packet) throws Exception {
        writeU64(output, packet.ptsUs);
        writeU32(output, packet.flags);
        writeU32(output, packet.payload.length);
        writeU64(output, packet.encoderOutputElapsedNs);
        writeU64(output, packet.encoderOutputUnixNs);
        output.write(packet.payload);
    }

    private static void captureEncoderOutputFormat(MediaCodec encoder, EncoderMetadata encoderMetadata) {
        encoderMetadata.outputFormatChangeCount++;
        MediaFormat outputFormat = encoder.getOutputFormat();
        encoderMetadata.outputMime = mediaFormatString(outputFormat, MediaFormat.KEY_MIME, MIME_H264);
        encoderMetadata.outputWidth = mediaFormatInt(outputFormat, MediaFormat.KEY_WIDTH, 0);
        encoderMetadata.outputHeight = mediaFormatInt(outputFormat, MediaFormat.KEY_HEIGHT, 0);
        encoderMetadata.profile = mediaFormatInt(outputFormat, MediaFormat.KEY_PROFILE, -1);
        encoderMetadata.level = mediaFormatInt(outputFormat, MediaFormat.KEY_LEVEL, -1);
        encoderMetadata.colorStandard = mediaFormatInt(outputFormat, MediaFormat.KEY_COLOR_STANDARD, -1);
        encoderMetadata.colorRange = mediaFormatInt(outputFormat, MediaFormat.KEY_COLOR_RANGE, -1);
        encoderMetadata.colorTransfer = mediaFormatInt(outputFormat, MediaFormat.KEY_COLOR_TRANSFER, -1);
        encoderMetadata.appliedLatencyFrames = mediaFormatInt(outputFormat, MediaFormat.KEY_LATENCY, -1);
        encoderMetadata.appliedOutputReorderDepth = mediaFormatInt(outputFormat, MediaFormat.KEY_OUTPUT_REORDER_DEPTH, -1);
        encoderMetadata.outputBitrateMode = bitrateModeName(mediaFormatInt(outputFormat, MediaFormat.KEY_BITRATE_MODE, -1));
        encoderMetadata.csdSps = mediaFormatByteBuffer(outputFormat, "csd-0");
        encoderMetadata.csdPps = mediaFormatByteBuffer(outputFormat, "csd-1");
    }

    private static void registerManifest(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int captureMs,
        int maxPackets,
        int bitrateBps,
        int frameRateHz,
        boolean liveStream,
        JSONObject endpoint,
        CameraSelection selection,
        EncoderMetadata encoderMetadata,
        boolean syntheticSource,
        String syntheticPattern) throws Exception {
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", syntheticSource ? STREAM_ID_SYNTHETIC_H264 : STREAM_ID_CAMERA_H264);
        manifest.put("session_id", sessionId);
        manifest.put("source", syntheticSource ? SOURCE_SYNTHETIC_H264 : SOURCE_CAMERA_H264);
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "adb_forwarded_tcp_binary");
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "surface");
        manifest.put("width", size.getWidth());
        manifest.put("height", size.getHeight());
        manifest.put("frame_rate_hz", frameRateHz);
        manifest.put("bitrate_bps", bitrateBps);
        manifest.put("source_kind", syntheticSource ? SOURCE_SYNTHETIC_H264 : SOURCE_CAMERA_H264);
        manifest.put("source_mode", syntheticSource ? SOURCE_MODE_SYNTHETIC_SURFACE : SOURCE_MODE_CAMERA2);
        manifest.put("camera_id", cameraId);
        manifest.put("capture_ms", captureMs);
        manifest.put("max_packets", maxPackets);
        manifest.put("live_stream", liveStream);
        manifest.put("stream_mode", streamMode(liveStream, captureMs, maxPackets));
        manifest.put("writer_backpressure_isolated", liveStream);
        manifest.put("writer_queue_depth", liveStream && endpoint != null ? endpoint.optInt("writer_queue_depth", 0) : 0);
        manifest.put("binary_schema_version", SCHEMA_VERSION);
        manifest.put("binary_endpoint", endpoint);
        putSourceSelectionFields(manifest, selection, syntheticSource, syntheticPattern, size, frameRateHz);
        if (selection != null) {
            manifest.put("camera_source_capabilities", buildCameraSourceCapabilities(selection, "Granted"));
        }
        putEncoderMetadata(manifest, encoderMetadata);
        sink.registerManifest(manifest);
    }

    private static void recordSample(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int index,
        EncodedPacket packet,
        boolean liveStream,
        boolean syntheticSource) throws Exception {
        JSONObject sample = new JSONObject();
        sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
        sample.put("stream_id", syntheticSource ? STREAM_ID_SYNTHETIC_H264 : STREAM_ID_CAMERA_H264);
        sample.put("session_id", sessionId);
        sample.put("sequence_id", System.currentTimeMillis() * 1000L + index);
        sample.put("source", syntheticSource ? SOURCE_SYNTHETIC_H264 : SOURCE_CAMERA_H264);
        sample.put("source_kind", syntheticSource ? SOURCE_SYNTHETIC_H264 : SOURCE_CAMERA_H264);
        sample.put("source_mode", syntheticSource ? SOURCE_MODE_SYNTHETIC_SURFACE : SOURCE_MODE_CAMERA2);
        sample.put("transport", "metadata_only");
        sample.put("payload_transport", "adb_forwarded_tcp_binary");
        sample.put("mime_type", "video/avc");
        sample.put("codec", "h264");
        sample.put("camera_id", cameraId);
        sample.put("encoded_size_bytes", packet.payload.length);
        sample.put("width", size.getWidth());
        sample.put("height", size.getHeight());
        sample.put("key_frame", (packet.flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0);
        sample.put("codec_config", packet.isCodecConfig());
        sample.put("video_frame", !packet.isCodecConfig());
        sample.put("pts_us", packet.ptsUs);
        sample.put("dts_us", packet.ptsUs);
        sample.put("source_time_unix_ns", packet.encoderOutputUnixNs);
        sample.put("source_time_elapsed_ns", packet.encoderOutputElapsedNs);
        sample.put("encoder_output_unix_ns", packet.encoderOutputUnixNs);
        sample.put("encoder_output_elapsed_ns", packet.encoderOutputElapsedNs);
        sample.put("stream_mode", liveStream ? "live_stream" : "bounded_capture_then_write");
        sink.recordSample(sample);
    }

    private static String streamMode(boolean liveStream, int captureMs, int maxPackets) {
        if (!liveStream) {
            return "bounded_capture_then_write";
        }
        if (captureMs <= 0 && maxPackets <= 0) {
            return "live_unbounded";
        }
        if (captureMs <= 0) {
            return "live_until_packet_count";
        }
        if (maxPackets <= 0) {
            return "live_until_capture_timeout";
        }
        return "live_bounded";
    }

    private static void recordMetric(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        List<EncodedPacket> packets,
        long encodeStartElapsedNs,
        long encodeEndElapsedNs,
        StreamWriteStats writeStats,
        int captureMs,
        int maxPackets,
        int frameRateHz,
        boolean liveStream,
        CameraSelection selection,
        EncoderMetadata encoderMetadata,
        String lastError,
        boolean syntheticSource,
        String syntheticPattern) throws Exception {
        long payloadBytes = 0L;
        for (int i = 0; i < packets.size(); i++) {
            payloadBytes += packets.get(i).payload.length;
        }
        JSONObject metric = new JSONObject();
        metric.put("schema", "rusty.xr.video_lab.metric_sample.v1");
        metric.put("stream_id", syntheticSource ? STREAM_ID_SYNTHETIC_H264 : STREAM_ID_CAMERA_H264);
        metric.put("source", syntheticSource ? SOURCE_SYNTHETIC_H264 : SOURCE_CAMERA_H264);
        metric.put("source_kind", syntheticSource ? SOURCE_SYNTHETIC_H264 : SOURCE_CAMERA_H264);
        metric.put("source_mode", syntheticSource ? SOURCE_MODE_SYNTHETIC_SURFACE : SOURCE_MODE_CAMERA2);
        metric.put("transport", "metadata_only");
        metric.put("payload_transport", "adb_forwarded_tcp_binary");
        metric.put("codec", "h264");
        metric.put("session_id", sessionId);
        metric.put("camera_id", cameraId != null ? cameraId : "");
        metric.put("sequence_id", System.currentTimeMillis() * 1000L);
        metric.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
        metric.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        metric.put("camera_encode_start_elapsed_ns", encodeStartElapsedNs);
        metric.put("camera_encode_end_elapsed_ns", encodeEndElapsedNs);
        metric.put("camera_encode_duration_ns", Math.max(0L, encodeEndElapsedNs - encodeStartElapsedNs));
        metric.put("live_stream", liveStream);
        metric.put("frame_rate_hz", frameRateHz);
        metric.put("stream_mode", streamMode(liveStream, captureMs, maxPackets));
        metric.put("binary_listen_start_elapsed_ns", writeStats.listenStartElapsedNs);
        metric.put("binary_accept_elapsed_ns", writeStats.acceptElapsedNs);
        metric.put("binary_write_start_elapsed_ns", writeStats.writeStartElapsedNs);
        metric.put("binary_write_end_elapsed_ns", writeStats.writeEndElapsedNs);
        metric.put("binary_write_duration_ns", Math.max(0L, writeStats.writeEndElapsedNs - writeStats.writeStartElapsedNs));
        metric.put("packet_count", packets.size());
        metric.put("video_packet_count", videoPacketCount(packets));
        metric.put("codec_config_packet_count", codecConfigPacketCount(packets));
        metric.put("keyframe_count", keyFrameCount(packets));
        metric.put("sps_present", byteLength(encoderMetadata != null ? encoderMetadata.csdSps : null) > 0);
        metric.put("pps_present", byteLength(encoderMetadata != null ? encoderMetadata.csdPps : null) > 0);
        metric.put("payload_size_bytes", payloadBytes);
        metric.put("dropped_frames", writeStats.writerQueueDroppedVideoPackets);
        metric.put("stale_frames", 0);
        metric.put("queue_depth", writeStats.writerQueueFinalDepth);
        metric.put("writer_backpressure_isolated", writeStats.writerBackpressureIsolated);
        metric.put("writer_packet_count", writeStats.writerPacketCount);
        metric.put("writer_queue_capacity", writeStats.writerQueueCapacity);
        metric.put("writer_queue_enqueued_packets", writeStats.writerQueueEnqueuedPackets);
        metric.put("writer_queue_max_depth", writeStats.writerQueueMaxDepth);
        metric.put("writer_queue_final_depth", writeStats.writerQueueFinalDepth);
        metric.put("writer_queue_dropped_packets", writeStats.writerQueueDroppedPackets);
        metric.put("writer_queue_dropped_video_packets", writeStats.writerQueueDroppedVideoPackets);
        metric.put("writer_queue_dropped_non_keyframe_packets", writeStats.writerQueueDroppedNonKeyframePackets);
        metric.put("writer_queue_dropped_keyframe_packets", writeStats.writerQueueDroppedKeyframePackets);
        metric.put("writer_queue_dropped_codec_config_packets", writeStats.writerQueueDroppedCodecConfigPackets);
        metric.put("writer_queue_dropped_incoming_packets", writeStats.writerQueueDroppedIncomingPackets);
        if (writeStats.writerError.length() > 0) {
            metric.put("writer_error", writeStats.writerError);
        }
        metric.put("width", size != null ? size.getWidth() : 0);
        metric.put("height", size != null ? size.getHeight() : 0);
        putSourceSelectionFields(metric, selection, syntheticSource, syntheticPattern, size, frameRateHz);
        if (lastError != null && lastError.length() > 0) {
            metric.put("last_error", lastError);
        }
        putEncoderMetadata(metric, encoderMetadata);
        sink.recordMetric(metric);
    }

    private static void putEncoderMetadata(JSONObject target, EncoderMetadata encoderMetadata) throws Exception {
        if (encoderMetadata == null) {
            return;
        }
        target.put("encoder_name", encoderMetadata.encoderName);
        target.put("encoder_selection_source", encoderMetadata.encoderSelectionSource);
        target.put("encoder_selected_name", encoderMetadata.encoderSelectedName);
        target.put("encoder_hardware_accelerated", encoderMetadata.encoderHardwareAccelerated);
        target.put("encoder_software_only", encoderMetadata.encoderSoftwareOnly);
        target.put("encoder_size_supported", encoderMetadata.encoderSizeSupported);
        target.put("encoder_size_and_rate_supported", encoderMetadata.encoderSizeAndRateSupported);
        target.put("encoder_bitrate_supported", encoderMetadata.encoderBitrateSupported);
        target.put("encoder_width_alignment", encoderMetadata.encoderWidthAlignment);
        target.put("encoder_height_alignment", encoderMetadata.encoderHeightAlignment);
        target.put("encoder_bitrate_lower", encoderMetadata.encoderBitrateLower);
        target.put("encoder_bitrate_upper", encoderMetadata.encoderBitrateUpper);
        target.put("encoder_cbr_supported", encoderMetadata.encoderCbrSupported);
        target.put("encoder_cbr_fd_supported", encoderMetadata.encoderCbrFdSupported);
        target.put("encoder_vbr_supported", encoderMetadata.encoderVbrSupported);
        target.put("bitrate_mode_requested", encoderMetadata.bitrateModeRequested);
        target.put("bitrate_mode_applied", encoderMetadata.bitrateModeApplied);
        target.put("bitrate_mode_output_format", encoderMetadata.outputBitrateMode);
        target.put("encoder_output_format_changes", encoderMetadata.outputFormatChangeCount);
        target.put("encoder_output_mime", encoderMetadata.outputMime);
        target.put("encoder_output_width", encoderMetadata.outputWidth);
        target.put("encoder_output_height", encoderMetadata.outputHeight);
        target.put("encoder_profile", encoderMetadata.profile);
        target.put("encoder_level", encoderMetadata.level);
        target.put("encoder_color_standard", encoderMetadata.colorStandard);
        target.put("encoder_color_range", encoderMetadata.colorRange);
        target.put("encoder_color_transfer", encoderMetadata.colorTransfer);
        target.put("encoder_latency_requested_frames", 1);
        target.put("encoder_latency_applied_frames", encoderMetadata.appliedLatencyFrames);
        target.put("encoder_output_reorder_depth_requested", 0);
        target.put("encoder_output_reorder_depth_applied", encoderMetadata.appliedOutputReorderDepth);
        target.put("prepend_headers_to_sync_frames_requested", encoderMetadata.prependHeadersToSyncFramesRequested);
        target.put("prepend_headers_to_sync_frames_applied", encoderMetadata.prependHeadersToSyncFramesApplied);
        target.put("encoder_optional_low_latency_hints_requested", encoderMetadata.optionalLowLatencyHintsRequested);
        target.put("encoder_optional_low_latency_hints_applied", encoderMetadata.optionalLowLatencyHintsApplied);
        target.put("sync_frame_request_on_start_requested", encoderMetadata.syncFrameRequestOnStartRequested);
        target.put("sync_frame_request_on_start_succeeded", encoderMetadata.syncFrameRequestOnStartSucceeded);
        target.put("csd_sps_bytes", byteLength(encoderMetadata.csdSps));
        target.put("csd_pps_bytes", byteLength(encoderMetadata.csdPps));
        target.put("csd_sps_base64", base64NoWrap(encoderMetadata.csdSps));
        target.put("csd_pps_base64", base64NoWrap(encoderMetadata.csdPps));
        target.put("sensor_timestamp_source", encoderMetadata.sensorTimestampSource);
        target.put("camera_capture_started_count", encoderMetadata.cameraCaptureStartedCount);
        target.put("camera_first_capture_started_ns", encoderMetadata.cameraFirstCaptureStartedNs);
        target.put("camera_last_capture_started_ns", encoderMetadata.cameraLastCaptureStartedNs);
        target.put("camera_first_frame_number", encoderMetadata.cameraFirstFrameNumber);
        target.put("camera_last_frame_number", encoderMetadata.cameraLastFrameNumber);
        target.put("camera_first_capture_callback_elapsed_ns", encoderMetadata.cameraFirstCaptureCallbackElapsedNs);
        target.put("camera_last_capture_callback_elapsed_ns", encoderMetadata.cameraLastCaptureCallbackElapsedNs);
        if (encoderMetadata.configureFallbackReason.length() > 0) {
            target.put("encoder_configure_fallback_reason", encoderMetadata.configureFallbackReason);
        }
        if (encoderMetadata.encoderSelectionFallbackReason.length() > 0) {
            target.put("encoder_selection_fallback_reason", encoderMetadata.encoderSelectionFallbackReason);
        }
        if (encoderMetadata.bitrateModeFallbackReason.length() > 0) {
            target.put("bitrate_mode_fallback_reason", encoderMetadata.bitrateModeFallbackReason);
        }
        if (encoderMetadata.syncFrameRequestOnStartError.length() > 0) {
            target.put("sync_frame_request_on_start_error", encoderMetadata.syncFrameRequestOnStartError);
        }
    }

    private static void writeU32(OutputStream output, int value) throws Exception {
        output.write((value >>> 24) & 0xff);
        output.write((value >>> 16) & 0xff);
        output.write((value >>> 8) & 0xff);
        output.write(value & 0xff);
    }

    private static void writeU64(OutputStream output, long value) throws Exception {
        output.write((int) ((value >>> 56) & 0xff));
        output.write((int) ((value >>> 48) & 0xff));
        output.write((int) ((value >>> 40) & 0xff));
        output.write((int) ((value >>> 32) & 0xff));
        output.write((int) ((value >>> 24) & 0xff));
        output.write((int) ((value >>> 16) & 0xff));
        output.write((int) ((value >>> 8) & 0xff));
        output.write((int) (value & 0xff));
    }

    private static int videoPacketCount(List<EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (!packets.get(i).isCodecConfig()) {
                count++;
            }
        }
        return count;
    }

    private static int codecConfigPacketCount(List<EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (packets.get(i).isCodecConfig()) {
                count++;
            }
        }
        return count;
    }

    private static int keyFrameCount(List<EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (packets.get(i).isKeyFrame()) {
                count++;
            }
        }
        return count;
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

    private static byte[] mediaFormatByteBuffer(MediaFormat format, String key) {
        try {
            if (!format.containsKey(key)) {
                return null;
            }
            ByteBuffer buffer = format.getByteBuffer(key);
            if (buffer == null) {
                return null;
            }
            ByteBuffer duplicate = buffer.duplicate();
            byte[] bytes = new byte[duplicate.remaining()];
            duplicate.get(bytes);
            return bytes.length > 0 ? bytes : null;
        } catch (Exception ignored) {
            return null;
        }
    }

    private static int byteLength(byte[] bytes) {
        return bytes != null ? bytes.length : 0;
    }

    private static String base64NoWrap(byte[] bytes) {
        return bytes != null && bytes.length > 0
            ? Base64.encodeToString(bytes, Base64.NO_WRAP)
            : "";
    }

    private static String safeCodecName(MediaCodec codec) {
        try {
            String name = codec.getName();
            return name != null ? name : "";
        } catch (Exception ignored) {
            return "";
        }
    }

    private static String normalizeBindHost(String requestedHost, boolean lanStreamEnabled) {
        String host = requestedHost != null ? requestedHost.trim() : "";
        if (host.length() == 0) {
            return lanStreamEnabled ? "0.0.0.0" : "127.0.0.1";
        }

        if (!lanStreamEnabled && !isLoopbackBindHost(host)) {
            throw new IllegalArgumentException("Non-loopback H.264 stream bind_host requires lan_stream_enabled=true.");
        }
        return host;
    }

    private static String normalizeAdvertisedHost(String requestedHost, String bindHost) {
        String host = requestedHost != null ? requestedHost.trim() : "";
        return host.length() > 0 ? host : bindHost;
    }

    private static boolean isLoopbackBindHost(String host) {
        if (host == null) {
            return false;
        }
        String normalized = host.trim().toLowerCase();
        return "127.0.0.1".equals(normalized) ||
            "localhost".equals(normalized) ||
            "::1".equals(normalized);
    }

    private static boolean isSyntheticSource(JSONObject params) {
        String mode = params != null ? params.optString("source_mode", "") : "";
        if (mode == null || mode.trim().length() == 0) {
            mode = params != null ? params.optString("source_kind", "") : "";
        }
        if (mode == null) {
            return false;
        }
        String normalized = mode.trim().toLowerCase().replace('-', '_');
        return "synthetic".equals(normalized) ||
            SOURCE_MODE_SYNTHETIC_SURFACE.equals(normalized) ||
            "synthetic_surface".equals(normalized) ||
            "diagnostic".equals(normalized) ||
            "diagnostic_surface".equals(normalized);
    }

    private static String normalizeSyntheticPattern(String value) {
        if (value == null || value.trim().length() == 0) {
            return DEFAULT_SYNTHETIC_PATTERN;
        }
        String normalized = value.trim().toLowerCase().replace('_', '-');
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

    private static int syntheticFrameLimit(int captureMs, int maxPackets, int frameRateHz) {
        int timeFrames = captureMs > 0
            ? Math.max(1, (int) Math.ceil(captureMs * frameRateHz / 1000.0))
            : MAX_SYNTHETIC_FRAME_COUNT;
        int packetFrames = maxPackets > 0 ? Math.max(maxPackets + 4, timeFrames) : timeFrames;
        return clamp(packetFrames, 1, MAX_SYNTHETIC_FRAME_COUNT);
    }

    private static void sleepUntilSyntheticFrameCadence(long frameStartElapsedNs, int frameRateHz) throws InterruptedException {
        long frameIntervalNs = 1_000_000_000L / Math.max(1, frameRateHz);
        long targetElapsedNs = frameStartElapsedNs + frameIntervalNs;
        long remainingNs = targetElapsedNs - SystemClock.elapsedRealtimeNanos();
        if (remainingNs > 0L) {
            Thread.sleep(Math.min(50L, Math.max(1L, remainingNs / 1_000_000L)));
        }
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static void joinWriterThread(Thread writerThread, Socket client) {
        if (writerThread == null) {
            return;
        }
        joinQuietly(writerThread, WRITER_JOIN_TIMEOUT_MS);
        if (writerThread.isAlive()) {
            closeQuietly(client);
            joinQuietly(writerThread, WRITER_JOIN_TIMEOUT_MS);
        }
    }

    private static void joinQuietly(Thread thread, long timeoutMs) {
        if (thread == null) {
            return;
        }
        try {
            thread.join(timeoutMs);
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
        }
    }

    private static void closeQuietly(CameraCaptureSession session) {
        if (session != null) {
            try {
                session.close();
            } catch (Exception ignored) {
            }
        }
    }

    private static void closeQuietly(CameraDevice device) {
        if (device != null) {
            try {
                device.close();
            } catch (Exception ignored) {
            }
        }
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

    private static String normalizeSessionId(String requested, String prefix) {
        if (requested != null && requested.trim().length() > 0) {
            return requested.trim();
        }

        return prefix + System.currentTimeMillis();
    }

    private static final class CameraSelection {
        final String cameraId;
        final Size size;
        final long score;
        final CameraCharacteristics characteristics;
        final Range<Integer> fpsRange;
        final long streamMinFrameDurationNs;
        final String selectionReason;

        CameraSelection(
            String cameraId,
            Size size,
            long score,
            CameraCharacteristics characteristics,
            Range<Integer> fpsRange,
            long streamMinFrameDurationNs,
            String selectionReason) {
            this.cameraId = cameraId;
            this.size = size;
            this.score = score;
            this.characteristics = characteristics;
            this.fpsRange = fpsRange;
            this.streamMinFrameDurationNs = streamMinFrameDurationNs;
            this.selectionReason = selectionReason != null ? selectionReason : "";
        }
    }

    private static final class EncoderSelection {
        final String codecName;
        final boolean hardwareAccelerated;
        final boolean softwareOnly;
        final boolean sizeSupported;
        final boolean sizeAndRateSupported;
        final boolean bitrateSupported;
        final int widthAlignment;
        final int heightAlignment;
        final int bitrateLower;
        final int bitrateUpper;
        final boolean cbrSupported;
        final boolean cbrFdSupported;
        final boolean vbrSupported;
        final long score;

        EncoderSelection(
            String codecName,
            boolean hardwareAccelerated,
            boolean softwareOnly,
            boolean sizeSupported,
            boolean sizeAndRateSupported,
            boolean bitrateSupported,
            int widthAlignment,
            int heightAlignment,
            int bitrateLower,
            int bitrateUpper,
            boolean cbrSupported,
            boolean cbrFdSupported,
            boolean vbrSupported,
            long score) {
            this.codecName = codecName != null ? codecName : "";
            this.hardwareAccelerated = hardwareAccelerated;
            this.softwareOnly = softwareOnly;
            this.sizeSupported = sizeSupported;
            this.sizeAndRateSupported = sizeAndRateSupported;
            this.bitrateSupported = bitrateSupported;
            this.widthAlignment = widthAlignment;
            this.heightAlignment = heightAlignment;
            this.bitrateLower = bitrateLower;
            this.bitrateUpper = bitrateUpper;
            this.cbrSupported = cbrSupported;
            this.cbrFdSupported = cbrFdSupported;
            this.vbrSupported = vbrSupported;
            this.score = score;
        }
    }

    static final class EncoderMetadata {
        String encoderName = "";
        String encoderSelectionSource = "";
        String encoderSelectedName = "";
        boolean encoderHardwareAccelerated;
        boolean encoderSoftwareOnly;
        boolean encoderSizeSupported;
        boolean encoderSizeAndRateSupported;
        boolean encoderBitrateSupported;
        int encoderWidthAlignment;
        int encoderHeightAlignment;
        int encoderBitrateLower;
        int encoderBitrateUpper;
        boolean encoderCbrSupported;
        boolean encoderCbrFdSupported;
        boolean encoderVbrSupported;
        String encoderSelectionFallbackReason = "";
        String bitrateModeRequested = "default";
        String bitrateModeApplied = "default";
        String outputBitrateMode = "default";
        String bitrateModeFallbackReason = "";
        String outputMime = "";
        String sensorTimestampSource = "";
        int outputWidth;
        int outputHeight;
        int outputFormatChangeCount;
        int profile = -1;
        int level = -1;
        int colorStandard = -1;
        int colorRange = -1;
        int colorTransfer = -1;
        int appliedLatencyFrames = -1;
        int appliedOutputReorderDepth = -1;
        byte[] csdSps;
        byte[] csdPps;
        boolean prependHeadersToSyncFramesRequested;
        boolean prependHeadersToSyncFramesApplied;
        boolean optionalLowLatencyHintsRequested;
        boolean optionalLowLatencyHintsApplied;
        String configureFallbackReason = "";
        boolean syncFrameRequestOnStartRequested;
        boolean syncFrameRequestOnStartSucceeded;
        String syncFrameRequestOnStartError = "";
        int cameraCaptureStartedCount;
        long cameraFirstCaptureStartedNs;
        long cameraLastCaptureStartedNs;
        long cameraFirstFrameNumber = -1L;
        long cameraLastFrameNumber = -1L;
        long cameraFirstCaptureCallbackElapsedNs;
        long cameraLastCaptureCallbackElapsedNs;

        void copyCaptureTiming(CaptureTimingTracker tracker) {
            if (tracker == null) {
                return;
            }
            cameraCaptureStartedCount = tracker.startedCount;
            cameraFirstCaptureStartedNs = tracker.firstCaptureStartedNs;
            cameraLastCaptureStartedNs = tracker.lastCaptureStartedNs;
            cameraFirstFrameNumber = tracker.firstFrameNumber;
            cameraLastFrameNumber = tracker.lastFrameNumber;
            cameraFirstCaptureCallbackElapsedNs = tracker.firstCallbackElapsedNs;
            cameraLastCaptureCallbackElapsedNs = tracker.lastCallbackElapsedNs;
        }
    }

    private static final class CaptureTimingTracker extends CameraCaptureSession.CaptureCallback {
        int startedCount;
        long firstCaptureStartedNs;
        long lastCaptureStartedNs;
        long firstFrameNumber = -1L;
        long lastFrameNumber = -1L;
        long firstCallbackElapsedNs;
        long lastCallbackElapsedNs;

        @Override
        public void onCaptureStarted(
            CameraCaptureSession session,
            CaptureRequest request,
            long timestamp,
            long frameNumber) {
            long callbackElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (startedCount == 0) {
                firstCaptureStartedNs = timestamp;
                firstFrameNumber = frameNumber;
                firstCallbackElapsedNs = callbackElapsedNs;
            }
            startedCount++;
            lastCaptureStartedNs = timestamp;
            lastFrameNumber = frameNumber;
            lastCallbackElapsedNs = callbackElapsedNs;
        }
    }

    static final class CaptureResult {
        final String sessionId;
        final String requestedCameraId;
        final String cameraId;
        final Size size;
        final int captureMs;
        final int maxPackets;
        final int bitrateBps;
        final int frameRateHz;
        final long encodeStartElapsedNs;
        final long encodeEndElapsedNs;
        final List<EncodedPacket> packets;
        final EncoderMetadata encoderMetadata;

        CaptureResult(
            String sessionId,
            String requestedCameraId,
            String cameraId,
            Size size,
            int captureMs,
            int maxPackets,
            int bitrateBps,
            int frameRateHz,
            long encodeStartElapsedNs,
            long encodeEndElapsedNs,
            List<EncodedPacket> packets,
            EncoderMetadata encoderMetadata) {
            this.sessionId = sessionId;
            this.requestedCameraId = requestedCameraId;
            this.cameraId = cameraId;
            this.size = size;
            this.captureMs = captureMs;
            this.maxPackets = maxPackets;
            this.bitrateBps = bitrateBps;
            this.frameRateHz = frameRateHz;
            this.encodeStartElapsedNs = encodeStartElapsedNs;
            this.encodeEndElapsedNs = encodeEndElapsedNs;
            this.packets = packets;
            this.encoderMetadata = encoderMetadata;
        }
    }

    static final class EncodedPacket {
        final long ptsUs;
        final int flags;
        final byte[] payload;
        final long encoderOutputElapsedNs;
        final long encoderOutputUnixNs;

        EncodedPacket(
            long ptsUs,
            int flags,
            byte[] payload,
            long encoderOutputElapsedNs,
            long encoderOutputUnixNs) {
            this.ptsUs = ptsUs;
            this.flags = flags;
            this.payload = payload;
            this.encoderOutputElapsedNs = encoderOutputElapsedNs;
            this.encoderOutputUnixNs = encoderOutputUnixNs;
        }

        boolean isCodecConfig() {
            return (flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0;
        }

        boolean isKeyFrame() {
            return (flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0;
        }
    }

    private static final class LivePacketQueue {
        private final int capacity;
        private final ArrayDeque<EncodedPacket> queue = new ArrayDeque<EncodedPacket>();
        private boolean closed;
        private int acceptedPacketCount;
        private int codecConfigAcceptedCount;
        private int enqueuedPacketCount;
        private int maxDepth;
        private int droppedPacketCount;
        private int droppedVideoPacketCount;
        private int droppedNonKeyframePacketCount;
        private int droppedKeyframePacketCount;
        private int droppedCodecConfigPacketCount;
        private int droppedIncomingPacketCount;

        LivePacketQueue(int capacity) {
            this.capacity = Math.max(1, capacity);
        }

        synchronized boolean offer(EncodedPacket packet) {
            if (closed || packet == null) {
                return false;
            }
            if (queue.size() >= capacity) {
                EncodedPacket dropped = removeFirstDropCandidate(false);
                if (dropped == null && (packet.isCodecConfig() || packet.isKeyFrame())) {
                    dropped = removeFirstDropCandidate(true);
                }
                if (dropped == null) {
                    droppedIncomingPacketCount++;
                    trackDropped(packet);
                    return false;
                }
                trackDropped(dropped);
            }
            queue.addLast(packet);
            acceptedPacketCount++;
            enqueuedPacketCount++;
            if (packet.isCodecConfig()) {
                codecConfigAcceptedCount++;
            }
            if (queue.size() > maxDepth) {
                maxDepth = queue.size();
            }
            notifyAll();
            return true;
        }

        synchronized EncodedPacket poll(long timeoutMs) throws InterruptedException {
            if (queue.isEmpty() && !closed) {
                wait(timeoutMs);
            }
            if (queue.isEmpty()) {
                return null;
            }
            return queue.removeFirst();
        }

        synchronized void close() {
            closed = true;
            notifyAll();
        }

        synchronized boolean isClosed() {
            return closed;
        }

        synchronized int acceptedPacketCount() {
            return acceptedPacketCount;
        }

        synchronized int codecConfigAcceptedCount() {
            return codecConfigAcceptedCount;
        }

        synchronized int finalDepth() {
            return queue.size();
        }

        synchronized int capacity() {
            return capacity;
        }

        synchronized int enqueuedPacketCount() {
            return enqueuedPacketCount;
        }

        synchronized int maxDepth() {
            return maxDepth;
        }

        synchronized int droppedPacketCount() {
            return droppedPacketCount;
        }

        synchronized int droppedVideoPacketCount() {
            return droppedVideoPacketCount;
        }

        synchronized int droppedNonKeyframePacketCount() {
            return droppedNonKeyframePacketCount;
        }

        synchronized int droppedKeyframePacketCount() {
            return droppedKeyframePacketCount;
        }

        synchronized int droppedCodecConfigPacketCount() {
            return droppedCodecConfigPacketCount;
        }

        synchronized int droppedIncomingPacketCount() {
            return droppedIncomingPacketCount;
        }

        private EncodedPacket removeFirstDropCandidate(boolean allowKeyFrame) {
            int count = queue.size();
            for (int i = 0; i < count; i++) {
                EncodedPacket candidate = queue.removeFirst();
                boolean canDrop = !candidate.isCodecConfig() && (allowKeyFrame || !candidate.isKeyFrame());
                if (canDrop) {
                    return candidate;
                }
                queue.addLast(candidate);
            }
            return null;
        }

        private void trackDropped(EncodedPacket packet) {
            droppedPacketCount++;
            if (packet.isCodecConfig()) {
                droppedCodecConfigPacketCount++;
            } else {
                droppedVideoPacketCount++;
            }
            if (packet.isKeyFrame()) {
                droppedKeyframePacketCount++;
            } else if (!packet.isCodecConfig()) {
                droppedNonKeyframePacketCount++;
            }
        }
    }

    private static final class LiveStreamWriter implements Runnable {
        private final OutputStream output;
        private final Size size;
        private final int maxPackets;
        private final LivePacketQueue queue;
        private final List<EncodedPacket> writtenPackets;
        private final Sink sink;
        private final String sessionId;
        private final String cameraId;
        private final JSONObject streamProjectionMetadata;
        private final boolean liveStream;
        private final boolean syntheticSource;
        private volatile int writtenPacketCount;
        private volatile long writeStartElapsedNs;
        private volatile long writeEndElapsedNs;
        private volatile Exception error;

        LiveStreamWriter(
            OutputStream output,
            Size size,
            int maxPackets,
            LivePacketQueue queue,
            List<EncodedPacket> writtenPackets,
            Sink sink,
            String sessionId,
            String cameraId,
            JSONObject streamProjectionMetadata,
            boolean liveStream,
            boolean syntheticSource) {
            this.output = output;
            this.size = size;
            this.maxPackets = maxPackets;
            this.queue = queue;
            this.writtenPackets = writtenPackets;
            this.sink = sink;
            this.sessionId = sessionId;
            this.cameraId = cameraId;
            this.streamProjectionMetadata = streamProjectionMetadata;
            this.liveStream = liveStream;
            this.syntheticSource = syntheticSource;
        }

        @Override
        public void run() {
            try {
                writeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                writeStreamHeader(output, size, maxPackets, streamProjectionMetadata);
                output.flush();
                while (maxPackets <= 0 || writtenPacketCount < maxPackets) {
                    EncodedPacket packet = queue.poll(WRITER_QUEUE_POLL_MS);
                    if (packet == null) {
                        if (queue.isClosed()) {
                            break;
                        }
                        continue;
                    }
                    writeEncodedPacket(output, packet);
                    output.flush();
                    int index = writtenPacketCount;
                    synchronized (writtenPackets) {
                        writtenPackets.add(packet);
                    }
                    recordSample(sink, sessionId, cameraId, size, index, packet, liveStream, syntheticSource);
                    writtenPacketCount++;
                }
            } catch (Exception ex) {
                error = ex;
                queue.close();
            } finally {
                writeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            }
        }

        boolean hasError() {
            return error != null;
        }

        Exception error() {
            return error;
        }

        int writtenPacketCount() {
            return writtenPacketCount;
        }

        long writeStartElapsedNs() {
            return writeStartElapsedNs;
        }

        long writeEndElapsedNs() {
            return writeEndElapsedNs;
        }

        String errorMessage() {
            Exception ex = error;
            return ex != null ? ex.getClass().getSimpleName() + ": " + safeMessage(ex) : "";
        }
    }

    private static final class LiveStreamResult {
        final List<EncodedPacket> packets;
        final StreamWriteStats writeStats;
        final long encodeEndElapsedNs;

        LiveStreamResult(List<EncodedPacket> packets, StreamWriteStats writeStats, long encodeEndElapsedNs) {
            this.packets = packets;
            this.writeStats = writeStats;
            this.encodeEndElapsedNs = encodeEndElapsedNs;
        }
    }

    private static final class ActiveEncoderControl {
        final String sessionId;
        final String streamId;
        final String cameraId;
        final MediaCodec encoder;
        final long activeSinceElapsedNs;
        int currentBitrateBps;
        int keyframeRequestCount;
        int bitrateChangeCount;
        long lastControlElapsedNs;
        String qualityProfile;

        ActiveEncoderControl(
            String sessionId,
            String streamId,
            String cameraId,
            MediaCodec encoder,
            int currentBitrateBps,
            String qualityProfile) {
            this.sessionId = sessionId != null ? sessionId : "";
            this.streamId = streamId != null ? streamId : "";
            this.cameraId = cameraId != null ? cameraId : "";
            this.encoder = encoder;
            this.currentBitrateBps = currentBitrateBps;
            this.qualityProfile = qualityProfile != null ? qualityProfile : "";
            this.activeSinceElapsedNs = SystemClock.elapsedRealtimeNanos();
            this.lastControlElapsedNs = 0L;
        }
    }

    private static final class StreamWriteStats {
        final long listenStartElapsedNs;
        final long acceptElapsedNs;
        final long writeStartElapsedNs;
        final long writeEndElapsedNs;
        final boolean writerBackpressureIsolated;
        final int writerPacketCount;
        final int writerQueueCapacity;
        final int writerQueueEnqueuedPackets;
        final int writerQueueMaxDepth;
        final int writerQueueFinalDepth;
        final int writerQueueDroppedPackets;
        final int writerQueueDroppedVideoPackets;
        final int writerQueueDroppedNonKeyframePackets;
        final int writerQueueDroppedKeyframePackets;
        final int writerQueueDroppedCodecConfigPackets;
        final int writerQueueDroppedIncomingPackets;
        final String writerError;

        StreamWriteStats(long listenStartElapsedNs, long acceptElapsedNs, long writeStartElapsedNs, long writeEndElapsedNs) {
            this.listenStartElapsedNs = listenStartElapsedNs;
            this.acceptElapsedNs = acceptElapsedNs;
            this.writeStartElapsedNs = writeStartElapsedNs;
            this.writeEndElapsedNs = writeEndElapsedNs;
            this.writerBackpressureIsolated = false;
            this.writerPacketCount = 0;
            this.writerQueueCapacity = 0;
            this.writerQueueEnqueuedPackets = 0;
            this.writerQueueMaxDepth = 0;
            this.writerQueueFinalDepth = 0;
            this.writerQueueDroppedPackets = 0;
            this.writerQueueDroppedVideoPackets = 0;
            this.writerQueueDroppedNonKeyframePackets = 0;
            this.writerQueueDroppedKeyframePackets = 0;
            this.writerQueueDroppedCodecConfigPackets = 0;
            this.writerQueueDroppedIncomingPackets = 0;
            this.writerError = "";
        }

        StreamWriteStats(
            long listenStartElapsedNs,
            long acceptElapsedNs,
            long writeStartElapsedNs,
            long writeEndElapsedNs,
            LivePacketQueue queue,
            LiveStreamWriter writer) {
            this.listenStartElapsedNs = listenStartElapsedNs;
            this.acceptElapsedNs = acceptElapsedNs;
            this.writeStartElapsedNs = writeStartElapsedNs;
            this.writeEndElapsedNs = writeEndElapsedNs;
            this.writerBackpressureIsolated = true;
            this.writerPacketCount = writer != null ? writer.writtenPacketCount() : 0;
            this.writerQueueCapacity = queue != null ? queue.capacity() : 0;
            this.writerQueueEnqueuedPackets = queue != null ? queue.enqueuedPacketCount() : 0;
            this.writerQueueMaxDepth = queue != null ? queue.maxDepth() : 0;
            this.writerQueueFinalDepth = queue != null ? queue.finalDepth() : 0;
            this.writerQueueDroppedPackets = queue != null ? queue.droppedPacketCount() : 0;
            this.writerQueueDroppedVideoPackets = queue != null ? queue.droppedVideoPacketCount() : 0;
            this.writerQueueDroppedNonKeyframePackets = queue != null ? queue.droppedNonKeyframePacketCount() : 0;
            this.writerQueueDroppedKeyframePackets = queue != null ? queue.droppedKeyframePacketCount() : 0;
            this.writerQueueDroppedCodecConfigPackets = queue != null ? queue.droppedCodecConfigPacketCount() : 0;
            this.writerQueueDroppedIncomingPackets = queue != null ? queue.droppedIncomingPacketCount() : 0;
            this.writerError = writer != null ? writer.errorMessage() : "";
        }
    }
}
