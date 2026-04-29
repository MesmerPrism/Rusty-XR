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
                "has_confidence": {"type": "boolean"},
                "byte_len": integer(0),
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
