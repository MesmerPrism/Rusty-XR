using System.IO;
using System.Text;
using NUnit.Framework;

namespace RustyXr.Gargoyle.Video.Tests
{
    public sealed class GargoyleRxyRvidParserTests
    {
        [Test]
        public void ReadHeaderAndPacketParsesSchema3Stream()
        {
            using var stream = new MemoryStream();
            var metadata = Encoding.UTF8.GetBytes("{\"projection\":\"synthetic\"}");
            var payload = new byte[] { 0, 0, 0, 1, 103 };

            WriteAscii(stream, "RXYRVID1");
            WriteInt32(stream, 3);
            WriteInt32(stream, 1);
            WriteInt32(stream, 1280);
            WriteInt32(stream, 720);
            WriteInt32(stream, 1);
            WriteInt32(stream, metadata.Length);
            stream.Write(metadata, 0, metadata.Length);

            WriteInt64(stream, 12345);
            WriteInt32(stream, GargoyleRxyRvidPacket.BufferFlagKeyFrame);
            WriteInt32(stream, payload.Length);
            WriteInt64(stream, 987654321);
            WriteInt64(stream, 1122334455);
            stream.Write(payload, 0, payload.Length);
            stream.Position = 0;

            var header = GargoyleRxyRvidParser.ReadHeader(stream, 1024);
            var packet = GargoyleRxyRvidParser.ReadPacket(stream, header.SchemaVersion, 1024);

            Assert.That(header.SchemaVersion, Is.EqualTo(3));
            Assert.That(header.Width, Is.EqualTo(1280));
            Assert.That(header.Height, Is.EqualTo(720));
            Assert.That(header.HeaderMetadataJson, Is.EqualTo("{\"projection\":\"synthetic\"}"));
            Assert.That(packet.PtsUs, Is.EqualTo(12345));
            Assert.That(packet.IsKeyFrame, Is.True);
            Assert.That(packet.SourceElapsedNs, Is.EqualTo(987654321));
            Assert.That(packet.PayloadBytes, Is.EqualTo(payload.Length));
        }

        static void WriteAscii(Stream stream, string value)
        {
            var bytes = Encoding.ASCII.GetBytes(value);
            stream.Write(bytes, 0, bytes.Length);
        }

        static void WriteInt32(Stream stream, int value)
        {
            stream.WriteByte((byte)((value >> 24) & 0xff));
            stream.WriteByte((byte)((value >> 16) & 0xff));
            stream.WriteByte((byte)((value >> 8) & 0xff));
            stream.WriteByte((byte)(value & 0xff));
        }

        static void WriteInt64(Stream stream, long value)
        {
            stream.WriteByte((byte)((value >> 56) & 0xff));
            stream.WriteByte((byte)((value >> 48) & 0xff));
            stream.WriteByte((byte)((value >> 40) & 0xff));
            stream.WriteByte((byte)((value >> 32) & 0xff));
            stream.WriteByte((byte)((value >> 24) & 0xff));
            stream.WriteByte((byte)((value >> 16) & 0xff));
            stream.WriteByte((byte)((value >> 8) & 0xff));
            stream.WriteByte((byte)(value & 0xff));
        }
    }
}
