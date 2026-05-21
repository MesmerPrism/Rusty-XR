using RustyXr.Gargoyle;
using UnityEngine;

namespace RustyXr.Gargoyle.Video
{
    [DisallowMultipleComponent]
    public sealed class GargoyleVideoController : MonoBehaviour
    {
        [SerializeField] GargoyleClient client;
        [SerializeField] bool subscribeTelemetryOnEnable = true;
        [SerializeField] GargoyleVideoStreamOptions defaultStreamOptions = new();

        public GargoyleVideoStreamOptions DefaultStreamOptions => defaultStreamOptions;

        void Reset()
        {
            client = GetComponent<GargoyleClient>();
        }

        void OnEnable()
        {
            ResolveClient();
            if (subscribeTelemetryOnEnable)
            {
                SubscribeVideoTelemetry();
            }
        }

        public void ConfigureClient(GargoyleClient gargoyleClient)
        {
            client = gargoyleClient;
        }

        public void SubscribeVideoTelemetry()
        {
            ResolveClient();
            if (client == null)
            {
                return;
            }

            client.Subscribe(GargoyleVideoProtocol.EncodedStreamManifestStream);
            client.Subscribe(GargoyleVideoProtocol.EncodedSampleMetadataStream);
            client.Subscribe(GargoyleVideoProtocol.MetricSampleStream);
        }

        public string GetVideoLabStatus()
        {
            ResolveClient();
            return client != null ? client.SendCommand(GargoyleVideoProtocol.GetVideoLabStatusCommand) : "";
        }

        public string GetVideoLabScorecard()
        {
            ResolveClient();
            return client != null ? client.SendCommand(GargoyleVideoProtocol.GetVideoLabScorecardCommand) : "";
        }

        public string StartSyntheticH264Stream()
        {
            return StartSyntheticH264Stream(defaultStreamOptions);
        }

        public string StartSyntheticH264Stream(GargoyleVideoStreamOptions options)
        {
            ResolveClient();
            return client != null
                ? client.SendCommand(
                    GargoyleVideoProtocol.StartSyntheticH264StreamCommand,
                    GargoyleVideoProtocol.BuildStartSyntheticH264Params(options))
                : "";
        }

        public string StartAppCameraH264Stream()
        {
            return StartAppCameraH264Stream(defaultStreamOptions);
        }

        public string StartAppCameraH264Stream(GargoyleVideoStreamOptions options)
        {
            ResolveClient();
            return client != null
                ? client.SendCommand(
                    GargoyleVideoProtocol.StartAppCameraH264StreamCommand,
                    GargoyleVideoProtocol.BuildStartAppCameraH264Params(options))
                : "";
        }

        public string RequestKeyframe(string sessionId = "", string streamId = "")
        {
            ResolveClient();
            return client != null
                ? client.SendCommand(
                    GargoyleVideoProtocol.RequestKeyframeCommand,
                    GargoyleVideoProtocol.BuildKeyframeParams(sessionId, streamId))
                : "";
        }

        public string SetVideoBitrate(int bitrateBps, string sessionId = "", string streamId = "", bool requestKeyframe = true)
        {
            ResolveClient();
            return client != null
                ? client.SendCommand(
                    GargoyleVideoProtocol.SetVideoBitrateCommand,
                    GargoyleVideoProtocol.BuildBitrateParams(bitrateBps, sessionId, streamId, requestKeyframe))
                : "";
        }

        public string SetQualityProfile(string qualityProfile, string sessionId = "", string streamId = "", bool requestKeyframe = true)
        {
            ResolveClient();
            return client != null
                ? client.SendCommand(
                    GargoyleVideoProtocol.SetQualityProfileCommand,
                    GargoyleVideoProtocol.BuildQualityProfileParams(qualityProfile, sessionId, streamId, requestKeyframe))
                : "";
        }

        void ResolveClient()
        {
            if (client == null)
            {
                client = GetComponent<GargoyleClient>();
            }

            if (client == null)
            {
                client = FindAnyObjectByType<GargoyleClient>();
            }
        }
    }
}
