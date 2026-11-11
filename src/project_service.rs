//! Explicit project operations are serialized and never coalesced like autosave.
use std::{sync::mpsc::{self,Receiver,SyncSender},thread::{self,JoinHandle}};
use crate::{project_catalog::{ProjectCatalog,ProjectEntry,ProjectId},workspace::WorkspaceState};
pub enum ProjectJob { List, SaveCopy(WorkspaceState), Open(ProjectId,WorkspaceState), New(WorkspaceState) }
pub struct ProjectResponse {pub entries:Vec<ProjectEntry>,pub workspace:Option<WorkspaceState>,pub message:String}
pub struct ProjectService {sender:Option<SyncSender<ProjectJob>>,results:Receiver<Result<ProjectResponse,String>>,thread:Option<JoinHandle<()>>,busy:bool}
fn execute(catalog:&mut ProjectCatalog,job:ProjectJob)->Result<ProjectResponse,String>{
    let (workspace,message)=match job {
        ProjectJob::List=>(None,"SAVE CREATES A NEW COPY".into()),
        ProjectJob::SaveCopy(state)=>{let entry=catalog.save_copy(None,&state)?;(None,format!("SAVED {}",entry.name))},
        ProjectJob::Open(id,current)=>{
            let loaded=catalog.load(id)?;
            // Fail closed: do not replace the live scene unless its backup commits.
            catalog.backup(&current)?;(Some(loaded),"OPENED - PREVIOUS WORK BACKED UP".into())
        },
        ProjectJob::New(current)=>{catalog.backup(&current)?;(Some(WorkspaceState::empty()),"NEW - PREVIOUS WORK BACKED UP".into())},
    };
    Ok(ProjectResponse{entries:catalog.list()?,workspace,message})
}
impl ProjectService {
    pub fn new(mut catalog:ProjectCatalog,wake:impl Fn()+Send+'static)->Result<Self,String>{
        let (sender,requests)=mpsc::sync_channel(1);let (completed,results)=mpsc::channel();
        let thread=thread::Builder::new().name("rustblend-projects".into()).spawn(move||{
            while let Ok(job)=requests.recv(){
                let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||execute(&mut catalog,job))).unwrap_or_else(|_|Err("Project worker panicked".into()));
                if completed.send(result).is_err(){break;}wake();
            }
        }).map_err(|e|e.to_string())?;
        Ok(Self{sender:Some(sender),results,thread:Some(thread),busy:false})
    }
    pub fn busy(&self)->bool{self.busy}
    pub fn submit(&mut self,job:ProjectJob)->Result<(),String>{
        if self.busy{return Err("Project operation already running".into());}
        self.sender.as_ref().ok_or("Project worker stopped")?.try_send(job).map_err(|e|e.to_string())?;self.busy=true;Ok(())
    }
    pub fn poll(&mut self)->Option<Result<ProjectResponse,String>>{
        match self.results.try_recv(){Ok(result)=>{self.busy=false;Some(result)},Err(mpsc::TryRecvError::Disconnected) if self.busy=>{self.busy=false;Some(Err("Project worker disconnected".into()))},_=>None}
    }
}
impl Drop for ProjectService {fn drop(&mut self){self.sender.take();if let Some(thread)=self.thread.take(){let _=thread.join();}}}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs,path::PathBuf,sync::atomic::{AtomicU64,Ordering},time::Duration};
    static NEXT:AtomicU64=AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {fn new()->Self{let p=std::env::temp_dir().join(format!("rb-service-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::create_dir(&p).unwrap();Self(p)}}
    impl Drop for Temp {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn current()->WorkspaceState {let mut s=WorkspaceState::empty();let id=s.scene.add_object("Unsaved work",crate::Mesh::cube(2.0).unwrap()).unwrap();s.scene.select(id).unwrap();s}
    #[test]fn opening_backs_up_unsaved_work_before_switch(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let empty=WorkspaceState::empty();let entry=c.save_copy(None,&empty).unwrap();let old=current();let r=execute(&mut c,ProjectJob::Open(entry.id,old.clone())).unwrap();assert!(r.workspace.unwrap().scene.is_empty());assert!(r.entries[0].name.starts_with("BACKUP"));assert_eq!(c.load(r.entries[0].id).unwrap().scene,old.scene);}
    #[test]fn failed_backup_blocks_new_project(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();fs::create_dir(d.0.join("pending.tmp")).unwrap();assert!(execute(&mut c,ProjectJob::New(current())).is_err());assert!(c.list().unwrap().is_empty());}
    #[test]fn new_project_preserves_previous_as_named_backup(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let old=current();let r=execute(&mut c,ProjectJob::New(old.clone())).unwrap();assert!(r.workspace.unwrap().scene.is_empty());assert_eq!(c.load(r.entries[0].id).unwrap().scene,old.scene);}
    #[test]fn service_is_single_flight_and_returns_saved_list(){let d=Temp::new();let c=ProjectCatalog::open(d.0.clone()).unwrap();let(tx,rx)=mpsc::channel();let mut service=ProjectService::new(c,move||{let _=tx.send(());}).unwrap();service.submit(ProjectJob::SaveCopy(current())).unwrap();assert!(service.busy());assert!(service.submit(ProjectJob::List).is_err());rx.recv_timeout(Duration::from_secs(5)).unwrap();let response=service.poll().unwrap().unwrap();assert_eq!(response.entries.len(),1);assert!(!service.busy());}
    #[test]fn missing_project_does_not_create_backup_or_return_replacement(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();assert!(execute(&mut c,ProjectJob::Open(ProjectId(999),current())).is_err());assert!(c.list().unwrap().is_empty());}

}
