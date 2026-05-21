using System.Collections.Generic;
using RustyXr.Gargoyle;

namespace RustyXr.Gargoyle.Video
{
    public static class GargoyleVideoProtocol
    {
        public const string StartSyntheticH264StreamCommand = "media.start_synthetic_h264_stream";
        public const string StartAppCameraH264StreamCommand = "camera_provider.start_app_camera_h264_stream";
        public const string RequestKeyframeCommand = "media.request_keyframe";
        public const string SetVideoBitrateCommand = "media.set_video_bitrate";
        public const string SetQualityProfileCommand = "media.set_quality_profile";
        public const string GetVideoLabStatusCommand = "video_lab.get_status";
        public const string GetVideoLabScorecardCommand = "video_lab.get_scorecard";

        public const string EncodedStreamManifestStream = "video_lab.encoded_stream_manifest";
        public const string EncodedSampleMetadataStream = "video_lab.encoded_sample_metadata";
        public const string MetricSampleStream = "video_lab.metric_sample";

        public const string EncodedStreamManifestSchema = "rusty.xr.video_lab.encoded_stream_manifest.v1";
        public const string EncodedSampleMetadataSchema = "rusty.xr.video_lab.encoded_sample_metadata.v1";
        public const string MetricSampleSchema = "rusty.xr.video_lab.metric_sample.v1";

        public const string RxyRvidMagic = "RXYRVID1";
        public const int CodecH264 = 1;

        public static string BuildStartSyntheticH264Params(GargoyleVideoStreamOptions options)
        {
            return (options ?? new GargoyleVideoStreamOptions()).ToParamsJson(true);
        }

        public static string BuildStartAppCameraH264Params(GargoyleVideoStreamOptions options)
        {
            return (options ?? new GargoyleVideoStreamOptions()).ToParamsJson(false);
        }

        public static string BuildKeyframeParams(string sessionId = "", string streamId = "")
        {
            return GargoyleProtocol.BuildParamsJson(new[]
            {
                GargoyleJsonField.OptionalString("session_id", sessionId),
                GargoyleJsonField.OptionalString("stream_id", streamId)
            });
        }

        public static string BuildBitrateParams(int bitrateBps, string sessionId = "", string streamId = "", bool requestKeyframe = true)
        {
            return GargoyleProtocol.BuildParamsJson(new[]
            {
                GargoyleJsonField.Number("bitrate_bps", bitrateBps),
                GargoyleJsonField.OptionalString("session_id", sessionId),
                GargoyleJsonField.OptionalString("stream_id", streamId),
                GargoyleJsonField.Boolean("request_keyframe", requestKeyframe)
            });
        }

        public static string BuildQualityProfileParams(string qualityProfile, string sessionId = "", string streamId = "", bool requestKeyframe = true)
        {
            return GargoyleProtocol.BuildParamsJson(new[]
            {
                GargoyleJsonField.String("quality_profile", qualityProfile),
                GargoyleJsonField.OptionalString("session_id", sessionId),
                GargoyleJsonField.OptionalString("stream_id", streamId),
                GargoyleJsonField.Boolean("request_keyframe", requestKeyframe)
            });
        }

        internal static List<GargoyleJsonField> BaseStreamFields(GargoyleVideoStreamOptions options)
        {
            var fields = new List<GargoyleJsonField>
            {
                GargoyleJsonField.OptionalString("session_id", options.SessionId),
                GargoyleJsonField.Number("device_port", options.DevicePort),
                GargoyleJsonField.Number("host_port", options.HostPort),
                GargoyleJsonField.Number("preferred_width", options.PreferredWidth),
                GargoyleJsonField.Number("preferred_height", options.PreferredHeight),
                GargoyleJsonField.Number("content_width", options.ContentWidth),
                GargoyleJsonField.Number("content_height", options.ContentHeight),
                GargoyleJsonField.Number("capture_ms", options.CaptureMs),
                GargoyleJsonField.Number("max_packets", options.MaxPackets),
                GargoyleJsonField.Number("writer_queue_depth", options.WriterQueueDepth),
                GargoyleJsonField.Number("accept_timeout_ms", options.AcceptTimeoutMs),
                GargoyleJsonField.Number("bitrate_bps", options.BitrateBps),
                GargoyleJsonField.Number("frame_rate_hz", options.FrameRateHz),
                GargoyleJsonField.Boolean("live_stream", options.LiveStream),
                GargoyleJsonField.Boolean("lan_stream_enabled", options.LanStreamEnabled),
                GargoyleJsonField.OptionalString("bind_host", options.BindHost),
                GargoyleJsonField.OptionalString("advertised_host", options.AdvertisedHost)
            };

            return fields;
        }
    }
}
