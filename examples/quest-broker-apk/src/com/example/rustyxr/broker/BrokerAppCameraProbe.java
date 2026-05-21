package com.example.rustyxr.broker;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.graphics.Rect;
import android.graphics.YuvImage;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureFailure;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.TotalCaptureResult;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.SystemClock;
import android.util.Range;
import android.util.Size;
import android.view.Surface;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.Set;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;

final class BrokerAppCameraProbe {
    private static final String HEADSET_CAMERA_PERMISSION = "horizonos.permission.HEADSET_CAMERA";
    private static final int DEFAULT_MAX_ATTEMPTS = 3;
    private static final int DEFAULT_CAPTURE_TIMEOUT_MS = 1800;
    private static final int DEFAULT_PREFERRED_WIDTH = 640;
    private static final int DEFAULT_PREFERRED_HEIGHT = 480;

    private BrokerAppCameraProbe() {
    }

    static JSONObject run(Context context, JSONObject params) throws Exception {
        long startedElapsed = SystemClock.elapsedRealtime();
        JSONObject probe = new JSONObject();
        probe.put("schema", "rusty.xr.camera_provider.app_camera_probe.v1");
        probe.put("source", "broker_app.camera2");
        probe.put("started_unix_ms", System.currentTimeMillis());
        probe.put("camera_permission_granted", hasPermission(context, Manifest.permission.CAMERA));
        probe.put("headset_camera_permission_granted", hasPermission(context, HEADSET_CAMERA_PERMISSION));
        probe.put("requires_runtime_camera_permission", true);

        JSONArray devices = new JSONArray();
        JSONArray attempts = new JSONArray();
        JSONArray targetCameraIds = new JSONArray();
        probe.put("devices", devices);
        probe.put("attempts", attempts);
        probe.put("target_camera_ids", targetCameraIds);
        probe.put("manager_state", "pending");
        probe.put("camera_id_count", 0);
        probe.put("attempted_count", 0);
        probe.put("open_success_count", 0);
        probe.put("capture_success_count", 0);

        if (context == null) {
            probe.put("manager_state", "missing_context");
            return finishProbe(probe, startedElapsed);
        }
        if (!hasPermission(context, Manifest.permission.CAMERA)) {
            probe.put("manager_state", "camera_permission_missing");
            return finishProbe(probe, startedElapsed);
        }

        CameraManager manager = (CameraManager) context.getSystemService(Context.CAMERA_SERVICE);
        if (manager == null) {
            probe.put("manager_state", "camera_manager_unavailable");
            return finishProbe(probe, startedElapsed);
        }

        String[] cameraIds;
        try {
            cameraIds = manager.getCameraIdList();
        } catch (SecurityException ex) {
            probe.put("manager_state", "camera_id_list_permission_denied");
            probe.put("error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
            return finishProbe(probe, startedElapsed);
        } catch (CameraAccessException ex) {
            probe.put("manager_state", "camera_id_list_access_failed");
            probe.put("error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
            return finishProbe(probe, startedElapsed);
        }

        probe.put("manager_state", "camera_ids_available");
        probe.put("camera_id_count", cameraIds.length);
        for (int i = 0; i < cameraIds.length; i++) {
            JSONObject device = describeCamera(manager, cameraIds[i]);
            devices.put(device);
        }

        String requestedCameraId = params != null ? params.optString("camera_id", "").trim() : "";
        int maxAttempts = clamp(params != null ? params.optInt("max_attempts", DEFAULT_MAX_ATTEMPTS) : DEFAULT_MAX_ATTEMPTS, 1, 8);
        int timeoutMs = clamp(params != null ? params.optInt("capture_timeout_ms", DEFAULT_CAPTURE_TIMEOUT_MS) : DEFAULT_CAPTURE_TIMEOUT_MS, 500, 8000);
        int preferredWidth = clamp(params != null ? params.optInt("preferred_width", DEFAULT_PREFERRED_WIDTH) : DEFAULT_PREFERRED_WIDTH, 1, 4096);
        int preferredHeight = clamp(params != null ? params.optInt("preferred_height", DEFAULT_PREFERRED_HEIGHT) : DEFAULT_PREFERRED_HEIGHT, 1, 4096);
        boolean persistFrame = params != null && params.optBoolean("persist_frame", false);
        String frameOutputDir = params != null ? params.optString("frame_output_dir", "").trim() : "";
        int jpegQuality = clamp(params != null ? params.optInt("jpeg_quality", 95) : 95, 1, 100);
        if (persistFrame) {
            probe.put("persist_frame", true);
            probe.put("frame_output_dir", frameOutputDir.length() > 0 ? frameOutputDir : defaultFrameOutputDir(context));
            probe.put("jpeg_quality", jpegQuality);
        }

        int openSuccess = 0;
        int captureSuccess = 0;
        int persistedFrameCount = 0;
        int attempted = 0;
        for (int i = 0; i < cameraIds.length && attempted < maxAttempts; i++) {
            String cameraId = cameraIds[i];
            if (requestedCameraId.length() > 0 && !requestedCameraId.equals(cameraId)) {
                continue;
            }

            targetCameraIds.put(cameraId);
            JSONObject attempt = attemptCapture(
                context,
                manager,
                cameraId,
                preferredWidth,
                preferredHeight,
                timeoutMs,
                persistFrame,
                frameOutputDir,
                jpegQuality);
            attempts.put(attempt);
            attempted++;
            if (attempt.optBoolean("open_succeeded", false)) {
                openSuccess++;
            }
            if (attempt.optBoolean("capture_succeeded", false)) {
                captureSuccess++;
            }
            if (attempt.has("persisted_frame")) {
                persistedFrameCount++;
            }
        }
        if (requestedCameraId.length() > 0 && attempted == 0) {
            targetCameraIds.put(requestedCameraId);
            JSONObject attempt = new JSONObject();
            attempt.put("camera_id", requestedCameraId);
            attempt.put("open_state", "not_found");
            attempt.put("open_succeeded", false);
            attempt.put("capture_state", "not_attempted");
            attempt.put("capture_succeeded", false);
            attempts.put(attempt);
        }

        probe.put("attempted_count", attempted);
        probe.put("open_success_count", openSuccess);
        probe.put("capture_success_count", captureSuccess);
        if (persistFrame) {
            probe.put("persisted_frame_count", persistedFrameCount);
        }
        return finishProbe(probe, startedElapsed);
    }

    private static JSONObject attemptCapture(
        final Context context,
        final CameraManager manager,
        final String cameraId,
        int preferredWidth,
        int preferredHeight,
        int timeoutMs,
        final boolean persistFrame,
        final String frameOutputDir,
        final int jpegQuality) throws Exception {
        final JSONObject attempt = new JSONObject();
        attempt.put("camera_id", cameraId);
        attempt.put("open_state", "pending");
        attempt.put("open_succeeded", false);
        attempt.put("capture_state", "pending");
        attempt.put("capture_succeeded", false);
        attempt.put("capture_format", "YUV_420_888");
        attempt.put("capture_timeout_ms", timeoutMs);

        CameraCharacteristics characteristics;
        try {
            characteristics = manager.getCameraCharacteristics(cameraId);
        } catch (Exception ex) {
            attempt.put("open_state", "characteristics_failed");
            attempt.put("capture_state", "not_attempted");
            attempt.put("error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
            return attempt;
        }

        Size captureSize = chooseCaptureSize(characteristics, preferredWidth, preferredHeight);
        if (captureSize == null) {
            attempt.put("open_state", "no_yuv_output");
            attempt.put("capture_state", "not_attempted");
            return attempt;
        }
        attempt.put("capture_size", sizeJson(captureSize));

        final CountDownLatch done = new CountDownLatch(1);
        HandlerThread thread = new HandlerThread("RustyXrAppCameraProbe-" + cameraId);
        thread.start();
        Handler handler = new Handler(thread.getLooper());
        final ImageReader reader = ImageReader.newInstance(
            captureSize.getWidth(),
            captureSize.getHeight(),
            ImageFormat.YUV_420_888,
            2);
        final CameraDevice[] deviceRef = new CameraDevice[1];
        final CameraCaptureSession[] sessionRef = new CameraCaptureSession[1];
        final long startedElapsed = SystemClock.elapsedRealtime();

        reader.setOnImageAvailableListener(new ImageReader.OnImageAvailableListener() {
            @Override
            public void onImageAvailable(ImageReader imageReader) {
                Image image = null;
                try {
                    image = imageReader.acquireLatestImage();
                    if (image == null) {
                        return;
                    }
                    synchronized (attempt) {
                        attempt.put("capture_state", "image_available");
                        attempt.put("capture_succeeded", true);
                        attempt.put("captured_width", image.getWidth());
                        attempt.put("captured_height", image.getHeight());
                        attempt.put("captured_format", imageFormatLabel(image.getFormat()));
                        attempt.put("captured_plane_count", image.getPlanes() != null ? image.getPlanes().length : 0);
                        if (persistFrame) {
                            attempt.put(
                                "persisted_frame",
                                persistCapturedYuvImage(context, image, cameraId, frameOutputDir, jpegQuality));
                        }
                    }
                } catch (Exception ex) {
                    setAttemptError(attempt, "image_available_failed", ex);
                } finally {
                    if (image != null) {
                        image.close();
                    }
                    done.countDown();
                }
            }
        }, handler);

        try {
            manager.openCamera(cameraId, new CameraDevice.StateCallback() {
                @Override
                public void onOpened(CameraDevice cameraDevice) {
                    deviceRef[0] = cameraDevice;
                    synchronized (attempt) {
                        try {
                            attempt.put("open_state", "opened");
                            attempt.put("open_succeeded", true);
                        } catch (Exception ignored) {
                        }
                    }
                    startSingleCapture(cameraDevice, reader.getSurface(), handler, attempt, done, sessionRef);
                }

                @Override
                public void onDisconnected(CameraDevice cameraDevice) {
                    deviceRef[0] = cameraDevice;
                    synchronized (attempt) {
                        try {
                            attempt.put("open_state", "disconnected");
                            attempt.put("capture_state", "disconnected");
                        } catch (Exception ignored) {
                        }
                    }
                    done.countDown();
                }

                @Override
                public void onError(CameraDevice cameraDevice, int error) {
                    deviceRef[0] = cameraDevice;
                    synchronized (attempt) {
                        try {
                            attempt.put("open_state", "error");
                            attempt.put("camera_error_code", error);
                            attempt.put("capture_state", "not_attempted");
                        } catch (Exception ignored) {
                        }
                    }
                    done.countDown();
                }
            }, handler);
        } catch (Exception ex) {
            setAttemptError(attempt, "open_call_failed", ex);
            synchronized (attempt) {
                attempt.put("capture_state", "not_attempted");
            }
            done.countDown();
        }

        boolean completed = done.await(timeoutMs, TimeUnit.MILLISECONDS);
        synchronized (attempt) {
            if (!completed) {
                attempt.put("timed_out", true);
                if ("pending".equals(attempt.optString("open_state"))) {
                    attempt.put("open_state", "timeout");
                }
                if ("pending".equals(attempt.optString("capture_state"))) {
                    attempt.put("capture_state", "timeout");
                }
            }
            attempt.put("elapsed_ms", SystemClock.elapsedRealtime() - startedElapsed);
        }

        if (sessionRef[0] != null) {
            sessionRef[0].close();
        }
        if (deviceRef[0] != null) {
            deviceRef[0].close();
        }
        reader.close();
        thread.quitSafely();
        return attempt;
    }

    private static void startSingleCapture(
        final CameraDevice device,
        final Surface surface,
        Handler handler,
        final JSONObject attempt,
        final CountDownLatch done,
        final CameraCaptureSession[] sessionRef) {
        try {
            device.createCaptureSession(
                Arrays.asList(surface),
                new CameraCaptureSession.StateCallback() {
                    @Override
                    public void onConfigured(CameraCaptureSession session) {
                        sessionRef[0] = session;
                        synchronized (attempt) {
                            try {
                                attempt.put("capture_state", "session_configured");
                            } catch (Exception ignored) {
                            }
                        }
                        try {
                            CaptureRequest.Builder builder = device.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                            builder.addTarget(surface);
                            session.capture(builder.build(), new CameraCaptureSession.CaptureCallback() {
                                @Override
                                public void onCaptureCompleted(
                                    CameraCaptureSession session,
                                    CaptureRequest request,
                                    TotalCaptureResult result) {
                                    synchronized (attempt) {
                                        try {
                                            if (!attempt.optBoolean("capture_succeeded", false)) {
                                                attempt.put("capture_state", "capture_completed_waiting_for_image");
                                            }
                                        } catch (Exception ignored) {
                                        }
                                    }
                                }

                                @Override
                                public void onCaptureFailed(
                                    CameraCaptureSession session,
                                    CaptureRequest request,
                                    CaptureFailure failure) {
                                    synchronized (attempt) {
                                        try {
                                            attempt.put("capture_state", "capture_failed");
                                            attempt.put("capture_failure_reason", failure != null ? failure.getReason() : -1);
                                        } catch (Exception ignored) {
                                        }
                                    }
                                    done.countDown();
                                }
                            }, null);
                        } catch (Exception ex) {
                            setAttemptError(attempt, "capture_request_failed", ex);
                            done.countDown();
                        }
                    }

                    @Override
                    public void onConfigureFailed(CameraCaptureSession session) {
                        synchronized (attempt) {
                            try {
                                attempt.put("capture_state", "configure_failed");
                            } catch (Exception ignored) {
                            }
                        }
                        done.countDown();
                    }
                },
                handler);
        } catch (Exception ex) {
            setAttemptError(attempt, "capture_session_failed", ex);
            done.countDown();
        }
    }

    private static JSONObject describeCamera(CameraManager manager, String cameraId) throws Exception {
        JSONObject device = new JSONObject();
        device.put("camera_id", cameraId);
        try {
            CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
            device.put("lens_facing", lensFacingLabel(characteristics.get(CameraCharacteristics.LENS_FACING)));
            device.put("supported_hardware_level", hardwareLevelLabel(characteristics.get(CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL)));
            if (characteristics.get(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES) != null) {
                device.put(
                    "logical_multi_camera",
                    contains(
                        characteristics.get(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES),
                        CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA));
            }

            Set<String> physicalCameraIds = characteristics.getPhysicalCameraIds();
            JSONArray physical = new JSONArray();
            for (String physicalId : physicalCameraIds) {
                physical.put(physicalId);
            }
            device.put("physical_camera_ids", physical);
            putInteger(device, "sensor_orientation_degrees", characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION));
            putInteger(device, "lens_pose_reference", characteristics.get(CameraCharacteristics.LENS_POSE_REFERENCE));
            putFloatArray(device, "lens_pose_rotation_xyzw", characteristics.get(CameraCharacteristics.LENS_POSE_ROTATION));
            putFloatArray(device, "lens_pose_translation_m", characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION));
            putFloatArray(device, "lens_intrinsic_calibration", characteristics.get(CameraCharacteristics.LENS_INTRINSIC_CALIBRATION));
            putAeFpsRanges(device, characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES));

            StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
            JSONArray configurations = new JSONArray();
            if (map != null) {
                appendOutputSizes(configurations, map, ImageFormat.PRIVATE, "PRIVATE");
                appendOutputSizes(configurations, map, ImageFormat.YUV_420_888, "YUV_420_888");
                appendOutputSizes(configurations, map, ImageFormat.JPEG, "BLOB");
            }
            device.put("stream_configurations", configurations);
        } catch (Exception ex) {
            device.put("error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
        }
        return device;
    }

    private static Size chooseCaptureSize(CameraCharacteristics characteristics, int preferredWidth, int preferredHeight) {
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
            long distance = Math.abs((long) size.getWidth() - preferredWidth) +
                Math.abs((long) size.getHeight() - preferredHeight);
            long oversized = area > (long) preferredWidth * (long) preferredHeight ? 0L : 1_000_000L;
            long score = oversized + distance * 10_000L + area;
            if (score < bestScore) {
                bestScore = score;
                best = size;
            }
        }
        return best;
    }

    private static JSONObject persistCapturedYuvImage(
        Context context,
        Image image,
        String cameraId,
        String requestedOutputDir,
        int jpegQuality) throws Exception {
        if (image.getFormat() != ImageFormat.YUV_420_888) {
            throw new IllegalArgumentException("Expected YUV_420_888 image, got format=" + image.getFormat());
        }
        File outputDir = new File(
            requestedOutputDir != null && requestedOutputDir.trim().length() > 0
                ? requestedOutputDir.trim()
                : defaultFrameOutputDir(context));
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
            jpegQuality,
            jpegBytes);
        if (!jpegWritten) {
            throw new IllegalStateException("YuvImage.compressToJpeg returned false");
        }
        writeBytes(jpegFile, jpegBytes.toByteArray());

        JSONObject record = new JSONObject();
        record.put("schema", "rusty.xr.broker_app.camera_yuv_frame_capture.v1");
        record.put("source", "broker_app.camera2");
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
        record.put("jpeg_quality", jpegQuality);
        record.put("planes", imagePlaneMetadataJson(image));
        record.put("metadata_path", metadataFile.getAbsolutePath());
        writeText(metadataFile, record.toString(2) + "\n");
        return record;
    }

    private static String defaultFrameOutputDir(Context context) {
        File external = context != null ? context.getExternalFilesDir("camera-frame-capture") : null;
        if (external != null) {
            return external.getAbsolutePath();
        }
        File internal = context != null ? new File(context.getFilesDir(), "camera-frame-capture") : null;
        return internal != null ? internal.getAbsolutePath() : "/sdcard/Android/data/com.example.rustyxr.broker/files/camera-frame-capture";
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

    private static JSONObject finishProbe(JSONObject probe, long startedElapsed) throws Exception {
        probe.put("duration_ms", SystemClock.elapsedRealtime() - startedElapsed);
        return probe;
    }

    private static boolean hasPermission(Context context, String permission) {
        return context != null && context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED;
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static void setAttemptError(JSONObject attempt, String state, Exception ex) {
        synchronized (attempt) {
            try {
                if ("pending".equals(attempt.optString("open_state"))) {
                    attempt.put("open_state", state);
                }
                attempt.put("capture_state", state);
                attempt.put("error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
            } catch (Exception ignored) {
            }
        }
    }

    private static String safeMessage(Throwable throwable) {
        String message = throwable != null ? throwable.getMessage() : "";
        return message != null ? message : "";
    }

    private static JSONObject sizeJson(Size size) throws Exception {
        JSONObject json = new JSONObject();
        json.put("width", size.getWidth());
        json.put("height", size.getHeight());
        return json;
    }

    private static void appendOutputSizes(JSONArray target, StreamConfigurationMap map, int format, String label) throws Exception {
        Size[] sizes = map.getOutputSizes(format);
        if (sizes == null) {
            return;
        }
        for (int i = 0; i < sizes.length; i++) {
            JSONObject item = sizeJson(sizes[i]);
            item.put("format_name", label);
            item.put("direction", "OUTPUT");
            target.put(item);
        }
    }

    private static void putFloatArray(JSONObject target, String key, float[] values) throws Exception {
        if (values == null) {
            return;
        }
        JSONArray array = new JSONArray();
        for (int i = 0; i < values.length; i++) {
            array.put((double) values[i]);
        }
        target.put(key, array);
    }

    private static void putInteger(JSONObject target, String key, Integer value) throws Exception {
        if (value != null) {
            target.put(key, value.intValue());
        }
    }

    private static void putAeFpsRanges(JSONObject target, Range<Integer>[] ranges) throws Exception {
        JSONArray rows = new JSONArray();
        if (ranges != null) {
            for (int i = 0; i < ranges.length; i++) {
                JSONArray row = new JSONArray();
                row.put(ranges[i].getLower());
                row.put(ranges[i].getUpper());
                rows.put(row);
            }
        }
        target.put("ae_available_target_fps_rows", rows);
    }

    private static boolean contains(int[] values, int target) {
        if (values == null) {
            return false;
        }
        for (int i = 0; i < values.length; i++) {
            if (values[i] == target) {
                return true;
            }
        }
        return false;
    }

    private static String lensFacingLabel(Integer value) {
        if (value == null) {
            return "";
        }
        if (value == CameraCharacteristics.LENS_FACING_FRONT) {
            return "FRONT";
        }
        if (value == CameraCharacteristics.LENS_FACING_BACK) {
            return "BACK";
        }
        if (value == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return "EXTERNAL";
        }
        return "UNKNOWN_" + value;
    }

    private static String hardwareLevelLabel(Integer value) {
        if (value == null) {
            return "";
        }
        if (value == CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_LEGACY) {
            return "LEGACY";
        }
        if (value == CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_LIMITED) {
            return "LIMITED";
        }
        if (value == CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_FULL) {
            return "FULL";
        }
        if (value == CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_3) {
            return "LEVEL_3";
        }
        if (value == CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_EXTERNAL) {
            return "EXTERNAL";
        }
        return "UNKNOWN_" + value;
    }

    private static String imageFormatLabel(int format) {
        if (format == ImageFormat.YUV_420_888) {
            return "YUV_420_888";
        }
        if (format == ImageFormat.PRIVATE) {
            return "PRIVATE";
        }
        if (format == ImageFormat.JPEG) {
            return "BLOB";
        }
        return "FORMAT_" + format;
    }
}
