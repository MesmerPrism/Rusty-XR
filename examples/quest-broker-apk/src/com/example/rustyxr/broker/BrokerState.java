package com.example.rustyxr.broker;

import android.os.SystemClock;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.atomic.AtomicLong;

final class BrokerState {
    static final String BROKER_VERSION = "0.1.0-public-proof";
    static final int PROTOCOL_VERSION = 1;
    static final String CONTRACT_VERSION = "rusty.manifold.broker.v1";
    static final String LEGACY_RUSTY_XR_CONTRACT_VERSION = "rusty.xr.broker.v1";
    static final String MANIFOLD_COMMAND_SCHEMA = "rusty.manifold.command.envelope.v1";
    static final String LEGACY_RUSTY_XR_BROKER_COMMAND_SCHEMA = "rusty.xr.broker.command.v1";
    static final String MANIFOLD_COMMAND_ACK_SCHEMA = "rusty.manifold.command_ack.v1";
    static final String LEGACY_RUSTY_XR_BROKER_COMMAND_ACK_SCHEMA = "rusty.xr.broker.command_ack.v1";
    static final String MANIFOLD_STREAM_EVENT_SCHEMA = "rusty.manifold.stream_event.v1";
    static final String LEGACY_RUSTY_XR_BROKER_STREAM_EVENT_SCHEMA = "rusty.xr.broker.stream_event.v1";
    static final String MANIFOLD_EVENTS_PATH = "/manifold/v1/events";
    static final String LEGACY_RUSTY_XR_EVENTS_PATH = "/rustyxr/v1/events";
    static final String KIOSK_CONTROL_PLANE_STATUS_SCHEMA = "rusty.xr.kiosk.control_plane.v1";
    static final String KIOSK_COMMAND_EVIDENCE_SCHEMA = "rusty.xr.kiosk.command_evidence.v1";
    static final String KIOSK_COMMAND_RUN_RECORD_SCHEMA = "rusty.xr.kiosk.command_run_record.v1";
    static final String COMMAND_REJECTION_SCHEMA = "rusty.xr.broker.command_rejection.v1";
    static final String CONTROL_SCOPE_SCHEMA = "rusty.xr.broker.control_scope.v1";
    static final String CONTROL_LEASE_SCHEMA = "rusty.xr.broker.control_lease.v1";
    static final String CONTROL_LEASE_REQUEST_SCHEMA = "rusty.xr.broker.control_lease_request.v1";
    static final String CONTROL_LEASE_RELEASE_SCHEMA = "rusty.xr.broker.control_lease_release.v1";
    static final String CONTROL_LEASE_REQUEST_COMMAND = "control_lease.request";
    static final String CONTROL_LEASE_RELEASE_COMMAND = "control_lease.release";
    static final String STREAM_REGISTRY_SNAPSHOT_SCHEMA = "rusty.xr.broker.stream_registry_snapshot.v1";
    static final String HOST_MANIFEST_SCHEMA = "rusty.xr.broker.host_manifest.v1";
    static final String HOST_MANIFEST_COMMAND = "broker.host_manifest";
    static final String HOST_MANIFEST_HTTP_PATH = "/broker/host_manifest";
    private static final long DEFAULT_CONTROL_LEASE_DURATION_ELAPSED_NS = 60_000_000_000L;

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
    private final AtomicLong controlLeaseSequence = new AtomicLong();
    private final AtomicLong controlLeaseRevision = new AtomicLong();
    private final Object controlLeaseLock = new Object();
    private final LinkedHashMap<String, JSONObject> activeControlLeases = new LinkedHashMap<>();
    private final CameraProjectionProviderState cameraProjectionProvider = new CameraProjectionProviderState();
    private final ShellHelperState shellHelper = new ShellHelperState();
    private final ExperimentControlState experimentControl = new ExperimentControlState();
    private final VideoLabState videoLab = new VideoLabState();
    private final TransportSessionRegistry transportSessions = new TransportSessionRegistry();
    private final BreathAssessmentState breathAssessment = new BreathAssessmentState();
    private final ClockCore clock = new ClockCore();
    private JSONObject polarHeartRateStatus = defaultPolarHeartRateStatus();
    private JSONObject polarPmdStatus = defaultPolarPmdStatus();
    private JSONObject deviceWatchdogStatus = defaultDeviceWatchdogStatus();

    JSONObject toStatusJson(LatencyPublisher publisher, OscIngressServer oscIngressServer) throws Exception {
        JSONObject status = new JSONObject();
        status.put("type", "status");
        status.put("brokerVersion", BROKER_VERSION);
        status.put("protocolVersion", PROTOCOL_VERSION);
        status.put("contractVersion", CONTRACT_VERSION);
        status.put("legacyContractVersion", LEGACY_RUSTY_XR_CONTRACT_VERSION);
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
        status.put("experimentControl", experimentControl.toStatusJson());
        status.put("polarHeartRate", polarHeartRateStatusJson());
        status.put("polarPmd", polarPmdStatusJson());
        status.put("breathAssessment", breathAssessment.toStatusJson());
        status.put("videoLab", videoLabStatusJson());
        status.put("deviceWatchdog", deviceWatchdogStatusJson());
        status.put("transportSessions", transportSessions.statusJson());
        status.put("q2qRelay", BrokerQ2QRelayClientSession.statusJson(null));
        status.put("clock", clock.statusJson());
        JSONObject kioskStatus = rustyKioskStatusJson();
        status.put("rustyKiosk", kioskStatus);
        status.put(
            "kioskCommandRunRecord",
            rustyKioskCommandRunRecordJson(
                "broker-http-status",
                "GET /status",
                JSONObject.NULL,
                kioskStatus,
                "broker_http_status_snapshot"));

        JSONObject commands = new JSONObject();
        commands.put("schema", MANIFOLD_COMMAND_SCHEMA);
        commands.put("legacySchema", LEGACY_RUSTY_XR_BROKER_COMMAND_SCHEMA);
        commands.put("acceptedSchemas", jsonArrayOf(
            MANIFOLD_COMMAND_SCHEMA,
            LEGACY_RUSTY_XR_BROKER_COMMAND_SCHEMA));
        commands.put("ackSchema", MANIFOLD_COMMAND_ACK_SCHEMA);
        commands.put("legacyAckSchema", LEGACY_RUSTY_XR_BROKER_COMMAND_ACK_SCHEMA);
        JSONArray supportedCommands = new JSONArray();
        supportedCommands.put("status_request");
        supportedCommands.put("list_capabilities");
        supportedCommands.put("list_streams");
        supportedCommands.put("stream_registry.snapshot");
        supportedCommands.put(HOST_MANIFEST_COMMAND);
        supportedCommands.put(CONTROL_LEASE_REQUEST_COMMAND);
        supportedCommands.put(CONTROL_LEASE_RELEASE_COMMAND);
        supportedCommands.put("subscribe");
        supportedCommands.put("unsubscribe");
        supportedCommands.put("configure_osc_ingress");
        supportedCommands.put("publish_stream_event");
        supportedCommands.put("breath_feedback.received");
        supportedCommands.put("clock.status");
        supportedCommands.put("clock.now");
        supportedCommands.put("clock.domains");
        supportedCommands.put("clock.correlations");
        supportedCommands.put("clock.health");
        supportedCommands.put("clock.compare_openxr");
        supportedCommands.put("clock.sync_probe");
        supportedCommands.put("lsl.capture_string");
        supportedCommands.put("kiosk.get_status");
        supportedCommands.put("polar.get_status");
        supportedCommands.put("polar.start");
        supportedCommands.put("polar.stop");
        supportedCommands.put("polar_hr.get_status");
        supportedCommands.put("polar_hr.start");
        supportedCommands.put("polar_hr.stop");
        supportedCommands.put("polar_pmd.get_status");
        supportedCommands.put("polar_pmd.start");
        supportedCommands.put("polar_pmd.stop");
        supportedCommands.put("breath_assessment.get_status");
        supportedCommands.put("breath_assessment.configure");
        supportedCommands.put("breath_assessment.reset");
        supportedCommands.put("breath_assessment.submit_controller_pose");
        supportedCommands.put("device_watchdog.get_status");
        supportedCommands.put("device_watchdog.start");
        supportedCommands.put("device_watchdog.stop");
        supportedCommands.put("device_watchdog.mark");
        supportedCommands.put("set_polar_breath_params");
        supportedCommands.put("polar_breath_calibrate_begin");
        supportedCommands.put("polar_breath_calibrate_reset");
        supportedCommands.put("open_ui");
        supportedCommands.put("close_ui");
        supportedCommands.put("camera_provider.get_status");
        supportedCommands.put("camera_provider.get_projection_profile");
        supportedCommands.put("camera_provider.run_app_camera_probe");
        supportedCommands.put("camera_provider.start_app_camera_luma_stream");
        supportedCommands.put("camera_provider.start_app_camera_h264_stream");
        supportedCommands.put("camera_provider.run_app_camera_h264_decode_probe");
        supportedCommands.put("media.request_keyframe");
        supportedCommands.put("media.set_video_bitrate");
        supportedCommands.put("media.set_quality_profile");
        supportedCommands.put("media.start_synthetic_h264_stream");
        supportedCommands.put("media.start_h264_tcp_proxy");
        supportedCommands.put("media.run_h264_tcp_proxy_probe");
        supportedCommands.put("q2q_relay.start_sender");
        supportedCommands.put("q2q_relay.start_receiver");
        supportedCommands.put("q2q_relay.get_status");
        supportedCommands.put("q2q_relay.stop");
        supportedCommands.put("camera_provider.set_source_eye_mapping");
        supportedCommands.put("camera_provider.set_texture_transform");
        supportedCommands.put("camera_provider.record_visual_acceptance");
        supportedCommands.put("shell_helper.get_status");
        supportedCommands.put("shell_helper.report_status");
        supportedCommands.put("experiment.get_control");
        supportedCommands.put("experiment.configure");
        supportedCommands.put("experiment.report_status");
        supportedCommands.put("transport.describe_capabilities");
        supportedCommands.put("transport.create_session");
        supportedCommands.put("transport.get_session");
        supportedCommands.put("transport.list_sessions");
        supportedCommands.put("transport.close_session");
        supportedCommands.put("video_lab.get_status");
        supportedCommands.put("video_lab.get_scorecard");
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
        capabilities.put("manifold.websocket.events");
        capabilities.put("manifold.command.envelope.v1");
        capabilities.put("manifold.command_ack.v1");
        capabilities.put("manifold.stream_event.v1");
        capabilities.put("websocket.events");
        capabilities.put("websocket.control");
        capabilities.put("http.status");
        capabilities.put("broker.command.v1");
        capabilities.put("broker.command.v1.legacy_rusty_xr_compat");
        capabilities.put("broker.command_rejection.v1");
        capabilities.put("broker.control_lease.v1");
        capabilities.put("broker.control_lease_request.v1");
        capabilities.put("broker.control_lease_release.v1");
        capabilities.put("broker.subscription.v1");
        capabilities.put("broker.stream_event.v1");
        capabilities.put("broker.stream_registry_snapshot.v1");
        capabilities.put("broker.host_manifest.v1");
        capabilities.put("broker.host_manifest.read");
        capabilities.put("broker.osc_ingress.configure");
        capabilities.put("broker.stream_event.publish");
        capabilities.put("broker.clock.status.v1");
        capabilities.put("broker.clock.snapshot.v1");
        capabilities.put("broker.clock.stamp.v1");
        capabilities.put("broker.clock.correlation.v1");
        capabilities.put("broker.clock.sync_probe.v1");
        capabilities.put("lsl.string_capture.v1");
        capabilities.put("broker.device_watchdog.control.v1");
        capabilities.put("rusty.xr.device_watchdog.status.v1");
        capabilities.put("rusty.xr.device_watchdog.sample_log.v1");
        capabilities.put("rusty_kiosk.control_plane.status.v1");
        capabilities.put("rusty.xr.kiosk.command_run_record.v1");
        capabilities.put("bio.polar_hr.android_ble.v1");
        capabilities.put("bio.polar_dual_receiver.hr_rr.v1");
        capabilities.put("bio.polar.pmd_optional.v1");
        capabilities.put("bio.polar_pmd.android_ble.v1");
        capabilities.put("bio.polar_acc.direct_ble.v1");
        capabilities.put("bio.breath_assessment.v1");
        capabilities.put("bio.breath.polar_acc.v1");
        capabilities.put("bio.breath.controller_pose.v1");
        capabilities.put("broker.console.activity");
        capabilities.put("broker.console.return_to_previous_app");
        capabilities.put("broker.console.close_command");
        capabilities.put("broker.console.initial_page.v1");
        capabilities.put("broker.system_panel.shortcuts.v1");
        capabilities.put("broker.launcher.local_lists.v1");
        capabilities.put("broker.launcher.package_manager_launch.v1");
        capabilities.put("broker.experiment_control.v1");
        capabilities.put("broker.experiment_control.makepad_tuning.v1");
        capabilities.put("broker.experiment_control.focus_guardian.v1");
        capabilities.put("camera_projection.metadata.v1");
        capabilities.put("camera_projection.profile.v1");
        capabilities.put("camera_projection.visual_acceptance.v1");
        capabilities.put("camera_projection.app_camera_probe.v1");
        capabilities.put("camera_projection.app_camera_luma_stream.v1");
        capabilities.put("camera_projection.app_camera_h264_stream.v1");
        capabilities.put("camera_projection.app_camera_h264_decode_probe.v1");
        capabilities.put("media.h264_runtime_keyframe.v1");
        capabilities.put("media.h264_runtime_bitrate.v1");
        capabilities.put("media.h264_quality_profile.v1");
        capabilities.put("media.synthetic_h264_stream.v1");
        capabilities.put("broker.h264_tcp_proxy.v1");
        capabilities.put("broker.h264_tcp_proxy_probe.v1");
        capabilities.put("broker.q2q_relay.native.v1");
        capabilities.put("broker.q2q_relay.sender.v1");
        capabilities.put("broker.q2q_relay.receiver.v1");
        capabilities.put("broker.q2q_relay.synthetic_h264.v1");
        capabilities.put("broker.q2q_relay.camera_h264.v1");
        capabilities.put("broker.q2q_relay.stream_stats.v1");
        capabilities.put("broker.q2q_relay.quality_ladder.v1");
        capabilities.put("broker.h264.frame_set_gate.v1");
        capabilities.put("broker.lan_control.opt_in.v1");
        capabilities.put("shell_helper.status.v1");
        capabilities.put("broker.transport.session_control.v1");
        capabilities.put("broker.transport.security_policy.v1");
        capabilities.put("broker.transport.metrics.v1");
        capabilities.put("video_lab.metrics.v1");
        capabilities.put("video_lab.encoded_stream_manifest.v1");
        capabilities.put("video_lab.encoded_sample_metadata.v1");
        capabilities.put("video_lab.scorecard.v1");
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
        streams.put(streamJson("clock:sample", "clock", "Broker-owned monotonic clock snapshots.", true));
        streams.put(streamJson("clock:health", "clock", "Clock health, wall-clock jump, and discontinuity events.", true));
        streams.put(streamJson("clock:correlation", "clock", "Timestamp-domain correlation updates.", true));
        streams.put(streamJson("clock:openxr_frame", "clock", "OpenXR predicted-display timing samples when an immersive Rusty XR session publishes them.", false));
        streams.put(streamJson("device_watchdog.status", "diagnostic", "On-device broker watchdog status and retention-limited sample log metadata.", deviceWatchdogStatus.optBoolean("running", false)));
        streams.put(streamJson("kiosk:control_plane", "control", "Rusty Kiosk phase, surface-intent, helper, and command evidence.", true));
        streams.put(streamJson("latency:sample", "latency", "WebSocket latency samples accepted by the broker.", true));
        streams.put(streamJson("diagnostics:termux_python", "diagnostic", "External Linux/Python sidecar feedback samples accepted by the broker.", publishedStreamEvents.get() > 0));
        streams.put(streamJson("bio:polar_hr_rr", "bio", "Synthetic, adapter-published, or direct Android BLE Polar heart-rate/RR events.", true));
        streams.put(streamJson("bio:polar_ecg", "bio", "Synthetic or adapter-published Polar-compatible ECG frame events.", true));
        streams.put(streamJson(
            "bio:polar_acc",
            "bio",
            "Synthetic, adapter-published, or direct Android BLE Polar PMD accelerometer frame events.",
            true));
        streams.put(streamJson("bio:breath", "bio", "Diagnostic breath volume/state assessments produced from supported motion sources.", breathAssessment.hasAssessments()));
        streams.put(streamJson("stream.motion.object_pose", "motion", "Source-agnostic object pose samples accepted for motion-derived breath assessment.", true));
        streams.put(streamJson("xr:controller_pose", "xr", "Legacy adapter-published controller pose samples accepted for broker-side breath assessment.", true));
        streams.put(streamJson("camera_provider.status", "camera", "Projection metadata provider status and limitations.", true));
        streams.put(streamJson("camera_provider.projection_profile", "camera", "Projection profile changes for XR clients that render their own layers.", true));
        streams.put(streamJson("camera_provider.visual_acceptance", "camera", "Operator visual-acceptance markers for projection profiles.", true));
        streams.put(streamJson("shell_helper.status", "shell_helper", "ADB-launched shell-helper status when a helper is connected.", shellHelper.isConnected()));
        streams.put(streamJson("experiment.control", "control", "Experiment target, tuning, and focus-guardian control state.", true));
        streams.put(streamJson("transport.session_created", "transport", "Transport session creation events.", transportSessions.createdCount() > 0));
        streams.put(streamJson("transport.session_closed", "transport", "Transport session close events.", transportSessions.closedCount() > 0));
        streams.put(streamJson("transport.session_failed", "transport", "Transport session failure events.", transportSessions.failedCount() > 0));
        streams.put(streamJson("q2q_relay.status", "transport", "Native Quest relay sender/receiver lane status.", true));
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

    JSONObject streamRegistrySnapshotJson(OscIngressServer oscIngressServer) throws Exception {
        JSONArray streams = streamsJson(oscIngressServer);
        LinkedHashMap<String, JSONArray> providerStreams = new LinkedHashMap<>();
        LinkedHashMap<String, String> providerModuleIds = new LinkedHashMap<>();
        LinkedHashMap<String, String> providerModuleKinds = new LinkedHashMap<>();
        LinkedHashMap<String, Boolean> providerActive = new LinkedHashMap<>();
        for (int index = 0; index < streams.length(); index++) {
            JSONObject stream = streams.optJSONObject(index);
            if (stream == null) {
                continue;
            }
            String streamId = stream.optString("id", "");
            String kind = stream.optString("kind", "unknown");
            String providerId = providerIdForStream(streamId, kind);
            JSONArray streamIds = providerStreams.get(providerId);
            if (streamIds == null) {
                streamIds = new JSONArray();
                providerStreams.put(providerId, streamIds);
            }
            streamIds.put(streamId);
            providerModuleIds.put(providerId, moduleIdForStream(streamId, kind));
            providerModuleKinds.put(providerId, moduleKindForStream(streamId, kind));
            Boolean wasActive = providerActive.get(providerId);
            providerActive.put(
                providerId,
                Boolean.valueOf((wasActive != null && wasActive.booleanValue())
                    || stream.optBoolean("active", false)));
        }

        JSONArray providers = new JSONArray();
        for (Map.Entry<String, JSONArray> entry : providerStreams.entrySet()) {
            JSONObject provider = new JSONObject();
            provider.put("provider_id", entry.getKey());
            provider.put("label", providerLabel(entry.getKey()));
            provider.put("module_id", nullableJsonString(providerModuleIds.get(entry.getKey())));
            provider.put("module_kind", nullableJsonString(providerModuleKinds.get(entry.getKey())));
            provider.put(
                "state",
                Boolean.TRUE.equals(providerActive.get(entry.getKey())) ? "active" : "idle");
            provider.put("data_sensitivity", providerSensitivity(entry.getKey()));
            provider.put("stream_ids", entry.getValue());
            providers.put(provider);
        }

        JSONArray registeredStreams = new JSONArray();
        for (int index = 0; index < streams.length(); index++) {
            JSONObject stream = streams.optJSONObject(index);
            if (stream == null) {
                continue;
            }
            String streamId = stream.optString("id", "");
            String kind = stream.optString("kind", "custom");
            JSONObject descriptor = new JSONObject();
            descriptor.put("stream_id", streamId);
            descriptor.put("label", streamId);
            descriptor.put("provider_id", providerIdForStream(streamId, kind));
            descriptor.put("module_id", moduleIdForStream(streamId, kind));
            descriptor.put("module_kind", moduleKindForStream(streamId, kind));
            descriptor.put("stream_kind", streamKindForStream(streamId, kind));
            descriptor.put("payload_kind", payloadKindForStream(streamId));
            descriptor.put("payload_schema", payloadSchemaForStream(streamId));
            descriptor.put("metrics", metricsForStream(streamId));
            descriptor.put("recommended_rate_hz", recommendedRateForStream(streamId, kind));
            descriptor.put("rate_class", rateClassForStream(streamId, kind));
            descriptor.put("data_sensitivity", dataSensitivityForStream(streamId, kind));
            descriptor.put("retention_policy", retentionPolicyForStream(streamId, kind));
            descriptor.put("ui_subscription_policy", uiSubscriptionPolicyForStream(streamId, kind));
            descriptor.put("chart_policy", chartPolicyForStream(streamId, kind));
            registeredStreams.put(descriptor);
        }

        JSONObject breathAdapter = new JSONObject();
        breathAdapter.put("adapter_id", "breath-assessment");
        breathAdapter.put("label", "Breath assessment");
        breathAdapter.put("module_id", "bio.breath_assessment");
        breathAdapter.put("module_kind", "processor");
        breathAdapter.put("state", breathAssessment.hasAssessments() ? "active" : "idle");
        breathAdapter.put("input_stream_ids", jsonArrayOf("bio:polar_acc", "stream.motion.object_pose", "xr:controller_pose"));
        breathAdapter.put("output_stream_ids", jsonArrayOf("bio:breath"));

        JSONObject videoAdapter = new JSONObject();
        videoAdapter.put("adapter_id", "video-lab");
        videoAdapter.put("label", "Video lab metadata");
        videoAdapter.put("module_id", "video.lab");
        videoAdapter.put("module_kind", "diagnostic");
        videoAdapter.put("state", videoLabMetricSamples.get() > 0 ? "active" : "idle");
        videoAdapter.put("input_stream_ids", jsonArrayOf("video_lab.encoded_sample_metadata"));
        videoAdapter.put("output_stream_ids", jsonArrayOf("video_lab.metric_sample"));

        JSONObject subscriber = new JSONObject();
        subscriber.put("subscriber_id", "broker-websocket-clients");
        subscriber.put("label", "WebSocket clients");
        subscriber.put("transport", "WebSocket");
        subscriber.put("stream_ids", activeStreamIds(streams));

        JSONObject commandClient = new JSONObject();
        commandClient.put("client_id", "broker-websocket-command-clients");
        commandClient.put("label", "WebSocket command clients");
        commandClient.put("command_scopes", jsonArrayOf(
            "session.lifecycle",
            "runtime.bio",
            "runtime.visuals",
            "camera.preview",
            "diagnostics.watchdog",
            "transport.session"));
        commandClient.put("held_lease_ids", activeControlLeaseIds());

        long revision = streamRegistrySemanticRevision();
        JSONArray modules = modulesJson(streams, oscIngressServer, revision);

        JSONObject snapshot = new JSONObject();
        snapshot.put("schema", STREAM_REGISTRY_SNAPSHOT_SCHEMA);
        snapshot.put("broker_id", "quest-broker-example");
        snapshot.put("revision", revision);
        snapshot.put("captured_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        snapshot.put("modules", modules);
        snapshot.put("providers", providers);
        snapshot.put("streams", registeredStreams);
        snapshot.put("adapters", new JSONArray().put(breathAdapter).put(videoAdapter));
        snapshot.put("subscribers", new JSONArray().put(subscriber));
        snapshot.put("command_clients", new JSONArray().put(commandClient));
        snapshot.put("active_leases", activeControlLeasesJson());
        return snapshot;
    }

    JSONObject hostManifestJson(
        String bindHost,
        int port,
        LatencyPublisher publisher,
        OscIngressServer oscIngressServer) throws Exception {
        String normalizedBindHost = bindHost != null && bindHost.trim().length() > 0
            ? bindHost.trim()
            : "127.0.0.1";
        boolean loopbackOnly = isLoopbackHost(normalizedBindHost);
        String endpointVisibility = loopbackOnly ? "loopback" : "paired_lan";

        JSONObject security = new JSONObject();
        security.put("schema", "rusty.xr.broker.transport_security_policy.v1");
        security.put("mode", loopbackOnly ? "LoopbackOnly" : "ExternalSidecarOwned");
        security.put("non_loopback_allowed", !loopbackOnly);
        security.put("pairing_token_required", false);
        security.put("expires_elapsed_ns", JSONObject.NULL);
        security.put(
            "capability_scope",
            loopbackOnly ? new JSONArray() : jsonArrayOf("broker.lan_control.opt_in"));

        JSONArray endpoints = new JSONArray();
        endpoints.put(hostEndpointJson(
            "events-ws",
            "Manifold WebSocket events",
            "WebSocket",
            normalizedBindHost,
            port,
            MANIFOLD_EVENTS_PATH,
            JSONObject.NULL,
            false,
            endpointVisibility,
            "broker.control",
            true));
        endpoints.put(hostEndpointJson(
            "events-ws-legacy-rustyxr",
            "Legacy Rusty-XR WebSocket events compatibility",
            "WebSocket",
            normalizedBindHost,
            port,
            LEGACY_RUSTY_XR_EVENTS_PATH,
            JSONObject.NULL,
            false,
            endpointVisibility,
            "broker.control",
            false));
        endpoints.put(hostEndpointJson(
            "status-http",
            "Broker HTTP status",
            "Tcp",
            normalizedBindHost,
            port,
            "/status",
            JSONObject.NULL,
            false,
            endpointVisibility,
            "broker.status",
            false));
        endpoints.put(hostEndpointJson(
            "stream-registry-http",
            "Stream registry snapshot",
            "Tcp",
            normalizedBindHost,
            port,
            "/stream_registry/snapshot",
            JSONObject.NULL,
            false,
            endpointVisibility,
            "broker.stream_registry",
            false));
        endpoints.put(hostEndpointJson(
            "host-manifest-http",
            "Broker host manifest",
            "Tcp",
            normalizedBindHost,
            port,
            HOST_MANIFEST_HTTP_PATH,
            JSONObject.NULL,
            false,
            endpointVisibility,
            "broker.host_manifest",
            false));

        JSONObject manifest = new JSONObject();
        manifest.put("schema", HOST_MANIFEST_SCHEMA);
        manifest.put("host_id", "quest-broker-example");
        manifest.put("label", "Quest broker example");
        manifest.put("authority_role", "headset_local_primary");
        manifest.put("endpoints", endpoints);
        manifest.put("capabilities", capabilitiesJson(publisher, oscIngressServer));
        manifest.put("security", security);
        manifest.put("broker_clock_domain", "ElapsedRealtime");
        manifest.put("session_manifest_required", true);
        manifest.put("observed_elapsed_ns", SystemClock.elapsedRealtimeNanos());
        manifest.put(
            "notes",
            loopbackOnly
                ? jsonArrayOf("Loopback broker endpoint; host access normally uses an explicit forwarder.")
                : jsonArrayOf("Non-loopback bind is opt-in and should be paired or sidecar-gated by the operator."));
        return manifest;
    }

    private static JSONObject hostEndpointJson(
        String endpointId,
        String label,
        String transport,
        String host,
        int port,
        String path,
        Object channelId,
        boolean authRequired,
        String visibility,
        String commandScope,
        boolean primary) throws Exception {
        JSONObject endpoint = new JSONObject();
        endpoint.put("transport", transport);
        endpoint.put("host", host);
        endpoint.put("port", port);
        endpoint.put("path", path);
        endpoint.put("channel_id", channelId);
        endpoint.put("max_datagram_bytes", JSONObject.NULL);
        endpoint.put("auth_required", authRequired);

        JSONObject descriptor = new JSONObject();
        descriptor.put("endpoint_id", endpointId);
        descriptor.put("label", label);
        descriptor.put("endpoint", endpoint);
        descriptor.put("visibility", visibility);
        descriptor.put("command_scope", commandScope);
        descriptor.put("primary", primary);
        return descriptor;
    }

    private static boolean isLoopbackHost(String host) {
        if (host == null) {
            return false;
        }
        String normalized = host.trim().toLowerCase(Locale.US);
        return "127.0.0.1".equals(normalized)
            || "localhost".equals(normalized)
            || "::1".equals(normalized);
    }

    private static JSONObject streamJson(String id, String kind, String description, boolean active) throws Exception {
        JSONObject stream = new JSONObject();
        stream.put("id", id);
        stream.put("kind", kind);
        stream.put("description", description);
        stream.put("active", active);
        return stream;
    }

    private JSONArray modulesJson(JSONArray streams, OscIngressServer oscIngressServer, long revision) throws Exception {
        boolean oscRunning = oscIngressServer != null && oscIngressServer.isRunning();
        boolean shellHelperActive = shellHelper.isConnected();
        boolean breathActive = breathAssessment.hasAssessments();
        boolean videoLabActive = videoLabMetricSamples.get() > 0
            || videoLabEncodedStreamManifests.get() > 0
            || videoLabEncodedSampleMetadata.get() > 0;
        boolean deviceWatchdogRunning = deviceWatchdogStatus.optBoolean("running", false);
        boolean deviceWatchdogObserved = deviceWatchdogRunning
            || deviceWatchdogStatus.optLong("sample_count", 0L) > 0L
            || deviceWatchdogStatus.optString("stop_reason", "").length() > 0;
        boolean transportActive = transportSessions.createdCount() > 0
            || transportSessions.closedCount() > 0
            || transportSessions.failedCount() > 0;

        JSONArray modules = new JSONArray();
        modules.put(moduleRuntimeState(
            "diagnostics.broker",
            "diagnostic",
            "active",
            revision,
            providedStreamIdsForModule(streams, "diagnostics.broker"),
            new JSONArray(),
            new JSONArray()
                .put(healthMetricJson("accepted_latency_samples", "Accepted latency samples", null, 0.0, null, acceptedLatencySamples.get(), "healthy"))
                .put(healthMetricJson("published_stream_events", "Published stream events", null, 0.0, null, publishedStreamEvents.get(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "termux.linux_sidecar",
            "processor",
            publishedStreamEvents.get() > 0 ? "active" : "idle",
            revision,
            providedStreamIdsForModule(streams, "termux.linux_sidecar"),
            jsonArrayOf("bio:polar_acc"),
            new JSONArray().put(healthMetricJson("feedback_samples", "Published feedback samples", null, 0.0, null, publishedStreamEvents.get(), publishedStreamEvents.get() > 0 ? "healthy" : "unknown")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "diagnostics.device_watchdog",
            "diagnostic",
            deviceWatchdogRunning ? "active" : "idle",
            revision,
            providedStreamIdsForModule(streams, "diagnostics.device_watchdog"),
            new JSONArray(),
            new JSONArray()
                .put(healthMetricJson("sample_count", "Watchdog samples", null, 0.0, null, deviceWatchdogStatus.optLong("sample_count", 0L), deviceWatchdogObserved ? "healthy" : "unknown"))
                .put(healthMetricJson("wake_lock_held", "Wake lock held", null, 0.0, 1.0, deviceWatchdogStatus.optBoolean("wake_lock_held", false) ? 1.0 : 0.0, "unknown")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "diagnostics.clock",
            "diagnostic",
            "active",
            revision,
            providedStreamIdsForModule(streams, "diagnostics.clock"),
            new JSONArray(),
            new JSONArray().put(healthMetricJson("clock_streams", "Clock streams", null, 1.0, null, providedStreamIdsForModule(streams, "diagnostics.clock").length(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "control.kiosk",
            "control_adapter",
            "active",
            revision,
            providedStreamIdsForModule(streams, "control.kiosk"),
            new JSONArray(),
            new JSONArray().put(healthMetricJson("console_open_requests", "Console open requests", null, 0.0, null, brokerConsoleOpenRequests.get(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "polar.communication",
            "provider",
            "active",
            revision,
            providedStreamIdsForModule(streams, "polar.communication"),
            new JSONArray(),
            new JSONArray()
                .put(healthMetricJson("polar_streams", "Polar communication streams", null, 1.0, null, providedStreamIdsForModule(streams, "polar.communication").length(), "healthy"))
                .put(healthMetricJson("published_stream_events", "Published stream events", null, 0.0, null, publishedStreamEvents.get(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "bio.telemetry",
            "provider",
            "active",
            revision,
            providedStreamIdsForModule(streams, "bio.telemetry"),
            new JSONArray(),
            new JSONArray().put(healthMetricJson("bio_streams", "Non-Polar bio input streams", null, 0.0, null, providedStreamIdsForModule(streams, "bio.telemetry").length(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "motion.telemetry",
            "provider",
            "active",
            revision,
            providedStreamIdsForModule(streams, "motion.telemetry"),
            new JSONArray(),
            new JSONArray().put(healthMetricJson("motion_streams", "Object motion input streams", null, 0.0, null, providedStreamIdsForModule(streams, "motion.telemetry").length(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "bio.breath_assessment",
            "processor",
            breathActive ? "active" : "idle",
            revision,
            providedStreamIdsForModule(streams, "bio.breath_assessment"),
            jsonArrayOf("bio:polar_acc", "stream.motion.object_pose", "xr:controller_pose"),
            new JSONArray().put(healthMetricJson("assessment_available", "Assessment available", null, 0.0, 1.0, breathActive ? 1.0 : 0.0, breathActive ? "healthy" : "unknown")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "camera.projection",
            "provider",
            "active",
            revision,
            providedStreamIdsForModule(streams, "camera.projection"),
            new JSONArray(),
            new JSONArray().put(healthMetricJson("projection_streams", "Projection streams", null, 1.0, null, providedStreamIdsForModule(streams, "camera.projection").length(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "shell.helper",
            "control_adapter",
            shellHelperActive ? "active" : "idle",
            revision,
            providedStreamIdsForModule(streams, "shell.helper"),
            new JSONArray(),
            new JSONArray().put(healthMetricJson("helper_connected", "Helper connected", null, 0.0, 1.0, shellHelperActive ? 1.0 : 0.0, shellHelperActive ? "healthy" : "unknown")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "transport.sessions",
            "bridge",
            transportActive ? "active" : "idle",
            revision,
            providedStreamIdsForModule(streams, "transport.sessions"),
            new JSONArray(),
            new JSONArray()
                .put(healthMetricJson("sessions_created", "Sessions created", null, 0.0, null, transportSessions.createdCount(), "healthy"))
                .put(healthMetricJson("sessions_failed", "Sessions failed", null, 0.0, null, transportSessions.failedCount(), transportSessions.failedCount() > 0 ? "warning" : "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "video.lab",
            "diagnostic",
            videoLabActive ? "active" : "idle",
            revision,
            providedStreamIdsForModule(streams, "video.lab"),
            jsonArrayOf("video_lab.encoded_sample_metadata"),
            new JSONArray()
                .put(healthMetricJson("metric_samples", "Metric samples", null, 0.0, null, videoLabMetricSamples.get(), videoLabActive ? "healthy" : "unknown"))
                .put(healthMetricJson("encoded_manifests", "Encoded manifests", null, 0.0, null, videoLabEncodedStreamManifests.get(), "healthy")),
            new JSONArray()));
        modules.put(moduleRuntimeState(
            "osc.ingress",
            "bridge",
            oscRunning ? "active" : "idle",
            revision,
            providedStreamIdsForModule(streams, "osc.ingress"),
            new JSONArray(),
            new JSONArray()
                .put(healthMetricJson("packets_received", "Packets received", null, 0.0, null, oscIngressPackets.get(), oscRunning ? "healthy" : "unknown"))
                .put(healthMetricJson("packets_rejected", "Packets rejected", null, 0.0, null, oscIngressRejectedPackets.get(), oscIngressRejectedPackets.get() > 0 ? "warning" : "healthy")),
            new JSONArray()));
        return modules;
    }

    private JSONObject moduleRuntimeState(
        String moduleId,
        String moduleKind,
        String lifecycleState,
        long revision,
        JSONArray providedStreamIds,
        JSONArray consumedStreamIds,
        JSONArray healthMetrics,
        JSONArray issueCodes) throws Exception {
        JSONObject module = new JSONObject();
        module.put("schema", "rusty.xr.broker.module_runtime_state.v1");
        module.put("module_id", moduleId);
        module.put("module_kind", moduleKind);
        module.put("lifecycle_state", lifecycleState);
        module.put("revision", revision);
        module.put("last_transition_elapsed_ns", startedElapsedNanos);
        module.put("provided_stream_ids", providedStreamIds);
        module.put("consumed_stream_ids", consumedStreamIds);
        module.put("active_resource_locks", new JSONArray());
        module.put("health_metrics", healthMetrics);
        module.put("issue_codes", issueCodes);
        return module;
    }

    private static JSONArray providedStreamIdsForModule(JSONArray streams, String moduleId) throws Exception {
        JSONArray streamIds = new JSONArray();
        for (int index = 0; index < streams.length(); index++) {
            JSONObject stream = streams.optJSONObject(index);
            if (stream == null) {
                continue;
            }
            String streamId = stream.optString("id", "");
            String kind = stream.optString("kind", "custom");
            if (moduleId.equals(moduleIdForStream(streamId, kind))) {
                streamIds.put(streamId);
            }
        }
        return streamIds;
    }

    private static JSONObject healthMetricJson(
        String metric,
        String label,
        Object unit,
        Object healthyMin,
        Object healthyMax,
        Object observedValue,
        String state) throws Exception {
        JSONObject value = new JSONObject();
        value.put("metric", metric);
        value.put("label", label);
        value.put("unit", unit != null ? unit : JSONObject.NULL);
        value.put("healthy_min", healthyMin != null ? healthyMin : JSONObject.NULL);
        value.put("healthy_max", healthyMax != null ? healthyMax : JSONObject.NULL);
        value.put("observed_value", observedValue != null ? observedValue : JSONObject.NULL);
        value.put("state", state != null && state.length() > 0 ? state : "unknown");
        return value;
    }

    private long streamRegistrySemanticRevision() {
        return 1L
            + publishedStreamEvents.get()
            + videoLabMetricSamples.get()
            + videoLabEncodedStreamManifests.get()
            + videoLabEncodedSampleMetadata.get()
            + deviceWatchdogStatus.optLong("sample_count", 0L)
            + (deviceWatchdogStatus.optBoolean("running", false) ? 1L : 0L)
            + transportSessions.createdCount()
            + transportSessions.closedCount()
            + transportSessions.failedCount()
            + controlLeaseRevision.get();
    }

    JSONObject requestControlLease(JSONObject params, String fallbackHolderClientId) throws Exception {
        if (params == null) {
            throw new CommandRejection("missing_params", "Command requires control lease request params.", false);
        }

        String holderClientId = params.optString(
            "holder_client_id",
            fallbackHolderClientId != null ? fallbackHolderClientId : "").trim();
        if (holderClientId.length() == 0) {
            throw new CommandRejection(
                "missing_holder_client_id",
                "Command requires params.holder_client_id or a command client_id.",
                false);
        }

        JSONObject requestedScope = params.optJSONObject("scope");
        if (requestedScope == null) {
            throw new CommandRejection("missing_scope", "Command requires params.scope.", false);
        }

        JSONObject scope = normalizedControlScope(requestedScope);
        Long expectedRevision = optionalLong(params, "expected_revision");
        if (expectedRevision != null && expectedRevision.longValue() < 0L) {
            throw new CommandRejection(
                "invalid_expected_revision",
                "params.expected_revision must be zero or greater.",
                false);
        }

        Long requestedDurationNs = optionalLong(params, "requested_duration_elapsed_ns");
        if (requestedDurationNs != null && requestedDurationNs.longValue() <= 0L) {
            throw new CommandRejection(
                "invalid_duration",
                "params.requested_duration_elapsed_ns must be positive when present.",
                false);
        }

        synchronized (controlLeaseLock) {
            long now = SystemClock.elapsedRealtimeNanos();
            pruneExpiredControlLeasesLocked(now);
            long currentRevision = streamRegistrySemanticRevision();
            if (expectedRevision != null && expectedRevision.longValue() != currentRevision) {
                throw new CommandRejection(
                    "stale_revision",
                    "Control lease request expected registry revision " + expectedRevision +
                        " but broker is at revision " + currentRevision + ".",
                    true)
                    .withCurrentRevision(currentRevision)
                    .withRequiredLeaseScope(scope);
            }

            for (JSONObject activeLease : activeControlLeases.values()) {
                JSONObject activeScope = activeLease.optJSONObject("scope");
                if (!sameControlScope(activeScope, scope)) {
                    continue;
                }

                String activeHolder = activeLease.optString("holder_client_id", "");
                if (holderClientId.equals(activeHolder)) {
                    return controlLeaseResult("control_lease_already_active", activeLease);
                }

                throw new CommandRejection(
                    "lease_conflict",
                    "Control scope is already leased by another holder.",
                    true)
                    .withLeaseId(activeLease.optString("lease_id", ""))
                    .withCurrentRevision(currentRevision)
                    .withRequiredLeaseScope(scope);
            }

            String leaseId = "control-lease-" + controlLeaseSequence.incrementAndGet();
            long durationNs = requestedDurationNs != null
                ? requestedDurationNs.longValue()
                : DEFAULT_CONTROL_LEASE_DURATION_ELAPSED_NS;
            long expiresElapsedNs = now > Long.MAX_VALUE - durationNs ? Long.MAX_VALUE : now + durationNs;
            controlLeaseRevision.incrementAndGet();
            long grantedRevision = streamRegistrySemanticRevision();

            JSONObject lease = new JSONObject();
            lease.put("schema", CONTROL_LEASE_SCHEMA);
            lease.put("lease_id", leaseId);
            lease.put("holder_client_id", holderClientId);
            lease.put("scope", scope);
            lease.put("granted_revision", grantedRevision);
            lease.put("expires_elapsed_ns", expiresElapsedNs);
            lease.put("state", "active");
            activeControlLeases.put(leaseId, lease);
            return controlLeaseResult("control_lease_granted", lease);
        }
    }

    JSONObject releaseControlLease(JSONObject params, String fallbackHolderClientId) throws Exception {
        if (params == null) {
            throw new CommandRejection("missing_params", "Command requires control lease release params.", false);
        }

        String leaseId = params.optString("lease_id", "").trim();
        if (leaseId.length() == 0) {
            throw new CommandRejection("missing_lease_id", "Command requires params.lease_id.", false);
        }

        String holderClientId = params.optString(
            "holder_client_id",
            fallbackHolderClientId != null ? fallbackHolderClientId : "").trim();
        if (holderClientId.length() == 0) {
            throw new CommandRejection(
                "missing_holder_client_id",
                "Command requires params.holder_client_id or a command client_id.",
                false);
        }

        Long expectedRevision = optionalLong(params, "expected_revision");
        if (expectedRevision != null && expectedRevision.longValue() < 0L) {
            throw new CommandRejection(
                "invalid_expected_revision",
                "params.expected_revision must be zero or greater.",
                false);
        }

        JSONObject requestedScope = params.optJSONObject("scope");
        JSONObject normalizedScope = requestedScope != null ? normalizedControlScope(requestedScope) : null;

        synchronized (controlLeaseLock) {
            long now = SystemClock.elapsedRealtimeNanos();
            pruneExpiredControlLeasesLocked(now);
            long currentRevision = streamRegistrySemanticRevision();
            if (expectedRevision != null && expectedRevision.longValue() != currentRevision) {
                throw new CommandRejection(
                    "stale_revision",
                    "Control lease release expected registry revision " + expectedRevision +
                        " but broker is at revision " + currentRevision + ".",
                    true)
                    .withCurrentRevision(currentRevision);
            }

            JSONObject lease = activeControlLeases.get(leaseId);
            if (lease == null) {
                throw new CommandRejection("lease_not_found", "Control lease was not found.", false)
                    .withLeaseId(leaseId)
                    .withCurrentRevision(currentRevision);
            }

            String activeHolder = lease.optString("holder_client_id", "");
            if (!holderClientId.equals(activeHolder)) {
                throw new CommandRejection(
                    "lease_holder_mismatch",
                    "Control lease is held by a different client.",
                    false)
                    .withLeaseId(leaseId)
                    .withCurrentRevision(currentRevision);
            }

            if (normalizedScope != null && !sameControlScope(lease.optJSONObject("scope"), normalizedScope)) {
                throw new CommandRejection(
                    "lease_scope_mismatch",
                    "Control lease does not match the requested release scope.",
                    false)
                    .withLeaseId(leaseId)
                    .withCurrentRevision(currentRevision)
                    .withRequiredLeaseScope(lease.optJSONObject("scope"));
            }

            JSONObject releasedLease = copyObject(lease);
            releasedLease.put("state", "released");
            activeControlLeases.remove(leaseId);
            controlLeaseRevision.incrementAndGet();
            return controlLeaseResult("control_lease_released", releasedLease);
        }
    }

    private JSONArray activeControlLeasesJson() throws Exception {
        synchronized (controlLeaseLock) {
            pruneExpiredControlLeasesLocked(SystemClock.elapsedRealtimeNanos());
            JSONArray leases = new JSONArray();
            for (JSONObject lease : activeControlLeases.values()) {
                leases.put(copyObject(lease));
            }
            return leases;
        }
    }

    private JSONArray activeControlLeaseIds() throws Exception {
        synchronized (controlLeaseLock) {
            pruneExpiredControlLeasesLocked(SystemClock.elapsedRealtimeNanos());
            JSONArray leaseIds = new JSONArray();
            for (String leaseId : activeControlLeases.keySet()) {
                leaseIds.put(leaseId);
            }
            return leaseIds;
        }
    }

    private void pruneExpiredControlLeasesLocked(long nowElapsedNs) {
        List<String> expiredLeaseIds = new ArrayList<>();
        for (Map.Entry<String, JSONObject> entry : activeControlLeases.entrySet()) {
            JSONObject lease = entry.getValue();
            if (!lease.isNull("expires_elapsed_ns")
                && lease.optLong("expires_elapsed_ns", Long.MAX_VALUE) <= nowElapsedNs) {
                expiredLeaseIds.add(entry.getKey());
            }
        }

        for (String leaseId : expiredLeaseIds) {
            activeControlLeases.remove(leaseId);
            controlLeaseRevision.incrementAndGet();
        }
    }

    private JSONObject controlLeaseResult(String outcome, JSONObject lease) throws Exception {
        JSONObject result = new JSONObject();
        result.put("outcome", outcome);
        result.put("lease", copyObject(lease));
        result.put("revision", streamRegistrySemanticRevision());
        return result;
    }

    private static JSONObject normalizedControlScope(JSONObject scope) throws Exception {
        String scopeId = scope.optString("scope_id", "").trim();
        String commandScope = scope.optString("command_scope", "").trim();
        if (scopeId.length() == 0) {
            throw new CommandRejection("invalid_scope", "params.scope.scope_id is required.", false);
        }
        if (commandScope.length() == 0) {
            throw new CommandRejection("invalid_scope", "params.scope.command_scope is required.", false);
        }

        JSONObject normalized = new JSONObject();
        normalized.put("schema", CONTROL_SCOPE_SCHEMA);
        normalized.put("scope_id", scopeId);
        normalized.put("command_scope", commandScope);
        if (scope.has("resource_id") && !scope.isNull("resource_id")) {
            String resourceId = scope.optString("resource_id", "").trim();
            normalized.put("resource_id", resourceId.length() > 0 ? resourceId : JSONObject.NULL);
        } else {
            normalized.put("resource_id", JSONObject.NULL);
        }
        return normalized;
    }

    private static boolean sameControlScope(JSONObject left, JSONObject right) {
        if (left == null || right == null) {
            return false;
        }

        return left.optString("scope_id", "").equals(right.optString("scope_id", ""))
            && left.optString("command_scope", "").equals(right.optString("command_scope", ""))
            && left.optString("resource_id", "").equals(right.optString("resource_id", ""));
    }

    private static Long optionalLong(JSONObject object, String key) {
        if (object == null || !object.has(key) || object.isNull(key)) {
            return null;
        }
        return Long.valueOf(object.optLong(key));
    }

    private static JSONObject copyObject(JSONObject object) throws Exception {
        return object != null ? new JSONObject(object.toString()) : new JSONObject();
    }

    private static JSONArray activeStreamIds(JSONArray streams) throws Exception {
        JSONArray active = new JSONArray();
        for (int index = 0; index < streams.length(); index++) {
            JSONObject stream = streams.optJSONObject(index);
            if (stream != null && stream.optBoolean("active", false)) {
                active.put(stream.optString("id", ""));
            }
        }
        return active;
    }

    static final class CommandRejection extends Exception {
        private final String code;
        private final boolean retryable;
        private final LinkedHashMap<String, Object> hints = new LinkedHashMap<>();

        CommandRejection(String code, String message, boolean retryable) {
            super(message);
            this.code = code != null ? code : "";
            this.retryable = retryable;
        }

        CommandRejection withCurrentRevision(long currentRevision) {
            hints.put("current_revision", Long.valueOf(currentRevision));
            return this;
        }

        CommandRejection withLeaseId(String leaseId) {
            if (leaseId != null && leaseId.length() > 0) {
                hints.put("lease_id", leaseId);
            }
            return this;
        }

        CommandRejection withRequiredLeaseScope(JSONObject scope) throws Exception {
            if (scope != null) {
                hints.put("required_lease_scope", copyObject(scope));
            }
            return this;
        }

        JSONObject toErrorJson() throws Exception {
            JSONObject error = new JSONObject();
            error.put("schema", COMMAND_REJECTION_SCHEMA);
            error.put("code", code);
            error.put("message", getMessage() != null ? getMessage() : "");
            error.put("retryable", retryable);
            for (Map.Entry<String, Object> hint : hints.entrySet()) {
                error.put(hint.getKey(), hint.getValue());
            }
            return error;
        }
    }

    private static String providerIdForKind(String kind) {
        String clean = kind == null || kind.length() == 0 ? "custom" : kind;
        return clean + "-provider";
    }

    private static String providerIdForStream(String streamId, String kind) {
        String moduleId = moduleIdForStream(streamId, kind);
        if ("diagnostics.broker".equals(moduleId)) {
            return "diagnostics-provider";
        }
        if ("diagnostics.clock".equals(moduleId)) {
            return "clock-provider";
        }
        if ("diagnostics.device_watchdog".equals(moduleId)) {
            return "device-watchdog-provider";
        }
        if ("control.kiosk".equals(moduleId)) {
            return "control-provider";
        }
        if ("termux.linux_sidecar".equals(moduleId)) {
            return "termux-provider";
        }
        if ("polar.communication".equals(moduleId)) {
            return "polar-provider";
        }
        if ("bio.telemetry".equals(moduleId)) {
            return "bio-provider";
        }
        if ("motion.telemetry".equals(moduleId)) {
            return "motion-provider";
        }
        if ("bio.breath_assessment".equals(moduleId)) {
            return "breath-assessment-provider";
        }
        if ("camera.projection".equals(moduleId)) {
            return "camera-provider";
        }
        if ("shell.helper".equals(moduleId)) {
            return "shell-helper-provider";
        }
        if ("transport.sessions".equals(moduleId)) {
            return "transport-provider";
        }
        if ("video.lab".equals(moduleId)) {
            return "video-provider";
        }
        if ("osc.ingress".equals(moduleId)) {
            return "osc-provider";
        }
        return providerIdForKind(kind);
    }

    private static String moduleIdForStream(String streamId, String kind) {
        if (streamId == null) {
            return "diagnostics.broker";
        }
        if (streamId.startsWith("clock:")) {
            return "diagnostics.clock";
        }
        if (streamId.startsWith("broker:") || streamId.startsWith("latency:")) {
            return "diagnostics.broker";
        }
        if ("diagnostics:termux_python".equals(streamId)) {
            return "termux.linux_sidecar";
        }
        if (streamId.startsWith("device_watchdog.")) {
            return "diagnostics.device_watchdog";
        }
        if (streamId.startsWith("kiosk:") || "experiment.control".equals(streamId) || "control".equals(kind)) {
            return "control.kiosk";
        }
        if ("bio:breath".equals(streamId)) {
            return "bio.breath_assessment";
        }
        if ("bio:polar_hr_rr".equals(streamId)
            || "bio:polar_acc".equals(streamId)
            || "bio:polar_ecg".equals(streamId)) {
            return "polar.communication";
        }
        if (streamId.startsWith("stream.motion.") || streamId.startsWith("motion:") || "motion".equals(kind)) {
            return "motion.telemetry";
        }
        if (streamId.startsWith("bio:") || streamId.startsWith("xr:")) {
            return "bio.telemetry";
        }
        if (streamId.startsWith("camera_provider.")) {
            return "camera.projection";
        }
        if (streamId.startsWith("shell_helper.")) {
            return "shell.helper";
        }
        if (streamId.startsWith("transport.") || streamId.startsWith("q2q_relay.")) {
            return "transport.sessions";
        }
        if (streamId.startsWith("video_lab.")) {
            return "video.lab";
        }
        if (streamId.startsWith("osc:") || "osc".equals(kind)) {
            return "osc.ingress";
        }
        return "diagnostics.broker";
    }

    private static String moduleKindForStream(String streamId, String kind) {
        String moduleId = moduleIdForStream(streamId, kind);
        if ("bio.breath_assessment".equals(moduleId)) {
            return "processor";
        }
        if ("termux.linux_sidecar".equals(moduleId)) {
            return "processor";
        }
        if ("control.kiosk".equals(moduleId) || "shell.helper".equals(moduleId)) {
            return "control_adapter";
        }
        if ("transport.sessions".equals(moduleId) || "osc.ingress".equals(moduleId)) {
            return "bridge";
        }
        if ("diagnostics.broker".equals(moduleId)
            || "diagnostics.clock".equals(moduleId)
            || "diagnostics.device_watchdog".equals(moduleId)
            || "video.lab".equals(moduleId)) {
            return "diagnostic";
        }
        return "provider";
    }

    private static String providerLabel(String providerId) {
        if ("diagnostics-provider".equals(providerId)) {
            return "Broker diagnostics";
        }
        if ("bio-provider".equals(providerId)) {
            return "Bio provider";
        }
        if ("motion-provider".equals(providerId)) {
            return "Motion provider";
        }
        if ("polar-provider".equals(providerId)) {
            return "Polar communication";
        }
        if ("breath-assessment-provider".equals(providerId)) {
            return "Breath assessment";
        }
        if ("video-provider".equals(providerId)) {
            return "Video provider";
        }
        if ("camera-provider".equals(providerId)) {
            return "Camera provider";
        }
        if ("clock-provider".equals(providerId)) {
            return "Clock provider";
        }
        if ("device-watchdog-provider".equals(providerId)) {
            return "Device watchdog";
        }
        if ("transport-provider".equals(providerId)) {
            return "Transport provider";
        }
        if ("control-provider".equals(providerId)) {
            return "Control provider";
        }
        if ("termux-provider".equals(providerId)) {
            return "Termux Linux sidecar";
        }
        if ("shell-helper-provider".equals(providerId)) {
            return "Shell helper";
        }
        if ("osc-provider".equals(providerId)) {
            return "OSC ingress";
        }
        return providerId;
    }

    private static String providerSensitivity(String providerId) {
        if ("bio-provider".equals(providerId)) {
            return "physiology";
        }
        if ("motion-provider".equals(providerId)) {
            return "body_motion";
        }
        if ("polar-provider".equals(providerId)) {
            return "physiology";
        }
        if ("termux-provider".equals(providerId)) {
            return "diagnostic";
        }
        if ("breath-assessment-provider".equals(providerId)) {
            return "derived_physiology";
        }
        if ("control-provider".equals(providerId)) {
            return "restricted";
        }
        return "diagnostic";
    }

    private static String streamKindForStream(String streamId, String kind) {
        if ("bio".equals(kind)) {
            return "Bio";
        }
        if ("motion".equals(kind)) {
            return "Motion";
        }
        if ("video".equals(kind) || streamId.contains("h264")) {
            return "Media";
        }
        if ("control".equals(kind)) {
            return "Control";
        }
        if ("xr".equals(kind)) {
            return "XrInput";
        }
        if ("clock".equals(kind) || "latency".equals(kind) || "status".equals(kind) || "diagnostic".equals(kind)) {
            return "Telemetry";
        }
        return "Custom";
    }

    private static String payloadKindForStream(String streamId) {
        return streamId.contains("h264") ? "H264" : "Json";
    }

    private static String payloadSchemaForStream(String streamId) {
        if ("latency:sample".equals(streamId)) {
            return "rusty.xr.broker.latency_sample.v1";
        }
        if ("bio:breath".equals(streamId)) {
            return "rusty.xr.bio.breath.v1";
        }
        if ("bio:polar_hr_rr".equals(streamId)) {
            return "rusty.xr.bio.polar_hr_rr.v1";
        }
        if ("bio:polar_acc".equals(streamId)) {
            return "rusty.xr.bio.polar_acc.v1";
        }
        if ("stream.motion.object_pose".equals(streamId) || "motion:object_pose".equals(streamId)) {
            return "rusty.manifold.motion.object_pose.sample.v1";
        }
        if ("video_lab.metric_sample".equals(streamId)) {
            return "rusty.xr.video_lab.metric_sample.v1";
        }
        if ("device_watchdog.status".equals(streamId)) {
            return "rusty.xr.device_watchdog.status.v1";
        }
        if (streamId.contains("h264")) {
            return "rusty.manifold.video.binary_stream.v1";
        }
        return "rusty.manifold.stream_payload.v1";
    }

    private static Object recommendedRateForStream(String streamId, String kind) {
        if ("clock:openxr_frame".equals(streamId) || "xr".equals(kind)) {
            return 72.0;
        }
        if ("stream.motion.object_pose".equals(streamId) || "motion:object_pose".equals(streamId) || "motion".equals(kind)) {
            return 20.0;
        }
        if ("video_lab.metric_sample".equals(streamId)) {
            return 30.0;
        }
        if ("device_watchdog.status".equals(streamId)) {
            return 0.033;
        }
        if ("bio:polar_acc".equals(streamId)) {
            return 52.0;
        }
        if ("bio".equals(kind) || "latency".equals(kind) || "clock".equals(kind)) {
            return 15.0;
        }
        return JSONObject.NULL;
    }

    private static String rateClassForStream(String streamId, String kind) {
        if (streamId.contains("h264")) {
            return "media";
        }
        if ("clock:openxr_frame".equals(streamId)
            || "bio:polar_acc".equals(streamId)
            || "stream.motion.object_pose".equals(streamId)
            || "motion:object_pose".equals(streamId)
            || "motion".equals(kind)
            || "xr".equals(kind)) {
            return "frame_rate_telemetry";
        }
        if (streamId.contains("encoded_stream_manifest") || streamId.contains("encoded_sample_metadata")) {
            return "metadata_only";
        }
        if ("device_watchdog.status".equals(streamId)
            || "bio".equals(kind)
            || "clock".equals(kind)
            || "latency".equals(kind)
            || "video_lab.metric_sample".equals(streamId)) {
            return "low_rate_telemetry";
        }
        return "unknown";
    }

    private static String dataSensitivityForStream(String streamId, String kind) {
        if ("bio:breath".equals(streamId)) {
            return "derived_physiology";
        }
        if ("bio".equals(kind)) {
            return "physiology";
        }
        if ("motion".equals(kind)) {
            return "body_motion";
        }
        if ("control".equals(kind)) {
            return "restricted";
        }
        return "diagnostic";
    }

    private static String retentionPolicyForStream(String streamId, String kind) {
        if (streamId.contains("h264")) {
            return "downstream_owned";
        }
        if ("unknown".equals(rateClassForStream(streamId, kind))) {
            return "none";
        }
        if ("device_watchdog.status".equals(streamId)) {
            return "rolling_file";
        }
        return "rolling_window";
    }

    private static String uiSubscriptionPolicyForStream(String streamId, String kind) {
        String rateClass = rateClassForStream(streamId, kind);
        String retentionPolicy = retentionPolicyForStream(streamId, kind);
        if ("downstream_owned".equals(retentionPolicy)
            || "media".equals(rateClass)
            || "burst".equals(rateClass)
            || streamId.contains("h264")) {
            return "never_subscribe_from_ui";
        }
        if ("low_rate_telemetry".equals(rateClass)) {
            return "auto_subscribe_low_rate";
        }
        if ("frame_rate_telemetry".equals(rateClass)) {
            return "auto_subscribe_when_selected";
        }
        return "manual_only";
    }

    private static String chartPolicyForStream(String streamId, String kind) {
        String rateClass = rateClassForStream(streamId, kind);
        String retentionPolicy = retentionPolicyForStream(streamId, kind);
        if ("downstream_owned".equals(retentionPolicy)
            || "media".equals(rateClass)
            || "burst".equals(rateClass)
            || streamId.contains("h264")) {
            return "dedicated_view_required";
        }
        if ("low_rate_telemetry".equals(rateClass)) {
            return "low_rate_direct";
        }
        if ("frame_rate_telemetry".equals(rateClass)) {
            return "downsample_required";
        }
        return "not_chartable";
    }

    private static JSONArray metricsForStream(String streamId) throws Exception {
        JSONArray metrics = new JSONArray();
        if ("latency:sample".equals(streamId)) {
            metrics.put(metricJson("latency_ms", "Latency", "ms", JSONObject.NULL, JSONObject.NULL));
        } else if ("bio:polar_hr_rr".equals(streamId)) {
            metrics.put(metricJson("heart_rate_bpm", "Heart rate", "bpm", JSONObject.NULL, JSONObject.NULL));
            metrics.put(metricJson("mean_rr_ms", "Mean RR", "ms", JSONObject.NULL, JSONObject.NULL));
        } else if ("bio:polar_acc".equals(streamId)) {
            metrics.put(metricJson("acc_magnitude_g", "Acceleration magnitude", "g", JSONObject.NULL, JSONObject.NULL));
        } else if ("stream.motion.object_pose".equals(streamId) || "motion:object_pose".equals(streamId)) {
            metrics.put(metricJson("position_m", "Position", "m", JSONObject.NULL, JSONObject.NULL));
            metrics.put(metricJson("quality01", "Quality", JSONObject.NULL, 0.0, 1.0));
        } else if ("bio:breath".equals(streamId)) {
            metrics.put(metricJson("volume01", "Volume", JSONObject.NULL, 0.0, 1.0));
            metrics.put(metricJson("quality01", "Quality", JSONObject.NULL, 0.0, 1.0));
        } else if ("video_lab.metric_sample".equals(streamId)) {
            metrics.put(metricJson("frame_age_ms", "Frame age", "ms", JSONObject.NULL, JSONObject.NULL));
            metrics.put(metricJson("latency_ms", "Latency", "ms", JSONObject.NULL, JSONObject.NULL));
        } else if ("device_watchdog.status".equals(streamId)) {
            metrics.put(metricJson("sample_count", "Samples", JSONObject.NULL, 0.0, JSONObject.NULL));
            metrics.put(metricJson("uptime_ms", "Uptime", "ms", 0.0, JSONObject.NULL));
        }
        return metrics;
    }

    private static JSONObject metricJson(
        String metric,
        String label,
        Object unit,
        Object minValue,
        Object maxValue) throws Exception {
        JSONObject value = new JSONObject();
        value.put("metric", metric);
        value.put("label", label);
        value.put("unit", unit);
        value.put("min_value", minValue);
        value.put("max_value", maxValue);
        return value;
    }

    private static JSONArray jsonArrayOf(String... values) {
        JSONArray array = new JSONArray();
        for (String value : values) {
            array.put(value);
        }
        return array;
    }

    private static Object nullableJsonString(String value) {
        return value != null && value.length() > 0 ? value : JSONObject.NULL;
    }

    private static boolean metaShellPackage(String packageName) {
        if (packageName == null) {
            return false;
        }
        String normalized = packageName.toLowerCase(Locale.ROOT);
        return normalized.startsWith("com.oculus.")
            || normalized.startsWith("com.meta.")
            || normalized.contains("horizon");
    }

    private static String activeBrokerPanelId() {
        String page = MainActivity.activePageName();
        if (page == null || page.length() == 0) {
            return "broker";
        }
        return "broker." + page.trim().toLowerCase(Locale.ROOT).replace(' ', '_');
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

    JSONObject rustyKioskStatusJson() throws Exception {
        JSONObject control = experimentControl.toStatusJson();
        JSONObject helperStatus = control.optJSONObject("helper_status");
        JSONObject clockStatus = clock.statusJson();

        boolean helperConnected = shellHelper.isConnected();
        boolean consoleVisible = MainActivity.isConsoleVisible();
        String desiredFocus = control.optString("desired_focus", "broker");
        String targetPackage = control.optString("target_package", "");
        String targetActivity = control.optString("target_activity", "");
        String foregroundPackage = helperStatus != null ? helperStatus.optString("foreground_package", "") : "";
        String foregroundActivity = helperStatus != null ? helperStatus.optString("foreground_activity", "") : "";
        if (foregroundPackage.length() == 0 && consoleVisible) {
            foregroundPackage = "com.example.rustyxr.broker";
            foregroundActivity = "com.example.rustyxr.broker.MainActivity";
        }

        String surfaceIntent = "RustyKioskDefault";
        boolean metaMenuActive = metaShellPackage(foregroundPackage);
        if (metaMenuActive) {
            surfaceIntent = "MetaPanelUnexpected";
        } else if (targetPackage.length() > 0 && targetPackage.equals(foregroundPackage)) {
            surfaceIntent = "RustyXrTarget";
        } else if ("target".equals(desiredFocus) && foregroundPackage.length() == 0) {
            surfaceIntent = "UnknownSurface";
        }

        String mode = control.optString("mode", "off");
        boolean focusGuardianActive = helperConnected &&
            control.optBoolean("enabled", false) &&
            !"off".equals(mode);
        boolean proximityWatchdogActive = helperStatus != null &&
            (helperStatus.optBoolean("proximity_watchdog_active", false) ||
                helperStatus.optBoolean("proximity_control_enabled", false));

        JSONObject latestCommand = new JSONObject();
        latestCommand.put("schema", KIOSK_COMMAND_EVIDENCE_SCHEMA);
        latestCommand.put("command_goal", "surface.current");
        latestCommand.put("provider", "Broker");
        latestCommand.put("preferred_command", "GET /status");
        latestCommand.put("fallback_command", "adb shell dumpsys window");
        latestCommand.put("foreground_before", JSONObject.NULL);
        latestCommand.put("foreground_after", foregroundPackage.length() > 0
            ? foregroundPackage + (foregroundActivity.length() > 0 ? "/" + foregroundActivity : "")
            : JSONObject.NULL);
        latestCommand.put("clock_epoch_id", nullableJsonString(clockStatus.optString("clock_epoch_id", "")));
        latestCommand.put("notes", new JSONArray());

        JSONArray limitations = new JSONArray();
        limitations.put("normal_android_panel_not_app_owned_immersive_home");
        limitations.put("no_preemptive_home_menu_intercept");
        if (!helperConnected) {
            limitations.put("continuous_helper_not_connected");
        }
        if (!consoleVisible) {
            limitations.put("broker_console_not_foreground");
        }

        JSONObject status = new JSONObject();
        status.put("schema", KIOSK_CONTROL_PLANE_STATUS_SCHEMA);
        status.put("phase", helperConnected ? "BrokerPanelWithShellHelper" : "BrokerPanel2d");
        status.put("surface_intent", surfaceIntent);
        status.put("home_mode", "Normal2d");
        status.put("broker_available", true);
        status.put("broker_panel_visible", consoleVisible);
        status.put("immersive_home_visible", false);
        status.put("shell_helper_connected", helperConnected);
        status.put("continuous_adb_shell_required", helperConnected);
        status.put("watchdog_required", helperConnected);
        status.put("focus_guardian_active", focusGuardianActive);
        status.put("proximity_watchdog_active", proximityWatchdogActive);
        status.put("meta_menu_active", metaMenuActive);
        status.put("meta_menu_entry_intentional", false);
        status.put("active_panel", consoleVisible ? activeBrokerPanelId() : JSONObject.NULL);
        status.put("foreground_package", nullableJsonString(foregroundPackage));
        status.put("foreground_activity", nullableJsonString(foregroundActivity));
        status.put("clock_epoch_id", nullableJsonString(clockStatus.optString("clock_epoch_id", "")));
        status.put("latest_command", latestCommand);
        status.put("limitations", limitations);
        return status;
    }

    JSONObject rustyKioskCommandRunRecordJson(
        String runId,
        String preferredCommand,
        Object statusBefore,
        Object statusAfter,
        String note) throws Exception {
        JSONObject after = statusAfter instanceof JSONObject ? (JSONObject) statusAfter : null;
        String foregroundAfter = after != null ? foregroundLabel(after) : "";
        String clockEpochId = after != null ? after.optString("clock_epoch_id", "") : "";
        String surfaceIntent = after != null ? after.optString("surface_intent", "UnknownSurface") : "UnknownSurface";

        JSONObject primary = kioskCommandEvidenceJson(
            "Companion",
            preferredCommand,
            "GET /kiosk/status",
            JSONObject.NULL,
            foregroundAfter,
            clockEpochId,
            "broker_json_report_path");
        JSONObject fallback = kioskCommandEvidenceJson(
            "Broker",
            "GET /kiosk/status",
            "adb shell dumpsys window",
            JSONObject.NULL,
            foregroundAfter,
            clockEpochId,
            "broker_http_fallback_path");

        JSONObject record = new JSONObject();
        record.put("schema", KIOSK_COMMAND_RUN_RECORD_SCHEMA);
        record.put("run_id", runId != null && runId.length() > 0 ? runId : "broker-kiosk-command-run");
        record.put("command_goal", "surface.current");
        record.put("surface_intent", surfaceIntent);
        record.put("primary", primary);
        record.put("fallback", fallback);
        record.put("status_before", statusBefore != null ? statusBefore : JSONObject.NULL);
        record.put("status_after", statusAfter != null ? statusAfter : JSONObject.NULL);
        record.put("outcome", "Succeeded");
        record.put("issue_codes", new JSONArray());
        JSONArray notes = new JSONArray();
        if (note != null && note.length() > 0) {
            notes.put(note);
        }
        record.put("notes", notes);
        return record;
    }

    private JSONObject kioskCommandEvidenceJson(
        String provider,
        String preferredCommand,
        String fallbackCommand,
        Object foregroundBefore,
        String foregroundAfter,
        String clockEpochId,
        String note) throws Exception {
        JSONObject evidence = new JSONObject();
        evidence.put("schema", KIOSK_COMMAND_EVIDENCE_SCHEMA);
        evidence.put("command_goal", "surface.current");
        evidence.put("provider", provider != null && provider.length() > 0 ? provider : "Unknown");
        evidence.put("preferred_command", preferredCommand != null && preferredCommand.length() > 0 ? preferredCommand : JSONObject.NULL);
        evidence.put("fallback_command", fallbackCommand != null && fallbackCommand.length() > 0 ? fallbackCommand : JSONObject.NULL);
        evidence.put("foreground_before", foregroundBefore != null ? foregroundBefore : JSONObject.NULL);
        evidence.put("foreground_after", foregroundAfter != null && foregroundAfter.length() > 0 ? foregroundAfter : JSONObject.NULL);
        evidence.put("clock_epoch_id", nullableJsonString(clockEpochId));
        JSONArray notes = new JSONArray();
        if (note != null && note.length() > 0) {
            notes.put(note);
        }
        evidence.put("notes", notes);
        return evidence;
    }

    private static String foregroundLabel(JSONObject status) {
        if (status == null) {
            return "";
        }
        String foregroundPackage = status.optString("foreground_package", "");
        String foregroundActivity = status.optString("foreground_activity", "");
        if (foregroundPackage.length() == 0) {
            return "";
        }
        return foregroundActivity.length() > 0 ? foregroundPackage + "/" + foregroundActivity : foregroundPackage;
    }

    JSONObject clockStatusJson() throws Exception {
        return clock.statusJson();
    }

    JSONObject clockSnapshotJson() throws Exception {
        return clock.snapshotJson();
    }

    JSONObject clockDomainsJson() throws Exception {
        return clock.domainsJson();
    }

    JSONObject clockCorrelationsJson() throws Exception {
        return clock.correlationsJson();
    }

    JSONObject clockHealthJson() throws Exception {
        return clock.healthJson();
    }

    JSONObject clockOpenXrComparisonJson() throws Exception {
        return clock.openXrComparisonJson();
    }

    JSONObject clockSyncProbeJson(JSONObject params) throws Exception {
        return clock.syncProbeJson(params);
    }

    JSONObject clockStampJson() throws Exception {
        return clock.stampJson();
    }

    JSONObject clockStampJson(String sourceDomain, Long sourceTimeNs, String correlationId) throws Exception {
        return clock.stampJson(sourceDomain, sourceTimeNs, correlationId);
    }

    JSONObject reportShellHelperStatus(JSONObject params) throws Exception {
        JSONObject status = shellHelper.reportStatus(params);
        cameraProjectionProvider.applyShellHelperDiagnostics(shellHelper.diagnosticsJson());
        return status;
    }

    JSONObject experimentControlJson() throws Exception {
        return experimentControl.toStatusJson();
    }

    JSONObject configureExperimentControl(JSONObject params) throws Exception {
        return experimentControl.configure(params);
    }

    JSONObject reportExperimentStatus(JSONObject params) throws Exception {
        return experimentControl.reportHelperStatus(params);
    }

    JSONObject videoLabStatusJson() throws Exception {
        return videoLab.toStatusJson(
            videoLabMetricSamples.get(),
            videoLabEncodedStreamManifests.get(),
            videoLabEncodedSampleMetadata.get());
    }

    JSONObject videoLabScorecardJson() throws Exception {
        return videoLab.toScorecardJson(
            videoLabMetricSamples.get(),
            videoLabEncodedStreamManifests.get(),
            videoLabEncodedSampleMetadata.get());
    }

    JSONObject transportCapabilitiesJson() throws Exception {
        return transportSessions.capabilitiesJson();
    }

    JSONObject createTransportSession(JSONObject params, String clientId) throws Exception {
        return transportSessions.createSession(params, clientId);
    }

    JSONObject getTransportSession(String sessionId) throws Exception {
        return transportSessions.getSession(sessionId);
    }

    JSONObject listTransportSessions() throws Exception {
        return transportSessions.listSessions();
    }

    JSONObject closeTransportSession(String sessionId, String reason) throws Exception {
        return transportSessions.closeSession(sessionId, reason);
    }

    JSONObject breathAssessmentStatusJson() throws Exception {
        return breathAssessment.toStatusJson();
    }

    synchronized void updatePolarHeartRateStatus(JSONObject status) throws Exception {
        polarHeartRateStatus = status == null ? defaultPolarHeartRateStatus() : new JSONObject(status.toString());
    }

    synchronized JSONObject polarHeartRateStatusJson() throws Exception {
        return new JSONObject(polarHeartRateStatus.toString());
    }

    synchronized void updatePolarPmdStatus(JSONObject status) throws Exception {
        polarPmdStatus = status == null ? defaultPolarPmdStatus() : new JSONObject(status.toString());
    }

    synchronized JSONObject polarPmdStatusJson() throws Exception {
        return new JSONObject(polarPmdStatus.toString());
    }

    synchronized boolean hasPolarPmdFrames() {
        return polarPmdStatus.optLong("acc_frame_count", 0L) > 0L
            || polarPmdStatus.optLong("ecg_frame_count", 0L) > 0L;
    }

    synchronized void updateDeviceWatchdogStatus(JSONObject status) throws Exception {
        deviceWatchdogStatus = status == null ? defaultDeviceWatchdogStatus() : new JSONObject(status.toString());
    }

    synchronized JSONObject deviceWatchdogStatusJson() throws Exception {
        return new JSONObject(deviceWatchdogStatus.toString());
    }

    JSONObject configureBreathAssessment(JSONObject params) throws Exception {
        return breathAssessment.configure(params);
    }

    JSONObject setPolarBreathParams(JSONObject params) throws Exception {
        return breathAssessment.setPolarBreathParams(params);
    }

    JSONObject beginPolarBreathCalibration(JSONObject params) throws Exception {
        return breathAssessment.beginPolarBreathCalibration(params);
    }

    JSONObject resetPolarBreathCalibration(JSONObject params) throws Exception {
        return breathAssessment.resetPolarBreathCalibration(params);
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

    private static JSONObject defaultPolarHeartRateStatus() {
        JSONObject status = new JSONObject();
        try {
            status.put("schema", PolarHeartRateBrokerSource.STATUS_SCHEMA);
            status.put("enabled", false);
            status.put("state", "idle");
            status.put("input_stream", PolarHeartRateBrokerSource.STREAM_ID);
            status.put("heart_rate_event_count", 0L);
            status.put("rr_interval_count", 0L);
            status.put("last_error", "");
            JSONArray limitations = new JSONArray();
            limitations.put("requires_android_ble_permissions");
            limitations.put("uses_standard_heart_rate_service_only");
            limitations.put("does_not_open_polar_pmd_control_or_data_characteristics");
            status.put("limitations", limitations);
        } catch (Exception ignored) {
        }
        return status;
    }

    private static JSONObject defaultPolarPmdStatus() {
        JSONObject status = new JSONObject();
        try {
            status.put("schema", PolarPmdBrokerSource.STATUS_SCHEMA);
            status.put("enabled", false);
            status.put("state", "idle");
            status.put("input_stream", BreathAssessmentState.POLAR_INPUT_STREAM);
            status.put("requested_pmd_stream", PolarPmdBrokerSource.PMD_STREAM_ACC);
            status.put("active_pmd_stream", PolarPmdBrokerSource.PMD_STREAM_ACC);
            status.put("active_measurement_type", PolarPmdProtocol.MEASUREMENT_TYPE_ACC & 0xff);
            status.put("output_stream", BreathAssessmentState.OUTPUT_STREAM);
            status.put("acc_frame_count", 0L);
            status.put("acc_sample_count", 0L);
            status.put("ecg_frame_count", 0L);
            status.put("ecg_sample_count", 0L);
            status.put("malformed_frame_count", 0L);
            status.put("last_error", "");
        } catch (Exception ignored) {
        }
        return status;
    }

    private static JSONObject defaultDeviceWatchdogStatus() {
        JSONObject status = new JSONObject();
        try {
            status.put("schema", DeviceWatchdog.STATUS_SCHEMA);
            status.put("running", false);
            status.put("run_id", "");
            status.put("interval_ms", 30_000L);
            status.put("started_unix_ms", 0L);
            status.put("started_elapsed_ms", 0L);
            status.put("uptime_ms", 0L);
            status.put("sample_count", 0L);
            status.put("log_path", "");
            status.put("wake_lock_requested", false);
            status.put("wake_lock_held", false);
            status.put("max_log_bytes", 8L * 1024L * 1024L);
            status.put("last_error", "");
            status.put("stop_reason", "");
            status.put("latest_sample", new JSONObject());
            JSONArray limitations = new JSONArray();
            limitations.put("normal_app_uid_not_android_shell");
            limitations.put("requires_broker_or_activity_launch_after_full_reboot");
            limitations.put("sleep_and_doze_policy_is_platform_owned");
            limitations.put("powered_off_device_cannot_run_watchdog");
            status.put("limitations", limitations);
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
                if (device.has("sensor_orientation_degrees")) {
                    candidate.put("sensor_orientation_degrees", device.optInt("sensor_orientation_degrees", 0));
                }
                candidate.put("lens_pose_rotation_xyzw", copyArray(device.optJSONArray("lens_pose_rotation_xyzw")));
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

    private static final class ExperimentControlState {
        private static final String BROKER_PACKAGE = "com.example.rustyxr.broker";
        private static final String BROKER_ACTIVITY = "com.example.rustyxr.broker.MainActivity";
        private static final String PROP_STRENGTH =
            "debug.rustyquest.makepad.horizontal.alignment.strength";
        private static final String PROP_GLOBAL_UV =
            "debug.rustyquest.makepad.horizontal.offset.uv";
        private static final String PROP_LEFT_UV =
            "debug.rustyquest.makepad.horizontal.offset.left.uv";
        private static final String PROP_RIGHT_UV =
            "debug.rustyquest.makepad.horizontal.offset.right.uv";
        private static final String PROP_VERTICAL_UV =
            "debug.rustyquest.makepad.vertical.offset.uv";
        private static final String PROP_CONTENT_SCALE =
            "debug.rustyquest.makepad.content.uv.scale";

        private long revision;
        private long lastUpdatedUnixMs;
        private boolean enabled;
        private String mode = "off";
        private String desiredFocus = "broker";
        private String targetPackage = "";
        private String targetActivity = "";
        private double strength = 0.0d;
        private double globalUv = 0.0d;
        private double leftUv = 0.0d;
        private double rightUv = 0.0d;
        private double verticalUv = 0.0d;
        private double contentScale = 1.60d;
        private int launchGuardTimeoutMs = 20_000;
        private boolean launchGuardPreviewTimeoutEnabled = false;
        private JSONObject helperStatus = new JSONObject();

        synchronized JSONObject toStatusJson() throws Exception {
            JSONObject status = new JSONObject();
            status.put("schema", "rusty.xr.broker.experiment_control.v1");
            status.put("revision", revision);
            status.put("last_updated_unix_ms", lastUpdatedUnixMs);
            status.put("enabled", enabled);
            status.put("mode", mode);
            status.put("desired_focus", desiredFocus);
            status.put("target_package", targetPackage);
            status.put("target_activity", targetActivity);
            status.put("target_component", componentName(targetPackage, targetActivity));
            status.put("broker_package", BROKER_PACKAGE);
            status.put("broker_activity", BROKER_ACTIVITY);
            status.put("broker_component", componentName(BROKER_PACKAGE, BROKER_ACTIVITY));
            status.put("launch_guard_timeout_ms", launchGuardTimeoutMs);
            status.put("launch_guard_preview_timeout_enabled", launchGuardPreviewTimeoutEnabled);
            status.put("makepad_tuning", makepadTuningJson());
            status.put("property_writes", makepadPropertyWritesJson());
            status.put("helper_status", new JSONObject(helperStatus.toString()));
            JSONArray limitations = new JSONArray();
            limitations.put("reactive_focus_recovery_not_preemptive_home_intercept");
            limitations.put("requires_adb_shell_helper_for_setprop_and_focus_recovery");
            limitations.put("does_not_override_guardian_permissions_or_safety_ui");
            status.put("limitations", limitations);
            return status;
        }

        synchronized JSONObject configure(JSONObject params) throws Exception {
            if (params == null) {
                params = new JSONObject();
            }

            if (params.has("enabled")) {
                enabled = params.optBoolean("enabled", enabled);
            }
            if (params.has("mode")) {
                mode = normalizeMode(params.optString("mode", mode));
                enabled = !"off".equals(mode);
            }
            if (params.has("desired_focus")) {
                desiredFocus = normalizeFocus(params.optString("desired_focus", desiredFocus));
            }
            if (params.has("target_package")) {
                targetPackage = clean(params.optString("target_package", targetPackage));
            }
            if (params.has("target_activity")) {
                targetActivity = clean(params.optString("target_activity", targetActivity));
            }
            if (params.has("launch_guard_timeout_ms")) {
                launchGuardTimeoutMs = clampInt(
                    params.optInt("launch_guard_timeout_ms", launchGuardTimeoutMs),
                    5_000,
                    120_000);
            }
            if (params.has("launch_guard_preview_timeout_enabled")) {
                launchGuardPreviewTimeoutEnabled =
                    params.optBoolean("launch_guard_preview_timeout_enabled", launchGuardPreviewTimeoutEnabled);
            }

            boolean reset = params.optBoolean("reset_makepad_tuning", false);
            if (reset) {
                strength = 0.0d;
                globalUv = 0.0d;
                leftUv = 0.0d;
                rightUv = 0.0d;
                verticalUv = 0.0d;
                contentScale = 1.60d;
            }

            if (params.has("strength")) {
                strength = clamp(params.optDouble("strength", strength), -4.0d, 4.0d);
            }
            if (params.has("global_uv")) {
                globalUv = clamp(params.optDouble("global_uv", globalUv), -0.5d, 0.5d);
            }
            if (params.has("left_uv")) {
                leftUv = clamp(params.optDouble("left_uv", leftUv), -0.5d, 0.5d);
            }
            if (params.has("right_uv")) {
                rightUv = clamp(params.optDouble("right_uv", rightUv), -0.5d, 0.5d);
            }
            if (params.has("vertical_uv")) {
                verticalUv = clamp(params.optDouble("vertical_uv", verticalUv), -0.5d, 0.5d);
            }
            if (params.has("symmetric_uv")) {
                double symmetric = clamp(params.optDouble("symmetric_uv", 0.0d), -0.5d, 0.5d);
                leftUv = symmetric;
                rightUv = -symmetric;
            }
            if (params.has("content_scale")) {
                contentScale = clamp(params.optDouble("content_scale", contentScale), 1.0d, 2.4d);
            }

            revision++;
            lastUpdatedUnixMs = System.currentTimeMillis();
            return toStatusJson();
        }

        synchronized JSONObject reportHelperStatus(JSONObject params) throws Exception {
            helperStatus = params != null ? new JSONObject(params.toString()) : new JSONObject();
            helperStatus.put("last_report_unix_ms", System.currentTimeMillis());
            return toStatusJson();
        }

        private JSONObject makepadTuningJson() throws Exception {
            JSONObject tuning = new JSONObject();
            tuning.put("schema", "rusty.xr.broker.makepad_tuning.v1");
            tuning.put("strength", strength);
            tuning.put("global_uv", globalUv);
            tuning.put("left_uv", leftUv);
            tuning.put("right_uv", rightUv);
            tuning.put("vertical_uv", verticalUv);
            tuning.put("content_scale", contentScale);
            tuning.put("reset_strength", 0.0d);
            tuning.put("reset_global_uv", 0.0d);
            tuning.put("reset_left_uv", 0.0d);
            tuning.put("reset_right_uv", 0.0d);
            tuning.put("reset_vertical_uv", 0.0d);
            tuning.put("reset_content_scale", 1.60d);
            return tuning;
        }

        private JSONArray makepadPropertyWritesJson() throws Exception {
            JSONArray writes = new JSONArray();
            putPropertyWrite(writes, PROP_STRENGTH, strength);
            putPropertyWrite(writes, PROP_GLOBAL_UV, globalUv);
            putPropertyWrite(writes, PROP_LEFT_UV, leftUv);
            putPropertyWrite(writes, PROP_RIGHT_UV, rightUv);
            putPropertyWrite(writes, PROP_VERTICAL_UV, verticalUv);
            putPropertyWrite(writes, PROP_CONTENT_SCALE, contentScale);
            return writes;
        }

        private static void putPropertyWrite(JSONArray writes, String name, double value) throws Exception {
            JSONObject write = new JSONObject();
            write.put("name", name);
            write.put("value", formatDouble(value));
            writes.put(write);
        }

        private static String normalizeMode(String value) {
            String normalized = clean(value).toLowerCase(Locale.ROOT);
            if ("observe".equals(normalized) ||
                "recover_target".equals(normalized) ||
                "recover_broker".equals(normalized) ||
                "toggle_broker_target".equals(normalized) ||
                "launch_target_guard".equals(normalized) ||
                "strict".equals(normalized)) {
                return normalized;
            }
            return "off";
        }

        private static String normalizeFocus(String value) {
            String normalized = clean(value).toLowerCase(Locale.ROOT);
            if ("target".equals(normalized) || "broker".equals(normalized)) {
                return normalized;
            }
            return "broker";
        }

        private static String componentName(String packageName, String activityName) {
            if (clean(packageName).length() == 0) {
                return "";
            }
            if (clean(activityName).length() == 0) {
                return packageName;
            }
            return packageName + "/" + activityName;
        }

        private static String clean(String value) {
            return value != null ? value.trim() : "";
        }

        private static double clamp(double value, double min, double max) {
            if (Double.isNaN(value) || Double.isInfinite(value)) {
                return min;
            }
            return Math.max(min, Math.min(max, value));
        }

        private static int clampInt(int value, int min, int max) {
            return Math.max(min, Math.min(max, value));
        }

        private static String formatDouble(double value) {
            String formatted = String.format(Locale.ROOT, "%.6f", value);
            while (formatted.indexOf('.') >= 0 && formatted.endsWith("0")) {
                formatted = formatted.substring(0, formatted.length() - 1);
            }
            if (formatted.endsWith(".")) {
                formatted = formatted.substring(0, formatted.length() - 1);
            }
            return formatted;
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

        synchronized JSONObject toScorecardJson(
            long acceptedMetricSamples,
            long acceptedEncodedStreamManifests,
            long acceptedEncodedSampleMetadata) throws Exception {
            boolean hasManifest = latestEncodedStreamManifest.length() > 0;
            boolean hasMetric = latestMetricSample.length() > 0;
            String manifestSessionId = latestEncodedStreamManifest.optString("session_id", "");
            String metricSessionId = latestMetricSample.optString("session_id", "");
            boolean metricMatchesManifest = hasManifest && hasMetric && metricSessionId.equals(manifestSessionId);
            String payloadTransport = hasManifest
                ? latestEncodedStreamManifest.optString("payload_transport", "")
                : latestMetricSample.optString("payload_transport", "");
            boolean hasPayloadTransport = payloadTransport.length() > 0 && !"metadata_only".equals(payloadTransport);
            boolean hasLastError = latestMetricSample.optString("last_error", "").length() > 0;
            long packetCount = latestMetricSample.optLong("packet_count", 0L);
            long payloadBytes = latestMetricSample.optLong("payload_size_bytes", 0L);
            boolean payloadTransportReady = hasPayloadTransport && hasMetric && !hasLastError &&
                packetCount > 0L && payloadBytes > 0L && (!hasManifest || metricMatchesManifest);
            boolean nativeDecodeVerified = payloadTransportReady &&
                "broker_app_camera2_mediacodec_decode_probe".equals(latestMetricSample.optString("source", "")) &&
                latestMetricSample.optBoolean("decode_succeeded", false) &&
                latestMetricSample.optLong("decoded_frame_count", 0L) > 0L;
            boolean binaryProxyVerified = payloadTransportReady &&
                "broker_peer_tcp_binary_proxy".equals(payloadTransport) &&
                "broker_peer_h264_tcp_proxy".equals(latestMetricSample.optString("source", ""));

            JSONObject scorecard = new JSONObject();
            scorecard.put("schema", "rusty.xr.video_lab.scorecard.v1");
            scorecard.put("state", !hasManifest && !hasMetric
                ? "empty"
                : hasLastError
                ? "failed"
                : (nativeDecodeVerified || binaryProxyVerified || payloadTransportReady)
                ? "passed"
                : "pending");
            scorecard.put("session_id", metricSessionId.length() > 0 ? metricSessionId : manifestSessionId);
            scorecard.put("stream_id", latestMetricSample.optString(
                "stream_id",
                latestEncodedStreamManifest.optString("stream_id", "")));
            scorecard.put("source", latestMetricSample.optString(
                "source",
                latestEncodedStreamManifest.optString("source", "")));
            scorecard.put("codec", latestMetricSample.optString(
                "codec",
                latestEncodedStreamManifest.optString("codec", "")));
            scorecard.put("payload_transport", payloadTransport);
            scorecard.put("transport", latestMetricSample.optString(
                "transport",
                latestEncodedStreamManifest.optString("transport", "")));
            scorecard.put("width", latestMetricSample.optInt(
                "width",
                latestEncodedStreamManifest.optInt("width", 0)));
            scorecard.put("height", latestMetricSample.optInt(
                "height",
                latestEncodedStreamManifest.optInt("height", 0)));
            scorecard.put("camera_source_id", latestMetricSample.optString(
                "camera_source_id",
                latestEncodedStreamManifest.optString("camera_source_id", "")));
            scorecard.put("source_api_path", latestMetricSample.optString(
                "source_api_path",
                latestEncodedStreamManifest.optString("source_api_path", "")));
            scorecard.put("camera_permission_state", latestMetricSample.optString(
                "camera_permission_state",
                latestEncodedStreamManifest.optString("camera_permission_state", "")));
            scorecard.put("headset_camera_permission_state", latestMetricSample.optString(
                "headset_camera_permission_state",
                latestEncodedStreamManifest.optString("headset_camera_permission_state", "")));
            scorecard.put("selected_camera_id", latestMetricSample.optString(
                "selected_camera_id",
                latestEncodedStreamManifest.optString("selected_camera_id", latestEncodedStreamManifest.optString("camera_id", ""))));
            scorecard.put("selected_width", latestMetricSample.optInt(
                "selected_width",
                latestEncodedStreamManifest.optInt("selected_width", latestEncodedStreamManifest.optInt("width", 0))));
            scorecard.put("selected_height", latestMetricSample.optInt(
                "selected_height",
                latestEncodedStreamManifest.optInt("selected_height", latestEncodedStreamManifest.optInt("height", 0))));
            scorecard.put("selected_fps_min_hz", latestMetricSample.optInt(
                "selected_fps_min_hz",
                latestEncodedStreamManifest.optInt("selected_fps_min_hz", 0)));
            scorecard.put("selected_fps_max_hz", latestMetricSample.optInt(
                "selected_fps_max_hz",
                latestEncodedStreamManifest.optInt("selected_fps_max_hz", 0)));
            scorecard.put("selected_reason", latestMetricSample.optString(
                "selected_reason",
                latestEncodedStreamManifest.optString("selected_reason", "")));
            scorecard.put("stream_min_frame_duration_ns", latestMetricSample.optLong(
                "stream_min_frame_duration_ns",
                latestEncodedStreamManifest.optLong("stream_min_frame_duration_ns", 0L)));
            scorecard.put("timestamp_domain", latestMetricSample.optString(
                "timestamp_domain",
                latestEncodedStreamManifest.optString("timestamp_domain", "")));
            scorecard.put("accepted_metric_samples", acceptedMetricSamples);
            scorecard.put("accepted_encoded_stream_manifests", acceptedEncodedStreamManifests);
            scorecard.put("accepted_encoded_sample_metadata", acceptedEncodedSampleMetadata);
            scorecard.put("packet_count", packetCount);
            scorecard.put("video_packet_count", latestMetricSample.optLong("video_packet_count", 0L));
            scorecard.put("codec_config_packet_count", latestMetricSample.optLong("codec_config_packet_count", 0L));
            scorecard.put("payload_size_bytes", payloadBytes);
            scorecard.put("wire_size_bytes", latestMetricSample.optLong("wire_size_bytes", 0L));
            scorecard.put("dropped_frames", latestMetricSample.optLong("dropped_frames", 0L));
            scorecard.put("stale_frames", latestMetricSample.optLong("stale_frames", 0L));
            scorecard.put("queue_depth", latestMetricSample.optLong("queue_depth", 0L));
            scorecard.put("writer_backpressure_isolated", latestMetricSample.optBoolean(
                "writer_backpressure_isolated",
                latestEncodedStreamManifest.optBoolean("writer_backpressure_isolated", false)));
            scorecard.put("writer_packet_count", latestMetricSample.optLong("writer_packet_count", 0L));
            scorecard.put("writer_queue_capacity", latestMetricSample.optLong("writer_queue_capacity", 0L));
            scorecard.put("writer_queue_enqueued_packets", latestMetricSample.optLong("writer_queue_enqueued_packets", 0L));
            scorecard.put("writer_queue_max_depth", latestMetricSample.optLong("writer_queue_max_depth", 0L));
            scorecard.put("writer_queue_final_depth", latestMetricSample.optLong("writer_queue_final_depth", 0L));
            scorecard.put("writer_queue_dropped_packets", latestMetricSample.optLong("writer_queue_dropped_packets", 0L));
            scorecard.put("writer_queue_dropped_video_packets", latestMetricSample.optLong("writer_queue_dropped_video_packets", 0L));
            scorecard.put("writer_queue_dropped_non_keyframe_packets", latestMetricSample.optLong("writer_queue_dropped_non_keyframe_packets", 0L));
            scorecard.put("writer_queue_dropped_keyframe_packets", latestMetricSample.optLong("writer_queue_dropped_keyframe_packets", 0L));
            scorecard.put("writer_queue_dropped_codec_config_packets", latestMetricSample.optLong("writer_queue_dropped_codec_config_packets", 0L));
            scorecard.put("writer_queue_dropped_incoming_packets", latestMetricSample.optLong("writer_queue_dropped_incoming_packets", 0L));
            scorecard.put("decode_succeeded", latestMetricSample.optBoolean("decode_succeeded", false));
            scorecard.put("decoded_frame_count", latestMetricSample.optLong("decoded_frame_count", 0L));
            scorecard.put("camera_encode_duration_ns", latestMetricSample.optLong("camera_encode_duration_ns", 0L));
            scorecard.put("decoder_duration_ns", latestMetricSample.optLong("decoder_duration_ns", 0L));
            scorecard.put("decoder_low_latency_feature_supported", latestMetricSample.optBoolean(
                "decoder_low_latency_feature_supported",
                latestEncodedStreamManifest.optBoolean("decoder_low_latency_feature_supported", false)));
            scorecard.put("decoder_low_latency_config_requested", latestMetricSample.optBoolean(
                "decoder_low_latency_config_requested",
                latestEncodedStreamManifest.optBoolean("decoder_low_latency_config_requested", false)));
            scorecard.put("decoder_low_latency_parameter_succeeded", latestMetricSample.optBoolean(
                "decoder_low_latency_parameter_succeeded",
                latestEncodedStreamManifest.optBoolean("decoder_low_latency_parameter_succeeded", false)));
            scorecard.put("proxy_forward_duration_ns", latestMetricSample.optLong("proxy_forward_duration_ns", 0L));
            scorecard.put("encoder_name", latestMetricSample.optString(
                "encoder_name",
                latestEncodedStreamManifest.optString("encoder_name", "")));
            scorecard.put("decoder_name", latestMetricSample.optString(
                "decoder_name",
                latestEncodedStreamManifest.optString("decoder_name", "")));
            scorecard.put("encoder_hardware_accelerated", latestMetricSample.optBoolean(
                "encoder_hardware_accelerated",
                latestEncodedStreamManifest.optBoolean("encoder_hardware_accelerated", false)));
            scorecard.put("bitrate_mode_requested", latestMetricSample.optString(
                "bitrate_mode_requested",
                latestEncodedStreamManifest.optString("bitrate_mode_requested", "")));
            scorecard.put("bitrate_mode_applied", latestMetricSample.optString(
                "bitrate_mode_applied",
                latestEncodedStreamManifest.optString("bitrate_mode_applied", "")));
            scorecard.put("csd_sps_bytes", latestMetricSample.optLong(
                "csd_sps_bytes",
                latestEncodedStreamManifest.optLong("csd_sps_bytes", 0L)));
            scorecard.put("csd_pps_bytes", latestMetricSample.optLong(
                "csd_pps_bytes",
                latestEncodedStreamManifest.optLong("csd_pps_bytes", 0L)));
            scorecard.put("sps_present", latestMetricSample.optBoolean(
                "sps_present",
                latestEncodedStreamManifest.optLong("csd_sps_bytes", 0L) > 0L));
            scorecard.put("pps_present", latestMetricSample.optBoolean(
                "pps_present",
                latestEncodedStreamManifest.optLong("csd_pps_bytes", 0L) > 0L));
            scorecard.put("keyframe_count", latestMetricSample.optLong("keyframe_count", 0L));
            scorecard.put("sync_frame_request_count", latestMetricSample.optBoolean(
                "sync_frame_request_on_start_succeeded",
                latestEncodedStreamManifest.optBoolean("sync_frame_request_on_start_succeeded", false)) ? 1L : 0L);
            scorecard.put("sync_frame_request_on_start_succeeded", latestMetricSample.optBoolean(
                "sync_frame_request_on_start_succeeded",
                latestEncodedStreamManifest.optBoolean("sync_frame_request_on_start_succeeded", false)));
            scorecard.put("sensor_timestamp_source", latestMetricSample.optString(
                "sensor_timestamp_source",
                latestEncodedStreamManifest.optString("sensor_timestamp_source", "")));
            scorecard.put("camera_capture_started_count", latestMetricSample.optLong(
                "camera_capture_started_count",
                latestEncodedStreamManifest.optLong("camera_capture_started_count", 0L)));
            scorecard.put("camera_first_frame_number", latestMetricSample.optLong(
                "camera_first_frame_number",
                latestEncodedStreamManifest.optLong("camera_first_frame_number", -1L)));
            scorecard.put("camera_last_frame_number", latestMetricSample.optLong(
                "camera_last_frame_number",
                latestEncodedStreamManifest.optLong("camera_last_frame_number", -1L)));
            if (hasLastError) {
                scorecard.put("last_error", latestMetricSample.optString("last_error", ""));
            }

            JSONObject evidence = new JSONObject();
            evidence.put("has_manifest", hasManifest);
            evidence.put("has_metric", hasMetric);
            evidence.put("metric_matches_manifest", metricMatchesManifest);
            evidence.put("payload_transport_ready", payloadTransportReady);
            evidence.put("native_decode_verified", nativeDecodeVerified);
            evidence.put("binary_proxy_verified", binaryProxyVerified);
            scorecard.put("evidence", evidence);

            JSONArray limitations = new JSONArray();
            limitations.put("latest_sample_only");
            limitations.put("no_high_rate_payload_over_json_websocket");
            if (nativeDecodeVerified) {
                limitations.put("decode_probe_outputs_byte_buffers_not_xr_textures");
            }
            if (binaryProxyVerified) {
                limitations.put("proxy_probe_uses_bounded_synthetic_payloads");
            }
            limitations.put("xr_layer_submit_client_owned");
            scorecard.put("limitations", limitations);
            return scorecard;
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

    private static final class TransportSessionRegistry {
        private final Map<String, JSONObject> sessions = new LinkedHashMap<>();
        private long created;
        private long closed;
        private long failed;

        synchronized long createdCount() {
            return created;
        }

        synchronized long closedCount() {
            return closed;
        }

        synchronized long failedCount() {
            return failed;
        }

        synchronized JSONObject capabilitiesJson() throws Exception {
            JSONObject capabilities = new JSONObject();
            capabilities.put("schema", "rusty.xr.broker.transport_capabilities.v1");
            capabilities.put("control_schema", "rusty.xr.broker.transport_session_offer.v1");
            capabilities.put("answer_schema", "rusty.xr.broker.transport_session_answer.v1");
            capabilities.put("security_schema", "rusty.xr.broker.transport_security_policy.v1");

            JSONArray transports = new JSONArray();
            transports.put("WebSocket");
            transports.put("Tcp");
            transports.put("AdbForwardedTcp");
            transports.put("MetadataOnly");
            capabilities.put("supported_transports", transports);

            JSONArray streamKinds = new JSONArray();
            streamKinds.put("Media");
            streamKinds.put("Telemetry");
            streamKinds.put("Control");
            streamKinds.put("XrInput");
            streamKinds.put("Bio");
            streamKinds.put("Synthetic");
            capabilities.put("stream_kinds", streamKinds);

            JSONArray securityModes = new JSONArray();
            securityModes.put("LoopbackOnly");
            securityModes.put("PairingToken");
            capabilities.put("security_modes", securityModes);

            JSONArray limitations = new JSONArray();
            limitations.put("in_memory_registry");
            limitations.put("loopback_only_default");
            limitations.put("no_binary_payload_change");
            limitations.put("client_owned_decode_texture_import_and_xr_submit");
            capabilities.put("limitations", limitations);
            return capabilities;
        }

        synchronized JSONObject statusJson() throws Exception {
            JSONObject status = new JSONObject();
            status.put("schema", "rusty.xr.broker.transport_session_registry.v1");
            status.put("active_count", activeCount());
            status.put("created_count", created);
            status.put("closed_count", closed);
            status.put("failed_count", failed);
            status.put("sessions", sessionsArray());
            return status;
        }

        synchronized JSONObject listSessions() throws Exception {
            JSONObject result = new JSONObject();
            result.put("schema", "rusty.xr.broker.transport_session_list.v1");
            result.put("sessions", sessionsArray());
            result.put("active_count", activeCount());
            return result;
        }

        synchronized JSONObject getSession(String sessionId) throws Exception {
            JSONObject session = sessions.get(sessionId);
            if (session == null) {
                JSONObject result = new JSONObject();
                result.put("found", false);
                result.put("session_id", sessionId != null ? sessionId : "");
                return result;
            }

            JSONObject result = new JSONObject();
            result.put("found", true);
            result.put("session", new JSONObject(session.toString()));
            return result;
        }

        synchronized JSONObject createSession(JSONObject params, String clientId) throws Exception {
            JSONObject offer = params != null ? new JSONObject(params.toString()) : new JSONObject();
            String sessionId = offer.optString("session_id", "");
            if (sessionId.trim().length() == 0) {
                sessionId = "transport-session-" + SystemClock.elapsedRealtimeNanos();
            }
            JSONArray streams = offer.optJSONArray("streams");
            if (streams == null || streams.length() == 0) {
                streams = new JSONArray();
                streams.put(defaultSyntheticStream());
            }

            JSONObject security = normalizeSecurity(offer.optJSONObject("security"));
            JSONObject session = new JSONObject();
            session.put("schema", "rusty.xr.broker.transport_session_answer.v1");
            session.put("session_id", sessionId);
            session.put("client_id", clientId != null ? clientId : "");
            session.put("accepted", true);
            session.put("state", "Accepted");
            session.put("selected_transport", selectTransport(offer.optJSONArray("requested_transports")));
            session.put("accepted_streams", new JSONArray(streams.toString()));
            session.put("security", security);
            session.put("created_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            session.put("last_heartbeat_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            session.put("reason", JSONObject.NULL);
            sessions.put(sessionId, session);
            created++;
            return new JSONObject(session.toString());
        }

        synchronized JSONObject closeSession(String sessionId, String reason) throws Exception {
            JSONObject session = sessions.get(sessionId);
            if (session == null) {
                failed++;
                JSONObject result = new JSONObject();
                result.put("found", false);
                result.put("session_id", sessionId != null ? sessionId : "");
                result.put("state", "Failed");
                result.put("reason", "unknown_session");
                return result;
            }

            session.put("state", "Closed");
            session.put("closed_elapsed_ns", SystemClock.elapsedRealtimeNanos());
            session.put("reason", reason != null && reason.trim().length() > 0 ? reason.trim() : "closed_by_command");
            closed++;
            return new JSONObject(session.toString());
        }

        private int activeCount() {
            int count = 0;
            for (JSONObject session : sessions.values()) {
                String state = session.optString("state", "");
                if (!"Closed".equals(state) && !"Failed".equals(state)) {
                    count++;
                }
            }
            return count;
        }

        private JSONArray sessionsArray() throws Exception {
            JSONArray array = new JSONArray();
            for (JSONObject session : sessions.values()) {
                array.put(new JSONObject(session.toString()));
            }
            return array;
        }

        private JSONObject defaultSyntheticStream() throws Exception {
            JSONObject stream = new JSONObject();
            stream.put("stream_id", "synthetic:wave");
            stream.put("stream_kind", "Synthetic");
            stream.put("direction", "ProducerToConsumer");
            stream.put("payload_kind", "Json");
            stream.put("payload_schema", "rusty.xr.synthetic.wave.v1");
            stream.put("codec", "Json");
            stream.put("reliability", "LossTolerant");
            stream.put("ordered", false);
            stream.put("nominal_rate_hz", 90.0);
            stream.put("target_latency_ms", 30.0);
            stream.put("max_payload_bytes", 4096);
            return stream;
        }

        private JSONObject normalizeSecurity(JSONObject requested) throws Exception {
            JSONObject security = requested != null ? new JSONObject(requested.toString()) : new JSONObject();
            String mode = security.optString("mode", "LoopbackOnly");
            security.put("schema", "rusty.xr.broker.transport_security_policy.v1");
            security.put("mode", mode);
            boolean nonLoopback = !"LoopbackOnly".equals(mode) && security.optBoolean("non_loopback_allowed", true);
            security.put("non_loopback_allowed", nonLoopback);
            security.put("pairing_token_required", "PairingToken".equals(mode));
            if (!security.has("expires_elapsed_ns")) {
                security.put("expires_elapsed_ns", JSONObject.NULL);
            }
            if (!security.has("capability_scope")) {
                security.put("capability_scope", new JSONArray());
            }
            return security;
        }

        private String selectTransport(JSONArray requested) {
            if (requested != null) {
                for (int i = 0; i < requested.length(); i++) {
                    String value = requested.optString(i, "");
                    if ("AdbForwardedTcp".equals(value) ||
                        "Tcp".equals(value) ||
                        "WebSocket".equals(value) ||
                        "MetadataOnly".equals(value)) {
                        return value;
                    }
                }
            }
            return "AdbForwardedTcp";
        }
    }
}
