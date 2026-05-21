using RustyXr.Gargoyle;
using UnityEngine;

namespace RustyXr.Gargoyle.Video.Samples
{
    public sealed class GargoyleVideoTelemetryExample : MonoBehaviour
    {
        [SerializeField] GargoyleVideoController videoController;
        [SerializeField] GargoyleVideoTelemetryReceiver telemetryReceiver;

        void Reset()
        {
            videoController = GetComponent<GargoyleVideoController>();
            telemetryReceiver = GetComponent<GargoyleVideoTelemetryReceiver>();
        }

        void OnEnable()
        {
            if (telemetryReceiver != null)
            {
                telemetryReceiver.ManifestReceived += HandleManifest;
                telemetryReceiver.MetricSampleReceived += HandleMetric;
            }
        }

        void OnDisable()
        {
            if (telemetryReceiver != null)
            {
                telemetryReceiver.ManifestReceived -= HandleManifest;
                telemetryReceiver.MetricSampleReceived -= HandleMetric;
            }
        }

        public void RequestVideoStatus()
        {
            videoController?.GetVideoLabStatus();
        }

        public void StartSyntheticStream()
        {
            videoController?.StartSyntheticH264Stream();
        }

        void HandleManifest(GargoyleStreamPayload payload)
        {
            Debug.Log($"[GargoyleVideoTelemetryExample] manifest stream={payload.stream_id} codec={payload.codec} size={payload.width}x{payload.height}", this);
        }

        void HandleMetric(GargoyleStreamPayload payload)
        {
            Debug.Log($"[GargoyleVideoTelemetryExample] metric stream={payload.stream_id} packets={payload.video_packet_count} keyframes={payload.keyframe_count}", this);
        }
    }
}
