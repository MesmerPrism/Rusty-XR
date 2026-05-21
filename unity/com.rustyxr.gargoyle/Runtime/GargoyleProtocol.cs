using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using UnityEngine;

namespace RustyXr.Gargoyle
{
    public static class GargoyleProtocol
    {
        public const string DefaultWebSocketUri = "ws://127.0.0.1:8765/rustyxr/v1/events";
        public const string ContractVersion = "rusty.xr.broker.v1";
        public const int ProtocolVersionMin = 1;
        public const int ProtocolVersionMax = 1;
        public const string HelloSchema = "rusty.xr.broker.client_hello.v1";
        public const string CommandSchema = "rusty.xr.broker.command.v1";
        public const string CommandAckSchema = "rusty.xr.broker.command_ack.v1";
        public const string StreamEventSchema = "rusty.xr.broker.stream_event.v1";
        public const string ReplayRecordSchema = "rusty.xr.broker.replay_record.v1";
        public const string StreamSampleHeaderSchema = "rusty.xr.broker.stream_sample_header.v1";

        public static string BuildHelloJson(GargoyleClientIdentity identity)
        {
            var hello = new GargoyleHelloEnvelope
            {
                type = "hello",
                schema = HelloSchema,
                client_id = identity.ClientId,
                app_package = identity.AppPackage,
                app_label = identity.AppLabel,
                app_version = identity.AppVersion,
                protocol_min = ProtocolVersionMin,
                protocol_max = ProtocolVersionMax,
                supports_commands = true
            };

            return JsonUtility.ToJson(hello);
        }

        public static string BuildStatusRequestCommandJson(string requestId, GargoyleClientIdentity identity) =>
            BuildCommandJson("status_request", requestId, identity, null);

        public static string BuildListStreamsCommandJson(string requestId, GargoyleClientIdentity identity) =>
            BuildCommandJson("list_streams", requestId, identity, null);

        public static string BuildListCapabilitiesCommandJson(string requestId, GargoyleClientIdentity identity) =>
            BuildCommandJson("list_capabilities", requestId, identity, null);

        public static string BuildOpenUiCommandJson(string requestId, GargoyleClientIdentity identity) =>
            BuildCommandJson("open_ui", requestId, identity, null);

        public static string BuildCloseUiCommandJson(string requestId, GargoyleClientIdentity identity) =>
            BuildCommandJson("close_ui", requestId, identity, null);

        public static string BuildSubscribeCommandJson(string requestId, GargoyleClientIdentity identity, string stream) =>
            BuildCommandJson("subscribe", requestId, identity, BuildParamsJson(GargoyleJsonField.String("stream", stream)));

        public static string BuildUnsubscribeCommandJson(string requestId, GargoyleClientIdentity identity, string stream) =>
            BuildCommandJson("unsubscribe", requestId, identity, BuildParamsJson(GargoyleJsonField.String("stream", stream)));

        public static string BuildCommandJson(
            string command,
            string requestId,
            GargoyleClientIdentity identity,
            string paramsJson)
        {
            var builder = new StringBuilder(256);
            builder.Append('{');
            AppendJsonStringField(builder, "type", "command", false);
            AppendJsonStringField(builder, "schema", CommandSchema, true);
            AppendJsonStringField(builder, "request_id", string.IsNullOrWhiteSpace(requestId) ? Guid.NewGuid().ToString("N") : requestId, true);
            AppendJsonStringField(builder, "command", command, true);
            AppendJsonStringField(builder, "client_id", identity.ClientId, true);
            AppendJsonStringField(builder, "app_package", identity.AppPackage, true);
            AppendJsonStringField(builder, "app_label", identity.AppLabel, true);
            AppendJsonStringField(builder, "app_version", identity.AppVersion, true);
            if (!string.IsNullOrWhiteSpace(paramsJson))
            {
                builder.Append(",\"params\":");
                builder.Append(paramsJson);
            }
            builder.Append('}');
            return builder.ToString();
        }

        public static string BuildParamsJson(params GargoyleJsonField[] fields)
        {
            return BuildParamsJson((IEnumerable<GargoyleJsonField>)fields);
        }

        public static string BuildParamsJson(IEnumerable<GargoyleJsonField> fields)
        {
            var builder = new StringBuilder(128);
            builder.Append('{');
            var wroteAny = false;
            if (fields != null)
            {
                foreach (var field in fields)
                {
                    if (!field.HasValue || string.IsNullOrWhiteSpace(field.Name))
                    {
                        continue;
                    }

                    if (wroteAny)
                    {
                        builder.Append(',');
                    }

                    AppendJsonQuoted(builder, field.Name);
                    builder.Append(':');
                    switch (field.Kind)
                    {
                        case GargoyleJsonValueKind.String:
                            AppendJsonQuoted(builder, field.Value);
                            break;
                        case GargoyleJsonValueKind.Boolean:
                        case GargoyleJsonValueKind.Number:
                        case GargoyleJsonValueKind.Raw:
                            builder.Append(field.Value);
                            break;
                        default:
                            AppendJsonQuoted(builder, field.Value);
                            break;
                    }

                    wroteAny = true;
                }
            }
            builder.Append('}');
            return builder.ToString();
        }

        public static bool TryParseCommandAck(string json, out GargoyleCommandAck ack)
        {
            ack = null;
            if (string.IsNullOrWhiteSpace(json))
            {
                return false;
            }

            try
            {
                var parsed = JsonUtility.FromJson<GargoyleCommandAck>(json);
                if (parsed == null ||
                    parsed.type != "command_ack" ||
                    parsed.schema != CommandAckSchema ||
                    string.IsNullOrWhiteSpace(parsed.request_id))
                {
                    return false;
                }

                ack = parsed;
                return true;
            }
            catch (ArgumentException)
            {
                return false;
            }
        }

        public static bool TryParseStreamEvent(string json, out GargoyleStreamEvent streamEvent)
        {
            streamEvent = null;
            if (string.IsNullOrWhiteSpace(json))
            {
                return false;
            }

            try
            {
                var parsed = JsonUtility.FromJson<GargoyleStreamEvent>(json);
                if (parsed == null ||
                    parsed.type != "stream_event" ||
                    parsed.schema != StreamEventSchema ||
                    !parsed.NormalizeFromHeader())
                {
                    return false;
                }

                streamEvent = parsed;
                return true;
            }
            catch (ArgumentException)
            {
                return false;
            }
        }

        public static bool TryParseReplayRecord(string json, out GargoyleStreamEvent streamEvent)
        {
            streamEvent = null;
            if (string.IsNullOrWhiteSpace(json))
            {
                return false;
            }

            try
            {
                var parsed = JsonUtility.FromJson<GargoyleReplayRecordEnvelope>(json);
                if (parsed == null ||
                    parsed.type != "replay_record" ||
                    parsed.schema != ReplayRecordSchema)
                {
                    return false;
                }

                var normalized = new GargoyleStreamEvent
                {
                    type = "stream_event",
                    schema = StreamEventSchema,
                    stream = parsed.stream,
                    header = parsed.header,
                    payload = parsed.payload
                };

                if (!normalized.NormalizeFromHeader())
                {
                    return false;
                }

                streamEvent = normalized;
                return true;
            }
            catch (ArgumentException)
            {
                return false;
            }
        }

        public static string EscapeJsonString(string value)
        {
            if (string.IsNullOrEmpty(value))
            {
                return "";
            }

            var builder = new StringBuilder(value.Length + 8);
            for (var i = 0; i < value.Length; i++)
            {
                var c = value[i];
                switch (c)
                {
                    case '\\':
                        builder.Append("\\\\");
                        break;
                    case '"':
                        builder.Append("\\\"");
                        break;
                    case '\b':
                        builder.Append("\\b");
                        break;
                    case '\f':
                        builder.Append("\\f");
                        break;
                    case '\n':
                        builder.Append("\\n");
                        break;
                    case '\r':
                        builder.Append("\\r");
                        break;
                    case '\t':
                        builder.Append("\\t");
                        break;
                    default:
                        if (c < ' ')
                        {
                            builder.Append("\\u");
                            builder.Append(((int)c).ToString("x4", CultureInfo.InvariantCulture));
                        }
                        else
                        {
                            builder.Append(c);
                        }
                        break;
                }
            }

            return builder.ToString();
        }

        static void AppendJsonStringField(StringBuilder builder, string name, string value, bool prefixComma)
        {
            if (prefixComma)
            {
                builder.Append(',');
            }

            AppendJsonQuoted(builder, name);
            builder.Append(':');
            AppendJsonQuoted(builder, value);
        }

        static void AppendJsonQuoted(StringBuilder builder, string value)
        {
            builder.Append('"');
            builder.Append(EscapeJsonString(value));
            builder.Append('"');
        }
    }

    public enum GargoyleJsonValueKind
    {
        String = 0,
        Number = 1,
        Boolean = 2,
        Raw = 3
    }

    public readonly struct GargoyleJsonField
    {
        GargoyleJsonField(string name, string value, GargoyleJsonValueKind kind, bool hasValue)
        {
            Name = name;
            Value = value;
            Kind = kind;
            HasValue = hasValue;
        }

        public string Name { get; }
        public string Value { get; }
        public GargoyleJsonValueKind Kind { get; }
        public bool HasValue { get; }

        public static GargoyleJsonField String(string name, string value)
        {
            return string.IsNullOrWhiteSpace(value)
                ? new GargoyleJsonField(name, "", GargoyleJsonValueKind.String, false)
                : new GargoyleJsonField(name, value, GargoyleJsonValueKind.String, true);
        }

        public static GargoyleJsonField OptionalString(string name, string value) => String(name, value);

        public static GargoyleJsonField Number(string name, int value) =>
            new GargoyleJsonField(name, value.ToString(CultureInfo.InvariantCulture), GargoyleJsonValueKind.Number, true);

        public static GargoyleJsonField Number(string name, long value) =>
            new GargoyleJsonField(name, value.ToString(CultureInfo.InvariantCulture), GargoyleJsonValueKind.Number, true);

        public static GargoyleJsonField Number(string name, float value) =>
            new GargoyleJsonField(name, value.ToString(CultureInfo.InvariantCulture), GargoyleJsonValueKind.Number, true);

        public static GargoyleJsonField Boolean(string name, bool value) =>
            new GargoyleJsonField(name, value ? "true" : "false", GargoyleJsonValueKind.Boolean, true);

        public static GargoyleJsonField Raw(string name, string rawJson)
        {
            return string.IsNullOrWhiteSpace(rawJson)
                ? new GargoyleJsonField(name, "", GargoyleJsonValueKind.Raw, false)
                : new GargoyleJsonField(name, rawJson, GargoyleJsonValueKind.Raw, true);
        }
    }
}
