package com.example.rustyxr.broker;

import android.content.Intent;
import android.os.Bundle;

final class BrokerRuntimeConfig {
    static final String EXTRA_OSC_ENABLED = "rustyxr.oscEnabled";
    static final String EXTRA_OSC_HOST = "rustyxr.oscHost";
    static final String EXTRA_OSC_PORT = "rustyxr.oscPort";
    static final String EXTRA_OSC_ADDRESS = "rustyxr.oscAddress";
    static final String EXTRA_OSC_INGRESS_ENABLED = "rustyxr.oscIngressEnabled";
    static final String EXTRA_OSC_INGRESS_PORT = "rustyxr.oscIngressPort";
    static final String EXTRA_OSC_INGRESS_ADDRESS = "rustyxr.oscIngressAddress";
    static final String EXTRA_POLAR_PMD_ENABLED = "rustyxr.polarPmdEnabled";
    static final String EXTRA_POLAR_ENABLED = "rustyxr.polarEnabled";
    static final String EXTRA_POLAR_DEVICE_ADDRESS = "rustyxr.polarDeviceAddress";
    static final String EXTRA_POLAR_SCAN_TIMEOUT_MS = "rustyxr.polarScanTimeoutMs";

    static final int DEFAULT_OSC_PORT = 9000;
    static final String DEFAULT_OSC_ADDRESS = "/rusty-xr/broker/latency";
    static final String DEFAULT_OSC_INGRESS_ADDRESS = "/rusty-xr/drive/radius";
    static final long DEFAULT_POLAR_SCAN_TIMEOUT_MS = 30_000L;

    final boolean oscEnabled;
    final String oscHost;
    final int oscPort;
    final String oscAddress;
    final boolean oscIngressEnabled;
    final int oscIngressPort;
    final String oscIngressAddress;
    final boolean polarPmdEnabled;
    final String polarDeviceAddress;
    final long polarScanTimeoutMs;

    private BrokerRuntimeConfig(
        boolean oscEnabled,
        String oscHost,
        int oscPort,
        String oscAddress,
        boolean oscIngressEnabled,
        int oscIngressPort,
        String oscIngressAddress,
        boolean polarPmdEnabled,
        String polarDeviceAddress,
        long polarScanTimeoutMs) {
        this.oscEnabled = oscEnabled;
        this.oscHost = oscHost != null ? oscHost.trim() : "";
        this.oscPort = oscPort;
        this.oscAddress = oscAddress != null && oscAddress.trim().length() > 0
            ? oscAddress.trim()
            : DEFAULT_OSC_ADDRESS;
        this.oscIngressEnabled = oscIngressEnabled;
        this.oscIngressPort = oscIngressPort;
        this.oscIngressAddress = oscIngressAddress != null && oscIngressAddress.trim().length() > 0
            ? oscIngressAddress.trim()
            : DEFAULT_OSC_INGRESS_ADDRESS;
        this.polarPmdEnabled = polarPmdEnabled;
        this.polarDeviceAddress = polarDeviceAddress != null ? polarDeviceAddress.trim() : "";
        this.polarScanTimeoutMs = polarScanTimeoutMs > 0L
            ? polarScanTimeoutMs
            : DEFAULT_POLAR_SCAN_TIMEOUT_MS;
    }

    static BrokerRuntimeConfig oscIngressConfig(boolean enabled, int port, String address) {
        return new BrokerRuntimeConfig(
            false,
            "",
            DEFAULT_OSC_PORT,
            DEFAULT_OSC_ADDRESS,
            enabled,
            port,
            address,
            false,
            "",
            DEFAULT_POLAR_SCAN_TIMEOUT_MS);
    }

    static BrokerRuntimeConfig fromIntent(Intent intent) {
        Bundle extras = intent != null ? intent.getExtras() : null;
        boolean oscEnabled = getBoolean(extras, false, EXTRA_OSC_ENABLED, "oscEnabled");
        String oscHost = getString(extras, "", EXTRA_OSC_HOST, "oscHost");
        int oscPort = getInt(extras, DEFAULT_OSC_PORT, EXTRA_OSC_PORT, "oscPort");
        String oscAddress = getString(extras, DEFAULT_OSC_ADDRESS, EXTRA_OSC_ADDRESS, "oscAddress");
        boolean oscIngressEnabled = getBoolean(extras, false, EXTRA_OSC_INGRESS_ENABLED, "oscIngressEnabled");
        int oscIngressPort = getInt(extras, DEFAULT_OSC_PORT, EXTRA_OSC_INGRESS_PORT, "oscIngressPort");
        String oscIngressAddress = getString(
            extras,
            DEFAULT_OSC_INGRESS_ADDRESS,
            EXTRA_OSC_INGRESS_ADDRESS,
            "oscIngressAddress");
        boolean polarPmdEnabled = getBoolean(extras, false, EXTRA_POLAR_PMD_ENABLED, EXTRA_POLAR_ENABLED, "polarPmdEnabled", "polarEnabled");
        String polarDeviceAddress = getString(extras, "", EXTRA_POLAR_DEVICE_ADDRESS, "polarDeviceAddress");
        long polarScanTimeoutMs = getLong(
            extras,
            DEFAULT_POLAR_SCAN_TIMEOUT_MS,
            EXTRA_POLAR_SCAN_TIMEOUT_MS,
            "polarScanTimeoutMs");
        return new BrokerRuntimeConfig(
            oscEnabled,
            oscHost,
            oscPort,
            oscAddress,
            oscIngressEnabled,
            oscIngressPort,
            oscIngressAddress,
            polarPmdEnabled,
            polarDeviceAddress,
            polarScanTimeoutMs);
    }

    private static boolean getBoolean(Bundle extras, boolean fallback, String... keys) {
        if (extras == null) {
            return fallback;
        }

        for (String key : keys) {
            if (!extras.containsKey(key)) {
                continue;
            }

            Object value = extras.get(key);
            if (value instanceof Boolean) {
                return ((Boolean) value).booleanValue();
            }

            if (value instanceof String) {
                String raw = ((String) value).trim();
                return "true".equalsIgnoreCase(raw) || "1".equals(raw) || "yes".equalsIgnoreCase(raw);
            }
        }

        return fallback;
    }

    private static int getInt(Bundle extras, int fallback, String... keys) {
        if (extras == null) {
            return fallback;
        }

        for (String key : keys) {
            if (!extras.containsKey(key)) {
                continue;
            }

            Object value = extras.get(key);
            if (value instanceof Number) {
                return ((Number) value).intValue();
            }

            if (value instanceof String) {
                try {
                    return Integer.parseInt(((String) value).trim());
                } catch (NumberFormatException ignored) {
                    return fallback;
                }
            }
        }

        return fallback;
    }

    private static long getLong(Bundle extras, long fallback, String... keys) {
        if (extras == null) {
            return fallback;
        }

        for (String key : keys) {
            if (!extras.containsKey(key)) {
                continue;
            }

            Object value = extras.get(key);
            if (value instanceof Number) {
                return ((Number) value).longValue();
            }

            if (value instanceof String) {
                try {
                    return Long.parseLong(((String) value).trim());
                } catch (NumberFormatException ignored) {
                    return fallback;
                }
            }
        }

        return fallback;
    }

    private static String getString(Bundle extras, String fallback, String... keys) {
        if (extras == null) {
            return fallback;
        }

        for (String key : keys) {
            if (!extras.containsKey(key)) {
                continue;
            }

            Object value = extras.get(key);
            if (value != null) {
                return value.toString();
            }
        }

        return fallback;
    }
}
