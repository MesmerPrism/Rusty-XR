using System;
using System.Collections.Generic;
using System.IO;
using System.Net.WebSockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace RustyXr.Gargoyle
{
    [DisallowMultipleComponent]
    [DefaultExecutionOrder(-18)]
    public sealed class GargoyleClient : MonoBehaviour
    {
        [SerializeField] GargoyleConfig config;
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

        readonly object _queueGate = new();
        readonly object _socketGate = new();
        readonly Queue<string> _incomingMessages = new();
        readonly SemaphoreSlim _sendGate = new(1, 1);

        CancellationTokenSource _loopCts;
        Task _loopTask;
        ClientWebSocket _socket;
        int _nextRequestId;
        long _sentMessages;
        long _receivedMessages;
        long _streamEvents;
        long _acceptedAcks;
        long _rejectedAcks;
        string _lastMessage = "";
        string _lastError = "";
        string _lastStatus = "disconnected";
        GargoyleConnectionState _state = GargoyleConnectionState.Disconnected;

        public event Action<string> MessageReceived;
        public event Action<GargoyleCommandAck> CommandAckReceived;
        public event Action<GargoyleStreamEvent, string> StreamEventReceived;

        public GargoyleConnectionState State => _state;
        public bool IsConnected => _state == GargoyleConnectionState.Connected;
        public long SentMessages => Interlocked.Read(ref _sentMessages);
        public long ReceivedMessages => Interlocked.Read(ref _receivedMessages);
        public long StreamEvents => Interlocked.Read(ref _streamEvents);
        public long AcceptedAcks => Interlocked.Read(ref _acceptedAcks);
        public long RejectedAcks => Interlocked.Read(ref _rejectedAcks);
        public string LastMessage => _lastMessage;
        public string LastError => _lastError;
        public string LastStatus => _lastStatus;

        void Awake()
        {
            ApplyConfig();
        }

        void OnEnable()
        {
            ApplyConfig();
            if (connectOnEnable)
            {
                ConnectNow();
            }
        }

        void Update()
        {
            DrainIncomingMessages();
        }

        void OnDisable()
        {
            DisconnectNow();
        }

        void OnDestroy()
        {
            DisconnectNow();
            _sendGate.Dispose();
        }

        public void ConfigureIdentity(string packageName, string label, string version)
        {
            appPackage = string.IsNullOrWhiteSpace(packageName) ? appPackage : packageName;
            appLabel = string.IsNullOrWhiteSpace(label) ? appLabel : label;
            appVersion = string.IsNullOrWhiteSpace(version) ? appVersion : version;
        }

        public void ConfigureDefaultStreams(params string[] streams)
        {
            if (streams == null || streams.Length == 0)
            {
                defaultStreams = Array.Empty<string>();
                return;
            }

            var sanitized = new List<string>(streams.Length);
            for (var i = 0; i < streams.Length; i++)
            {
                var stream = streams[i];
                if (string.IsNullOrWhiteSpace(stream) || sanitized.Contains(stream))
                {
                    continue;
                }

                sanitized.Add(stream);
            }

            defaultStreams = sanitized.ToArray();
        }

        public void ConnectNow()
        {
            if (_loopTask != null && !_loopTask.IsCompleted)
            {
                return;
            }

            _loopCts?.Cancel();
            _loopCts?.Dispose();
            _loopCts = new CancellationTokenSource();
            _loopTask = Task.Run(() => ConnectLoopAsync(_loopCts.Token));
        }

        public void DisconnectNow()
        {
            _loopCts?.Cancel();
            ClientWebSocket socket;
            lock (_socketGate)
            {
                socket = _socket;
                _socket = null;
            }

            try
            {
                socket?.Abort();
                socket?.Dispose();
            }
            catch (ObjectDisposedException)
            {
            }

            SetState(GargoyleConnectionState.Disconnected, "disconnected");
        }

        public string SendStatusRequest() => SendCommand("status_request");

        public string ListStreams() => SendCommand("list_streams");

        public string ListCapabilities() => SendCommand("list_capabilities");

        public string OpenGargoyleUi() => SendCommand("open_ui");

        public string CloseGargoyleUi() => SendCommand("close_ui");

        public string Subscribe(string stream)
        {
            if (string.IsNullOrWhiteSpace(stream))
            {
                return "";
            }

            return SendCommand("subscribe", GargoyleProtocol.BuildParamsJson(GargoyleJsonField.String("stream", stream)));
        }

        public string Unsubscribe(string stream)
        {
            if (string.IsNullOrWhiteSpace(stream))
            {
                return "";
            }

            return SendCommand("unsubscribe", GargoyleProtocol.BuildParamsJson(GargoyleJsonField.String("stream", stream)));
        }

        public string SendCommand(string command, string paramsJson = null)
        {
            if (string.IsNullOrWhiteSpace(command))
            {
                return "";
            }

            var requestId = NextRequestId(command);
            var identity = BuildIdentity();
            var json = GargoyleProtocol.BuildCommandJson(command, requestId, identity, paramsJson);
            _ = SendTextAsync(json, _loopCts != null ? _loopCts.Token : CancellationToken.None);
            return requestId;
        }

        void ApplyConfig()
        {
            if (config == null)
            {
                websocketUri = string.IsNullOrWhiteSpace(websocketUri) ? GargoyleProtocol.DefaultWebSocketUri : websocketUri;
                return;
            }

            websocketUri = config.WebSocketUri;
            clientId = string.IsNullOrWhiteSpace(config.ClientId) ? clientId : config.ClientId;
            appPackage = string.IsNullOrWhiteSpace(config.AppPackage) ? appPackage : config.AppPackage;
            appLabel = string.IsNullOrWhiteSpace(config.AppLabel) ? appLabel : config.AppLabel;
            appVersion = string.IsNullOrWhiteSpace(config.AppVersion) ? appVersion : config.AppVersion;
            connectOnEnable = config.ConnectOnEnable;
            subscribeOnConnect = config.SubscribeOnConnect;
            defaultStreams = config.DefaultStreams;
            reconnectDelaySeconds = config.ReconnectDelaySeconds;
            reconnectMaxDelaySeconds = config.ReconnectMaxDelaySeconds;
            receiveBufferBytes = config.ReceiveBufferBytes;
            maxMessagesPerFrame = config.MaxMessagesPerFrame;
        }

        GargoyleClientIdentity BuildIdentity()
        {
            return new GargoyleClientIdentity(clientId, appPackage, appLabel, appVersion);
        }

        async Task ConnectLoopAsync(CancellationToken token)
        {
            var delay = Mathf.Max(0.1f, reconnectDelaySeconds);
            while (!token.IsCancellationRequested)
            {
                SetState(GargoyleConnectionState.Connecting, "connecting");
                ClientWebSocket socket = null;
                try
                {
                    socket = new ClientWebSocket();
                    await socket.ConnectAsync(new Uri(websocketUri), token).ConfigureAwait(false);
                    lock (_socketGate)
                    {
                        _socket = socket;
                    }

                    SetState(GargoyleConnectionState.Connected, "connected");
                    delay = Mathf.Max(0.1f, reconnectDelaySeconds);

                    var identity = BuildIdentity();
                    await SendOnSocketAsync(socket, GargoyleProtocol.BuildHelloJson(identity), token).ConfigureAwait(false);
                    await SendOnSocketAsync(socket, GargoyleProtocol.BuildStatusRequestCommandJson(NextRequestId("status"), identity), token).ConfigureAwait(false);
                    if (subscribeOnConnect && defaultStreams != null)
                    {
                        for (var i = 0; i < defaultStreams.Length; i++)
                        {
                            var stream = defaultStreams[i];
                            if (!string.IsNullOrWhiteSpace(stream))
                            {
                                await SendOnSocketAsync(
                                    socket,
                                    GargoyleProtocol.BuildSubscribeCommandJson(NextRequestId("subscribe"), identity, stream),
                                    token).ConfigureAwait(false);
                            }
                        }
                    }

                    await ReceiveLoopAsync(socket, token).ConfigureAwait(false);
                }
                catch (Exception ex) when (ex is WebSocketException || ex is IOException || ex is InvalidOperationException || ex is UriFormatException || ex is OperationCanceledException)
                {
                    if (!token.IsCancellationRequested)
                    {
                        _lastError = ex.Message;
                    }
                }
                finally
                {
                    lock (_socketGate)
                    {
                        if (_socket == socket)
                        {
                            _socket = null;
                        }
                    }

                    socket?.Dispose();
                }

                if (token.IsCancellationRequested)
                {
                    break;
                }

                SetState(GargoyleConnectionState.WaitingToReconnect, "waiting to reconnect");
                try
                {
                    await Task.Delay(TimeSpan.FromSeconds(delay), token).ConfigureAwait(false);
                }
                catch (OperationCanceledException)
                {
                    break;
                }

                delay = Mathf.Min(Mathf.Max(delay * 1.6f, reconnectDelaySeconds), reconnectMaxDelaySeconds);
            }

            SetState(GargoyleConnectionState.Disconnected, "disconnected");
        }

        async Task ReceiveLoopAsync(ClientWebSocket socket, CancellationToken token)
        {
            var buffer = new byte[Mathf.Max(1024, receiveBufferBytes)];
            while (!token.IsCancellationRequested && socket.State == WebSocketState.Open)
            {
                using var message = new MemoryStream();
                WebSocketReceiveResult result;
                do
                {
                    result = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), token).ConfigureAwait(false);
                    if (result.MessageType == WebSocketMessageType.Close)
                    {
                        return;
                    }

                    message.Write(buffer, 0, result.Count);
                }
                while (!result.EndOfMessage);

                if (result.MessageType != WebSocketMessageType.Text)
                {
                    continue;
                }

                var json = Encoding.UTF8.GetString(message.ToArray());
                lock (_queueGate)
                {
                    _incomingMessages.Enqueue(json);
                }
            }
        }

        async Task SendTextAsync(string json, CancellationToken token)
        {
            ClientWebSocket socket;
            lock (_socketGate)
            {
                socket = _socket;
            }

            if (socket == null || socket.State != WebSocketState.Open)
            {
                _lastError = "Gargoyle WebSocket is not open.";
                return;
            }

            await SendOnSocketAsync(socket, json, token).ConfigureAwait(false);
        }

        async Task SendOnSocketAsync(ClientWebSocket socket, string json, CancellationToken token)
        {
            var bytes = Encoding.UTF8.GetBytes(json);
            await _sendGate.WaitAsync(token).ConfigureAwait(false);
            try
            {
                await socket.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, token).ConfigureAwait(false);
                Interlocked.Increment(ref _sentMessages);
            }
            finally
            {
                _sendGate.Release();
            }
        }

        void DrainIncomingMessages()
        {
            for (var i = 0; i < maxMessagesPerFrame; i++)
            {
                string json = null;
                lock (_queueGate)
                {
                    if (_incomingMessages.Count > 0)
                    {
                        json = _incomingMessages.Dequeue();
                    }
                }

                if (json == null)
                {
                    return;
                }

                _lastMessage = json;
                Interlocked.Increment(ref _receivedMessages);
                MessageReceived?.Invoke(json);

                if (GargoyleProtocol.TryParseCommandAck(json, out var ack))
                {
                    if (ack.accepted)
                    {
                        Interlocked.Increment(ref _acceptedAcks);
                    }
                    else
                    {
                        Interlocked.Increment(ref _rejectedAcks);
                    }

                    CommandAckReceived?.Invoke(ack);
                    continue;
                }

                if (GargoyleProtocol.TryParseStreamEvent(json, out var streamEvent) ||
                    GargoyleProtocol.TryParseReplayRecord(json, out streamEvent))
                {
                    Interlocked.Increment(ref _streamEvents);
                    StreamEventReceived?.Invoke(streamEvent, json);
                }
            }
        }

        string NextRequestId(string prefix)
        {
            var index = Interlocked.Increment(ref _nextRequestId);
            var safePrefix = string.IsNullOrWhiteSpace(prefix) ? "request" : prefix.Trim();
            return $"{safePrefix}-{index}";
        }

        void SetState(GargoyleConnectionState state, string status)
        {
            _state = state;
            _lastStatus = status ?? "";
        }
    }
}
