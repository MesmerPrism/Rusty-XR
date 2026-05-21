using UnityEngine;

namespace RustyXr.Gargoyle
{
    public interface IGargoyleStreamReceiver
    {
        bool CanReceive(GargoyleStreamEvent streamEvent);
        bool Receive(GargoyleStreamEvent streamEvent, string rawJson);
    }

    public abstract class GargoyleStreamReceiver : MonoBehaviour, IGargoyleStreamReceiver
    {
        [SerializeField] string streamId = "";
        [SerializeField] string payloadSchema = "";

        public string StreamId
        {
            get => streamId;
            set => streamId = value;
        }

        public string PayloadSchema
        {
            get => payloadSchema;
            set => payloadSchema = value;
        }

        public virtual bool CanReceive(GargoyleStreamEvent streamEvent)
        {
            if (streamEvent == null)
            {
                return false;
            }

            if (!string.IsNullOrWhiteSpace(streamId) &&
                !string.Equals(streamEvent.stream, streamId, System.StringComparison.Ordinal))
            {
                return false;
            }

            if (!string.IsNullOrWhiteSpace(payloadSchema) &&
                !string.Equals(streamEvent.payload_schema, payloadSchema, System.StringComparison.Ordinal))
            {
                return false;
            }

            return true;
        }

        public bool Receive(GargoyleStreamEvent streamEvent, string rawJson)
        {
            if (!CanReceive(streamEvent))
            {
                return false;
            }

            OnReceiveStreamEvent(streamEvent, rawJson);
            return true;
        }

        protected abstract void OnReceiveStreamEvent(GargoyleStreamEvent streamEvent, string rawJson);
    }
}
