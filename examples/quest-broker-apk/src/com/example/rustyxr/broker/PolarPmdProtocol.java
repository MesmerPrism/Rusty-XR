package com.example.rustyxr.broker;

import android.util.Base64;

import org.json.JSONArray;
import org.json.JSONObject;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

final class PolarPmdProtocol {
    static final UUID HEART_RATE_SERVICE =
        UUID.fromString("0000180d-0000-1000-8000-00805f9b34fb");
    static final UUID HEART_RATE_MEASUREMENT =
        UUID.fromString("00002a37-0000-1000-8000-00805f9b34fb");
    static final UUID BATTERY_SERVICE =
        UUID.fromString("0000180f-0000-1000-8000-00805f9b34fb");
    static final UUID BATTERY_LEVEL =
        UUID.fromString("00002a19-0000-1000-8000-00805f9b34fb");
    static final UUID PMD_SERVICE =
        UUID.fromString("fb005c80-02e7-f387-1cad-8acd2d8df0c8");
    static final UUID PMD_CONTROL_POINT =
        UUID.fromString("fb005c81-02e7-f387-1cad-8acd2d8df0c8");
    static final UUID PMD_DATA =
        UUID.fromString("fb005c82-02e7-f387-1cad-8acd2d8df0c8");
    static final UUID CCCD_DESCRIPTOR =
        UUID.fromString("00002902-0000-1000-8000-00805f9b34fb");

    static final byte MEASUREMENT_TYPE_ECG = 0x00;
    static final byte MEASUREMENT_TYPE_ACC = 0x02;
    static final byte OPCODE_GET_SETTINGS = 0x01;
    static final byte OPCODE_START_STREAM = 0x02;
    static final byte OPCODE_STOP_STREAM = 0x03;

    private static final int RESPONSE_FRAME_ID = 0xF0;
    private static final int SETTING_TYPE_SAMPLE_RATE = 0x00;
    private static final int SETTING_TYPE_RESOLUTION = 0x01;
    private static final int SETTING_TYPE_RANGE = 0x02;
    private static final int PMD_HEADER_SIZE = 10;
    private static final int ECG_BYTES_PER_UNCOMPRESSED_SAMPLE = 3;
    private static final int ACC_BYTES_PER_UNCOMPRESSED_SAMPLE = 6;

    private PolarPmdProtocol() {
    }

    static byte[] buildGetSettingsRequest(byte measurementType) {
        return new byte[] { OPCODE_GET_SETTINGS, measurementType };
    }

    static byte[] buildStartAccRequest(int sampleRate, int resolution, int rangeG) {
        return new byte[] {
            OPCODE_START_STREAM,
            MEASUREMENT_TYPE_ACC,
            (byte) SETTING_TYPE_RANGE,
            0x01,
            (byte) (rangeG & 0xff),
            (byte) ((rangeG >> 8) & 0xff),
            (byte) SETTING_TYPE_SAMPLE_RATE,
            0x01,
            (byte) (sampleRate & 0xff),
            (byte) ((sampleRate >> 8) & 0xff),
            (byte) SETTING_TYPE_RESOLUTION,
            0x01,
            (byte) (resolution & 0xff),
            (byte) ((resolution >> 8) & 0xff)
        };
    }

    static byte[] buildStartEcgRequest(int sampleRate, int resolution) {
        return new byte[] {
            OPCODE_START_STREAM,
            MEASUREMENT_TYPE_ECG,
            (byte) SETTING_TYPE_SAMPLE_RATE,
            0x01,
            (byte) (sampleRate & 0xff),
            (byte) ((sampleRate >> 8) & 0xff),
            (byte) SETTING_TYPE_RESOLUTION,
            0x01,
            (byte) (resolution & 0xff),
            (byte) ((resolution >> 8) & 0xff)
        };
    }

    static byte[] buildStopRequest(byte measurementType) {
        return new byte[] { OPCODE_STOP_STREAM, measurementType };
    }

    static ControlResponse parseControlResponse(byte[] bytes) {
        if (bytes == null || bytes.length < 4) {
            return null;
        }
        return new ControlResponse(
            bytes[0] & 0xff,
            bytes[1] & 0xff,
            bytes[2] & 0xff,
            bytes[3] & 0xff,
            toHex(bytes));
    }

    static SettingsSummary parseSettingsResponse(byte[] bytes) {
        if (bytes == null || bytes.length < 5) {
            return null;
        }
        if ((bytes[0] & 0xff) != RESPONSE_FRAME_ID) {
            return null;
        }
        if (bytes[1] != OPCODE_GET_SETTINGS || (bytes[3] & 0xff) != 0) {
            return null;
        }

        int measurementType = bytes[2] & 0xff;
        SettingsSummary parsed = parseSettingsPayload(bytes, measurementType, 4);
        if (parsed == null || !parsed.hasAny()) {
            parsed = parseSettingsPayload(bytes, measurementType, 5);
        }
        return parsed != null && parsed.hasAny() ? parsed : null;
    }

    static AccFrame decodeAccFrame(byte[] bytes) {
        if (bytes == null || bytes.length < PMD_HEADER_SIZE || bytes[0] != MEASUREMENT_TYPE_ACC) {
            return null;
        }

        int frameType = bytes[9] & 0xff;
        boolean compressed = (frameType & 0x80) != 0;
        int frameTypeBase = frameType & 0x7f;
        List<AccSample> samples = (!compressed && frameTypeBase == 0x01)
            ? decodeUncompressedAcc(bytes)
            : decodeCompressedAcc(bytes);
        if (samples.isEmpty()) {
            return null;
        }

        return new AccFrame(readTimestampNs(bytes), samples, compressed || frameTypeBase != 0x01);
    }

    static EcgFrame decodeEcgFrame(byte[] bytes) {
        if (bytes == null || bytes.length < PMD_HEADER_SIZE || bytes[0] != MEASUREMENT_TYPE_ECG) {
            return null;
        }

        int frameType = bytes[9] & 0xff;
        boolean compressed = (frameType & 0x80) != 0;
        if (compressed || (frameType & 0x7f) != 0x00) {
            return null;
        }

        int payloadLength = bytes.length - PMD_HEADER_SIZE;
        if (payloadLength <= 0 || payloadLength % ECG_BYTES_PER_UNCOMPRESSED_SAMPLE != 0) {
            return null;
        }

        List<Integer> samples = new ArrayList<>();
        for (int offset = PMD_HEADER_SIZE; offset + 2 < bytes.length; offset += ECG_BYTES_PER_UNCOMPRESSED_SAMPLE) {
            samples.add(Integer.valueOf(readInt24Le(bytes, offset)));
        }
        return samples.isEmpty() ? null : new EcgFrame(readTimestampNs(bytes), samples, frameType);
    }

    static JSONObject accFramePayload(byte[] rawBytes, AccFrame frame, String deviceAddress, String deviceName) throws Exception {
        long nowUnixNs = unixNowNs();
        JSONObject payload = new JSONObject();
        payload.put("schema", "rusty.xr.polar.acc_pmd.v1");
        payload.put("stream_id", BreathAssessmentState.POLAR_INPUT_STREAM);
        payload.put("source", "android_ble_pmd");
        payload.put("device_address", deviceAddress != null ? deviceAddress : "");
        payload.put("device_name", deviceName != null ? deviceName : "");
        payload.put("sensor_timestamp_ns", frame.sensorTimestampNs);
        payload.put("sample_time_unix_ns", nowUnixNs);
        payload.put("sample_time_elapsed_ns", android.os.SystemClock.elapsedRealtimeNanos());
        payload.put("sample_count", frame.samples.size());
        payload.put("compressed", frame.compressed);
        payload.put("payload_base64", Base64.encodeToString(rawBytes, Base64.NO_WRAP));
        payload.put("payload_size_bytes", rawBytes != null ? rawBytes.length : 0);

        JSONArray samples = new JSONArray();
        for (int i = 0; i < frame.samples.size(); i++) {
            AccSample sample = frame.samples.get(i);
            JSONArray row = new JSONArray();
            row.put(sample.xMg);
            row.put(sample.yMg);
            row.put(sample.zMg);
            samples.put(row);
        }
        payload.put("samples_mg", samples);

        AccSample first = frame.samples.get(0);
        JSONObject decoded = new JSONObject();
        decoded.put("first_x_mg", first.xMg);
        decoded.put("first_y_mg", first.yMg);
        decoded.put("first_z_mg", first.zMg);
        decoded.put("sample_count", frame.samples.size());
        decoded.put("compressed", frame.compressed);
        payload.put("decoded", decoded);
        return payload;
    }

    static JSONObject ecgFramePayload(byte[] rawBytes, EcgFrame frame, String deviceAddress, String deviceName) throws Exception {
        long nowUnixNs = unixNowNs();
        JSONObject payload = new JSONObject();
        payload.put("schema", "rusty.xr.polar.ecg_pmd.v1");
        payload.put("stream_id", "bio:polar_ecg");
        payload.put("source", "android_ble_pmd");
        payload.put("device_address", deviceAddress != null ? deviceAddress : "");
        payload.put("device_name", deviceName != null ? deviceName : "");
        payload.put("sensor_timestamp_ns", frame.sensorTimestampNs);
        payload.put("sample_time_unix_ns", nowUnixNs);
        payload.put("sample_time_elapsed_ns", android.os.SystemClock.elapsedRealtimeNanos());
        payload.put("sample_count", frame.samplesMicrovolts.size());
        payload.put("frame_type", frame.frameType);
        payload.put("payload_base64", Base64.encodeToString(rawBytes, Base64.NO_WRAP));
        payload.put("payload_size_bytes", rawBytes != null ? rawBytes.length : 0);

        JSONArray samples = new JSONArray();
        int min = Integer.MAX_VALUE;
        int max = Integer.MIN_VALUE;
        long sum = 0L;
        for (int i = 0; i < frame.samplesMicrovolts.size(); i++) {
            int sample = frame.samplesMicrovolts.get(i).intValue();
            samples.put(sample);
            min = Math.min(min, sample);
            max = Math.max(max, sample);
            sum += sample;
        }
        payload.put("samples_microvolts", samples);

        JSONObject decoded = new JSONObject();
        decoded.put("sample_count", frame.samplesMicrovolts.size());
        decoded.put("first_microvolts", frame.samplesMicrovolts.get(0).intValue());
        decoded.put("min_microvolts", min);
        decoded.put("max_microvolts", max);
        decoded.put("mean_microvolts", sum / (double) frame.samplesMicrovolts.size());
        payload.put("decoded", decoded);
        return payload;
    }

    private static SettingsSummary parseSettingsPayload(byte[] bytes, int measurementType, int offset) {
        if (bytes.length <= offset) {
            return null;
        }

        List<Integer> sampleRates = new ArrayList<>();
        List<Integer> resolutions = new ArrayList<>();
        List<Integer> ranges = new ArrayList<>();
        int index = offset;
        while (index + 1 < bytes.length) {
            int settingType = bytes[index++] & 0xff;
            int count = bytes[index++] & 0xff;
            if (index + (count * 2) > bytes.length) {
                break;
            }

            for (int i = 0; i < count; i++) {
                int value = (bytes[index] & 0xff) | ((bytes[index + 1] & 0xff) << 8);
                index += 2;
                if (settingType == SETTING_TYPE_SAMPLE_RATE) {
                    sampleRates.add(value);
                } else if (settingType == SETTING_TYPE_RESOLUTION) {
                    resolutions.add(value);
                } else if (settingType == SETTING_TYPE_RANGE) {
                    ranges.add(value);
                }
            }
        }

        return new SettingsSummary(measurementType, sampleRates, resolutions, ranges);
    }

    private static long readTimestampNs(byte[] bytes) {
        return ByteBuffer.wrap(bytes, 1, 8)
            .order(ByteOrder.LITTLE_ENDIAN)
            .getLong();
    }

    private static List<AccSample> decodeUncompressedAcc(byte[] bytes) {
        int payloadLength = bytes.length - PMD_HEADER_SIZE;
        if (payloadLength <= 0 || payloadLength % ACC_BYTES_PER_UNCOMPRESSED_SAMPLE != 0) {
            return new ArrayList<>();
        }

        List<AccSample> samples = new ArrayList<>();
        for (int offset = PMD_HEADER_SIZE; offset < bytes.length; offset += ACC_BYTES_PER_UNCOMPRESSED_SAMPLE) {
            samples.add(new AccSample(
                readInt16Le(bytes, offset),
                readInt16Le(bytes, offset + 2),
                readInt16Le(bytes, offset + 4)));
        }
        return samples;
    }

    private static List<AccSample> decodeCompressedAcc(byte[] bytes) {
        if (bytes.length < 16) {
            return new ArrayList<>();
        }

        List<AccSample> samples = new ArrayList<>();
        int previousX = readInt16Le(bytes, 10);
        int previousY = readInt16Le(bytes, 12);
        int previousZ = readInt16Le(bytes, 14);
        samples.add(new AccSample(previousX, previousY, previousZ));

        int deltaBitWidth = 16;
        int bitsPerSample = deltaBitWidth * 3;
        int totalBits = (bytes.length - 16) * 8;
        int deltaSampleCount = totalBits / bitsPerSample;
        int bitOffset = 0;
        for (int i = 0; i < deltaSampleCount; i++) {
            int dx = readSignedBits(bytes, 16, bitOffset, deltaBitWidth);
            bitOffset += deltaBitWidth;
            int dy = readSignedBits(bytes, 16, bitOffset, deltaBitWidth);
            bitOffset += deltaBitWidth;
            int dz = readSignedBits(bytes, 16, bitOffset, deltaBitWidth);
            bitOffset += deltaBitWidth;

            previousX = clampInt16(previousX + dx);
            previousY = clampInt16(previousY + dy);
            previousZ = clampInt16(previousZ + dz);
            samples.add(new AccSample(previousX, previousY, previousZ));
        }
        return samples;
    }

    private static int readSignedBits(byte[] bytes, int startByteOffset, int bitOffset, int bitWidth) {
        int bytePos = startByteOffset + (bitOffset / 8);
        int bitInByte = bitOffset % 8;
        long value = 0L;
        int bitsRead = 0;

        while (bitsRead < bitWidth && bytePos < bytes.length) {
            int bitsAvailable = 8 - bitInByte;
            int bitsToRead = Math.min(bitsAvailable, bitWidth - bitsRead);
            int mask = (1 << bitsToRead) - 1;
            int bits = ((bytes[bytePos] & 0xff) >> bitInByte) & mask;
            value |= ((long) bits) << bitsRead;
            bitsRead += bitsToRead;
            bytePos++;
            bitInByte = 0;
        }

        if (bitWidth < 32 && (value & (1L << (bitWidth - 1))) != 0L) {
            value |= (-1L << bitWidth);
        }
        return (int) value;
    }

    private static int readInt16Le(byte[] bytes, int offset) {
        int raw = (bytes[offset] & 0xff) | ((bytes[offset + 1] & 0xff) << 8);
        return (short) raw;
    }

    private static int readInt24Le(byte[] bytes, int offset) {
        int raw = (bytes[offset] & 0xff)
            | ((bytes[offset + 1] & 0xff) << 8)
            | ((bytes[offset + 2] & 0xff) << 16);
        if ((raw & 0x00800000) != 0) {
            raw |= 0xff000000;
        }
        return raw;
    }

    private static int clampInt16(int value) {
        return Math.max(Short.MIN_VALUE, Math.min(Short.MAX_VALUE, value));
    }

    private static long unixNowNs() {
        return System.currentTimeMillis() * 1_000_000L;
    }

    private static String toHex(byte[] bytes) {
        StringBuilder builder = new StringBuilder(bytes.length * 2);
        for (byte value : bytes) {
            builder.append(String.format(java.util.Locale.ROOT, "%02x", value & 0xff));
        }
        return builder.toString();
    }

    static final class ControlResponse {
        final int frameId;
        final int opCode;
        final int measurementType;
        final int errorCode;
        final String payloadHex;

        private ControlResponse(int frameId, int opCode, int measurementType, int errorCode, String payloadHex) {
            this.frameId = frameId;
            this.opCode = opCode;
            this.measurementType = measurementType;
            this.errorCode = errorCode;
            this.payloadHex = payloadHex;
        }

        boolean success() {
            return errorCode == 0;
        }

        JSONObject toJson() throws Exception {
            JSONObject json = new JSONObject();
            json.put("frame_id", frameId);
            json.put("op_code", opCode);
            json.put("measurement_type", measurementType);
            json.put("error_code", errorCode);
            json.put("payload_hex", payloadHex);
            json.put("success", success());
            return json;
        }
    }

    static final class SettingsSummary {
        final int measurementType;
        final List<Integer> sampleRates;
        final List<Integer> resolutions;
        final List<Integer> ranges;

        private SettingsSummary(
            int measurementType,
            List<Integer> sampleRates,
            List<Integer> resolutions,
            List<Integer> ranges) {
            this.measurementType = measurementType;
            this.sampleRates = sampleRates;
            this.resolutions = resolutions;
            this.ranges = ranges;
        }

        boolean hasAny() {
            return !sampleRates.isEmpty() || !resolutions.isEmpty() || !ranges.isEmpty();
        }

        JSONObject toJson() throws Exception {
            JSONObject json = new JSONObject();
            json.put("measurement_type", measurementType);
            json.put("sample_rates", intArray(sampleRates));
            json.put("resolutions", intArray(resolutions));
            json.put("ranges", intArray(ranges));
            return json;
        }

        private static JSONArray intArray(List<Integer> values) {
            JSONArray array = new JSONArray();
            for (int i = 0; i < values.size(); i++) {
                array.put(values.get(i).intValue());
            }
            return array;
        }
    }

    static final class AccFrame {
        final long sensorTimestampNs;
        final List<AccSample> samples;
        final boolean compressed;

        private AccFrame(long sensorTimestampNs, List<AccSample> samples, boolean compressed) {
            this.sensorTimestampNs = sensorTimestampNs;
            this.samples = samples;
            this.compressed = compressed;
        }
    }

    static final class EcgFrame {
        final long sensorTimestampNs;
        final List<Integer> samplesMicrovolts;
        final int frameType;

        private EcgFrame(long sensorTimestampNs, List<Integer> samplesMicrovolts, int frameType) {
            this.sensorTimestampNs = sensorTimestampNs;
            this.samplesMicrovolts = samplesMicrovolts;
            this.frameType = frameType;
        }
    }

    static final class AccSample {
        final int xMg;
        final int yMg;
        final int zMg;

        private AccSample(int xMg, int yMg, int zMg) {
            this.xMg = xMg;
            this.yMg = yMg;
            this.zMg = zMg;
        }
    }
}
