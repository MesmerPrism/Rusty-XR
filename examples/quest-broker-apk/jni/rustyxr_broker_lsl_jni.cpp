#include <jni.h>

#include <string>

extern "C" {
typedef void *lsl_streaminfo;
typedef void *lsl_outlet;
typedef void *lsl_inlet;

lsl_streaminfo lsl_create_streaminfo(
    const char *name,
    const char *type,
    int channel_count,
    double nominal_srate,
    int channel_format,
    const char *source_id);
void lsl_destroy_streaminfo(lsl_streaminfo info);
lsl_outlet lsl_create_outlet(lsl_streaminfo info, int chunk_size, int max_buffered);
void lsl_destroy_outlet(lsl_outlet outlet);
int lsl_push_sample_strtp(lsl_outlet outlet, const char **data, double timestamp, int pushthrough);
int lsl_resolve_byprop(
    lsl_streaminfo *buffer,
    unsigned int buffer_elements,
    const char *prop,
    const char *value,
    int minimum,
    double timeout);
lsl_inlet lsl_create_inlet(lsl_streaminfo info, int max_buflen, int max_chunklen, int recover);
void lsl_destroy_inlet(lsl_inlet inlet);
void lsl_open_stream(lsl_inlet inlet, double timeout, int *ec);
double lsl_pull_sample_str(lsl_inlet inlet, char **buffer, int buffer_elements, double timeout, int *ec);
void lsl_destroy_string(char *s);
double lsl_time_correction_ex(lsl_inlet inlet, double *remote_time, double *uncertainty, double timeout, int *ec);
double lsl_local_clock();
}

namespace {
constexpr int kLslStringFormat = 3;
thread_local std::string g_last_error;

struct OutletHandle {
    lsl_streaminfo info = nullptr;
    lsl_outlet outlet = nullptr;
};

struct InletHandle {
    lsl_inlet inlet = nullptr;
    double last_sample_timestamp = 0.0;
};

std::string ToUtf8(JNIEnv *env, jstring value) {
    if (value == nullptr) {
        return std::string();
    }

    const char *chars = env->GetStringUTFChars(value, nullptr);
    if (chars == nullptr) {
        return std::string();
    }

    std::string result(chars);
    env->ReleaseStringUTFChars(value, chars);
    return result;
}

void DestroyHandle(OutletHandle *handle) {
    if (handle == nullptr) {
        return;
    }

    if (handle->outlet != nullptr) {
        lsl_destroy_outlet(handle->outlet);
        handle->outlet = nullptr;
    }

    if (handle->info != nullptr) {
        lsl_destroy_streaminfo(handle->info);
        handle->info = nullptr;
    }

    delete handle;
}

void DestroyInletHandle(InletHandle *handle) {
    if (handle == nullptr) {
        return;
    }

    if (handle->inlet != nullptr) {
        lsl_destroy_inlet(handle->inlet);
        handle->inlet = nullptr;
    }

    delete handle;
}
} // namespace

extern "C" JNIEXPORT jlong JNICALL
Java_com_example_rustyxr_broker_NativeLslLatencyPublisher_nativeCreateOutlet(
    JNIEnv *env,
    jclass,
    jstring name,
    jstring type,
    jstring source_id,
    jint max_buffered_seconds) {
    g_last_error.clear();
    std::string name_utf8 = ToUtf8(env, name);
    std::string type_utf8 = ToUtf8(env, type);
    std::string source_utf8 = ToUtf8(env, source_id);

    if (name_utf8.empty()) {
        g_last_error = "stream name is empty";
        return 0;
    }

    auto *handle = new OutletHandle();
    handle->info = lsl_create_streaminfo(
        name_utf8.c_str(),
        type_utf8.empty() ? "rusty.xr.latency" : type_utf8.c_str(),
        1,
        0.0,
        kLslStringFormat,
        source_utf8.c_str());
    if (handle->info == nullptr) {
        g_last_error = "lsl_create_streaminfo returned null";
        DestroyHandle(handle);
        return 0;
    }

    handle->outlet = lsl_create_outlet(handle->info, 0, max_buffered_seconds > 0 ? max_buffered_seconds : 8);
    if (handle->outlet == nullptr) {
        g_last_error = "lsl_create_outlet returned null";
        DestroyHandle(handle);
        return 0;
    }

    return reinterpret_cast<jlong>(handle);
}

extern "C" JNIEXPORT jint JNICALL
Java_com_example_rustyxr_broker_NativeLslLatencyPublisher_nativePushStringSample(
    JNIEnv *env,
    jclass,
    jlong outlet_handle,
    jstring payload) {
    g_last_error.clear();
    auto *handle = reinterpret_cast<OutletHandle *>(outlet_handle);
    if (handle == nullptr || handle->outlet == nullptr) {
        g_last_error = "outlet handle is null";
        return -4;
    }

    std::string payload_utf8 = ToUtf8(env, payload);
    const char *sample[1] = {payload_utf8.c_str()};
    return lsl_push_sample_strtp(handle->outlet, sample, 0.0, 1);
}

extern "C" JNIEXPORT void JNICALL
Java_com_example_rustyxr_broker_NativeLslLatencyPublisher_nativeDestroyOutlet(
    JNIEnv *,
    jclass,
    jlong outlet_handle) {
    auto *handle = reinterpret_cast<OutletHandle *>(outlet_handle);
    DestroyHandle(handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_example_rustyxr_broker_NativeLslLatencyPublisher_nativeLastError(JNIEnv *env, jclass) {
    return env->NewStringUTF(g_last_error.c_str());
}

extern "C" JNIEXPORT jlong JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativeResolveStringInlet(
    JNIEnv *env,
    jclass,
    jstring property,
    jstring value,
    jdouble timeout_seconds) {
    g_last_error.clear();
    std::string property_utf8 = ToUtf8(env, property);
    std::string value_utf8 = ToUtf8(env, value);
    if (property_utf8.empty() || value_utf8.empty()) {
        g_last_error = "resolve property and value are required";
        return 0;
    }

    lsl_streaminfo infos[1] = {nullptr};
    int count = lsl_resolve_byprop(
        infos,
        1,
        property_utf8.c_str(),
        value_utf8.c_str(),
        1,
        timeout_seconds > 0 ? timeout_seconds : 10.0);
    if (count <= 0 || infos[0] == nullptr) {
        g_last_error = "no matching LSL stream resolved";
        return 0;
    }

    auto *handle = new InletHandle();
    handle->inlet = lsl_create_inlet(infos[0], 8, 1, 1);
    lsl_destroy_streaminfo(infos[0]);
    if (handle->inlet == nullptr) {
        g_last_error = "lsl_create_inlet returned null";
        DestroyInletHandle(handle);
        return 0;
    }

    return reinterpret_cast<jlong>(handle);
}

extern "C" JNIEXPORT jint JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativeOpenStream(
    JNIEnv *,
    jclass,
    jlong inlet_handle,
    jdouble timeout_seconds) {
    g_last_error.clear();
    auto *handle = reinterpret_cast<InletHandle *>(inlet_handle);
    if (handle == nullptr || handle->inlet == nullptr) {
        g_last_error = "inlet handle is null";
        return -4;
    }

    int ec = 0;
    lsl_open_stream(handle->inlet, timeout_seconds > 0 ? timeout_seconds : 5.0, &ec);
    if (ec != 0) {
        g_last_error = "lsl_open_stream failed";
    }
    return ec;
}

extern "C" JNIEXPORT jdoubleArray JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativeTimeCorrection(
    JNIEnv *env,
    jclass,
    jlong inlet_handle,
    jdouble timeout_seconds) {
    g_last_error.clear();
    double values[4] = {0.0, 0.0, 0.0, 0.0};
    auto *handle = reinterpret_cast<InletHandle *>(inlet_handle);
    if (handle == nullptr || handle->inlet == nullptr) {
        g_last_error = "inlet handle is null";
        values[3] = -4.0;
    } else {
        int ec = 0;
        values[0] = lsl_time_correction_ex(
            handle->inlet,
            &values[1],
            &values[2],
            timeout_seconds > 0 ? timeout_seconds : 5.0,
            &ec);
        values[3] = static_cast<double>(ec);
        if (ec != 0) {
            g_last_error = "lsl_time_correction_ex failed";
        }
    }

    jdoubleArray result = env->NewDoubleArray(4);
    if (result != nullptr) {
        env->SetDoubleArrayRegion(result, 0, 4, values);
    }
    return result;
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativePullStringSample(
    JNIEnv *env,
    jclass,
    jlong inlet_handle,
    jdouble timeout_seconds) {
    g_last_error.clear();
    auto *handle = reinterpret_cast<InletHandle *>(inlet_handle);
    if (handle == nullptr || handle->inlet == nullptr) {
        g_last_error = "inlet handle is null";
        return nullptr;
    }

    char *sample[1] = {nullptr};
    int ec = 0;
    double timestamp = lsl_pull_sample_str(
        handle->inlet,
        sample,
        1,
        timeout_seconds > 0 ? timeout_seconds : 1.0,
        &ec);
    if (ec != 0) {
        g_last_error = "lsl_pull_sample_str failed";
        return nullptr;
    }

    if (timestamp <= 0.0 || sample[0] == nullptr) {
        handle->last_sample_timestamp = 0.0;
        return nullptr;
    }

    handle->last_sample_timestamp = timestamp;
    jstring result = env->NewStringUTF(sample[0]);
    lsl_destroy_string(sample[0]);
    return result;
}

extern "C" JNIEXPORT jdouble JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativeLastSampleTimestamp(
    JNIEnv *,
    jclass,
    jlong inlet_handle) {
    auto *handle = reinterpret_cast<InletHandle *>(inlet_handle);
    return handle != nullptr ? handle->last_sample_timestamp : 0.0;
}

extern "C" JNIEXPORT jdouble JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativeLocalClock(JNIEnv *, jclass) {
    return lsl_local_clock();
}

extern "C" JNIEXPORT void JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativeDestroyInlet(
    JNIEnv *,
    jclass,
    jlong inlet_handle) {
    auto *handle = reinterpret_cast<InletHandle *>(inlet_handle);
    DestroyInletHandle(handle);
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_example_rustyxr_broker_NativeLslStringInletDiagnostics_nativeLastError(JNIEnv *env, jclass) {
    return env->NewStringUTF(g_last_error.c_str());
}
