package com.example.rustyxr.broker;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.Rect;
import android.graphics.ImageFormat;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaFormat;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import android.util.Size;
import android.view.Surface;

import org.json.JSONObject;

import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

final class BrokerAppCameraH264StreamSession {
    private static final String STREAM_SCHEMA = "rusty.xr.video_lab.binary_stream.v1";
    private static final String MAGIC = "RXYRVID1";
    private static final int SCHEMA_VERSION = 2;
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
    private static final int FRAME_RATE_HZ = 30;
    private static final int OPEN_TIMEOUT_MS = 4000;
    private static final int SESSION_TIMEOUT_MS = 4000;
    private static final int ENCODER_DRAIN_TIMEOUT_US = 10000;
    private static final int BINARY_STREAM_MAX_PACKET_BYTES = 1024 * 1024;

    interface Sink {
        void registerManifest(JSONObject manifest) throws Exception;

        void recordSample(JSONObject sample) throws Exception;

        void recordMetric(JSONObject metric) throws Exception;
    }

    private BrokerAppCameraH264StreamSession() {
    }

    static JSONObject start(Context context, JSONObject params, Sink sink) throws Exception {
        final Context appContext = context != null ? context.getApplicationContext() : null;
        final String sessionId = "broker-app-camera-h264-" + System.currentTimeMillis();
        final int devicePort = clamp(params != null ? params.optInt("device_port", DEFAULT_PORT) : DEFAULT_PORT, 1, 65535);
        final int hostPort = clamp(params != null ? params.optInt("host_port", DEFAULT_HOST_PORT) : DEFAULT_HOST_PORT, 1, 65535);
        final int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_WIDTH) : DEFAULT_WIDTH, 16, 4096);
        final int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_HEIGHT) : DEFAULT_HEIGHT, 16, 4096);
        final boolean liveStream = params != null && params.optBoolean("live_stream", false);
        final int captureMs = clamp(
            params != null ? params.optInt("capture_ms", DEFAULT_CAPTURE_MS) : DEFAULT_CAPTURE_MS,
            100,
            liveStream ? MAX_LIVE_CAPTURE_MS : MAX_CAPTURE_MS);
        final int maxPackets = clamp(
            params != null ? params.optInt("max_packets", DEFAULT_MAX_PACKETS) : DEFAULT_MAX_PACKETS,
            1,
            liveStream ? MAX_LIVE_PACKETS : MAX_PACKETS);
        final int bitrateBps = clamp(params != null ? params.optInt("bitrate_bps", DEFAULT_BITRATE_BPS) : DEFAULT_BITRATE_BPS, 100_000, 20_000_000);
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

        JSONObject start = new JSONObject();
        start.put("schema", "rusty.xr.camera_provider.app_camera_h264_stream_start.v1");
        start.put("session_id", sessionId);
        start.put("stream_id", "broker_app.camera_h264");
        start.put("source", "broker_app_camera2_mediacodec_surface");
        start.put("state", "starting");
        start.put("camera_id", requestedCameraId);
        start.put("preferred_width", preferredWidth);
        start.put("preferred_height", preferredHeight);
        start.put("capture_ms", captureMs);
        start.put("max_packets", maxPackets);
        start.put("bitrate_bps", bitrateBps);
        start.put("live_stream", liveStream);
        start.put("stream_mode", liveStream ? "live_bounded" : "bounded_capture_then_write");
        start.put("binary_endpoint", endpoint);
        try {
            if (appContext != null &&
                    appContext.checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
                CameraManager manager = (CameraManager) appContext.getSystemService(Context.CAMERA_SERVICE);
                if (manager != null) {
                    CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight);
                    start.put("selected_camera_id", selection.cameraId);
                    start.put("selected_width", selection.size.getWidth());
                    start.put("selected_height", selection.size.getHeight());
                    start.put("selection_score", selection.score);
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
                    bitrateBps,
                    liveStream);
            }
        }, "RustyXrAppCameraH264Stream");
        thread.start();
        return start;
    }

    static CaptureResult capturePacketsForProbe(Context context, JSONObject params) throws Exception {
        final Context appContext = context != null ? context.getApplicationContext() : null;
        final String sessionId = "broker-app-camera-h264-decode-" + System.currentTimeMillis();
        final int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_WIDTH) : DEFAULT_WIDTH, 16, 4096);
        final int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_HEIGHT) : DEFAULT_HEIGHT, 16, 4096);
        final int captureMs = clamp(params != null ? params.optInt("capture_ms", DEFAULT_CAPTURE_MS) : DEFAULT_CAPTURE_MS, 100, MAX_CAPTURE_MS);
        final int maxPackets = clamp(params != null ? params.optInt("max_packets", DEFAULT_MAX_PACKETS) : DEFAULT_MAX_PACKETS, 1, MAX_PACKETS);
        final int bitrateBps = clamp(params != null ? params.optInt("bitrate_bps", DEFAULT_BITRATE_BPS) : DEFAULT_BITRATE_BPS, 100_000, 20_000_000);
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

            CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight);
            long encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            List<EncodedPacket> packets = encodeCameraPackets(manager, selection.cameraId, selection.size, captureMs, maxPackets, bitrateBps);
        long encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
        return new CaptureResult(
            sessionId,
            requestedCameraId,
            selection.cameraId,
            selection.size,
            captureMs,
            maxPackets,
            bitrateBps,
            encodeStartElapsedNs,
            encodeEndElapsedNs,
            packets);
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
        int bitrateBps,
        boolean liveStream) {
        long encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long encodeEndElapsedNs = encodeStartElapsedNs;
        StreamWriteStats writeStats = new StreamWriteStats(0L, 0L, 0L, 0L);
        List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        String cameraId = requestedCameraId;
        Size size = null;
        String lastError = "";
        try {
            if (context == null) {
                throw new IllegalStateException("Broker app context is unavailable.");
            }
            if (context.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
                throw new SecurityException("Broker app camera permission is not granted.");
            }

            CameraManager manager = (CameraManager) context.getSystemService(Context.CAMERA_SERVICE);
            if (manager == null) {
                throw new IllegalStateException("CameraManager is unavailable.");
            }
            CameraSelection selection = chooseCamera(manager, requestedCameraId, preferredWidth, preferredHeight);
            cameraId = selection.cameraId;
            size = selection.size;
            registerManifest(sink, sessionId, cameraId, size, captureMs, maxPackets, bitrateBps, liveStream, endpoint);
            encodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            if (liveStream) {
                LiveStreamResult liveResult = streamCameraPacketsLive(
                    manager,
                    cameraId,
                    size,
                    captureMs,
                    maxPackets,
                    bitrateBps,
                    devicePort,
                    bindHost,
                    sink,
                    sessionId);
                packets = liveResult.packets;
                writeStats = liveResult.writeStats;
                encodeEndElapsedNs = liveResult.encodeEndElapsedNs;
            } else {
                packets = encodeCameraPackets(manager, cameraId, size, captureMs, maxPackets, bitrateBps);
                encodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
                for (int i = 0; i < packets.size(); i++) {
                    recordSample(sink, sessionId, cameraId, size, i, packets.get(i), false);
                }
                writeStats = writePackets(devicePort, bindHost, size, packets);
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
                    liveStream,
                    lastError);
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
        final int bitrateBps) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        HandlerThread thread = new HandlerThread("RustyXrAppCameraH264Capture");
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        MediaCodec encoder = MediaCodec.createEncoderByType("video/avc");
        Surface encoderSurface = null;
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        try {
            MediaFormat format = MediaFormat.createVideoFormat("video/avc", size.getWidth(), size.getHeight());
            format.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
            format.setInteger(MediaFormat.KEY_BIT_RATE, bitrateBps);
            format.setInteger(MediaFormat.KEY_FRAME_RATE, FRAME_RATE_HZ);
            format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);
            encoder.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            encoderSurface = encoder.createInputSurface();
            encoder.start();

            deviceRef[0] = openCamera(manager, cameraId, handler);
            sessionRef[0] = configureSession(deviceRef[0], encoderSurface, handler);
            CaptureRequest.Builder builder = createRecordRequest(deviceRef[0]);
            builder.addTarget(encoderSurface);
            sessionRef[0].setRepeatingRequest(builder.build(), null, handler);

            long deadlineElapsedNs = SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L;
            while (SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs && packets.size() < maxPackets) {
                drainEncoder(encoder, packets, false, maxPackets);
                Thread.sleep(10);
            }
            try {
                sessionRef[0].stopRepeating();
            } catch (Exception ignored) {
            }
            encoder.signalEndOfInputStream();
            drainEncoder(encoder, packets, true, maxPackets);
            if (packets.size() == 0) {
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

    private static LiveStreamResult streamCameraPacketsLive(
        final CameraManager manager,
        final String cameraId,
        final Size size,
        final int captureMs,
        final int maxPackets,
        final int bitrateBps,
        final int devicePort,
        final String bindHost,
        final Sink sink,
        final String sessionId) throws Exception {
        final List<EncodedPacket> packets = new ArrayList<EncodedPacket>();
        HandlerThread thread = new HandlerThread("RustyXrAppCameraH264LiveCapture");
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        MediaCodec encoder = MediaCodec.createEncoderByType("video/avc");
        Surface encoderSurface = null;
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        ServerSocket server = null;
        Socket client = null;
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        long encodeEndElapsedNs = listenStartElapsedNs;
        try {
            server = new ServerSocket(devicePort, 1, InetAddress.getByName(bindHost));
            server.setSoTimeout(15000);
            client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            client.setTcpNoDelay(true);
            OutputStream output = client.getOutputStream();
            writeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            writeStreamHeader(output, size, maxPackets);
            output.flush();

            MediaFormat format = MediaFormat.createVideoFormat("video/avc", size.getWidth(), size.getHeight());
            format.setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface);
            format.setInteger(MediaFormat.KEY_BIT_RATE, bitrateBps);
            format.setInteger(MediaFormat.KEY_FRAME_RATE, FRAME_RATE_HZ);
            format.setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1);
            encoder.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE);
            encoderSurface = encoder.createInputSurface();
            encoder.start();

            deviceRef[0] = openCamera(manager, cameraId, handler);
            sessionRef[0] = configureSession(deviceRef[0], encoderSurface, handler);
            CaptureRequest.Builder builder = createRecordRequest(deviceRef[0]);
            builder.addTarget(encoderSurface);
            sessionRef[0].setRepeatingRequest(builder.build(), null, handler);

            long deadlineElapsedNs = SystemClock.elapsedRealtimeNanos() + captureMs * 1_000_000L;
            while (SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs && packets.size() < maxPackets) {
                drainEncoderToStream(encoder, output, packets, false, maxPackets, sink, sessionId, cameraId, size);
                output.flush();
                Thread.sleep(5);
            }
            try {
                sessionRef[0].stopRepeating();
            } catch (Exception ignored) {
            }
            encoder.signalEndOfInputStream();
            drainEncoderToStream(encoder, output, packets, true, maxPackets, sink, sessionId, cameraId, size);
            output.flush();
            writeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            encodeEndElapsedNs = writeEndElapsedNs;
            if (packets.size() == 0) {
                throw new IllegalStateException("MediaCodec produced no live app-camera H.264 packets.");
            }
            return new LiveStreamResult(
                packets,
                new StreamWriteStats(listenStartElapsedNs, acceptElapsedNs, writeStartElapsedNs, writeEndElapsedNs),
                encodeEndElapsedNs);
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

    private static void drainEncoder(
        MediaCodec encoder,
        List<EncodedPacket> packets,
        boolean endOfStream,
        int maxPackets) throws Exception {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        int emptyPolls = 0;
        while (packets.size() < maxPackets) {
            int status = encoder.dequeueOutputBuffer(info, ENCODER_DRAIN_TIMEOUT_US);
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
                packets.add(new EncodedPacket(
                    info.presentationTimeUs,
                    info.flags,
                    payload,
                    SystemClock.elapsedRealtimeNanos(),
                    System.currentTimeMillis() * 1_000_000L));
            }
            boolean reachedEos = (info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0;
            encoder.releaseOutputBuffer(status, false);
            if (reachedEos) {
                break;
            }
        }
    }

    private static void drainEncoderToStream(
        MediaCodec encoder,
        OutputStream output,
        List<EncodedPacket> packets,
        boolean endOfStream,
        int maxPackets,
        Sink sink,
        String sessionId,
        String cameraId,
        Size size) throws Exception {
        MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
        int emptyPolls = 0;
        while (packets.size() < maxPackets) {
            int status = encoder.dequeueOutputBuffer(info, ENCODER_DRAIN_TIMEOUT_US);
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
                EncodedPacket packet = new EncodedPacket(
                    info.presentationTimeUs,
                    info.flags,
                    payload,
                    SystemClock.elapsedRealtimeNanos(),
                    System.currentTimeMillis() * 1_000_000L);
                int index = packets.size();
                packets.add(packet);
                writeEncodedPacket(output, packet);
                recordSample(sink, sessionId, cameraId, size, index, packet, true);
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
        int preferredHeight) throws Exception {
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
                best = new CameraSelection(id, size, score, characteristics);
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

    private static StreamWriteStats writePackets(int devicePort, String bindHost, Size size, List<EncodedPacket> packets) throws Exception {
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
                writeEncodedPacketStream(output, size, packets);
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

    private static void writeEncodedPacketStream(OutputStream output, Size size, List<EncodedPacket> packets) throws Exception {
        writeStreamHeader(output, size, packets.size());
        for (int i = 0; i < packets.size(); i++) {
            writeEncodedPacket(output, packets.get(i));
        }
    }

    private static void writeStreamHeader(OutputStream output, Size size, int packetCount) throws Exception {
        output.write(MAGIC.getBytes(StandardCharsets.US_ASCII));
        writeU32(output, SCHEMA_VERSION);
        writeU32(output, CODEC_H264);
        writeU32(output, size.getWidth());
        writeU32(output, size.getHeight());
        writeU32(output, packetCount);
        writeU32(output, 0);
    }

    private static void writeEncodedPacket(OutputStream output, EncodedPacket packet) throws Exception {
        writeU64(output, packet.ptsUs);
        writeU32(output, packet.flags);
        writeU32(output, packet.payload.length);
        writeU64(output, packet.encoderOutputElapsedNs);
        writeU64(output, packet.encoderOutputUnixNs);
        output.write(packet.payload);
    }

    private static void registerManifest(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int captureMs,
        int maxPackets,
        int bitrateBps,
        boolean liveStream,
        JSONObject endpoint) throws Exception {
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "broker_app.camera_h264");
        manifest.put("session_id", sessionId);
        manifest.put("source", "broker_app_camera2_mediacodec_surface");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "adb_forwarded_tcp_binary");
        manifest.put("mime_type", "video/avc");
        manifest.put("codec", "h264");
        manifest.put("decoder_target", "surface");
        manifest.put("width", size.getWidth());
        manifest.put("height", size.getHeight());
        manifest.put("frame_rate_hz", FRAME_RATE_HZ);
        manifest.put("bitrate_bps", bitrateBps);
        manifest.put("source_kind", "broker_app_camera2_mediacodec_surface");
        manifest.put("camera_id", cameraId);
        manifest.put("capture_ms", captureMs);
        manifest.put("max_packets", maxPackets);
        manifest.put("live_stream", liveStream);
        manifest.put("stream_mode", liveStream ? "live_bounded" : "bounded_capture_then_write");
        manifest.put("binary_schema_version", SCHEMA_VERSION);
        manifest.put("binary_endpoint", endpoint);
        sink.registerManifest(manifest);
    }

    private static void recordSample(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int index,
        EncodedPacket packet,
        boolean liveStream) throws Exception {
        JSONObject sample = new JSONObject();
        sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
        sample.put("stream_id", "broker_app.camera_h264");
        sample.put("session_id", sessionId);
        sample.put("sequence_id", System.currentTimeMillis() * 1000L + index);
        sample.put("source", "broker_app_camera2_mediacodec_surface");
        sample.put("transport", "metadata_only");
        sample.put("payload_transport", "adb_forwarded_tcp_binary");
        sample.put("mime_type", "video/avc");
        sample.put("codec", "h264");
        sample.put("camera_id", cameraId);
        sample.put("encoded_size_bytes", packet.payload.length);
        sample.put("width", size.getWidth());
        sample.put("height", size.getHeight());
        sample.put("key_frame", (packet.flags & MediaCodec.BUFFER_FLAG_KEY_FRAME) != 0);
        sample.put("codec_config", (packet.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0);
        sample.put("pts_us", packet.ptsUs);
        sample.put("dts_us", packet.ptsUs);
        sample.put("source_time_unix_ns", packet.encoderOutputUnixNs);
        sample.put("source_time_elapsed_ns", packet.encoderOutputElapsedNs);
        sample.put("encoder_output_unix_ns", packet.encoderOutputUnixNs);
        sample.put("encoder_output_elapsed_ns", packet.encoderOutputElapsedNs);
        sample.put("stream_mode", liveStream ? "live_bounded" : "bounded_capture_then_write");
        sink.recordSample(sample);
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
        boolean liveStream,
        String lastError) throws Exception {
        long payloadBytes = 0L;
        for (int i = 0; i < packets.size(); i++) {
            payloadBytes += packets.get(i).payload.length;
        }
        JSONObject metric = new JSONObject();
        metric.put("schema", "rusty.xr.video_lab.metric_sample.v1");
        metric.put("stream_id", "broker_app.camera_h264");
        metric.put("source", "broker_app_camera2_mediacodec_surface");
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
        metric.put("stream_mode", liveStream ? "live_bounded" : "bounded_capture_then_write");
        metric.put("binary_listen_start_elapsed_ns", writeStats.listenStartElapsedNs);
        metric.put("binary_accept_elapsed_ns", writeStats.acceptElapsedNs);
        metric.put("binary_write_start_elapsed_ns", writeStats.writeStartElapsedNs);
        metric.put("binary_write_end_elapsed_ns", writeStats.writeEndElapsedNs);
        metric.put("binary_write_duration_ns", Math.max(0L, writeStats.writeEndElapsedNs - writeStats.writeStartElapsedNs));
        metric.put("packet_count", packets.size());
        metric.put("payload_size_bytes", payloadBytes);
        metric.put("dropped_frames", 0);
        metric.put("stale_frames", 0);
        metric.put("queue_depth", 0);
        metric.put("width", size != null ? size.getWidth() : 0);
        metric.put("height", size != null ? size.getHeight() : 0);
        if (lastError != null && lastError.length() > 0) {
            metric.put("last_error", lastError);
        }
        sink.recordMetric(metric);
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

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
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

    private static final class CameraSelection {
        final String cameraId;
        final Size size;
        final long score;
        final CameraCharacteristics characteristics;

        CameraSelection(String cameraId, Size size, long score, CameraCharacteristics characteristics) {
            this.cameraId = cameraId;
            this.size = size;
            this.score = score;
            this.characteristics = characteristics;
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
        final long encodeStartElapsedNs;
        final long encodeEndElapsedNs;
        final List<EncodedPacket> packets;

        CaptureResult(
            String sessionId,
            String requestedCameraId,
            String cameraId,
            Size size,
            int captureMs,
            int maxPackets,
            int bitrateBps,
            long encodeStartElapsedNs,
            long encodeEndElapsedNs,
            List<EncodedPacket> packets) {
            this.sessionId = sessionId;
            this.requestedCameraId = requestedCameraId;
            this.cameraId = cameraId;
            this.size = size;
            this.captureMs = captureMs;
            this.maxPackets = maxPackets;
            this.bitrateBps = bitrateBps;
            this.encodeStartElapsedNs = encodeStartElapsedNs;
            this.encodeEndElapsedNs = encodeEndElapsedNs;
            this.packets = packets;
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

    private static final class StreamWriteStats {
        final long listenStartElapsedNs;
        final long acceptElapsedNs;
        final long writeStartElapsedNs;
        final long writeEndElapsedNs;

        StreamWriteStats(long listenStartElapsedNs, long acceptElapsedNs, long writeStartElapsedNs, long writeEndElapsedNs) {
            this.listenStartElapsedNs = listenStartElapsedNs;
            this.acceptElapsedNs = acceptElapsedNs;
            this.writeStartElapsedNs = writeStartElapsedNs;
            this.writeEndElapsedNs = writeEndElapsedNs;
        }
    }
}
