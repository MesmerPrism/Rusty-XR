#!/usr/bin/env python3
"""Export hand-reviewed public JSON Schemas for stable Rusty XR contracts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


SCHEMA_VERSION = "https://json-schema.org/draft/2020-12/schema"


def obj(title: str, properties: dict, required: list[str] | None = None) -> dict:
    return {
        "$schema": SCHEMA_VERSION,
        "title": title,
        "type": "object",
        "additionalProperties": False,
        "properties": properties,
        "required": required or list(properties.keys()),
    }


def enum(title: str, values: list[str]) -> dict:
    return {"title": title, "type": "string", "enum": values}


def number() -> dict:
    return {"type": "number"}


def integer(minimum: int | None = None) -> dict:
    schema = {"type": "integer"}
    if minimum is not None:
        schema["minimum"] = minimum
    return schema


def string() -> dict:
    return {"type": "string"}


def nullable_string() -> dict:
    return {"type": ["string", "null"]}


def boolean() -> dict:
    return {"type": "boolean"}


def array(items: dict) -> dict:
    return {"type": "array", "items": items}


def vec2() -> dict:
    return obj("Vec2", {"x": number(), "y": number()})


def vec3() -> dict:
    return obj("Vec3", {"x": number(), "y": number(), "z": number()})


def image_size() -> dict:
    return obj("ImageSize", {"width": integer(0), "height": integer(0)})


def pose() -> dict:
    return obj(
        "Pose",
        {
            "position": vec3(),
            "orientation": obj("Quat", {"x": number(), "y": number(), "z": number(), "w": number()}),
        },
    )


def schemas() -> dict[str, dict]:
    runtime_value = {
        "oneOf": [
            obj("RuntimeBool", {"Bool": {"type": "boolean"}}),
            obj("RuntimeInteger", {"Integer": {"type": "integer"}}),
            obj("RuntimeFloat", {"Float": number()}),
            obj("RuntimeText", {"Text": {"type": "string"}}),
        ]
    }
    lsl_descriptor = obj(
        "LslStreamDescriptor",
        {
            "name": {"type": "string"},
            "stream_type": {"type": "string"},
            "source_id": {"type": ["string", "null"]},
            "channel_count": integer(1),
            "nominal_srate_hz": {"type": ["number", "null"], "minimum": 0},
            "channel_format": enum(
                "LslChannelFormat", ["Float32", "Double64", "Int32", "Int16", "Int8", "String"]
            ),
            "role": {
                "type": ["string", "null"],
                "enum": [
                    None,
                    "Biofeedback",
                    "ClockProbe",
                    "ClockEcho",
                    "ParticleTelemetry",
                    "PolarHeartRate",
                    "PolarEcg",
                    "PolarAccelerometer",
                    "Custom",
                ],
            },
        },
    )
    capture_source_kind = enum(
        "CaptureSourceKind",
        [
            "Unknown",
            "PassthroughCamera",
            "EnvironmentDepth",
            "MediaProjection",
            "RoomMesh",
            "AppRender",
            "ImportedFile",
            "Synthetic",
        ],
    )
    capture_lifecycle_state = enum(
        "CaptureLifecycleState",
        ["Unavailable", "PermissionRequired", "Idle", "Starting", "Running", "Paused", "Stopping", "Failed"],
    )
    capture_permission_state = enum(
        "CapturePermissionState",
        ["Unknown", "NotRequired", "Required", "Requesting", "Granted", "Denied", "Blocked"],
    )
    room_mesh_source_kind = enum(
        "RoomMeshSourceKind",
        ["Unknown", "RuntimeRoomMesh", "SemanticSceneModel", "DepthFusion", "ImportedMesh", "Synthetic"],
    )
    room_mesh_surface = obj(
        "RoomMeshSurface",
        {
            "label": enum(
                "RoomMeshSemanticLabel",
                ["Unknown", "Floor", "Ceiling", "Wall", "Door", "Window", "Table", "Seat", "Platform", "Other"],
            ),
            "first_triangle_index": integer(0),
            "triangle_count": integer(1),
            "confidence": {"type": "integer", "minimum": 0, "maximum": 255},
            "last_seen_time_ns": {"type": ["integer", "null"], "minimum": 0},
        },
    )
    quest_catalog_app = obj(
        "QuestCatalogApp",
        {
            "id": {"type": "string"},
            "label": {"type": "string"},
            "packageName": {"type": "string"},
            "activityName": {"type": ["string", "null"]},
            "apkFile": {"type": ["string", "null"]},
            "description": {"type": "string"},
        },
    )
    quest_device_profile = obj(
        "QuestDeviceProfile",
        {
            "id": {"type": "string"},
            "label": {"type": "string"},
            "properties": array(obj("QuestDeviceProperty", {"key": {"type": "string"}, "value": {"type": "string"}})),
            "description": {"type": "string"},
        },
    )
    quest_runtime_profile = obj(
        "QuestRuntimeProfile",
        {
            "id": {"type": "string"},
            "label": {"type": "string"},
            "values": {"type": "object", "additionalProperties": {"type": "string"}},
            "description": {"type": "string"},
        },
    )
    quest_tool_provider_kind = enum(
        "QuestToolProviderKind",
        ["Adb", "HzdbCli", "HzdbMcp", "RustyXrCompanion", "BrokerShellHelper", "Manual", "Other"],
    )
    provider_operation_safety = enum(
        "ProviderOperationSafety",
        [
            "ReadOnly",
            "BoundedCapture",
            "FileRead",
            "FileWrite",
            "FileDelete",
            "AppLifecycle",
            "DeviceSetting",
            "ShellCommand",
            "NetworkForward",
            "Root",
            "Unknown",
        ],
    )
    quest_device_readiness = enum(
        "DeviceReadiness",
        ["Unknown", "Disconnected", "PowerOnly", "SystemDialog", "RuntimeReady", "AppVisible"],
    )
    provider_capability = obj(
        "ProviderCapability",
        {
            "provider": quest_tool_provider_kind,
            "capability_id": string(),
            "command_group": string(),
            "description": string(),
            "safety": provider_operation_safety,
            "requires_device": boolean(),
            "requires_network": boolean(),
        },
    )
    quest_device_health = obj(
        "DeviceHealth",
        {
            "provider": quest_tool_provider_kind,
            "connected": boolean(),
            "readiness": quest_device_readiness,
            "battery_level_percent": {"type": ["integer", "null"], "minimum": 0, "maximum": 100},
            "storage_available_bytes": {"type": ["integer", "null"], "minimum": 0},
            "controller_count": {"type": "integer", "minimum": 0, "maximum": 255},
            "ui_ready": boolean(),
            "issues": array(string()),
        },
    )
    foreground_app = obj(
        "ForegroundApp",
        {
            "package_name": nullable_string(),
            "activity_name": nullable_string(),
            "process_id": {"type": ["integer", "null"], "minimum": 0},
            "source": string(),
        },
    )
    mcp_transport = enum("McpTransport", ["Stdio", "Sse", "StreamableHttp"])
    mcp_server_config = obj(
        "McpServerConfig",
        {
            "server_name": string(),
            "command": string(),
            "args": array(string()),
            "transport": mcp_transport,
            "provider": quest_tool_provider_kind,
            "project_local": boolean(),
        },
    )
    quest_development_provider_snapshot = obj(
        "QuestDevelopmentProviderSnapshot",
        {
            "provider": quest_tool_provider_kind,
            "version": nullable_string(),
            "capabilities": array(provider_capability),
            "device_health": {"oneOf": [quest_device_health, {"type": "null"}]},
            "foreground_app": {"oneOf": [foreground_app, {"type": "null"}]},
            "mcp": {"oneOf": [mcp_server_config, {"type": "null"}]},
            "notes": array(string()),
        },
    )
    broker_transport_kind = enum(
        "BrokerTransportKind",
        [
            "WebSocket",
            "Tcp",
            "Udp",
            "AdbForwardedTcp",
            "Quic",
            "WebTransport",
            "WebRtcDiagnostic",
            "ExternalSidecar",
            "MetadataOnly",
        ],
    )
    broker_reliability_class = enum(
        "BrokerReliabilityClass",
        ["Reliable", "LossTolerant", "BestEffort", "MetadataOnly"],
    )
    broker_payload_kind = enum(
        "BrokerPayloadKind",
        ["Json", "Text", "Binary", "H264", "H265", "RawLuma8", "Custom"],
    )
    broker_stream_kind = enum(
        "BrokerStreamKind",
        ["Media", "Audio", "Telemetry", "Control", "XrInput", "Bio", "Synthetic", "Custom"],
    )
    broker_codec_id = enum(
        "BrokerCodecId",
        ["H264", "H265", "Av1", "RawLuma8", "RawRgba8", "Opus", "PcmF32", "Json", "Custom"],
    )
    broker_stream_direction = enum(
        "BrokerStreamDirection",
        ["ProducerToConsumer", "ConsumerToProducer", "Bidirectional", "MetadataOnly"],
    )
    broker_security_mode = enum(
        "BrokerSecurityMode",
        ["LoopbackOnly", "PairingToken", "PreSharedKey", "ExternalSidecarOwned"],
    )
    broker_transport_session_state = enum(
        "BrokerTransportSessionState",
        ["Created", "Offered", "Accepted", "Starting", "Streaming", "Draining", "Closed", "Failed"],
    )
    broker_packet_drop_reason = enum(
        "BrokerPacketDropReason",
        [
            "LatePacket",
            "DecodeTimeout",
            "MissingKeyframe",
            "SurfaceUnavailable",
            "HardwareBufferImportFailed",
            "ProjectionMetadataMissing",
            "XrFrameBudgetExceeded",
            "QueueOverflow",
            "ClientShutdown",
            "Unknown",
        ],
    )
    broker_timestamp_domain = enum(
        "BrokerTimestampDomain",
        [
            "ElapsedRealtime",
            "CameraSensor",
            "MediaPts",
            "Unix",
            "OpenXrPredictedDisplay",
            "RelayReceive",
            "Unknown",
        ],
    )
    broker_clock_health_state = enum(
        "BrokerClockHealthState",
        ["Healthy", "Degraded", "Unavailable"],
    )
    broker_clock_correlation_quality = enum(
        "BrokerClockCorrelationQuality",
        ["High", "Medium", "Low", "Unavailable"],
    )
    broker_clock_discontinuity_reason = enum(
        "BrokerClockDiscontinuityReason",
        ["None", "ServiceRestart", "WallClockJump", "SleepResume", "RuntimeLoss", "SampleGap", "Unknown"],
    )
    broker_camera_api_path = enum(
        "BrokerCameraApiPath",
        [
            "AndroidCamera2",
            "AndroidNdkCamera2",
            "MetaPassthroughCameraApi",
            "OpenXrPassthrough",
            "Synthetic",
            "Unknown",
        ],
    )
    broker_camera_permission_state = enum(
        "BrokerCameraPermissionState",
        ["Granted", "Denied", "Unavailable", "NotRequired", "Unknown"],
    )
    broker_drop_counters = obj(
        "BrokerDropCounters",
        {
            "received_samples": integer(0),
            "emitted_samples": integer(0),
            "dropped_samples": integer(0),
            "late_samples": integer(0),
            "duplicate_samples": integer(0),
            "out_of_order_samples": integer(0),
            "queue_overflow_count": integer(0),
        },
    )
    broker_transport_endpoint = obj(
        "BrokerTransportEndpoint",
        {
            "transport": broker_transport_kind,
            "host": nullable_string(),
            "port": {"type": ["integer", "null"], "minimum": 1, "maximum": 65535},
            "path": nullable_string(),
            "channel_id": nullable_string(),
            "max_datagram_bytes": {"type": ["integer", "null"], "minimum": 1, "maximum": 65507},
            "auth_required": boolean(),
        },
    )
    broker_transport_security_policy = obj(
        "BrokerTransportSecurityPolicy",
        {
            "schema": string(),
            "mode": broker_security_mode,
            "non_loopback_allowed": boolean(),
            "pairing_token_required": boolean(),
            "expires_elapsed_ns": {"type": ["integer", "null"], "minimum": 0},
            "capability_scope": array(string()),
        },
    )
    broker_transport_stream_descriptor = obj(
        "BrokerTransportStreamDescriptor",
        {
            "stream_id": string(),
            "stream_kind": broker_stream_kind,
            "direction": broker_stream_direction,
            "payload_kind": broker_payload_kind,
            "payload_schema": string(),
            "codec": {"oneOf": [broker_codec_id, {"type": "null"}]},
            "reliability": broker_reliability_class,
            "ordered": boolean(),
            "nominal_rate_hz": {"type": ["number", "null"], "exclusiveMinimum": 0},
            "target_latency_ms": {"type": ["number", "null"], "minimum": 0},
            "max_payload_bytes": {"type": ["integer", "null"], "minimum": 1},
        },
    )
    broker_transport_session_offer = obj(
        "BrokerTransportSessionOffer",
        {
            "schema": string(),
            "session_id": string(),
            "client_id": string(),
            "requested_transports": array(broker_transport_kind),
            "streams": array(broker_transport_stream_descriptor),
            "security": broker_transport_security_policy,
            "target_latency_ms": {"type": ["number", "null"], "minimum": 0},
        },
    )
    broker_transport_session_answer = obj(
        "BrokerTransportSessionAnswer",
        {
            "schema": string(),
            "session_id": string(),
            "accepted": boolean(),
            "state": broker_transport_session_state,
            "selected_transport": {"oneOf": [broker_transport_kind, {"type": "null"}]},
            "accepted_streams": array(broker_transport_stream_descriptor),
            "security": broker_transport_security_policy,
            "reason": nullable_string(),
        },
    )
    broker_media_sample_timing = obj(
        "BrokerMediaSampleTiming",
        {
            "schema": string(),
            "session_id": string(),
            "stream_id": string(),
            "sequence_number": integer(0),
            "source_capture_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "encode_start_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "encode_done_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "packet_send_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "packet_receive_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "decode_start_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "decode_done_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "texture_import_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "xr_submit_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "present_estimate_time_ns": {"type": ["integer", "null"], "minimum": 0},
        },
    )
    broker_network_quality_sample = obj(
        "BrokerNetworkQualitySample",
        {
            "schema": string(),
            "session_id": string(),
            "stream_id": nullable_string(),
            "measured_time_elapsed_ns": integer(0),
            "packet_loss_estimate01": {"type": ["number", "null"], "minimum": 0, "maximum": 1},
            "late_packet_count": integer(0),
            "decode_gap_count": integer(0),
            "jitter_buffer_depth": integer(0),
            "target_latency_ms": {"type": ["number", "null"], "minimum": 0},
            "actual_latency_ms": {"type": ["number", "null"], "minimum": 0},
            "clock_sync_quality01": {"type": ["number", "null"], "minimum": 0, "maximum": 1},
        },
    )
    broker_packet_descriptor = obj(
        "BrokerPacketDescriptor",
        {
            "schema": string(),
            "session_id": string(),
            "stream_id": string(),
            "sequence_number": integer(0),
            "payload_kind": broker_payload_kind,
            "payload_byte_len": integer(1),
            "key_frame": boolean(),
            "drop_reason": {"oneOf": [broker_packet_drop_reason, {"type": "null"}]},
        },
    )
    broker_video_size = obj(
        "BrokerVideoSize",
        {
            "width": integer(1),
            "height": integer(1),
        },
    )
    broker_fps_range = obj(
        "BrokerFpsRange",
        {
            "min_hz": integer(1),
            "max_hz": integer(1),
        },
    )
    broker_camera_source_capabilities = obj(
        "BrokerCameraSourceCapabilities",
        {
            "schema": {"const": "rusty.xr.broker.camera_source_capabilities.v1"},
            "source_id": string(),
            "source_api_path": broker_camera_api_path,
            "horizon_os_version_observed": nullable_string(),
            "camera_permission_state": broker_camera_permission_state,
            "headset_camera_permission_state": broker_camera_permission_state,
            "camera_id": nullable_string(),
            "physical_camera_ids": array(string()),
            "meta_vendor_camera_source": nullable_string(),
            "meta_vendor_position": nullable_string(),
            "supported_private_sizes": array(broker_video_size),
            "supported_yuv_sizes": array(broker_video_size),
            "supported_fps_ranges": array(broker_fps_range),
            "selected_size": {"oneOf": [broker_video_size, {"type": "null"}]},
            "selected_fps_range": {"oneOf": [broker_fps_range, {"type": "null"}]},
            "stream_min_frame_duration_ns": {"type": ["integer", "null"], "minimum": 1},
            "timestamp_domain": broker_timestamp_domain,
            "selected_reason": nullable_string(),
        },
    )
    broker_clock_stamp = obj(
        "BrokerClockStamp",
        {
            "schema": {"const": "rusty.xr.clock.stamp.v1"},
            "clock_id": string(),
            "clock_epoch_id": string(),
            "canonical_domain": broker_timestamp_domain,
            "event_elapsed_realtime_ns": integer(0),
            "event_unix_ns": {"type": ["integer", "null"], "minimum": 0},
            "source_domain": {"oneOf": [broker_timestamp_domain, {"type": "null"}]},
            "source_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "correlation_id": nullable_string(),
            "uncertainty_ns": integer(0),
            "sequence_number": integer(0),
        },
    )
    broker_clock_snapshot = obj(
        "BrokerClockSnapshot",
        {
            "schema": {"const": "rusty.xr.clock.snapshot.v1"},
            "clock_id": string(),
            "clock_epoch_id": string(),
            "sequence_number": integer(0),
            "canonical_domain": broker_timestamp_domain,
            "android_elapsed_realtime_ns": integer(0),
            "android_realtime_unix_ns": {"type": ["integer", "null"], "minimum": 0},
            "read_uncertainty_ns": integer(0),
            "wall_clock_adjustment_counter": integer(0),
            "health": broker_clock_health_state,
        },
    )
    broker_clock_correlation = obj(
        "BrokerClockCorrelation",
        {
            "schema": {"const": "rusty.xr.clock.correlation.v1"},
            "correlation_id": string(),
            "source_domain": broker_timestamp_domain,
            "target_domain": broker_timestamp_domain,
            "sample_count": integer(0),
            "window_start_elapsed_ns": integer(0),
            "window_end_elapsed_ns": integer(0),
            "offset_ns": {"type": "integer"},
            "drift_ppm": number(),
            "rms_error_ns": integer(0),
            "max_error_ns": integer(0),
            "p95_error_ns": integer(0),
            "uncertainty_ns": integer(0),
            "quality": broker_clock_correlation_quality,
            "last_discontinuity_reason": broker_clock_discontinuity_reason,
        },
    )
    broker_clock_health = obj(
        "BrokerClockHealth",
        {
            "schema": {"const": "rusty.xr.clock.health.v1"},
            "clock_id": string(),
            "clock_epoch_id": string(),
            "health": broker_clock_health_state,
            "wall_clock_adjustment_counter": integer(0),
            "last_snapshot": broker_clock_snapshot,
            "active_correlations": array(broker_clock_correlation),
        },
    )
    broker_clock_sync_probe = obj(
        "BrokerClockSyncProbe",
        {
            "schema": {"const": "rusty.xr.clock.sync_probe.v1"},
            "probe_id": string(),
            "sequence_number": integer(0),
            "host_send_unix_ns": integer(0),
            "target_receive_elapsed_ns": integer(0),
            "target_receive_unix_ns": integer(0),
            "target_send_elapsed_ns": integer(0),
            "target_send_unix_ns": integer(0),
            "host_receive_unix_ns": {"type": ["integer", "null"], "minimum": 0},
        },
    )
    broker_h264_stream_invariants = obj(
        "BrokerH264StreamInvariants",
        {
            "schema": {"const": "rusty.xr.broker.h264_stream_invariants.v1"},
            "session_id": string(),
            "stream_id": string(),
            "role": string(),
            "direction": broker_stream_direction,
            "peer_id": nullable_string(),
            "track_id": nullable_string(),
            "eye": nullable_string(),
            "bitstream_format": string(),
            "encoder_name": nullable_string(),
            "decoder_name": nullable_string(),
            "width": integer(1),
            "height": integer(1),
            "bitrate_bps": {"type": ["integer", "null"], "minimum": 1},
            "bitrate_mode_requested": nullable_string(),
            "bitrate_mode_applied": nullable_string(),
            "i_frame_interval_seconds": {"type": ["integer", "null"], "minimum": 0},
            "encoder_latency_requested_frames": {"type": ["integer", "null"], "minimum": 0},
            "encoder_latency_applied_frames": {"type": ["integer", "null"], "minimum": 0},
            "decoder_low_latency_config_requested": {"type": ["boolean", "null"]},
            "decoder_low_latency_parameter_succeeded": {"type": ["boolean", "null"]},
            "codec_config_packet_count": integer(0),
            "sps_present": boolean(),
            "pps_present": boolean(),
            "keyframe_count": integer(0),
            "sync_frame_request_count": integer(0),
            "sync_frame_request_on_start_succeeded": {"type": ["boolean", "null"]},
            "decoder_output_mode": nullable_string(),
            "hardware_buffer_import_succeeded": {"type": ["boolean", "null"]},
            "close_reason": nullable_string(),
        },
    )
    broker_heartbeat = obj(
        "BrokerHeartbeatState",
        {
            "last_heartbeat_elapsed_ns": {"type": ["integer", "null"], "minimum": 0},
            "timeout_after_ns": integer(0),
        },
    )
    broker_stream_manifest = obj(
        "BrokerStreamManifest",
        {
            "manifest_schema": string(),
            "stream_id": string(),
            "session_id": nullable_string(),
            "source_id": string(),
            "payload_kind": broker_payload_kind,
            "payload_schema": string(),
            "sequence_start": integer(0),
            "recommended_rate_hz": {"type": ["number", "null"], "exclusiveMinimum": 0},
            "max_datagram_bytes": {"type": ["integer", "null"], "minimum": 1, "maximum": 65507},
            "reliability": broker_reliability_class,
            "ordered": boolean(),
            "endpoint": {"oneOf": [broker_transport_endpoint, {"type": "null"}]},
            "heartbeat": {"oneOf": [broker_heartbeat, {"type": "null"}]},
            "drop_counters": broker_drop_counters,
        },
    )
    broker_sample_header = obj(
        "BrokerStreamSampleHeader",
        {
            "schema": string(),
            "stream_id": string(),
            "session_id": nullable_string(),
            "source_id": string(),
            "payload_kind": broker_payload_kind,
            "payload_schema": string(),
            "sequence_number": integer(0),
            "broker_time_elapsed_ns": integer(0),
            "broker_time_unix_ns": {"type": ["integer", "null"], "minimum": 0},
            "source_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "source_time_unix_ns": {"type": ["integer", "null"], "minimum": 0},
            "dropped_before_sample": integer(0),
            "late_before_sample": integer(0),
        },
    )
    broker_session_metadata = obj(
        "BrokerSessionMetadata",
        {
            "key": string(),
            "value": string(),
        },
    )
    broker_replay_record = obj(
        "BrokerReplayRecord",
        {
            "type": {"const": "replay_record"},
            "schema": string(),
            "session_id": string(),
            "stream": string(),
            "header": broker_sample_header,
            "payload": {},
        },
    )
    synthetic_wave_sample = obj(
        "SyntheticWaveSample",
        {
            "sequence_number": integer(0),
            "sample_time_elapsed_ns": integer(0),
            "value01": {"type": "number", "minimum": 0, "maximum": 1},
            "phase01": {"type": "number", "minimum": 0, "maximum": 1},
            "valid": boolean(),
        },
    )
    eye_coordinate_space = enum(
        "EyeCoordinateSpace",
        ["ScreenNormalized", "ScreenPixels", "XrLocal", "XrWorld", "SceneObject"],
    )
    eye_identity = enum("EyeIdentity", ["Left", "Right", "Combined"])
    eye_derived_kind = enum("EyeDerivedKind", ["Fixation", "Dwell", "Blink"])
    eye_validity_flags = obj(
        "EyeValidityFlags",
        {
            "sample_valid": boolean(),
            "left_valid": boolean(),
            "right_valid": boolean(),
            "blink": boolean(),
            "tracking_lost": boolean(),
        },
    )
    eye_sample_base = obj(
        "EyeSampleBase",
        {
            "provider_id": string(),
            "source_device_id": string(),
            "sequence_number": integer(0),
            "sample_time_ns": integer(0),
            "broker_receive_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "validity": eye_validity_flags,
            "confidence": {"type": ["number", "null"], "minimum": 0, "maximum": 1},
            "eye": {"oneOf": [eye_identity, {"type": "null"}]},
            "coordinate_space": eye_coordinate_space,
        },
    )
    eye_derived_provenance = obj(
        "EyeDerivedProvenance",
        {
            "source_stream_id": string(),
            "processor_id": string(),
            "source_sequence_start": integer(0),
            "source_sequence_end": integer(0),
        },
    )
    eye_scene_hit = obj(
        "EyeSceneHit",
        {
            "target_id": string(),
            "position_m": vec3(),
            "normal": {"oneOf": [vec3(), {"type": "null"}]},
            "distance_m": {"type": ["number", "null"], "minimum": 0},
            "derived_from": eye_derived_provenance,
        },
    )
    eye_screen_gaze_point = obj(
        "EyeScreenGazePoint",
        {
            "schema": string(),
            "base": eye_sample_base,
            "display_id": nullable_string(),
            "normalized_point": vec2(),
            "screen_pixel": {"oneOf": [vec2(), {"type": "null"}]},
            "pupil_diameter_mm": {"type": ["number", "null"], "exclusiveMinimum": 0},
        },
    )
    eye_xr_gaze_ray = obj(
        "EyeXrGazeRay",
        {
            "schema": string(),
            "base": eye_sample_base,
            "origin_m": vec3(),
            "direction": vec3(),
            "scene_hit": {"oneOf": [eye_scene_hit, {"type": "null"}]},
        },
    )
    eye_screen_aoi_hit = obj(
        "EyeScreenAoiHit",
        {
            "schema": string(),
            "base": eye_sample_base,
            "aoi_id": string(),
            "hit": boolean(),
            "dwell_time_ns": {"type": ["integer", "null"], "minimum": 0},
            "derived_from": eye_derived_provenance,
        },
    )
    eye_processor_event = obj(
        "EyeProcessorEvent",
        {
            "schema": string(),
            "kind": eye_derived_kind,
            "base": eye_sample_base,
            "duration_ns": {"type": ["integer", "null"], "minimum": 0},
            "derived_from": eye_derived_provenance,
        },
    )
    home_mode = enum(
        "HomeMode",
        ["Normal2d", "ImmersivePassthrough", "ImmersiveVirtual", "DeveloperSupervisor", "ManagedKiosk"],
    )
    home_panel_kind = enum(
        "HomePanelKind",
        [
            "BrokerPage",
            "LocalApplet",
            "WebApplet",
            "CooperatingApp",
            "RemoteSurface",
            "SettingsShortcut",
            "Diagnostic",
        ],
    )
    home_panel_placement = enum(
        "HomePanelPlacement",
        ["Flat2d", "HeadLocked", "WorldLocked", "HandAnchored", "Desk"],
    )
    home_panel_descriptor = obj(
        "HomePanelDescriptor",
        {
            "schema": {"const": "rusty.xr.home.panel.v1"},
            "panel_id": string(),
            "title": string(),
            "kind": home_panel_kind,
            "default_size_m": vec2(),
            "min_size_m": vec2(),
            "max_size_m": vec2(),
            "placement": home_panel_placement,
            "requires_helper": boolean(),
            "commands": array(string()),
        },
    )
    home_launcher_entry_source = enum(
        "LauncherEntrySource",
        ["PackageManager", "Catalog", "Manual", "HelperObserved"],
    )
    home_launcher_entry = obj(
        "LauncherEntry",
        {
            "schema": {"const": "rusty.xr.home.launcher_entry.v1"},
            "package_name": string(),
            "label": string(),
            "launch_component": nullable_string(),
            "source": home_launcher_entry_source,
            "requires_helper": boolean(),
            "profile_id": nullable_string(),
            "warnings": array(string()),
        },
    )
    home_settings_shortcut_category = enum(
        "SettingsShortcutCategory",
        ["Network", "Bluetooth", "Display", "Apps", "Cast", "Developer", "Privacy", "Boundary", "Other"],
    )
    home_settings_shortcut = obj(
        "SettingsShortcutDescriptor",
        {
            "schema": {"const": "rusty.xr.home.settings_shortcut.v1"},
            "shortcut_id": string(),
            "label": string(),
            "android_action": string(),
            "category": home_settings_shortcut_category,
            "requires_confirmation": boolean(),
            "requires_helper": boolean(),
            "warning": nullable_string(),
        },
    )
    home_helper_state = obj(
        "HomeHelperState",
        {
            "connected": boolean(),
            "uid_label": nullable_string(),
            "capabilities": array(string()),
            "last_heartbeat_elapsed_ns": {"type": ["integer", "null"], "minimum": 0},
        },
    )
    home_supervisor_policy = enum(
        "HomeSupervisorPolicy",
        [
            "Disabled",
            "ObserveOnly",
            "ReturnToBrokerAfterLimbo",
            "ReturnToTargetAfterHome",
            "GuardedDemoSession",
            "ManagedDevicePolicy",
        ],
    )
    home_supervisor_state = obj(
        "HomeSupervisorState",
        {
            "enabled": boolean(),
            "policy": home_supervisor_policy,
            "max_attempts": integer(0),
            "cooldown_ms": integer(0),
            "attempt_count": integer(0),
            "last_event_id": nullable_string(),
        },
    )
    home_external_launch_state = obj(
        "ExternalLaunchState",
        {
            "package_name": string(),
            "launch_mode": string(),
            "requested_at_unix_ms": {"type": ["integer", "null"], "minimum": 0},
            "observed_foreground": nullable_string(),
        },
    )
    home_session_state = obj(
        "HomeSessionState",
        {
            "schema": {"const": "rusty.xr.home.state.v1"},
            "mode": home_mode,
            "active_panels": array(string()),
            "last_external_launch": {"oneOf": [home_external_launch_state, {"type": "null"}]},
            "helper": home_helper_state,
            "supervisor": home_supervisor_state,
        },
    )
    kiosk_control_plane_phase = enum(
        "KioskControlPlanePhase",
        [
            "BrokerPanel2d",
            "BrokerPanelWithShellHelper",
            "ImmersiveHomePrototype",
            "ImmersiveHomeWithSupervisor",
            "ManagedDeviceKiosk",
        ],
    )
    kiosk_surface_intent = enum(
        "KioskSurfaceIntent",
        [
            "RustyKioskDefault",
            "RustyXrTarget",
            "MetaPanelIntentional",
            "MetaPanelUnexpected",
            "UnknownSurface",
        ],
    )
    kiosk_command_provider = enum(
        "KioskCommandProvider",
        ["Broker", "ShellHelper", "Adb", "HzdbCli", "HzdbMcp", "Companion", "Manual", "Unknown"],
    )
    kiosk_command_evidence = obj(
        "KioskCommandEvidence",
        {
            "schema": {"const": "rusty.xr.kiosk.command_evidence.v1"},
            "command_goal": string(),
            "provider": kiosk_command_provider,
            "preferred_command": nullable_string(),
            "fallback_command": nullable_string(),
            "foreground_before": nullable_string(),
            "foreground_after": nullable_string(),
            "clock_epoch_id": nullable_string(),
            "notes": array(string()),
        },
    )
    kiosk_control_plane_status = obj(
        "KioskControlPlaneStatus",
        {
            "schema": {"const": "rusty.xr.kiosk.control_plane.v1"},
            "phase": kiosk_control_plane_phase,
            "surface_intent": kiosk_surface_intent,
            "home_mode": home_mode,
            "broker_available": boolean(),
            "broker_panel_visible": boolean(),
            "immersive_home_visible": boolean(),
            "shell_helper_connected": boolean(),
            "continuous_adb_shell_required": boolean(),
            "watchdog_required": boolean(),
            "focus_guardian_active": boolean(),
            "proximity_watchdog_active": boolean(),
            "meta_menu_active": boolean(),
            "meta_menu_entry_intentional": boolean(),
            "active_panel": nullable_string(),
            "foreground_package": nullable_string(),
            "foreground_activity": nullable_string(),
            "clock_epoch_id": nullable_string(),
            "latest_command": {"oneOf": [kiosk_command_evidence, {"type": "null"}]},
            "limitations": array(string()),
        },
    )
    home_focus_recovery_action = enum(
        "FocusRecoveryAction",
        ["Observe", "ReturnToBroker", "ReturnToTarget", "OpenSystemPanel", "StopSupervisor"],
    )
    home_focus_recovery_result = enum(
        "FocusRecoveryResult",
        [
            "NotAttempted",
            "Started",
            "Succeeded",
            "Failed",
            "SkippedProtectedPrompt",
            "CooldownActive",
            "MaxAttemptsReached",
        ],
    )
    home_focus_recovery_event = obj(
        "FocusRecoveryEvent",
        {
            "schema": {"const": "rusty.xr.home.focus_recovery_event.v1"},
            "event_id": string(),
            "policy": home_supervisor_policy,
            "action": home_focus_recovery_action,
            "result": home_focus_recovery_result,
            "reason": string(),
            "previous_foreground": nullable_string(),
            "requested_target": nullable_string(),
            "attempt_count": integer(0),
            "event_time_unix_ms": {"type": ["integer", "null"], "minimum": 0},
        },
    )
    effect_pass_kind = enum(
        "EffectPassKind",
        [
            "Source",
            "LumaTransform",
            "Blur",
            "ColorMap",
            "EdgeDetection",
            "ScalarMap",
            "Displacement",
            "Composite",
            "DiagnosticTap",
        ],
    )
    effect_pass_input_role = enum(
        "EffectPassInputRole",
        ["SourceColor", "SourceLuma", "Guide", "Mask", "DisplacementMap", "PreviousPass"],
    )
    effect_buffer_format = enum(
        "EffectBufferFormat",
        ["Rgba8", "Rgba16Float", "Rgba32Float", "R8", "R16Float", "R32Float", "ExternalGpu"],
    )
    stereo_media_layout = {
        "oneOf": [
            {"const": "Mono"},
            obj("StereoMediaLayoutSideBySide", {"SideBySide": obj("SideBySideFields", {"left_first": boolean()})}),
            obj("StereoMediaLayoutTopBottom", {"TopBottom": obj("TopBottomFields", {"left_first": boolean()})}),
            {"const": "Separate"},
        ]
    }
    effect_buffer_descriptor = obj(
        "EffectBufferDescriptor",
        {
            "buffer_id": string(),
            "size": image_size(),
            "format": effect_buffer_format,
            "stereo_layout": stereo_media_layout,
            "persistent": boolean(),
        },
    )
    effect_pass_input = obj(
        "EffectPassInput",
        {
            "input_id": string(),
            "role": effect_pass_input_role,
        },
    )
    effect_pass_descriptor = obj(
        "EffectPassDescriptor",
        {
            "pass_id": string(),
            "kind": effect_pass_kind,
            "inputs": array(effect_pass_input),
            "output_buffer": nullable_string(),
            "enabled_by_default": boolean(),
            "offscreen": boolean(),
            "separable": boolean(),
            "diagnostic_label": nullable_string(),
            "parameter_keys": array(string()),
        },
    )
    effect_diagnostic_layer = obj(
        "EffectDiagnosticLayer",
        {
            "layer_id": string(),
            "label": string(),
            "pass_id": nullable_string(),
            "buffer_id": nullable_string(),
            "expected_role": effect_pass_input_role,
        },
    )
    effect_stack_descriptor = obj(
        "EffectStackDescriptor",
        {
            "schema": {"const": "rusty.xr.effect_stack.descriptor.v1"},
            "stack_id": string(),
            "source_size": image_size(),
            "source_layout": stereo_media_layout,
            "buffers": array(effect_buffer_descriptor),
            "passes": array(effect_pass_descriptor),
            "diagnostic_layers": array(effect_diagnostic_layer),
        },
    )
    effect_layer_metrics = obj(
        "EffectLayerMetrics",
        {
            "active_pixel_fraction": {"type": "number", "minimum": 0, "maximum": 1},
            "luma_mean": {"type": "number", "minimum": 0, "maximum": 1},
            "luma_std": {"type": "number", "minimum": 0},
            "edge_energy": {"type": "number", "minimum": 0},
            "high_frequency_energy": {"type": "number", "minimum": 0},
        },
    )
    effect_layer_comparison_metrics = obj(
        "EffectLayerComparisonMetrics",
        {
            "luma_rmse": {"type": "number", "minimum": 0},
            "luma_bias": number(),
            "luma_correlation": {"type": "number", "minimum": -1, "maximum": 1},
            "edge_ratio_candidate_over_reference": {"type": "number", "minimum": 0},
            "high_frequency_ratio_candidate_over_reference": {"type": "number", "minimum": 0},
        },
    )
    effect_layer_comparison = obj(
        "EffectLayerComparison",
        {
            "layer_id": string(),
            "reference": {"oneOf": [effect_layer_metrics, {"type": "null"}]},
            "candidate": {"oneOf": [effect_layer_metrics, {"type": "null"}]},
            "pair": {"oneOf": [effect_layer_comparison_metrics, {"type": "null"}]},
            "note": nullable_string(),
        },
    )
    effect_stack_comparison_report = obj(
        "EffectStackComparisonReport",
        {
            "schema": {"const": "rusty.xr.effect_stack.comparison_report.v1"},
            "report_id": string(),
            "stack_id": string(),
            "reference_label": string(),
            "candidate_label": string(),
            "layers": array(effect_layer_comparison),
        },
    )
    return {
        "runtime-config.schema.json": obj(
            "RuntimeConfig",
            {
                "settings": {
                    "type": "object",
                    "additionalProperties": obj(
                        "RuntimeSetting",
                        {
                            "key": {"type": "string", "pattern": "^[a-z0-9]+([_-][a-z0-9]+)*$"},
                            "value": runtime_value,
                            "source": enum(
                                "RuntimeConfigSource",
                                ["Default", "Environment", "AndroidProperty", "File", "CommandLine", "Synthetic"],
                            ),
                        },
                    ),
                }
            },
        ),
        "telemetry-sample.schema.json": obj(
            "TelemetrySample",
            {
                "timestamp_ns": integer(0),
                "labels": {"type": "array", "items": {"type": "string"}},
                "values": {"type": "array", "items": number()},
            },
        ),
        "lsl-stream-descriptor.schema.json": lsl_descriptor,
        "camera-frame-metadata.schema.json": obj(
            "CameraFrameMetadata",
            {
                "source": obj(
                    "CameraSourceId",
                    {"label": {"type": "string"}, "physical_id": {"type": ["string", "null"]}},
                ),
                "frame_index": integer(0),
                "timestamp_ns": {"type": ["integer", "null"], "minimum": 0},
                "intrinsics": obj(
                    "CameraIntrinsics",
                    {
                        "focal_length_px": vec2(),
                        "principal_point_px": vec2(),
                        "image_size": image_size(),
                    },
                ),
                "extrinsics": {"type": ["object", "null"]},
            },
        ),
        "depth-frame-summary.schema.json": obj(
            "DepthFrameSummary",
            {
                "frame_index": integer(0),
                "size": image_size(),
                "format": enum("DepthFormat", ["Float32Meters", "Uint16Millimeters", "Uint16Raw"]),
                "meter_scale": number(),
                "runtime_capture_time_ns": {"type": ["integer", "null"]},
                "depth_range": {
                    "type": ["object", "null"],
                    "properties": {
                        "near_z_m": number(),
                        "far_z_m": number(),
                    },
                    "required": ["near_z_m", "far_z_m"],
                    "additionalProperties": False,
                },
                "layer_index": {"type": ["integer", "null"], "minimum": 0},
                "layer_count": integer(1),
                "has_confidence": {"type": "boolean"},
                "confidence_source": enum(
                    "DepthConfidenceSource",
                    ["None", "RuntimePayload", "AppDerived", "Unknown"],
                ),
                "byte_len": integer(0),
            },
        ),
        "environment-depth-diagnostics-summary.schema.json": obj(
            "EnvironmentDepthDiagnosticsSummary",
            {
                "xr_frame_count": integer(0),
                "acquire_attempts": integer(0),
                "acquired_frames": integer(0),
                "unavailable_frames": integer(0),
                "acquire_errors": integer(0),
                "repeated_capture_time_count": integer(0),
                "observed_acquire_hz": number(),
                "observed_depth_hz": number(),
                "average_acquire_cpu_ms": number(),
                "latest_frame": {"type": ["object", "null"]},
                "confidence_source": enum(
                    "DepthConfidenceSource",
                    ["None", "RuntimePayload", "AppDerived", "Unknown"],
                ),
            },
        ),
        "plain-stereo-layer.schema.json": obj(
            "PlainStereoLayer",
            {
                "source_size": image_size(),
                "source_layout": {"type": "object"},
                "surface_size": vec2(),
                "content_mode": enum("StereoLayerContentMode", ["Fit", "Fill", "Stretch"]),
                "pose": {"type": "object"},
                "opacity": number(),
                "border": {"type": ["object", "null"]},
                "border_tuning": {"type": ["object", "null"]},
                "visual_feedback_tuning": {"type": ["object", "null"]},
                "performance_hints": {"type": "object"},
            },
        ),
        "quest-session-manifest.schema.json": obj(
            "QuestSessionManifest",
            {
                "schema_version": {"type": "string"},
                "device_label": {"type": "string"},
                "selected_package": {"type": "string"},
                "runtime_profile": {"type": ["string", "null"]},
                "device_profile": {"type": ["string", "null"]},
                "started_at_utc": {"type": "string"},
                "artifacts": {"type": "array", "items": {"type": "string"}},
            },
        ),
        "quest-app-catalog.schema.json": obj(
            "QuestAppCatalog",
            {
                "schemaVersion": {"type": "string"},
                "apps": array(quest_catalog_app),
                "deviceProfiles": array(quest_device_profile),
                "runtimeProfiles": array(quest_runtime_profile),
            },
        ),
        "quest-development-provider-snapshot.schema.json": quest_development_provider_snapshot,
        "capture-source-state.schema.json": obj(
            "CaptureSourceState",
            {
                "source_kind": capture_source_kind,
                "lifecycle": capture_lifecycle_state,
                "permission": capture_permission_state,
                "frame_count": integer(0),
                "dropped_frame_count": integer(0),
                "last_frame_time_ns": {"type": ["integer", "null"], "minimum": 0},
            },
        ),
        "room-mesh-source-state.schema.json": obj(
            "RoomMeshSourceState",
            {
                "source_kind": room_mesh_source_kind,
                "lifecycle": capture_lifecycle_state,
                "permission": capture_permission_state,
                "mesh_version": integer(0),
                "vertex_count": integer(0),
                "triangle_count": integer(0),
                "surface_count": integer(0),
                "last_update_time_ns": {"type": ["integer", "null"], "minimum": 0},
            },
        ),
        "room-mesh-snapshot.schema.json": obj(
            "RoomMeshSnapshot",
            {
                "version": integer(0),
                "source_kind": room_mesh_source_kind,
                "coordinate_space": enum("RoomMeshCoordinateSpace", ["Local", "Stage", "World"]),
                "root_pose": pose(),
                "captured_time_ns": {"type": ["integer", "null"], "minimum": 0},
                "vertices": array(vec3()),
                "indices": array(
                    {
                        "type": "array",
                        "items": integer(0),
                        "minItems": 3,
                        "maxItems": 3,
                    }
                ),
                "surfaces": array(room_mesh_surface),
            },
        ),
        "polar-acc-frame.schema.json": obj(
            "PolarAccFrame",
            {
                "sensor_timestamp_ns": integer(0),
                "samples_mg": {
                    "type": "array",
                    "items": obj(
                        "PolarAccSample",
                        {"x_mg": {"type": "integer"}, "y_mg": {"type": "integer"}, "z_mg": {"type": "integer"}},
                    ),
                },
            },
        ),
        "scan-surface-sample.schema.json": obj(
            "ScanSurfaceSample",
            {
                "coord": obj("VoxelCoord3", {"x": {"type": "integer"}, "y": {"type": "integer"}, "z": {"type": "integer"}}),
                "world_position": vec3(),
                "world_normal": vec3(),
                "confidence": integer(0),
                "signed_distance_meters": number(),
                "last_seen_time_ns": {"type": ["integer", "null"], "minimum": 0},
            },
        ),
        "broker-stream-manifest.schema.json": broker_stream_manifest,
        "broker-stream-sample-header.schema.json": broker_sample_header,
        "broker-transport-security-policy.schema.json": broker_transport_security_policy,
        "broker-transport-session-offer.schema.json": broker_transport_session_offer,
        "broker-transport-session-answer.schema.json": broker_transport_session_answer,
        "broker-media-sample-timing.schema.json": broker_media_sample_timing,
        "broker-network-quality-sample.schema.json": broker_network_quality_sample,
        "broker-packet-descriptor.schema.json": broker_packet_descriptor,
        "broker-camera-source-capabilities.schema.json": broker_camera_source_capabilities,
        "broker-clock-snapshot.schema.json": broker_clock_snapshot,
        "broker-clock-stamp.schema.json": broker_clock_stamp,
        "broker-clock-correlation.schema.json": broker_clock_correlation,
        "broker-clock-health.schema.json": broker_clock_health,
        "broker-clock-sync-probe.schema.json": broker_clock_sync_probe,
        "broker-h264-stream-invariants.schema.json": broker_h264_stream_invariants,
        "broker-session-manifest.schema.json": obj(
            "BrokerSessionManifest",
            {
                "schema": string(),
                "session_id": string(),
                "started_time_unix_ns": {"type": ["integer", "null"], "minimum": 0},
                "ended_time_unix_ns": {"type": ["integer", "null"], "minimum": 0},
                "streams": array(broker_stream_manifest),
                "metadata": array(broker_session_metadata),
            },
        ),
        "broker-command.schema.json": obj(
            "BrokerCommand",
            {
                "type": {"const": "command"},
                "schema": string(),
                "request_id": string(),
                "client_id": string(),
                "command": string(),
                "params": {"type": ["object", "null"], "additionalProperties": True},
            },
        ),
        "broker-command-ack.schema.json": obj(
            "BrokerCommandAck",
            {
                "type": {"const": "command_ack"},
                "schema": string(),
                "request_id": string(),
                "accepted": boolean(),
                "result": {"type": ["object", "null"], "additionalProperties": True},
                "error": nullable_string(),
            },
        ),
        "broker-stream-event.schema.json": obj(
            "BrokerStreamEvent",
            {
                "type": {"const": "stream_event"},
                "schema": string(),
                "stream": string(),
                "subscription_id": nullable_string(),
                "header": broker_sample_header,
                "payload": {},
            },
        ),
        "broker-replay-record.schema.json": broker_replay_record,
        "synthetic-wave-sample.schema.json": synthetic_wave_sample,
        "eye-screen-gaze-point.schema.json": eye_screen_gaze_point,
        "eye-xr-gaze-ray.schema.json": eye_xr_gaze_ray,
        "eye-screen-aoi-hit.schema.json": eye_screen_aoi_hit,
        "eye-processor-event.schema.json": eye_processor_event,
        "home-panel-descriptor.schema.json": home_panel_descriptor,
        "home-session-state.schema.json": home_session_state,
        "home-launcher-entry.schema.json": home_launcher_entry,
        "home-settings-shortcut.schema.json": home_settings_shortcut,
        "home-kiosk-control-plane-status.schema.json": kiosk_control_plane_status,
        "home-focus-recovery-event.schema.json": home_focus_recovery_event,
        "effect-stack-descriptor.schema.json": effect_stack_descriptor,
        "effect-stack-comparison-report.schema.json": effect_stack_comparison_report,
    }


def write_schemas(out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, schema in schemas().items():
        (out_dir / name).write_text(json.dumps(schema, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def check_schemas() -> None:
    encoded = json.dumps(schemas(), sort_keys=True)
    decoded = json.loads(encoded)
    if len(decoded) < 8:
        raise SystemExit("schema export produced too few schemas")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default="generated/schemas", help="Output directory.")
    parser.add_argument("--check", action="store_true", help="Validate schema generation without writing.")
    args = parser.parse_args()

    if args.check:
        check_schemas()
    else:
        write_schemas(Path(args.out))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
