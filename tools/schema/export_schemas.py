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


def object_map(value_schema: dict) -> dict:
    return {"type": "object", "additionalProperties": value_schema}


def loose_object() -> dict:
    return {"type": "object", "additionalProperties": True}


def open_obj(title: str, properties: dict, required: list[str] | None = None) -> dict:
    return {
        "$schema": SCHEMA_VERSION,
        "title": title,
        "type": "object",
        "additionalProperties": True,
        "properties": properties,
        "required": required or list(properties.keys()),
    }


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
    mesh_surface_topology_key = obj(
        "MeshSurfaceTopologyKey",
        {
            "vertex_count": integer(0),
            "triangle_count": integer(0),
            "index_hash": integer(0),
        },
    )
    mesh_fixture_kind = enum(
        "MeshFixtureKind",
        ["HandMesh", "SyntheticSurface", "Icosphere", "DeformingMesh", "Grid", "Other"],
    )
    mesh_fixture_coordinate_space = enum(
        "MeshFixtureCoordinateSpace",
        ["Local", "Stage", "World", "UnitSphere"],
    )
    mesh_fixture_units = enum("MeshFixtureUnits", ["Meters", "UnitRadius", "Unitless"])
    mesh_fixture_coordinate_convention = enum(
        "MeshFixtureCoordinateConvention",
        ["RightHandedYUpNegativeZForward", "UnitSphere"],
    )
    mesh_fixture_winding_order = enum(
        "MeshFixtureWindingOrder",
        ["Clockwise", "CounterClockwise", "MixedOrUnspecified"],
    )
    mesh_fixture_index_format = enum("MeshFixtureIndexFormat", ["U16", "U32", "Usize"])
    mesh_fixture_motion_kind = enum("MeshFixtureMotionKind", ["Static", "Animated", "Deforming"])
    mesh_fixture_neighbor_tier = obj(
        "MeshFixtureNeighborTier",
        {
            "tier": integer(1),
            "min_neighbor_count": integer(0),
            "max_neighbor_count": integer(0),
        },
    )
    mesh_fixture_frame_range = obj(
        "MeshFixtureFrameRange",
        {
            "min_frame_count": integer(1),
            "max_frame_count": integer(1),
        },
    )
    mesh_fixture_validation_expectation = enum(
        "MeshFixtureValidationExpectation",
        [
            "CountsMatchTopology",
            "IndicesInRange",
            "FiniteCoordinates",
            "NonDegenerateSurface",
            "StableTopologyHash",
            "NeighborTiersMatchSampleCount",
            "DeformationFrameRange",
        ],
    )
    mesh_fixture_intended_use = enum(
        "MeshFixtureIntendedUse",
        [
            "TopologyTests",
            "SamplingTests",
            "SdfDepthTests",
            "ParticleTests",
            "RenderPayloadTests",
            "ColliderTests",
        ],
    )
    mesh_fixture_provenance = enum("MeshFixtureProvenance", ["Synthetic", "Public", "Example", "Generated"])
    mesh_fixture_manifest = obj(
        "MeshFixtureManifest",
        {
            "schema": {"const": "rusty.xr.mesh_fixture_manifest.v1"},
            "fixture_id": string(),
            "fixture_kind": mesh_fixture_kind,
            "topology_key": mesh_surface_topology_key,
            "topology_hash": integer(0),
            "vertex_count": integer(0),
            "index_count": integer(0),
            "coordinate_sample_count": integer(0),
            "coordinate_space": mesh_fixture_coordinate_space,
            "coordinate_units": mesh_fixture_units,
            "coordinate_convention": mesh_fixture_coordinate_convention,
            "winding_order": mesh_fixture_winding_order,
            "index_format": mesh_fixture_index_format,
            "expected_neighbor_tiers": array(mesh_fixture_neighbor_tier),
            "motion": mesh_fixture_motion_kind,
            "allowed_deformation_frames": mesh_fixture_frame_range,
            "validation_expectations": array(mesh_fixture_validation_expectation),
            "intended_uses": array(mesh_fixture_intended_use),
            "provenance": mesh_fixture_provenance,
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
    frame_rate_summary = obj(
        "FrameRateSummary",
        {
            "sample_count": integer(0),
            "average_fps": number(),
            "min_fps": number(),
            "max_fps": number(),
        },
    )
    openxr_gles_feasibility_state = enum(
        "OpenXrGlesFeasibilityState",
        [
            "Unknown",
            "NotStarted",
            "ExtensionsEnumerated",
            "EglContextReady",
            "GraphicsRequirementsKnown",
            "SessionReady",
            "SwapchainsReady",
            "Rendering",
            "Failed",
        ],
    )
    gl_framebuffer_completeness = enum(
        "GlFramebufferCompleteness",
        [
            "Unknown",
            "Complete",
            "IncompleteAttachment",
            "IncompleteMissingAttachment",
            "IncompleteDimensions",
            "IncompleteUnsupported",
            "IncompleteMultisample",
            "IncompleteLayerTargets",
            "OtherIncomplete",
        ],
    )
    openxr_gles_extension_status = obj(
        "OpenXrGlesExtensionStatus",
        {
            "extension_name": string(),
            "required": boolean(),
            "available": boolean(),
        },
    )
    openxr_gles_graphics_requirements = obj(
        "OpenXrGlesGraphicsRequirements",
        {
            "min_api_version": nullable_string(),
            "max_api_version": nullable_string(),
        },
    )
    egl_gles_context_status = obj(
        "EglGlesContextStatus",
        {
            "egl_version": nullable_string(),
            "gles_version": nullable_string(),
            "glsl_version": nullable_string(),
            "vendor": nullable_string(),
            "renderer": nullable_string(),
            "config_red_bits": {"type": ["integer", "null"], "minimum": 0, "maximum": 255},
            "config_green_bits": {"type": ["integer", "null"], "minimum": 0, "maximum": 255},
            "config_blue_bits": {"type": ["integer", "null"], "minimum": 0, "maximum": 255},
            "config_alpha_bits": {"type": ["integer", "null"], "minimum": 0, "maximum": 255},
            "config_depth_bits": {"type": ["integer", "null"], "minimum": 0, "maximum": 255},
            "config_stencil_bits": {"type": ["integer", "null"], "minimum": 0, "maximum": 255},
            "config_samples": {"type": ["integer", "null"], "minimum": 0, "maximum": 255},
            "egl_context_current": boolean(),
            "external_oes_supported": boolean(),
        },
    )
    openxr_gles_swapchain_format = obj(
        "OpenXrGlesSwapchainFormat",
        {
            "format_id": integer(),
            "label": string(),
            "color_renderable": boolean(),
            "depth_renderable": boolean(),
            "selected": boolean(),
        },
    )
    openxr_gles_view_status = obj(
        "OpenXrGlesViewStatus",
        {
            "view_index": integer(0),
            "recommended_width": integer(0),
            "recommended_height": integer(0),
            "swapchain_width": integer(0),
            "swapchain_height": integer(0),
            "acquired_image_index": {"type": ["integer", "null"], "minimum": 0},
            "fbo_status": gl_framebuffer_completeness,
            "viewport_x": integer(),
            "viewport_y": integer(),
            "viewport_width": integer(0),
            "viewport_height": integer(0),
            "diagnostic_pattern": string(),
            "last_rendered_frame_index": {"type": ["integer", "null"], "minimum": 0},
        },
    )
    openxr_gles_feasibility_status = obj(
        "OpenXrGlesFeasibilityStatus",
        {
            "schema": {"const": "rusty.xr.quest.openxr_gles_feasibility.v1"},
            "state": openxr_gles_feasibility_state,
            "runtime_name": nullable_string(),
            "runtime_version": nullable_string(),
            "required_extensions": array(openxr_gles_extension_status),
            "graphics_requirements": {"oneOf": [openxr_gles_graphics_requirements, {"type": "null"}]},
            "context": {"oneOf": [egl_gles_context_status, {"type": "null"}]},
            "swapchain_formats": array(openxr_gles_swapchain_format),
            "views": array(openxr_gles_view_status),
            "frame_rate": {"oneOf": [frame_rate_summary, {"type": "null"}]},
            "issue_codes": array(string()),
            "notes": array(string()),
        },
    )
    surface_texture_oes_ingest_state = enum(
        "SurfaceTextureOesIngestState",
        [
            "Unknown",
            "NotStarted",
            "ExternalTextureCreated",
            "SurfaceTextureCreated",
            "OutputSurfaceReady",
            "DecoderConfigured",
            "DecoderStarted",
            "FrameAvailable",
            "TextureUpdated",
            "Failed",
        ],
    )
    surface_texture_oes_eye_status = obj(
        "SurfaceTextureOesEyeStatus",
        {
            "view_index": integer(0),
            "stream_id": nullable_string(),
            "source_eye": nullable_string(),
            "external_texture_created": boolean(),
            "surface_texture_created": boolean(),
            "output_surface_created": boolean(),
            "decoder_configured": boolean(),
            "decoder_started": boolean(),
            "source_width": {"type": ["integer", "null"], "minimum": 0},
            "source_height": {"type": ["integer", "null"], "minimum": 0},
            "frame_available_count": integer(0),
            "update_tex_image_count": integer(0),
            "skipped_update_count": integer(0),
            "latest_stream_sequence": {"type": ["integer", "null"], "minimum": 0},
            "latest_queued_pts_us": {"type": ["integer", "null"]},
            "latest_surface_texture_timestamp_ns": {"type": ["integer", "null"]},
            "latest_transform_matrix_hash": nullable_string(),
            "transform_matrix_sample_count": integer(0),
            "decoder_error_count": integer(0),
            "latest_decoder_error": nullable_string(),
            "last_update_frame_index": {"type": ["integer", "null"], "minimum": 0},
        },
    )
    surface_texture_oes_ingest_status = obj(
        "SurfaceTextureOesIngestStatus",
        {
            "schema": {"const": "rusty.xr.quest.surface_texture_oes_ingest.v1"},
            "state": surface_texture_oes_ingest_state,
            "session_id": nullable_string(),
            "codec_name": nullable_string(),
            "codec_mime": nullable_string(),
            "eyes": array(surface_texture_oes_eye_status),
            "source_feed_rate": {"oneOf": [frame_rate_summary, {"type": "null"}]},
            "texture_update_rate": {"oneOf": [frame_rate_summary, {"type": "null"}]},
            "cpu_yuv_upload_count": integer(0),
            "issue_codes": array(string()),
            "notes": array(string()),
        },
    )
    broker_transport_kind = enum(
        "BrokerTransportKind",
        [
            "WebSocket",
            "Tcp",
            "ZeroMq",
            "Udp",
            "AdbForwardedTcp",
            "Quic",
            "WebTransport",
            "WebRtcDiagnostic",
            "ExternalSidecar",
            "MetadataOnly",
        ],
    )
    broker_zeromq_pattern = enum(
        "BrokerZeroMqPattern",
        ["Pair", "PubSub", "PushPull", "RequestReply", "DealerRouter"],
    )
    broker_zeromq_bind_mode = enum(
        "BrokerZeroMqBindMode",
        ["Bind", "Connect", "Either"],
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
    broker_command_mutation_class = enum(
        "BrokerCommandMutationClass",
        ["read_only", "mutating", "exclusive_lease", "external_gate"],
    )
    broker_control_lease_state = enum(
        "BrokerControlLeaseState",
        ["offered", "active", "expired", "revoked", "released", "denied"],
    )
    broker_panel_kind = enum(
        "BrokerPanelKind",
        ["state_card", "command_group", "stream_list", "telemetry_chart", "domain_status", "custom"],
    )
    broker_data_sensitivity = enum(
        "BrokerDataSensitivity",
        ["public", "diagnostic", "mixed", "physiology", "derived_physiology", "restricted", "unknown"],
    )
    broker_stream_rate_class = enum(
        "BrokerStreamRateClass",
        ["low_rate_telemetry", "frame_rate_telemetry", "media", "burst", "metadata_only", "unknown"],
    )
    broker_stream_retention_policy = enum(
        "BrokerStreamRetentionPolicy",
        ["none", "rolling_window", "session_replay", "downstream_owned"],
    )
    broker_ui_subscription_policy = enum(
        "BrokerUiSubscriptionPolicy",
        ["manual_only", "auto_subscribe_low_rate", "auto_subscribe_when_selected", "never_subscribe_from_ui"],
    )
    broker_chart_policy = enum(
        "BrokerChartPolicy",
        ["not_chartable", "low_rate_direct", "downsample_required", "dedicated_view_required"],
    )
    broker_registry_node_state = enum(
        "BrokerRegistryNodeState",
        ["starting", "active", "idle", "stopped", "degraded", "failed", "unknown"],
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
    broker_zeromq_bridge_manifest = obj(
        "BrokerZeroMqBridgeManifest",
        {
            "schema": {"const": "rusty.xr.broker.zeromq_bridge_manifest.v1"},
            "bridge_id": string(),
            "endpoint": broker_transport_endpoint,
            "pattern": broker_zeromq_pattern,
            "bind_mode": broker_zeromq_bind_mode,
            "direction": broker_stream_direction,
            "payload_kind": broker_payload_kind,
            "payload_schema": string(),
            "stream_id": nullable_string(),
            "topic_prefix": nullable_string(),
            "max_message_bytes": {"type": ["integer", "null"], "minimum": 1, "maximum": 67108864},
            "high_water_mark": {"type": ["integer", "null"], "minimum": 1},
            "consent_data_categories": array(string()),
            "notes": array(string()),
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
    broker_control_scope = obj(
        "BrokerControlScope",
        {
            "schema": {"const": "rusty.xr.broker.control_scope.v1"},
            "scope_id": string(),
            "command_scope": string(),
            "resource_id": nullable_string(),
        },
    )
    broker_command_precondition = obj(
        "BrokerCommandPrecondition",
        {
            "schema": {"const": "rusty.xr.broker.command_precondition.v1"},
            "expected_revision": {"type": ["integer", "null"], "minimum": 0},
            "lease_id": nullable_string(),
            "holder_client_id": nullable_string(),
        },
    )
    broker_command_authority_requirement = obj(
        "BrokerCommandAuthorityRequirement",
        {
            "schema": {"const": "rusty.xr.broker.command_authority_requirement.v1"},
            "command": string(),
            "command_scope": string(),
            "mutation_class": broker_command_mutation_class,
            "required_capability": nullable_string(),
            "required_capabilities": array(string()),
            "required_role": nullable_string(),
            "allowed_roles": array(string()),
            "lease_required": boolean(),
            "required_lease_scope": {"oneOf": [broker_control_scope, {"type": "null"}]},
            "required_revision": {"type": ["integer", "null"], "minimum": 0},
            "revision_required": boolean(),
            "operator_confirm_required": boolean(),
        },
        required=[
            "schema",
            "command",
            "command_scope",
            "mutation_class",
            "required_capability",
            "lease_required",
            "required_lease_scope",
            "required_revision",
            "operator_confirm_required",
        ],
    )
    broker_control_lease = obj(
        "BrokerControlLease",
        {
            "schema": {"const": "rusty.xr.broker.control_lease.v1"},
            "lease_id": string(),
            "holder_client_id": string(),
            "scope": broker_control_scope,
            "granted_revision": integer(0),
            "expires_elapsed_ns": {"type": ["integer", "null"], "minimum": 0},
            "state": broker_control_lease_state,
        },
    )
    broker_panel_widget_state_card = obj(
        "BrokerPanelWidgetStateCard",
        {
            "kind": {"const": "state_card"},
            "id": string(),
            "label": string(),
            "value_path": string(),
        },
    )
    broker_panel_widget_command_button = obj(
        "BrokerPanelWidgetCommandButton",
        {
            "kind": {"const": "command_button"},
            "id": string(),
            "label": string(),
            "command": string(),
            "read_only": boolean(),
            "command_scope": string(),
            "required_capability": nullable_string(),
            "lease_required": boolean(),
        },
    )
    broker_panel_widget_stream_list = obj(
        "BrokerPanelWidgetStreamList",
        {
            "kind": {"const": "stream_list"},
            "id": string(),
            "label": string(),
            "stream_ids": array(string()),
            "data_sensitivity": broker_data_sensitivity,
        },
    )
    broker_telemetry_chart_descriptor = obj(
        "BrokerTelemetryChartDescriptor",
        {
            "id": string(),
            "title": string(),
            "stream_id": string(),
            "metric": string(),
            "x_axis": string(),
            "y_axis": string(),
            "max_points": integer(1),
            "data_sensitivity": broker_data_sensitivity,
            "command_scope": string(),
            "high_rate_policy": string(),
        },
    )
    broker_panel_widget_telemetry_chart = obj(
        "BrokerPanelWidgetTelemetryChart",
        {
            "kind": {"const": "telemetry_chart"},
            "id": string(),
            "title": string(),
            "stream_id": string(),
            "metric": string(),
            "x_axis": string(),
            "y_axis": string(),
            "max_points": integer(1),
            "data_sensitivity": broker_data_sensitivity,
            "command_scope": string(),
            "high_rate_policy": string(),
        },
    )
    broker_panel_widget = {
        "oneOf": [
            broker_panel_widget_state_card,
            broker_panel_widget_command_button,
            broker_panel_widget_stream_list,
            broker_panel_widget_telemetry_chart,
        ]
    }
    broker_panel_descriptor = obj(
        "BrokerPanelDescriptor",
        {
            "id": string(),
            "title": string(),
            "kind": broker_panel_kind,
            "data_sensitivity": broker_data_sensitivity,
            "command_scope": string(),
            "required_capability": nullable_string(),
            "lease_required": boolean(),
            "widgets": array(broker_panel_widget),
        },
    )
    broker_panel_descriptor_document = obj(
        "BrokerPanelDescriptorDocument",
        {
            "schema": {"const": "rusty.xr.broker.panel_descriptor_set.v1"},
            "version": string(),
            "panels": array(broker_panel_descriptor),
        },
    )
    broker_stream_metric_descriptor = obj(
        "BrokerStreamMetricDescriptor",
        {
            "metric": string(),
            "label": string(),
            "unit": nullable_string(),
            "min_value": {"type": ["number", "null"]},
            "max_value": {"type": ["number", "null"]},
        },
    )
    broker_registered_stream_descriptor = obj(
        "BrokerRegisteredStreamDescriptor",
        {
            "stream_id": string(),
            "label": string(),
            "provider_id": nullable_string(),
            "stream_kind": broker_stream_kind,
            "payload_kind": broker_payload_kind,
            "payload_schema": string(),
            "metrics": array(broker_stream_metric_descriptor),
            "recommended_rate_hz": {"type": ["number", "null"], "exclusiveMinimum": 0},
            "rate_class": broker_stream_rate_class,
            "data_sensitivity": broker_data_sensitivity,
            "retention_policy": broker_stream_retention_policy,
            "ui_subscription_policy": broker_ui_subscription_policy,
            "chart_policy": broker_chart_policy,
        },
        required=[
            "stream_id",
            "label",
            "provider_id",
            "stream_kind",
            "payload_kind",
            "payload_schema",
            "metrics",
            "recommended_rate_hz",
            "rate_class",
            "data_sensitivity",
            "retention_policy",
        ],
    )
    broker_stream_provider_descriptor = obj(
        "BrokerStreamProviderDescriptor",
        {
            "provider_id": string(),
            "label": string(),
            "state": broker_registry_node_state,
            "data_sensitivity": broker_data_sensitivity,
            "stream_ids": array(string()),
        },
    )
    broker_stream_adapter_descriptor = obj(
        "BrokerStreamAdapterDescriptor",
        {
            "adapter_id": string(),
            "label": string(),
            "state": broker_registry_node_state,
            "input_stream_ids": array(string()),
            "output_stream_ids": array(string()),
        },
    )
    broker_stream_subscriber_descriptor = obj(
        "BrokerStreamSubscriberDescriptor",
        {
            "subscriber_id": string(),
            "label": string(),
            "transport": broker_transport_kind,
            "stream_ids": array(string()),
        },
    )
    broker_command_client_descriptor = obj(
        "BrokerCommandClientDescriptor",
        {
            "client_id": string(),
            "label": string(),
            "command_scopes": array(string()),
            "held_lease_ids": array(string()),
        },
    )
    broker_stream_registry_snapshot = obj(
        "BrokerStreamRegistrySnapshot",
        {
            "schema": {"const": "rusty.xr.broker.stream_registry_snapshot.v1"},
            "broker_id": string(),
            "revision": integer(0),
            "captured_elapsed_ns": {"type": ["integer", "null"], "minimum": 0},
            "providers": array(broker_stream_provider_descriptor),
            "streams": array(broker_registered_stream_descriptor),
            "adapters": array(broker_stream_adapter_descriptor),
            "subscribers": array(broker_stream_subscriber_descriptor),
            "command_clients": array(broker_command_client_descriptor),
            "active_leases": array(broker_control_lease),
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
    kiosk_command_outcome = enum(
        "KioskCommandOutcome",
        ["NotStarted", "Succeeded", "Failed", "Blocked", "Skipped", "TimedOut", "Unknown"],
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
    kiosk_command_run_record = obj(
        "KioskCommandRunRecord",
        {
            "schema": {"const": "rusty.xr.kiosk.command_run_record.v1"},
            "run_id": string(),
            "command_goal": string(),
            "surface_intent": kiosk_surface_intent,
            "primary": kiosk_command_evidence,
            "fallback": {"oneOf": [kiosk_command_evidence, {"type": "null"}]},
            "status_before": {"oneOf": [kiosk_control_plane_status, {"type": "null"}]},
            "status_after": {"oneOf": [kiosk_control_plane_status, {"type": "null"}]},
            "outcome": kiosk_command_outcome,
            "issue_codes": array(string()),
            "notes": array(string()),
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
            "IngestCopy",
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
        [
            "SourceColor",
            "SourceExternal",
            "SourceLuma",
            "Guide",
            "Mask",
            "DisplacementMap",
            "PreviousPass",
        ],
    )
    effect_buffer_format = enum(
        "EffectBufferFormat",
        [
            "Rgba8",
            "Rgba16Float",
            "Rgba32Float",
            "R8",
            "R16Float",
            "R32Float",
            "ExternalOes",
            "ExternalGpu",
        ],
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
    eye = enum("Eye", ["Mono", "Left", "Right"])
    matrix_lane_kind = enum(
        "ProjectionMatrixLaneKind",
        [
            "OpenXrVulkanHardwareBuffer",
            "FrameworkCpuYuv",
            "OpenXrOpenGlSurfaceTextureOes",
            "Reference",
            "Other",
        ],
    )
    matrix_step_status = enum(
        "MatrixStepStatus",
        ["NotRun", "Passed", "Failed", "Blocked", "NotApplicable", "Ambiguous"],
    )
    projection_stage_kind = enum(
        "ProjectionStageKind",
        ["SurfaceToScreen", "ScreenToSurface", "SurfaceToCamera", "ScreenToCamera"],
    )
    projection_guide_domain = enum(
        "ProjectionGuideDomain",
        ["Unknown", "DisplayScreen", "SubmittedSurface", "DirectSurfaceCamera", "ScreenCamera", "Other"],
    )
    invalid_projection_fill_policy = enum(
        "InvalidProjectionFillPolicy",
        [
            "Unknown",
            "NotApplicable",
            "Black",
            "SolidRed",
            "Transparent",
            "Clamp",
            "Repeat",
            "OrientedSourceFallback",
            "VisualContinuityFallback",
            "Other",
        ],
    )
    homography_rows = {
        "type": "array",
        "minItems": 3,
        "maxItems": 3,
        "items": {"type": "array", "minItems": 3, "maxItems": 3, "items": number()},
    }
    nullable_number = {"type": ["number", "null"]}
    nullable_integer = {"type": ["integer", "null"], "minimum": 0}
    nullable_signed_integer = {"type": ["integer", "null"]}
    matrix_synthetic_video_source = obj(
        "MatrixSyntheticVideoSource",
        {
            "pattern": string(),
            "size": image_size(),
            "left_port": integer(0),
            "right_port": integer(0),
            "bitrate_bps": integer(0),
            "max_packets": integer(0),
            "stream_header_projection_metadata": boolean(),
            "live_unbounded": boolean(),
        },
    )
    projection_stage_token_row = obj(
        "ProjectionStageTokenRow",
        {
            "lane_id": string(),
            "eye": eye,
            "stage": projection_stage_kind,
            "token": nullable_string(),
            "rows": {"oneOf": [homography_rows, {"type": "null"}]},
            "source": nullable_string(),
        },
    )
    projection_footprint_row_span = obj(
        "ProjectionFootprintRowSpan",
        {
            "row_fraction": {"type": "number", "minimum": 0, "maximum": 1},
            "x0_fraction": nullable_number,
            "x1_fraction": nullable_number,
            "width_fraction": {"type": "number", "minimum": 0, "maximum": 1},
            "center_fraction": nullable_number,
        },
    )
    projection_footprint_summary = obj(
        "ProjectionFootprintSummary",
        {
            "lane_id": string(),
            "layer_id": string(),
            "active_fraction": nullable_number,
            "bbox_fraction": {
                "oneOf": [
                    {"type": "array", "minItems": 4, "maxItems": 4, "items": {"type": "number", "minimum": 0, "maximum": 1}},
                    {"type": "null"},
                ]
            },
            "row_spans": array(projection_footprint_row_span),
            "mask_iou_against_reference": nullable_number,
            "invalid_fill_policy": invalid_projection_fill_policy,
            "guide_domain": projection_guide_domain,
            "explicit_valid_mask": boolean(),
            "note": nullable_string(),
        },
    )
    projection_performance_scorecard = obj(
        "ProjectionPerformanceScorecard",
        {
            "source_packet_fps": nullable_number,
            "decoder_input_access_unit_fps": nullable_number,
            "decoded_texture_update_fps": nullable_number,
            "surface_texture_update_count": nullable_integer,
            "surface_texture_skipped_frame_count": nullable_integer,
            "cpu_yuv_upload_update_fps": nullable_number,
            "hardware_buffer_import_count": nullable_integer,
            "hardware_buffer_import_cache_miss_count": nullable_integer,
            "hardware_buffer_import_cache_evict_count": nullable_integer,
            "openxr_fps": nullable_number,
            "app_cpu_ms": nullable_number,
            "app_gpu_ms": nullable_number,
            "app_cpu_gpu_ms": nullable_number,
            "gpu_percent": nullable_number,
            "thermal_status": nullable_signed_integer,
            "performance_level_cpu": nullable_integer,
            "performance_level_gpu": nullable_integer,
            "pass_count": nullable_integer,
            "fbo_switch_count": nullable_integer,
            "render_target_switch_count": nullable_integer,
            "intermediate_texture_bytes_per_frame": nullable_integer,
            "frame_age_at_submit_ms": nullable_number,
            "repeated_render_frames_per_distinct_source_frame": nullable_number,
            "app_fatal_count": nullable_integer,
            "gpu_fault_count": nullable_integer,
            "android_runtime_crash_count": nullable_integer,
        },
    )
    projection_matrix_lane_report = obj(
        "ProjectionMatrixLaneReport",
        {
            "lane_id": string(),
            "label": string(),
            "kind": matrix_lane_kind,
            "source_feed": matrix_step_status,
            "decoded_texture": matrix_step_status,
            "projection_stage": matrix_step_status,
            "projection_footprint": matrix_step_status,
            "public_or_raw_layer": matrix_step_status,
            "effect_or_guide_layer": matrix_step_status,
            "performance_budget": matrix_step_status,
            "stage_tokens": array(projection_stage_token_row),
            "footprints": array(projection_footprint_summary),
            "performance": {"oneOf": [projection_performance_scorecard, {"type": "null"}]},
            "notes": array(string()),
            "blockers": array(string()),
        },
    )
    projection_performance_matrix_packet = obj(
        "ProjectionPerformanceMatrixPacket",
        {
            "schema": {"const": "rusty.xr.projection_performance_matrix.v1"},
            "packet_id": string(),
            "source": matrix_synthetic_video_source,
            "lanes": array(projection_matrix_lane_report),
            "notes": array(string()),
        },
    )
    field_of_view = obj(
        "FieldOfView",
        {
            "angle_left_radians": number(),
            "angle_right_radians": number(),
            "angle_up_radians": number(),
            "angle_down_radians": number(),
        },
    )
    depth_payload_descriptor = obj(
        "DepthPayloadDescriptor",
        {
            "size": image_size(),
            "byte_len": integer(0),
            "row_stride_bytes": nullable_integer,
        },
    )
    depth_metric_range = obj(
        "DepthMetricRange",
        {
            "near_z_m": number(),
            "far_z_m": number(),
        },
    )
    depth_world_space_metric_range = obj(
        "DepthWorldSpaceMetricRange",
        {
            "near_z_m": number(),
            "far_z_m": {"type": ["number", "null"]},
            "far_z_infinite": boolean(),
        },
    )
    depth_view_descriptor = obj(
        "DepthViewDescriptor",
        {
            "eye": eye,
            "pose": pose(),
            "fov": field_of_view,
        },
    )
    depth_world_space_stage = obj(
        "DepthWorldSpaceStageEvidence",
        {
            "stage": enum(
                "DepthWorldSpaceStageKind",
                [
                    "DepthUvToDepthViewRay",
                    "DepthViewRayToMetricPoint",
                    "DepthViewPointToReferenceSpace",
                    "ReferenceSpacePointToRenderEye",
                    "RenderEyePointToScreen",
                ],
            ),
            "owner": string(),
            "evidence": string(),
        },
    )
    depth_world_space_contract = obj(
        "DepthWorldSpaceContract",
        {
            "schema": {"const": "rusty.xr.depth_world_space_contract.v1"},
            "contract_id": string(),
            "source_kind": enum(
                "DepthWorldSpaceSourceKind",
                ["RuntimeEnvironmentDepth", "Synthetic", "Imported", "Other"],
            ),
            "render_path": enum(
                "DepthWorldSpaceRenderPath",
                [
                    "FullscreenDepthVisualizer",
                    "GeneratedDepthMesh",
                    "RetainedMetricParticles",
                    "SceneParticleMap",
                    "Other",
                ],
            ),
            "depth_payload": depth_payload_descriptor,
            "depth_format": enum("DepthFormat", ["Float32Meters", "Uint16Millimeters", "Uint16Raw"]),
            "depth_range": depth_world_space_metric_range,
            "runtime_capture_time_ns": {"type": ["integer", "null"]},
            "layer_count": integer(1),
            "left_depth_view": depth_view_descriptor,
            "right_depth_view": depth_view_descriptor,
            "reference_space": string(),
            "reference_space_units": string(),
            "depth_uv_origin": string(),
            "depth_texture_transform": string(),
            "linearization": string(),
            "point_reconstruction": string(),
            "render_eye_view_source": string(),
            "projection_y_convention": string(),
            "render_target_size": {"oneOf": [image_size(), {"type": "null"}]},
            "sample_identity_policy": enum(
                "DepthSampleIdentityPolicy",
                [
                    "DepthRasterSlot",
                    "RetainedReferencePoint",
                    "ReferenceSpaceCell",
                    "NotRetained",
                ],
            ),
            "passthrough_visible": boolean(),
            "stages": array(depth_world_space_stage),
        },
    )
    parity_capture_provider = enum("ParitySuiteHeadsetCaptureProvider", ["fast-adb", "hzdb"])
    parity_source_mode = enum("ParitySuiteSourceMode", ["direct-camera", "broker-camera", "broker-synthetic"])
    parity_evidence_mode = enum("ParitySuiteEvidenceMode", ["custom", "fast-visual", "full-evidence"])
    parity_status = enum("ParitySuiteStatus", ["pending", "ok", "failed", "skipped"])
    projection_property_hygiene_mode = enum("ProjectionPropertyHygieneMode", ["fail", "clear", "ignore"])
    projection_property_hygiene_value = open_obj(
        "ProjectionPropertyHygieneValue",
        {
            "property": string(),
            "value": string(),
            "nonEmpty": boolean(),
        },
    )
    projection_property_hygiene_summary = open_obj(
        "ProjectionPropertyHygieneSummary",
        {
            "schemaVersion": {"const": "rusty.xr.projection-property-hygiene.v1"},
            "checkedAt": string(),
            "mode": projection_property_hygiene_mode,
            "keyCount": integer(0),
            "staleBeforeCount": integer(0),
            "staleBefore": array(projection_property_hygiene_value),
            "clearedCount": integer(0),
            "clearedProperties": array(string()),
            "afterNonEmptyCount": integer(0),
            "afterNonEmpty": array(projection_property_hygiene_value),
            "status": enum("ProjectionPropertyHygieneStatus", ["ok", "failed"]),
        },
    )
    parity_timing_record = open_obj(
        "CanvasCustomProjectionParitySuiteTimingRecord",
        {
            "caseId": string(),
            "step": string(),
            "status": enum("ParitySuiteTimingStatus", ["ok", "failed"]),
            "startedAt": string(),
            "endedAt": string(),
            "startElapsedMs": integer(0),
            "endElapsedMs": integer(0),
            "durationMs": integer(0),
            "error": string(),
        },
    )
    parity_timing_step_summary = open_obj(
        "CanvasCustomProjectionParitySuiteTimingStepSummary",
        {
            "step": string(),
            "count": integer(0),
            "totalMs": integer(0),
            "minMs": integer(0),
            "maxMs": integer(0),
            "avgMs": number(),
            "failures": integer(0),
        },
    )
    parity_timing_summary = open_obj(
        "CanvasCustomProjectionParitySuiteTimingSummary",
        {
            "schemaVersion": {"const": "rusty.xr.canvas-custom-projection-parity-suite.timing.v1"},
            "totalElapsedMs": integer(0),
            "timingJsonl": string(),
            "records": array(parity_timing_record),
            "byStep": array(parity_timing_step_summary),
        },
    )
    parity_case_record = open_obj(
        "CanvasCustomProjectionParitySuiteCaseRecord",
        {
            "id": string(),
            "lane": enum("ParitySuiteLane", ["hwb", "oes", "makepad"]),
            "mode": enum("ParitySuiteProjectionMode", ["canvas", "custom"]),
            "runtimeProfile": string(),
            "artifactDir": string(),
            "mediaProjection": {"type": ["string", "null"]},
            "hzdb": string(),
            "headsetCapture": string(),
            "headsetCaptureProvider": parity_capture_provider,
            "brokerH264SourceMode": parity_source_mode,
            "processingLayer": enum("ParitySuiteProcessingLayer", ["raw", "blur", "peripheral-stretch"]),
            "blurRadiusPx": number(),
        },
    )
    parity_capture_contract = open_obj(
        "CanvasCustomProjectionParitySuiteCaptureContract",
        {
            "evidenceMode": parity_evidence_mode,
            "mediaProjectionEnabled": boolean(),
            "analyzerEnabled": boolean(),
            "contactSheetEnabled": boolean(),
            "timingEnabled": boolean(),
            "readinessTimingEnabled": boolean(),
            "projectionPropertyHygiene": projection_property_hygiene_mode,
            "geometryWitness": string(),
            "modeSemantics": string(),
        },
    )
    parity_summary = open_obj(
        "CanvasCustomProjectionParitySuiteSummary",
        {
            "schemaVersion": {"const": "rusty.xr.canvas-custom-projection-parity-suite.v1"},
            "capturedAt": string(),
            "serial": string(),
            "sourceMode": parity_source_mode,
            "evidenceMode": parity_evidence_mode,
            "sessionRoot": string(),
            "screenshotsRoot": string(),
            "contactSheet": string(),
            "screenSpaceAnalysis": string(),
            "timingJsonl": string(),
            "timingSummary": string(),
            "headsetCaptureProvider": parity_capture_provider,
            "captureContract": parity_capture_contract,
            "geometry": open_obj(
                "CanvasCustomProjectionParitySuiteGeometry",
                {
                    "projectionDepthMeters": number(),
                    "cameraPreviewFovYDegrees": number(),
                    "cameraPreviewOffsetYMeters": number(),
                    "cameraRawOverlayOverscan": number(),
                    "projectionBorderPolicy": enum("ProjectionBorderPolicy", ["passthrough-underlay", "solid-red"]),
                    "processingLayer": enum("ParitySuiteGeometryProcessingLayer", ["raw", "blur", "peripheral-stretch"]),
                    "peripheralStretchMode": enum("PeripheralStretchMode", ["edge-stretch"]),
                    "peripheralStretchCoreScale": number(),
                    "peripheralStretchEdgeInsetUv": number(),
                    "peripheralStretchMaxInsetUv": number(),
                    "peripheralStretchCurve": number(),
                    "peripheralStretchInnerBlendUv": number(),
                    "peripheralStretchBlendCurve": number(),
                    "peripheralStretchBlendMode": enum("PeripheralStretchBlendMode", ["off", "target-inner-band"]),
                    "peripheralStretchCornerMode": enum("PeripheralStretchCornerMode", ["target-footprint"]),
                    "peripheralStretchDebug": enum("PeripheralStretchDebugMode", ["off", "regions", "sample-uv"]),
                    "blurRadiusPx": number(),
                    "projectionAreaOpacity": number(),
                    "projectionBorderOpacity": number(),
                    "boundedCanvasProjectionArea": boolean(),
                    "skipMediaProjection": boolean(),
                    "useResolvedProjectionRuntime": boolean(),
                    "captureReadinessMode": enum("ParitySuiteCaptureReadinessMode", ["contract", "warmup", "none"]),
                    "readyTimeoutSeconds": integer(0),
                    "readyPollIntervalMs": integer(0),
                    "readySettleMs": integer(0),
                    "projectionAreaRadiusXUv": number(),
                    "projectionAreaRadiusYUv": number(),
                    "projectionAreaCornerRadiusUv": number(),
                    "makepadStartupTimeoutSeconds": integer(0),
                    "makepadUseFixedSampleWindow": boolean(),
                    "makepadSampleSeconds": integer(0),
                    "makepadReadySettleMs": integer(0),
                    "makepadPostRunSettleSeconds": integer(0),
                    "expectedMakepadSourceEyeMapping": string(),
                    "failOnAnalyzerIssue": boolean(),
                    "skipAnalyzer": boolean(),
                },
            ),
            "brokerH264": loose_object(),
            "captureRouteNotes": array(string()),
            "boundedFootprintEvidence": array(loose_object()),
            "records": array(parity_case_record),
            "analysis": open_obj(
                "CanvasCustomProjectionParitySuiteAnalysisStatus",
                {"skipped": boolean(), "status": parity_status, "outDir": string(), "error": string()},
            ),
            "contactSheetStatus": open_obj(
                "CanvasCustomProjectionParitySuiteContactSheetStatus",
                {"skipped": boolean(), "status": parity_status, "path": string(), "error": string()},
            ),
            "timing": open_obj(
                "CanvasCustomProjectionParitySuiteTimingPointer",
                {"totalElapsedMs": integer(0), "jsonl": string(), "summary": string()},
            ),
            "artifactValidation": open_obj(
                "CanvasCustomProjectionParitySuiteArtifactValidationStatus",
                {"skipped": boolean(), "status": parity_status, "validator": string(), "error": string()},
            ),
        },
    )
    screen_space_report = open_obj(
        "RawStackScreenSpaceReport",
        {
            "schema_version": {"const": "rusty.xr.raw-stack-screen-space.v1"},
            "suite_root": string(),
            "out_dir": string(),
            "projection_border_policy": string(),
            "processing_layer": string(),
            "allow_visible_fallback": boolean(),
            "lanes": array(loose_object()),
            "projection_mapping_schema_version": {"const": "rusty.xr.projection-mapping-run-record.v1"},
            "projection_mapping_summary": loose_object(),
            "projection_coordinate_contract_schema_version": {"const": "rusty.xr.projection-coordinate-contract.v1"},
            "projection_coordinate_contract_summary": loose_object(),
            "source_sampling_contract_schema_version": {"const": "rusty.xr.source-sampling-contract.v1"},
            "source_sampling_contract_summary": loose_object(),
        },
    )
    projection_mapping_record = open_obj(
        "ProjectionMappingRunRecord",
        {
            "schema_version": {"const": "rusty.xr.projection-mapping-run-record.v1"},
            "suite_root": string(),
            "mode": string(),
            "eye": eye,
            "artifact_root": string(),
            "image_path": string(),
            "log_path": {"type": ["string", "null"]},
            "content": loose_object(),
            "orientation": loose_object(),
            "app_projection": loose_object(),
            "expected_screenshot": loose_object(),
            "observed_screenshot": loose_object(),
            "verdict": loose_object(),
        },
    )
    projection_mapping_summary = open_obj(
        "ProjectionMappingSummary",
        {
            "schema_version": {"const": "rusty.xr.projection-mapping-run-record.v1"},
            "record_count": integer(0),
            "verdict_counts": object_map(integer(0)),
            "modes": object_map(loose_object()),
            "parity_checks": array(loose_object()),
        },
    )
    projection_coordinate_contract = open_obj(
        "ProjectionCoordinateContract",
        {
            "schema_version": {"const": "rusty.xr.projection-coordinate-contract.v1"},
            "suite_root": string(),
            "mode": string(),
            "status": enum("ProjectionCoordinateContractStatus", ["ready", "needs-evidence", "blocked"]),
            "lane": loose_object(),
            "run_request": loose_object(),
            "source": loose_object(),
            "metadata": loose_object(),
            "texture_or_upload": loose_object(),
            "source_sampling": loose_object(),
            "projection": loose_object(),
            "openxr": loose_object(),
            "transforms": loose_object(),
            "mask_and_processing": loose_object(),
            "analysis": loose_object(),
            "gaps": array(string()),
        },
    )
    projection_coordinate_contract_summary = open_obj(
        "ProjectionCoordinateContractSummary",
        {
            "schema_version": {"const": "rusty.xr.projection-coordinate-contract.v1"},
            "record_count": integer(0),
            "status_counts": object_map(integer(0)),
            "gap_counts": object_map(integer(0)),
            "modes": object_map(loose_object()),
        },
    )
    source_sampling_payload = {
        "type": "object",
        "additionalProperties": True,
        "properties": {
            "contract": string(),
            "source_eye_mapping": string(),
            "content_uv_rect": array(number()),
            "source_visible_uv_rect": array(number()),
            "homography_output_uv": string(),
            "sample_input_uv": string(),
            "sample_transform_stage": string(),
            "sample_transform": string(),
            "sample_transform_owner": string(),
            "sample_transform_applied": boolean(),
            "sample_output_uv": string(),
            "sampler_uv_origin": string(),
            "sampler_y_axis": string(),
            "texture_transform_stage": string(),
            "texture_transform_owner": string(),
        },
    }
    source_eye_mapping = enum(
        "StereoSourceEyeMapping",
        ["display-left-from-left-source", "display-left-from-right-source"],
    )
    source_sampling_transform_stage = enum(
        "SourceSamplingTransformStage",
        [
            "none",
            "post-homography-pre-texture-sample",
            "post-homography-pre-oes-sample",
            "post-homography-pre-yuv-sample",
            "post-homography-pre-source-visible-rect-then-texture-sample",
            "other",
        ],
    )
    source_sampler_y_axis = enum(
        "SourceSamplerYAxis",
        [
            "renderer-defined",
            "surface-texture-transform-defined",
            "content-top-left-y-down",
            "makepad-sampler-origin-convention",
            "other",
        ],
    )
    source_uv_rect = obj("SourceUvRect", {"origin_uv": vec2(), "size_uv": vec2()})
    source_sampling_backend = enum("SourceSamplingBackend", ["hwb", "oes", "makepad"])
    source_sampling_contract = open_obj(
        "SourceSamplingContract",
        {
            "schema_version": {"const": "rusty.xr.source-sampling-contract.v1"},
            "backend": source_sampling_backend,
            "suite_root": string(),
            "mode": string(),
            "source_eye_mapping": source_eye_mapping,
            "content_uv_rect": source_uv_rect,
            "source_visible_uv_rect": source_uv_rect,
            "transform_stage": source_sampling_transform_stage,
            "transform_label": string(),
            "transform_owner": string(),
            "transform_applied": boolean(),
            "output_uv_label": string(),
            "sampler_uv_origin": string(),
            "sampler_y_axis": source_sampler_y_axis,
            "texture_transform_stage": source_sampling_transform_stage,
            "texture_transform_owner": string(),
            "status": enum("SourceSamplingContractStatus", ["ready", "needs-evidence", "blocked"]),
            "lane": loose_object(),
            "run_request": loose_object(),
            "source": loose_object(),
            "metadata": loose_object(),
            "texture_or_upload": loose_object(),
            "source_sampling": source_sampling_payload,
            "evidence": loose_object(),
            "gaps": array(string()),
        },
    )
    source_sampling_contract_summary = open_obj(
        "SourceSamplingContractSummary",
        {
            "schema_version": {"const": "rusty.xr.source-sampling-contract.v1"},
            "record_count": integer(0),
            "status_counts": object_map(integer(0)),
            "gap_counts": object_map(integer(0)),
            "modes": object_map(loose_object()),
        },
    )
    camera_texture_lane_kind = enum(
        "CameraTextureLaneKind",
        [
            "vulkan-hwb-direct-camera2-raw",
            "gles-oes-direct-camera2-raw",
            "makepad-cpuyuv-direct-camera2-raw",
            "makepad-hwb-external-direct-camera2-raw",
            "other",
        ],
    )
    camera_texture_source_kind = enum(
        "CameraTextureSourceKind", ["direct-camera2", "broker-h264", "synthetic", "other"]
    )
    camera_texture_resource_kind = enum(
        "CameraTextureResourceKind",
        [
            "android-hardware-buffer-vulkan",
            "surface-texture-oes",
            "cpu-yuv-plane-textures",
            "makepad-hardware-buffer-external",
            "other",
        ],
    )
    camera_texture_descriptor_shape = enum(
        "CameraTextureDescriptorShape",
        [
            "unknown",
            "cpu-yuv-plane-textures",
            "hardware-buffer-yuv-plane-textures",
            "sampled-image-and-sampler",
            "combined-image-sampler",
            "sampler-external-oes",
            "not-applicable",
        ],
    )
    camera_texture_color_status = enum(
        "CameraTextureColorStatus",
        ["accepted-reference", "experimental", "diagnostic-only", "unknown"],
    )
    optional_non_negative_integer = {"type": ["integer", "null"], "minimum": 0}
    optional_integer = {"type": ["integer", "null"]}
    camera_texture_lane_source = obj(
        "CameraTextureLaneSource",
        {
            "source_kind": camera_texture_source_kind,
            "source_label": string(),
            "delivered_size": image_size(),
            "handoff_label": string(),
            "source_eye_mapping": source_eye_mapping,
        },
    )
    camera_texture_lane_resource = obj(
        "CameraTextureLaneResource",
        {
            "resource_kind": camera_texture_resource_kind,
            "resource_label": string(),
            "descriptor_shape": camera_texture_descriptor_shape,
            "texture_label": string(),
            "buffer_id": optional_non_negative_integer,
            "import_cache_size": optional_non_negative_integer,
            "shader_interface_label": string(),
        },
    )
    camera_texture_lane_transform = obj(
        "CameraTextureLaneTransform",
        {
            "source_visible_uv_rect": source_uv_rect,
            "transform_stage": source_sampling_transform_stage,
            "transform_label": string(),
            "transform_owner": string(),
            "oes_transform_matrix": {
                "type": ["array", "null"],
                "items": number(),
                "minItems": 16,
                "maxItems": 16,
            },
            "hwb_transform_flags": optional_non_negative_integer,
            "yuv_rotation_steps": {"type": ["integer", "null"], "minimum": 0, "maximum": 3},
        },
    )
    camera_texture_lane_color = obj(
        "CameraTextureLaneColor",
        {
            "color_status": camera_texture_color_status,
            "color_reference": string(),
            "color_matrix": string(),
            "color_range": string(),
            "color_transfer": string(),
        },
    )
    camera_texture_lane_timing = obj(
        "CameraTextureLaneTiming",
        {
            "camera_frame_sequence": optional_non_negative_integer,
            "camera_timestamp_ns": optional_non_negative_integer,
            "acquire_time_ns": optional_non_negative_integer,
            "upload_time_ns": optional_non_negative_integer,
            "import_time_ns": optional_non_negative_integer,
            "texture_update_sequence": optional_non_negative_integer,
            "texture_submit_sequence": optional_non_negative_integer,
            "xr_end_frame_time_ns": optional_non_negative_integer,
        },
    )
    camera_texture_lane_lifecycle = obj(
        "CameraTextureLaneLifecycle",
        {
            "first_frame_seen": boolean(),
            "fallback_active": boolean(),
            "fallback_reason": nullable_string(),
            "frame_reuse_policy": string(),
            "resource_release_policy": string(),
            "app_focused": {"type": ["boolean", "null"]},
        },
    )
    camera_texture_lane_projection = obj(
        "CameraTextureLaneProjection",
        {
            "projection_border_policy": string(),
            "processing_layer": string(),
            "projection_surface_label": string(),
            "projection_status_label": string(),
        },
    )
    camera_texture_lane_summary_timing = obj(
        "CameraTextureLaneSummaryTiming",
        {
            "camera_frame_sequence": optional_non_negative_integer,
            "camera_timestamp_ns": optional_non_negative_integer,
            "acquire_time_ns": optional_non_negative_integer,
            "upload_time_ns": optional_non_negative_integer,
            "import_time_ns": optional_non_negative_integer,
            "texture_update_sequence": optional_non_negative_integer,
            "texture_submit_sequence": optional_non_negative_integer,
            "xr_end_frame_time_ns": optional_non_negative_integer,
        },
    )
    camera_texture_lane_summary_timing_relations = obj(
        "CameraTextureLaneSummaryTimingRelations",
        {
            "acquire_to_upload_ns": optional_integer,
            "acquire_to_import_ns": optional_integer,
            "upload_to_xr_end_frame_ns": optional_integer,
            "import_to_xr_end_frame_ns": optional_integer,
            "texture_update_to_submit_sequence_delta": optional_integer,
            "texture_update_to_submit_sequence_relation": string(),
        },
    )
    camera_texture_lane_run_config = obj(
        "CameraTextureLaneRunConfig",
        {
            "app_id": nullable_string(),
            "package_name": nullable_string(),
            "runtime_profile": nullable_string(),
            "source_mode": nullable_string(),
            "evidence_mode": nullable_string(),
            "camera_pipeline_preset": nullable_string(),
            "camera_projection_effect_mode": nullable_string(),
            "camera_projection_mode": nullable_string(),
            "direct_camera_texture_path": nullable_string(),
            "xr_render_scale": {"type": ["number", "null"]},
            "projection_border_policy": nullable_string(),
            "processing_layer": nullable_string(),
            "blur_radius_px": {"type": ["number", "null"]},
        },
    )
    camera_texture_lane_summary = obj(
        "CameraTextureLaneSummary",
        {
            "source_kind": camera_texture_source_kind,
            "delivered_size": image_size(),
            "resource_kind": camera_texture_resource_kind,
            "descriptor_shape": camera_texture_descriptor_shape,
            "color_status": camera_texture_color_status,
            "projection_border_policy": string(),
            "processing_layer": string(),
            "first_frame_seen": boolean(),
            "fallback_active": boolean(),
            "fallback_reason": nullable_string(),
            "frame_reuse_policy": string(),
            "resource_release_policy": string(),
            "timing": camera_texture_lane_summary_timing,
            "timing_relations": camera_texture_lane_summary_timing_relations,
        },
    )
    camera_texture_lane_contract = obj(
        "CameraTextureLaneContract",
        {
            "schema_version": {"const": "rusty.xr.camera-texture-lane-contract.v1"},
            "lane_kind": camera_texture_lane_kind,
            "source": camera_texture_lane_source,
            "resource": camera_texture_lane_resource,
            "transform": camera_texture_lane_transform,
            "color": camera_texture_lane_color,
            "timing": camera_texture_lane_timing,
            "lifecycle": camera_texture_lane_lifecycle,
            "projection": camera_texture_lane_projection,
        },
    )
    camera_texture_lane_contract_summary = obj(
        "CameraTextureLaneContractSummary",
        {
            "schema_version": {"const": "rusty.xr.camera-texture-lane-contract-summary.v1"},
            "contract_schema_version": {"const": "rusty.xr.camera-texture-lane-contract.v1"},
            "run_config": camera_texture_lane_run_config,
            "record_count": integer(0),
            "lane_kind_counts": object_map(integer(0)),
            "color_status_counts": object_map(integer(0)),
            "descriptor_shape_counts": object_map(integer(0)),
            "log_file_count": integer(0),
            "source_kind_counts": object_map(integer(0)),
            "resource_kind_counts": object_map(integer(0)),
            "projection_border_policy_counts": object_map(integer(0)),
            "processing_layer_counts": object_map(integer(0)),
            "fallback_active_counts": object_map(integer(0)),
            "timing_field_counts": object_map(integer(0)),
            "lane_summaries": object_map(camera_texture_lane_summary),
        },
        required=[
            "schema_version",
            "contract_schema_version",
            "record_count",
            "lane_kind_counts",
            "color_status_counts",
            "descriptor_shape_counts",
            "log_file_count",
        ],
    )
    camera_texture_lane_suite_summary = open_obj(
        "CameraTextureLaneSuiteSummary",
        {
            "schema_version": {"const": "rusty.xr.camera-texture-lane-suite-summary.v1"},
            "input_schema_version": {"const": "rusty.xr.camera-texture-lane-contract-summary.v1"},
            "summary_count": integer(0),
            "lane_case_count": integer(0),
            "unreadable_summary_count": integer(0),
            "unreadable_summaries": array(string()),
            "summary_paths": array(string()),
            "lane_kind_counts": object_map(integer(0)),
            "color_status_counts": object_map(integer(0)),
            "descriptor_shape_counts": object_map(integer(0)),
            "projection_border_policy_counts": object_map(integer(0)),
            "processing_layer_counts": object_map(integer(0)),
            "fallback_active_counts": object_map(integer(0)),
            "timing_field_counts": object_map(integer(0)),
            "run_config_counts": loose_object(),
            "lane_records": array(loose_object()),
        },
    )
    perfetto_trace_mode = enum("CameraPerfettoTraceMode", ["skip", "capture", "analyze", "required"])
    perfetto_trace_provider = enum(
        "CameraPerfettoTraceProvider", ["hzdb", "meta-mcp", "adb-perfetto", "manual", "skipped"]
    )
    perfetto_capture_preset = enum(
        "CameraPerfettoCapturePreset", ["standard", "gpu", "cpu", "lightweight", "full", "custom"]
    )
    perfetto_analysis_focus = enum(
        "CameraPerfettoAnalysisFocus", ["overview", "gpu", "cpu", "frames", "threads"]
    )
    perfetto_intended_use = enum(
        "CameraPerfettoIntendedUse",
        [
            "diagnostic-calibration",
            "effect-layer-ab",
            "stale-localization",
            "gpu-deep-dive",
            "cpu-deep-dive",
            "manual",
        ],
    )
    perfetto_overhead_policy = enum(
        "CameraPerfettoOverheadPolicy", ["rare-deep-trace", "routine-gate"]
    )
    perfetto_raw_trace_policy = enum(
        "CameraPerfettoRawTracePolicy", ["ignored-artifact-only", "external-retention", "manual"]
    )
    perfetto_custom_flags = obj(
        "CameraPerfettoCustomFlags",
        {
            "gpu_render_stage": boolean(),
            "gpu_metrics": boolean(),
            "cpu_scheduling": boolean(),
            "xr_runtime": boolean(),
            "vulkan_layer": boolean(),
            "extended_scheduling": boolean(),
        },
    )
    camera_perfetto_trace_plan = obj(
        "CameraPerfettoTracePlan",
        {
            "schema_version": {"const": "rusty.xr.camera-perfetto-trace-plan.v1"},
            "enabled": boolean(),
            "mode": perfetto_trace_mode,
            "provider": perfetto_trace_provider,
            "capture_preset": perfetto_capture_preset,
            "duration_ms": optional_non_negative_integer,
            "package_name": nullable_string(),
            "output_label": string(),
            "artifact_dir": nullable_string(),
            "trace_path": nullable_string(),
            "analysis_path": nullable_string(),
            "custom_flags": perfetto_custom_flags,
            "analysis_focus": perfetto_analysis_focus,
            "intended_use": perfetto_intended_use,
            "overhead_policy": perfetto_overhead_policy,
            "raw_trace_policy": perfetto_raw_trace_policy,
            "notes": array(string()),
            "suggested_commands": array(string()),
        },
    )
    projection_runtime_readback = open_obj(
        "ProjectionRuntimeReadback",
        {
            "schemaVersion": {"const": "rusty.xr.projection-runtime-readback.v1"},
            "status": enum("ProjectionRuntimeReadbackStatus", ["ok", "warning", "failed", "skipped"]),
            "issueCount": integer(0),
            "errorCount": integer(0),
            "warningCount": integer(0),
            "expectedCount": integer(0),
            "manifestValueCount": integer(0),
            "resolvedCount": integer(0),
            "comparedCount": integer(0),
            "comparedKeys": array(string()),
            "expectedBackend": string(),
            "expectedPhase": string(),
            "manifestScopes": array(string()),
            "logcatPaths": array(string()),
            "expected": array(loose_object()),
            "resolved": object_map(loose_object()),
            "manifestValues": array(loose_object()),
            "issues": array(loose_object()),
        },
        required=["schemaVersion", "status"],
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
        "depth-world-space-contract.schema.json": depth_world_space_contract,
        "canvas-custom-projection-parity-suite-summary.schema.json": parity_summary,
        "canvas-custom-projection-parity-suite-timing-summary.schema.json": parity_timing_summary,
        "canvas-custom-projection-parity-suite-timing-record.schema.json": parity_timing_record,
        "raw-stack-screen-space-report.schema.json": screen_space_report,
        "projection-property-hygiene.schema.json": projection_property_hygiene_summary,
        "projection-mapping-run-record.schema.json": projection_mapping_record,
        "projection-mapping-summary.schema.json": projection_mapping_summary,
        "projection-coordinate-contract.schema.json": projection_coordinate_contract,
        "projection-coordinate-contract-summary.schema.json": projection_coordinate_contract_summary,
        "source-sampling-contract.schema.json": source_sampling_contract,
        "source-sampling-contract-summary.schema.json": source_sampling_contract_summary,
        "camera-texture-lane-contract.schema.json": camera_texture_lane_contract,
        "camera-texture-lane-contract-summary.schema.json": camera_texture_lane_contract_summary,
        "camera-texture-lane-suite-summary.schema.json": camera_texture_lane_suite_summary,
        "camera-perfetto-trace-plan.schema.json": camera_perfetto_trace_plan,
        "projection-runtime-readback.schema.json": projection_runtime_readback,
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
        "quest-openxr-gles-feasibility-status.schema.json": openxr_gles_feasibility_status,
        "quest-surface-texture-oes-ingest-status.schema.json": surface_texture_oes_ingest_status,
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
        "mesh-fixture-manifest.schema.json": mesh_fixture_manifest,
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
        "broker-control-scope.schema.json": broker_control_scope,
        "broker-command-precondition.schema.json": broker_command_precondition,
        "broker-command-authority-requirement.schema.json": broker_command_authority_requirement,
        "broker-control-lease.schema.json": broker_control_lease,
        "broker-panel-descriptor.schema.json": broker_panel_descriptor,
        "broker-panel-descriptor-document.schema.json": broker_panel_descriptor_document,
        "broker-telemetry-chart-descriptor.schema.json": broker_telemetry_chart_descriptor,
        "broker-stream-registry-snapshot.schema.json": broker_stream_registry_snapshot,
        "broker-stream-manifest.schema.json": broker_stream_manifest,
        "broker-stream-sample-header.schema.json": broker_sample_header,
        "broker-transport-security-policy.schema.json": broker_transport_security_policy,
        "broker-transport-session-offer.schema.json": broker_transport_session_offer,
        "broker-transport-session-answer.schema.json": broker_transport_session_answer,
        "broker-zeromq-bridge-manifest.schema.json": broker_zeromq_bridge_manifest,
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
        "home-kiosk-command-run-record.schema.json": kiosk_command_run_record,
        "home-focus-recovery-event.schema.json": home_focus_recovery_event,
        "effect-stack-descriptor.schema.json": effect_stack_descriptor,
        "effect-stack-comparison-report.schema.json": effect_stack_comparison_report,
        "projection-performance-matrix.schema.json": projection_performance_matrix_packet,
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
