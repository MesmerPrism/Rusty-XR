using System;
using UnityEngine;

namespace RustyXr.Gargoyle
{
    public readonly struct GargoyleClientIdentity
    {
        public GargoyleClientIdentity(string clientId, string appPackage, string appLabel, string appVersion)
        {
            ClientId = string.IsNullOrWhiteSpace(clientId) ? "unity-client" : clientId.Trim();
            AppPackage = string.IsNullOrWhiteSpace(appPackage) ? Application.identifier : appPackage.Trim();
            AppLabel = string.IsNullOrWhiteSpace(appLabel) ? Application.productName : appLabel.Trim();
            AppVersion = string.IsNullOrWhiteSpace(appVersion) ? Application.version : appVersion.Trim();
        }

        public string ClientId { get; }
        public string AppPackage { get; }
        public string AppLabel { get; }
        public string AppVersion { get; }
    }

    [Serializable]
    public sealed class GargoyleHelloEnvelope
    {
        public string type;
        public string schema;
        public string client_id;
        public string app_package;
        public string app_label;
        public string app_version;
        public int protocol_min;
        public int protocol_max;
        public bool supports_commands;
    }

    [Serializable]
    public sealed class GargoyleCommandAck
    {
        public string type;
        public string schema;
        public string request_id;
        public string command;
        public bool accepted;
        public string message;
        public GargoyleCommandAckResult result;
        public GargoyleCommandError error;
    }

    [Serializable]
    public sealed class GargoyleCommandAckResult
    {
        public string stream;
        public string subscription_id;
        public string status;
    }

    [Serializable]
    public sealed class GargoyleCommandError
    {
        public string code;
        public string message;
    }

    [Serializable]
    public sealed class GargoyleReplayRecordEnvelope
    {
        public string type;
        public string schema;
        public string session_id;
        public string stream;
        public GargoyleStreamSampleHeader header;
        public GargoyleStreamPayload payload;
    }

    [Serializable]
    public sealed class GargoyleStreamEvent
    {
        public string type;
        public string schema;
        public string stream;
        public string subscription_id;
        public GargoyleStreamSampleHeader header;
        public long sequence_id;
        public long broker_time_unix_ns;
        public long broker_time_elapsed_ns;
        public long source_time_ns;
        public long source_time_unix_ns;
        public long dropped_before_sample;
        public long late_before_sample;
        public string payload_schema;
        public GargoyleStreamPayload payload;

        public bool NormalizeFromHeader()
        {
            if (header != null)
            {
                if (string.IsNullOrWhiteSpace(stream))
                {
                    stream = header.stream_id;
                }

                if (sequence_id == 0L)
                {
                    sequence_id = header.sequence_number;
                }

                if (broker_time_unix_ns == 0L)
                {
                    broker_time_unix_ns = header.broker_time_unix_ns;
                }

                if (broker_time_elapsed_ns == 0L)
                {
                    broker_time_elapsed_ns = header.broker_time_elapsed_ns;
                }

                if (source_time_ns == 0L)
                {
                    source_time_ns = header.source_time_ns;
                }

                if (source_time_unix_ns == 0L)
                {
                    source_time_unix_ns = header.source_time_unix_ns;
                }

                if (dropped_before_sample == 0L)
                {
                    dropped_before_sample = header.dropped_before_sample;
                }

                if (late_before_sample == 0L)
                {
                    late_before_sample = header.late_before_sample;
                }

                if (string.IsNullOrWhiteSpace(payload_schema))
                {
                    payload_schema = header.payload_schema;
                }
            }

            if (payload != null && string.IsNullOrWhiteSpace(payload_schema))
            {
                payload_schema = payload.schema;
            }

            return !string.IsNullOrWhiteSpace(stream);
        }
    }

    [Serializable]
    public sealed class GargoyleStreamSampleHeader
    {
        public string schema;
        public string stream_id;
        public string session_id;
        public string source_id;
        public string payload_kind;
        public string payload_schema;
        public long sequence_number;
        public long broker_time_elapsed_ns;
        public long broker_time_unix_ns;
        public long source_time_ns;
        public long source_time_unix_ns;
        public long dropped_before_sample;
        public long late_before_sample;
    }

    [Serializable]
    public sealed class GargoyleStreamPayload
    {
        public string schema;
        public string stream_id;
        public string session_id;
        public string source_id;
        public string source;
        public string source_kind;
        public string source_mode;
        public string codec;
        public string mime_type;
        public string layout;
        public string transport;
        public string state;
        public string message;
        public string reason;
        public string role;
        public string direction;
        public string eye;
        public string source_eye;
        public string host;
        public string bind_host;
        public string advertised_host;
        public string path;
        public string camera_id;
        public string synthetic_pattern;
        public string projection_geometry_profile;
        public string synthetic_projection_profile;
        public int port;
        public int device_port;
        public int host_port;
        public int width;
        public int height;
        public int selected_width;
        public int selected_height;
        public int frame_rate_hz;
        public int bitrate_bps;
        public int codec_config_packet_count;
        public int keyframe_count;
        public int video_packet_count;
        public int packet_count;
        public int decoded_frame_count;
        public int dropped_samples;
        public int dropped_frames;
        public int writer_queue_dropped_video_packets;
        public long sequence_id;
        public long pts_us;
        public long capture_time_elapsed_ns;
        public long source_elapsed_ns;
        public long source_unix_ns;
        public long payload_bytes;
        public long accepted_metric_samples;
        public long accepted_encoded_stream_manifests;
        public long accepted_encoded_sample_metadata;
        public float value01;
        public GargoyleScreenPoint normalized_point;
        public GargoyleEyeSampleBase @base;
    }

    [Serializable]
    public sealed class GargoyleScreenPoint
    {
        public float x;
        public float y;
    }

    [Serializable]
    public sealed class GargoyleEyeSampleBase
    {
        public string provider_id;
        public string source_device_id;
        public long sequence_number;
        public long sample_time_ns;
        public long broker_receive_time_ns;
        public float confidence;
        public string eye;
        public string coordinate_space;
        public GargoyleEyeValidity validity;
    }

    [Serializable]
    public sealed class GargoyleEyeValidity
    {
        public bool sample_valid;
        public bool left_valid;
        public bool right_valid;
        public bool blink;
        public bool tracking_lost;
    }
}
