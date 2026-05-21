using UnityEngine;

namespace RustyXr.Gargoyle
{
    [CreateAssetMenu(menuName = "Rusty XR/Gargoyle Config", fileName = "GargoyleConfig")]
    public sealed class GargoyleConfig : ScriptableObject
    {
        [SerializeField] string websocketUri = GargoyleProtocol.DefaultWebSocketUri;
        [SerializeField] string clientId = "unity-client";
        [SerializeField] string appPackage = "";
        [SerializeField] string appLabel = "";
        [SerializeField] string appVersion = "";
        [SerializeField] bool connectOnEnable = true;
        [SerializeField] bool subscribeOnConnect = true;
        [SerializeField] string[] defaultStreams = { };
        [SerializeField, Min(0.1f)] float reconnectDelaySeconds = 1f;
        [SerializeField, Min(0.1f)] float reconnectMaxDelaySeconds = 8f;
        [SerializeField, Min(1024)] int receiveBufferBytes = 8192;
        [SerializeField, Min(1)] int maxMessagesPerFrame = 16;

        public string WebSocketUri => string.IsNullOrWhiteSpace(websocketUri)
            ? GargoyleProtocol.DefaultWebSocketUri
            : websocketUri;

        public string ClientId => clientId;
        public string AppPackage => appPackage;
        public string AppLabel => appLabel;
        public string AppVersion => appVersion;
        public bool ConnectOnEnable => connectOnEnable;
        public bool SubscribeOnConnect => subscribeOnConnect;
        public string[] DefaultStreams => defaultStreams ?? System.Array.Empty<string>();
        public float ReconnectDelaySeconds => reconnectDelaySeconds;
        public float ReconnectMaxDelaySeconds => reconnectMaxDelaySeconds;
        public int ReceiveBufferBytes => receiveBufferBytes;
        public int MaxMessagesPerFrame => maxMessagesPerFrame;
    }
}
