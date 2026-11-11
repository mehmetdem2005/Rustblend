//! A single writer with one replaceable pending snapshot. Encoding and disk IO
//! happen outside the UI thread and outside the queue lock.
use std::{sync::{Arc, Condvar, Mutex}, thread::{self, JoinHandle}};
use crate::{project::RecoveryStore, workspace::WorkspaceState};

type SaveResult = Result<(), String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveStatus { Idle, Saving, Saved, Failed(String) }

#[derive(Default)]
struct Queue {
    pending: Option<(u64, WorkspaceState)>,
    completed: Option<(u64, SaveResult)>,
    stop: bool,
}

pub struct RecoveryWorker {
    shared: Arc<(Mutex<Queue>, Condvar)>,
    thread: Option<JoinHandle<()>>,
    requested: u64,
}

impl RecoveryWorker {
    pub fn new(mut store: RecoveryStore, wake: impl Fn() + Send + 'static) -> Result<Self, String> {
        Self::spawn(move |state| store.save(state), wake)
    }

    fn spawn(
        mut save: impl FnMut(&WorkspaceState) -> SaveResult + Send + 'static,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, String> {
        let shared = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let worker = shared.clone();
        let thread = thread::Builder::new().name("rustblend-recovery".into()).spawn(move || {
            loop {
                let request = {
                    let (lock, ready) = &*worker;
                    let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
                    while queue.pending.is_none() && !queue.stop {
                        queue = ready.wait(queue).unwrap_or_else(|e| e.into_inner());
                    }
                    match queue.pending.take() {
                        Some(request) => request,
                        None => break,
                    }
                };
                let (revision, state) = request;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| save(&state)))
                    .unwrap_or_else(|_| Err("Recovery writer panicked".into()));
                if let Err(error) = &result {
                    eprintln!("RUSTBLEND recovery write failed: {error}");
                }
                // Release the large snapshot before notifying the UI.
                drop(state);
                {
                    let (lock, ready) = &*worker;
                    let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
                    queue.completed = Some((revision, result));
                    ready.notify_all();
                }
                wake();
            }
        }).map_err(|e| e.to_string())?;
        Ok(Self { shared, thread: Some(thread), requested: 0 })
    }

    pub fn submit(&mut self, state: WorkspaceState) -> Result<(), String> {
        let revision = self.requested.checked_add(1).ok_or("Recovery revision exhausted")?;
        let (lock, ready) = &*self.shared;
        let mut queue = lock.lock().unwrap_or_else(|e| e.into_inner());
        if queue.stop { return Err("Recovery writer stopped".into()); }
        // At most one in-flight and one queued snapshot. Drop displaced data
        // after releasing the lock so the writer cannot stall behind its free.
        let displaced = queue.pending.replace((revision, state));
        self.requested = revision;
        ready.notify_one();
        drop(queue);
        drop(displaced);
        Ok(())
    }

    pub fn status(&self) -> SaveStatus {
        let queue = self.shared.0.lock().unwrap_or_else(|e| e.into_inner());
        match &queue.completed {
            Some((revision, result)) if *revision == self.requested => match result {
                Ok(()) => SaveStatus::Saved,
                Err(error) => SaveStatus::Failed(error.clone()),
            },
            _ if self.requested == 0 => SaveStatus::Idle,
            _ => SaveStatus::Saving,
        }
    }
}

impl Drop for RecoveryWorker {
    fn drop(&mut self) {
        {
            let (lock, ready) = &*self.shared;
            lock.lock().unwrap_or_else(|e| e.into_inner()).stop = true;
            ready.notify_one();
        }
        // Normal application exit drains the last pending request. Android
        // process termination cannot be guaranteed to run Rust destructors.
        if let Some(thread) = self.thread.take() { let _ = thread.join(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Camera, Mesh, Scene, Vec3, viewport_ui::ToolMode};
    use std::{sync::mpsc, time::Duration};
    const TIMEOUT: Duration = Duration::from_secs(5);
    fn state(name: &str) -> WorkspaceState {
        let mut scene = Scene::new(); scene.add_object(name, Mesh::cube(1.0).unwrap()).unwrap();
        WorkspaceState { scene, camera: Camera::new(Vec3::ZERO,5.0,1.0).unwrap(),
            tool: ToolMode::Orbit, edit_mode:false, outliner_open:true, history:Default::default() }
    }
    #[test]
    fn slow_writer_does_not_block_submit_and_only_latest_pending_is_saved() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (wake_tx, wake_rx) = mpsc::channel();
        let saved = Arc::new(Mutex::new(Vec::new())); let results=saved.clone();
        let mut first=true;
        let mut worker=RecoveryWorker::spawn(move |s| {
            if first { first=false; started_tx.send(()).unwrap(); release_rx.recv_timeout(TIMEOUT).unwrap(); }
            results.lock().unwrap().push(s.scene.objects()[0].name().to_owned()); Ok(())
        }, move || { let _=wake_tx.send(()); }).unwrap();
        worker.submit(state("first")).unwrap(); started_rx.recv_timeout(TIMEOUT).unwrap();
        for name in ["second","third","latest"] { worker.submit(state(name)).unwrap(); }
        assert_eq!(worker.status(),SaveStatus::Saving);
        release_tx.send(()).unwrap();
        wake_rx.recv_timeout(TIMEOUT).unwrap(); wake_rx.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(worker.status(),SaveStatus::Saved);
        drop(worker);
        assert_eq!(*saved.lock().unwrap(),vec!["first","latest"]);
    }
    #[test]
    fn old_completion_cannot_mark_newer_request_saved() {
        let (started_tx, started_rx)=mpsc::channel(); let (release_tx,release_rx)=mpsc::channel();
        let (wake_tx,wake_rx)=mpsc::channel();
        let mut worker=RecoveryWorker::spawn(move |_| {
            started_tx.send(()).unwrap(); release_rx.recv_timeout(TIMEOUT).unwrap(); Ok(())
        },move || {let _=wake_tx.send(());}).unwrap();
        worker.submit(state("one")).unwrap(); started_rx.recv_timeout(TIMEOUT).unwrap();
        worker.submit(state("two")).unwrap(); release_tx.send(()).unwrap();
        wake_rx.recv_timeout(TIMEOUT).unwrap(); started_rx.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(worker.status(),SaveStatus::Saving);
        release_tx.send(()).unwrap(); wake_rx.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(worker.status(),SaveStatus::Saved);
    }
    #[test]
    fn failed_write_is_reported_and_later_save_can_succeed() {
        let (tx,rx)=mpsc::channel(); let mut fail=true;
        let mut worker=RecoveryWorker::spawn(move |_| { if fail {fail=false;Err("disk full".into())} else {Ok(())} },move || {let _=tx.send(());}).unwrap();
        assert_eq!(worker.status(),SaveStatus::Idle);
        worker.submit(state("one")).unwrap(); rx.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(worker.status(),SaveStatus::Failed("disk full".into()));
        worker.submit(state("two")).unwrap(); rx.recv_timeout(TIMEOUT).unwrap();
        assert_eq!(worker.status(),SaveStatus::Saved);
    }
    #[test]
    fn shutdown_drains_pending_and_returns() {
        let count=Arc::new(Mutex::new(0)); let copy=count.clone();
        let mut worker=RecoveryWorker::spawn(move |_| {*copy.lock().unwrap()+=1;Ok(())},||{}).unwrap();
        worker.submit(state("last")).unwrap(); drop(worker);
        assert_eq!(*count.lock().unwrap(),1);
    }
    #[test]
    fn panic_becomes_failure_instead_of_permanent_saving() {
        let (tx,rx)=mpsc::channel();
        let mut worker=RecoveryWorker::spawn(|_|panic!("test sink"),move || {let _=tx.send(());}).unwrap();
        worker.submit(state("one")).unwrap(); rx.recv_timeout(TIMEOUT).unwrap();
        assert!(matches!(worker.status(),SaveStatus::Failed(_)));
    }
    #[test]
    fn normal_shutdown_commits_latest_snapshot_to_reopenable_store() {
        let directory = std::env::temp_dir().join(format!("rustblend-worker-{}-{}",
            std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
        let _cleanup = Cleanup(directory.clone());
        let (store, _) = RecoveryStore::open(directory.clone()).unwrap();
        let mut worker = RecoveryWorker::new(store, || {}).unwrap();
        worker.submit(state("old")).unwrap();
        worker.submit(state("latest")).unwrap();
        drop(worker);
        let (_, restored) = RecoveryStore::open(directory).unwrap();
        assert_eq!(restored.unwrap().scene.objects()[0].name(), "latest");
    }

}
