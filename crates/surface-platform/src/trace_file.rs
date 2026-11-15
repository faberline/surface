//! Native trace file sink for `SURFACE_TRACE=<path>`. Feature `devtools`.
//! See `docs/trace.md`.
use std::path::{Path, PathBuf};
use surface_app::trace::Recorder;

/// The environment variable that turns the file sink on.
pub(crate) const ENV: &str = "SURFACE_TRACE";

/// Milliseconds since the sink was made. Copy, so the recorder's clock and
/// the host's `Present` timing share one base.
#[derive(Clone, Copy)]
pub(crate) struct TraceClock;
impl TraceClock {
    pub(crate) fn now_ms(&self) -> f64 {
        unimplemented!()
    }
}

/// A trace file, created new, mode 0600. Records are appended as JSON
/// lines; the buffer is flushed by `finish` and on drop.
pub(crate) struct TraceFile {
    pub(crate) clock: TraceClock,
}
impl TraceFile {
    /// `Ok(None)` when `value` is unset or empty.
    pub(crate) fn from_env_value(
        value: Option<std::ffi::OsString>,
    ) -> Result<Option<Self>, String> {
        let _ = value;
        unimplemented!()
    }
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let _ = path;
        unimplemented!()
    }
    pub(crate) fn path(&self) -> &PathBuf {
        unimplemented!()
    }
    /// The clock to give `Recorder::new`.
    pub(crate) fn recorder_clock(&self) -> Box<dyn FnMut() -> f64> {
        unimplemented!()
    }
    /// Append the records not written yet.
    pub(crate) fn drain(&mut self, recorder: &Recorder) {
        let _ = recorder;
        unimplemented!()
    }
    /// Write a host `Error` record, unless the runtime just recorded the
    /// same message.
    pub(crate) fn error(&mut self, recorder: &Recorder, message: &str) {
        let _ = (recorder, message);
        unimplemented!()
    }
    pub(crate) fn finish(&mut self) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_path(name: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        std::env::temp_dir().join(format!(
            "surface-trace-test-{}-{}-{name}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ))
    }
    fn recorder(file: &TraceFile) -> Recorder {
        Recorder::new(file.recorder_clock())
    }
    fn lines(path: &Path) -> Vec<serde_json::Value> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).expect("line is JSON"))
            .collect()
    }

    #[test]
    fn unset_or_empty_env_makes_no_file() {
        assert!(TraceFile::from_env_value(None).unwrap().is_none());
        assert!(TraceFile::from_env_value(Some("".into()))
            .unwrap()
            .is_none());
    }

    #[test]
    fn env_path_creates_the_file() {
        let path = temp_path("env");
        let file = TraceFile::from_env_value(Some(path.clone().into_os_string()))
            .unwrap()
            .expect("enabled");
        assert_eq!(file.path(), &path);
        assert!(path.exists());
        drop(file);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn existing_file_is_an_error_and_is_not_touched() {
        let path = temp_path("existing");
        std::fs::write(&path, "keep me\n").unwrap();
        let error = TraceFile::open(&path).err().expect("must fail");
        assert!(error.contains(&path.display().to_string()), "{error}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep me\n");
        std::fs::remove_file(&path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn file_mode_is_0600() {
        use std::os::unix::fs::PermissionsExt;
        let path = temp_path("mode");
        let file = TraceFile::open(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        drop(file);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn drain_writes_each_record_once_and_drop_flushes() {
        let path = temp_path("drain");
        let mut file = TraceFile::open(&path).unwrap();
        let mut rec = recorder(&file);
        rec.record_present(1., 2.);
        file.drain(&rec);
        file.drain(&rec);
        rec.record_present(3., 4.);
        file.drain(&rec);
        drop(file); // no finish: drop must flush
        let got = lines(&path);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0]["seq"], 0);
        assert_eq!(got[0]["kind"], "Present");
        assert_eq!(got[1]["seq"], 1);
        assert_eq!(got[1]["dur_ms"], 4.0);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn clock_is_finite_and_does_not_run_backwards() {
        let path = temp_path("clock");
        let file = TraceFile::open(&path).unwrap();
        let mut clock = file.recorder_clock();
        let a = clock();
        let b = clock();
        assert!(a.is_finite() && b >= a && a >= 0.);
        assert!(file.clock.now_ms() >= b);
        drop(file);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn host_error_is_written_as_an_error_record() {
        let path = temp_path("error");
        let mut file = TraceFile::open(&path).unwrap();
        let mut rec = recorder(&file);
        rec.record_present(1., 1.);
        file.error(&rec, "surface lost \"x\"");
        drop(file);
        let got = lines(&path);
        assert_eq!(got.len(), 2);
        assert_eq!(got[1]["kind"], "Error");
        assert_eq!(got[1]["seq"], 1);
        assert_eq!(got[1]["data"]["message"], "surface lost \"x\"");
        assert!(got[1]["t_ms"].as_f64().unwrap().is_finite());
        std::fs::remove_file(&path).unwrap();
    }
}
