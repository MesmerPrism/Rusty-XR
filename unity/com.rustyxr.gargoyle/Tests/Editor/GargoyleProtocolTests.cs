using NUnit.Framework;

namespace RustyXr.Gargoyle.Tests
{
    public sealed class GargoyleProtocolTests
    {
        [Test]
        public void BuildSubscribeCommandJsonUsesBrokerEnvelope()
        {
            var identity = new GargoyleClientIdentity(
                "test-client",
                "org.example.test",
                "Test App",
                "0.1.0");

            var json = GargoyleProtocol.BuildSubscribeCommandJson("req-1", identity, "synthetic:wave");

            StringAssert.Contains("\"type\":\"command\"", json);
            StringAssert.Contains("\"schema\":\"rusty.xr.broker.command.v1\"", json);
            StringAssert.Contains("\"request_id\":\"req-1\"", json);
            StringAssert.Contains("\"command\":\"subscribe\"", json);
            StringAssert.Contains("\"stream\":\"synthetic:wave\"", json);
        }

        [Test]
        public void TryParseStreamEventNormalizesHeaderFields()
        {
            const string json =
                "{\"type\":\"stream_event\",\"schema\":\"rusty.xr.broker.stream_event.v1\",\"header\":{\"schema\":\"rusty.xr.broker.stream_sample_header.v1\",\"stream_id\":\"synthetic:wave\",\"payload_schema\":\"rusty.xr.synthetic.wave.v1\",\"sequence_number\":7,\"broker_time_elapsed_ns\":1234},\"payload\":{\"schema\":\"rusty.xr.synthetic.wave.v1\",\"value01\":0.5}}";

            Assert.That(GargoyleProtocol.TryParseStreamEvent(json, out var streamEvent), Is.True);
            Assert.That(streamEvent.stream, Is.EqualTo("synthetic:wave"));
            Assert.That(streamEvent.sequence_id, Is.EqualTo(7));
            Assert.That(streamEvent.payload_schema, Is.EqualTo("rusty.xr.synthetic.wave.v1"));
            Assert.That(streamEvent.payload.value01, Is.EqualTo(0.5f));
        }

        [Test]
        public void BuildParamsJsonEscapesStrings()
        {
            var json = GargoyleProtocol.BuildParamsJson(
                GargoyleJsonField.String("stream", "line\nquote\"slash\\"),
                GargoyleJsonField.Number("port", 8879),
                GargoyleJsonField.Boolean("live_stream", true));

            StringAssert.Contains("\"stream\":\"line\\nquote\\\"slash\\\\\"", json);
            StringAssert.Contains("\"port\":8879", json);
            StringAssert.Contains("\"live_stream\":true", json);
        }
    }
}
