package com.example.rustyxr.composite;

import android.Manifest;
import android.app.NativeActivity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.media.projection.MediaProjectionManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.view.Window;
import android.view.WindowManager;
import java.util.ArrayList;
import java.util.List;

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
    private static final float DEFAULT_CAMERA_PROJECTION_SCALE = 0.75f;
    private static final float DEFAULT_CAMERA_RAW_OVERLAY_OVERSCAN = 1.06f;
    private static final float DEFAULT_CAMERA_FULL_VIEW_OVERLAY_OVERSCAN = 2.10f;
    private static final float DEFAULT_CAMERA_EDGE_FADE = 0.12f;
    private static final String DEFAULT_CAMERA_PROJECTION_MODE = "display-screen-homography";
    private static final String DEFAULT_CAMERA_PIPELINE_PRESET = "manual";
    private static final String DEFAULT_CAMERA_PROJECTION_EFFECT_MODE = "border-composite";
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
    private static final float DEFAULT_CAMERA_BORDER_CYCLE_HZ = 0.18f;
    private static final String DEFAULT_CAMERA_TEXTURE_ROTATION = "rotate0";
    private static final boolean DEFAULT_CAMERA_TEXTURE_FLIP_X = false;
    private static final boolean DEFAULT_CAMERA_TEXTURE_FLIP_Y = false;
    private static final boolean DEFAULT_CAMERA_TEXTURE_MIRROR = false;
    private static final String DEFAULT_CAMERA_TEXTURE_TRANSFORM_SOURCE = "default";
    private static final String DEFAULT_CAMERA_TEXTURE_TRANSFORM_REASON = "unspecified";
    private static final String DEFAULT_CAMERA_SOURCE_EYE_MAPPING = "left-right";
    private static final String DEFAULT_CAMERA_ORIENTATION_DIAGNOSTIC_MODE = "off";
    private static final float DEFAULT_XR_RENDER_SCALE = 0.75f;
    private static final int DEFAULT_XR_FIXED_FOVEATION_LEVEL = 0;
    private static final String DEFAULT_XR_COLOR_FORMAT = "rgba8-srgb";
    private static final String DEFAULT_OPENXR_PASSTHROUGH_PROBE = "off";

    private MediaProjectionManager mediaProjectionManager;

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

        boolean cameraEnabled = shouldStartHeadsetCamera();
        boolean mediaProjectionEnabled = shouldRequestMediaProjection();
        sendRuntimeConfig(cameraEnabled, mediaProjectionEnabled);

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

        if (mediaProjectionEnabled) {
            new Handler(Looper.getMainLooper()).postDelayed(new Runnable() {
                @Override
                public void run() {
                    requestMediaProjection();
                }
            }, mediaProjectionDelayMs());
        }
    }

    private boolean shouldRequestMediaProjection() {
        return booleanExtra("rustyxr.mediaProjection", false);
    }

    private boolean shouldStartHeadsetCamera() {
        return booleanExtra("rustyxr.camera", true);
    }

    private boolean booleanExtra(String name, boolean defaultValue) {
        Intent intent = getIntent();
        if (intent == null || intent.getExtras() == null || !intent.getExtras().containsKey(name)) {
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

    private void sendRuntimeConfig(boolean cameraEnabled, boolean mediaProjectionEnabled) {
        String tier = cameraEnabled ? cameraTier() : "synthetic";
        int cpuUploadHz = intExtra("rustyxr.cameraCpuUploadHz", DEFAULT_CAMERA_CPU_UPLOAD_HZ);
        int cameraTargetFps = Math.max(0, intExtra("rustyxr.cameraTargetFps", DEFAULT_CAMERA_TARGET_FPS));
        int cameraFpsMin = Math.max(0, intExtra("rustyxr.cameraFpsMin", DEFAULT_CAMERA_FPS_MIN));
        int cameraFpsMax = Math.max(0, intExtra("rustyxr.cameraFpsMax", DEFAULT_CAMERA_FPS_MAX));
        int stereoImageReaderMaxImages = Math.max(2, intExtra("rustyxr.cameraStereoImageReaderMaxImages", DEFAULT_CAMERA_STEREO_IMAGE_READER_MAX_IMAGES));
        int fixedFoveationLevel = Math.max(0, intExtra("rustyxr.xrFixedFoveationLevel", DEFAULT_XR_FIXED_FOVEATION_LEVEL));
        StringBuilder builder = new StringBuilder(256);
        builder.append('{');
        appendJsonString(builder, "cameraTier", tier);
        builder.append(',');
        appendJsonString(builder, "cameraAcquisition", cameraAcquisition());
        builder.append(",\"cameraEnabled\":").append(cameraEnabled);
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
        builder.append(",\"cameraProjectionFovYDegrees\":").append(floatJson(floatExtra("rustyxr.cameraProjectionFovYDegrees", DEFAULT_CAMERA_PROJECTION_FOV_Y_DEGREES)));
        builder.append(",\"cameraPreviewFovYDegrees\":").append(floatJson(floatExtra("rustyxr.cameraPreviewFovYDegrees", DEFAULT_CAMERA_PREVIEW_FOV_Y_DEGREES)));
        builder.append(",\"cameraProjectionScale\":").append(floatJson(floatExtra("rustyxr.cameraProjectionScale", DEFAULT_CAMERA_PROJECTION_SCALE)));
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
        builder.append(",\"cameraBorderCycleHz\":").append(floatJson(floatExtra("rustyxr.cameraBorderCycleHz", DEFAULT_CAMERA_BORDER_CYCLE_HZ)));
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
        builder.append(",\"xrFixedFoveationLevel\":").append(fixedFoveationLevel);
        builder.append(',');
        appendJsonString(builder, "xrColorFormat", stringExtra("rustyxr.xrColorFormat", DEFAULT_XR_COLOR_FORMAT));
        builder.append(',');
        appendJsonString(builder, "openxrPassthroughProbe", stringExtra("rustyxr.openxrPassthroughProbe", DEFAULT_OPENXR_PASSTHROUGH_PROBE));
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
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_PORT, 8787);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_WIDTH, 512);
        serviceIntent.putExtra(MediaProjectionStreamService.EXTRA_HEIGHT, 288);
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
