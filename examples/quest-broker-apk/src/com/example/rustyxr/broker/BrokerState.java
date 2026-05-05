package com.example.rustyxr.broker;

import android.os.SystemClock;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.concurrent.atomic.AtomicLong;

final class BrokerState {
    static final String BROKER_VERSION = "0.1.0-public-proof";
    static final int PROTOCOL_VERSION = 1;
    static final String CONTRACT_VERSION = "rusty.xr.broker.v1";

    final long startedElapsedNanos = SystemClock.elapsedRealtimeNanos();
    final long startedUnixMs = System.currentTimeMillis();
    final AtomicLong httpStatusRequests = new AtomicLong();
    final AtomicLong websocketConnections = new AtomicLong();
    final AtomicLong acceptedLatencySamples = new AtomicLong();
    final AtomicLong rejectedMessages = new AtomicLong();
    final AtomicLong acceptedCommands = new AtomicLong();
    final AtomicLong rejectedCommands = new AtomicLong();
    final AtomicLong brokerConsoleOpenRequests = new AtomicLong();
    final AtomicLong brokerConsoleCloseRequests = new AtomicLong();
    final AtomicLong oscIngressPackets = new AtomicLong();
    final AtomicLong oscIngressRejectedPackets = new AtomicLong();
    final AtomicLong oscIngressBroadcasts = new AtomicLong();
    final AtomicLong publishedStreamEvents = new AtomicLong();
    final AtomicLong videoLabMetricSamples = new AtomicLong();
    final AtomicLong videoLabEncodedStreamManifests = new AtomicLong();
    final AtomicLong videoLabEncodedSampleMetadata = new AtomicLong();
    private final CameraProjectionProviderState cameraProjectionProvider = new CameraProjectionProviderState();
    private final ShellHelperState shellHelper = new ShellHelperState();
    private final VideoLabState videoLab = new VideoLabState();
    private final BreathAssessmentState breathAssessment = new BreathAssessmentState();
    private JSONObject polarPmdStatus = defaultPolarPmdStatus();

    JSONObject toStatusJson(LatencyPublisher publisher, OscIngressServer oscIngressServer) throws Exception {
        JSONObject status = new JSONObject();
        status.put("type", "status");
        status.put("brokerVersion", BROKER_VERSION);
        status.put("protocolVersion", PROTOCOL_VERSION);
        status.put("contractVersion", CONTRACT_VERSION);
        status.put("uptimeMs", (SystemClock.elapsedRealtimeNanos() - startedElapsedNanos) / 1_000_000L);
        status.put("startedUnixMs", startedUnixMs);
        status.put("bindAddress", "127.0.0.1");
        status.put("port", BrokerService.DEFAULT_PORT);

        JSONArray capabilities = capabilitiesJson(publisher, oscIngressServer);
        status.put("capabilities", capabilities);
        status.put("streams", streamsJson(oscIngressServer));
        status.put("cameraProvider", cameraProjectionProvider.toStatusJson());
        status.put("projectionProfile", cameraProjectionProvider.projectionProfileJson());
        status.put("shellHelper", shellHelper.toStatusJson());
        status.put("polarPmd", polarPmdStatusJson());
        status.put("breathAssessment", breathAssessment.toStatusJson());
        status.put("videoLab", videoLabStatusJson());

        JSONObject commands = new JSONObject();
        commands.put("schema", "rusty.xr.broker.command.v1");
        commands.put("ackSchema", "rusty.xr.broker.command_ack.v1");
        JSONArray supportedCommands = new JSONArray();
        supportedCommands.put("status_request");
        supportedCommands.put("list_capabilities");
        supportedCommands.put("list_streams");
        supportedCommands.put("subscribe");
        supportedCommands.put("unsubscribe");
        supportedCommands.put("configure_osc_ingress");
        supportedCommands.put("publish_stream_event");
        supportedCommands.put("polar_pmd.get_status");
        supportedCommands.put("polar_pmd.start");
        supportedCommands.put("polar_pmd.stop");
        supportedCommands.put("breath_assessment.get_status");
        supportedCommands.put("breath_assessment.configure");
        supportedCommands.put("breath_assessment.reset");
        supportedCommands.put("breath_assessment.submit_controller_pose");
        supportedCommands.put("open_ui");
        supportedCommands.put("close_ui");
        supportedCommands.put("camera_provider.get_status");
        supportedCommands.put("camera_provider.get_projection_profile");
        supportedCommands.put("camera_provider.run_app_camera_probe");
        supportedCommands.put("camera_provider.start_app_camera_luma_stream");
        supportedCommands.put("camera_provider.start_app_camera_h264_stream");
        supportedCommands.put("camera_provider.run_app_camera_h264_decode_probe");
        supportedCommands.put("camera_provider.set_source_eye_mapping");
        supportedCommands.put("camera_provider.set_texture_transform");
        supportedCommands.put("camera_provider.record_visual_acceptance");
        supportedCommands.put("shell_helper.get_status");
        supportedCommands.put("shell_helper.report_status");
        supportedCommands.put("video_lab.get_status");
        supportedCommands.put("video_lab.register_encoded_stream_manifest");
        supportedCommands.put("video_lab.record_encoded_sample_metadata");
        supportedCommands.put("video_lab.record_metric_sample");
        commands.put("supported", supportedCommands);
        status.put("commands", commands);

        JSONObject counters = new JSONObject();
        counters.put("httpStatusRequests", httpStatusRequests.get());
        counters.put("websocketConnections", websocketConnections.get());
        counters.put("acceptedLatencySamples", acceptedLatencySamples.get());
        counters.put("rejectedMessages", rejectedMessages.get());
        counters.put("acceptedCommands", acceptedCommands.get());
        counters.put("rejectedCommands", rejectedCommands.get());
        counters.put("brokerConsoleOpenRequests", brokerConsoleOpenRequests.get());
        counters.put("brokerConsoleCloseRequests", brokerConsoleCloseRequests.get());
        counters.put("oscIngressPackets", oscIngressPackets.get());
        counters.put("oscIngressRejectedPackets", oscIngressRejectedPackets.get());
        counters.put("oscIngressBroadcasts", oscIngressBroadcasts.get());
        counters.put("publishedStreamEvents", publishedStreamEvents.get());
        counters.put("videoLabMetricSamples", videoLabMetricSamples.get());
        counters.put("videoLabEncodedStreamManifests", videoLabEncodedStreamManifests.get());
        counters.put("videoLabEncodedSampleMetadata", videoLabEncodedSampleMetadata.get());
        status.put("counters", counters);

        JSONObject lsl = new JSONObject();
        lsl.put("enabled", publisher != null && publisher.isLslAvailable());
        lsl.put("publisher", publisher != null ? publisher.mode() : "none");
        lsl.put("streamName", NativeLslLatencyPublisher.STREAM_NAME);
        lsl.put("streamType", NativeLslLatencyPublisher.STREAM_TYPE);
        if (publisher != null && publisher.blocker() != null && publisher.blocker().length() > 0) {
            lsl.put("blocker", publisher.blocker());
        }
        status.put("lsl", lsl);

        JSONObject osc = new JSONObject();
        osc.put("egress", publisher != null ? publisher.oscStatus() : new JSONObject().put("enabled", false));
        osc.put("ingress", oscIngressServer != null ? oscIngressServer.toStatusJson() : new JSONObject().put("enabled", false));
        status.put("osc", osc);
        return status;
    }

    JSONArray capabilitiesJson(LatencyPublisher publisher, OscIngressServer oscIngressServer) {
        JSONArray capabilities = new JSONArray();
        capabilities.put("websocket.events");
        capabilities.put("websocket.control");
        capabilities.put("http.status");
        capabilities.put("broker.command.v1");
        capabilities.put("broker.subscription.v1");
        capabilities.put("broker.stream_event.v1");
        capabilities.put("broker.osc_ingress.configure");
        capabilities.put("broker.stream_event.publish");
        capabilities.put("bio.polar_pmd.android_ble.v1");
        capabilities.put("bio.polar_acc.direct_ble.v1");
        capabilities.put("bio.breath_assessment.v1");
        capabilities.put("bio.breath.polar_acc.v1");
        capabilities.put("bio.breath.controller_pose.v1");
        capabilities.put("broker.console.activity");
        capabilities.put("broker.console.return_to_previous_app");
        capabilities.put("broker.console.close_command");
        capabilities.put("broker.launcher.local_lists.v1");
        capabilities.put("broker.launcher.package_manager_launch.v1");
        capabilities.put("camera_projection.metadata.v1");
        capabilities.put("camera_projection.profile.v1");
        capabilities.put("camera_projection.visual_acceptance.v1");
        capabilities.put("camera_projection.app_camera_probe.v1");
        capabilities.put("camera_projection.app_camera_luma_stream.v1");
        capabilities.put("camera_projection.app_camera_h264_stream.v1");
        capabilities.put("camera_projection.app_camera_h264_decode_probe.v1");
        capabilities.put("shell_helper.status.v1");
        capabilities.put("video_lab.metrics.v1");
        capabilities.put("video_lab.encoded_stream_manifest.v1");
        capabilities.put("video_lab.encoded_sample_metadata.v1");
        capabilities.put("latency.sample.accept");
        capabilities.put("logcat.diagnostics");
        if (publisher != null && publisher.isLslAvailable()) {
            capabilities.put("lsl.gateway");
        } else {
            capabilities.put("lsl.gateway.pending_native_binding");
        }
        if (publisher != null && publisher.isOscAvailable()) {
            capabilities.put("osc.udp.send");
        }
        if (oscIngressServer != null && oscIngressServer.isRunning()) {
            capabilities.put("osc.udp.receive");
            capabilities.put("osc.drive.websocket.broadcast");
        }
        return capabilities;
    }

    JSONArray streamsJson(OscIngressServer oscIngressServer) throws Exception {
        JSONArray streams = new JSONArray();
        streams.put(streamJson("broker:status", "status", "Broker status snapshots and capability reports.", true));
        streams.put(streamJson("latency:sample", "latency", "WebSocket latency samples accepted by the broker.", true));
        streams.put(streamJson("bio:polar_hr_rr", "bio", "Synthetic or adapter-published Polar-compatible heart-rate/RR events.", true));
        streams.put(streamJson("bio:polar_ecg", "bio", "Synthetic or adapter-published Polar-compatible ECG frame events.", true));
        streams.put(streamJson(
            "bio:polar_acc",
            "bio",
            "Synthetic, adapter-published, or direct Android BLE Polar PMD accelerometer frame events.",
            true));
        streams.put(streamJson("bio:breath", "bio", "Diagnostic breath volume/state assessments produced from supported motion sources.", breathAssessment.hasAssessments()));
        streams.put(streamJson("xr:controller_pose", "xr", "Adapter-published controller pose samples accepted for broker-side breath assessment.", true));
        streams.put(streamJson("camera_provider.status", "camera", "Projection metadata provider status and limitations.", true));
        streams.put(streamJson("camera_provider.projection_profile", "camera", "Projection profile changes for XR clients that render their own layers.", true));
        streams.put(streamJson("camera_provider.visual_acceptance", "camera", "Operator visual-acceptance markers for projection profiles.", true));
        streams.put(streamJson("shell_helper.status", "shell_helper", "ADB-launched shell-helper status when a helper is connected.", shellHelper.isConnected()));
        streams.put(streamJson("video_lab.metric_sample", "video", "Video texture latency lab metric samples.", videoLabMetricSamples.get() > 0));
        streams.put(streamJson(
            "video_lab.encoded_stream_manifest",
            "video",
            "Metadata manifest for a candidate encoded video stream.",
            videoLabEncodedStreamManifests.get() > 0));
        streams.put(streamJson(
            "video_lab.encoded_sample_metadata",
            "video",
            "Low-rate metadata for encoded video samples; frame payloads use another transport.",
            videoLabEncodedSampleMetadata.get() > 0));

        if (oscIngressServer != null && oscIngressServer.isRunning()) {
            streams.put(streamJson(
                oscIngressServer.streamId(),
                "osc",
                "Accepted OSC ingress drive values normalized to 0..1.",
                true));
        } else {
            streams.put(streamJson("osc:/rusty-xr/drive/radius", "osc", "OSC ingress drive values when enabled.", false));
        }

        return streams;
    }

    private static JSONObject streamJson(String id, String kind, String description, boolean active) throws Exception {
        JSONObject stream = new JSONObject();
        stream.put("id", id);
        stream.put("kind", kind);
        stream.put("description", description);
        stream.put("active", active);
        return stream;
    }

    JSONObject cameraProviderStatusJson() throws Exception {
        return cameraProjectionProvider.toStatusJson();
    }

    JSONObject projectionProfileJson() throws Exception {
        return cameraProjectionProvider.projectionProfileJson();
    }

    JSONObject setSourceEyeMapping(String sourceEyeMapping) throws Exception {
        return cameraProjectionProvider.setSourceEyeMapping(sourceEyeMapping);
    }

    JSONObject setTextureTransform(String leftTextureTransform, String rightTextureTransform) throws Exception {
        return cameraProjectionProvider.setTextureTransform(leftTextureTransform, rightTextureTransform);
    }

    JSONObject recordVisualAcceptance(boolean accepted, String note, String source) throws Exception {
        return cameraProjectionProvider.recordVisualAcceptance(accepted, note, source);
    }

    JSONObject recordAppCameraProbe(JSONObject probe) throws Exception {
        return cameraProjectionProvider.recordAppCameraProbe(probe);
    }

    JSONObject shellHelperStatusJson() throws Exception {
        return shellHelper.toStatusJson();
    }

    JSONObject reportShellHelperStatus(JSONObject params) throws Exception {
        JSONObject status = shellHelper.reportStatus(params);
        cameraProjectionProvider.applyShellHelperDiagnostics(shellHelper.diagnosticsJson());
        return status;
    }

    JSONObject videoLabStatusJson() throws Exception {
        return videoLab.toStatusJson(
            videoLabMetricSamples.get(),
            videoLabEncodedStreamManifests.get(),
            videoLabEncodedSampleMetadata.get());
    }

    JSONObject breathAssessmentStatusJson() throws Exception {
        return breathAssessment.toStatusJson();
    }

    synchronized void updatePolarPmdStatus(JSONObject status) throws Exception {
        polarPmdStatus = status == null ? defaultPolarPmdStatus() : new JSONObject(status.toString());
    }

    synchronized JSONObject polarPmdStatusJson() throws Exception {
        return new JSONObject(polarPmdStatus.toString());
    }

    synchronized boolean hasPolarPmdFrames() {
        return polarPmdStatus.optLong("acc_frame_count", 0L) > 0L;
    }

    JSONObject configureBreathAssessment(JSONObject params) throws Exception {
        return breathAssessment.configure(params);
    }

    JSONObject resetBreathAssessment(JSONObject params) throws Exception {
        return breathAssessment.reset(params);
    }

    JSONObject processBreathAssessmentStreamEvent(
        String stream,
        JSONObject payload,
        long sequence,
        long receiveUnixNs,
        long receiveElapsedNs) throws Exception {
        return breathAssessment.processPublishedStreamEvent(stream, payload, sequence, receiveUnixNs, receiveElapsedNs);
    }

    JSONObject processControllerBreathPose(
        JSONObject params,
        long sequence,
        long receiveUnixNs,
        long receiveElapsedNs) throws Exception {
        return breathAssessment.processControllerPose(params, sequence, receiveUnixNs, receiveElapsedNs);
    }

    JSONObject registerVideoLabEncodedStreamManifest(
        JSONObject params,
        long revision,
        long receiveUnixNs,
        long receiveElapsedNs) throws Exception {
        return videoLab.registerEncodedStreamManifest(params, revision, receiveUnixNs, receiveElapsedNs);
    }

    JSONObject recordVideoLabEncodedSampleMetadata(
        JSONObject params,
        long sequence,
        long receiveUnixNs,
        long receiveElapsedNs) throws Exception {
        return videoLab.recordEncodedSampleMetadata(params, sequence, receiveUnixNs, receiveElapsedNs);
    }

    JSONObject recordVideoLabMetricSample(JSONObject params) throws Exception {
        return videoLab.recordMetricSample(params);
    }

    private static long unixNowNs() {
        return System.currentTimeMillis() * 1_000_000L;
    }

    private static JSONObject defaultPolarPmdStatus() {
        JSONObject status = new JSONObject();
        try {
            status.put("schema", PolarPmdBrokerSource.STATUS_SCHEMA);
            status.put("enabled", false);
            status.put("state", "idle");
            status.put("input_stream", BreathAssessmentState.POLAR_INPUT_STREAM);
            status.put("output_stream", BreathAssessmentState.OUTPUT_STREAM);
            status.put("acc_frame_count", 0L);
            status.put("acc_sample_count", 0L);
            status.put("malformed_frame_count", 0L);
            status.put("last_error", "");
        } catch (Exception ignored) {
        }
        return status;
    }

    private static final class CameraProjectionProviderState {
        private String sourceEyeMapping = "left-right";
        private String leftTextureTransform = "rotate0";
        private String rightTextureTransform = "rotate0";
        private boolean visualReleaseAccepted;
        private String visualAcceptanceNote = "";
        private String visualAcceptanceSource = "";
        private long visualAcceptanceUnixMs;
        private long revision = 1L;
        private JSONObject latestShellCameraProbe = new JSONObject();
        private JSONObject latestShellCameraOpenProbe = new JSONObject();
        private JSONObject latestAppCameraProbe = new JSONObject();
        private long shellCameraDiagnosticsUnixMs;
        private long appCameraProbeUnixMs;

        synchronized void applyShellHelperDiagnostics(JSONObject diagnostics) throws Exception {
            if (diagnostics == null) {
                return;
            }

            boolean changed = false;
            JSONObject cameraProbe = diagnostics.optJSONObject("camera_probe");
            if (cameraProbe != null && !cameraProbe.toString().equals(latestShellCameraProbe.toString())) {
                latestShellCameraProbe = new JSONObject(cameraProbe.toString());
                changed = true;
            }

            JSONObject cameraOpenProbe = diagnostics.optJSONObject("camera_open_probe");
            if (cameraOpenProbe != null && !cameraOpenProbe.toString().equals(latestShellCameraOpenProbe.toString())) {
                latestShellCameraOpenProbe = new JSONObject(cameraOpenProbe.toString());
                changed = true;
            }

            if (changed) {
                shellCameraDiagnosticsUnixMs = System.currentTimeMillis();
                revision++;
            }
        }

        synchronized JSONObject toStatusJson() throws Exception {
            boolean hasShellCameraMetadata = latestShellCameraProbe.length() > 0;
            boolean hasShellCameraOpenProbe = latestShellCameraOpenProbe.length() > 0;
            boolean hasAppCameraProbe = latestAppCameraProbe.length() > 0;
            int shellOpenSuccessCount = latestShellCameraOpenProbe.optInt("open_success_count", 0);
            int shellCaptureSuccessCount = latestShellCameraOpenProbe.optInt("capture_success_count", 0);
            int appOpenSuccessCount = latestAppCameraProbe.optInt("open_success_count", 0);
            int appCaptureSuccessCount = latestAppCameraProbe.optInt("capture_success_count", 0);
            JSONArray sourceCandidates = buildSourceCandidates();

            JSONObject status = new JSONObject();
            status.put("schema", "rusty.xr.camera_provider.status.v1");
            status.put("provider_id", "camera_projection_provider");
            status.put(
                "state",
                providerStateLabel(
                    hasShellCameraMetadata,
                    shellOpenSuccessCount,
                    shellCaptureSuccessCount,
                    hasAppCameraProbe,
                    appOpenSuccessCount,
                    appCaptureSuccessCount));
            status.put("tier", "P0");
            status.put("revision", revision);

            JSONArray capabilities = new JSONArray();
            capabilities.put("camera_projection.metadata.v1");
            capabilities.put("camera_projection.profile.v1");
            capabilities.put("camera_projection.visual_acceptance.v1");
            if (hasShellCameraMetadata) {
                capabilities.put("camera_projection.shell_camera_metadata.v1");
            }
            if (hasShellCameraOpenProbe) {
                capabilities.put("camera_projection.shell_camera_open_probe.v1");
            }
            if (shellCaptureSuccessCount > 0) {
                capabilities.put("camera_projection.shell_camera_capture_probe.v1");
            }
            if (hasAppCameraProbe) {
                capabilities.put("camera_projection.app_camera_probe.v1");
            }
            if (appOpenSuccessCount > 0) {
                capabilities.put("camera_projection.app_camera_open_probe.v1");
            }
            if (appCaptureSuccessCount > 0) {
                capabilities.put("camera_projection.app_camera_capture_probe.v1");
            }
            status.put("capabilities", capabilities);

            JSONArray limitations = new JSONArray();
            limitations.put("client_owned_openxr_eye_views");
            limitations.put("client_owned_texture_import");
            limitations.put("client_owned_layer_submission");
            limitations.put("no_cross_app_layer_injection");
            limitations.put("normal_broker_apk_not_shell_uid");
            if (hasShellCameraMetadata) {
                limitations.put("shell_camera_metadata_is_diagnostic");
                limitations.put("no_camera_frame_transport_from_broker");
            }
            if (hasShellCameraOpenProbe && shellCaptureSuccessCount == 0) {
                limitations.put("shell_camera_capture_not_verified");
            }
            if (shellCaptureSuccessCount > 0) {
                limitations.put("shell_camera_capture_probe_is_not_streaming_provider");
            }
            if (hasAppCameraProbe) {
                limitations.put("app_camera_probe_requires_runtime_camera_permission");
                limitations.put("app_camera_probe_is_diagnostic");
                limitations.put("no_camera_frame_transport_from_broker");
                if (!latestAppCameraProbe.optBoolean("camera_permission_granted", false)) {
                    limitations.put("app_camera_runtime_permission_missing");
                }
                if (appCaptureSuccessCount == 0) {
                    limitations.put("app_camera_capture_not_verified");
                }
            }
            if (appCaptureSuccessCount > 0) {
                limitations.put("app_camera_capture_probe_is_not_streaming_provider");
            }
            status.put("limitations", limitations);

            status.put("projection_profile_id", profileId(hasShellCameraMetadata, hasAppCameraProbe, appCaptureSuccessCount));
            status.put("source_count", sourceCandidates.length());
            status.put("source_candidates", sourceCandidates);
            if (hasShellCameraMetadata) {
                status.put("shell_camera_probe", summarizeShellCameraProbe(sourceCandidates));
                status.put("last_shell_camera_diagnostics_unix_ms", shellCameraDiagnosticsUnixMs);
            }
            if (hasShellCameraOpenProbe) {
                status.put("open_capture_probe", summarizeShellCameraOpenProbe());
            }
            if (hasAppCameraProbe) {
                status.put("app_camera_probe", summarizeAppCameraProbe());
                status.put("last_app_camera_probe_unix_ms", appCameraProbeUnixMs);
            }
            status.put("visual_release_accepted", visualReleaseAccepted);
            status.put("visual_acceptance_unix_ms", visualAcceptanceUnixMs);
            status.put("last_error", "");
            return status;
        }

        synchronized JSONObject projectionProfileJson() throws Exception {
            boolean hasShellCameraMetadata = latestShellCameraProbe.length() > 0;
            boolean hasAppCameraProbe = latestAppCameraProbe.length() > 0;
            int shellCaptureSuccessCount = latestShellCameraOpenProbe.optInt("capture_success_count", 0);
            int appCaptureSuccessCount = latestAppCameraProbe.optInt("capture_success_count", 0);
            JSONArray sourceCandidates = buildSourceCandidates();

            JSONObject profile = new JSONObject();
            profile.put("schema", "rusty.xr.projection.profile.v1");
            profile.put("profile_id", profileId(hasShellCameraMetadata, hasAppCameraProbe, appCaptureSuccessCount));
            profile.put("revision", revision);
            profile.put("mapping", hasShellCameraMetadata || hasCamera2AppMetadata()
                ? "camera2_lens_pose_intrinsics_metadata"
                : "display_screen_homography");
            profile.put("stereo_layout", "separate");
            profile.put("source_eye_mapping", sourceEyeMapping);
            profile.put("left_texture_transform", leftTextureTransform);
            profile.put("right_texture_transform", rightTextureTransform);
            profile.put("color_mode", "raw-feed-unorm");
            profile.put("source", hasShellCameraMetadata
                ? "shell_helper.camera_probe"
                : hasAppCameraProbe
                ? "broker_app.camera2_probe"
                : "static_display_projection_baseline");
            profile.put("delivery", appCaptureSuccessCount > 0
                ? "diagnostic_app_yuv_capture_verified"
                : shellCaptureSuccessCount > 0
                ? "diagnostic_shell_yuv_capture_verified"
                : "metadata_only");
            profile.put("requires_client_eye_views", true);
            profile.put("requires_client_layer_submission", true);
            if (hasShellCameraMetadata || hasCamera2AppMetadata()) {
                profile.put("source_candidates", sourceCandidates);
                profile.put("candidate_pair", chooseCandidatePair(sourceCandidates));
            }
            if (hasShellCameraMetadata) {
                profile.put("last_shell_camera_diagnostics_unix_ms", shellCameraDiagnosticsUnixMs);
            }
            if (latestShellCameraOpenProbe.length() > 0) {
                profile.put("open_capture_probe", summarizeShellCameraOpenProbe());
            }
            if (hasAppCameraProbe) {
                profile.put("app_camera_probe", summarizeAppCameraProbe());
                profile.put("last_app_camera_probe_unix_ms", appCameraProbeUnixMs);
            }
            profile.put("visual_release_accepted", visualReleaseAccepted);
            profile.put("visual_acceptance_note", visualAcceptanceNote);
            profile.put("visual_acceptance_source", visualAcceptanceSource);
            profile.put("visual_acceptance_unix_ms", visualAcceptanceUnixMs);
            return profile;
        }

        private String providerStateLabel(
                boolean hasShellCameraMetadata,
                int shellOpenSuccessCount,
                int shellCaptureSuccessCount,
                boolean hasAppCameraProbe,
                int appOpenSuccessCount,
                int appCaptureSuccessCount) {
            if (appCaptureSuccessCount > 0) {
                return "app_camera_capture_verified";
            }
            if (appOpenSuccessCount > 0) {
                return "app_camera_open_verified";
            }
            if (hasAppCameraProbe && latestAppCameraProbe.optInt("camera_id_count", 0) > 0) {
                return "app_camera_metadata_available";
            }
            if (shellCaptureSuccessCount > 0) {
                return "shell_camera_capture_verified";
            }
            if (shellOpenSuccessCount > 0) {
                return "shell_camera_open_verified";
            }
            if (hasShellCameraMetadata) {
                return "shell_camera_metadata_available";
            }
            return "metadata_only";
        }

        private JSONObject summarizeShellCameraProbe(JSONArray sourceCandidates) throws Exception {
            JSONObject summary = new JSONObject();
            summary.put("schema", latestShellCameraProbe.optString("schema"));
            summary.put("source", latestShellCameraProbe.optString("source"));
            summary.put("camera_count", latestShellCameraProbe.optInt("camera_count", 0));
            summary.put("api1_visible_count", latestShellCameraProbe.optInt("api1_visible_count", 0));
            summary.put("public_api1_visible_count", latestShellCameraProbe.optInt("public_api1_visible_count", 0));
            summary.put("parsed_device_count", latestShellCameraProbe.optInt("parsed_device_count", 0));
            summary.put("dynamic_camera_ids", copyArray(latestShellCameraProbe.optJSONArray("dynamic_camera_ids")));
            summary.put("source_camera_ids", sourceCameraIds(sourceCandidates));
            summary.put("raw_output_bytes", latestShellCameraProbe.optInt("raw_output_bytes", 0));
            summary.put("raw_output_truncated", latestShellCameraProbe.optBoolean("raw_output_truncated", false));
            summary.put("timed_out", latestShellCameraProbe.optBoolean("timed_out", false));
            return summary;
        }

        private JSONObject summarizeShellCameraOpenProbe() throws Exception {
            JSONObject summary = new JSONObject();
            summary.put("schema", latestShellCameraOpenProbe.optString("schema"));
            summary.put("source", latestShellCameraOpenProbe.optString("source"));
            summary.put("manager_state", latestShellCameraOpenProbe.optString("manager_state"));
            summary.put("camera_id_count", latestShellCameraOpenProbe.optInt("camera_id_count", 0));
            summary.put("attempted_count", latestShellCameraOpenProbe.optInt("attempted_count", 0));
            summary.put("open_success_count", latestShellCameraOpenProbe.optInt("open_success_count", 0));
            summary.put("capture_success_count", latestShellCameraOpenProbe.optInt("capture_success_count", 0));
            summary.put("target_camera_ids", copyArray(latestShellCameraOpenProbe.optJSONArray("target_camera_ids")));
            if (latestShellCameraOpenProbe.has("error")) {
                summary.put("error", latestShellCameraOpenProbe.optString("error"));
            }
            return summary;
        }

        private JSONObject summarizeAppCameraProbe() throws Exception {
            JSONObject summary = new JSONObject();
            summary.put("schema", latestAppCameraProbe.optString("schema"));
            summary.put("source", latestAppCameraProbe.optString("source"));
            summary.put("manager_state", latestAppCameraProbe.optString("manager_state"));
            summary.put("camera_permission_granted", latestAppCameraProbe.optBoolean("camera_permission_granted", false));
            summary.put("headset_camera_permission_granted", latestAppCameraProbe.optBoolean("headset_camera_permission_granted", false));
            summary.put("camera_id_count", latestAppCameraProbe.optInt("camera_id_count", 0));
            summary.put("attempted_count", latestAppCameraProbe.optInt("attempted_count", 0));
            summary.put("open_success_count", latestAppCameraProbe.optInt("open_success_count", 0));
            summary.put("capture_success_count", latestAppCameraProbe.optInt("capture_success_count", 0));
            summary.put("target_camera_ids", copyArray(latestAppCameraProbe.optJSONArray("target_camera_ids")));
            summary.put("duration_ms", latestAppCameraProbe.optLong("duration_ms", 0L));
            if (latestAppCameraProbe.has("error")) {
                summary.put("error", latestAppCameraProbe.optString("error"));
            }
            return summary;
        }

        private JSONArray buildSourceCandidates() throws Exception {
            JSONArray candidates = new JSONArray();
            JSONArray devices = latestShellCameraProbe.optJSONArray("devices");
            String metadataSource = "shell_helper.camera_probe";
            if (devices == null || devices.length() == 0) {
                devices = latestAppCameraProbe.optJSONArray("devices");
                metadataSource = "broker_app.camera2_probe";
            }
            if (devices == null || devices.length() == 0) {
                return candidates;
            }

            for (int i = 0; i < devices.length(); i++) {
                JSONObject device = devices.getJSONObject(i);
                JSONObject candidate = new JSONObject();
                String cameraId = device.optString("camera_id");
                candidate.put("camera_id", cameraId);
                candidate.put("metadata_source", metadataSource);
                candidate.put("hal_device", device.optString("hal_device"));
                candidate.put("hal_version", device.optString("hal_version"));
                candidate.put("lens_facing", device.optString("lens_facing", device.optString("api1_facing")));
                candidate.put("lens_pose_reference", device.optString("lens_pose_reference"));
                candidate.put("has_lens_pose", hasArrayLength(device, "lens_pose_rotation_xyzw", 4) &&
                    hasArrayLength(device, "lens_pose_translation_m", 3));
                candidate.put("has_intrinsics", hasArrayLength(device, "lens_intrinsic_calibration", 5));
                candidate.put("supported_hardware_level", device.optString("supported_hardware_level"));
                candidate.put("resource_cost", device.optInt("resource_cost", 0));
                candidate.put("lens_pose_translation_m", copyArray(device.optJSONArray("lens_pose_translation_m")));
                candidate.put("lens_intrinsic_calibration", copyArray(device.optJSONArray("lens_intrinsic_calibration")));

                JSONArray streamConfigurations = device.optJSONArray("stream_configurations");
                int privateCount = countStreamConfigurations(streamConfigurations, "PRIVATE");
                int yuvCount = countStreamConfigurations(streamConfigurations, "YUV_420_888");
                int blobCount = countStreamConfigurations(streamConfigurations, "BLOB");
                candidate.put("private_stream_config_count", privateCount);
                candidate.put("yuv_420_888_stream_config_count", yuvCount);
                candidate.put("blob_stream_config_count", blobCount);
                JSONObject maxPrivateSize = maxStreamSize(streamConfigurations, "PRIVATE");
                JSONObject maxYuvSize = maxStreamSize(streamConfigurations, "YUV_420_888");
                if (maxPrivateSize != null) {
                    candidate.put("max_private_size", maxPrivateSize);
                }
                if (maxYuvSize != null) {
                    candidate.put("max_yuv_420_888_size", maxYuvSize);
                }
                candidate.put("max_ae_fps", maxAeFps(device.optJSONArray("ae_available_target_fps_rows")));

                JSONObject openAttempt = findOpenAttempt(cameraId);
                if (openAttempt != null) {
                    candidate.put("open_state", openAttempt.optString("open_state"));
                    candidate.put("open_succeeded", openAttempt.optBoolean("open_succeeded", false));
                    candidate.put("capture_state", openAttempt.optString("capture_state"));
                    candidate.put("capture_succeeded", openAttempt.optBoolean("capture_succeeded", false));
                    if (openAttempt.has("capture_size")) {
                        candidate.put("capture_size", new JSONObject(openAttempt.getJSONObject("capture_size").toString()));
                    }
                    if (openAttempt.has("captured_width")) {
                        candidate.put("captured_width", openAttempt.optInt("captured_width", 0));
                        candidate.put("captured_height", openAttempt.optInt("captured_height", 0));
                    }
                }

                JSONObject appAttempt = findAppCaptureAttempt(cameraId);
                if (appAttempt != null) {
                    candidate.put("app_open_state", appAttempt.optString("open_state"));
                    candidate.put("app_open_succeeded", appAttempt.optBoolean("open_succeeded", false));
                    candidate.put("app_capture_state", appAttempt.optString("capture_state"));
                    candidate.put("app_capture_succeeded", appAttempt.optBoolean("capture_succeeded", false));
                    if (appAttempt.has("capture_size")) {
                        candidate.put("app_capture_size", new JSONObject(appAttempt.getJSONObject("capture_size").toString()));
                    }
                    if (appAttempt.has("captured_width")) {
                        candidate.put("app_captured_width", appAttempt.optInt("captured_width", 0));
                        candidate.put("app_captured_height", appAttempt.optInt("captured_height", 0));
                    }
                    if (openAttempt == null) {
                        candidate.put("open_state", appAttempt.optString("open_state"));
                        candidate.put("open_succeeded", appAttempt.optBoolean("open_succeeded", false));
                        candidate.put("capture_state", appAttempt.optString("capture_state"));
                        candidate.put("capture_succeeded", appAttempt.optBoolean("capture_succeeded", false));
                    }
                }
                candidates.put(candidate);
            }
            return candidates;
        }

        private String profileId(
                boolean hasShellCameraMetadata,
                boolean hasAppCameraProbe,
                int appCaptureSuccessCount) {
            if (appCaptureSuccessCount > 0) {
                return "app-camera2-capture-profile";
            }
            if (hasShellCameraMetadata) {
                return "shell-camera2-metadata-profile";
            }
            if (hasAppCameraProbe && hasCamera2AppMetadata()) {
                return "app-camera2-metadata-profile";
            }
            return "display-screen-homography-baseline";
        }

        private boolean hasCamera2AppMetadata() {
            JSONArray devices = latestAppCameraProbe.optJSONArray("devices");
            return devices != null && devices.length() > 0;
        }

        private JSONObject chooseCandidatePair(JSONArray sourceCandidates) throws Exception {
            List<JSONObject> eligible = new ArrayList<>();
            for (int i = 0; i < sourceCandidates.length(); i++) {
                JSONObject candidate = sourceCandidates.getJSONObject(i);
                boolean hasOutput = candidate.optInt("private_stream_config_count", 0) > 0 ||
                    candidate.optInt("yuv_420_888_stream_config_count", 0) > 0;
                if (candidate.optBoolean("has_lens_pose", false) &&
                        candidate.optBoolean("has_intrinsics", false) &&
                        hasOutput) {
                    eligible.add(candidate);
                }
            }

            JSONObject pair = new JSONObject();
            if (eligible.size() < 2) {
                pair.put("available", false);
                pair.put("reason", "fewer_than_two_pose_intrinsics_sources");
                pair.put("eligible_source_count", eligible.size());
                return pair;
            }

            eligible.sort(new Comparator<JSONObject>() {
                @Override
                public int compare(JSONObject left, JSONObject right) {
                    return Double.compare(lensTranslationX(left), lensTranslationX(right));
                }
            });
            JSONObject left = eligible.get(0);
            JSONObject right = eligible.get(eligible.size() - 1);
            pair.put("available", true);
            pair.put("selection_rule", "two_pose_intrinsics_sources_ordered_by_lens_pose_translation_x");
            pair.put("left_camera_id", left.optString("camera_id"));
            pair.put("right_camera_id", right.optString("camera_id"));
            pair.put("left_translation_x_m", lensTranslationX(left));
            pair.put("right_translation_x_m", lensTranslationX(right));
            pair.put("confidence", "metadata_heuristic_requires_visual_acceptance");
            return pair;
        }

        private JSONArray sourceCameraIds(JSONArray sourceCandidates) throws Exception {
            JSONArray ids = new JSONArray();
            for (int i = 0; i < sourceCandidates.length(); i++) {
                ids.put(sourceCandidates.getJSONObject(i).optString("camera_id"));
            }
            return ids;
        }

        private JSONArray copyArray(JSONArray array) throws Exception {
            return array == null ? new JSONArray() : new JSONArray(array.toString());
        }

        private boolean hasArrayLength(JSONObject json, String key, int length) {
            JSONArray array = json.optJSONArray(key);
            return array != null && array.length() >= length;
        }

        private int countStreamConfigurations(JSONArray configurations, String formatName) throws Exception {
            int count = 0;
            if (configurations == null) {
                return count;
            }
            for (int i = 0; i < configurations.length(); i++) {
                if (formatName.equals(configurations.getJSONObject(i).optString("format_name"))) {
                    count++;
                }
            }
            return count;
        }

        private JSONObject maxStreamSize(JSONArray configurations, String formatName) throws Exception {
            JSONObject best = null;
            long bestArea = -1L;
            if (configurations == null) {
                return null;
            }
            for (int i = 0; i < configurations.length(); i++) {
                JSONObject configuration = configurations.getJSONObject(i);
                if (!formatName.equals(configuration.optString("format_name"))) {
                    continue;
                }
                long area = (long) configuration.optInt("width", 0) * (long) configuration.optInt("height", 0);
                if (area > bestArea) {
                    bestArea = area;
                    best = new JSONObject();
                    best.put("width", configuration.optInt("width", 0));
                    best.put("height", configuration.optInt("height", 0));
                    best.put("direction", configuration.optString("direction"));
                }
            }
            return best;
        }

        private int maxAeFps(JSONArray rows) throws Exception {
            int max = 0;
            if (rows == null) {
                return max;
            }
            for (int i = 0; i < rows.length(); i++) {
                JSONArray row = rows.getJSONArray(i);
                for (int j = 0; j < row.length(); j++) {
                    max = Math.max(max, row.optInt(j, 0));
                }
            }
            return max;
        }

        private JSONObject findOpenAttempt(String cameraId) throws Exception {
            JSONArray attempts = latestShellCameraOpenProbe.optJSONArray("attempts");
            if (attempts == null) {
                return null;
            }
            for (int i = 0; i < attempts.length(); i++) {
                JSONObject attempt = attempts.getJSONObject(i);
                if (cameraId.equals(attempt.optString("camera_id"))) {
                    return attempt;
                }
            }
            return null;
        }

        private JSONObject findAppCaptureAttempt(String cameraId) throws Exception {
            JSONArray attempts = latestAppCameraProbe.optJSONArray("attempts");
            if (attempts == null) {
                return null;
            }
            for (int i = 0; i < attempts.length(); i++) {
                JSONObject attempt = attempts.getJSONObject(i);
                if (cameraId.equals(attempt.optString("camera_id"))) {
                    return attempt;
                }
            }
            return null;
        }

        private double lensTranslationX(JSONObject candidate) {
            JSONArray translation = candidate.optJSONArray("lens_pose_translation_m");
            return translation != null && translation.length() > 0 ? translation.optDouble(0, 0.0) : 0.0;
        }

        synchronized JSONObject setSourceEyeMapping(String nextSourceEyeMapping) throws Exception {
            if (nextSourceEyeMapping != null && nextSourceEyeMapping.trim().length() > 0) {
                sourceEyeMapping = nextSourceEyeMapping.trim();
                revision++;
            }
            return projectionProfileJson();
        }

        synchronized JSONObject setTextureTransform(String nextLeft, String nextRight) throws Exception {
            if (nextLeft != null && nextLeft.trim().length() > 0) {
                leftTextureTransform = nextLeft.trim();
            }
            if (nextRight != null && nextRight.trim().length() > 0) {
                rightTextureTransform = nextRight.trim();
            }
            revision++;
            return projectionProfileJson();
        }

        synchronized JSONObject recordVisualAcceptance(boolean accepted, String note, String source) throws Exception {
            visualReleaseAccepted = accepted;
            visualAcceptanceNote = note != null ? note : "";
            visualAcceptanceSource = source != null ? source : "";
            visualAcceptanceUnixMs = System.currentTimeMillis();
            revision++;
            return projectionProfileJson();
        }

        synchronized JSONObject recordAppCameraProbe(JSONObject probe) throws Exception {
            if (probe != null) {
                latestAppCameraProbe = new JSONObject(probe.toString());
                appCameraProbeUnixMs = System.currentTimeMillis();
                revision++;
            }
            return toStatusJson();
        }
    }

    private static final class ShellHelperState {
        private boolean connected;
        private String helperVersion = "";
        private String uid = "";
        private JSONArray capabilities = new JSONArray();
        private JSONArray activeStreams = new JSONArray();
        private JSONObject diagnostics = new JSONObject();
        private long lastHeartbeatUnixMs;
        private String lastError = "";

        synchronized boolean isConnected() {
            return connected;
        }

        synchronized JSONObject diagnosticsJson() throws Exception {
            return new JSONObject(diagnostics.toString());
        }

        synchronized JSONObject toStatusJson() throws Exception {
            JSONObject status = new JSONObject();
            status.put("schema", "rusty.xr.shell_helper.status.v1");
            status.put("connected", connected);
            status.put("helper_version", helperVersion);
            status.put("uid", uid);
            status.put("capabilities", new JSONArray(capabilities.toString()));
            status.put("active_streams", new JSONArray(activeStreams.toString()));
            status.put("last_heartbeat_unix_ms", lastHeartbeatUnixMs);
            status.put("last_error", lastError);
            if (diagnostics.length() > 0) {
                status.put("diagnostics", new JSONObject(diagnostics.toString()));
            }
            status.put("requires_adb_authorization", true);
            status.put("normal_broker_apk_is_shell", false);
            return status;
        }

        synchronized JSONObject reportStatus(JSONObject params) throws Exception {
            connected = params == null || params.optBoolean("connected", true);
            helperVersion = params != null ? params.optString("helper_version", helperVersion) : helperVersion;
            uid = params != null ? params.optString("uid", uid) : uid;
            JSONArray nextCapabilities = params != null ? params.optJSONArray("capabilities") : null;
            if (nextCapabilities != null) {
                capabilities = new JSONArray(nextCapabilities.toString());
            }
            JSONArray nextActiveStreams = params != null ? params.optJSONArray("active_streams") : null;
            if (nextActiveStreams != null) {
                activeStreams = new JSONArray(nextActiveStreams.toString());
            }
            JSONObject nextDiagnostics = params != null ? params.optJSONObject("diagnostics") : null;
            if (nextDiagnostics != null) {
                diagnostics = new JSONObject(nextDiagnostics.toString());
            }
            lastError = params != null ? params.optString("last_error", "") : "";
            lastHeartbeatUnixMs = System.currentTimeMillis();
            return toStatusJson();
        }
    }

    private static final class VideoLabState {
        private JSONObject latestEncodedStreamManifest = new JSONObject();
        private JSONObject latestEncodedSampleMetadata = new JSONObject();
        private JSONObject latestMetricSample = new JSONObject();

        synchronized JSONObject toStatusJson(
            long acceptedMetricSamples,
            long acceptedEncodedStreamManifests,
            long acceptedEncodedSampleMetadata) throws Exception {
            boolean hasManifest = latestEncodedStreamManifest.length() > 0;
            String codec = latestEncodedStreamManifest.optString("codec", "");
            String payloadTransport = latestEncodedStreamManifest.optString("payload_transport", "");
            boolean hasPayloadTransport = payloadTransport.length() > 0 && !"metadata_only".equals(payloadTransport);
            boolean metricMatchesManifest = latestMetricSample.length() > 0 &&
                latestMetricSample.optString("session_id", "").equals(latestEncodedStreamManifest.optString("session_id", ""));
            boolean payloadMetricSucceeded = metricMatchesManifest &&
                !latestMetricSample.has("last_error") &&
                latestMetricSample.optInt("packet_count", 0) > 0 &&
                latestMetricSample.optLong("payload_size_bytes", 0L) > 0L;
            boolean payloadTransportReady = hasPayloadTransport && payloadMetricSucceeded;
            boolean rawLumaProbe = "raw_luma8".equals(codec);
            boolean h264Probe = "h264".equals(codec);
            boolean h264DecodeProbe = "broker_app_camera2_mediacodec_decode_probe"
                .equals(latestEncodedStreamManifest.optString("source", ""));

            JSONObject status = new JSONObject();
            status.put("schema", "rusty.xr.video_lab.status.v1");
            status.put("state", payloadTransportReady
                ? "payload_transport_ready"
                : acceptedEncodedStreamManifests > 0
                ? "encoded_metadata_ready"
                : "metrics_only");
            status.put("metric_stream", "video_lab.metric_sample");
            status.put("encoded_stream_manifest_stream", "video_lab.encoded_stream_manifest");
            status.put("encoded_sample_metadata_stream", "video_lab.encoded_sample_metadata");
            status.put("accepted_metric_samples", acceptedMetricSamples);
            status.put("accepted_encoded_stream_manifests", acceptedEncodedStreamManifests);
            status.put("accepted_encoded_sample_metadata", acceptedEncodedSampleMetadata);

            JSONObject payloadSchemas = new JSONObject();
            payloadSchemas.put("metric", "rusty.xr.video_lab.metric_sample.v1");
            payloadSchemas.put("encoded_stream_manifest", "rusty.xr.video_lab.encoded_stream_manifest.v1");
            payloadSchemas.put("encoded_sample_metadata", "rusty.xr.video_lab.encoded_sample_metadata.v1");
            status.put("payload_schemas", payloadSchemas);
            status.put("payload_schema", "rusty.xr.video_lab.metric_sample.v1");

            JSONArray timestampFields = new JSONArray();
            timestampFields.put("source_time_unix_ns");
            timestampFields.put("source_time_elapsed_ns");
            timestampFields.put("broker_receive_time_unix_ns");
            timestampFields.put("broker_publish_time_unix_ns");
            timestampFields.put("client_receive_time_unix_ns");
            timestampFields.put("decoder_output_time_unix_ns");
            timestampFields.put("texture_available_time_unix_ns");
            timestampFields.put("xr_submit_time_unix_ns");
            status.put("timestamp_fields", timestampFields);

            JSONArray transportHints = new JSONArray();
            transportHints.put("metadata_over_json_websocket");
            transportHints.put(payloadTransportReady ? "payload_transport_ready" : "encoded_payload_binary_transport_pending");
            transportHints.put(rawLumaProbe ? "client_owned_raw_texture_upload" : "surface_decode_client_owned");
            if (h264DecodeProbe && payloadTransportReady) {
                transportHints.put("android_mediacodec_decode_consumption_verified");
            }
            transportHints.put("xr_layer_submit_client_owned");
            status.put("transport_hints", transportHints);

            if (latestEncodedStreamManifest.length() > 0) {
                status.put("latest_encoded_stream_manifest", new JSONObject(latestEncodedStreamManifest.toString()));
            }
            if (latestEncodedSampleMetadata.length() > 0) {
                status.put("latest_encoded_sample_metadata", new JSONObject(latestEncodedSampleMetadata.toString()));
            }
            if (latestMetricSample.length() > 0) {
                status.put("latest_metric_sample", new JSONObject(latestMetricSample.toString()));
            }

            JSONArray limitations = new JSONArray();
            if (!hasManifest && acceptedEncodedSampleMetadata == 0) {
                limitations.put("metric_contract_only");
            }
            limitations.put("no_high_rate_payload_over_json_websocket");
            if (rawLumaProbe) {
                limitations.put("raw_luma_probe_not_encoded_video");
            } else if (!h264Probe || !payloadTransportReady) {
                limitations.put("encoded_video_source_pending");
            }
            if (!payloadTransportReady) {
                limitations.put("encoded_payload_transport_pending");
            } else {
                limitations.put("payload_transport_is_probe_only");
            }
            if (h264DecodeProbe) {
                limitations.put("decode_probe_outputs_byte_buffers_not_xr_textures");
            }
            limitations.put(rawLumaProbe
                ? "client_owned_texture_upload_and_xr_submit"
                : "client_owned_decode_texture_and_xr_submit");
            status.put("limitations", limitations);
            return status;
        }

        synchronized JSONObject registerEncodedStreamManifest(
            JSONObject params,
            long revision,
            long receiveUnixNs,
            long receiveElapsedNs) throws Exception {
            JSONObject manifest = new JSONObject(params.toString());
            if (!manifest.has("schema")) {
                manifest.put("schema", "rusty.xr.video_lab.encoded_stream_manifest.v1");
            }
            if (!manifest.has("stream_id")) {
                manifest.put("stream_id", "video_lab.synthetic_encoded");
            }
            if (!manifest.has("session_id")) {
                manifest.put("session_id", "video-lab-session-" + revision);
            }
            if (!manifest.has("source")) {
                manifest.put("source", "synthetic");
            }
            if (!manifest.has("transport")) {
                manifest.put("transport", "metadata_only");
            }
            if (!manifest.has("mime_type")) {
                manifest.put("mime_type", manifest.optString("codec", "video/avc"));
            }
            if (!manifest.has("codec")) {
                manifest.put("codec", manifest.optString("mime_type", "video/avc"));
            }
            if (!manifest.has("width")) {
                manifest.put("width", 0);
            }
            if (!manifest.has("height")) {
                manifest.put("height", 0);
            }
            if (!manifest.has("frame_rate_hz")) {
                manifest.put("frame_rate_hz", 0);
            }
            if (!manifest.has("bitrate_bps")) {
                manifest.put("bitrate_bps", 0);
            }
            manifest.put("revision", revision);
            manifest.put("broker_receive_time_unix_ns", receiveUnixNs);
            manifest.put("broker_receive_time_elapsed_ns", receiveElapsedNs);
            manifest.put("broker_publish_time_unix_ns", unixNowNs());
            manifest.put("broker_publish_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            latestEncodedStreamManifest = new JSONObject(manifest.toString());
            return new JSONObject(manifest.toString());
        }

        synchronized JSONObject recordEncodedSampleMetadata(
            JSONObject params,
            long sequence,
            long receiveUnixNs,
            long receiveElapsedNs) throws Exception {
            JSONObject sample = new JSONObject(params.toString());
            if (!sample.has("schema")) {
                sample.put("schema", "rusty.xr.video_lab.encoded_sample_metadata.v1");
            }
            if (!sample.has("stream_id")) {
                sample.put("stream_id", "video_lab.synthetic_encoded");
            }
            if (!sample.has("session_id")) {
                sample.put("session_id", latestEncodedStreamManifest.optString("session_id", "video-lab-session-unknown"));
            }
            if (!sample.has("sequence_id")) {
                sample.put("sequence_id", sequence);
            }
            if (!sample.has("transport")) {
                sample.put("transport", "metadata_only");
            }
            if (!sample.has("mime_type")) {
                sample.put("mime_type", latestEncodedStreamManifest.optString("mime_type", "video/avc"));
            }
            if (!sample.has("codec")) {
                sample.put("codec", sample.optString("mime_type", "video/avc"));
            }
            if (!sample.has("encoded_size_bytes")) {
                sample.put("encoded_size_bytes", 0);
            }
            if (!sample.has("key_frame")) {
                sample.put("key_frame", false);
            }
            if (!sample.has("source_time_unix_ns")) {
                sample.put("source_time_unix_ns", receiveUnixNs);
            }
            if (!sample.has("source_time_elapsed_ns")) {
                sample.put("source_time_elapsed_ns", receiveElapsedNs);
            }
            sample.put("broker_receive_time_unix_ns", receiveUnixNs);
            sample.put("broker_receive_time_elapsed_ns", receiveElapsedNs);
            sample.put("broker_publish_time_unix_ns", unixNowNs());
            sample.put("broker_publish_time_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            latestEncodedSampleMetadata = new JSONObject(sample.toString());
            return new JSONObject(sample.toString());
        }

        synchronized JSONObject recordMetricSample(JSONObject params) throws Exception {
            latestMetricSample = new JSONObject(params.toString());
            return new JSONObject(latestMetricSample.toString());
        }
    }
}
