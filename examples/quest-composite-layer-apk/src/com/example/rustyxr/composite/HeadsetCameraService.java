package com.example.rustyxr.composite;

import android.Manifest;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.graphics.Rect;
import android.hardware.HardwareBuffer;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.OutputConfiguration;
import android.hardware.camera2.params.SessionConfiguration;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.IBinder;
import android.util.Log;
import android.util.Range;
import android.util.Size;
import android.view.Surface;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.concurrent.Executor;

public final class HeadsetCameraService extends Service {
    public static final String EXTRA_WIDTH = "width";
    public static final String EXTRA_HEIGHT = "height";
    public static final String EXTRA_PREFERRED_SQUARE_SIZE = "preferredSquareSize";
    public static final String EXTRA_MAX_DIMENSION = "maxDimension";
    public static final String EXTRA_CPU_UPLOAD_HZ = "cpuUploadHz";
    public static final String EXTRA_CAMERA_TARGET_FPS = "cameraTargetFps";
    public static final String EXTRA_CAMERA_FPS_MIN = "cameraFpsMin";
    public static final String EXTRA_CAMERA_FPS_MAX = "cameraFpsMax";
    public static final String EXTRA_CAMERA_TIER = "cameraTier";
    public static final String EXTRA_STEREO_LAYOUT = "stereoLayout";
    public static final String EXTRA_ALLOW_CPU_FALLBACK = "allowCpuFallback";
    public static final String EXTRA_ESTIMATED_POSE = "estimatedPose";
    public static final String EXTRA_ESTIMATED_POSE_X = "estimatedPoseX";
    public static final String EXTRA_ESTIMATED_POSE_Y = "estimatedPoseY";
    public static final String EXTRA_ESTIMATED_POSE_Z = "estimatedPoseZ";
    public static final String EXTRA_ESTIMATED_POSE_QX = "estimatedPoseQx";
    public static final String EXTRA_ESTIMATED_POSE_QY = "estimatedPoseQy";
    public static final String EXTRA_ESTIMATED_POSE_QZ = "estimatedPoseQz";
    public static final String EXTRA_ESTIMATED_POSE_QW = "estimatedPoseQw";
    public static final String EXTRA_ESTIMATED_STEREO_POSE = "estimatedStereoPose";
    public static final String EXTRA_ESTIMATED_POSE_LABEL = "estimatedPoseLabel";
    public static final String EXTRA_ESTIMATED_POSE_VERSION = "estimatedPoseVersion";
    public static final String EXTRA_POSE_COORDINATE_CONVENTION = "poseCoordinateConvention";
    public static final String EXTRA_ESTIMATED_LEFT_POSE_X = "estimatedLeftPoseX";
    public static final String EXTRA_ESTIMATED_LEFT_POSE_Y = "estimatedLeftPoseY";
    public static final String EXTRA_ESTIMATED_LEFT_POSE_Z = "estimatedLeftPoseZ";
    public static final String EXTRA_ESTIMATED_LEFT_POSE_QX = "estimatedLeftPoseQx";
    public static final String EXTRA_ESTIMATED_LEFT_POSE_QY = "estimatedLeftPoseQy";
    public static final String EXTRA_ESTIMATED_LEFT_POSE_QZ = "estimatedLeftPoseQz";
    public static final String EXTRA_ESTIMATED_LEFT_POSE_QW = "estimatedLeftPoseQw";
    public static final String EXTRA_ESTIMATED_RIGHT_POSE_X = "estimatedRightPoseX";
    public static final String EXTRA_ESTIMATED_RIGHT_POSE_Y = "estimatedRightPoseY";
    public static final String EXTRA_ESTIMATED_RIGHT_POSE_Z = "estimatedRightPoseZ";
    public static final String EXTRA_ESTIMATED_RIGHT_POSE_QX = "estimatedRightPoseQx";
    public static final String EXTRA_ESTIMATED_RIGHT_POSE_QY = "estimatedRightPoseQy";
    public static final String EXTRA_ESTIMATED_RIGHT_POSE_QZ = "estimatedRightPoseQz";
    public static final String EXTRA_ESTIMATED_RIGHT_POSE_QW = "estimatedRightPoseQw";
    public static final String EXTRA_STEREO_PAIR_MAX_DELTA_NS = "stereoPairMaxDeltaNs";

    private static final String TAG = "RustyXrHeadsetCamera";
    private static final String HEADSET_CAMERA_PERMISSION = "horizonos.permission.HEADSET_CAMERA";
    private static final int DEFAULT_PREFERRED_SQUARE_SIZE = 1280;
    private static final int DEFAULT_MAX_DIMENSION = 1920;
    private static final int DEFAULT_CPU_UPLOAD_HZ = 4;
    private static final long DEFAULT_STEREO_PAIR_MAX_DELTA_NS = 5_000_000L;
    private static final int STEREO_PENDING_QUEUE_LIMIT = 3;
    private static final int STEREO_IMAGE_READER_MAX_IMAGES = 8;
    private static final String CAMERA_SOURCE_DIAGNOSTICS_FILE = "camera-source-diagnostics.json";
    private static final String TIER_SOURCE_DIAGNOSTICS = "camera-source-diagnostics";
    private static final String TIER_CPU_DIAGNOSTIC = "cpu-diagnostic-flat-copy";
    private static final String TIER_GPU_BUFFER_PROBE = "gpu-buffer-probe";
    private static final String TIER_GPU_PROJECTED = "gpu-projected";
    private static final String CAMERA2_POSE_CONVENTION =
        "android-camera2-lens-pose-reference-from-camera";

    private HandlerThread cameraThread;
    private Handler cameraHandler;
    private CameraDevice cameraDevice;
    private CameraDevice leftCameraDevice;
    private CameraDevice rightCameraDevice;
    private CameraCaptureSession captureSession;
    private CameraCaptureSession leftCaptureSession;
    private CameraCaptureSession rightCaptureSession;
    private ImageReader imageReader;
    private ImageReader leftImageReader;
    private ImageReader rightImageReader;
    private CameraChoice activeCameraChoice;
    private StereoCameraChoice activeStereoChoice;
    private int requestedWidth;
    private int requestedHeight;
    private int preferredSquareSize;
    private int maxDimension;
    private int cpuUploadHz;
    private int cameraTargetFps;
    private int cameraFpsMin;
    private int cameraFpsMax;
    private String cameraTier;
    private String stereoLayout;
    private boolean allowCpuFallback;
    private boolean estimatedPose;
    private boolean estimatedStereoPose;
    private String estimatedPoseLabel;
    private String estimatedPoseVersion;
    private String poseCoordinateConvention;
    private float estimatedPoseX;
    private float estimatedPoseY;
    private float estimatedPoseZ;
    private float estimatedPoseQx;
    private float estimatedPoseQy;
    private float estimatedPoseQz;
    private float estimatedPoseQw = 1.0f;
    private float estimatedLeftPoseX;
    private float estimatedLeftPoseY;
    private float estimatedLeftPoseZ;
    private float estimatedLeftPoseQx;
    private float estimatedLeftPoseQy;
    private float estimatedLeftPoseQz;
    private float estimatedLeftPoseQw = 1.0f;
    private float estimatedRightPoseX;
    private float estimatedRightPoseY;
    private float estimatedRightPoseZ;
    private float estimatedRightPoseQx;
    private float estimatedRightPoseQy;
    private float estimatedRightPoseQz;
    private float estimatedRightPoseQw = 1.0f;
    private long stereoPairMaxDeltaNs = DEFAULT_STEREO_PAIR_MAX_DELTA_NS;
    private int activeImageFormat = ImageFormat.YUV_420_888;
    private long cpuFrameIntervalNs;
    private long lastDeliveredTimestampNs = Long.MIN_VALUE;
    private long frameIndex;
    private long gpuFrameIndex;
    private long gpuProbeFailureCount;
    private final ArrayDeque<PendingGpuImage> leftFrames = new ArrayDeque<PendingGpuImage>();
    private final ArrayDeque<PendingGpuImage> rightFrames = new ArrayDeque<PendingGpuImage>();
    private long stereoLeftReceivedCount;
    private long stereoRightReceivedCount;
    private long stereoPairedCount;
    private long stereoDroppedCount;
    private long stereoSoftPairOverMaxCount;
    private long stereoPairDeltaTotalNs;
    private long stereoPairDeltaMaxNs;
    private Range<Integer> monoAppliedAeFpsRange;
    private Range<Integer> logicalStereoAppliedAeFpsRange;
    private Range<Integer> leftAppliedAeFpsRange;
    private Range<Integer> rightAppliedAeFpsRange;
    private final DeliveryStats monoDeliveryStats = new DeliveryStats();
    private final DeliveryStats leftDeliveryStats = new DeliveryStats();
    private final DeliveryStats rightDeliveryStats = new DeliveryStats();
    private final DeliveryStats stereoPairDeliveryStats = new DeliveryStats();
    private boolean leftStereoSessionRunning;
    private boolean rightStereoSessionRunning;

    static {
        System.loadLibrary("rusty_xr_quest_composite_native");
    }

    private static native void nativeHeadsetCameraEvent(String eventJson);
    private static native void nativeHeadsetCameraFrame(
        int width,
        int height,
        long timestampNs,
        String metadataJson,
        byte[] rgba);
    private static native boolean nativeHeadsetCameraHardwareBufferFrame(
        int width,
        int height,
        long timestampNs,
        String metadataJson,
        HardwareBuffer buffer,
        int hardwareBufferFormat,
        long hardwareBufferUsage,
        int hardwareBufferLayers,
        long hardwareBufferId);
    private static native boolean nativeHeadsetStereoCameraHardwareBufferFrame(
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

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        requestedWidth = intent != null ? intent.getIntExtra(EXTRA_WIDTH, DEFAULT_PREFERRED_SQUARE_SIZE) : DEFAULT_PREFERRED_SQUARE_SIZE;
        requestedHeight = intent != null ? intent.getIntExtra(EXTRA_HEIGHT, DEFAULT_PREFERRED_SQUARE_SIZE) : DEFAULT_PREFERRED_SQUARE_SIZE;
        preferredSquareSize = intent != null ? intent.getIntExtra(EXTRA_PREFERRED_SQUARE_SIZE, DEFAULT_PREFERRED_SQUARE_SIZE) : DEFAULT_PREFERRED_SQUARE_SIZE;
        maxDimension = intent != null ? intent.getIntExtra(EXTRA_MAX_DIMENSION, DEFAULT_MAX_DIMENSION) : DEFAULT_MAX_DIMENSION;
        cpuUploadHz = intent != null ? intent.getIntExtra(EXTRA_CPU_UPLOAD_HZ, DEFAULT_CPU_UPLOAD_HZ) : DEFAULT_CPU_UPLOAD_HZ;
        cameraTargetFps = intent != null ? intent.getIntExtra(EXTRA_CAMERA_TARGET_FPS, 0) : 0;
        cameraFpsMin = intent != null ? intent.getIntExtra(EXTRA_CAMERA_FPS_MIN, 0) : 0;
        cameraFpsMax = intent != null ? intent.getIntExtra(EXTRA_CAMERA_FPS_MAX, 0) : 0;
        cameraTier = intent != null ? intent.getStringExtra(EXTRA_CAMERA_TIER) : TIER_CPU_DIAGNOSTIC;
        stereoLayout = intent != null ? intent.getStringExtra(EXTRA_STEREO_LAYOUT) : "mono";
        allowCpuFallback = intent == null || intent.getBooleanExtra(EXTRA_ALLOW_CPU_FALLBACK, true);
        estimatedPose = intent != null && intent.getBooleanExtra(EXTRA_ESTIMATED_POSE, false);
        estimatedStereoPose = intent != null && intent.getBooleanExtra(EXTRA_ESTIMATED_STEREO_POSE, false);
        estimatedPoseLabel = intent != null ? intent.getStringExtra(EXTRA_ESTIMATED_POSE_LABEL) : null;
        estimatedPoseVersion = intent != null ? intent.getStringExtra(EXTRA_ESTIMATED_POSE_VERSION) : null;
        poseCoordinateConvention = intent != null ? intent.getStringExtra(EXTRA_POSE_COORDINATE_CONVENTION) : null;
        estimatedPoseX = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_POSE_X, 0.0f) : 0.0f;
        estimatedPoseY = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_POSE_Y, 0.0f) : 0.0f;
        estimatedPoseZ = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_POSE_Z, 0.0f) : 0.0f;
        estimatedPoseQx = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_POSE_QX, 0.0f) : 0.0f;
        estimatedPoseQy = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_POSE_QY, 0.0f) : 0.0f;
        estimatedPoseQz = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_POSE_QZ, 0.0f) : 0.0f;
        estimatedPoseQw = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_POSE_QW, 1.0f) : 1.0f;
        estimatedLeftPoseX = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_LEFT_POSE_X, 0.0f) : 0.0f;
        estimatedLeftPoseY = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_LEFT_POSE_Y, 0.0f) : 0.0f;
        estimatedLeftPoseZ = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_LEFT_POSE_Z, 0.0f) : 0.0f;
        estimatedLeftPoseQx = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_LEFT_POSE_QX, 0.0f) : 0.0f;
        estimatedLeftPoseQy = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_LEFT_POSE_QY, 0.0f) : 0.0f;
        estimatedLeftPoseQz = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_LEFT_POSE_QZ, 0.0f) : 0.0f;
        estimatedLeftPoseQw = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_LEFT_POSE_QW, 1.0f) : 1.0f;
        estimatedRightPoseX = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_RIGHT_POSE_X, 0.0f) : 0.0f;
        estimatedRightPoseY = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_RIGHT_POSE_Y, 0.0f) : 0.0f;
        estimatedRightPoseZ = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_RIGHT_POSE_Z, 0.0f) : 0.0f;
        estimatedRightPoseQx = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_RIGHT_POSE_QX, 0.0f) : 0.0f;
        estimatedRightPoseQy = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_RIGHT_POSE_QY, 0.0f) : 0.0f;
        estimatedRightPoseQz = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_RIGHT_POSE_QZ, 0.0f) : 0.0f;
        estimatedRightPoseQw = intent != null ? intent.getFloatExtra(EXTRA_ESTIMATED_RIGHT_POSE_QW, 1.0f) : 1.0f;
        stereoPairMaxDeltaNs = intent != null ? intent.getLongExtra(EXTRA_STEREO_PAIR_MAX_DELTA_NS, DEFAULT_STEREO_PAIR_MAX_DELTA_NS) : DEFAULT_STEREO_PAIR_MAX_DELTA_NS;
        if (cameraTier == null || cameraTier.trim().isEmpty()) {
            cameraTier = TIER_CPU_DIAGNOSTIC;
        }
        if (stereoLayout == null || stereoLayout.trim().isEmpty()) {
            stereoLayout = "mono";
        }
        if (estimatedPoseLabel == null || estimatedPoseLabel.trim().isEmpty()) {
            estimatedPoseLabel = "launch-extra";
        }
        if (estimatedPoseVersion == null || estimatedPoseVersion.trim().isEmpty()) {
            estimatedPoseVersion = "unspecified";
        }
        if (poseCoordinateConvention == null || poseCoordinateConvention.trim().isEmpty()) {
            poseCoordinateConvention = CAMERA2_POSE_CONVENTION;
        }
        estimatedPose = estimatedPose && normalizeMonoEstimatedPose();
        estimatedStereoPose = estimatedStereoPose && normalizeEstimatedStereoPose();
        preferredSquareSize = Math.max(1, preferredSquareSize);
        maxDimension = Math.max(preferredSquareSize, maxDimension);
        cpuUploadHz = Math.max(0, cpuUploadHz);
        cameraTargetFps = Math.max(0, cameraTargetFps);
        cameraFpsMin = Math.max(0, cameraFpsMin);
        cameraFpsMax = Math.max(0, cameraFpsMax);
        stereoPairMaxDeltaNs = Math.max(1L, stereoPairMaxDeltaNs);
        cpuFrameIntervalNs = cpuUploadHz > 0 ? Math.max(1L, 1_000_000_000L / cpuUploadHz) : Long.MAX_VALUE;
        lastDeliveredTimestampNs = Long.MIN_VALUE;
        frameIndex = 0;
        gpuFrameIndex = 0;
        gpuProbeFailureCount = 0;
        stereoLeftReceivedCount = 0;
        stereoRightReceivedCount = 0;
        stereoPairedCount = 0;
        stereoDroppedCount = 0;
        stereoSoftPairOverMaxCount = 0;
        stereoPairDeltaTotalNs = 0;
        stereoPairDeltaMaxNs = 0;
        monoAppliedAeFpsRange = null;
        logicalStereoAppliedAeFpsRange = null;
        leftAppliedAeFpsRange = null;
        rightAppliedAeFpsRange = null;
        monoDeliveryStats.reset();
        leftDeliveryStats.reset();
        rightDeliveryStats.reset();
        stereoPairDeliveryStats.reset();
        leftFrames.clear();
        rightFrames.clear();

        if (checkSelfPermission(Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED ||
            checkSelfPermission(HEADSET_CAMERA_PERMISSION) != PackageManager.PERMISSION_GRANTED) {
            Log.w(TAG, "Headset camera permissions are not granted");
            sendNativeEvent("headsetCameraPermissionMissing");
            stopSelf();
            return START_NOT_STICKY;
        }

        cameraThread = new HandlerThread("RustyXrHeadsetCamera");
        cameraThread.start();
        cameraHandler = new Handler(cameraThread.getLooper());
        cameraHandler.post(new Runnable() {
            @Override
            public void run() {
                openCamera();
            }
        });

        return START_STICKY;
    }

    @Override
    public void onDestroy() {
        closeCamera();
        sendNativeEvent("headsetCameraStopped");
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    private void openCamera() {
        try {
            CameraManager manager = (CameraManager) getSystemService(Context.CAMERA_SERVICE);
            if (manager == null) {
                Log.e(TAG, "CameraManager is unavailable");
                sendNativeEvent("headsetCameraManagerUnavailable");
                stopSelf();
                return;
            }

            CameraSourceDiagnostics diagnostics = collectCameraSourceDiagnostics(manager);
            writeCameraSourceDiagnosticsFile(diagnostics.json);
            Log.i(TAG, "Rusty XR camera source diagnostics ready file=files/" +
                CAMERA_SOURCE_DIAGNOSTICS_FILE +
                " bytes=" + diagnostics.json.length() +
                " sources=" + diagnostics.sources.size() +
                " stereoCandidates=" + diagnostics.stereoCandidates.size() +
                " selectedProvider=" + (diagnostics.selectedStereoChoice != null
                    ? diagnostics.selectedStereoChoice.providerKind
                    : "none") +
                " fallbackReason=" + diagnostics.stereoFallbackReason);
            if (TIER_SOURCE_DIAGNOSTICS.equals(cameraTier)) {
                sendNativeEvent("headsetCameraSourceDiagnosticsComplete");
                stopSelf();
                return;
            }

            if (wantsProjectedTier()) {
                StereoCameraChoice stereoChoice = chooseStereoCamera(diagnostics);
                if (stereoChoice != null) {
                    openStereoCamera(manager, stereoChoice);
                    return;
                }

                Log.w(TAG, "No stereo Camera2 provider accepted for gpu-projected; falling back to gpu-buffer-probe diagnostics. reason=" + diagnostics.stereoFallbackReason);
                sendNativeEvent("headsetCameraStereoProviderMissing");
            }

            activeImageFormat = wantsGpuBufferTier() ? ImageFormat.PRIVATE : ImageFormat.YUV_420_888;
            CameraChoice choice = chooseCamera(manager, activeImageFormat);
            if (choice == null && activeImageFormat == ImageFormat.PRIVATE && allowCpuFallback) {
                Log.w(TAG, "No PRIVATE GPU-importable Camera2 source was found; falling back to CPU diagnostic YUV path");
                sendNativeEvent("headsetCameraGpuSourceMissingCpuFallback");
                activeImageFormat = ImageFormat.YUV_420_888;
                choice = chooseCamera(manager, activeImageFormat);
            }
            if (choice == null) {
                Log.e(TAG, "No Camera2 source was found for format " + imageFormatLabel(activeImageFormat));
                sendNativeEvent("headsetCameraNoSource");
                stopSelf();
                return;
            }

            imageReader = ImageReader.newInstance(
                choice.size.getWidth(),
                choice.size.getHeight(),
                activeImageFormat,
                3);
            imageReader.setOnImageAvailableListener(new ImageReader.OnImageAvailableListener() {
                @Override
                public void onImageAvailable(ImageReader reader) {
                    HeadsetCameraService.this.onImageAvailable(reader);
                }
            }, cameraHandler);

            Log.i(TAG, "Opening headset Camera2 source " + choice.cameraId + " at " +
                choice.size.getWidth() + "x" + choice.size.getHeight() +
                " activeTier=" + activeTierLabel() +
                " requestedTier=" + cameraTier +
                " imageFormat=" + imageFormatLabel(activeImageFormat) +
                " lensFacing=" + choice.lensFacingLabel +
                " score=" + choice.score +
                " preferredSquare=" + preferredSquareSize +
                " maxDimension=" + maxDimension +
                " cpuUploadHz=" + cpuUploadHz +
                " requestedAeFpsRange=" + rangeLabel(requestedCameraFpsRange()) +
                " allowCpuFallback=" + allowCpuFallback +
                " requestedStereoLayout=" + stereoLayout +
                " poseSource=" + monoPoseSource(choice) +
                " intrinsics=" + (choice.intrinsicCalibration != null ? "available" : "missing") +
                " activeArray=" + sizeLabel(choice.activeArraySize) +
                " sensorPixelArray=" + sizeLabel(choice.sensorPixelArraySize) +
                " sensorOrientation=" + optionalIntLabel(choice.sensorOrientationDegrees));
            activeCameraChoice = choice;
            sendNativeEvent("headsetCameraOpening");
            manager.openCamera(choice.cameraId, new CameraDevice.StateCallback() {
                @Override
                public void onOpened(CameraDevice device) {
                    cameraDevice = device;
                    startCaptureSession();
                }

                @Override
                public void onDisconnected(CameraDevice device) {
                    Log.w(TAG, "Headset camera disconnected");
                    sendNativeEvent("headsetCameraDisconnected");
                    closeCamera();
                    stopSelf();
                }

                @Override
                public void onError(CameraDevice device, int error) {
                    Log.e(TAG, "Headset camera error " + error);
                    sendNativeEvent("headsetCameraError" + error);
                    closeCamera();
                    stopSelf();
                }
            }, cameraHandler);
        } catch (CameraAccessException error) {
            Log.e(TAG, "Headset camera access failed", error);
            sendNativeEvent("headsetCameraAccessFailed");
            stopSelf();
        } catch (RuntimeException error) {
            Log.e(TAG, "Headset camera setup failed", error);
            sendNativeEvent("headsetCameraSetupFailed");
            stopSelf();
        }
    }

    private void writeCameraSourceDiagnosticsFile(String json) {
        File output = new File(getFilesDir(), CAMERA_SOURCE_DIAGNOSTICS_FILE);
        byte[] bytes = json.getBytes(StandardCharsets.UTF_8);
        try (FileOutputStream stream = new FileOutputStream(output, false)) {
            stream.write(bytes);
            stream.flush();
        } catch (IOException error) {
            Log.w(TAG, "Could not write camera source diagnostics file", error);
        }
    }

    private CameraChoice chooseCamera(CameraManager manager, int imageFormat) throws CameraAccessException {
        CameraChoice best = null;
        long bestScore = Long.MIN_VALUE;
        String[] cameraIds = manager.getCameraIdList();
        for (int i = 0; i < cameraIds.length; i++) {
            String cameraId = cameraIds[i];
            CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
            StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
            if (map == null) {
                continue;
            }

            Size[] sizes = map.getOutputSizes(imageFormat);
            if (sizes == null || sizes.length == 0) {
                continue;
            }

            for (int sizeIndex = 0; sizeIndex < sizes.length; sizeIndex++) {
                Size size = sizes[sizeIndex];
                long score = scoreSize(characteristics, size);
                if (score > bestScore) {
                    bestScore = score;
                    best = new CameraChoice(cameraId, size, score, characteristics);
                }
            }
        }

        return best;
    }

    private CameraSourceDiagnostics collectCameraSourceDiagnostics(CameraManager manager) throws CameraAccessException {
        String[] cameraIds = manager.getCameraIdList();
        List<CameraSourceInfo> sources = new ArrayList<CameraSourceInfo>();
        Set<String> concurrentMembers = new HashSet<String>();
        List<Set<String>> concurrentSets = new ArrayList<Set<String>>();
        if (Build.VERSION.SDK_INT >= 30) {
            Set<Set<String>> exposed = manager.getConcurrentCameraIds();
            if (exposed != null) {
                for (Set<String> set : exposed) {
                    if (set == null || set.isEmpty()) {
                        continue;
                    }
                    Set<String> copy = new HashSet<String>(set);
                    concurrentSets.add(copy);
                    concurrentMembers.addAll(copy);
                }
            }
        }

        for (int i = 0; i < cameraIds.length; i++) {
            String cameraId = cameraIds[i];
            CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
            StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
            CameraSourceInfo source = new CameraSourceInfo(cameraId, characteristics, map, concurrentMembers.contains(cameraId));
            sources.add(source);
        }

        List<StereoCandidateInfo> candidates = new ArrayList<StereoCandidateInfo>();
        StereoCameraChoice selected = null;
        for (int i = 0; i < sources.size(); i++) {
            CameraSourceInfo source = sources.get(i);
            if (source.logicalMultiCamera && source.physicalCameraIds.size() >= 2 && !source.privateSizes.isEmpty()) {
                List<String> physical = new ArrayList<String>(source.physicalCameraIds);
                Collections.sort(physical);
                Size size = choosePreferredSize(source.privateSizes);
                StereoCandidateInfo candidate = new StereoCandidateInfo(
                    "logical-physical",
                    physical.get(0),
                    physical.get(1),
                    true,
                    scoreStereoPair(source, size, source, size),
                    "logical multi-camera exposes at least two physical IDs and PRIVATE output");
                candidates.add(candidate);
                if (selected == null) {
                    selected = StereoCameraChoice.logicalPhysical(source.cameraId, physical.get(0), physical.get(1), size, source);
                }
            } else if (source.logicalMultiCamera) {
                candidates.add(new StereoCandidateInfo(
                    "logical-physical",
                    source.cameraId,
                    null,
                    false,
                    null,
                    "logical multi-camera did not expose at least two physical IDs with PRIVATE output"));
            } else {
                candidates.add(new StereoCandidateInfo(
                    "logical-physical",
                    source.cameraId,
                    null,
                    false,
                    null,
                    "not a logical multi-camera source"));
            }
        }

        StereoCameraChoice bestConcurrent = null;
        for (int left = 0; left < sources.size(); left++) {
            for (int right = left + 1; right < sources.size(); right++) {
                CameraSourceInfo leftSource = sources.get(left);
                CameraSourceInfo rightSource = sources.get(right);
                boolean canRunTogether = canRunConcurrently(concurrentSets, leftSource.cameraId, rightSource.cameraId);
                boolean hasBuffers = !leftSource.privateSizes.isEmpty() && !rightSource.privateSizes.isEmpty();
                boolean accepted = canRunTogether && hasBuffers;
                Long score = null;
                if (accepted) {
                    score = Long.valueOf(scoreStereoPair(
                        leftSource,
                        choosePreferredSize(leftSource.privateSizes),
                        rightSource,
                        choosePreferredSize(rightSource.privateSizes)));
                }
                candidates.add(new StereoCandidateInfo(
                    "concurrent-separate",
                    leftSource.cameraId,
                    rightSource.cameraId,
                    accepted,
                    score,
                    accepted
                        ? "concurrent separate Camera2 PRIVATE outputs accepted for paired acquisition"
                        : "separate cameras are not exposed as a concurrent PRIVATE pair"));
                if (accepted) {
                    StereoCameraChoice candidateChoice = StereoCameraChoice.concurrentSeparate(
                        leftSource,
                        rightSource,
                        choosePreferredSize(leftSource.privateSizes),
                        choosePreferredSize(rightSource.privateSizes));
                    if (bestConcurrent == null || candidateChoice.score > bestConcurrent.score) {
                        bestConcurrent = candidateChoice;
                    }
                }
            }
        }
        if (selected == null) {
            selected = bestConcurrent;
        }

        String fallbackReason = selected != null
            ? "selected " + selected.providerKind + " " + selected.leftPhysicalId + "/" + selected.rightPhysicalId
            : "no logical physical or concurrent separate stereo Camera2 provider accepted";
        String json = buildCameraSourceDiagnosticsJson(sources, candidates, selected, fallbackReason);
        return new CameraSourceDiagnostics(json, sources, candidates, selected, fallbackReason);
    }

    private static boolean canRunConcurrently(List<Set<String>> concurrentSets, String left, String right) {
        for (int i = 0; i < concurrentSets.size(); i++) {
            Set<String> set = concurrentSets.get(i);
            if (set.contains(left) && set.contains(right)) {
                return true;
            }
        }
        return false;
    }

    private Size choosePreferredSize(List<Size> sizes) {
        Size best = null;
        long bestScore = Long.MIN_VALUE;
        for (int i = 0; i < sizes.size(); i++) {
            Size size = sizes.get(i);
            long pixels = (long) size.getWidth() * (long) size.getHeight();
            long score = pixels;
            if (size.getWidth() == preferredSquareSize && size.getHeight() == preferredSquareSize) {
                score += 1_000_000_000L;
            }
            if (size.getWidth() == size.getHeight()) {
                score += 100_000_000L;
            }
            if (size.getWidth() > maxDimension || size.getHeight() > maxDimension) {
                score -= 2_000_000_000L;
            }
            if (score > bestScore) {
                bestScore = score;
                best = size;
            }
        }
        return best != null ? best : new Size(preferredSquareSize, preferredSquareSize);
    }

    private StereoCameraChoice chooseStereoCamera(CameraSourceDiagnostics diagnostics) {
        return diagnostics.selectedStereoChoice;
    }

    private static long scoreStereoPair(
        CameraSourceInfo leftSource,
        Size leftSize,
        CameraSourceInfo rightSource,
        Size rightSize) {
        long score = 0L;
        score += (long) (leftSource.lensFacingRank + rightSource.lensFacingRank) * 1_000_000_000_000L;
        score += (long) leftSize.getWidth() * (long) leftSize.getHeight();
        score += (long) rightSize.getWidth() * (long) rightSize.getHeight();
        if (leftSource.lensFacingLabel.equals(rightSource.lensFacingLabel)) {
            score += 100_000_000_000L;
        }
        if (leftSize.getWidth() == rightSize.getWidth() && leftSize.getHeight() == rightSize.getHeight()) {
            score += 50_000_000_000L;
        }
        if (leftSize.getWidth() == leftSize.getHeight() && rightSize.getWidth() == rightSize.getHeight()) {
            score += 10_000_000_000L;
        }
        return score;
    }

    private String buildCameraSourceDiagnosticsJson(
        List<CameraSourceInfo> sources,
        List<StereoCandidateInfo> candidates,
        StereoCameraChoice selected,
        String fallbackReason) {
        StringBuilder builder = new StringBuilder(4096);
        builder.append('{');
        appendJsonString(builder, "schemaVersion", "rusty.xr.camera-source-diagnostics.v1");
        builder.append(',');
        appendJsonString(builder, "requestedTier", cameraTier);
        builder.append(',');
        appendFpsRequest(builder);
        builder.append(",\"stereoPairPolicy\":\"latest-pair-soft-timestamp-target\"");
        builder.append(",\"stereoPairSoftTargetNs\":").append(stereoPairMaxDeltaNs);
        builder.append(',');
        appendJsonString(builder, "selectedProvider", selected != null ? selected.providerKind : "none");
        builder.append(',');
        appendJsonString(builder, "fallbackReason", fallbackReason);
        if (selected != null) {
            builder.append(",\"selectedStereoPair\":{");
            appendJsonString(builder, "providerKind", selected.providerKind);
            builder.append(',');
            appendJsonString(builder, "leftCameraId", selected.leftPhysicalId);
            builder.append(',');
            appendJsonString(builder, "rightCameraId", selected.rightPhysicalId);
            builder.append(",\"score\":").append(selected.score);
            builder.append(',');
            appendJsonString(builder, "reason", fallbackReason);
            builder.append('}');
            builder.append(",\"selectedStereoPairScore\":").append(selected.score);
            builder.append(',');
            appendJsonString(builder, "selectedStereoPairReason", fallbackReason);
        }
        builder.append(",\"sources\":[");
        for (int i = 0; i < sources.size(); i++) {
            if (i > 0) {
                builder.append(',');
            }
            appendCameraSourceInfo(builder, sources.get(i));
        }
        builder.append("],\"stereoCandidates\":[");
        for (int i = 0; i < candidates.size(); i++) {
            if (i > 0) {
                builder.append(',');
            }
            appendStereoCandidateInfo(builder, candidates.get(i));
        }
        builder.append("]}");
        return builder.toString();
    }

    private static void appendCameraSourceInfo(StringBuilder builder, CameraSourceInfo source) {
        builder.append('{');
        appendJsonString(builder, "cameraId", source.cameraId);
        builder.append(",\"physicalCameraIds\":[");
        for (int i = 0; i < source.physicalCameraIds.size(); i++) {
            if (i > 0) {
                builder.append(',');
            }
            builder.append('"');
            appendJsonEscaped(builder, source.physicalCameraIds.get(i));
            builder.append('"');
        }
        builder.append(']');
        builder.append(",\"logicalMultiCamera\":").append(source.logicalMultiCamera);
        builder.append(",\"concurrentCamera\":").append(source.concurrentCamera);
        builder.append(',');
        appendJsonString(builder, "lensFacing", source.lensFacingLabel);
        builder.append(',');
        appendJsonString(builder, "hardwareLevel", source.hardwareLevelLabel);
        if (source.sensorOrientationDegrees != null) {
            builder.append(",\"sensorOrientationDegrees\":").append(source.sensorOrientationDegrees.intValue());
        }
        appendSizeObject(builder, "activeArray", source.activeArraySize);
        appendSizeObject(builder, "sensorPixelArray", source.sensorPixelArraySize);
        appendSizeArray(builder, "privateOutputSizes", source.privateSizes);
        appendSizeArray(builder, "yuvOutputSizes", source.yuvSizes);
        builder.append(",\"outputFormats\":[");
        for (int i = 0; i < source.outputFormats.length; i++) {
            if (i > 0) {
                builder.append(',');
            }
            appendJsonStringValue(builder, imageFormatLabel(source.outputFormats[i]));
        }
        builder.append(']');
        builder.append(",\"fpsRanges\":[");
        for (int i = 0; i < source.fpsRanges.length; i++) {
            if (i > 0) {
                builder.append(',');
            }
            Range<Integer> range = source.fpsRanges[i];
            builder.append("{\"min\":").append(range.getLower().intValue());
            builder.append(",\"max\":").append(range.getUpper().intValue()).append('}');
        }
        builder.append(']');
        builder.append(",\"intrinsicsAvailable\":").append(source.intrinsicCalibration != null);
        appendFloatArray(builder, "intrinsicCalibration", source.intrinsicCalibration);
        builder.append(",\"distortionAvailable\":").append(source.distortion != null);
        appendFloatArray(builder, "distortion", source.distortion);
        builder.append(",\"lensPoseAvailable\":").append(source.lensPoseTranslation != null && source.lensPoseRotation != null);
        appendFloatArray(builder, "lensPoseTranslation", source.lensPoseTranslation);
        appendFloatArray(builder, "lensPoseRotation", source.lensPoseRotation);
        appendFloatArray(builder, "lensPoseRotationNormalized", normalizeQuaternionOrNull(source.lensPoseRotation));
        if (source.lensPoseReference != null) {
            builder.append(",\"lensPoseReference\":").append(source.lensPoseReference.intValue());
            builder.append(',');
            appendJsonString(builder, "lensPoseReferenceLabel", lensPoseReferenceLabel(source.lensPoseReference));
        }
        builder.append(",\"lensPoseUsableForProjection\":").append(hasUsablePlatformPose(source));
        builder.append('}');
    }

    private static void appendStereoCandidateInfo(StringBuilder builder, StereoCandidateInfo candidate) {
        builder.append('{');
        appendJsonString(builder, "providerKind", candidate.providerKind);
        if (candidate.leftCameraId != null) {
            builder.append(',');
            appendJsonString(builder, "leftCameraId", candidate.leftCameraId);
        }
        if (candidate.rightCameraId != null) {
            builder.append(',');
            appendJsonString(builder, "rightCameraId", candidate.rightCameraId);
        }
        builder.append(",\"accepted\":").append(candidate.accepted);
        if (candidate.score != null) {
            builder.append(",\"score\":").append(candidate.score.longValue());
        }
        builder.append(',');
        appendJsonString(builder, "reason", candidate.reason);
        builder.append('}');
    }

    private static void appendSizeArray(StringBuilder builder, String key, List<Size> sizes) {
        builder.append(",\"").append(key).append("\":[");
        for (int i = 0; i < sizes.size(); i++) {
            if (i > 0) {
                builder.append(',');
            }
            Size size = sizes.get(i);
            builder.append("{\"width\":").append(size.getWidth());
            builder.append(",\"height\":").append(size.getHeight()).append('}');
        }
        builder.append(']');
    }

    private static void appendSizeObject(StringBuilder builder, String key, Rect rect) {
        if (rect == null) {
            return;
        }
        builder.append(",\"").append(key).append("\":{\"width\":").append(rect.width());
        builder.append(",\"height\":").append(rect.height()).append('}');
    }

    private static void appendSizeObject(StringBuilder builder, String key, Size size) {
        if (size == null) {
            return;
        }
        builder.append(",\"").append(key).append("\":{\"width\":").append(size.getWidth());
        builder.append(",\"height\":").append(size.getHeight()).append('}');
    }

    private long scoreSize(CameraCharacteristics characteristics, Size size) {
        return scoreSize(characteristics, size.getWidth(), size.getHeight());
    }

    private long scoreSize(CameraCharacteristics characteristics, int width, int height) {
        int targetWidth = Math.max(1, Math.max(requestedWidth, preferredSquareSize));
        int targetHeight = Math.max(1, Math.max(requestedHeight, preferredSquareSize));
        int cap = Math.max(maxDimension, Math.max(targetWidth, targetHeight));
        long pixels = (long) width * (long) height;
        long capPixels = (long) cap * (long) cap;
        long score = 0;
        score += lensFacingRank(characteristics) * 1_000_000_000_000L;
        if (width == preferredSquareSize && height == preferredSquareSize) {
            score += 100_000_000_000L;
        }
        if (width <= cap && height <= cap) {
            score += 10_000_000_000L;
        } else {
            score -= 10_000_000_000L;
        }
        if (width == height) {
            score += 1_000_000_000L;
        }

        score += Math.min(pixels, capPixels) / 16L;
        score -= Math.abs(width - targetWidth) * 5_000L;
        score -= Math.abs(height - targetHeight) * 5_000L;
        score += maxFrameRateMillihz(characteristics);
        return score;
    }

    private static long lensFacingRank(CameraCharacteristics characteristics) {
        Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
        if (facing == null) {
            return 0L;
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_BACK) {
            return 2L;
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return 1L;
        }
        return 0L;
    }

    private static String lensFacingLabel(CameraCharacteristics characteristics) {
        Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
        if (facing == null) {
            return "unknown";
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_BACK) {
            return "back";
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_FRONT) {
            return "front";
        }
        if (facing.intValue() == CameraCharacteristics.LENS_FACING_EXTERNAL) {
            return "external";
        }
        return "other";
    }

    private static String sizeLabel(Rect rect) {
        return rect != null ? rect.width() + "x" + rect.height() : "missing";
    }

    private static String sizeLabel(Size size) {
        return size != null ? size.getWidth() + "x" + size.getHeight() : "missing";
    }

    private static String optionalIntLabel(Integer value) {
        return value != null ? value.toString() : "missing";
    }

    private long maxFrameRateMillihz(CameraCharacteristics characteristics) {
        Range<Integer>[] ranges = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES);
        if (ranges == null || ranges.length == 0) {
            return 0L;
        }

        int best = 0;
        for (int i = 0; i < ranges.length; i++) {
            Range<Integer> range = ranges[i];
            if (range != null && range.getUpper() != null) {
                best = Math.max(best, range.getUpper().intValue());
            }
        }
        return best * 1000L;
    }

    private Range<Integer> requestedCameraFpsRange() {
        int min = cameraFpsMin;
        int max = cameraFpsMax;
        if (min <= 0 && max <= 0 && cameraTargetFps > 0) {
            min = cameraTargetFps;
            max = cameraTargetFps;
        } else if (cameraTargetFps > 0) {
            if (min <= 0) {
                min = cameraTargetFps;
            }
            if (max <= 0) {
                max = cameraTargetFps;
            }
        }
        if (min <= 0 && max <= 0) {
            return null;
        }
        if (min <= 0) {
            min = max;
        }
        if (max <= 0) {
            max = min;
        }
        if (min > max) {
            int tmp = min;
            min = max;
            max = tmp;
        }
        return new Range<Integer>(Integer.valueOf(min), Integer.valueOf(max));
    }

    private Range<Integer> selectAeTargetFpsRange(Range<Integer>[] supported) {
        Range<Integer> requested = requestedCameraFpsRange();
        if (requested == null || supported == null || supported.length == 0) {
            return null;
        }

        Range<Integer> best = null;
        long bestScore = Long.MIN_VALUE;
        int requestedMin = requested.getLower().intValue();
        int requestedMax = requested.getUpper().intValue();
        boolean fixedRequest = requestedMin == requestedMax;
        for (int i = 0; i < supported.length; i++) {
            Range<Integer> range = supported[i];
            if (range == null || range.getLower() == null || range.getUpper() == null) {
                continue;
            }

            int lower = range.getLower().intValue();
            int upper = range.getUpper().intValue();
            long score = 0L;
            score -= Math.abs(lower - requestedMin) * 10_000L;
            score -= Math.abs(upper - requestedMax) * 10_000L;
            if (lower <= requestedMin && upper >= requestedMax) {
                score += 1_000_000_000L;
            }
            if (fixedRequest && lower <= requestedMin && upper >= requestedMin) {
                score += 500_000_000L;
                score -= (long) (upper - lower) * 1_000L;
            }
            if (lower == requestedMin && upper == requestedMax) {
                score += 2_000_000_000L;
            }
            score += upper;
            if (best == null || score > bestScore) {
                best = range;
                bestScore = score;
            }
        }
        return best;
    }

    private Range<Integer> applyAeTargetFpsRange(
        CaptureRequest.Builder builder,
        Range<Integer>[] supported,
        String streamLabel) {
        Range<Integer> requested = requestedCameraFpsRange();
        if (requested == null) {
            Log.i(TAG, "Camera2 AE FPS range " + streamLabel +
                " requested=device-controlled supported=" + rangeArrayLabel(supported));
            return null;
        }

        Range<Integer> selected = selectAeTargetFpsRange(supported);
        if (selected == null) {
            Log.w(TAG, "Camera2 AE FPS range " + streamLabel +
                " requested=" + rangeLabel(requested) +
                " selected=none supported=" + rangeArrayLabel(supported));
            return null;
        }

        builder.set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, selected);
        Log.i(TAG, "Camera2 AE FPS range " + streamLabel +
            " requested=" + rangeLabel(requested) +
            " selected=" + rangeLabel(selected) +
            " supported=" + rangeArrayLabel(supported));
        return selected;
    }

    private void startCaptureSession() {
        if (cameraDevice == null || imageReader == null) {
            return;
        }

        try {
            final Surface surface = imageReader.getSurface();
            cameraDevice.createCaptureSession(
                Arrays.asList(surface),
                new CameraCaptureSession.StateCallback() {
                    @Override
                    public void onConfigured(CameraCaptureSession session) {
                        captureSession = session;
                        try {
                            CaptureRequest.Builder builder =
                                cameraDevice.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                            builder.addTarget(surface);
                            CameraChoice choice = activeCameraChoice;
                            monoAppliedAeFpsRange = applyAeTargetFpsRange(
                                builder,
                                choice != null ? choice.fpsRanges : null,
                                "mono cameraId=" + (choice != null ? choice.cameraId : "unknown"));
                            session.setRepeatingRequest(builder.build(), null, cameraHandler);
                            Log.i(TAG, "Headset camera capture session running");
                            sendNativeEvent("headsetCameraRunning");
                        } catch (CameraAccessException error) {
                            Log.e(TAG, "Could not start headset camera repeating request", error);
                            sendNativeEvent("headsetCameraRepeatingFailed");
                            stopSelf();
                        }
                    }

                    @Override
                    public void onConfigureFailed(CameraCaptureSession session) {
                        Log.e(TAG, "Headset camera capture session configure failed");
                        sendNativeEvent("headsetCameraConfigureFailed");
                        stopSelf();
                    }
                },
                cameraHandler);
        } catch (CameraAccessException error) {
            Log.e(TAG, "Could not create headset camera capture session", error);
            sendNativeEvent("headsetCameraSessionFailed");
            stopSelf();
        }
    }

    private void openStereoCamera(CameraManager manager, final StereoCameraChoice choice) throws CameraAccessException {
        activeImageFormat = ImageFormat.PRIVATE;
        activeStereoChoice = choice;
        leftImageReader = ImageReader.newInstance(
            choice.leftSize.getWidth(),
            choice.leftSize.getHeight(),
            ImageFormat.PRIVATE,
            STEREO_IMAGE_READER_MAX_IMAGES);
        rightImageReader = ImageReader.newInstance(
            choice.rightSize.getWidth(),
            choice.rightSize.getHeight(),
            ImageFormat.PRIVATE,
            STEREO_IMAGE_READER_MAX_IMAGES);
        leftImageReader.setOnImageAvailableListener(new ImageReader.OnImageAvailableListener() {
            @Override
            public void onImageAvailable(ImageReader reader) {
                HeadsetCameraService.this.onStereoImageAvailable(reader, true);
            }
        }, cameraHandler);
        rightImageReader.setOnImageAvailableListener(new ImageReader.OnImageAvailableListener() {
            @Override
            public void onImageAvailable(ImageReader reader) {
                HeadsetCameraService.this.onStereoImageAvailable(reader, false);
            }
        }, cameraHandler);

        Log.i(TAG, "Opening stereo Camera2 provider kind=" + choice.providerKind +
            " logicalId=" + (choice.logicalCameraId != null ? choice.logicalCameraId : "none") +
            " leftPhysicalId=" + choice.leftPhysicalId +
            " rightPhysicalId=" + choice.rightPhysicalId +
            " leftSize=" + choice.leftSize.getWidth() + "x" + choice.leftSize.getHeight() +
            " rightSize=" + choice.rightSize.getWidth() + "x" + choice.rightSize.getHeight() +
            " requestedTier=" + cameraTier +
            " activeTier=" + activeTierLabel() +
            " stereoLayout=separate" +
            " poseSource=" + stereoPoseSourceLabel(choice) +
            " pairMaxDeltaNs=" + stereoPairMaxDeltaNs +
            " score=" + choice.score);
        sendNativeEvent("headsetCameraStereoOpening");

        if ("concurrent-separate".equals(choice.providerKind)) {
            openConcurrentStereoCameras(manager, choice);
            return;
        }

        if (Build.VERSION.SDK_INT < 28) {
            Log.w(TAG, "Stereo logical physical Camera2 output needs API 28+");
            sendNativeEvent("headsetCameraStereoApiUnavailable");
            return;
        }

        manager.openCamera(choice.logicalCameraId, new CameraDevice.StateCallback() {
            @Override
            public void onOpened(CameraDevice device) {
                cameraDevice = device;
                startStereoCaptureSession(choice);
            }

            @Override
            public void onDisconnected(CameraDevice device) {
                Log.w(TAG, "Stereo headset camera disconnected");
                sendNativeEvent("headsetCameraStereoDisconnected");
                closeCamera();
                stopSelf();
            }

            @Override
            public void onError(CameraDevice device, int error) {
                Log.e(TAG, "Stereo headset camera error " + error);
                sendNativeEvent("headsetCameraStereoError" + error);
                closeCamera();
                stopSelf();
            }
        }, cameraHandler);
    }

    private void openConcurrentStereoCameras(CameraManager manager, final StereoCameraChoice choice) throws CameraAccessException {
        leftStereoSessionRunning = false;
        rightStereoSessionRunning = false;
        manager.openCamera(choice.leftPhysicalId, new CameraDevice.StateCallback() {
            @Override
            public void onOpened(CameraDevice device) {
                leftCameraDevice = device;
                startConcurrentStereoSessionIfReady(choice);
            }

            @Override
            public void onDisconnected(CameraDevice device) {
                Log.w(TAG, "Left concurrent stereo camera disconnected");
                sendNativeEvent("headsetCameraStereoLeftDisconnected");
                closeCamera();
                stopSelf();
            }

            @Override
            public void onError(CameraDevice device, int error) {
                Log.e(TAG, "Left concurrent stereo camera error " + error);
                sendNativeEvent("headsetCameraStereoLeftError" + error);
                closeCamera();
                stopSelf();
            }
        }, cameraHandler);

        manager.openCamera(choice.rightPhysicalId, new CameraDevice.StateCallback() {
            @Override
            public void onOpened(CameraDevice device) {
                rightCameraDevice = device;
                startConcurrentStereoSessionIfReady(choice);
            }

            @Override
            public void onDisconnected(CameraDevice device) {
                Log.w(TAG, "Right concurrent stereo camera disconnected");
                sendNativeEvent("headsetCameraStereoRightDisconnected");
                closeCamera();
                stopSelf();
            }

            @Override
            public void onError(CameraDevice device, int error) {
                Log.e(TAG, "Right concurrent stereo camera error " + error);
                sendNativeEvent("headsetCameraStereoRightError" + error);
                closeCamera();
                stopSelf();
            }
        }, cameraHandler);
    }

    private void startConcurrentStereoSessionIfReady(final StereoCameraChoice choice) {
        if (leftCameraDevice == null || rightCameraDevice == null || leftImageReader == null || rightImageReader == null) {
            return;
        }

        if (!leftStereoSessionRunning && leftCaptureSession == null) {
            startSingleConcurrentStereoSession(choice, true);
        }
        if (!rightStereoSessionRunning && rightCaptureSession == null) {
            startSingleConcurrentStereoSession(choice, false);
        }
    }

    private void startSingleConcurrentStereoSession(final StereoCameraChoice choice, final boolean leftEye) {
        final CameraDevice device = leftEye ? leftCameraDevice : rightCameraDevice;
        final ImageReader reader = leftEye ? leftImageReader : rightImageReader;
        if (device == null || reader == null) {
            return;
        }

        try {
            final Surface surface = reader.getSurface();
            device.createCaptureSession(
                Arrays.asList(surface),
                new CameraCaptureSession.StateCallback() {
                    @Override
                    public void onConfigured(CameraCaptureSession session) {
                        if (leftEye) {
                            leftCaptureSession = session;
                        } else {
                            rightCaptureSession = session;
                        }
                        try {
                            CaptureRequest.Builder builder =
                                device.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                            builder.addTarget(surface);
                            CameraSourceInfo source = leftEye ? choice.leftSource : choice.rightSource;
                            Range<Integer> appliedRange = applyAeTargetFpsRange(
                                builder,
                                source != null ? source.fpsRanges : null,
                                (leftEye ? "stereo-left" : "stereo-right") +
                                    " cameraId=" + (leftEye ? choice.leftPhysicalId : choice.rightPhysicalId));
                            if (leftEye) {
                                leftAppliedAeFpsRange = appliedRange;
                            } else {
                                rightAppliedAeFpsRange = appliedRange;
                            }
                            session.setRepeatingRequest(builder.build(), null, cameraHandler);
                            if (leftEye) {
                                leftStereoSessionRunning = true;
                            } else {
                                rightStereoSessionRunning = true;
                            }
                            Log.i(TAG, "Concurrent stereo Camera2 " + (leftEye ? "left" : "right") +
                                " stream running cameraId=" + (leftEye ? choice.leftPhysicalId : choice.rightPhysicalId));
                            if (leftStereoSessionRunning && rightStereoSessionRunning) {
                                Log.i(TAG, "Concurrent separate stereo headset camera capture running provider=" + choice.providerKind);
                                sendNativeEvent("headsetCameraStereoRunning");
                            }
                        } catch (CameraAccessException error) {
                            Log.e(TAG, "Could not start concurrent stereo repeating request eye=" + (leftEye ? "left" : "right"), error);
                            sendNativeEvent("headsetCameraStereoRepeatingFailed");
                            stopSelf();
                        }
                    }

                    @Override
                    public void onConfigureFailed(CameraCaptureSession session) {
                        Log.e(TAG, "Concurrent stereo capture session configure failed eye=" + (leftEye ? "left" : "right"));
                        sendNativeEvent("headsetCameraStereoConfigureFailed");
                        stopSelf();
                    }
                },
                cameraHandler);
        } catch (CameraAccessException error) {
            Log.e(TAG, "Could not create concurrent stereo capture session eye=" + (leftEye ? "left" : "right"), error);
            sendNativeEvent("headsetCameraStereoSessionFailed");
            stopSelf();
        }
    }

    private void startStereoCaptureSession(final StereoCameraChoice choice) {
        if (Build.VERSION.SDK_INT < 28 || cameraDevice == null || leftImageReader == null || rightImageReader == null) {
            return;
        }

        try {
            final Surface leftSurface = leftImageReader.getSurface();
            final Surface rightSurface = rightImageReader.getSurface();
            OutputConfiguration leftOutput = new OutputConfiguration(leftSurface);
            leftOutput.setPhysicalCameraId(choice.leftPhysicalId);
            OutputConfiguration rightOutput = new OutputConfiguration(rightSurface);
            rightOutput.setPhysicalCameraId(choice.rightPhysicalId);
            List<OutputConfiguration> outputs = Arrays.asList(leftOutput, rightOutput);
            SessionConfiguration sessionConfiguration = new SessionConfiguration(
                SessionConfiguration.SESSION_REGULAR,
                outputs,
                new HandlerExecutor(cameraHandler),
                new CameraCaptureSession.StateCallback() {
                    @Override
                    public void onConfigured(CameraCaptureSession session) {
                        captureSession = session;
                        try {
                            CaptureRequest.Builder builder =
                                cameraDevice.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
                            builder.addTarget(leftSurface);
                            builder.addTarget(rightSurface);
                            logicalStereoAppliedAeFpsRange = applyAeTargetFpsRange(
                                builder,
                                choice.leftSource != null ? choice.leftSource.fpsRanges : null,
                                "stereo-logical cameraId=" + choice.logicalCameraId);
                            session.setRepeatingRequest(builder.build(), null, cameraHandler);
                            Log.i(TAG, "Stereo headset camera capture session running provider=" + choice.providerKind);
                            sendNativeEvent("headsetCameraStereoRunning");
                        } catch (CameraAccessException error) {
                            Log.e(TAG, "Could not start stereo headset camera repeating request", error);
                            sendNativeEvent("headsetCameraStereoRepeatingFailed");
                            stopSelf();
                        }
                    }

                    @Override
                    public void onConfigureFailed(CameraCaptureSession session) {
                        Log.e(TAG, "Stereo headset camera capture session configure failed");
                        sendNativeEvent("headsetCameraStereoConfigureFailed");
                        stopSelf();
                    }
                });
            cameraDevice.createCaptureSession(sessionConfiguration);
        } catch (CameraAccessException error) {
            Log.e(TAG, "Could not create stereo headset camera capture session", error);
            sendNativeEvent("headsetCameraStereoSessionFailed");
            stopSelf();
        } catch (RuntimeException error) {
            Log.e(TAG, "Stereo headset camera capture setup failed", error);
            sendNativeEvent("headsetCameraStereoSetupFailed");
            stopSelf();
        }
    }

    private void onImageAvailable(ImageReader reader) {
        Image image = null;
        try {
            image = reader.acquireLatestImage();
            if (image == null) {
                return;
            }

            monoDeliveryStats.record(image.getTimestamp());
            maybeLogDeliveryStats(
                "mono",
                activeCameraChoice != null ? activeCameraChoice.cameraId : "unknown",
                monoDeliveryStats,
                monoAppliedAeFpsRange,
                60);

            if (activeImageFormat == ImageFormat.PRIVATE) {
                deliverGpuHardwareBufferFrame(image);
                return;
            }

            if (!shouldDeliverCpuFrame(image.getTimestamp())) {
                return;
            }

            byte[] rgba = yuv420ToRgba(image);
            CameraChoice choice = activeCameraChoice;
            String metadataJson = buildFrameMetadataJson(
                choice,
                image,
                "cpu-yuv-rgba",
                activeTierLabel(),
                true,
                "missing camera pose; diagnostic flat camera copy");
            nativeHeadsetCameraFrame(
                image.getWidth(),
                image.getHeight(),
                image.getTimestamp(),
                metadataJson,
                rgba);
            lastDeliveredTimestampNs = image.getTimestamp();
            long index = frameIndex++;
            if (index == 0 || index % 30 == 0) {
                Log.i(TAG, "Headset camera frame " + index +
                    " source=" + (choice != null ? choice.cameraId : "unknown") +
                    " " + image.getWidth() + "x" + image.getHeight() +
                    " metadataIntrinsics=" + (choice != null && choice.intrinsicCalibration != null ? "available" : "missing") +
                    " metadataPose=" + monoPoseSource(choice) +
                    " monoFallback=true");
            }
        } catch (RuntimeException error) {
            Log.e(TAG, "Could not deliver headset camera frame", error);
            sendNativeEvent("headsetCameraFrameFailed");
        } finally {
            if (image != null) {
                image.close();
            }
        }
    }

    private void deliverGpuHardwareBufferFrame(Image image) {
        HardwareBuffer buffer = null;
        try {
            buffer = image.getHardwareBuffer();
            if (buffer == null) {
                gpuProbeFailureCount++;
                Log.w(TAG, "Camera2 PRIVATE image did not expose a HardwareBuffer");
                sendNativeEvent("headsetCameraHardwareBufferMissing");
                return;
            }

            long bufferId = 0L;
            if (Build.VERSION.SDK_INT >= 34) {
                bufferId = buffer.getId();
            }
            String metadataJson = buildFrameMetadataJson(
                activeCameraChoice,
                image,
                "android-hardware-buffer",
                activeTierLabel(),
                true,
                "gpu-buffer probe active; metadata-backed projection or stereo pose unavailable");
            boolean accepted = nativeHeadsetCameraHardwareBufferFrame(
                image.getWidth(),
                image.getHeight(),
                image.getTimestamp(),
                metadataJson,
                buffer,
                buffer.getFormat(),
                buffer.getUsage(),
                buffer.getLayers(),
                bufferId);
            long index = gpuFrameIndex++;
            if (!accepted) {
                gpuProbeFailureCount++;
            }
            if (index == 0 || index % 120 == 0 || !accepted) {
                Log.i(TAG, "Headset camera GPU buffer frame " + index +
                    " accepted=" + accepted +
                    " source=" + (activeCameraChoice != null ? activeCameraChoice.cameraId : "unknown") +
                    " " + image.getWidth() + "x" + image.getHeight() +
                    " bufferFormat=" + buffer.getFormat() +
                    " bufferUsage=" + buffer.getUsage() +
                    " layers=" + buffer.getLayers() +
                    " bufferId=" + bufferId +
                    " metadataIntrinsics=" + (activeCameraChoice != null && activeCameraChoice.intrinsicCalibration != null ? "available" : "missing") +
                    " metadataPose=" + monoPoseSource(activeCameraChoice) +
                    " stereoLayout=mono requestedStereoLayout=" + stereoLayout +
                    " failures=" + gpuProbeFailureCount);
            }
        } catch (RuntimeException error) {
            gpuProbeFailureCount++;
            Log.e(TAG, "Could not deliver headset camera GPU buffer frame", error);
            sendNativeEvent("headsetCameraHardwareBufferFrameFailed");
        } finally {
            if (buffer != null) {
                buffer.close();
            }
        }
    }

    private void onStereoImageAvailable(ImageReader reader, boolean leftEye) {
        Image image = null;
        HardwareBuffer buffer = null;
        try {
            image = reader.acquireLatestImage();
            if (image == null) {
                return;
            }
            DeliveryStats stats = leftEye ? leftDeliveryStats : rightDeliveryStats;
            stats.record(image.getTimestamp());
            Range<Integer> appliedRange = leftEye ? leftAppliedAeFpsRange : rightAppliedAeFpsRange;
            if (appliedRange == null && logicalStereoAppliedAeFpsRange != null) {
                appliedRange = logicalStereoAppliedAeFpsRange;
            }
            maybeLogDeliveryStats(
                leftEye ? "stereo-left" : "stereo-right",
                activeStereoChoice != null
                    ? (leftEye ? activeStereoChoice.leftPhysicalId : activeStereoChoice.rightPhysicalId)
                    : "unknown",
                stats,
                appliedRange,
                120);
            buffer = image.getHardwareBuffer();
            if (buffer == null) {
                stereoDroppedCount++;
                Log.w(TAG, "Stereo Camera2 PRIVATE image did not expose a HardwareBuffer eye=" + (leftEye ? "left" : "right"));
                return;
            }

            long bufferId = 0L;
            if (Build.VERSION.SDK_INT >= 34) {
                bufferId = buffer.getId();
            }
            PendingGpuImage pending = new PendingGpuImage(
                leftEye,
                image.getWidth(),
                image.getHeight(),
                image.getTimestamp(),
                buildStereoFrameMetadataJson(leftEye, image),
                buffer,
                buffer.getFormat(),
                buffer.getUsage(),
                buffer.getLayers(),
                bufferId);
            buffer = null;
            enqueueStereoFrame(pending);
            if (leftEye) {
                stereoLeftReceivedCount++;
            } else {
                stereoRightReceivedCount++;
            }
            tryPairStereoFrames();
        } catch (RuntimeException error) {
            stereoDroppedCount++;
            Log.e(TAG, "Could not deliver stereo headset camera GPU buffer frame", error);
            sendNativeEvent("headsetCameraStereoFrameFailed");
        } finally {
            if (image != null) {
                image.close();
            }
            if (buffer != null) {
                buffer.close();
            }
        }
    }

    private void enqueueStereoFrame(PendingGpuImage frame) {
        ArrayDeque<PendingGpuImage> queue = frame.leftEye ? leftFrames : rightFrames;
        queue.addLast(frame);
        while (queue.size() > STEREO_PENDING_QUEUE_LIMIT) {
            PendingGpuImage dropped = queue.removeFirst();
            dropped.close();
            stereoDroppedCount++;
        }
    }

    private void tryPairStereoFrames() {
        while (!leftFrames.isEmpty() && !rightFrames.isEmpty()) {
            PendingGpuImage bestLeft = null;
            PendingGpuImage bestRight = null;
            long bestDelta = Long.MAX_VALUE;
            for (PendingGpuImage left : leftFrames) {
                for (PendingGpuImage right : rightFrames) {
                    long delta = Math.abs(left.timestampNs - right.timestampNs);
                    if (delta < bestDelta) {
                        bestDelta = delta;
                        bestLeft = left;
                        bestRight = right;
                    }
                }
            }

            if (bestLeft != null && bestRight != null && bestDelta <= stereoPairMaxDeltaNs) {
                leftFrames.remove(bestLeft);
                rightFrames.remove(bestRight);
                deliverStereoPair(bestLeft, bestRight, bestDelta);
                continue;
            }

            PendingGpuImage latestLeft = leftFrames.removeLast();
            PendingGpuImage latestRight = rightFrames.removeLast();
            stereoDroppedCount += closePendingQueue(leftFrames);
            stereoDroppedCount += closePendingQueue(rightFrames);
            long latestDelta = Math.abs(latestLeft.timestampNs - latestRight.timestampNs);
            if (latestDelta > stereoPairMaxDeltaNs) {
                stereoSoftPairOverMaxCount++;
                if (stereoSoftPairOverMaxCount == 1 || stereoSoftPairOverMaxCount % 120 == 0) {
                    Log.w(TAG, "Stereo headset camera pair exceeded soft timestamp target" +
                        " deltaNs=" + latestDelta +
                        " softTargetNs=" + stereoPairMaxDeltaNs +
                        " overSoftTarget=" + stereoSoftPairOverMaxCount +
                        " leftTs=" + latestLeft.timestampNs +
                        " rightTs=" + latestRight.timestampNs +
                        " dropped=" + stereoDroppedCount);
                }
            }
            deliverStereoPair(latestLeft, latestRight, latestDelta);
        }
    }

    private long closePendingQueue(ArrayDeque<PendingGpuImage> queue) {
        long closed = 0;
        while (!queue.isEmpty()) {
            queue.removeFirst().close();
            closed++;
        }
        return closed;
    }

    private void deliverStereoPair(PendingGpuImage left, PendingGpuImage right, long pairDeltaNs) {
        try {
            long pairIndex = stereoPairedCount;
            long midpointTs = left.timestampNs / 2L + right.timestampNs / 2L;
            stereoPairDeliveryStats.record(midpointTs);
            boolean accepted = nativeHeadsetStereoCameraHardwareBufferFrame(
                left.width,
                left.height,
                left.timestampNs,
                left.metadataJson,
                left.buffer,
                left.hardwareBufferFormat,
                left.hardwareBufferUsage,
                left.hardwareBufferLayers,
                left.hardwareBufferId,
                right.width,
                right.height,
                right.timestampNs,
                right.metadataJson,
                right.buffer,
                right.hardwareBufferFormat,
                right.hardwareBufferUsage,
                right.hardwareBufferLayers,
                right.hardwareBufferId,
                pairDeltaNs,
                pairIndex);
            if (!accepted) {
                stereoDroppedCount++;
            }
            stereoPairedCount++;
            stereoPairDeltaTotalNs += pairDeltaNs;
            stereoPairDeltaMaxNs = Math.max(stereoPairDeltaMaxNs, pairDeltaNs);
            if (pairIndex == 0 || pairIndex % 120 == 0 || !accepted) {
                long avgDelta = stereoPairedCount > 0 ? stereoPairDeltaTotalNs / stereoPairedCount : 0L;
                Log.i(TAG, "Stereo headset camera pair " + pairIndex +
                    " accepted=" + accepted +
                    " leftTs=" + left.timestampNs +
                    " rightTs=" + right.timestampNs +
                    " deltaNs=" + pairDeltaNs +
                    " softTargetNs=" + stereoPairMaxDeltaNs +
                    " overSoftTarget=" + (pairDeltaNs > stereoPairMaxDeltaNs) +
                    " avgDeltaNs=" + avgDelta +
                    " maxDeltaNs=" + stereoPairDeltaMaxNs +
                    " softPairOverMax=" + stereoSoftPairOverMaxCount +
                    " observedPairFps=" + fpsLabel(stereoPairDeliveryStats.observedFps()) +
                    " requestedAeFpsRange=" + rangeLabel(requestedCameraFpsRange()) +
                    " leftAppliedAeFpsRange=" + rangeLabel(leftAppliedAeFpsRange != null ? leftAppliedAeFpsRange : logicalStereoAppliedAeFpsRange) +
                    " rightAppliedAeFpsRange=" + rangeLabel(rightAppliedAeFpsRange != null ? rightAppliedAeFpsRange : logicalStereoAppliedAeFpsRange) +
                    " leftReceived=" + stereoLeftReceivedCount +
                    " rightReceived=" + stereoRightReceivedCount +
                    " paired=" + stereoPairedCount +
                    " dropped=" + stereoDroppedCount +
                    " activeTier=" + activeTierLabel() +
                    " stereoLayout=Separate" +
                    " poseSource=" + stereoPoseSourceLabel(activeStereoChoice));
            }
        } finally {
            left.close();
            right.close();
        }
    }

    private String buildFrameMetadataJson(
        CameraChoice choice,
        Image image,
        String transport,
        String activeTier,
        boolean monoFallback,
        String fallbackReason) {
        String sourceLabel = choice != null
            ? "Camera2 " + choice.cameraId + " " + choice.lensFacingLabel
            : "Camera2 unknown";
        String cameraId = choice != null ? choice.cameraId : "unknown";
        String lensFacing = choice != null ? choice.lensFacingLabel : "unknown";
        long score = choice != null ? choice.score : 0L;
        Integer sensorOrientation = choice != null ? choice.sensorOrientationDegrees : null;
        Rect activeArray = choice != null ? choice.activeArraySize : null;
        Size sensorPixelArray = choice != null ? choice.sensorPixelArraySize : null;
        float[] calibration = choice != null ? choice.intrinsicCalibration : null;
        int intrinsicsWidth = activeArray != null ? activeArray.width() : (sensorPixelArray != null ? sensorPixelArray.getWidth() : 0);
        int intrinsicsHeight = activeArray != null ? activeArray.height() : (sensorPixelArray != null ? sensorPixelArray.getHeight() : 0);
        boolean hasIntrinsics = calibration != null && calibration.length >= 4 && intrinsicsWidth > 0 && intrinsicsHeight > 0;
        String intrinsicsDomainKind = activeArray != null ? "activeArray" : (sensorPixelArray != null ? "sensorPixelArray" : "other");
        String poseSource = monoPoseSource(choice);
        boolean hasPose = !"missing".equals(poseSource);

        StringBuilder builder = new StringBuilder(512);
        builder.append('{');
        appendJsonString(builder, "sourceLabel", sourceLabel);
        builder.append(',');
        appendJsonString(builder, "cameraId", cameraId);
        builder.append(',');
        appendJsonString(builder, "lensFacing", lensFacing);
        builder.append(",\"lensFacingRank\":").append(choice != null ? choice.lensFacingRank : 0);
        builder.append(",\"selectionScore\":").append(score);
        builder.append(",\"deliveredWidth\":").append(image.getWidth());
        builder.append(",\"deliveredHeight\":").append(image.getHeight());
        builder.append(",\"timestampNs\":").append(image.getTimestamp());
        appendFpsTelemetry(builder, requestedCameraFpsRange(), monoAppliedAeFpsRange, monoDeliveryStats);
        if (sensorOrientation != null) {
            builder.append(",\"sensorOrientationDegrees\":").append(sensorOrientation.intValue());
        }
        builder.append(",\"stereoLayout\":\"mono\"");
        builder.append(',');
        appendJsonString(builder, "requestedStereoLayout", stereoLayout);
        builder.append(',');
        appendJsonString(builder, "transport", transport);
        builder.append(',');
        appendJsonString(builder, "requestedTier", cameraTier);
        builder.append(',');
        appendJsonString(builder, "activeTier", activeTier);
        builder.append(",\"gpuImportRequested\":").append(wantsGpuBufferTier());
        builder.append(",\"missingIntrinsics\":").append(!hasIntrinsics);
        builder.append(",\"missingPose\":").append(!hasPose);
        builder.append(',');
        appendJsonString(builder, "poseSource", poseSource);
        builder.append(',');
        appendJsonString(builder, "poseCoordinateConvention", poseCoordinateConvention);
        if (hasPose) {
            if ("platform".equals(poseSource) && choice != null) {
                appendPlatformExtrinsics(builder, choice.lensPoseTranslation, choice.lensPoseRotation, choice.lensPoseReference);
            } else if ("estimated-profile".equals(poseSource)) {
                appendEstimatedExtrinsics(builder, estimatedPoseX, estimatedPoseY, estimatedPoseZ,
                    estimatedPoseQx, estimatedPoseQy, estimatedPoseQz, estimatedPoseQw);
            }
        }
        builder.append(",\"monoFallback\":").append(monoFallback);
        builder.append(',');
        appendJsonString(builder, "fallbackReason", fallbackReason);
        if (hasIntrinsics) {
            builder.append(",\"intrinsics\":{");
            builder.append("\"fx\":").append(floatJson(calibration[0]));
            builder.append(",\"fy\":").append(floatJson(calibration[1]));
            builder.append(",\"cx\":").append(floatJson(calibration[2]));
            builder.append(",\"cy\":").append(floatJson(calibration[3]));
            builder.append(",\"skew\":").append(floatJson(calibration.length >= 5 ? calibration[4] : 0.0f));
            builder.append('}');
            builder.append(",\"intrinsicsDomain\":{");
            appendJsonString(builder, "kind", intrinsicsDomainKind);
            builder.append(",\"width\":").append(intrinsicsWidth);
            builder.append(",\"height\":").append(intrinsicsHeight);
            builder.append('}');
        }
        if (activeArray != null) {
            builder.append(",\"activeArrayDomain\":{");
            appendJsonString(builder, "kind", "activeArray");
            builder.append(",\"width\":").append(activeArray.width());
            builder.append(",\"height\":").append(activeArray.height());
            builder.append('}');
        }
        if (sensorPixelArray != null) {
            builder.append(",\"sensorPixelDomain\":{");
            appendJsonString(builder, "kind", "sensorPixelArray");
            builder.append(",\"width\":").append(sensorPixelArray.getWidth());
            builder.append(",\"height\":").append(sensorPixelArray.getHeight());
            builder.append('}');
        }
        builder.append('}');
        return builder.toString();
    }

    private String buildStereoFrameMetadataJson(boolean leftEye, Image image) {
        StereoCameraChoice choice = activeStereoChoice;
        CameraSourceInfo source = choice != null ? (leftEye ? choice.leftSource : choice.rightSource) : null;
        String cameraId = choice != null ? (leftEye ? choice.leftPhysicalId : choice.rightPhysicalId) : "unknown";
        String sourceLabel = "Camera2 " + cameraId + (leftEye ? " left" : " right");
        float[] calibration = source != null ? source.intrinsicCalibration : null;
        Rect activeArray = source != null ? source.activeArraySize : null;
        Size sensorPixelArray = source != null ? source.sensorPixelArraySize : null;
        int intrinsicsWidth = activeArray != null ? activeArray.width() : (sensorPixelArray != null ? sensorPixelArray.getWidth() : 0);
        int intrinsicsHeight = activeArray != null ? activeArray.height() : (sensorPixelArray != null ? sensorPixelArray.getHeight() : 0);
        boolean hasIntrinsics = calibration != null && calibration.length >= 4 && intrinsicsWidth > 0 && intrinsicsHeight > 0;
        String intrinsicsDomainKind = activeArray != null ? "activeArray" : (sensorPixelArray != null ? "sensorPixelArray" : "other");
        String poseSource = stereoPoseSourceLabel(choice);
        boolean hasPose = !"missing".equals(poseSource);

        StringBuilder builder = new StringBuilder(512);
        builder.append('{');
        appendJsonString(builder, "sourceLabel", sourceLabel);
        builder.append(',');
        appendJsonString(builder, "cameraId", cameraId);
        builder.append(',');
        appendJsonString(builder, "eye", leftEye ? "left" : "right");
        builder.append(',');
        appendJsonString(builder, "lensFacing", source != null ? source.lensFacingLabel : "unknown");
        builder.append(",\"lensFacingRank\":").append(source != null ? source.lensFacingRank : 0);
        builder.append(",\"selectionScore\":").append(source != null ? scoreSize(source.characteristics, image.getWidth(), image.getHeight()) : 0L);
        builder.append(",\"deliveredWidth\":").append(image.getWidth());
        builder.append(",\"deliveredHeight\":").append(image.getHeight());
        builder.append(",\"timestampNs\":").append(image.getTimestamp());
        Range<Integer> appliedRange = leftEye ? leftAppliedAeFpsRange : rightAppliedAeFpsRange;
        if (appliedRange == null && logicalStereoAppliedAeFpsRange != null) {
            appliedRange = logicalStereoAppliedAeFpsRange;
        }
        appendFpsTelemetry(
            builder,
            requestedCameraFpsRange(),
            appliedRange,
            leftEye ? leftDeliveryStats : rightDeliveryStats);
        if (source != null && source.sensorOrientationDegrees != null) {
            builder.append(",\"sensorOrientationDegrees\":").append(source.sensorOrientationDegrees.intValue());
        }
        builder.append(",\"stereoLayout\":\"separate\"");
        builder.append(',');
        appendJsonString(builder, "requestedStereoLayout", stereoLayout);
        builder.append(',');
        appendJsonString(builder, "transport", "android-hardware-buffer");
        builder.append(',');
        appendJsonString(builder, "requestedTier", cameraTier);
        builder.append(',');
        appendJsonString(builder, "activeTier", activeTierLabel());
        builder.append(",\"gpuImportRequested\":true");
        builder.append(",\"missingIntrinsics\":").append(!hasIntrinsics);
        builder.append(",\"missingPose\":").append(!hasPose);
        builder.append(',');
        appendJsonString(builder, "poseSource", poseSource);
        builder.append(',');
        appendJsonString(builder, "poseCoordinateConvention", "platform".equals(poseSource)
            ? CAMERA2_POSE_CONVENTION
            : poseCoordinateConvention);
        if (hasPose && source != null) {
            if ("platform".equals(poseSource)) {
                appendPlatformExtrinsics(builder, source.lensPoseTranslation, source.lensPoseRotation, source.lensPoseReference);
            } else if ("estimated-profile".equals(poseSource)) {
                if (leftEye) {
                    appendEstimatedExtrinsics(builder, estimatedLeftPoseX, estimatedLeftPoseY, estimatedLeftPoseZ,
                        estimatedLeftPoseQx, estimatedLeftPoseQy, estimatedLeftPoseQz, estimatedLeftPoseQw);
                } else {
                    appendEstimatedExtrinsics(builder, estimatedRightPoseX, estimatedRightPoseY, estimatedRightPoseZ,
                        estimatedRightPoseQx, estimatedRightPoseQy, estimatedRightPoseQz, estimatedRightPoseQw);
                }
                builder.append(',');
                appendJsonString(builder, "calibrationLabel", estimatedPoseLabel);
                builder.append(',');
                appendJsonString(builder, "calibrationVersion", estimatedPoseVersion);
            }
        }
        builder.append(",\"monoFallback\":false");
        builder.append(',');
        appendJsonString(builder, "fallbackReason", stereoFallbackReason(poseSource));
        if (hasIntrinsics) {
            builder.append(",\"intrinsics\":{");
            builder.append("\"fx\":").append(floatJson(calibration[0]));
            builder.append(",\"fy\":").append(floatJson(calibration[1]));
            builder.append(",\"cx\":").append(floatJson(calibration[2]));
            builder.append(",\"cy\":").append(floatJson(calibration[3]));
            builder.append(",\"skew\":").append(floatJson(calibration.length >= 5 ? calibration[4] : 0.0f));
            builder.append('}');
            builder.append(",\"intrinsicsDomain\":{");
            appendJsonString(builder, "kind", intrinsicsDomainKind);
            builder.append(",\"width\":").append(intrinsicsWidth);
            builder.append(",\"height\":").append(intrinsicsHeight);
            builder.append('}');
        }
        if (activeArray != null) {
            builder.append(",\"activeArrayDomain\":{");
            appendJsonString(builder, "kind", "activeArray");
            builder.append(",\"width\":").append(activeArray.width());
            builder.append(",\"height\":").append(activeArray.height());
            builder.append('}');
        }
        if (sensorPixelArray != null) {
            builder.append(",\"sensorPixelDomain\":{");
            appendJsonString(builder, "kind", "sensorPixelArray");
            builder.append(",\"width\":").append(sensorPixelArray.getWidth());
            builder.append(",\"height\":").append(sensorPixelArray.getHeight());
            builder.append('}');
        }
        builder.append('}');
        return builder.toString();
    }

    private boolean wantsGpuBufferTier() {
        return TIER_GPU_BUFFER_PROBE.equals(cameraTier)
            || "camera-gpu-buffer-probe".equals(cameraTier)
            || TIER_GPU_PROJECTED.equals(cameraTier)
            || "camera-stereo-gpu-composite".equals(cameraTier);
    }

    private boolean wantsProjectedTier() {
        return TIER_GPU_PROJECTED.equals(cameraTier)
            || "camera-stereo-gpu-composite".equals(cameraTier);
    }

    private String activeTierLabel() {
        if (TIER_SOURCE_DIAGNOSTICS.equals(cameraTier)) {
            return TIER_SOURCE_DIAGNOSTICS;
        }
        if (activeImageFormat == ImageFormat.PRIVATE) {
            return TIER_GPU_BUFFER_PROBE;
        }
        return "cpu-diagnostic-flat-copy";
    }

    private static String imageFormatLabel(int imageFormat) {
        if (imageFormat == ImageFormat.PRIVATE) {
            return "PRIVATE";
        }
        if (imageFormat == ImageFormat.YUV_420_888) {
            return "YUV_420_888";
        }
        return Integer.toString(imageFormat);
    }

    private boolean normalizeMonoEstimatedPose() {
        float[] normalized = normalizeQuaternionOrNull(new float[] {
            estimatedPoseQx,
            estimatedPoseQy,
            estimatedPoseQz,
            estimatedPoseQw
        });
        if (normalized == null || !isFinite(estimatedPoseX) || !isFinite(estimatedPoseY) || !isFinite(estimatedPoseZ)) {
            Log.w(TAG, "Ignoring invalid mono estimated camera pose extras");
            return false;
        }
        estimatedPoseQx = normalized[0];
        estimatedPoseQy = normalized[1];
        estimatedPoseQz = normalized[2];
        estimatedPoseQw = normalized[3];
        return true;
    }

    private boolean normalizeEstimatedStereoPose() {
        float[] left = normalizeQuaternionOrNull(new float[] {
            estimatedLeftPoseQx,
            estimatedLeftPoseQy,
            estimatedLeftPoseQz,
            estimatedLeftPoseQw
        });
        float[] right = normalizeQuaternionOrNull(new float[] {
            estimatedRightPoseQx,
            estimatedRightPoseQy,
            estimatedRightPoseQz,
            estimatedRightPoseQw
        });
        boolean finiteTranslation =
            isFinite(estimatedLeftPoseX) &&
            isFinite(estimatedLeftPoseY) &&
            isFinite(estimatedLeftPoseZ) &&
            isFinite(estimatedRightPoseX) &&
            isFinite(estimatedRightPoseY) &&
            isFinite(estimatedRightPoseZ);
        if (left == null || right == null || !finiteTranslation) {
            Log.w(TAG, "Ignoring invalid stereo estimated camera pose extras");
            return false;
        }
        estimatedLeftPoseQx = left[0];
        estimatedLeftPoseQy = left[1];
        estimatedLeftPoseQz = left[2];
        estimatedLeftPoseQw = left[3];
        estimatedRightPoseQx = right[0];
        estimatedRightPoseQy = right[1];
        estimatedRightPoseQz = right[2];
        estimatedRightPoseQw = right[3];
        return true;
    }

    private static boolean isFinite(float value) {
        return !Float.isNaN(value) && !Float.isInfinite(value);
    }

    private static boolean isFiniteArray(float[] values, int expectedLength) {
        if (values == null || values.length < expectedLength) {
            return false;
        }
        for (int i = 0; i < expectedLength; i++) {
            if (!isFinite(values[i])) {
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
            ((double) quaternion[0] * (double) quaternion[0]) +
            ((double) quaternion[1] * (double) quaternion[1]) +
            ((double) quaternion[2] * (double) quaternion[2]) +
            ((double) quaternion[3] * (double) quaternion[3]);
        if (normSquared <= 1.0e-12 || Double.isNaN(normSquared) || Double.isInfinite(normSquared)) {
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

    private static boolean hasUsablePlatformPose(CameraSourceInfo source) {
        return source != null
            && isFiniteArray(source.lensPoseTranslation, 3)
            && normalizeQuaternionOrNull(source.lensPoseRotation) != null
            && isAcceptedLensPoseReference(source.lensPoseReference);
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

    private String monoPoseSource(CameraChoice choice) {
        if (choice != null && isFiniteArray(choice.lensPoseTranslation, 3)
            && normalizeQuaternionOrNull(choice.lensPoseRotation) != null
            && isAcceptedLensPoseReference(choice.lensPoseReference)) {
            return "platform";
        }
        return estimatedPose ? "estimated-profile" : "missing";
    }

    private String stereoPoseSourceLabel(StereoCameraChoice choice) {
        if (choice != null && hasUsablePlatformPose(choice.leftSource) && hasUsablePlatformPose(choice.rightSource)) {
            return "platform";
        }
        return estimatedStereoPose ? "estimated-profile" : "missing";
    }

    private String stereoFallbackReason(String poseSource) {
        if ("platform".equals(poseSource)) {
            return "paired stereo GPU buffers with Camera2 platform lens pose";
        }
        if ("estimated-profile".equals(poseSource)) {
            return "paired stereo GPU buffers with public estimated-profile pose";
        }
        return "paired stereo GPU buffers missing valid per-eye pose/extrinsics";
    }

    private void maybeLogDeliveryStats(
        String streamLabel,
        String cameraId,
        DeliveryStats stats,
        Range<Integer> appliedRange,
        long cadence) {
        if (!stats.shouldLog(cadence)) {
            return;
        }
        Log.i(TAG, "Camera2 delivery stats stream=" + streamLabel +
            " cameraId=" + cameraId +
            " frameCount=" + stats.count +
            " observedFps=" + fpsLabel(stats.observedFps()) +
            " requestedAeFpsRange=" + rangeLabel(requestedCameraFpsRange()) +
            " appliedAeFpsRange=" + rangeLabel(appliedRange) +
            " firstTimestampNs=" + stats.firstTimestampNs +
            " lastTimestampNs=" + stats.lastTimestampNs);
    }

    private void appendFpsRequest(StringBuilder builder) {
        builder.append("\"cameraFpsRequest\":{");
        builder.append("\"targetFps\":").append(cameraTargetFps);
        builder.append(",\"minFps\":").append(cameraFpsMin);
        builder.append(",\"maxFps\":").append(cameraFpsMax);
        Range<Integer> normalized = requestedCameraFpsRange();
        if (normalized != null) {
            builder.append(",\"normalizedMin\":").append(normalized.getLower().intValue());
            builder.append(",\"normalizedMax\":").append(normalized.getUpper().intValue());
        }
        builder.append('}');
    }

    private static void appendFpsTelemetry(
        StringBuilder builder,
        Range<Integer> requestedRange,
        Range<Integer> appliedRange,
        DeliveryStats stats) {
        appendRangeObject(builder, "requestedAeFpsRange", requestedRange);
        appendRangeObject(builder, "appliedAeFpsRange", appliedRange);
        if (stats != null && stats.count > 1) {
            builder.append(",\"observedDeliveryFps\":").append(fpsLabel(stats.observedFps()));
        }
    }

    private static void appendRangeObject(StringBuilder builder, String key, Range<Integer> range) {
        if (range == null) {
            return;
        }
        builder.append(",\"").append(key).append("\":{");
        builder.append("\"min\":").append(range.getLower().intValue());
        builder.append(",\"max\":").append(range.getUpper().intValue());
        builder.append('}');
    }

    private static String rangeArrayLabel(Range<Integer>[] ranges) {
        if (ranges == null || ranges.length == 0) {
            return "[]";
        }
        StringBuilder builder = new StringBuilder();
        builder.append('[');
        for (int i = 0; i < ranges.length; i++) {
            if (i > 0) {
                builder.append(',');
            }
            builder.append(rangeLabel(ranges[i]));
        }
        builder.append(']');
        return builder.toString();
    }

    private static String rangeLabel(Range<Integer> range) {
        if (range == null) {
            return "device-controlled";
        }
        return range.getLower().intValue() + "-" + range.getUpper().intValue();
    }

    private static String fpsLabel(double value) {
        if (Double.isNaN(value) || Double.isInfinite(value) || value < 0.0) {
            return "0.00";
        }
        return String.format(Locale.US, "%.2f", value);
    }

    private void appendPlatformExtrinsics(
        StringBuilder builder,
        float[] translation,
        float[] quaternion,
        Integer reference) {
        float[] normalized = normalizeQuaternionOrNull(quaternion);
        if (!isFiniteArray(translation, 3) || normalized == null) {
            return;
        }
        appendEstimatedExtrinsics(
            builder,
            translation[0],
            translation[1],
            translation[2],
            normalized[0],
            normalized[1],
            normalized[2],
            normalized[3]);
        builder.append(',');
        appendJsonString(builder, "lensPoseReferenceLabel", lensPoseReferenceLabel(reference));
    }

    private void appendEstimatedExtrinsics(
        StringBuilder builder,
        float px,
        float py,
        float pz,
        float qx,
        float qy,
        float qz,
        float qw) {
        builder.append(",\"extrinsics\":{");
        builder.append("\"px\":").append(floatJson(px));
        builder.append(",\"py\":").append(floatJson(py));
        builder.append(",\"pz\":").append(floatJson(pz));
        builder.append(",\"qx\":").append(floatJson(qx));
        builder.append(",\"qy\":").append(floatJson(qy));
        builder.append(",\"qz\":").append(floatJson(qz));
        builder.append(",\"qw\":").append(floatJson(qw));
        builder.append('}');
    }

    private static void appendFloatArray(StringBuilder builder, String key, float[] values) {
        if (values == null) {
            return;
        }
        builder.append(",\"").append(key).append("\":[");
        for (int i = 0; i < values.length; i++) {
            if (i > 0) {
                builder.append(',');
            }
            builder.append(floatJson(values[i]));
        }
        builder.append(']');
    }

    private static String floatJson(float value) {
        if (!Float.isNaN(value) && !Float.isInfinite(value)) {
            return Float.toString(value);
        }
        return "0.0";
    }

    private static void appendJsonString(StringBuilder builder, String key, String value) {
        builder.append('"').append(key).append("\":\"");
        appendJsonEscaped(builder, value);
        builder.append('"');
    }

    private static void appendJsonStringValue(StringBuilder builder, String value) {
        builder.append('"');
        appendJsonEscaped(builder, value);
        builder.append('"');
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

    private boolean shouldDeliverCpuFrame(long timestampNs) {
        if (cpuUploadHz <= 0) {
            return false;
        }
        if (lastDeliveredTimestampNs == Long.MIN_VALUE) {
            return true;
        }

        long elapsedNs = timestampNs - lastDeliveredTimestampNs;
        return elapsedNs < 0 || elapsedNs >= cpuFrameIntervalNs;
    }

    private byte[] yuv420ToRgba(Image image) {
        int width = image.getWidth();
        int height = image.getHeight();
        byte[] output = new byte[width * height * 4];
        Image.Plane[] planes = image.getPlanes();
        Image.Plane yPlane = planes[0];
        Image.Plane uPlane = planes[1];
        Image.Plane vPlane = planes[2];
        java.nio.ByteBuffer yBuffer = yPlane.getBuffer();
        java.nio.ByteBuffer uBuffer = uPlane.getBuffer();
        java.nio.ByteBuffer vBuffer = vPlane.getBuffer();
        int yRowStride = yPlane.getRowStride();
        int yPixelStride = yPlane.getPixelStride();
        int uRowStride = uPlane.getRowStride();
        int uPixelStride = uPlane.getPixelStride();
        int vRowStride = vPlane.getRowStride();
        int vPixelStride = vPlane.getPixelStride();

        int dst = 0;
        for (int y = 0; y < height; y++) {
            int yRow = y * yRowStride;
            int uvY = (y / 2);
            int uRow = uvY * uRowStride;
            int vRow = uvY * vRowStride;
            for (int x = 0; x < width; x++) {
                int uvX = x / 2;
                int yValue = yBuffer.get(yRow + x * yPixelStride) & 0xff;
                int uValue = uBuffer.get(uRow + uvX * uPixelStride) & 0xff;
                int vValue = vBuffer.get(vRow + uvX * vPixelStride) & 0xff;

                int c = Math.max(0, yValue - 16);
                int d = uValue - 128;
                int e = vValue - 128;
                int r = clamp((298 * c + 409 * e + 128) >> 8);
                int g = clamp((298 * c - 100 * d - 208 * e + 128) >> 8);
                int b = clamp((298 * c + 516 * d + 128) >> 8);

                output[dst++] = (byte) r;
                output[dst++] = (byte) g;
                output[dst++] = (byte) b;
                output[dst++] = (byte) 255;
            }
        }

        return output;
    }

    private int clamp(int value) {
        if (value < 0) {
            return 0;
        }
        if (value > 255) {
            return 255;
        }
        return value;
    }

    private void closeCamera() {
        if (captureSession != null) {
            captureSession.close();
            captureSession = null;
        }
        if (leftCaptureSession != null) {
            leftCaptureSession.close();
            leftCaptureSession = null;
        }
        if (rightCaptureSession != null) {
            rightCaptureSession.close();
            rightCaptureSession = null;
        }
        if (cameraDevice != null) {
            cameraDevice.close();
            cameraDevice = null;
        }
        if (leftCameraDevice != null) {
            leftCameraDevice.close();
            leftCameraDevice = null;
        }
        if (rightCameraDevice != null) {
            rightCameraDevice.close();
            rightCameraDevice = null;
        }
        if (imageReader != null) {
            imageReader.close();
            imageReader = null;
        }
        if (leftImageReader != null) {
            leftImageReader.close();
            leftImageReader = null;
        }
        if (rightImageReader != null) {
            rightImageReader.close();
            rightImageReader = null;
        }
        while (!leftFrames.isEmpty()) {
            leftFrames.removeFirst().close();
        }
        while (!rightFrames.isEmpty()) {
            rightFrames.removeFirst().close();
        }
        activeCameraChoice = null;
        activeStereoChoice = null;
        leftStereoSessionRunning = false;
        rightStereoSessionRunning = false;
        if (cameraThread != null) {
            cameraThread.quitSafely();
            cameraThread = null;
            cameraHandler = null;
        }
    }

    private static void sendNativeEvent(String name) {
        try {
            nativeHeadsetCameraEvent("{\"event\":\"" + name + "\",\"timeNs\":" + System.nanoTime() + "}");
        } catch (UnsatisfiedLinkError error) {
            Log.w(TAG, "Native camera event bridge unavailable: " + name);
        }
    }

    private static final class HandlerExecutor implements Executor {
        private final Handler handler;

        HandlerExecutor(Handler handler) {
            this.handler = handler;
        }

        @Override
        public void execute(Runnable command) {
            handler.post(command);
        }
    }

    private static final class CameraSourceDiagnostics {
        final String json;
        final List<CameraSourceInfo> sources;
        final List<StereoCandidateInfo> stereoCandidates;
        final StereoCameraChoice selectedStereoChoice;
        final String stereoFallbackReason;

        CameraSourceDiagnostics(
            String json,
            List<CameraSourceInfo> sources,
            List<StereoCandidateInfo> stereoCandidates,
            StereoCameraChoice selectedStereoChoice,
            String stereoFallbackReason) {
            this.json = json;
            this.sources = sources;
            this.stereoCandidates = stereoCandidates;
            this.selectedStereoChoice = selectedStereoChoice;
            this.stereoFallbackReason = stereoFallbackReason;
        }
    }

    private static final class CameraSourceInfo {
        final String cameraId;
        final CameraCharacteristics characteristics;
        final List<String> physicalCameraIds;
        final boolean logicalMultiCamera;
        final boolean concurrentCamera;
        final String lensFacingLabel;
        final int lensFacingRank;
        final String hardwareLevelLabel;
        final Integer sensorOrientationDegrees;
        final Rect activeArraySize;
        final Size sensorPixelArraySize;
        final float[] intrinsicCalibration;
        final float[] distortion;
        final float[] lensPoseTranslation;
        final float[] lensPoseRotation;
        final Integer lensPoseReference;
        final List<Size> privateSizes;
        final List<Size> yuvSizes;
        final int[] outputFormats;
        final Range<Integer>[] fpsRanges;

        CameraSourceInfo(
            String cameraId,
            CameraCharacteristics characteristics,
            StreamConfigurationMap map,
            boolean concurrentCamera) {
            this.cameraId = cameraId;
            this.characteristics = characteristics;
            this.physicalCameraIds = physicalCameraIds(characteristics);
            this.logicalMultiCamera = hasCapability(
                characteristics,
                CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA);
            this.concurrentCamera = concurrentCamera;
            this.lensFacingLabel = lensFacingLabel(characteristics);
            this.lensFacingRank = (int) lensFacingRank(characteristics);
            this.hardwareLevelLabel = hardwareLevelLabel(characteristics.get(CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL));
            this.sensorOrientationDegrees = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);
            this.activeArraySize = characteristics.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE);
            this.sensorPixelArraySize = characteristics.get(CameraCharacteristics.SENSOR_INFO_PIXEL_ARRAY_SIZE);
            this.intrinsicCalibration = characteristics.get(CameraCharacteristics.LENS_INTRINSIC_CALIBRATION);
            this.distortion = characteristics.get(CameraCharacteristics.LENS_DISTORTION);
            this.lensPoseTranslation = characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION);
            this.lensPoseRotation = characteristics.get(CameraCharacteristics.LENS_POSE_ROTATION);
            this.lensPoseReference = characteristics.get(CameraCharacteristics.LENS_POSE_REFERENCE);
            this.privateSizes = sizesFor(map, ImageFormat.PRIVATE);
            this.yuvSizes = sizesFor(map, ImageFormat.YUV_420_888);
            this.outputFormats = map != null ? map.getOutputFormats() : new int[0];
            Range<Integer>[] ranges = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES);
            this.fpsRanges = ranges != null ? ranges : new Range[0];
        }
    }

    private static List<String> physicalCameraIds(CameraCharacteristics characteristics) {
        if (Build.VERSION.SDK_INT < 28) {
            return Collections.emptyList();
        }
        Set<String> ids = characteristics.getPhysicalCameraIds();
        if (ids == null || ids.isEmpty()) {
            return Collections.emptyList();
        }
        List<String> sorted = new ArrayList<String>(ids);
        Collections.sort(sorted);
        return sorted;
    }

    private static boolean hasCapability(CameraCharacteristics characteristics, int capability) {
        int[] capabilities = characteristics.get(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES);
        if (capabilities == null) {
            return false;
        }
        for (int i = 0; i < capabilities.length; i++) {
            if (capabilities[i] == capability) {
                return true;
            }
        }
        return false;
    }

    private static List<Size> sizesFor(StreamConfigurationMap map, int format) {
        if (map == null) {
            return Collections.emptyList();
        }
        Size[] sizes = map.getOutputSizes(format);
        if (sizes == null || sizes.length == 0) {
            return Collections.emptyList();
        }
        return Arrays.asList(sizes);
    }

    private static String hardwareLevelLabel(Integer level) {
        if (level == null) {
            return "unknown";
        }
        switch (level.intValue()) {
            case CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_LEGACY:
                return "legacy";
            case CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_LIMITED:
                return "limited";
            case CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_FULL:
                return "full";
            case CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_3:
                return "level3";
            case CameraCharacteristics.INFO_SUPPORTED_HARDWARE_LEVEL_EXTERNAL:
                return "external";
            default:
                return "other";
        }
    }

    private static final class StereoCandidateInfo {
        final String providerKind;
        final String leftCameraId;
        final String rightCameraId;
        final boolean accepted;
        final Long score;
        final String reason;

        StereoCandidateInfo(
            String providerKind,
            String leftCameraId,
            String rightCameraId,
            boolean accepted,
            Long score,
            String reason) {
            this.providerKind = providerKind;
            this.leftCameraId = leftCameraId;
            this.rightCameraId = rightCameraId;
            this.accepted = accepted;
            this.score = score;
            this.reason = reason;
        }
    }

    private static final class StereoCameraChoice {
        final String providerKind;
        final String logicalCameraId;
        final String leftPhysicalId;
        final String rightPhysicalId;
        final Size leftSize;
        final Size rightSize;
        final CameraSourceInfo leftSource;
        final CameraSourceInfo rightSource;
        final long score;

        static StereoCameraChoice logicalPhysical(
            String logicalCameraId,
            String leftPhysicalId,
            String rightPhysicalId,
            Size size,
            CameraSourceInfo logicalSource) {
            return new StereoCameraChoice(
                "logical-physical",
                logicalCameraId,
                leftPhysicalId,
                rightPhysicalId,
                size,
                size,
                logicalSource,
                logicalSource,
                scoreStereoPair(logicalSource, size, logicalSource, size));
        }

        static StereoCameraChoice concurrentSeparate(
            CameraSourceInfo leftSource,
            CameraSourceInfo rightSource,
            Size leftSize,
            Size rightSize) {
            return new StereoCameraChoice(
                "concurrent-separate",
                null,
                leftSource.cameraId,
                rightSource.cameraId,
                leftSize,
                rightSize,
                leftSource,
                rightSource,
                scoreStereoPair(leftSource, leftSize, rightSource, rightSize));
        }

        StereoCameraChoice(
            String providerKind,
            String logicalCameraId,
            String leftPhysicalId,
            String rightPhysicalId,
            Size leftSize,
            Size rightSize,
            CameraSourceInfo leftSource,
            CameraSourceInfo rightSource,
            long score) {
            this.providerKind = providerKind;
            this.logicalCameraId = logicalCameraId;
            this.leftPhysicalId = leftPhysicalId;
            this.rightPhysicalId = rightPhysicalId;
            this.leftSize = leftSize;
            this.rightSize = rightSize;
            this.leftSource = leftSource;
            this.rightSource = rightSource;
            this.score = score;
        }
    }

    private static final class PendingGpuImage {
        final boolean leftEye;
        final int width;
        final int height;
        final long timestampNs;
        final String metadataJson;
        final HardwareBuffer buffer;
        final int hardwareBufferFormat;
        final long hardwareBufferUsage;
        final int hardwareBufferLayers;
        final long hardwareBufferId;

        PendingGpuImage(
            boolean leftEye,
            int width,
            int height,
            long timestampNs,
            String metadataJson,
            HardwareBuffer buffer,
            int hardwareBufferFormat,
            long hardwareBufferUsage,
            int hardwareBufferLayers,
            long hardwareBufferId) {
            this.leftEye = leftEye;
            this.width = width;
            this.height = height;
            this.timestampNs = timestampNs;
            this.metadataJson = metadataJson;
            this.buffer = buffer;
            this.hardwareBufferFormat = hardwareBufferFormat;
            this.hardwareBufferUsage = hardwareBufferUsage;
            this.hardwareBufferLayers = hardwareBufferLayers;
            this.hardwareBufferId = hardwareBufferId;
        }

        void close() {
            buffer.close();
        }
    }

    private static final class DeliveryStats {
        long count;
        long firstTimestampNs;
        long lastTimestampNs;
        long lastLoggedCount;

        void reset() {
            count = 0L;
            firstTimestampNs = 0L;
            lastTimestampNs = 0L;
            lastLoggedCount = 0L;
        }

        void record(long timestampNs) {
            if (count == 0L) {
                firstTimestampNs = timestampNs;
            }
            lastTimestampNs = timestampNs;
            count++;
        }

        double observedFps() {
            if (count < 2L || lastTimestampNs <= firstTimestampNs) {
                return 0.0;
            }
            return ((double) (count - 1L) * 1_000_000_000.0) /
                (double) (lastTimestampNs - firstTimestampNs);
        }

        boolean shouldLog(long cadence) {
            if (count == 0L) {
                return false;
            }
            if (count == 1L || count - lastLoggedCount >= cadence) {
                lastLoggedCount = count;
                return true;
            }
            return false;
        }
    }

    private static final class CameraChoice {
        final String cameraId;
        final Size size;
        final long score;
        final String lensFacingLabel;
        final int lensFacingRank;
        final Integer sensorOrientationDegrees;
        final Rect activeArraySize;
        final Size sensorPixelArraySize;
        final float[] intrinsicCalibration;
        final float[] lensPoseTranslation;
        final float[] lensPoseRotation;
        final Integer lensPoseReference;
        final Range<Integer>[] fpsRanges;

        CameraChoice(
            String cameraId,
            Size size,
            long score,
            CameraCharacteristics characteristics) {
            this.cameraId = cameraId;
            this.size = size;
            this.score = score;
            this.lensFacingLabel = lensFacingLabel(characteristics);
            this.lensFacingRank = (int) lensFacingRank(characteristics);
            this.sensorOrientationDegrees = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);
            this.activeArraySize = characteristics.get(CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE);
            this.sensorPixelArraySize = characteristics.get(CameraCharacteristics.SENSOR_INFO_PIXEL_ARRAY_SIZE);
            this.intrinsicCalibration = characteristics.get(CameraCharacteristics.LENS_INTRINSIC_CALIBRATION);
            this.lensPoseTranslation = characteristics.get(CameraCharacteristics.LENS_POSE_TRANSLATION);
            this.lensPoseRotation = characteristics.get(CameraCharacteristics.LENS_POSE_ROTATION);
            this.lensPoseReference = characteristics.get(CameraCharacteristics.LENS_POSE_REFERENCE);
            Range<Integer>[] ranges = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES);
            this.fpsRanges = ranges != null ? ranges : new Range[0];
        }
    }
}
