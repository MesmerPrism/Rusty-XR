package com.example.rustyxr.broker;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
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

final class BrokerAppCameraLumaStreamSession {
    private static final String STREAM_SCHEMA = "rusty.manifold.video.binary_stream.v1";
    private static final String LEGACY_RUSTY_XR_STREAM_SCHEMA = "rusty.xr.video_lab.binary_stream.v1";
    private static final String MAGIC = "RMANVID1";
    private static final String LEGACY_RUSTY_XR_MAGIC = "RXYRVID1";
    private static final int SCHEMA_VERSION = 1;
    private static final int CODEC_RAW_LUMA8 = 2;
    private static final int DEFAULT_PORT = 8878;
    private static final int DEFAULT_HOST_PORT = 18878;
    private static final int DEFAULT_FRAME_COUNT = 2;
    private static final int MAX_FRAME_COUNT = 6;
    private static final int DEFAULT_WIDTH = 720;
    private static final int DEFAULT_HEIGHT = 480;
    private static final int CAPTURE_TIMEOUT_MS = 6000;
    private static final int ACCEPT_TIMEOUT_MS = 15000;

    interface Sink {
        void registerManifest(JSONObject manifest) throws Exception;

        void recordSample(JSONObject sample) throws Exception;

        void recordMetric(JSONObject metric) throws Exception;
    }

    private BrokerAppCameraLumaStreamSession() {
    }

    static JSONObject start(Context context, JSONObject params, Sink sink) throws Exception {
        final Context appContext = context != null ? context.getApplicationContext() : null;
        final String sessionId = "broker-app-camera-luma-" + System.currentTimeMillis();
        final int devicePort = clamp(params != null ? params.optInt("device_port", DEFAULT_PORT) : DEFAULT_PORT, 1, 65535);
        final int hostPort = clamp(params != null ? params.optInt("host_port", DEFAULT_HOST_PORT) : DEFAULT_HOST_PORT, 1, 65535);
        final int frameCount = clamp(params != null ? params.optInt("frame_count", DEFAULT_FRAME_COUNT) : DEFAULT_FRAME_COUNT, 1, MAX_FRAME_COUNT);
        final int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_WIDTH) : DEFAULT_WIDTH, 1, 4096);
        final int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_HEIGHT) : DEFAULT_HEIGHT, 1, 4096);
        final String requestedCameraId = params != null ? params.optString("camera_id", "").trim() : "";

        JSONObject endpoint = new JSONObject();
        endpoint.put("host", "127.0.0.1");
        endpoint.put("device_port", devicePort);
        endpoint.put("host_port", hostPort);
        endpoint.put("framing", STREAM_SCHEMA);
        endpoint.put("legacy_framing", LEGACY_RUSTY_XR_STREAM_SCHEMA);
        endpoint.put("magic", MAGIC);
        endpoint.put("legacy_magic", LEGACY_RUSTY_XR_MAGIC);
        endpoint.put("codec_id", CODEC_RAW_LUMA8);
        endpoint.put("codec", "raw_luma8");

        JSONObject start = new JSONObject();
        start.put("schema", "rusty.manifold.camera_provider.app_camera_luma_stream_start.v1");
        start.put("legacy_schema", "rusty.xr.camera_provider.app_camera_luma_stream_start.v1");
        start.put("session_id", sessionId);
        start.put("stream_id", "broker_app.camera_luma");
        start.put("source", "broker_app_camera2_luma");
        start.put("state", "starting");
        start.put("camera_id", requestedCameraId);
        start.put("frame_count", frameCount);
        start.put("preferred_width", preferredWidth);
        start.put("preferred_height", preferredHeight);
        start.put("binary_endpoint", endpoint);

        Thread thread = new Thread(new Runnable() {
            @Override
            public void run() {
                runSession(
                    appContext,
                    sink,
                    sessionId,
                    requestedCameraId,
                    devicePort,
                    endpoint,
                    frameCount,
                    preferredWidth,
                    preferredHeight);
            }
        }, "ManifoldAppCameraLumaStream");
        thread.start();
        return start;
    }

    private static void runSession(
        Context context,
        Sink sink,
        String sessionId,
        String requestedCameraId,
        int devicePort,
        JSONObject endpoint,
        int frameCount,
        int preferredWidth,
        int preferredHeight) {
        long captureStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long captureEndElapsedNs = captureStartElapsedNs;
        StreamWriteStats writeStats = new StreamWriteStats(0L, 0L, 0L, 0L);
        List<LumaPacket> packets = new ArrayList<LumaPacket>();
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

            registerManifest(sink, sessionId, cameraId, size, frameCount, endpoint);
            captureStartElapsedNs = SystemClock.elapsedRealtimeNanos();
            packets = captureLumaPackets(manager, cameraId, size, frameCount);
            captureEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            for (int i = 0; i < packets.size(); i++) {
                recordSample(sink, sessionId, cameraId, size, i, packets.get(i));
            }
            writeStats = writePackets(devicePort, size, packets);
        } catch (Exception ex) {
            captureEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            lastError = ex.getClass().getSimpleName() + ": " + safeMessage(ex);
        } finally {
            try {
                recordMetric(
                    sink,
                    sessionId,
                    cameraId,
                    size,
                    packets,
                    captureStartElapsedNs,
                    captureEndElapsedNs,
                    writeStats,
                    lastError);
            } catch (Exception ignored) {
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
                best = new CameraSelection(id, size, score);
            }
        }
        if (best == null) {
            throw new IllegalStateException(requestedCameraId != null && requestedCameraId.length() > 0
                ? "Requested camera has no YUV_420_888 output: " + requestedCameraId
                : "No app-visible YUV_420_888 camera source found.");
        }
        return best;
    }

    private static Size chooseSize(CameraCharacteristics characteristics, int preferredWidth, int preferredHeight) {
        StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
        if (map == null) {
            return null;
        }
        Size[] sizes = map.getOutputSizes(ImageFormat.YUV_420_888);
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

    private static List<LumaPacket> captureLumaPackets(
        final CameraManager manager,
        final String cameraId,
        final Size size,
        final int frameCount) throws Exception {
        final List<LumaPacket> packets = new ArrayList<LumaPacket>();
        final CountDownLatch done = new CountDownLatch(1);
        final CountDownLatch opened = new CountDownLatch(1);
        final Exception[] errorRef = new Exception[1];
        HandlerThread thread = new HandlerThread("RustyXrAppCameraLumaCapture");
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        final ImageReader reader = ImageReader.newInstance(size.getWidth(), size.getHeight(), ImageFormat.YUV_420_888, 3);
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];

        reader.setOnImageAvailableListener(new ImageReader.OnImageAvailableListener() {
            @Override
            public void onImageAvailable(ImageReader imageReader) {
                Image image = null;
                try {
                    image = imageReader.acquireLatestImage();
                    if (image == null) {
                        return;
                    }
                    synchronized (packets) {
                        if (packets.size() < frameCount) {
                            packets.add(new LumaPacket(image.getTimestamp() / 1000L, copyLuma(image)));
                        }
                        if (packets.size() >= frameCount) {
                            done.countDown();
                        }
                    }
                } catch (Exception ex) {
                    errorRef[0] = ex;
                    done.countDown();
                } finally {
                    if (image != null) {
                        image.close();
                    }
                }
            }
        }, handler);

        try {
            manager.openCamera(cameraId, new CameraDevice.StateCallback() {
                @Override
                public void onOpened(CameraDevice device) {
                    deviceRef[0] = device;
                    try {
                        Surface surface = reader.getSurface();
                        device.createCaptureSession(
                            Arrays.asList(surface),
                            new CameraCaptureSession.StateCallback() {
                                @Override
                                public void onConfigured(CameraCaptureSession session) {
                                    sessionRef[0] = session;
                                    try {
                                        CaptureRequest.Builder builder = device.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                                        builder.addTarget(surface);
                                        session.setRepeatingRequest(builder.build(), null, handler);
                                        opened.countDown();
                                    } catch (CameraAccessException ex) {
                                        errorRef[0] = ex;
                                        opened.countDown();
                                        done.countDown();
                                    }
                                }

                                @Override
                                public void onConfigureFailed(CameraCaptureSession session) {
                                    errorRef[0] = new IllegalStateException("Camera capture session configure failed.");
                                    opened.countDown();
                                    done.countDown();
                                }
                            },
                            handler);
                    } catch (Exception ex) {
                        errorRef[0] = ex;
                        opened.countDown();
                        done.countDown();
                    }
                }

                @Override
                public void onDisconnected(CameraDevice device) {
                    deviceRef[0] = device;
                    errorRef[0] = new IllegalStateException("Camera disconnected.");
                    opened.countDown();
                    done.countDown();
                }

                @Override
                public void onError(CameraDevice device, int error) {
                    deviceRef[0] = device;
                    errorRef[0] = new IllegalStateException("Camera error " + error);
                    opened.countDown();
                    done.countDown();
                }
            }, handler);

            opened.await(CAPTURE_TIMEOUT_MS, TimeUnit.MILLISECONDS);
            done.await(CAPTURE_TIMEOUT_MS, TimeUnit.MILLISECONDS);
        } finally {
            if (sessionRef[0] != null) {
                sessionRef[0].close();
            }
            if (deviceRef[0] != null) {
                deviceRef[0].close();
            }
            reader.close();
            thread.quitSafely();
        }

        if (errorRef[0] != null) {
            throw errorRef[0];
        }
        if (packets.size() == 0) {
            throw new IllegalStateException("Camera produced no luma frames before timeout.");
        }
        return packets;
    }

    private static byte[] copyLuma(Image image) {
        int width = image.getWidth();
        int height = image.getHeight();
        Image.Plane plane = image.getPlanes()[0];
        ByteBuffer buffer = plane.getBuffer().duplicate();
        int rowStride = plane.getRowStride();
        int pixelStride = plane.getPixelStride();
        byte[] out = new byte[width * height];
        int offset = 0;
        if (pixelStride == 1 && rowStride == width) {
            buffer.position(0);
            buffer.get(out, 0, out.length);
            return out;
        }
        for (int y = 0; y < height; y++) {
            int rowStart = y * rowStride;
            for (int x = 0; x < width; x++) {
                out[offset++] = buffer.get(rowStart + x * pixelStride);
            }
        }
        return out;
    }

    private static StreamWriteStats writePackets(int port, Size size, List<LumaPacket> packets) throws Exception {
        ServerSocket server = new ServerSocket(port, 1, InetAddress.getByName("127.0.0.1"));
        long listenStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        long acceptElapsedNs = 0L;
        long writeStartElapsedNs = 0L;
        long writeEndElapsedNs = 0L;
        try {
            server.setSoTimeout(ACCEPT_TIMEOUT_MS);
            Socket client = server.accept();
            acceptElapsedNs = SystemClock.elapsedRealtimeNanos();
            try {
                client.setTcpNoDelay(true);
                OutputStream output = client.getOutputStream();
                writeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
                writeStream(output, size, packets);
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

    private static void writeStream(OutputStream output, Size size, List<LumaPacket> packets) throws Exception {
        output.write(MAGIC.getBytes(StandardCharsets.US_ASCII));
        writeU32(output, SCHEMA_VERSION);
        writeU32(output, CODEC_RAW_LUMA8);
        writeU32(output, size.getWidth());
        writeU32(output, size.getHeight());
        writeU32(output, packets.size());
        writeU32(output, size.getWidth() * size.getHeight());
        for (int i = 0; i < packets.size(); i++) {
            LumaPacket packet = packets.get(i);
            writeU64(output, packet.ptsUs);
            writeU32(output, 0);
            writeU32(output, packet.payload.length);
            output.write(packet.payload);
        }
    }

    private static void registerManifest(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int frameCount,
        JSONObject endpoint) throws Exception {
        JSONObject manifest = new JSONObject();
        manifest.put("schema", "rusty.manifold.video.encoded_stream_manifest.v1");
        manifest.put("legacy_schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
        manifest.put("stream_id", "broker_app.camera_luma");
        manifest.put("session_id", sessionId);
        manifest.put("source", "broker_app_camera2_luma");
        manifest.put("transport", "metadata_only");
        manifest.put("payload_transport", "adb_forwarded_tcp_binary");
        manifest.put("mime_type", "application/octet-stream");
        manifest.put("codec", "raw_luma8");
        manifest.put("width", size.getWidth());
        manifest.put("height", size.getHeight());
        manifest.put("frame_rate_hz", 0);
        manifest.put("bitrate_bps", 0);
        manifest.put("source_kind", "broker_app_camera2_raw_luma");
        manifest.put("camera_id", cameraId);
        manifest.put("frame_count", frameCount);
        manifest.put("binary_endpoint", endpoint);
        sink.registerManifest(manifest);
    }

    private static void recordSample(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        int index,
        LumaPacket packet) throws Exception {
        JSONObject sample = new JSONObject();
        sample.put("schema", "rusty.manifold.video.encoded_sample_metadata.v1");
        sample.put("legacy_schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
        sample.put("stream_id", "broker_app.camera_luma");
        sample.put("session_id", sessionId);
        sample.put("sequence_id", System.currentTimeMillis() * 1000L + index);
        sample.put("source", "broker_app_camera2_luma");
        sample.put("transport", "metadata_only");
        sample.put("payload_transport", "adb_forwarded_tcp_binary");
        sample.put("mime_type", "application/octet-stream");
        sample.put("codec", "raw_luma8");
        sample.put("frame_format", "raw_luma8");
        sample.put("camera_id", cameraId);
        sample.put("encoded_size_bytes", packet.payload.length);
        sample.put("width", size.getWidth());
        sample.put("height", size.getHeight());
        sample.put("pts_us", packet.ptsUs);
        sample.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
        sample.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        sink.recordSample(sample);
    }

    private static void recordMetric(
        Sink sink,
        String sessionId,
        String cameraId,
        Size size,
        List<LumaPacket> packets,
        long captureStartElapsedNs,
        long captureEndElapsedNs,
        StreamWriteStats writeStats,
        String lastError) throws Exception {
        long payloadBytes = 0L;
        for (int i = 0; i < packets.size(); i++) {
            payloadBytes += packets.get(i).payload.length;
        }
        JSONObject metric = new JSONObject();
        metric.put("schema", "rusty.manifold.video.metric_sample.v1");
        metric.put("legacy_schema", "rusty.xr.video_lab.metric_sample.v1");
        metric.put("stream_id", "broker_app.camera_luma");
        metric.put("source", "broker_app_camera2_luma");
        metric.put("transport", "metadata_only");
        metric.put("payload_transport", "adb_forwarded_tcp_binary");
        metric.put("codec", "raw_luma8");
        metric.put("session_id", sessionId);
        metric.put("camera_id", cameraId != null ? cameraId : "");
        metric.put("sequence_id", System.currentTimeMillis() * 1000L);
        metric.put("source_time_unix_ns", System.currentTimeMillis() * 1_000_000L);
        metric.put("source_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        metric.put("camera_capture_start_elapsed_ns", captureStartElapsedNs);
        metric.put("camera_capture_end_elapsed_ns", captureEndElapsedNs);
        metric.put("camera_capture_duration_ns", Math.max(0L, captureEndElapsedNs - captureStartElapsedNs));
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
        for (int shift = 56; shift >= 0; shift -= 8) {
            output.write((int) ((value >>> shift) & 0xffL));
        }
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static String safeMessage(Throwable throwable) {
        String message = throwable != null ? throwable.getMessage() : "";
        return message != null ? message : "";
    }

    private static final class CameraSelection {
        final String cameraId;
        final Size size;
        final long score;

        CameraSelection(String cameraId, Size size, long score) {
            this.cameraId = cameraId;
            this.size = size;
            this.score = score;
        }
    }

    private static final class LumaPacket {
        final long ptsUs;
        final byte[] payload;

        LumaPacket(long ptsUs, byte[] payload) {
            this.ptsUs = ptsUs;
            this.payload = payload;
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
