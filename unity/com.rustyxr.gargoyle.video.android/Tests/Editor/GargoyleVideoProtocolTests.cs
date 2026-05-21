using NUnit.Framework;

namespace RustyXr.Gargoyle.Video.Tests
{
    public sealed class GargoyleVideoProtocolTests
    {
        [Test]
        public void BuildStartSyntheticH264ParamsUsesBrokerVideoKeys()
        {
            var options = new GargoyleVideoStreamOptions
            {
                SessionId = "session-001",
                DevicePort = 8879,
                HostPort = 8879,
                PreferredWidth = 1280,
                PreferredHeight = 720,
                ContentWidth = 1280,
                ContentHeight = 720,
                BitrateBps = 6000000,
                FrameRateHz = 30,
                LiveStream = true,
                CaptureMs = 0,
                MaxPackets = 0,
                SyntheticPattern = "diagnostic-grid",
                SyntheticProjectionProfile = "head-anchored"
            };

            var json = GargoyleVideoProtocol.BuildStartSyntheticH264Params(options);

            StringAssert.Contains("\"session_id\":\"session-001\"", json);
            StringAssert.Contains("\"device_port\":8879", json);
            StringAssert.Contains("\"preferred_width\":1280", json);
            StringAssert.Contains("\"bitrate_bps\":6000000", json);
            StringAssert.Contains("\"live_stream\":true", json);
            StringAssert.Contains("\"synthetic_pattern\":\"diagnostic-grid\"", json);
        }

        [Test]
        public void BuildBitrateParamsCanRequestKeyframe()
        {
            var json = GargoyleVideoProtocol.BuildBitrateParams(1000000, "session-001", "broker_app.synthetic_h264");

            StringAssert.Contains("\"bitrate_bps\":1000000", json);
            StringAssert.Contains("\"session_id\":\"session-001\"", json);
            StringAssert.Contains("\"stream_id\":\"broker_app.synthetic_h264\"", json);
            StringAssert.Contains("\"request_keyframe\":true", json);
        }
    }
}
