using UnityEngine;

namespace RustyXr.Gargoyle.Samples
{
    public sealed class GargoyleConnectionExample : MonoBehaviour
    {
        [SerializeField] GargoyleClient client;
        [SerializeField] string[] streams = { "synthetic:wave" };

        void Reset()
        {
            client = GetComponent<GargoyleClient>();
        }

        void Awake()
        {
            if (client == null)
            {
                client = GetComponent<GargoyleClient>();
            }
        }

        void OnEnable()
        {
            if (client == null)
            {
                return;
            }

            client.CommandAckReceived += HandleCommandAck;
            client.StreamEventReceived += HandleStreamEvent;
        }

        void OnDisable()
        {
            if (client == null)
            {
                return;
            }

            client.CommandAckReceived -= HandleCommandAck;
            client.StreamEventReceived -= HandleStreamEvent;
        }

        public void SubscribeConfiguredStreams()
        {
            if (client == null || streams == null)
            {
                return;
            }

            for (var i = 0; i < streams.Length; i++)
            {
                client.Subscribe(streams[i]);
            }
        }

        void HandleCommandAck(GargoyleCommandAck ack)
        {
            Debug.Log($"[GargoyleConnectionExample] command={ack.command} accepted={ack.accepted} message={ack.message}", this);
        }

        void HandleStreamEvent(GargoyleStreamEvent streamEvent, string rawJson)
        {
            Debug.Log($"[GargoyleConnectionExample] stream={streamEvent.stream} sequence={streamEvent.sequence_id}", this);
        }
    }
}
