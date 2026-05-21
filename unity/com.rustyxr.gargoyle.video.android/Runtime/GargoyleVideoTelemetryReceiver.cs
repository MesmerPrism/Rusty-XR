using System;
using RustyXr.Gargoyle;
using UnityEngine;

namespace RustyXr.Gargoyle.Video
{
    public sealed class GargoyleVideoTelemetryReceiver : GargoyleStreamReceiver
    {
        public event Action<GargoyleStreamPayload> ManifestReceived;
        public event Action<GargoyleStreamPayload> SampleMetadataReceived;
        public event Action<GargoyleStreamPayload> MetricSampleReceived;

        GargoyleStreamPayload _lastManifest;
        GargoyleStreamPayload _lastSampleMetadata;
        GargoyleStreamPayload _lastMetricSample;
        string _lastRawJson = "";

        public GargoyleStreamPayload LastManifest => _lastManifest;
        public GargoyleStreamPayload LastSampleMetadata => _lastSampleMetadata;
        public GargoyleStreamPayload LastMetricSample => _lastMetricSample;
        public string LastRawJson => _lastRawJson;

        public override bool CanReceive(GargoyleStreamEvent streamEvent)
        {
            if (streamEvent == null)
            {
                return false;
            }

            return streamEvent.stream == GargoyleVideoProtocol.EncodedStreamManifestStream ||
                   streamEvent.stream == GargoyleVideoProtocol.EncodedSampleMetadataStream ||
                   streamEvent.stream == GargoyleVideoProtocol.MetricSampleStream;
        }

        protected override void OnReceiveStreamEvent(GargoyleStreamEvent streamEvent, string rawJson)
        {
            _lastRawJson = rawJson ?? "";
            var payload = streamEvent.payload;
            if (payload == null)
            {
                return;
            }

            switch (streamEvent.stream)
            {
                case GargoyleVideoProtocol.EncodedStreamManifestStream:
                    _lastManifest = payload;
                    ManifestReceived?.Invoke(payload);
                    break;
                case GargoyleVideoProtocol.EncodedSampleMetadataStream:
                    _lastSampleMetadata = payload;
                    SampleMetadataReceived?.Invoke(payload);
                    break;
                case GargoyleVideoProtocol.MetricSampleStream:
                    _lastMetricSample = payload;
                    MetricSampleReceived?.Invoke(payload);
                    break;
            }
        }
    }
}
