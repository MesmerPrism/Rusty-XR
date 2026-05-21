using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using UnityEngine;

namespace RustyXr.Gargoyle.Video
{
    public enum GargoyleRxyRvidReceiverState
    {
        Idle = 0,
        Connecting = 1,
        Receiving = 2,
        Stopped = 3,
        Failed = 4
    }

    [DisallowMultipleComponent]
    public sealed class GargoyleRxyRvidH264Receiver : MonoBehaviour
    {
        [SerializeField] string host = "127.0.0.1";
        [SerializeField, Range(1, 65535)] int port = 8879;
        [SerializeField] bool connectOnEnable;
        [SerializeField, Min(1024)] int maxPacketBytes = 8 * 1024 * 1024;
        [SerializeField, Min(0)] int maxHeaderMetadataBytes = 1024 * 1024;
        [SerializeField, Min(1)] int maxEventsPerFrame = 8;

        readonly object _eventGate = new();
        readonly Queue<RxyRvidQueuedEvent> _queuedEvents = new();

        CancellationTokenSource _cts;
        Task _receiveTask;
        TcpClient _tcpClient;
        GargoyleRxyRvidReceiverState _state = GargoyleRxyRvidReceiverState.Idle;
        string _lastError = "";
        long _packetCount;
        long _payloadBytes;
        GargoyleRxyRvidStreamHeader _lastHeader;
        GargoyleRxyRvidPacket _lastPacket;

        public event Action<GargoyleRxyRvidStreamHeader> HeaderReceived;
        public event Action<GargoyleRxyRvidPacket> PacketReceived;
        public event Action<string> ErrorReceived;

        public string Host { get => host; set => host = value; }
        public int Port { get => port; set => port = Mathf.Clamp(value, 1, 65535); }
        public GargoyleRxyRvidReceiverState State => _state;
        public string LastError => _lastError;
        public long PacketCount => Interlocked.Read(ref _packetCount);
        public long PayloadBytes => Interlocked.Read(ref _payloadBytes);
        public GargoyleRxyRvidStreamHeader LastHeader => _lastHeader;
        public GargoyleRxyRvidPacket LastPacket => _lastPacket;

        void OnEnable()
        {
            if (connectOnEnable)
            {
                StartReceiving();
            }
        }

        void Update()
        {
            DrainQueuedEvents();
        }

        void OnDisable()
        {
            StopReceiving();
        }

        void OnDestroy()
        {
            StopReceiving();
        }

        public void StartReceiving()
        {
            if (_receiveTask != null && !_receiveTask.IsCompleted)
            {
                return;
            }

            StopReceiving();
            _lastError = "";
            _lastHeader = null;
            _lastPacket = null;
            Interlocked.Exchange(ref _packetCount, 0L);
            Interlocked.Exchange(ref _payloadBytes, 0L);
            _cts = new CancellationTokenSource();
            _receiveTask = Task.Run(() => ReceiveLoop(_cts.Token));
        }

        public void StopReceiving()
        {
            _cts?.Cancel();
            try
            {
                _tcpClient?.Close();
            }
            catch (ObjectDisposedException)
            {
            }

            _tcpClient = null;
            if (_state != GargoyleRxyRvidReceiverState.Failed)
            {
                _state = GargoyleRxyRvidReceiverState.Stopped;
            }
        }

        void ReceiveLoop(CancellationToken token)
        {
            try
            {
                _state = GargoyleRxyRvidReceiverState.Connecting;
                using var tcp = new TcpClient();
                _tcpClient = tcp;
                tcp.Connect(host, port);
                using var stream = tcp.GetStream();
                _state = GargoyleRxyRvidReceiverState.Receiving;

                var header = GargoyleRxyRvidParser.ReadHeader(stream, maxHeaderMetadataBytes);
                Enqueue(RxyRvidQueuedEvent.ForHeader(header));

                while (!token.IsCancellationRequested)
                {
                    var packet = GargoyleRxyRvidParser.ReadPacket(stream, header.SchemaVersion, maxPacketBytes);
                    Enqueue(RxyRvidQueuedEvent.ForPacket(packet));
                }
            }
            catch (Exception ex) when (ex is IOException || ex is SocketException || ex is InvalidDataException || ex is EndOfStreamException || ex is ObjectDisposedException)
            {
                if (!token.IsCancellationRequested)
                {
                    _lastError = ex.Message;
                    _state = GargoyleRxyRvidReceiverState.Failed;
                    Enqueue(RxyRvidQueuedEvent.ForError(ex.Message));
                }
            }
            finally
            {
                if (!token.IsCancellationRequested && _state != GargoyleRxyRvidReceiverState.Failed)
                {
                    _state = GargoyleRxyRvidReceiverState.Stopped;
                }
            }
        }

        void Enqueue(RxyRvidQueuedEvent queuedEvent)
        {
            lock (_eventGate)
            {
                _queuedEvents.Enqueue(queuedEvent);
            }
        }

        void DrainQueuedEvents()
        {
            for (var i = 0; i < maxEventsPerFrame; i++)
            {
                RxyRvidQueuedEvent queuedEvent = null;
                lock (_eventGate)
                {
                    if (_queuedEvents.Count > 0)
                    {
                        queuedEvent = _queuedEvents.Dequeue();
                    }
                }

                if (queuedEvent == null)
                {
                    return;
                }

                if (queuedEvent.Header != null)
                {
                    _lastHeader = queuedEvent.Header;
                    HeaderReceived?.Invoke(queuedEvent.Header);
                }
                else if (queuedEvent.Packet != null)
                {
                    _lastPacket = queuedEvent.Packet;
                    Interlocked.Increment(ref _packetCount);
                    Interlocked.Add(ref _payloadBytes, queuedEvent.Packet.PayloadBytes);
                    PacketReceived?.Invoke(queuedEvent.Packet);
                }
                else if (!string.IsNullOrWhiteSpace(queuedEvent.Error))
                {
                    ErrorReceived?.Invoke(queuedEvent.Error);
                }
            }
        }

        sealed class RxyRvidQueuedEvent
        {
            public GargoyleRxyRvidStreamHeader Header;
            public GargoyleRxyRvidPacket Packet;
            public string Error;

            public static RxyRvidQueuedEvent ForHeader(GargoyleRxyRvidStreamHeader header) => new() { Header = header };
            public static RxyRvidQueuedEvent ForPacket(GargoyleRxyRvidPacket packet) => new() { Packet = packet };
            public static RxyRvidQueuedEvent ForError(string error) => new() { Error = error };
        }
    }

    public sealed class GargoyleRxyRvidStreamHeader
    {
        public GargoyleRxyRvidStreamHeader(
            int schemaVersion,
            int codecId,
            int width,
            int height,
            int declaredPacketCount,
            int headerMetadataBytes,
            string headerMetadataJson)
        {
            SchemaVersion = schemaVersion;
            CodecId = codecId;
            Width = width;
            Height = height;
            DeclaredPacketCount = declaredPacketCount;
            HeaderMetadataBytes = headerMetadataBytes;
            HeaderMetadataJson = headerMetadataJson ?? "";
        }

        public int SchemaVersion { get; }
        public int CodecId { get; }
        public int Width { get; }
        public int Height { get; }
        public int DeclaredPacketCount { get; }
        public int HeaderMetadataBytes { get; }
        public string HeaderMetadataJson { get; }
    }

    public sealed class GargoyleRxyRvidPacket
    {
        public const int BufferFlagKeyFrame = 1;
        public const int BufferFlagCodecConfig = 2;
        public const int BufferFlagEndOfStream = 4;

        public GargoyleRxyRvidPacket(long ptsUs, int flags, long sourceElapsedNs, long sourceUnixNs, byte[] payload)
        {
            PtsUs = ptsUs;
            Flags = flags;
            SourceElapsedNs = sourceElapsedNs;
            SourceUnixNs = sourceUnixNs;
            Payload = payload ?? Array.Empty<byte>();
        }

        public long PtsUs { get; }
        public int Flags { get; }
        public long SourceElapsedNs { get; }
        public long SourceUnixNs { get; }
        public byte[] Payload { get; }
        public int PayloadBytes => Payload.Length;
        public bool IsKeyFrame => (Flags & BufferFlagKeyFrame) != 0;
        public bool IsCodecConfig => (Flags & BufferFlagCodecConfig) != 0;
        public bool IsEndOfStream => (Flags & BufferFlagEndOfStream) != 0;
    }

    public static class GargoyleRxyRvidParser
    {
        public static GargoyleRxyRvidStreamHeader ReadHeader(Stream stream, int maxMetadataBytes)
        {
            var magic = Encoding.ASCII.GetString(ReadExact(stream, 8));
            if (magic != GargoyleVideoProtocol.RxyRvidMagic)
            {
                throw new InvalidDataException("Unexpected RXYRVID1 magic: " + magic);
            }

            var schemaVersion = ReadInt32BigEndian(stream);
            var codecId = ReadInt32BigEndian(stream);
            var width = ReadInt32BigEndian(stream);
            var height = ReadInt32BigEndian(stream);
            var declaredPacketCount = ReadInt32BigEndian(stream);
            var tailWord = ReadInt32BigEndian(stream);

            if (schemaVersion < 1 || schemaVersion > 3)
            {
                throw new InvalidDataException("Unsupported RXYRVID1 schema version: " + schemaVersion);
            }

            if (codecId != GargoyleVideoProtocol.CodecH264)
            {
                throw new InvalidDataException("Unsupported RXYRVID1 codec id: " + codecId);
            }

            if (width <= 0 || height <= 0)
            {
                throw new InvalidDataException("Invalid RXYRVID1 dimensions: " + width + "x" + height);
            }

            var metadataBytes = schemaVersion >= 3 ? tailWord : 0;
            if (metadataBytes < 0 || metadataBytes > maxMetadataBytes)
            {
                throw new InvalidDataException("RXYRVID1 header metadata size is out of range: " + metadataBytes);
            }

            var metadataJson = metadataBytes > 0
                ? Encoding.UTF8.GetString(ReadExact(stream, metadataBytes))
                : "";

            return new GargoyleRxyRvidStreamHeader(
                schemaVersion,
                codecId,
                width,
                height,
                declaredPacketCount,
                metadataBytes,
                metadataJson);
        }

        public static GargoyleRxyRvidPacket ReadPacket(Stream stream, int schemaVersion, int maxPacketBytes)
        {
            var ptsUs = ReadInt64BigEndian(stream);
            var flags = ReadInt32BigEndian(stream);
            var size = ReadInt32BigEndian(stream);
            if (size < 0 || size > maxPacketBytes)
            {
                throw new InvalidDataException("RXYRVID1 packet size is out of range: " + size);
            }

            long sourceElapsedNs = 0L;
            long sourceUnixNs = 0L;
            if (schemaVersion >= 2)
            {
                sourceElapsedNs = ReadInt64BigEndian(stream);
                sourceUnixNs = ReadInt64BigEndian(stream);
            }

            var payload = ReadExact(stream, size);
            return new GargoyleRxyRvidPacket(ptsUs, flags, sourceElapsedNs, sourceUnixNs, payload);
        }

        static byte[] ReadExact(Stream stream, int count)
        {
            var buffer = new byte[count];
            var offset = 0;
            while (offset < count)
            {
                var read = stream.Read(buffer, offset, count - offset);
                if (read <= 0)
                {
                    throw new EndOfStreamException("Unexpected end of RXYRVID1 stream.");
                }

                offset += read;
            }

            return buffer;
        }

        static int ReadInt32BigEndian(Stream stream)
        {
            var bytes = ReadExact(stream, 4);
            return (bytes[0] << 24) |
                   (bytes[1] << 16) |
                   (bytes[2] << 8) |
                   bytes[3];
        }

        static long ReadInt64BigEndian(Stream stream)
        {
            var bytes = ReadExact(stream, 8);
            return ((long)bytes[0] << 56) |
                   ((long)bytes[1] << 48) |
                   ((long)bytes[2] << 40) |
                   ((long)bytes[3] << 32) |
                   ((long)bytes[4] << 24) |
                   ((long)bytes[5] << 16) |
                   ((long)bytes[6] << 8) |
                   bytes[7];
        }
    }
}
