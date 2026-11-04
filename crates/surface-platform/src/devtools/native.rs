use crate::{HostError, UserEvent};
use jet_bridge_protocol::{AuthenticatedRequest, NativeSession, Response, VERSION};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use winit::event_loop::EventLoopProxy;

pub(crate) struct Bridge {
    path: PathBuf,
    token: String,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Bridge {
    pub(crate) fn install(
        proxy: EventLoopProxy<UserEvent>,
        application: &str,
    ) -> Result<Option<Self>, HostError> {
        let Some(path) = std::env::var_os("SURFACE_BRIDGE_SESSION").map(PathBuf::from) else {
            return Ok(None);
        };
        Self::start(path, proxy, application)
            .map(Some)
            .map_err(|error| HostError(format!("native bridge: {error}")))
    }
    fn start(
        path: PathBuf,
        proxy: EventLoopProxy<UserEvent>,
        application: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|error| format!("session token: {error}"))?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let session = NativeSession {
            version: VERSION,
            endpoint: listener.local_addr()?.to_string(),
            token: token.clone(),
            pid: std::process::id(),
            application: application.into(),
        };
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&path)?;
        let stop = Arc::new(AtomicBool::new(false));
        let mut bridge = Self {
            path,
            token: token.clone(),
            stop: stop.clone(),
            worker: None,
        };
        serde_json::to_writer(&mut file, &session)?;
        file.flush()?;
        bridge.worker = Some(std::thread::Builder::new().name("surface-bridge".into()).spawn(move || {
            while !stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let response = receive(&mut stream, &stop).and_then(|bytes| {
                            let request: AuthenticatedRequest = serde_json::from_slice(&bytes).map_err(|_| "invalid request")?;
                            let id = request.request.id;
                            if request.token != token {
                                return Ok(Response::error(id, "unauthorized", "invalid application session"));
                            }
                            let (sender, receiver) = mpsc::sync_channel(1);
                            if proxy.send_event(UserEvent::Bridge { request:request.request, reply:super::Reply::Native(sender) }).is_err() {
                                return Ok(Response::error(id, "host_closed", "the application event loop closed"));
                            }
                            Ok(receiver.recv_timeout(Duration::from_secs(5)).unwrap_or_else(|_| Response::error(id, "host_timeout", "the application did not reply; a submitted action may already have run")))
                        }).unwrap_or_else(|message| Response::error(0, "invalid_request", message));
                        if let Ok(mut bytes) = serde_json::to_vec(&response) {
                            bytes.push(b'\n');
                            let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                            let _ = stream.write_all(&bytes);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(20)),
                    Err(_) => break,
                }
            }
        })?);
        Ok(bridge)
    }
}
fn receive(stream: &mut TcpStream, stop: &AtomicBool) -> Result<Vec<u8>, &'static str> {
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|_| "cannot read application request")?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    while Instant::now() < deadline && !stop.load(Ordering::Acquire) {
        match stream.read(&mut chunk) {
            Ok(0) => return Err("incomplete request"),
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.len() > 1024 * 1024 {
                    return Err("request exceeds 1 MiB");
                }
                if let Some(end) = bytes.iter().position(|byte| *byte == b'\n') {
                    bytes.truncate(end);
                    return Ok(bytes);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return Err("cannot read application request"),
        }
    }
    Err("request timed out")
}
impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let owned = std::fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<NativeSession>(&bytes).ok())
            .is_some_and(|session| {
                session.token == self.token && session.pid == std::process::id()
            });
        if owned {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
