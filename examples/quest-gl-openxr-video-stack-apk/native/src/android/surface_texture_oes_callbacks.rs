use std::sync::{
    atomic::{AtomicI64, AtomicU64, Ordering},
    Mutex, OnceLock,
};

use jni::{
    objects::{JClass, JString},
    sys::{jint, jlong},
    JNIEnv,
};

use super::{log_info, VIEW_COUNT};

static OES_DECODE_CALLBACKS: OnceLock<OesDecodeCallbackState> = OnceLock::new();

struct OesDecodeCallbackState {
    frame_available_counts: [AtomicU64; VIEW_COUNT],
    latest_sequences: [AtomicU64; VIEW_COUNT],
    latest_queued_pts_us: [AtomicI64; VIEW_COUNT],
    report_sequence: AtomicU64,
    latest_report: Mutex<Option<String>>,
    projection_metadata_reports: Mutex<[Option<String>; VIEW_COUNT]>,
}

impl OesDecodeCallbackState {
    fn new() -> Self {
        Self {
            frame_available_counts: [AtomicU64::new(0), AtomicU64::new(0)],
            latest_sequences: [AtomicU64::new(0), AtomicU64::new(0)],
            latest_queued_pts_us: [AtomicI64::new(-1), AtomicI64::new(-1)],
            report_sequence: AtomicU64::new(0),
            latest_report: Mutex::new(None),
            projection_metadata_reports: Mutex::new([None, None]),
        }
    }

    fn reset(&self) {
        for index in 0..VIEW_COUNT {
            self.frame_available_counts[index].store(0, Ordering::Relaxed);
            self.latest_sequences[index].store(0, Ordering::Relaxed);
            self.latest_queued_pts_us[index].store(-1, Ordering::Relaxed);
        }
        self.report_sequence.store(0, Ordering::Relaxed);
        if let Ok(mut latest_report) = self.latest_report.lock() {
            *latest_report = None;
        }
        if let Ok(mut reports) = self.projection_metadata_reports.lock() {
            *reports = [None, None];
        }
    }

    fn mark_frame_available(&self, view_index: usize, sequence: u64, queued_pts_us: i64) {
        if view_index >= VIEW_COUNT {
            return;
        }
        self.latest_sequences[view_index].store(sequence, Ordering::Relaxed);
        self.latest_queued_pts_us[view_index].store(queued_pts_us, Ordering::Relaxed);
        self.frame_available_counts[view_index].fetch_add(1, Ordering::Relaxed);
    }

    fn frame_snapshot(&self, view_index: usize) -> (u64, u64, i64) {
        (
            self.frame_available_counts[view_index].load(Ordering::Relaxed),
            self.latest_sequences[view_index].load(Ordering::Relaxed),
            self.latest_queued_pts_us[view_index].load(Ordering::Relaxed),
        )
    }

    fn record_report(&self, report: String) {
        if let Some(view_index) = projection_metadata_report_view_index(&report) {
            if let Ok(mut reports) = self.projection_metadata_reports.lock() {
                reports[view_index] = Some(report.clone());
            }
        }
        if let Ok(mut latest_report) = self.latest_report.lock() {
            *latest_report = Some(report);
            self.report_sequence.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn latest_report_after(&self, last_seen_sequence: &mut u64) -> Option<String> {
        let sequence = self.report_sequence.load(Ordering::Relaxed);
        if sequence == *last_seen_sequence {
            return None;
        }
        *last_seen_sequence = sequence;
        self.latest_report
            .lock()
            .ok()
            .and_then(|report| report.clone())
    }

    fn projection_metadata_report_snapshot(&self) -> [Option<String>; VIEW_COUNT] {
        self.projection_metadata_reports
            .lock()
            .map(|reports| [reports[0].clone(), reports[1].clone()])
            .unwrap_or([None, None])
    }
}

fn projection_metadata_report_view_index(report: &str) -> Option<usize> {
    let report = serde_json::from_str::<serde_json::Value>(report).ok()?;
    report.get("header_projection_metadata")?;
    report_view_index(&report)
}

pub(super) fn report_view_index(report: &serde_json::Value) -> Option<usize> {
    report
        .get("view_index")
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value < VIEW_COUNT)
}

fn oes_decode_callbacks() -> &'static OesDecodeCallbackState {
    OES_DECODE_CALLBACKS.get_or_init(OesDecodeCallbackState::new)
}

pub(super) fn reset_decode_callbacks() {
    oes_decode_callbacks().reset();
}

pub(super) fn decode_frame_snapshot(view_index: usize) -> (u64, u64, i64) {
    oes_decode_callbacks().frame_snapshot(view_index)
}

pub(super) fn latest_decode_report_after(last_seen_sequence: &mut u64) -> Option<String> {
    oes_decode_callbacks().latest_report_after(last_seen_sequence)
}

pub(super) fn projection_metadata_report_snapshot() -> [Option<String>; VIEW_COUNT] {
    oes_decode_callbacks().projection_metadata_report_snapshot()
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_BrokerH264OesDecodeProbe_nativeBrokerH264FrameAvailable(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    view_index: jint,
    sequence: jlong,
    queued_pts_us: jlong,
) {
    let Ok(view_index) = usize::try_from(view_index) else {
        return;
    };
    let sequence = u64::try_from(sequence).unwrap_or(0);
    oes_decode_callbacks().mark_frame_available(view_index, sequence, queued_pts_us);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_BrokerH264OesDecodeProbe_nativeBrokerH264DecodeReport(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    report_json: JString<'_>,
) {
    let report = env
        .get_string(&report_json)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "{\"event\":\"invalidJniString\"}".to_string());
    log_info(format!("Rusty XR broker H.264 OES decode report {report}"));
    oes_decode_callbacks().record_report(report);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_DirectCamera2OesProbe_nativeDirectCamera2OesFrameAvailable(
    _env: JNIEnv<'_>,
    _class: JClass<'_>,
    view_index: jint,
    sequence: jlong,
    queued_pts_us: jlong,
) {
    let Ok(view_index) = usize::try_from(view_index) else {
        return;
    };
    let sequence = u64::try_from(sequence).unwrap_or(0);
    oes_decode_callbacks().mark_frame_available(view_index, sequence, queued_pts_us);
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_opengles_DirectCamera2OesProbe_nativeDirectCamera2OesReport(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    report_json: JString<'_>,
) {
    let report = env
        .get_string(&report_json)
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "{\"event\":\"invalidJniString\"}".to_string());
    log_info(format!("Rusty XR direct Camera2 OES report {report}"));
    oes_decode_callbacks().record_report(report);
}
