using System;
using System.Collections.Generic;
using RustyXr.Gargoyle;
using UnityEngine;

namespace RustyXr.Gargoyle.Video
{
    [Serializable]
    public sealed class GargoyleVideoStreamOptions
    {
        [SerializeField] string sessionId = "";
        [SerializeField, Range(1, 65535)] int devicePort = 8879;
        [SerializeField, Range(1, 65535)] int hostPort = 8879;
        [SerializeField, Range(16, 4096)] int preferredWidth = 1280;
        [SerializeField, Range(16, 4096)] int preferredHeight = 720;
        [SerializeField, Range(16, 4096)] int contentWidth = 1280;
        [SerializeField, Range(16, 4096)] int contentHeight = 720;
        [SerializeField, Min(0)] int captureMs = 10000;
        [SerializeField, Min(0)] int maxPackets = 300;
        [SerializeField, Range(1, 1024)] int writerQueueDepth = 32;
        [SerializeField, Min(0)] int acceptTimeoutMs = 5000;
        [SerializeField, Range(100000, 20000000)] int bitrateBps = 6000000;
        [SerializeField, Range(1, 120)] int frameRateHz = 30;
        [SerializeField] bool liveStream;
        [SerializeField] bool lanStreamEnabled;
        [SerializeField] string bindHost = "";
        [SerializeField] string advertisedHost = "";
        [SerializeField] string cameraId = "";
        [SerializeField] string syntheticPattern = "diagnostic-grid";
        [SerializeField] string syntheticSideMarker = "";
        [SerializeField] string syntheticProjectionProfile = "head-anchored";

        public string SessionId { get => sessionId; set => sessionId = value; }
        public int DevicePort { get => devicePort; set => devicePort = Mathf.Clamp(value, 1, 65535); }
        public int HostPort { get => hostPort; set => hostPort = Mathf.Clamp(value, 1, 65535); }
        public int PreferredWidth { get => preferredWidth; set => preferredWidth = Mathf.Clamp(value, 16, 4096); }
        public int PreferredHeight { get => preferredHeight; set => preferredHeight = Mathf.Clamp(value, 16, 4096); }
        public int ContentWidth { get => contentWidth; set => contentWidth = Mathf.Clamp(value, 16, 4096); }
        public int ContentHeight { get => contentHeight; set => contentHeight = Mathf.Clamp(value, 16, 4096); }
        public int CaptureMs { get => captureMs; set => captureMs = Mathf.Max(0, value); }
        public int MaxPackets { get => maxPackets; set => maxPackets = Mathf.Max(0, value); }
        public int WriterQueueDepth { get => writerQueueDepth; set => writerQueueDepth = Mathf.Clamp(value, 1, 1024); }
        public int AcceptTimeoutMs { get => acceptTimeoutMs; set => acceptTimeoutMs = Mathf.Max(0, value); }
        public int BitrateBps { get => bitrateBps; set => bitrateBps = Mathf.Clamp(value, 100000, 20000000); }
        public int FrameRateHz { get => frameRateHz; set => frameRateHz = Mathf.Clamp(value, 1, 120); }
        public bool LiveStream { get => liveStream; set => liveStream = value; }
        public bool LanStreamEnabled { get => lanStreamEnabled; set => lanStreamEnabled = value; }
        public string BindHost { get => bindHost; set => bindHost = value; }
        public string AdvertisedHost { get => advertisedHost; set => advertisedHost = value; }
        public string CameraId { get => cameraId; set => cameraId = value; }
        public string SyntheticPattern { get => syntheticPattern; set => syntheticPattern = value; }
        public string SyntheticSideMarker { get => syntheticSideMarker; set => syntheticSideMarker = value; }
        public string SyntheticProjectionProfile { get => syntheticProjectionProfile; set => syntheticProjectionProfile = value; }

        public string ToParamsJson(bool synthetic)
        {
            var fields = GargoyleVideoProtocol.BaseStreamFields(this);
            if (synthetic)
            {
                fields.Add(GargoyleJsonField.String("synthetic_pattern", syntheticPattern));
                fields.Add(GargoyleJsonField.OptionalString("synthetic_side_marker", syntheticSideMarker));
                fields.Add(GargoyleJsonField.String("synthetic_projection_profile", syntheticProjectionProfile));
            }
            else
            {
                fields.Add(GargoyleJsonField.OptionalString("camera_id", cameraId));
            }

            return GargoyleProtocol.BuildParamsJson((IEnumerable<GargoyleJsonField>)fields);
        }
    }
}
