package com.example.rustyxr.opengles;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Rect;
import android.graphics.SurfaceTexture;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.Log;
import android.util.Range;
import android.util.Size;
import android.view.Surface;

import org.json.JSONObject;

import java.util.Arrays;
import java.util.Locale;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

public final class DirectCamera2OesProbe {
    private static final String TAG = "RustyXrGles";
    private static final String REPORT_SCHEMA =
        "rusty.xr.quest.direct_camera2_oes_probe.v1";
    private static final String SOURCE = "app.camera2_oes_surface_texture";
    private static final String CAMERA2_POSE_CONVENTION =
        "android-camera2-lens-pose-reference-from-camera";
    private static final String STREAM_RASTER_ORIENTATION_SCHEMA = "rusty.xr.stream_raster_orientation.v1";
    private static final String STREAM_RASTER_ORIENTATION_TOP_LEFT_Y_DOWN = "top-left-origin-y-down";
    private static final String STIMULUS_ORIENTATION_SCHEMA = "rusty.xr.stimulus_orientation.v1";
    private static final String STREAM_CONTENT_GEOMETRY_SCHEMA = "rusty.xr.stream_content_geometry.v1";
    private static final String TARGET_FOOTPRINT_SCHEMA = "rusty.xr.target_screen_footprint.v1";
    private static final String SOURCE_SAMPLING_MODE_SCHEMA = "rusty.xr.source_sampling_mode.v1";
    private static final String SOURCE_SAMPLING_TARGET_LOCAL_RASTER = "target-local-raster";
    private static final String SOURCE_SAMPLING_SCREEN_TO_CAMERA_HOMOGRAPHY = "screen-to-camera-homography";
    private static final String CONTENT_MAPPING_CAMERA_FULL_FRAME = "map-camera-frame-to-full-frame-projection-surface";
    private static final String PROJECTION_GEOMETRY_PROFILE_FULL_FRAME_DIAGNOSTIC = "full-frame-diagnostic";
    private static final String PROJECTION_GEOMETRY_PROFILE_CAMERA_PROJECTION = "camera-projection";
    private static final double DEFAULT_TARGET_SCREEN_X = 0.03d;
    private static final double DEFAULT_TARGET_SCREEN_Y = 0.14d;
    private static final double DEFAULT_TARGET_SCREEN_WIDTH = 0.94d;
    private static final double DEFAULT_TARGET_SCREEN_HEIGHT = 0.72d;
    private static final int DEFAULT_WIDTH = 1280;
    private static final int DEFAULT_HEIGHT = 1280;
    private static final int DEFAULT_FRAME_RATE_HZ = 50;

    private final AtomicBoolean running = new AtomicBoolean(true);
    private final EyeCamera[] eyes;

    static {
        System.loadLibrary("rusty_xr_quest_gl_openxr_video_stack_native");
    }

    private static native void nativeDirectCamera2OesFrameAvailable(
        int viewIndex,
        long sequence,
        long queuedPtsUs);

    private static native void nativeDirectCamera2OesReport(String reportJson);

    private DirectCamera2OesProbe(EyeCamera left, EyeCamera right) {
        eyes = new EyeCamera[] { left, right };
    }

    public static DirectCamera2OesProbe start(
        Activity activity,
        Surface leftSurface,
        Surface rightSurface,
        SurfaceTexture leftSurfaceTexture,
        SurfaceTexture rightSurfaceTexture,
        int defaultWidth,
        int defaultHeight,
        int defaultFrameRateHz) {
        Config config = Config.fromActivity(activity, defaultWidth, defaultHeight, defaultFrameRateHz);
        leftSurfaceTexture.setDefaultBufferSize(config.width, config.height);
        rightSurfaceTexture.setDefaultBufferSize(config.width, config.height);
        EyeCamera left = new EyeCamera(
            activity,
            0,
            "left",
            config.leftCameraId,
            leftSurface,
            leftSurfaceTexture,
            config.width,
            config.height,
            config.frameRateHz,
            config.projectionGeometryProfile,
            config.sourceSamplingMode);
        EyeCamera right = new EyeCamera(
            activity,
            1,
            "right",
            config.rightCameraId,
            rightSurface,
            rightSurfaceTexture,
            config.width,
            config.height,
            config.frameRateHz,
            config.projectionGeometryProfile,
            config.sourceSamplingMode);
        DirectCamera2OesProbe probe = new DirectCamera2OesProbe(left, right);
        emitReport(baseReport("start", config));
        left.start(probe.running);
        right.start(probe.running);
        return probe;
    }

    public void stop() {
        running.set(false);
        for (EyeCamera eye : eyes) {
            eye.stop();
        }
    }

    private static JSONObject baseReport(String event, Config config) {
        JSONObject report = new JSONObject();
        try {
            report.put("schema", REPORT_SCHEMA);
            report.put("event", event);
            report.put("source", SOURCE);
            report.put("left_camera_id", config.leftCameraId);
            report.put("right_camera_id", config.rightCameraId);
            report.put("width", config.width);
            report.put("height", config.height);
            report.put("requested_frame_rate_hz", config.frameRateHz);
            report.put("projection_geometry_profile", config.projectionGeometryProfile);
            report.put("source_sampling_mode", config.sourceSamplingMode);
        } catch (Exception ignored) {
        }
        return report;
    }

    private static void emitReport(JSONObject report) {
        nativeDirectCamera2OesReport(report.toString());
    }

    private static String stringExtra(Activity activity, String key, String defaultValue) {
        Intent intent = activity != null ? activity.getIntent() : null;
        if (intent == null || !intent.hasExtra(key)) {
            return defaultValue;
        }
        Bundle extras = intent.getExtras();
        if (extras == null || !extras.containsKey(key)) {
            return defaultValue;
        }
        Object value = extras.get(key);
        if (value == null) {
            return defaultValue;
        }
        String text = String.valueOf(value);
        return text.length() > 0 ? text : defaultValue;
    }

    private static int intExtra(Activity activity, String key, int defaultValue) {
        Intent intent = activity != null ? activity.getIntent() : null;
        return intent != null && intent.hasExtra(key) ? intent.getIntExtra(key, defaultValue) : defaultValue;
    }

    private static String normalizeProjectionGeometryProfile(String requested) {
        if (requested == null || requested.trim().isEmpty()) {
            return PROJECTION_GEOMETRY_PROFILE_FULL_FRAME_DIAGNOSTIC;
        }
        String value = requested.trim();
        if (PROJECTION_GEOMETRY_PROFILE_FULL_FRAME_DIAGNOSTIC.equals(value)) {
            return value;
        }
        if (PROJECTION_GEOMETRY_PROFILE_CAMERA_PROJECTION.equals(value)) {
            return value;
        }
        throw new IllegalArgumentException(
            "Unsupported direct Camera2 projection geometry profile: " + requested);
    }

    private static String contentMappingIntentForProjectionGeometryProfile(String profile) {
        if (PROJECTION_GEOMETRY_PROFILE_CAMERA_PROJECTION.equals(profile)) {
            return "map-camera-frame-through-screen-to-camera-homography";
        }
        return CONTENT_MAPPING_CAMERA_FULL_FRAME;
    }

    private static String normalizeSourceSamplingMode(String requested, String projectionGeometryProfile) {
        if (requested == null || requested.trim().isEmpty()) {
            return sourceSamplingModeForProjectionGeometryProfile(projectionGeometryProfile);
        }
        String normalized = requested.trim().toLowerCase().replace('_', '-');
        if ("target-local-raster".equals(normalized) ||
                "target-local".equals(normalized) ||
                "target-raster".equals(normalized) ||
                "local-raster".equals(normalized) ||
                "raster".equals(normalized) ||
                "default".equals(normalized)) {
            return SOURCE_SAMPLING_TARGET_LOCAL_RASTER;
        }
        if ("screen-to-camera-homography".equals(normalized) ||
                "screen-camera-homography".equals(normalized) ||
                "screen-to-source-homography".equals(normalized) ||
                "camera-homography".equals(normalized) ||
                "camera-projection".equals(normalized) ||
                "homography".equals(normalized)) {
            return SOURCE_SAMPLING_SCREEN_TO_CAMERA_HOMOGRAPHY;
        }
        throw new IllegalArgumentException("Unsupported direct Camera2 OES source sampling mode: " + requested);
    }

    private static String sourceSamplingModeForProjectionGeometryProfile(String profile) {
        return PROJECTION_GEOMETRY_PROFILE_CAMERA_PROJECTION.equals(profile)
            ? SOURCE_SAMPLING_SCREEN_TO_CAMERA_HOMOGRAPHY
            : SOURCE_SAMPLING_TARGET_LOCAL_RASTER;
    }

    private static String contentMappingIntentForSourceSamplingMode(String mode) {
        if (SOURCE_SAMPLING_SCREEN_TO_CAMERA_HOMOGRAPHY.equals(mode)) {
            return "map-camera-frame-through-screen-to-camera-homography";
        }
        return CONTENT_MAPPING_CAMERA_FULL_FRAME;
    }

    private static final class Config {
        final String leftCameraId;
        final String rightCameraId;
        final int width;
        final int height;
        final int frameRateHz;
        final String projectionGeometryProfile;
        final String sourceSamplingMode;

        Config(
            String leftCameraId,
            String rightCameraId,
            int width,
            int height,
            int frameRateHz,
            String projectionGeometryProfile,
            String sourceSamplingMode) {
            this.leftCameraId = leftCameraId != null && leftCameraId.length() > 0 ? leftCameraId : "50";
            this.rightCameraId = rightCameraId != null && rightCameraId.length() > 0 ? rightCameraId : "51";
            this.width = Math.max(16, width);
            this.height = Math.max(16, height);
            this.frameRateHz = Math.max(1, Math.min(120, frameRateHz));
            this.projectionGeometryProfile = normalizeProjectionGeometryProfile(projectionGeometryProfile);
            this.sourceSamplingMode =
                normalizeSourceSamplingMode(sourceSamplingMode, this.projectionGeometryProfile);
        }

        static Config fromActivity(Activity activity, int defaultWidth, int defaultHeight, int defaultFrameRateHz) {
            String cameraId = stringExtra(activity, "rustyxr.directCamera2OesCameraId", "");
            String leftCameraId = stringExtra(
                activity,
                "rustyxr.directCamera2OesLeftCameraId",
                stringExtra(activity, "rustyxr.brokerH264LeftCameraId", cameraId));
            String rightCameraId = stringExtra(
                activity,
                "rustyxr.directCamera2OesRightCameraId",
                stringExtra(activity, "rustyxr.brokerH264RightCameraId", ""));
            return new Config(
                leftCameraId,
                rightCameraId,
                intExtra(
                    activity,
                    "rustyxr.directCamera2OesWidth",
                    intExtra(activity, "rustyxr.brokerH264Width", defaultWidth > 0 ? defaultWidth : DEFAULT_WIDTH)),
                intExtra(
                    activity,
                    "rustyxr.directCamera2OesHeight",
                    intExtra(activity, "rustyxr.brokerH264Height", defaultHeight > 0 ? defaultHeight : DEFAULT_HEIGHT)),
                intExtra(
                    activity,
                    "rustyxr.directCamera2OesFrameRateHz",
                    intExtra(
                        activity,
                        "rustyxr.brokerH264FrameRateHz",
                        defaultFrameRateHz > 0 ? defaultFrameRateHz : DEFAULT_FRAME_RATE_HZ)),
                stringExtra(
                    activity,
                    "rustyxr.directCamera2OesProjectionGeometryProfile",
                    stringExtra(activity, "rustyxr.cameraProjectionGeometryProfile", PROJECTION_GEOMETRY_PROFILE_FULL_FRAME_DIAGNOSTIC)),
                stringExtra(
                    activity,
                    "rustyxr.directCamera2OesSourceSamplingMode",
                    stringExtra(activity, "rustyxr.cameraSourceSamplingMode", "")));
        }
    }

    private static final class EyeCamera {
        final Activity activity;
        final int viewIndex;
        final String eye;
        final String cameraId;
        final Surface outputSurface;
        final SurfaceTexture surfaceTexture;
        final int width;
        final int height;
        final int frameRateHz;
        final String projectionGeometryProfile;
        final String sourceSamplingMode;
        final AtomicLong sequence = new AtomicLong(0L);

        HandlerThread thread;
        Handler handler;
        CameraDevice cameraDevice;
        CameraCaptureSession session;

        EyeCamera(
            Activity activity,
            int viewIndex,
            String eye,
            String cameraId,
            Surface outputSurface,
            SurfaceTexture surfaceTexture,
            int width,
            int height,
            int frameRateHz,
            String projectionGeometryProfile,
            String sourceSamplingMode) {
            this.activity = activity;
            this.viewIndex = viewIndex;
            this.eye = eye;
            this.cameraId = cameraId;
            this.outputSurface = outputSurface;
            this.surfaceTexture = surfaceTexture;
            this.width = width;
            this.height = height;
            this.frameRateHz = frameRateHz;
            this.projectionGeometryProfile = projectionGeometryProfile;
            this.sourceSamplingMode = sourceSamplingMode;
        }

        void start(AtomicBoolean running) {
            thread = new HandlerThread("DirectCamera2Oes-" + eye);
            thread.start();
            handler = new Handler(thread.getLooper());
            surfaceTexture.setOnFrameAvailableListener(new SurfaceTexture.OnFrameAvailableListener() {
                @Override
                public void onFrameAvailable(SurfaceTexture texture) {
                    long next = sequence.incrementAndGet();
                    nativeDirectCamera2OesFrameAvailable(viewIndex, next, -1L);
                }
            }, handler);
            handler.post(new Runnable() {
                @Override
                public void run() {
                    open(running);
                }
            });
        }

        void open(AtomicBoolean running) {
            if (!running.get()) {
                return;
            }
            try {
                if (activity.checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
                    emitError("camera_permission_missing", "android.permission.CAMERA not granted");
                    return;
                }
                CameraManager manager = (CameraManager) activity.getSystemService(Context.CAMERA_SERVICE);
                if (manager == null) {
                    emitError("camera_manager_missing", "CameraManager unavailable");
                    return;
                }
                CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
                emitReport(metadataReport("projection_metadata", characteristics, null));
                manager.openCamera(cameraId, new CameraDevice.StateCallback() {
                    @Override
                    public void onOpened(CameraDevice camera) {
                        cameraDevice = camera;
                        configureSession(characteristics, running);
                    }

                    @Override
                    public void onDisconnected(CameraDevice camera) {
                        emitError("camera_disconnected", "Camera2 device disconnected");
                        closeCamera(camera);
                    }

                    @Override
                    public void onError(CameraDevice camera, int error) {
                        emitError("camera_error_" + error, "Camera2 device error " + error);
                        closeCamera(camera);
                    }
                }, handler);
            } catch (Exception error) {
                emitError("camera_open_failed", safeMessage(error));
            }
        }

        void configureSession(CameraCharacteristics characteristics, AtomicBoolean running) {
            if (!running.get() || cameraDevice == null) {
                return;
            }
            try {
                cameraDevice.createCaptureSession(
                    Arrays.asList(outputSurface),
                    new CameraCaptureSession.StateCallback() {
                        @Override
                        public void onConfigured(CameraCaptureSession configuredSession) {
                            session = configuredSession;
                            startRepeating(characteristics, running);
                        }

                        @Override
                        public void onConfigureFailed(CameraCaptureSession failedSession) {
                            emitError("capture_session_configure_failed", "Camera2 session configure failed");
                        }
                    },
                    handler);
            } catch (Exception error) {
                emitError("capture_session_create_failed", safeMessage(error));
            }
        }

        void startRepeating(CameraCharacteristics characteristics, AtomicBoolean running) {
            if (!running.get() || cameraDevice == null || session == null) {
                return;
            }
            try {
                CaptureRequest.Builder request =
                    cameraDevice.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                request.addTarget(outputSurface);
                request.set(CaptureRequest.CONTROL_MODE, CaptureRequest.CONTROL_MODE_AUTO);
                Range<Integer> fpsRange = chooseFpsRange(characteristics, frameRateHz);
                if (fpsRange != null) {
                    request.set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, fpsRange);
                }
                session.setRepeatingRequest(request.build(), null, handler);
                emitReport(metadataReport("session_started", characteristics, fpsRange));
            } catch (Exception error) {
                emitError("capture_repeating_failed", safeMessage(error));
            }
        }

        JSONObject metadataReport(
            String event,
            CameraCharacteristics characteristics,
            Range<Integer> appliedFpsRange) {
            JSONObject report = new JSONObject();
            try {
                report.put("schema", REPORT_SCHEMA);
                report.put("event", event);
                report.put("source", SOURCE);
                report.put("view_index", viewIndex);
                report.put("source_eye", eye);
                report.put("camera_id", cameraId);
                report.put("width", width);
                report.put("height", height);
                report.put("requested_frame_rate_hz", frameRateHz);
                if (appliedFpsRange != null) {
                    report.put("applied_ae_fps_min", appliedFpsRange.getLower());
                    report.put("applied_ae_fps_max", appliedFpsRange.getUpper());
                }
                report.put("header_projection_metadata", projectionMetadata(characteristics));
            } catch (Exception ignored) {
            }
            return report;
        }

        JSONObject projectionMetadata(CameraCharacteristics characteristics) throws Exception {
            JSONObject metadata = new JSONObject();
            metadata.put("schema", "rusty.xr.camera_projection.stream_source_metadata.v1");
            metadata.put("source", SOURCE);
            metadata.put("sourceLabel", "Camera2 direct SurfaceTexture/OES " + eye);
            metadata.put("cameraId", cameraId);
            metadata.put("eye", eye);
            metadata.put("syntheticPattern", "none");
            metadata.put("deliveredWidth", width);
            metadata.put("deliveredHeight", height);
            metadata.put("projectionGeometryProfile", projectionGeometryProfile);
            metadata.put("sourceSamplingModeSchema", SOURCE_SAMPLING_MODE_SCHEMA);
            metadata.put("sourceSamplingMode", sourceSamplingMode);
            metadata.put("rasterOrientationSchema", STREAM_RASTER_ORIENTATION_SCHEMA);
            metadata.put("orientationKind", "camera-frame");
            metadata.put("rasterOrientation", STREAM_RASTER_ORIENTATION_TOP_LEFT_Y_DOWN);
            metadata.put("rasterOrigin", "top-left");
            metadata.put("rasterYAxis", "down");
            metadata.put("uprightMarker", "camera-native-upright");
            metadata.put("orientationMetadataSource", "direct-camera2-oes-characteristics");
            metadata.put("orientationDefault", false);
            metadata.put("stimulusOrientationSchema", STIMULUS_ORIENTATION_SCHEMA);
            metadata.put("stimulusRasterOrientation", STREAM_RASTER_ORIENTATION_TOP_LEFT_Y_DOWN);
            metadata.put("stimulusOrigin", "top-left");
            metadata.put("stimulusYAxis", "down");
            metadata.put("stimulusUprightMarker", "camera-native-upright");
            metadata.put("stimulusOrientationMetadataSource", "direct-camera2-oes-characteristics");
            metadata.put("stimulusOrientationDefault", false);
            metadata.put("contentGeometrySchema", STREAM_CONTENT_GEOMETRY_SCHEMA);
            metadata.put("contentKind", "camera-frame");
            metadata.put("contentWidth", width);
            metadata.put("contentHeight", height);
            double contentAspectRatio = height > 0 ? (double) width / (double) height : 1.0;
            metadata.put("contentAspectRatio", contentAspectRatio);
            metadata.put("desiredDisplayAspectRatio", contentAspectRatio);
            metadata.put("desiredProjectionAspectRatio", contentAspectRatio);
            metadata.put("contentCoordinateSpace", "normalized-uv");
            metadata.put("contentOrigin", "top-left");
            metadata.put("contentXAxis", "right");
            metadata.put("contentYAxis", "down");
            JSONObject contentUvRect = new JSONObject();
            contentUvRect.put("left", 0.0);
            contentUvRect.put("top", 0.0);
            contentUvRect.put("right", 1.0);
            contentUvRect.put("bottom", 1.0);
            metadata.put("contentUvRect", contentUvRect);
            metadata.put(
                "contentMappingIntent",
                contentMappingIntentForSourceSamplingMode(sourceSamplingMode));
            metadata.put("contentGeometryMetadataSource", "direct-camera2-oes-characteristics");
            metadata.put("contentGeometryDefault", false);
            putDefaultTargetFootprint(metadata, "direct-camera2-oes-characteristics");
            Integer lensFacing = characteristics.get(CameraCharacteristics.LENS_FACING);
            metadata.put("lensFacing", lensFacingLabel(lensFacing));
            metadata.put("lensFacingRank", lensFacingRank(lensFacing));
            Integer sensorOrientation = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);
            if (sensorOrientation != null) {
                metadata.put("sensorOrientationDegrees", sensorOrientation.intValue());
            }
            StreamConfigurationMap map =
                characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
            Size selectedSize = selectClosestSize(map, width, height);
            if (selectedSize != null) {
                metadata.put("selectedWidth", selectedSize.getWidth());
                metadata.put("selectedHeight", selectedSize.getHeight());
            }

            Rect activeArray = characteristics.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE);
            Size sensorPixelArray = characteristics.get(CameraCharacteristics.SENSOR_INFO_PIXEL_ARRAY_SIZE);
            float[] intrinsics = characteristics.get(CameraCharacteristics.LENS_INTRINSIC_CALIBRATION);
            boolean hasIntrinsics = intrinsics != null && intrinsics.length >= 4;
            int intrinsicsWidth = activeArray != null
                ? activeArray.width()
                : (sensorPixelArray != null ? sensorPixelArray.getWidth() : width);
            int intrinsicsHeight = activeArray != null
                ? activeArray.height()
                : (sensorPixelArray != null ? sensorPixelArray.getHeight() : height);
            if (hasIntrinsics && intrinsicsWidth > 0 && intrinsicsHeight > 0) {
                JSONObject values = new JSONObject();
                values.put("fx", intrinsics[0]);
                values.put("fy", intrinsics[1]);
                values.put("cx", intrinsics[2]);
                values.put("cy", intrinsics[3]);
                values.put("skew", intrinsics.length >= 5 ? intrinsics[4] : 0.0f);
                metadata.put("intrinsics", values);
                metadata.put("intrinsicsDomain", domain(
                    activeArray != null ? "activeArray" : (sensorPixelArray != null ? "sensorPixelArray" : "other"),
                    intrinsicsWidth,
                    intrinsicsHeight));
            }
            if (activeArray != null) {
                metadata.put("activeArrayDomain", domain("activeArray", activeArray.width(), activeArray.height()));
            }
            if (sensorPixelArray != null) {
                metadata.put(
                    "sensorPixelDomain",
                    domain("sensorPixelArray", sensorPixelArray.getWidth(), sensorPixelArray.getHeight()));
            }

            float[] translation = characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION);
            float[] rotation = characteristics.get(CameraCharacteristics.LENS_POSE_ROTATION);
            Integer poseReference = characteristics.get(CameraCharacteristics.LENS_POSE_REFERENCE);
            float[] normalizedRotation = normalizeQuaternionOrNull(rotation);
            boolean hasPose = isFiniteArray(translation, 3)
                && normalizedRotation != null
                && isAcceptedLensPoseReference(poseReference);
            metadata.put("missingIntrinsics", !hasIntrinsics);
            metadata.put("missingPose", !hasPose);
            metadata.put("poseSource", hasPose ? "platform" : "missing");
            metadata.put("poseCoordinateConvention", CAMERA2_POSE_CONVENTION);
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
                metadata.put("lensPoseReferenceLabel", lensPoseReferenceLabel(poseReference));
            }
            metadata.put("projectionMetadataReady", hasIntrinsics && hasPose);
            return metadata;
        }

        private static void putDefaultTargetFootprint(JSONObject metadata, String metadataSource) throws Exception {
            JSONObject targetRect = new JSONObject();
            targetRect.put("x", DEFAULT_TARGET_SCREEN_X);
            targetRect.put("y", DEFAULT_TARGET_SCREEN_Y);
            targetRect.put("width", DEFAULT_TARGET_SCREEN_WIDTH);
            targetRect.put("height", DEFAULT_TARGET_SCREEN_HEIGHT);
            metadata.put("targetFootprintSchema", TARGET_FOOTPRINT_SCHEMA);
            metadata.put("targetCoordinateSpace", "display-eye-screen-uv");
            metadata.put("targetScreenUvRect", targetRect);
            metadata.put("targetClipPolicy", "clip-to-visible-eye");
            metadata.put("targetFootprintMetadataSource", metadataSource);
            metadata.put("targetFootprintDefault", false);
        }

        void emitError(String code, String message) {
            JSONObject report = new JSONObject();
            try {
                report.put("schema", REPORT_SCHEMA);
                report.put("event", "error");
                report.put("source", SOURCE);
                report.put("view_index", viewIndex);
                report.put("source_eye", eye);
                report.put("camera_id", cameraId);
                report.put("width", width);
                report.put("height", height);
                report.put("error", code);
                report.put("message", message);
            } catch (Exception ignored) {
            }
            Log.w(TAG, String.format(Locale.US, "Direct Camera2 OES error eye=%s cameraId=%s code=%s message=%s", eye, cameraId, code, message));
            emitReport(report);
        }

        void stop() {
            CameraCaptureSession sessionToClose = session;
            session = null;
            if (sessionToClose != null) {
                try {
                    sessionToClose.stopRepeating();
                } catch (CameraAccessException ignored) {
                } catch (IllegalStateException ignored) {
                }
                try {
                    sessionToClose.close();
                } catch (RuntimeException ignored) {
                }
            }
            closeCamera(cameraDevice);
            cameraDevice = null;
            if (thread != null) {
                thread.quitSafely();
                try {
                    thread.join(1000);
                } catch (InterruptedException error) {
                    Thread.currentThread().interrupt();
                }
                thread = null;
            }
        }
    }

    private static void closeCamera(CameraDevice camera) {
        if (camera != null) {
            try {
                camera.close();
            } catch (RuntimeException ignored) {
            }
        }
    }

    private static Range<Integer> chooseFpsRange(CameraCharacteristics characteristics, int targetHz) {
        Range<Integer>[] ranges = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES);
        if (ranges == null || ranges.length == 0) {
            return null;
        }
        Range<Integer> best = null;
        int bestScore = Integer.MAX_VALUE;
        for (Range<Integer> range : ranges) {
            if (range == null) {
                continue;
            }
            int upper = range.getUpper();
            int lower = range.getLower();
            int score = Math.abs(upper - targetHz) * 1000 + Math.abs(lower - targetHz);
            if (upper >= targetHz && lower <= targetHz) {
                score -= 100000;
            }
            if (score < bestScore) {
                bestScore = score;
                best = range;
            }
        }
        return best;
    }

    private static Size selectClosestSize(StreamConfigurationMap map, int targetWidth, int targetHeight) {
        if (map == null) {
            return null;
        }
        Size[] sizes;
        try {
            sizes = map.getOutputSizes(SurfaceTexture.class);
        } catch (Exception ignored) {
            sizes = null;
        }
        if (sizes == null || sizes.length == 0) {
            try {
                sizes = map.getOutputSizes(Surface.class);
            } catch (Exception ignored) {
                sizes = null;
            }
        }
        if (sizes == null || sizes.length == 0) {
            return null;
        }
        Size best = null;
        long bestScore = Long.MAX_VALUE;
        for (Size size : sizes) {
            long score =
                Math.abs(size.getWidth() - targetWidth) * 10000L +
                Math.abs(size.getHeight() - targetHeight);
            if (score < bestScore) {
                bestScore = score;
                best = size;
            }
        }
        return best;
    }

    private static JSONObject domain(String kind, int width, int height) throws Exception {
        JSONObject domain = new JSONObject();
        domain.put("kind", kind);
        domain.put("width", width);
        domain.put("height", height);
        return domain;
    }

    private static boolean isFiniteArray(float[] values, int minLength) {
        if (values == null || values.length < minLength) {
            return false;
        }
        for (int i = 0; i < minLength; i++) {
            if (!Float.isFinite(values[i])) {
                return false;
            }
        }
        return true;
    }

    private static float[] normalizeQuaternionOrNull(float[] quaternion) {
        if (!isFiniteArray(quaternion, 4)) {
            return null;
        }
        double norm = Math.sqrt(
            quaternion[0] * quaternion[0] +
            quaternion[1] * quaternion[1] +
            quaternion[2] * quaternion[2] +
            quaternion[3] * quaternion[3]);
        if (!Double.isFinite(norm) || norm <= 1.0e-12) {
            return null;
        }
        double invNorm = 1.0 / norm;
        return new float[] {
            (float) (quaternion[0] * invNorm),
            (float) (quaternion[1] * invNorm),
            (float) (quaternion[2] * invNorm),
            (float) (quaternion[3] * invNorm)
        };
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

    private static String lensFacingLabel(Integer facing) {
        if (facing == null) {
            return "unknown";
        }
        int value = facing.intValue();
        if (value == CameraCharacteristics.LENS_FACING_FRONT) {
            return "front";
        }
        if (value == CameraCharacteristics.LENS_FACING_BACK) {
            return "back";
        }
        if (value == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return "external";
        }
        return "other";
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

    private static String safeMessage(Throwable error) {
        String message = error != null ? error.getMessage() : null;
        return message != null && message.length() > 0
            ? message
            : (error != null ? error.getClass().getSimpleName() : "unknown");
    }
}
