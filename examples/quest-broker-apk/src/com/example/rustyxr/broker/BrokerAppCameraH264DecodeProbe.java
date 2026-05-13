package com.example.rustyxr.broker;

import android.content.Context;
import android.media.MediaCodec;
import android.media.MediaCodecInfo;
import android.media.MediaFormat;
import android.os.Bundle;
import android.os.SystemClock;

import org.json.JSONObject;

import java.nio.ByteBuffer;
import java.util.List;

final class BrokerAppCameraH264DecodeProbe {
    private static final int DEFAULT_DECODE_TIMEOUT_MS = 5000;
    private static final int MAX_DECODE_TIMEOUT_MS = 15000;
    private static final int DEQUEUE_TIMEOUT_US = 10000;

    private BrokerAppCameraH264DecodeProbe() {
    }

    static JSONObject run(Context context, JSONObject params) throws Exception {
        long totalStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        JSONObject probe = new JSONObject();
        probe.put("schema", "rusty.xr.video_lab.h264_decode_probe.v1");
        probe.put("source", "broker_app_camera2_mediacodec_decode_probe");
        probe.put("capture_source", "broker_app_camera2_mediacodec_surface");
        probe.put("decoder_api", "android.media.MediaCodec");
        probe.put("decoder_output_mode", "byte_buffer");
        probe.put("mime_type", "video/avc");
        probe.put("codec", "h264");
        probe.put("decode_succeeded", false);

        try {
            BrokerAppCameraH264StreamSession.CaptureResult capture =
                BrokerAppCameraH264StreamSession.capturePacketsForProbe(context, params);
            int decodeTimeoutMs = clamp(
                params != null ? params.optInt("decode_timeout_ms", DEFAULT_DECODE_TIMEOUT_MS) : DEFAULT_DECODE_TIMEOUT_MS,
                1000,
                MAX_DECODE_TIMEOUT_MS);
            DecodeResult decode = decodePackets(capture, decodeTimeoutMs);
            long totalEndElapsedNs = SystemClock.elapsedRealtimeNanos();

            probe.put("session_id", capture.sessionId);
            probe.put("requested_camera_id", capture.requestedCameraId);
            probe.put("camera_id", capture.cameraId);
            probe.put("width", capture.size.getWidth());
            probe.put("height", capture.size.getHeight());
            probe.put("capture_ms", capture.captureMs);
            probe.put("max_packets", capture.maxPackets);
            probe.put("bitrate_bps", capture.bitrateBps);
            probe.put("decode_timeout_ms", decodeTimeoutMs);
            probe.put("encoded_packet_count", capture.packets.size());
            probe.put("encoded_video_packet_count", videoPacketCount(capture.packets));
            probe.put("codec_config_packet_count", codecConfigPacketCount(capture.packets));
            probe.put("keyframe_count", keyFrameCount(capture.packets));
            probe.put("encoded_payload_bytes", encodedPayloadBytes(capture.packets));
            probe.put("camera_encode_start_elapsed_ns", capture.encodeStartElapsedNs);
            probe.put("camera_encode_end_elapsed_ns", capture.encodeEndElapsedNs);
            probe.put("camera_encode_duration_ns", Math.max(0L, capture.encodeEndElapsedNs - capture.encodeStartElapsedNs));
            if (capture.encoderMetadata != null) {
                probe.put("encoder_name", capture.encoderMetadata.encoderName);
                probe.put("encoder_selection_source", capture.encoderMetadata.encoderSelectionSource);
                probe.put("encoder_selected_name", capture.encoderMetadata.encoderSelectedName);
                probe.put("encoder_hardware_accelerated", capture.encoderMetadata.encoderHardwareAccelerated);
                probe.put("encoder_software_only", capture.encoderMetadata.encoderSoftwareOnly);
                probe.put("encoder_size_supported", capture.encoderMetadata.encoderSizeSupported);
                probe.put("encoder_size_and_rate_supported", capture.encoderMetadata.encoderSizeAndRateSupported);
                probe.put("encoder_bitrate_supported", capture.encoderMetadata.encoderBitrateSupported);
                probe.put("encoder_cbr_supported", capture.encoderMetadata.encoderCbrSupported);
                probe.put("encoder_cbr_fd_supported", capture.encoderMetadata.encoderCbrFdSupported);
                probe.put("encoder_vbr_supported", capture.encoderMetadata.encoderVbrSupported);
                probe.put("bitrate_mode_requested", capture.encoderMetadata.bitrateModeRequested);
                probe.put("bitrate_mode_applied", capture.encoderMetadata.bitrateModeApplied);
                probe.put("bitrate_mode_output_format", capture.encoderMetadata.outputBitrateMode);
                probe.put("encoder_output_format_changes", capture.encoderMetadata.outputFormatChangeCount);
                probe.put("encoder_output_mime", capture.encoderMetadata.outputMime);
                probe.put("encoder_output_width", capture.encoderMetadata.outputWidth);
                probe.put("encoder_output_height", capture.encoderMetadata.outputHeight);
                probe.put("prepend_headers_to_sync_frames_applied", capture.encoderMetadata.prependHeadersToSyncFramesApplied);
                probe.put("sync_frame_request_on_start_succeeded", capture.encoderMetadata.syncFrameRequestOnStartSucceeded);
                probe.put("sensor_timestamp_source", capture.encoderMetadata.sensorTimestampSource);
                probe.put("camera_capture_started_count", capture.encoderMetadata.cameraCaptureStartedCount);
                probe.put("camera_first_capture_started_ns", capture.encoderMetadata.cameraFirstCaptureStartedNs);
                probe.put("camera_last_capture_started_ns", capture.encoderMetadata.cameraLastCaptureStartedNs);
                probe.put("camera_first_frame_number", capture.encoderMetadata.cameraFirstFrameNumber);
                probe.put("camera_last_frame_number", capture.encoderMetadata.cameraLastFrameNumber);
                probe.put("camera_first_capture_callback_elapsed_ns", capture.encoderMetadata.cameraFirstCaptureCallbackElapsedNs);
                probe.put("camera_last_capture_callback_elapsed_ns", capture.encoderMetadata.cameraLastCaptureCallbackElapsedNs);
                probe.put("csd_sps_base64", capture.encoderMetadata.csdSps != null
                    ? android.util.Base64.encodeToString(capture.encoderMetadata.csdSps, android.util.Base64.NO_WRAP)
                    : "");
                probe.put("csd_pps_base64", capture.encoderMetadata.csdPps != null
                    ? android.util.Base64.encodeToString(capture.encoderMetadata.csdPps, android.util.Base64.NO_WRAP)
                    : "");
            }
            probe.put("decode_start_elapsed_ns", decode.decodeStartElapsedNs);
            probe.put("decode_end_elapsed_ns", decode.decodeEndElapsedNs);
            probe.put("decode_duration_ns", Math.max(0L, decode.decodeEndElapsedNs - decode.decodeStartElapsedNs));
            probe.put("total_duration_ns", Math.max(0L, totalEndElapsedNs - totalStartElapsedNs));
            probe.put("decoder_name", decode.decoderName);
            probe.put("decoder_low_latency_feature_supported", decode.lowLatencyFeatureSupported);
            probe.put("decoder_low_latency_config_requested", decode.lowLatencyConfigRequested);
            probe.put("decoder_low_latency_parameter_succeeded", decode.lowLatencyParameterSucceeded);
            probe.put("csd_sps_found", decode.spsBytes > 0);
            probe.put("csd_pps_found", decode.ppsBytes > 0);
            probe.put("csd_sps_bytes", decode.spsBytes);
            probe.put("csd_pps_bytes", decode.ppsBytes);
            probe.put("csd_source", decode.csdSource);
            probe.put("codec_config_packets_skipped", decode.codecConfigPacketsSkipped);
            probe.put("input_buffer_count", decode.inputBufferCount);
            probe.put("input_bytes", decode.inputBytes);
            probe.put("input_eos_queued", decode.inputEosQueued);
            probe.put("output_format_changes", decode.outputFormatChanges);
            probe.put("output_buffer_count", decode.outputBufferCount);
            probe.put("decoded_frame_count", decode.decodedFrameCount);
            probe.put("decoder_output_bytes", decode.outputBytes);
            probe.put("output_eos_seen", decode.outputEosSeen);
            probe.put("output_format_mime", decode.outputMime);
            probe.put("output_format_width", decode.outputWidth);
            probe.put("output_format_height", decode.outputHeight);
            probe.put("first_output_pts_us", decode.firstOutputPtsUs);
            probe.put("last_output_pts_us", decode.lastOutputPtsUs);
            probe.put("decode_succeeded", decode.decodedFrameCount > 0);
            if (decode.lastError.length() > 0) {
                probe.put("last_error", decode.lastError);
            }
        } catch (Exception ex) {
            long totalEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            probe.put("total_duration_ns", Math.max(0L, totalEndElapsedNs - totalStartElapsedNs));
            probe.put("last_error", ex.getClass().getSimpleName() + ": " + safeMessage(ex));
        }

        return probe;
    }

    private static DecodeResult decodePackets(
        BrokerAppCameraH264StreamSession.CaptureResult capture,
        int timeoutMs) throws Exception {
        DecodeResult result = new DecodeResult();
        result.decodeStartElapsedNs = SystemClock.elapsedRealtimeNanos();
        NalUnit packetSps = findNalUnit(capture.packets, 7);
        NalUnit packetPps = findNalUnit(capture.packets, 8);
        byte[] sps = capture.encoderMetadata != null && capture.encoderMetadata.csdSps != null
            ? capture.encoderMetadata.csdSps
            : (packetSps != null ? packetSps.bytes : null);
        byte[] pps = capture.encoderMetadata != null && capture.encoderMetadata.csdPps != null
            ? capture.encoderMetadata.csdPps
            : (packetPps != null ? packetPps.bytes : null);
        result.spsBytes = sps != null ? sps.length : 0;
        result.ppsBytes = pps != null ? pps.length : 0;
        result.csdSource = capture.encoderMetadata != null &&
            capture.encoderMetadata.csdSps != null &&
            capture.encoderMetadata.csdPps != null
            ? "encoder_output_format"
            : (packetSps != null && packetPps != null ? "packet_payload" : "missing");
        boolean hasCompleteCsd = sps != null && pps != null;

        MediaFormat format = MediaFormat.createVideoFormat(
            "video/avc",
            capture.size.getWidth(),
            capture.size.getHeight());
        if (sps != null) {
            format.setByteBuffer("csd-0", ByteBuffer.wrap(sps));
        }
        if (pps != null) {
            format.setByteBuffer("csd-1", ByteBuffer.wrap(pps));
        }

        MediaCodec decoder = MediaCodec.createDecoderByType("video/avc");
        try {
            result.decoderName = decoder.getName();
            result.lowLatencyFeatureSupported = decoderLowLatencySupported(decoder);
            result.lowLatencyConfigRequested = true;
            format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1);
            decoder.configure(format, null, null, 0);
            decoder.start();
            result.lowLatencyParameterSucceeded = requestDecoderLowLatency(decoder);

            MediaCodec.BufferInfo info = new MediaCodec.BufferInfo();
            long deadlineElapsedNs = SystemClock.elapsedRealtimeNanos() + timeoutMs * 1_000_000L;
            int nextInput = 0;
            while (!result.outputEosSeen && SystemClock.elapsedRealtimeNanos() < deadlineElapsedNs) {
                if (!result.inputEosQueued) {
                    if (hasCompleteCsd) {
                        while (nextInput < capture.packets.size() &&
                            (capture.packets.get(nextInput).flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0) {
                            nextInput++;
                            result.codecConfigPacketsSkipped++;
                        }
                    }

                    int inputIndex = decoder.dequeueInputBuffer(DEQUEUE_TIMEOUT_US);
                    if (inputIndex >= 0) {
                        if (nextInput < capture.packets.size()) {
                            BrokerAppCameraH264StreamSession.EncodedPacket packet = capture.packets.get(nextInput++);
                            queuePacket(decoder, inputIndex, packet);
                            result.inputBufferCount++;
                            result.inputBytes += packet.payload.length;
                        } else {
                            long eosPtsUs = lastPresentationTimeUs(capture.packets);
                            decoder.queueInputBuffer(
                                inputIndex,
                                0,
                                0,
                                eosPtsUs,
                                MediaCodec.BUFFER_FLAG_END_OF_STREAM);
                            result.inputEosQueued = true;
                        }
                    }
                }

                int outputIndex = decoder.dequeueOutputBuffer(info, DEQUEUE_TIMEOUT_US);
                if (outputIndex == MediaCodec.INFO_TRY_AGAIN_LATER) {
                    continue;
                }
                if (outputIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                    result.outputFormatChanges++;
                    applyOutputFormat(result, decoder.getOutputFormat(), capture);
                    continue;
                }
                if (outputIndex < 0) {
                    continue;
                }

                if (info.size > 0 && (info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) == 0) {
                    result.outputBufferCount++;
                    result.outputBytes += info.size;
                    result.decodedFrameCount++;
                    if (result.firstOutputPtsUs < 0L) {
                        result.firstOutputPtsUs = info.presentationTimeUs;
                    }
                    result.lastOutputPtsUs = info.presentationTimeUs;
                }
                if ((info.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0) {
                    result.outputEosSeen = true;
                }
                decoder.releaseOutputBuffer(outputIndex, false);
            }

            if (result.decodedFrameCount == 0 && result.lastError.length() == 0) {
                result.lastError = result.outputEosSeen
                    ? "Decoder reached end-of-stream without a decoded output frame."
                    : "Timed out before a decoded output frame was produced.";
            }
            if (result.outputWidth == 0 || result.outputHeight == 0) {
                applyOutputFormat(result, decoder.getOutputFormat(), capture);
            }
        } finally {
            result.decodeEndElapsedNs = SystemClock.elapsedRealtimeNanos();
            try {
                decoder.stop();
            } catch (Exception ignored) {
            }
            decoder.release();
        }

        return result;
    }

    private static void queuePacket(
        MediaCodec decoder,
        int inputIndex,
        BrokerAppCameraH264StreamSession.EncodedPacket packet) throws Exception {
        ByteBuffer inputBuffer = decoder.getInputBuffer(inputIndex);
        if (inputBuffer == null) {
            throw new IllegalStateException("Decoder input buffer is unavailable.");
        }
        if (packet.payload.length > inputBuffer.capacity()) {
            throw new IllegalStateException("Encoded packet exceeds decoder input capacity.");
        }

        inputBuffer.clear();
        inputBuffer.put(packet.payload);
        int flags = (packet.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0
            ? MediaCodec.BUFFER_FLAG_CODEC_CONFIG
            : 0;
        decoder.queueInputBuffer(inputIndex, 0, packet.payload.length, packet.ptsUs, flags);
    }

    private static void applyOutputFormat(
        DecodeResult result,
        MediaFormat format,
        BrokerAppCameraH264StreamSession.CaptureResult capture) {
        result.outputMime = mediaFormatString(format, MediaFormat.KEY_MIME, "video/avc");
        result.outputWidth = mediaFormatInt(format, MediaFormat.KEY_WIDTH, capture.size.getWidth());
        result.outputHeight = mediaFormatInt(format, MediaFormat.KEY_HEIGHT, capture.size.getHeight());
    }

    private static boolean decoderLowLatencySupported(MediaCodec decoder) {
        try {
            MediaCodecInfo.CodecCapabilities capabilities =
                decoder.getCodecInfo().getCapabilitiesForType("video/avc");
            return capabilities.isFeatureSupported(MediaCodecInfo.CodecCapabilities.FEATURE_LowLatency);
        } catch (Exception ignored) {
            return false;
        }
    }

    private static boolean requestDecoderLowLatency(MediaCodec decoder) {
        Bundle params = new Bundle();
        params.putInt(MediaCodec.PARAMETER_KEY_LOW_LATENCY, 1);
        try {
            decoder.setParameters(params);
            return true;
        } catch (Exception ignored) {
            return false;
        }
    }

    private static NalUnit findNalUnit(
        List<BrokerAppCameraH264StreamSession.EncodedPacket> packets,
        int nalType) {
        for (int i = 0; i < packets.size(); i++) {
            byte[] payload = packets.get(i).payload;
            int start = findStartCode(payload, 0);
            while (start >= 0) {
                int startCodeLength = startCodeLengthAt(payload, start);
                int nalStart = start + startCodeLength;
                if (nalStart >= payload.length) {
                    break;
                }

                int nextStart = findStartCode(payload, nalStart);
                int nalEnd = nextStart >= 0 ? nextStart : payload.length;
                int type = payload[nalStart] & 0x1f;
                if (type == nalType) {
                    byte[] bytes = new byte[nalEnd - start];
                    System.arraycopy(payload, start, bytes, 0, bytes.length);
                    return new NalUnit(bytes);
                }

                start = nextStart;
            }
        }

        return null;
    }

    private static int findStartCode(byte[] data, int offset) {
        for (int i = Math.max(0, offset); i < data.length - 2; i++) {
            if (startCodeLengthAt(data, i) > 0) {
                return i;
            }
        }
        return -1;
    }

    private static int startCodeLengthAt(byte[] data, int offset) {
        if (offset + 4 <= data.length &&
            data[offset] == 0 &&
            data[offset + 1] == 0 &&
            data[offset + 2] == 0 &&
            data[offset + 3] == 1) {
            return 4;
        }
        if (offset + 3 <= data.length &&
            data[offset] == 0 &&
            data[offset + 1] == 0 &&
            data[offset + 2] == 1) {
            return 3;
        }
        return 0;
    }

    private static long encodedPayloadBytes(List<BrokerAppCameraH264StreamSession.EncodedPacket> packets) {
        long bytes = 0L;
        for (int i = 0; i < packets.size(); i++) {
            bytes += packets.get(i).payload.length;
        }
        return bytes;
    }

    private static int videoPacketCount(List<BrokerAppCameraH264StreamSession.EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (!packets.get(i).isCodecConfig()) {
                count++;
            }
        }
        return count;
    }

    private static int codecConfigPacketCount(List<BrokerAppCameraH264StreamSession.EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (packets.get(i).isCodecConfig()) {
                count++;
            }
        }
        return count;
    }

    private static int keyFrameCount(List<BrokerAppCameraH264StreamSession.EncodedPacket> packets) {
        int count = 0;
        for (int i = 0; i < packets.size(); i++) {
            if (packets.get(i).isKeyFrame()) {
                count++;
            }
        }
        return count;
    }

    private static long lastPresentationTimeUs(List<BrokerAppCameraH264StreamSession.EncodedPacket> packets) {
        return packets.size() > 0 ? packets.get(packets.size() - 1).ptsUs : 0L;
    }

    private static int mediaFormatInt(MediaFormat format, String key, int fallback) {
        try {
            return format.containsKey(key) ? format.getInteger(key) : fallback;
        } catch (Exception ignored) {
            return fallback;
        }
    }

    private static String mediaFormatString(MediaFormat format, String key, String fallback) {
        try {
            String value = format.containsKey(key) ? format.getString(key) : fallback;
            return value != null ? value : fallback;
        } catch (Exception ignored) {
            return fallback;
        }
    }

    private static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private static String safeMessage(Exception ex) {
        String message = ex.getMessage();
        return message != null ? message : "";
    }

    private static final class NalUnit {
        final byte[] bytes;

        NalUnit(byte[] bytes) {
            this.bytes = bytes;
        }
    }

    private static final class DecodeResult {
        String decoderName = "";
        String outputMime = "";
        String csdSource = "";
        boolean lowLatencyFeatureSupported;
        boolean lowLatencyConfigRequested;
        boolean lowLatencyParameterSucceeded;
        int outputWidth;
        int outputHeight;
        int spsBytes;
        int ppsBytes;
        int codecConfigPacketsSkipped;
        int inputBufferCount;
        long inputBytes;
        boolean inputEosQueued;
        int outputFormatChanges;
        int outputBufferCount;
        int decodedFrameCount;
        long outputBytes;
        boolean outputEosSeen;
        long firstOutputPtsUs = -1L;
        long lastOutputPtsUs = -1L;
        long decodeStartElapsedNs;
        long decodeEndElapsedNs;
        String lastError = "";
    }
}
