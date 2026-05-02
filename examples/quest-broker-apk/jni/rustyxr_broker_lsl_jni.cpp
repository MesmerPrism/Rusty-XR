#include <jni.h>

#include <string>

extern "C" {
typedef void *lsl_streaminfo;
typedef void *lsl_outlet;

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
}

namespace {
constexpr int kLslStringFormat = 3;
thread_local std::string g_last_error;

struct OutletHandle {
    lsl_streaminfo info = nullptr;
    lsl_outlet outlet = nullptr;
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
