package com.example.rustyxr.composite;

import android.Manifest;
import android.app.NativeActivity;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.projection.MediaProjectionManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.util.Size;
import android.view.Surface;
import android.view.Window;
import android.view.WindowManager;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

public final class CompositeLayerActivity extends NativeActivity {
    private static final String TAG = "RustyXrComposite";
    private static final String HEADSET_CAMERA_PERMISSION = "horizonos.permission.HEADSET_CAMERA";
    private static final int MEDIA_PROJECTION_REQUEST = 8701;
    private static final int NOTIFICATION_PERMISSION_REQUEST = 8702;
    private static final int CAMERA_PERMISSION_REQUEST = 8704;
    private static final long DEFAULT_MEDIA_PROJECTION_DELAY_MS = 5000;
    private static final long DEFAULT_CAMERA_START_DELAY_MS = 0;
    private static final int DEFAULT_CAMERA_SIZE = 1280;
    private static final int DEFAULT_CAMERA_MAX_DIMENSION = 1920;
    private static final int DEFAULT_CAMERA_CPU_UPLOAD_HZ = 4;
    private static final int DEFAULT_CAMERA_TARGET_FPS = 0;
    private static final int DEFAULT_CAMERA_FPS_MIN = 0;
    private static final int DEFAULT_CAMERA_FPS_MAX = 0;
    private static final int DEFAULT_CAMERA_STEREO_IMAGE_READER_MAX_IMAGES = 8;
    private static final String DEFAULT_CAMERA_ACQUISITION = "java-camera2";
    private static final String DEFAULT_CAMERA_TIER = "cpu-diagnostic-flat-copy";
    private static final String DEFAULT_CAMERA_STEREO_LAYOUT = "mono";
    private static final boolean DEFAULT_CAMERA_ALLOW_CPU_FALLBACK = true;
    private static final float DEFAULT_CAMERA_PROJECTION_FOV_Y_DEGREES = 92.0f;
    private static final float DEFAULT_CAMERA_PREVIEW_FOV_Y_DEGREES = 60.0f;
    private static final float DEFAULT_CAMERA_PREVIEW_OFFSET_Y_METERS = 0.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_SCALE = 1.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_DEPTH_METERS = 1.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_AREA_SCALE_UV = 1.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_AREA_OFFSET_X_UV = 0.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_AREA_OFFSET_Y_UV = 0.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_AREA_RADIUS_X_UV = 0.5f;
    private static final float DEFAULT_CAMERA_PROJECTION_AREA_RADIUS_Y_UV = 0.5f;
    private static final float DEFAULT_CAMERA_PROJECTION_AREA_CORNER_RADIUS_UV = 0.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_AREA_OPACITY = 1.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_BORDER_OPACITY = 1.0f;
    private static final String DEFAULT_PROJECTION_BORDER_POLICY = "solid-red";
    private static final String DEFAULT_CAMERA_PROJECTION_ALPHA_MODE = "fixed";
    private static final float DEFAULT_CAMERA_PROJECTION_ALPHA_SCALE = 1.0f;
    private static final float DEFAULT_CAMERA_PROJECTION_ALPHA_BIAS = 0.0f;
    private static final float DEFAULT_CAMERA_RAW_OVERLAY_OVERSCAN = 1.06f;
    private static final float DEFAULT_CAMERA_FULL_VIEW_OVERLAY_OVERSCAN = 2.10f;
    private static final float DEFAULT_CAMERA_EDGE_FADE = 0.12f;
    private static final String DEFAULT_CAMERA_PROJECTION_MODE = "display-screen-homography";
    private static final String DEFAULT_CAMERA_PIPELINE_PRESET = "manual";
    private static final String DEFAULT_CAMERA_PROJECTION_EFFECT_MODE = "raw-projection";
    private static final String DEFAULT_PROCESSING_LAYER = "raw";
    private static final String DEFAULT_PERIPHERAL_STRETCH_MODE = "edge-stretch";
    private static final float DEFAULT_PERIPHERAL_STRETCH_CORE_SCALE = 1.0f;
    private static final float DEFAULT_PERIPHERAL_STRETCH_EDGE_INSET_UV = 0.015f;
    private static final float DEFAULT_PERIPHERAL_STRETCH_MAX_INSET_UV = 0.14f;
    private static final float DEFAULT_PERIPHERAL_STRETCH_CURVE = 1.6f;
    private static final String DEFAULT_PERIPHERAL_STRETCH_CORNER_MODE = "target-footprint";
    private static final String DEFAULT_PERIPHERAL_STRETCH_DEBUG = "off";
    private static final String DEFAULT_CAMERA_FEED_MODE = "projected-feed";
    private static final String DEFAULT_CAMERA_COLOR_MODE = "external-rgb";
    private static final String DEFAULT_CAMERA_SAMPLER_BINDING_MODE = "combined-immutable-sampler";
    private static final String DEFAULT_CAMERA_IMPORT_IMAGE_LAYOUT = "shader-read-transition";
    private static final int DEFAULT_CAMERA_IMPORT_CACHE_LIMIT = 16;
    private static final String DEFAULT_CAMERA_COLOR_MATRIX = "identity";
    private static final String DEFAULT_CAMERA_COLOR_OFFSET = "zero";
    private static final float DEFAULT_CAMERA_COLOR_CONTRAST = 1.0f;
    private static final float DEFAULT_CAMERA_COLOR_BRIGHTNESS = 0.0f;
    private static final float DEFAULT_CAMERA_COLOR_SATURATION = 1.0f;
    private static final float DEFAULT_CAMERA_BLUR_RADIUS_PX = 2.0f;
    private static final boolean DEFAULT_CAMERA_TEMPORAL_PROJECTION_ENABLED = false;
    private static final String DEFAULT_CAMERA_TEMPORAL_MODE = "off";
    private static final float DEFAULT_CAMERA_TEMPORAL_MAX_PIXELS_PER_FRAME = 18.0f;
    private static final float DEFAULT_CAMERA_TEMPORAL_MAX_ANGULAR_DEGREES_PER_FRAME = 1.25f;
    private static final float DEFAULT_CAMERA_TEMPORAL_MAX_LINEAR_METERS_PER_FRAME = 0.012f;
    private static final float DEFAULT_CAMERA_TEMPORAL_CATCHUP_HALF_LIFE_MS = 50.0f;
    private static final float DEFAULT_CAMERA_TEMPORAL_MAX_VISUAL_LAG_MS = 120.0f;
    private static final boolean DEFAULT_CAMERA_TEMPORAL_STEREO_LOCKSTEP = true;
    private static final String DEFAULT_CAMERA_TEMPORAL_EDGE_MODE = "none";
    private static final String DEFAULT_CAMERA_FRAME_ADOPTION_MODE = "off";
    private static final float DEFAULT_CAMERA_FRAME_ADOPTION_MAX_JUMP_PX = 24.0f;
    private static final float DEFAULT_CAMERA_FRAME_ADOPTION_MAX_HOLD_MS = 80.0f;
    private static final String DEFAULT_CAMERA_TEXTURE_ROTATION = "rotate0";
    private static final boolean DEFAULT_CAMERA_TEXTURE_FLIP_X = false;
    private static final boolean DEFAULT_CAMERA_TEXTURE_FLIP_Y = false;
    private static final boolean DEFAULT_CAMERA_TEXTURE_MIRROR = false;
    private static final String DEFAULT_CAMERA_TEXTURE_TRANSFORM_SOURCE = "default";
    private static final String DEFAULT_CAMERA_TEXTURE_TRANSFORM_REASON = "unspecified";
    private static final String DEFAULT_CAMERA_SOURCE_EYE_MAPPING = "left-right";
    private static final String DEFAULT_CAMERA_ORIENTATION_DIAGNOSTIC_MODE = "off";
    private static final float DEFAULT_XR_RENDER_SCALE = 1.0f;
    private static final int DEFAULT_XR_FIXED_FOVEATION_LEVEL = 0;
    private static final String DEFAULT_XR_COLOR_FORMAT = "rgba8-srgb";
    private static final String DEFAULT_ENVIRONMENT_DEPTH_MODE = "off";
    private static final boolean DEFAULT_ENVIRONMENT_DEPTH_HAND_REMOVAL = false;
    private static final String DEFAULT_HAND_PARTICLE_MODE = "off";
    private static final String DEFAULT_OPENXR_PASSTHROUGH_PROBE = "off";
    private static final String DEFAULT_PASSTHROUGH_STYLE_MODE = "none";
    private static final float DEFAULT_PASSTHROUGH_OPACITY = 1.0f;
    private static final float DEFAULT_PASSTHROUGH_EDGE_R = 0.0f;
    private static final float DEFAULT_PASSTHROUGH_EDGE_G = 0.0f;
    private static final float DEFAULT_PASSTHROUGH_EDGE_B = 0.0f;
    private static final float DEFAULT_PASSTHROUGH_EDGE_A = 0.0f;
    private static final float DEFAULT_PASSTHROUGH_BRIGHTNESS = 0.0f;
    private static final float DEFAULT_PASSTHROUGH_CONTRAST = 1.0f;
    private static final float DEFAULT_PASSTHROUGH_SATURATION = 1.0f;
    private static final float DEFAULT_PASSTHROUGH_COLOR_PHASE = 0.0f;
    private static final float DEFAULT_PASSTHROUGH_COLOR_AMPLITUDE = 0.0f;
    private static final int DEFAULT_PASSTHROUGH_LUT_RESOLUTION = 32;
    private static final float DEFAULT_PASSTHROUGH_LUT_WEIGHT = 1.0f;
    private static final float DEFAULT_PASSTHROUGH_LUT_FLICKER_HZ = 0.0f;
    private static final float DEFAULT_FULL_FIELD_FLICKER_HZ = 0.0f;
    private static final boolean DEFAULT_PROJECTION_LAYER_VISIBLE = true;
    private static final float DEFAULT_XR_DISPLAY_REFRESH_HZ = 72.0f;
    private static final boolean DEFAULT_DIAGNOSTIC_HUD_VISIBLE = false;
    private static final String DEFAULT_DIAGNOSTIC_HUD_COMMAND = "";
    private static final boolean DEFAULT_OSC_ENABLED = false;
    private static final boolean DEFAULT_OSC_OVERLAY_ENABLED = true;
    private static final String DEFAULT_OSC_LISTEN_ADDR = "0.0.0.0:9000";
    private static final int DEFAULT_OSC_MAX_PACKET_BYTES = 8192;
    private static final String DEFAULT_BROKER_HOST = "127.0.0.1";
    private static final int DEFAULT_BROKER_PORT = 8765;
    private static final int DEFAULT_BROKER_H264_STREAM_PORT = 8879;
    private static final int DEFAULT_BROKER_H264_RIGHT_STREAM_PORT = 8880;
    private static final String DEFAULT_BROKER_H264_LEFT_CAMERA_ID = "50";
    private static final String DEFAULT_BROKER_H264_RIGHT_CAMERA_ID = "51";
    private static final int DEFAULT_BROKER_H264_WIDTH = 720;
    private static final int DEFAULT_BROKER_H264_HEIGHT = 480;
    private static final int DEFAULT_BROKER_H264_CAPTURE_MS = 900;
    private static final int DEFAULT_BROKER_H264_MAX_PACKETS = 12;
    private static final int DEFAULT_BROKER_H264_BITRATE_BPS = 1_000_000;
    private static final int DEFAULT_BROKER_H264_FRAME_RATE_HZ = 30;
    private static final int DEFAULT_BROKER_H264_COMMAND_TIMEOUT_MS = 10000;
    private static final int DEFAULT_BROKER_H264_STREAM_TIMEOUT_MS = 20000;
    private static final int DEFAULT_BROKER_H264_DECODE_TIMEOUT_MS = 5000;
    private static final String DEFAULT_BROKER_H264_DECODE_OUTPUT_MODE = "surface-texture";
    private static final String DEFAULT_BROKER_H264_SOURCE_MODE = "broker-camera";
    private static final String DEFAULT_BROKER_H264_SYNTHETIC_PATTERN = "diagnostic-grid";
    private static final String DEFAULT_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE = "head-anchored-virtual-camera";
    private static final boolean DEFAULT_BROKER_H264_LIVE_DECODE = true;
    private static final boolean DEFAULT_BROKER_H264_BYTE_IDENTITY_PROBE = false;
    private static final String DEFAULT_BROKER_H264_STEREO_PAIRING_MODE = "timestamp-nearest";
    private static final int DEFAULT_BROKER_H264_LIVE_PAIR_QUEUE_LIMIT = 8;

    private MediaProjectionManager mediaProjectionManager;
    private BrokerH264ConsumerProbe brokerH264ConsumerProbe;

    static {
        System.loadLibrary("rusty_xr_quest_composite_native");
    }

    private static native String contractJson();
    private static native void nativeActivityEvent(String eventJson);
    private static native void nativeRuntimeConfig(String configJson);
    private static native boolean nativeStartNativeCamera(String configJson);
    private static native void nativeStopNativeCamera();

    @Override
    protected void onCreate(Bundle bundle) {
        Log.i(TAG, "CompositeLayerActivity onCreate before NativeActivity");
        boolean cameraEnabled = shouldStartHeadsetCamera();
        boolean mediaProjectionEnabled = shouldRequestMediaProjection();
        sendRuntimeConfig(cameraEnabled, mediaProjectionEnabled);
        requestWindowFeature(Window.FEATURE_NO_TITLE);
        super.onCreate(bundle);
        Log.i(TAG, "CompositeLayerActivity onCreate after NativeActivity");
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        mediaProjectionManager = (MediaProjectionManager) getSystemService(MEDIA_PROJECTION_SERVICE);
        Log.i(TAG, "Rusty XR composite layer activity created");
        try {
            Log.i(TAG, "Rusty XR composite layer contract: " + contractJson());
        } catch (RuntimeException error) {
            Log.e(TAG, "Could not serialize Rusty XR contract", error);
        }

        if (android.os.Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[] { Manifest.permission.POST_NOTIFICATIONS }, NOTIFICATION_PERMISSION_REQUEST);
        }

        if (cameraEnabled) {
            long cameraStartDelay = cameraStartDelayMs();
            if (cameraStartDelay > 0) {
                Log.i(TAG, "Delaying headset camera start by " + cameraStartDelay + " ms");
                new Handler(Looper.getMainLooper()).postDelayed(new Runnable() {
                    @Override
                    public void run() {
                        requestHeadsetCameraPermissionsOrStart();
                    }
                }, cameraStartDelay);
            } else {
                requestHeadsetCameraPermissionsOrStart();
            }
        } else {
            sendNativeEvent("headsetCameraDisabledByIntent");
        }

        if (shouldStartBrokerH264Consumer()) {
            startBrokerH264ConsumerProbe();
        }

        if (mediaProjectionEnabled) {
            new Handler(Looper.getMainLooper()).postDelayed(new Runnable() {
                @Override
                public void run() {
                    requestMediaProjection();
                }
            }, mediaProjectionDelayMs());
        }
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        boolean cameraEnabled = shouldStartHeadsetCamera();
        boolean mediaProjectionEnabled = shouldRequestMediaProjection();
        sendRuntimeConfig(cameraEnabled, mediaProjectionEnabled);
        sendNativeEvent("runtimeConfigHotloaded");
        if (shouldStartBrokerH264Consumer()) {
            startBrokerH264ConsumerProbe();
        } else {
            stopBrokerH264ConsumerProbe();
        }
        Log.i(TAG, "Rusty XR runtime config hotloaded from new intent");
    }

    private boolean shouldRequestMediaProjection() {
        return booleanExtra("rustyxr.mediaProjection", false);
    }

    private boolean shouldStartHeadsetCamera() {
        return booleanExtra("rustyxr.camera", true);
    }

    private boolean shouldStartBrokerH264Consumer() {
        return booleanExtra("rustyxr.brokerH264Consumer", false);
    }

    private boolean shouldRenderBrokerH264HardwareBuffer() {
        String mode = stringExtra("rustyxr.brokerH264DecodeOutputMode", DEFAULT_BROKER_H264_DECODE_OUTPUT_MODE);
        return shouldStartBrokerH264Consumer() &&
            "hardware-buffer".equals(mode.trim().toLowerCase(java.util.Locale.US).replace('_', '-'));
    }

    private boolean hasExtra(String name) {
        Intent intent = getIntent();
        return intent != null && intent.getExtras() != null && intent.getExtras().containsKey(name);
    }

    private boolean booleanExtra(String name, boolean defaultValue) {
        Intent intent = getIntent();
        if (!hasExtra(name)) {
            return defaultValue;
        }

        Object value = intent.getExtras().get(name);
        if (value instanceof Boolean) {
            return ((Boolean) value).booleanValue();
        }
        if (value instanceof String) {
            return Boolean.parseBoolean((String) value);
        }

        return defaultValue;
    }

    private String stringExtra(String name, String defaultValue) {
        Intent intent = getIntent();
        if (intent == null || intent.getExtras() == null || !intent.getExtras().containsKey(name)) {
            return defaultValue;
        }

        Object value = intent.getExtras().get(name);
        return value != null ? value.toString() : defaultValue;
    }

    private int intExtra(String name, int defaultValue) {
        Intent intent = getIntent();
        if (intent == null || intent.getExtras() == null || !intent.getExtras().containsKey(name)) {
            return defaultValue;
        }

        Object value = intent.getExtras().get(name);
        if (value instanceof Integer) {
            return ((Integer) value).intValue();
        }
        if (value instanceof String) {
            try {
                return Integer.parseInt((String) value);
            } catch (NumberFormatException ignored) {
                return defaultValue;
            }
        }

        return defaultValue;
    }

    private static int clampInt(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private float floatExtra(String name, float defaultValue) {
        Intent intent = getIntent();
        if (intent == null || intent.getExtras() == null || !intent.getExtras().containsKey(name)) {
            return defaultValue;
        }

        Object value = intent.getExtras().get(name);
        if (value instanceof Float) {
            return ((Float) value).floatValue();
        }
        if (value instanceof Double) {
            return ((Double) value).floatValue();
        }
        if (value instanceof Integer) {
            return ((Integer) value).floatValue();
        }
        if (value instanceof String) {
            try {
                return Float.parseFloat((String) value);
            } catch (NumberFormatException ignored) {
                return defaultValue;
            }
        }

        return defaultValue;
    }

    private long longExtra(String name, long defaultValue) {
        Intent intent = getIntent();
        if (intent == null || intent.getExtras() == null || !intent.getExtras().containsKey(name)) {
            return defaultValue;
        }

        Object value = intent.getExtras().get(name);
        if (value instanceof Long) {
            return ((Long) value).longValue();
        }
        if (value instanceof Integer) {
            return ((Integer) value).longValue();
        }
        if (value instanceof String) {
            try {
                return Long.parseLong((String) value);
            } catch (NumberFormatException ignored) {
                return defaultValue;
            }
        }

        return defaultValue;
    }

    private long mediaProjectionDelayMs() {
        long delay = longExtra("rustyxr.mediaProjectionDelayMs", DEFAULT_MEDIA_PROJECTION_DELAY_MS);
        return Math.max(0, delay);
    }

    private long cameraStartDelayMs() {
        long delay = longExtra("rustyxr.cameraStartDelayMs", DEFAULT_CAMERA_START_DELAY_MS);
        return Math.max(0, delay);
    }

    private String cameraTier() {
        return stringExtra("rustyxr.cameraTier", DEFAULT_CAMERA_TIER);
    }

    private String cameraStereoLayout() {
        return stringExtra("rustyxr.cameraStereoLayout", DEFAULT_CAMERA_STEREO_LAYOUT);
    }

    private String cameraAcquisition() {
        return stringExtra("rustyxr.cameraAcquisition", DEFAULT_CAMERA_ACQUISITION);
    }

    private boolean allowCpuFallback() {
        return booleanExtra("rustyxr.cameraAllowCpuFallback", DEFAULT_CAMERA_ALLOW_CPU_FALLBACK);
    }

    private boolean diagnosticHudVisible() {
        if (hasExtra("rustyxr.diagnosticHudVisible")) {
            return booleanExtra("rustyxr.diagnosticHudVisible", DEFAULT_DIAGNOSTIC_HUD_VISIBLE);
        }
        if (hasExtra("rustyxr.diagnosticsHudVisible")) {
            return booleanExtra("rustyxr.diagnosticsHudVisible", DEFAULT_DIAGNOSTIC_HUD_VISIBLE);
        }
        return booleanExtra(
            "rustyxr.oscOverlayEnabled",
            booleanExtra("rustyxr.oscEnabled", DEFAULT_DIAGNOSTIC_HUD_VISIBLE));
    }

    private void sendRuntimeConfig(boolean cameraEnabled, boolean mediaProjectionEnabled) {
        boolean brokerH264HardwareBufferRender = shouldRenderBrokerH264HardwareBuffer();
        boolean rendererCameraEnabled = cameraEnabled || brokerH264HardwareBufferRender;
        String tier = rendererCameraEnabled ? cameraTier() : "synthetic";
        int cpuUploadHz = intExtra("rustyxr.cameraCpuUploadHz", DEFAULT_CAMERA_CPU_UPLOAD_HZ);
        int cameraTargetFps = Math.max(0, intExtra("rustyxr.cameraTargetFps", DEFAULT_CAMERA_TARGET_FPS));
        int cameraFpsMin = Math.max(0, intExtra("rustyxr.cameraFpsMin", DEFAULT_CAMERA_FPS_MIN));
        int cameraFpsMax = Math.max(0, intExtra("rustyxr.cameraFpsMax", DEFAULT_CAMERA_FPS_MAX));
        int stereoImageReaderMaxImages = Math.max(2, intExtra("rustyxr.cameraStereoImageReaderMaxImages", DEFAULT_CAMERA_STEREO_IMAGE_READER_MAX_IMAGES));
        int fixedFoveationLevel = Math.max(0, intExtra("rustyxr.xrFixedFoveationLevel", DEFAULT_XR_FIXED_FOVEATION_LEVEL));
        boolean hudVisible = diagnosticHudVisible();
        StringBuilder builder = new StringBuilder(256);
        builder.append('{');
        appendJsonString(builder, "cameraTier", tier);
        builder.append(',');
        appendJsonString(builder, "cameraAcquisition", cameraAcquisition());
        builder.append(",\"cameraEnabled\":").append(rendererCameraEnabled);
        builder.append(",\"mediaProjectionEnabled\":").append(mediaProjectionEnabled);
        builder.append(",\"allowCpuFallback\":").append(allowCpuFallback());
        builder.append(",\"cpuUploadHz\":").append(Math.max(0, cpuUploadHz));
        builder.append(",\"cameraTargetFps\":").append(cameraTargetFps);
        builder.append(",\"cameraFpsMin\":").append(cameraFpsMin);
        builder.append(",\"cameraFpsMax\":").append(cameraFpsMax);
        builder.append(",\"cameraStereoImageReaderMaxImages\":").append(stereoImageReaderMaxImages);
        builder.append(",\"cameraStartDelayMs\":").append(cameraStartDelayMs());
        builder.append(',');
        appendJsonString(builder, "nativeSourceMode", stringExtra("rustyxr.nativeSourceMode", "auto"));
        builder.append(",\"projectionRuntimeResolutionEnabled\":").append(booleanExtra("rustyxr.projectionRuntimeResolutionEnabled", false));
        builder.append(",\"cameraProjectionFovYDegrees\":").append(floatJson(floatExtra("rustyxr.cameraProjectionFovYDegrees", DEFAULT_CAMERA_PROJECTION_FOV_Y_DEGREES)));
        builder.append(",\"cameraPreviewFovYDegrees\":").append(floatJson(floatExtra("rustyxr.cameraPreviewFovYDegrees", DEFAULT_CAMERA_PREVIEW_FOV_Y_DEGREES)));
        builder.append(",\"cameraPreviewOffsetYMeters\":").append(floatJson(floatExtra("rustyxr.cameraPreviewOffsetYMeters", DEFAULT_CAMERA_PREVIEW_OFFSET_Y_METERS)));
        builder.append(",\"cameraProjectionScale\":").append(floatJson(floatExtra("rustyxr.cameraProjectionScale", DEFAULT_CAMERA_PROJECTION_SCALE)));
        builder.append(",\"projectionDepthMeters\":").append(floatJson(floatExtra("rustyxr.projectionDepthMeters", DEFAULT_CAMERA_PROJECTION_DEPTH_METERS)));
        builder.append(",\"projectionAreaScaleUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaScaleUv", DEFAULT_CAMERA_PROJECTION_AREA_SCALE_UV)));
        float projectionAreaOffsetXUv = floatExtra("rustyxr.projectionAreaOffsetXUv", DEFAULT_CAMERA_PROJECTION_AREA_OFFSET_X_UV);
        float projectionAreaOffsetYUv = floatExtra("rustyxr.projectionAreaOffsetYUv", DEFAULT_CAMERA_PROJECTION_AREA_OFFSET_Y_UV);
        builder.append(",\"projectionAreaOffsetXUv\":").append(floatJson(projectionAreaOffsetXUv));
        builder.append(",\"projectionAreaOffsetYUv\":").append(floatJson(projectionAreaOffsetYUv));
        builder.append(",\"projectionAreaLeftOffsetXUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaLeftOffsetXUv", projectionAreaOffsetXUv)));
        builder.append(",\"projectionAreaLeftOffsetYUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaLeftOffsetYUv", projectionAreaOffsetYUv)));
        builder.append(",\"projectionAreaRightOffsetXUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaRightOffsetXUv", projectionAreaOffsetXUv)));
        builder.append(",\"projectionAreaRightOffsetYUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaRightOffsetYUv", projectionAreaOffsetYUv)));
        builder.append(",\"projectionAreaRadiusXUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaRadiusXUv", DEFAULT_CAMERA_PROJECTION_AREA_RADIUS_X_UV)));
        builder.append(",\"projectionAreaRadiusYUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaRadiusYUv", DEFAULT_CAMERA_PROJECTION_AREA_RADIUS_Y_UV)));
        builder.append(",\"projectionAreaCornerRadiusUv\":").append(floatJson(floatExtra("rustyxr.projectionAreaCornerRadiusUv", DEFAULT_CAMERA_PROJECTION_AREA_CORNER_RADIUS_UV)));
        builder.append(",\"projectionAreaOpacity\":").append(floatJson(floatExtra("rustyxr.projectionAreaOpacity", DEFAULT_CAMERA_PROJECTION_AREA_OPACITY)));
        builder.append(",\"projectionBorderOpacity\":").append(floatJson(floatExtra("rustyxr.projectionBorderOpacity", DEFAULT_CAMERA_PROJECTION_BORDER_OPACITY)));
        builder.append(',');
        appendJsonString(builder, "projectionBorderPolicy", stringExtra("rustyxr.projectionBorderPolicy", DEFAULT_PROJECTION_BORDER_POLICY));
        builder.append(',');
        appendJsonString(builder, "projectionAlphaMode", stringExtra("rustyxr.projectionAlphaMode", DEFAULT_CAMERA_PROJECTION_ALPHA_MODE));
        builder.append(",\"projectionAlphaScale\":").append(floatJson(floatExtra("rustyxr.projectionAlphaScale", DEFAULT_CAMERA_PROJECTION_ALPHA_SCALE)));
        builder.append(",\"projectionAlphaBias\":").append(floatJson(floatExtra("rustyxr.projectionAlphaBias", DEFAULT_CAMERA_PROJECTION_ALPHA_BIAS)));
        builder.append(",\"cameraRawOverlayOverscan\":").append(floatJson(floatExtra("rustyxr.cameraRawOverlayOverscan", DEFAULT_CAMERA_RAW_OVERLAY_OVERSCAN)));
        builder.append(",\"cameraFullViewOverlayOverscan\":").append(floatJson(floatExtra("rustyxr.cameraFullViewOverlayOverscan", DEFAULT_CAMERA_FULL_VIEW_OVERLAY_OVERSCAN)));
        builder.append(",\"cameraEdgeFade\":").append(floatJson(floatExtra("rustyxr.cameraEdgeFade", DEFAULT_CAMERA_EDGE_FADE)));
        builder.append(',');
        appendJsonString(builder, "cameraProjectionMode", stringExtra("rustyxr.cameraProjectionMode", DEFAULT_CAMERA_PROJECTION_MODE));
        builder.append(',');
        appendJsonString(builder, "cameraPipelinePreset", stringExtra("rustyxr.cameraPipelinePreset", DEFAULT_CAMERA_PIPELINE_PRESET));
        builder.append(',');
        appendJsonString(builder, "cameraProjectionEffectMode", stringExtra("rustyxr.cameraProjectionEffectMode", DEFAULT_CAMERA_PROJECTION_EFFECT_MODE));
        builder.append(',');
        appendJsonString(builder, "processingLayer", stringExtra("rustyxr.processingLayer", DEFAULT_PROCESSING_LAYER));
        builder.append(',');
        appendJsonString(builder, "peripheralStretchMode", stringExtra("rustyxr.peripheralStretchMode", DEFAULT_PERIPHERAL_STRETCH_MODE));
        builder.append(",\"peripheralStretchCoreScale\":").append(floatJson(floatExtra("rustyxr.peripheralStretchCoreScale", DEFAULT_PERIPHERAL_STRETCH_CORE_SCALE)));
        builder.append(",\"peripheralStretchEdgeInsetUv\":").append(floatJson(floatExtra("rustyxr.peripheralStretchEdgeInsetUv", DEFAULT_PERIPHERAL_STRETCH_EDGE_INSET_UV)));
        builder.append(",\"peripheralStretchMaxInsetUv\":").append(floatJson(floatExtra("rustyxr.peripheralStretchMaxInsetUv", DEFAULT_PERIPHERAL_STRETCH_MAX_INSET_UV)));
        builder.append(",\"peripheralStretchCurve\":").append(floatJson(floatExtra("rustyxr.peripheralStretchCurve", DEFAULT_PERIPHERAL_STRETCH_CURVE)));
        builder.append(',');
        appendJsonString(builder, "peripheralStretchCornerMode", stringExtra("rustyxr.peripheralStretchCornerMode", DEFAULT_PERIPHERAL_STRETCH_CORNER_MODE));
        builder.append(',');
        appendJsonString(builder, "peripheralStretchDebug", stringExtra("rustyxr.peripheralStretchDebug", DEFAULT_PERIPHERAL_STRETCH_DEBUG));
        builder.append(',');
        appendJsonString(builder, "cameraFeedMode", stringExtra("rustyxr.cameraFeedMode", DEFAULT_CAMERA_FEED_MODE));
        builder.append(',');
        appendJsonString(builder, "cameraColorMode", stringExtra("rustyxr.cameraColorMode", DEFAULT_CAMERA_COLOR_MODE));
        builder.append(',');
        appendJsonString(builder, "cameraSamplerBindingMode", stringExtra("rustyxr.cameraSamplerBindingMode", DEFAULT_CAMERA_SAMPLER_BINDING_MODE));
        builder.append(',');
        appendJsonString(builder, "cameraImportImageLayout", stringExtra("rustyxr.cameraImportImageLayout", DEFAULT_CAMERA_IMPORT_IMAGE_LAYOUT));
        builder.append(",\"cameraImportCacheLimit\":").append(Math.max(2, intExtra("rustyxr.cameraImportCacheLimit", DEFAULT_CAMERA_IMPORT_CACHE_LIMIT)));
        builder.append(',');
        appendJsonString(builder, "cameraColorMatrix", stringExtra("rustyxr.cameraColorMatrix", DEFAULT_CAMERA_COLOR_MATRIX));
        builder.append(',');
        appendJsonString(builder, "cameraColorOffset", stringExtra("rustyxr.cameraColorOffset", DEFAULT_CAMERA_COLOR_OFFSET));
        builder.append(",\"cameraColorContrast\":").append(floatJson(floatExtra("rustyxr.cameraColorContrast", DEFAULT_CAMERA_COLOR_CONTRAST)));
        builder.append(",\"cameraColorBrightness\":").append(floatJson(floatExtra("rustyxr.cameraColorBrightness", DEFAULT_CAMERA_COLOR_BRIGHTNESS)));
        builder.append(",\"cameraColorSaturation\":").append(floatJson(floatExtra("rustyxr.cameraColorSaturation", DEFAULT_CAMERA_COLOR_SATURATION)));
        builder.append(",\"cameraBlurRadiusPx\":").append(floatJson(floatExtra("rustyxr.cameraBlurRadiusPx", DEFAULT_CAMERA_BLUR_RADIUS_PX)));
        builder.append(",\"cameraTemporalProjectionEnabled\":").append(booleanExtra("rustyxr.cameraTemporalProjectionEnabled", DEFAULT_CAMERA_TEMPORAL_PROJECTION_ENABLED));
        builder.append(',');
        appendJsonString(builder, "cameraTemporalMode", stringExtra("rustyxr.cameraTemporalMode", DEFAULT_CAMERA_TEMPORAL_MODE));
        builder.append(",\"cameraTemporalMaxPixelsPerFrame\":").append(floatJson(floatExtra("rustyxr.cameraTemporalMaxPixelsPerFrame", DEFAULT_CAMERA_TEMPORAL_MAX_PIXELS_PER_FRAME)));
        builder.append(",\"cameraTemporalMaxAngularDegreesPerFrame\":").append(floatJson(floatExtra("rustyxr.cameraTemporalMaxAngularDegreesPerFrame", DEFAULT_CAMERA_TEMPORAL_MAX_ANGULAR_DEGREES_PER_FRAME)));
        builder.append(",\"cameraTemporalMaxLinearMetersPerFrame\":").append(floatJson(floatExtra("rustyxr.cameraTemporalMaxLinearMetersPerFrame", DEFAULT_CAMERA_TEMPORAL_MAX_LINEAR_METERS_PER_FRAME)));
        builder.append(",\"cameraTemporalCatchupHalfLifeMs\":").append(floatJson(floatExtra("rustyxr.cameraTemporalCatchupHalfLifeMs", DEFAULT_CAMERA_TEMPORAL_CATCHUP_HALF_LIFE_MS)));
        builder.append(",\"cameraTemporalMaxVisualLagMs\":").append(floatJson(floatExtra("rustyxr.cameraTemporalMaxVisualLagMs", DEFAULT_CAMERA_TEMPORAL_MAX_VISUAL_LAG_MS)));
        builder.append(",\"cameraTemporalStereoLockstep\":").append(booleanExtra("rustyxr.cameraTemporalStereoLockstep", DEFAULT_CAMERA_TEMPORAL_STEREO_LOCKSTEP));
        builder.append(',');
        appendJsonString(builder, "cameraTemporalEdgeMode", stringExtra("rustyxr.cameraTemporalEdgeMode", DEFAULT_CAMERA_TEMPORAL_EDGE_MODE));
        builder.append(',');
        appendJsonString(builder, "cameraFrameAdoptionMode", stringExtra("rustyxr.cameraFrameAdoptionMode", DEFAULT_CAMERA_FRAME_ADOPTION_MODE));
        builder.append(",\"cameraFrameAdoptionMaxJumpPx\":").append(floatJson(floatExtra("rustyxr.cameraFrameAdoptionMaxJumpPx", DEFAULT_CAMERA_FRAME_ADOPTION_MAX_JUMP_PX)));
        builder.append(",\"cameraFrameAdoptionMaxHoldMs\":").append(floatJson(floatExtra("rustyxr.cameraFrameAdoptionMaxHoldMs", DEFAULT_CAMERA_FRAME_ADOPTION_MAX_HOLD_MS)));
        builder.append(',');
        appendJsonString(builder, "cameraTextureRotation", stringExtra("rustyxr.cameraTextureRotation", DEFAULT_CAMERA_TEXTURE_ROTATION));
        builder.append(",\"cameraTextureFlipX\":").append(booleanExtra("rustyxr.cameraTextureFlipX", DEFAULT_CAMERA_TEXTURE_FLIP_X));
        builder.append(",\"cameraTextureFlipY\":").append(booleanExtra("rustyxr.cameraTextureFlipY", DEFAULT_CAMERA_TEXTURE_FLIP_Y));
        builder.append(",\"cameraTextureMirror\":").append(booleanExtra("rustyxr.cameraTextureMirror", DEFAULT_CAMERA_TEXTURE_MIRROR));
        builder.append(',');
        appendJsonString(builder, "cameraTextureTransformSource", stringExtra("rustyxr.cameraTextureTransformSource", DEFAULT_CAMERA_TEXTURE_TRANSFORM_SOURCE));
        builder.append(',');
        appendJsonString(builder, "cameraTextureTransformReason", stringExtra("rustyxr.cameraTextureTransformReason", DEFAULT_CAMERA_TEXTURE_TRANSFORM_REASON));
        builder.append(',');
        appendJsonString(builder, "leftCameraTextureRotation", stringExtra("rustyxr.leftCameraTextureRotation", stringExtra("rustyxr.cameraTextureRotation", DEFAULT_CAMERA_TEXTURE_ROTATION)));
        builder.append(",\"leftCameraTextureFlipX\":").append(booleanExtra("rustyxr.leftCameraTextureFlipX", booleanExtra("rustyxr.cameraTextureFlipX", DEFAULT_CAMERA_TEXTURE_FLIP_X)));
        builder.append(",\"leftCameraTextureFlipY\":").append(booleanExtra("rustyxr.leftCameraTextureFlipY", booleanExtra("rustyxr.cameraTextureFlipY", DEFAULT_CAMERA_TEXTURE_FLIP_Y)));
        builder.append(",\"leftCameraTextureMirror\":").append(booleanExtra("rustyxr.leftCameraTextureMirror", booleanExtra("rustyxr.cameraTextureMirror", DEFAULT_CAMERA_TEXTURE_MIRROR)));
        builder.append(',');
        appendJsonString(builder, "leftCameraTextureTransformSource", stringExtra("rustyxr.leftCameraTextureTransformSource", stringExtra("rustyxr.cameraTextureTransformSource", DEFAULT_CAMERA_TEXTURE_TRANSFORM_SOURCE)));
        builder.append(',');
        appendJsonString(builder, "leftCameraTextureTransformReason", stringExtra("rustyxr.leftCameraTextureTransformReason", stringExtra("rustyxr.cameraTextureTransformReason", DEFAULT_CAMERA_TEXTURE_TRANSFORM_REASON)));
        builder.append(',');
        appendJsonString(builder, "rightCameraTextureRotation", stringExtra("rustyxr.rightCameraTextureRotation", stringExtra("rustyxr.cameraTextureRotation", DEFAULT_CAMERA_TEXTURE_ROTATION)));
        builder.append(",\"rightCameraTextureFlipX\":").append(booleanExtra("rustyxr.rightCameraTextureFlipX", booleanExtra("rustyxr.cameraTextureFlipX", DEFAULT_CAMERA_TEXTURE_FLIP_X)));
        builder.append(",\"rightCameraTextureFlipY\":").append(booleanExtra("rustyxr.rightCameraTextureFlipY", booleanExtra("rustyxr.cameraTextureFlipY", DEFAULT_CAMERA_TEXTURE_FLIP_Y)));
        builder.append(",\"rightCameraTextureMirror\":").append(booleanExtra("rustyxr.rightCameraTextureMirror", booleanExtra("rustyxr.cameraTextureMirror", DEFAULT_CAMERA_TEXTURE_MIRROR)));
        builder.append(',');
        appendJsonString(builder, "rightCameraTextureTransformSource", stringExtra("rustyxr.rightCameraTextureTransformSource", stringExtra("rustyxr.cameraTextureTransformSource", DEFAULT_CAMERA_TEXTURE_TRANSFORM_SOURCE)));
        builder.append(',');
        appendJsonString(builder, "rightCameraTextureTransformReason", stringExtra("rustyxr.rightCameraTextureTransformReason", stringExtra("rustyxr.cameraTextureTransformReason", DEFAULT_CAMERA_TEXTURE_TRANSFORM_REASON)));
        builder.append(',');
        appendJsonString(builder, "cameraSourceEyeMapping", stringExtra("rustyxr.cameraSourceEyeMapping", DEFAULT_CAMERA_SOURCE_EYE_MAPPING));
        builder.append(',');
        appendJsonString(builder, "cameraOrientationDiagnosticMode", stringExtra("rustyxr.cameraOrientationDiagnosticMode", DEFAULT_CAMERA_ORIENTATION_DIAGNOSTIC_MODE));
        builder.append(",\"visualReleaseAccepted\":").append(booleanExtra("rustyxr.visualReleaseAccepted", false));
        builder.append(',');
        appendJsonString(builder, "visualAcceptanceToken", stringExtra("rustyxr.visualAcceptanceToken", ""));
        builder.append(",\"xrRenderScale\":").append(floatJson(floatExtra("rustyxr.xrRenderScale", DEFAULT_XR_RENDER_SCALE)));
        builder.append(",\"xrDisplayRefreshHz\":").append(floatJson(floatExtra("rustyxr.xrDisplayRefreshHz", DEFAULT_XR_DISPLAY_REFRESH_HZ)));
        builder.append(",\"xrFixedFoveationLevel\":").append(fixedFoveationLevel);
        builder.append(',');
        appendJsonString(builder, "xrColorFormat", stringExtra("rustyxr.xrColorFormat", DEFAULT_XR_COLOR_FORMAT));
        builder.append(',');
        appendJsonString(builder, "environmentDepthMode", stringExtra("rustyxr.depth", DEFAULT_ENVIRONMENT_DEPTH_MODE));
        builder.append(",\"environmentDepthHandRemoval\":").append(booleanExtra("rustyxr.depthHandRemoval", DEFAULT_ENVIRONMENT_DEPTH_HAND_REMOVAL));
        builder.append(',');
        appendJsonString(builder, "handParticleMode", stringExtra("rustyxr.handParticles", DEFAULT_HAND_PARTICLE_MODE));
        builder.append(',');
        appendJsonString(builder, "openxrPassthroughProbe", stringExtra("rustyxr.openxrPassthroughProbe", DEFAULT_OPENXR_PASSTHROUGH_PROBE));
        builder.append(',');
        appendJsonString(builder, "passthroughStyleMode", stringExtra("rustyxr.passthroughStyleMode", DEFAULT_PASSTHROUGH_STYLE_MODE));
        builder.append(",\"passthroughOpacity\":").append(floatJson(floatExtra("rustyxr.passthroughOpacity", DEFAULT_PASSTHROUGH_OPACITY)));
        builder.append(",\"passthroughEdgeR\":").append(floatJson(floatExtra("rustyxr.passthroughEdgeR", DEFAULT_PASSTHROUGH_EDGE_R)));
        builder.append(",\"passthroughEdgeG\":").append(floatJson(floatExtra("rustyxr.passthroughEdgeG", DEFAULT_PASSTHROUGH_EDGE_G)));
        builder.append(",\"passthroughEdgeB\":").append(floatJson(floatExtra("rustyxr.passthroughEdgeB", DEFAULT_PASSTHROUGH_EDGE_B)));
        builder.append(",\"passthroughEdgeA\":").append(floatJson(floatExtra("rustyxr.passthroughEdgeA", DEFAULT_PASSTHROUGH_EDGE_A)));
        builder.append(",\"passthroughBrightness\":").append(floatJson(floatExtra("rustyxr.passthroughBrightness", DEFAULT_PASSTHROUGH_BRIGHTNESS)));
        builder.append(",\"passthroughContrast\":").append(floatJson(floatExtra("rustyxr.passthroughContrast", DEFAULT_PASSTHROUGH_CONTRAST)));
        builder.append(",\"passthroughSaturation\":").append(floatJson(floatExtra("rustyxr.passthroughSaturation", DEFAULT_PASSTHROUGH_SATURATION)));
        builder.append(",\"passthroughColorPhase\":").append(floatJson(floatExtra("rustyxr.passthroughColorPhase", DEFAULT_PASSTHROUGH_COLOR_PHASE)));
        builder.append(",\"passthroughColorAmplitude\":").append(floatJson(floatExtra("rustyxr.passthroughColorAmplitude", DEFAULT_PASSTHROUGH_COLOR_AMPLITUDE)));
        builder.append(",\"passthroughLutResolution\":").append(Math.max(2, intExtra("rustyxr.passthroughLutResolution", DEFAULT_PASSTHROUGH_LUT_RESOLUTION)));
        builder.append(",\"passthroughLutWeight\":").append(floatJson(floatExtra("rustyxr.passthroughLutWeight", DEFAULT_PASSTHROUGH_LUT_WEIGHT)));
        builder.append(",\"passthroughLutFlickerHz\":").append(floatJson(floatExtra("rustyxr.passthroughLutFlickerHz", DEFAULT_PASSTHROUGH_LUT_FLICKER_HZ)));
        builder.append(",\"fullFieldFlickerHz\":").append(floatJson(floatExtra("rustyxr.fullFieldFlickerHz", DEFAULT_FULL_FIELD_FLICKER_HZ)));
        builder.append(",\"projectionLayerVisible\":").append(booleanExtra("rustyxr.projectionLayerVisible", DEFAULT_PROJECTION_LAYER_VISIBLE));
        builder.append(",\"diagnosticHudVisible\":").append(hudVisible);
        builder.append(',');
        appendJsonString(builder, "diagnosticHudCommand", stringExtra("rustyxr.diagnosticHudCommand", DEFAULT_DIAGNOSTIC_HUD_COMMAND));
        builder.append(",\"oscEnabled\":").append(booleanExtra("rustyxr.oscEnabled", DEFAULT_OSC_ENABLED));
        builder.append(",\"oscOverlayEnabled\":").append(booleanExtra("rustyxr.oscOverlayEnabled", hudVisible && DEFAULT_OSC_OVERLAY_ENABLED));
        builder.append(',');
        appendJsonString(builder, "oscListenAddr", stringExtra("rustyxr.oscListenAddr", DEFAULT_OSC_LISTEN_ADDR));
        builder.append(",\"oscMaxPacketBytes\":").append(Math.max(256, intExtra("rustyxr.oscMaxPacketBytes", DEFAULT_OSC_MAX_PACKET_BYTES)));
        builder.append(',');
        appendJsonString(builder, "stereoLayout", cameraStereoLayout());
        builder.append('}');

        try {
            nativeRuntimeConfig(builder.toString());
        } catch (UnsatisfiedLinkError error) {
            Log.w(TAG, "Native runtime config bridge unavailable");
        }
    }

    private void requestHeadsetCameraPermissionsOrStart() {
        List<String> missing = new ArrayList<String>();
        if (checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
            missing.add(Manifest.permission.CAMERA);
        }
        if (checkSelfPermission(HEADSET_CAMERA_PERMISSION) != PackageManager.PERMISSION_GRANTED) {
            missing.add(HEADSET_CAMERA_PERMISSION);
        }

        if (missing.isEmpty()) {
            startHeadsetCamera();
            return;
        }

        Log.i(TAG, "Requesting headset camera permissions");
        sendNativeEvent("headsetCameraPermissionRequesting");
        requestPermissions(missing.toArray(new String[missing.size()]), CAMERA_PERMISSION_REQUEST);
    }

    private void startHeadsetCamera() {
        if ("native-ndk".equals(cameraAcquisition())) {
            startNativeHeadsetCamera();
            return;
        }

        Intent serviceIntent = new Intent(this, HeadsetCameraService.class);
        serviceIntent.putExtra(HeadsetCameraService.EXTRA_WIDTH, intExtra("rustyxr.cameraWidth", DEFAULT_CAMERA_SIZE));
        serviceIntent.putExtra(HeadsetCameraService.EXTRA_HEIGHT, intExtra("rustyxr.cameraHeight", DEFAULT_CAMERA_SIZE));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_PREFERRED_SQUARE_SIZE,
            intExtra("rustyxr.cameraPreferredSquare", DEFAULT_CAMERA_SIZE));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_MAX_DIMENSION,
            intExtra("rustyxr.cameraMaxDimension", DEFAULT_CAMERA_MAX_DIMENSION));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_CPU_UPLOAD_HZ,
            intExtra("rustyxr.cameraCpuUploadHz", DEFAULT_CAMERA_CPU_UPLOAD_HZ));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_CAMERA_TARGET_FPS,
            intExtra("rustyxr.cameraTargetFps", DEFAULT_CAMERA_TARGET_FPS));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_CAMERA_FPS_MIN,
            intExtra("rustyxr.cameraFpsMin", DEFAULT_CAMERA_FPS_MIN));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_CAMERA_FPS_MAX,
            intExtra("rustyxr.cameraFpsMax", DEFAULT_CAMERA_FPS_MAX));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_STEREO_IMAGE_READER_MAX_IMAGES,
            intExtra("rustyxr.cameraStereoImageReaderMaxImages", DEFAULT_CAMERA_STEREO_IMAGE_READER_MAX_IMAGES));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_PROJECTION_GEOMETRY_PROFILE,
            stringExtra("rustyxr.cameraProjectionGeometryProfile", "full-frame-diagnostic"));
        serviceIntent.putExtra(HeadsetCameraService.EXTRA_CAMERA_TIER, cameraTier());
        serviceIntent.putExtra(HeadsetCameraService.EXTRA_STEREO_LAYOUT, cameraStereoLayout());
        serviceIntent.putExtra(HeadsetCameraService.EXTRA_ALLOW_CPU_FALLBACK, allowCpuFallback());
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE,
            booleanExtra("rustyxr.cameraEstimatedPose", false));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_X,
            floatExtra("rustyxr.cameraEstimatedPoseX", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_Y,
            floatExtra("rustyxr.cameraEstimatedPoseY", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_Z,
            floatExtra("rustyxr.cameraEstimatedPoseZ", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_QX,
            floatExtra("rustyxr.cameraEstimatedPoseQx", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_QY,
            floatExtra("rustyxr.cameraEstimatedPoseQy", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_QZ,
            floatExtra("rustyxr.cameraEstimatedPoseQz", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_QW,
            floatExtra("rustyxr.cameraEstimatedPoseQw", 1.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_STEREO_POSE,
            booleanExtra("rustyxr.cameraEstimatedStereoPose", false));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_LABEL,
            stringExtra("rustyxr.cameraEstimatedPoseLabel", "launch-extra"));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_POSE_VERSION,
            stringExtra("rustyxr.cameraEstimatedPoseVersion", "unspecified"));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_POSE_COORDINATE_CONVENTION,
            stringExtra("rustyxr.cameraPoseCoordinateConvention", "android-camera2-lens-pose-reference-from-camera"));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_LEFT_POSE_X,
            floatExtra("rustyxr.cameraEstimatedLeftPoseX", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_LEFT_POSE_Y,
            floatExtra("rustyxr.cameraEstimatedLeftPoseY", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_LEFT_POSE_Z,
            floatExtra("rustyxr.cameraEstimatedLeftPoseZ", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_LEFT_POSE_QX,
            floatExtra("rustyxr.cameraEstimatedLeftPoseQx", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_LEFT_POSE_QY,
            floatExtra("rustyxr.cameraEstimatedLeftPoseQy", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_LEFT_POSE_QZ,
            floatExtra("rustyxr.cameraEstimatedLeftPoseQz", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_LEFT_POSE_QW,
            floatExtra("rustyxr.cameraEstimatedLeftPoseQw", 1.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_RIGHT_POSE_X,
            floatExtra("rustyxr.cameraEstimatedRightPoseX", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_RIGHT_POSE_Y,
            floatExtra("rustyxr.cameraEstimatedRightPoseY", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_RIGHT_POSE_Z,
            floatExtra("rustyxr.cameraEstimatedRightPoseZ", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_RIGHT_POSE_QX,
            floatExtra("rustyxr.cameraEstimatedRightPoseQx", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_RIGHT_POSE_QY,
            floatExtra("rustyxr.cameraEstimatedRightPoseQy", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_RIGHT_POSE_QZ,
            floatExtra("rustyxr.cameraEstimatedRightPoseQz", 0.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_ESTIMATED_RIGHT_POSE_QW,
            floatExtra("rustyxr.cameraEstimatedRightPoseQw", 1.0f));
        serviceIntent.putExtra(
            HeadsetCameraService.EXTRA_STEREO_PAIR_MAX_DELTA_NS,
            longExtra("rustyxr.cameraStereoPairMaxDeltaNs", 5_000_000L));
        try {
            startService(serviceIntent);
            sendNativeEvent("headsetCameraServiceStarted");
        } catch (RuntimeException error) {
            Log.e(TAG, "Could not start headset camera service; app may not be foreground in the headset", error);
            sendNativeEvent("headsetCameraServiceStartFailed");
        }
    }

    private void startBrokerH264ConsumerProbe() {
        if (brokerH264ConsumerProbe != null) {
            return;
        }

        boolean brokerStereo = booleanExtra("rustyxr.brokerH264Stereo", false);
        String brokerCameraId = stringExtra("rustyxr.brokerH264CameraId", "");
        String brokerLeftCameraId = stringExtra(
            "rustyxr.brokerH264LeftCameraId",
            brokerStereo ? DEFAULT_BROKER_H264_LEFT_CAMERA_ID : brokerCameraId);
        String brokerRightCameraId = stringExtra(
            "rustyxr.brokerH264RightCameraId",
            brokerStereo ? DEFAULT_BROKER_H264_RIGHT_CAMERA_ID : "");
        if (brokerStereo && shouldAutoSelectBrokerH264StereoCameraIds(brokerLeftCameraId, brokerRightCameraId)) {
            BrokerH264StereoCameraIds autoIds =
                chooseBrokerH264StereoCameraIds(brokerLeftCameraId, brokerRightCameraId);
            if (autoIds != null) {
                if (brokerLeftCameraId.length() == 0 || brokerLeftCameraId.equals(brokerRightCameraId)) {
                    brokerLeftCameraId = autoIds.leftCameraId;
                }
                if (brokerRightCameraId.length() == 0 || brokerRightCameraId.equals(brokerLeftCameraId)) {
                    brokerRightCameraId = autoIds.rightCameraId;
                }
                Log.i(TAG, "Broker H.264 stereo Camera2 ids left=" + brokerLeftCameraId +
                    " right=" + brokerRightCameraId +
                    " reason=" + autoIds.reason);
                sendNativeEvent("brokerH264StereoCameraIdsSelected");
            } else {
                Log.w(TAG, "Broker H.264 stereo Camera2 auto-selection found no distinct concurrent pair");
                sendNativeEvent("brokerH264StereoCameraIdsUnavailable");
            }
        }

        boolean brokerLiveStream = booleanExtra("rustyxr.brokerH264LiveStream", false);
        int brokerH264CaptureMs = intExtra("rustyxr.brokerH264CaptureMs", DEFAULT_BROKER_H264_CAPTURE_MS);
        int brokerH264MaxPackets = intExtra("rustyxr.brokerH264MaxPackets", DEFAULT_BROKER_H264_MAX_PACKETS);

        BrokerH264ConsumerProbe.Config config = new BrokerH264ConsumerProbe.Config(
            stringExtra("rustyxr.brokerHost", DEFAULT_BROKER_HOST),
            Math.max(1, intExtra("rustyxr.brokerPort", DEFAULT_BROKER_PORT)),
            Math.max(1, intExtra("rustyxr.brokerH264StreamPort", DEFAULT_BROKER_H264_STREAM_PORT)),
            Math.max(1, intExtra("rustyxr.brokerH264RightStreamPort", DEFAULT_BROKER_H264_RIGHT_STREAM_PORT)),
            brokerCameraId,
            brokerLeftCameraId,
            brokerRightCameraId,
            Math.max(16, intExtra("rustyxr.brokerH264Width", DEFAULT_BROKER_H264_WIDTH)),
            Math.max(16, intExtra("rustyxr.brokerH264Height", DEFAULT_BROKER_H264_HEIGHT)),
            brokerLiveStream ? Math.max(0, brokerH264CaptureMs) : Math.max(100, brokerH264CaptureMs),
            brokerLiveStream ? Math.max(0, brokerH264MaxPackets) : Math.max(1, brokerH264MaxPackets),
            Math.max(100000, intExtra("rustyxr.brokerH264BitrateBps", DEFAULT_BROKER_H264_BITRATE_BPS)),
            clampInt(
                intExtra("rustyxr.brokerH264FrameRateHz", DEFAULT_BROKER_H264_FRAME_RATE_HZ),
                1,
                120),
            Math.max(1000, intExtra("rustyxr.brokerH264CommandTimeoutMs", DEFAULT_BROKER_H264_COMMAND_TIMEOUT_MS)),
            Math.max(1000, intExtra("rustyxr.brokerH264StreamTimeoutMs", DEFAULT_BROKER_H264_STREAM_TIMEOUT_MS)),
            Math.max(1000, intExtra("rustyxr.brokerH264DecodeTimeoutMs", DEFAULT_BROKER_H264_DECODE_TIMEOUT_MS)),
            stringExtra("rustyxr.brokerH264DecodeOutputMode", DEFAULT_BROKER_H264_DECODE_OUTPUT_MODE),
            brokerStereo,
            brokerLiveStream,
            stringExtra("rustyxr.brokerH264SourceMode", DEFAULT_BROKER_H264_SOURCE_MODE),
            stringExtra("rustyxr.brokerH264SyntheticPattern", DEFAULT_BROKER_H264_SYNTHETIC_PATTERN),
            stringExtra(
                "rustyxr.brokerH264SyntheticProjectionProfile",
                DEFAULT_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE),
            stringExtra(
                "rustyxr.brokerH264ProjectionGeometryProfile",
                stringExtra(
                    "rustyxr.brokerH264SyntheticProjectionProfile",
                    DEFAULT_BROKER_H264_SYNTHETIC_PROJECTION_PROFILE)),
            booleanExtra("rustyxr.brokerH264LiveDecode", DEFAULT_BROKER_H264_LIVE_DECODE),
            booleanExtra("rustyxr.brokerH264ByteIdentityProbe", DEFAULT_BROKER_H264_BYTE_IDENTITY_PROBE),
            stringExtra("rustyxr.brokerH264StereoPairingMode", DEFAULT_BROKER_H264_STEREO_PAIRING_MODE),
            Math.max(2, intExtra(
                "rustyxr.brokerH264LivePairQueueLimit",
                DEFAULT_BROKER_H264_LIVE_PAIR_QUEUE_LIMIT)),
            stringExtra("rustyxr.brokerH264ProjectionMetadataJson", ""),
            stringExtra("rustyxr.brokerH264LeftProjectionMetadataJson", ""),
            stringExtra("rustyxr.brokerH264RightProjectionMetadataJson", ""),
            stringExtra("rustyxr.brokerH264ProjectionMetadataBase64", ""),
            stringExtra("rustyxr.brokerH264LeftProjectionMetadataBase64", ""),
            stringExtra("rustyxr.brokerH264RightProjectionMetadataBase64", ""));
        Log.i(TAG, "Starting broker H.264 consumer probe");
        sendNativeEvent("brokerH264ConsumerStarting");
        brokerH264ConsumerProbe = BrokerH264ConsumerProbe.start(
            config,
            new BrokerH264ConsumerProbe.Sink() {
                @Override
                public void onBrokerH264ConsumerProbe(JSONObject report) {
                    try {
                        report.put("event", "brokerH264ConsumerProbe");
                        nativeActivityEvent(report.toString());
                    } catch (Exception error) {
                        Log.w(TAG, "Could not forward broker H.264 consumer report", error);
                    }
                    brokerH264ConsumerProbe = null;
                }
            });
    }

    private boolean shouldAutoSelectBrokerH264StereoCameraIds(String leftCameraId, String rightCameraId) {
        if (!booleanExtra("rustyxr.brokerH264AutoStereoCameraIds", true)) {
            return false;
        }
        return leftCameraId == null ||
            rightCameraId == null ||
            leftCameraId.length() == 0 ||
            rightCameraId.length() == 0 ||
            leftCameraId.equals(rightCameraId);
    }

    private BrokerH264StereoCameraIds chooseBrokerH264StereoCameraIds(
        String requestedLeftCameraId,
        String requestedRightCameraId) {
        CameraManager manager = (CameraManager) getSystemService(Context.CAMERA_SERVICE);
        if (manager == null) {
            return null;
        }
        try {
            List<Set<String>> concurrentSets = brokerH264ConcurrentCameraSets(manager);
            List<BrokerH264CameraCandidate> candidates = new ArrayList<BrokerH264CameraCandidate>();
            String[] cameraIds = manager.getCameraIdList();
            for (int i = 0; i < cameraIds.length; i++) {
                BrokerH264CameraCandidate candidate = brokerH264CameraCandidate(manager, cameraIds[i]);
                if (candidate != null && candidate.hasPrivateOutput) {
                    candidates.add(candidate);
                }
            }

            BrokerH264StereoCameraIds best = null;
            long bestScore = Long.MIN_VALUE;
            for (int leftIndex = 0; leftIndex < candidates.size(); leftIndex++) {
                for (int rightIndex = leftIndex + 1; rightIndex < candidates.size(); rightIndex++) {
                    BrokerH264CameraCandidate first = candidates.get(leftIndex);
                    BrokerH264CameraCandidate second = candidates.get(rightIndex);
                    if (!concurrentSets.isEmpty() &&
                            !brokerH264CanRunConcurrently(concurrentSets, first.cameraId, second.cameraId)) {
                        continue;
                    }

                    BrokerH264CameraCandidate left = first;
                    BrokerH264CameraCandidate right = second;
                    if (requestedLeftCameraId != null && requestedLeftCameraId.length() > 0) {
                        if (requestedLeftCameraId.equals(second.cameraId)) {
                            left = second;
                            right = first;
                        } else if (!requestedLeftCameraId.equals(first.cameraId)) {
                            continue;
                        }
                    }
                    if (requestedRightCameraId != null && requestedRightCameraId.length() > 0) {
                        if (requestedRightCameraId.equals(left.cameraId)) {
                            BrokerH264CameraCandidate swap = left;
                            left = right;
                            right = swap;
                        }
                        if (!requestedRightCameraId.equals(right.cameraId)) {
                            continue;
                        }
                    }
                    if (left.cameraId.equals(right.cameraId)) {
                        continue;
                    }

                    long score = brokerH264StereoScore(left, right);
                    if (!concurrentSets.isEmpty()) {
                        score += 100_000_000_000_000L;
                    }
                    if (score > bestScore) {
                        bestScore = score;
                        String reason = (!concurrentSets.isEmpty() ? "selected concurrent Camera2 pair " : "selected distinct Camera2 pair ") +
                            left.cameraId + "/" + right.cameraId;
                        best = new BrokerH264StereoCameraIds(left.cameraId, right.cameraId, reason);
                    }
                }
            }
            return best;
        } catch (CameraAccessException error) {
            Log.w(TAG, "Could not auto-select broker H.264 stereo Camera2 ids", error);
            return null;
        } catch (SecurityException error) {
            Log.w(TAG, "Camera permission blocked broker H.264 stereo Camera2 id auto-selection", error);
            return null;
        } catch (RuntimeException error) {
            Log.w(TAG, "Broker H.264 stereo Camera2 id auto-selection failed", error);
            return null;
        }
    }

    private static List<Set<String>> brokerH264ConcurrentCameraSets(CameraManager manager)
        throws CameraAccessException {
        List<Set<String>> sets = new ArrayList<Set<String>>();
        if (Build.VERSION.SDK_INT < 30) {
            return sets;
        }
        Set<Set<String>> exposed = manager.getConcurrentCameraIds();
        if (exposed == null) {
            return sets;
        }
        for (Set<String> set : exposed) {
            if (set != null && set.size() >= 2) {
                sets.add(new HashSet<String>(set));
            }
        }
        return sets;
    }

    private static boolean brokerH264CanRunConcurrently(List<Set<String>> sets, String left, String right) {
        for (int i = 0; i < sets.size(); i++) {
            Set<String> set = sets.get(i);
            if (set.contains(left) && set.contains(right)) {
                return true;
            }
        }
        return false;
    }

    private static BrokerH264CameraCandidate brokerH264CameraCandidate(CameraManager manager, String cameraId)
        throws CameraAccessException {
        CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
        StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
        if (map == null) {
            return null;
        }
        Size bestSize = brokerH264BestOutputSize(map);
        if (bestSize == null) {
            return null;
        }
        Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
        boolean hasPose =
            characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION) != null &&
            characteristics.get(CameraCharacteristics.LENS_POSE_ROTATION) != null;
        boolean hasIntrinsics =
            characteristics.get(CameraCharacteristics.LENS_INTRINSIC_CALIBRATION) != null;
        return new BrokerH264CameraCandidate(
            cameraId,
            bestSize.getWidth(),
            bestSize.getHeight(),
            brokerH264LensFacingRank(facing),
            hasPose,
            hasIntrinsics);
    }

    private static Size brokerH264BestOutputSize(StreamConfigurationMap map) {
        Size best = brokerH264BestOutputSize(map.getOutputSizes(ImageFormat.PRIVATE), null);
        if (best != null) {
            return best;
        }
        try {
            return brokerH264BestOutputSize(map.getOutputSizes(Surface.class), null);
        } catch (IllegalArgumentException ignored) {
            return null;
        }
    }

    private static Size brokerH264BestOutputSize(Size[] sizes, Size fallback) {
        Size best = fallback;
        long bestArea = fallback != null ? (long) fallback.getWidth() * (long) fallback.getHeight() : -1L;
        if (sizes == null) {
            return best;
        }
        for (int i = 0; i < sizes.length; i++) {
            Size size = sizes[i];
            long area = (long) size.getWidth() * (long) size.getHeight();
            if (area > bestArea) {
                best = size;
                bestArea = area;
            }
        }
        return best;
    }

    private static int brokerH264LensFacingRank(Integer facing) {
        if (facing == null) {
            return 0;
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_BACK) {
            return 3;
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return 2;
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_FRONT) {
            return 1;
        }
        return 0;
    }

    private static long brokerH264StereoScore(BrokerH264CameraCandidate left, BrokerH264CameraCandidate right) {
        long score = 0L;
        score += (long) (left.lensFacingRank + right.lensFacingRank) * 1_000_000_000_000L;
        score += (long) left.width * (long) left.height;
        score += (long) right.width * (long) right.height;
        if (left.width == right.width && left.height == right.height) {
            score += 50_000_000_000L;
        }
        if (left.hasPose && right.hasPose) {
            score += 20_000_000_000L;
        }
        if (left.hasIntrinsics && right.hasIntrinsics) {
            score += 20_000_000_000L;
        }
        return score;
    }

    private static final class BrokerH264StereoCameraIds {
        final String leftCameraId;
        final String rightCameraId;
        final String reason;

        BrokerH264StereoCameraIds(String leftCameraId, String rightCameraId, String reason) {
            this.leftCameraId = leftCameraId;
            this.rightCameraId = rightCameraId;
            this.reason = reason;
        }
    }

    private static final class BrokerH264CameraCandidate {
        final String cameraId;
        final int width;
        final int height;
        final int lensFacingRank;
        final boolean hasPose;
        final boolean hasIntrinsics;
        final boolean hasPrivateOutput;

        BrokerH264CameraCandidate(
            String cameraId,
            int width,
            int height,
            int lensFacingRank,
            boolean hasPose,
            boolean hasIntrinsics) {
            this.cameraId = cameraId;
            this.width = width;
            this.height = height;
            this.lensFacingRank = lensFacingRank;
            this.hasPose = hasPose;
            this.hasIntrinsics = hasIntrinsics;
            this.hasPrivateOutput = width > 0 && height > 0;
        }
    }

    private void stopBrokerH264ConsumerProbe() {
        if (brokerH264ConsumerProbe != null) {
            brokerH264ConsumerProbe.stop();
            brokerH264ConsumerProbe = null;
            sendNativeEvent("brokerH264ConsumerStopped");
        }
    }

    private void startNativeHeadsetCamera() {
        String configJson = nativeCameraConfigJson();
        try {
            if (nativeStartNativeCamera(configJson)) {
                Log.i(TAG, "Native NDK headset camera acquisition started");
                sendNativeEvent("headsetCameraNativeStarted");
            } else {
                Log.e(TAG, "Native NDK headset camera acquisition failed to start");
                sendNativeEvent("headsetCameraNativeStartFailed");
            }
        } catch (RuntimeException error) {
            Log.e(TAG, "Could not start native NDK headset camera acquisition", error);
            sendNativeEvent("headsetCameraNativeStartFailed");
        } catch (UnsatisfiedLinkError error) {
            Log.e(TAG, "Native NDK headset camera bridge unavailable", error);
            sendNativeEvent("headsetCameraNativeStartUnavailable");
        }
    }

    private String nativeCameraConfigJson() {
        StringBuilder builder = new StringBuilder(256);
        builder.append('{');
        builder.append("\"width\":").append(Math.max(1, intExtra("rustyxr.cameraWidth", DEFAULT_CAMERA_SIZE)));
        builder.append(",\"height\":").append(Math.max(1, intExtra("rustyxr.cameraHeight", DEFAULT_CAMERA_SIZE)));
        builder.append(",\"maxDimension\":").append(Math.max(1, intExtra("rustyxr.cameraMaxDimension", DEFAULT_CAMERA_MAX_DIMENSION)));
        builder.append(",\"preferredSquare\":").append(Math.max(0, intExtra("rustyxr.cameraPreferredSquare", DEFAULT_CAMERA_SIZE)));
        builder.append(",\"readerMaxImages\":").append(Math.max(2, intExtra("rustyxr.cameraStereoImageReaderMaxImages", 3)));
        builder.append(",\"stereoPairMaxDeltaNs\":").append(Math.max(0L, longExtra("rustyxr.cameraStereoPairMaxDeltaNs", 5_000_000L)));
        builder.append(',');
        appendJsonString(builder, "requestedTier", cameraTier());
        builder.append(',');
        appendJsonString(builder, "requestedStereoLayout", cameraStereoLayout());
        builder.append(',');
        appendJsonString(builder, "sourceMode", stringExtra("rustyxr.nativeSourceMode", "auto"));
        builder.append(',');
        appendJsonString(builder, "leftCameraId", stringExtra("rustyxr.nativeLeftCameraId", ""));
        builder.append(',');
        appendJsonString(builder, "rightCameraId", stringExtra("rustyxr.nativeRightCameraId", ""));
        builder.append('}');
        return builder.toString();
    }

    private void requestMediaProjection() {
        if (mediaProjectionManager == null) {
            Log.w(TAG, "MediaProjectionManager is unavailable");
            sendNativeEvent("mediaProjectionManagerUnavailable");
            return;
        }

        Log.i(TAG, "Requesting MediaProjection consent");
        sendNativeEvent("mediaProjectionRequesting");
        startActivityForResult(
            mediaProjectionManager.createScreenCaptureIntent(),
            MEDIA_PROJECTION_REQUEST);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != MEDIA_PROJECTION_REQUEST) {
            return;
        }

        if (resultCode != RESULT_OK || data == null) {
            Log.w(TAG, "MediaProjection consent denied or cancelled");
            sendNativeEvent("mediaProjectionDenied");
            return;
        }

        Log.i(TAG, "MediaProjection consent granted; starting stream service");
        Intent serviceIntent = new Intent(this, MediaProjectionStreamService.class);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_RESULT_CODE, resultCode);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_RESULT_DATA, data);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_HOST, "127.0.0.1");
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_PORT, intExtra("rustyxr.mediaProjectionPort", 8787));
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_WIDTH, intExtra("rustyxr.mediaProjectionWidth", 512));
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_HEIGHT, intExtra("rustyxr.mediaProjectionHeight", 288));
        startForegroundService(serviceIntent);
        sendNativeEvent("mediaProjectionGranted");
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode != CAMERA_PERMISSION_REQUEST) {
            return;
        }

        for (int i = 0; i < grantResults.length; i++) {
            if (grantResults[i] != PackageManager.PERMISSION_GRANTED) {
                Log.w(TAG, "Headset camera permission denied");
                sendNativeEvent("headsetCameraPermissionDenied");
                return;
            }
        }

        Log.i(TAG, "Headset camera permissions granted");
        sendNativeEvent("headsetCameraPermissionGranted");
        startHeadsetCamera();
    }

    @Override
    protected void onDestroy() {
        stopBrokerH264ConsumerProbe();
        try {
            nativeStopNativeCamera();
        } catch (UnsatisfiedLinkError ignored) {
            // Older local APKs may not have the optional native camera bridge.
        }
        stopService(new Intent(this, MediaProjectionStreamService.class));
        stopService(new Intent(this, HeadsetCameraService.class));
        sendNativeEvent("activityDestroyed");
        Log.i(TAG, "Rusty XR composite layer activity destroyed");
        super.onDestroy();
    }

    private static void sendNativeEvent(String name) {
        try {
            nativeActivityEvent("{\"event\":\"" + name + "\",\"timeNs\":" + System.nanoTime() + "}");
        } catch (UnsatisfiedLinkError error) {
            Log.w(TAG, "Native event bridge unavailable: " + name);
        }
    }

    private static void appendJsonString(StringBuilder builder, String key, String value) {
        builder.append('"').append(key).append("\":\"");
        appendJsonEscaped(builder, value);
        builder.append('"');
    }

    private static String floatJson(float value) {
        if (!Float.isNaN(value) && !Float.isInfinite(value)) {
            return Float.toString(value);
        }
        return "0.0";
    }

    private static void appendJsonEscaped(StringBuilder builder, String value) {
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            if (c == '"' || c == '\\') {
                builder.append('\\').append(c);
            } else if (c == '\n') {
                builder.append("\\n");
            } else if (c == '\r') {
                builder.append("\\r");
            } else if (c == '\t') {
                builder.append("\\t");
            } else {
                builder.append(c);
            }
        }
    }
}
