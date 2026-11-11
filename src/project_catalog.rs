//! Immutable named snapshots. Each save creates a new file; existing projects
//! are never overwritten. A single ProjectService owns the catalog writer.
use std::{fs::{self,File,OpenOptions},io::{Read,Write},path::PathBuf};
use crate::{project,workspace::WorkspaceState};
type Result<T> = std::result::Result<T,String>;
const MAGIC:&[u8;8]=b"RBCAT001";
const MAX_NAME:usize=120;
const MAX_PROJECTS:usize=256;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ProjectId(pub(crate) u64);
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct ProjectEntry { pub id:ProjectId, pub name:String }
pub struct ProjectCatalog { directory:PathBuf }
impl ProjectCatalog {
    pub fn open(directory:PathBuf)->Result<Self> {
        fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
        Ok(Self{directory})
    }
    fn path(&self,id:ProjectId)->PathBuf {self.directory.join(format!("{:016x}.rblend",id.0))}
    fn ids(&self)->Result<Vec<ProjectId>> {
        let mut ids=Vec::new();
        for entry in fs::read_dir(&self.directory).map_err(|e|e.to_string())? {
            let entry=entry.map_err(|e|e.to_string())?;
            let name=entry.file_name();let Some(name)=name.to_str() else {continue;};
            let Some(stem)=name.strip_suffix(".rblend") else {continue;};
            if stem.len()!=16 || !stem.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)) {continue;}
            let id=ProjectId(u64::from_str_radix(stem,16).map_err(|_|"Invalid project ID")?);
            // Do not follow externally introduced symlinks.
            if !entry.file_type().map_err(|e|e.to_string())?.is_file() {return Err("Project is not a regular file".into());}
            ids.push(id);
            if ids.len()>MAX_PROJECTS {return Err("Project catalog limit exceeded".into());}
        }
        ids.sort_by_key(|id|std::cmp::Reverse(id.0)); Ok(ids)
    }
    fn header(file:&mut File)->Result<String> {
        let mut header=[0u8;12];file.read_exact(&mut header).map_err(|e|e.to_string())?;
        if &header[..8]!=MAGIC {return Err("Unsupported catalog file".into());}
        let n=u32::from_le_bytes(header[8..].try_into().unwrap()) as usize;
        if n==0||n>MAX_NAME {return Err("Invalid project name length".into());}
        let mut name=vec![0;n];file.read_exact(&mut name).map_err(|e|e.to_string())?;
        let name=String::from_utf8(name).map_err(|_|"Invalid project name")?;
        Self::validate_name(&name)?;Ok(name)
    }
    fn validate_name(name:&str)->Result<()> {
        if name.trim().is_empty()||name.len()>MAX_NAME||name.chars().any(char::is_control) {
            return Err("Project name must contain 1-120 UTF-8 bytes without control characters".into());
        } Ok(())
    }
    pub fn list(&self)->Result<Vec<ProjectEntry>> {
        self.ids()?.into_iter().map(|id|{
            let mut file=File::open(self.path(id)).map_err(|e|e.to_string())?;
            let name=Self::header(&mut file).unwrap_or_else(|_|format!("UNREADABLE {:04}",id.0));
            Ok(ProjectEntry{id,name})
        }).collect()
    }
    pub fn load(&self,id:ProjectId)->Result<WorkspaceState> {
        if !self.ids()?.contains(&id) {return Err("Project not found".into());}
        let mut file=File::open(self.path(id)).map_err(|e|e.to_string())?;Self::header(&mut file)?;
        let mut bytes=Vec::new();file.take(project::MAX_FILE_BYTES as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        project::decode(&bytes).map(|(_,state)|state)
    }
    /// Preserve the previous committed workspace before atomically replacing it.
    pub fn save_current(&mut self,id:ProjectId,state:&WorkspaceState)->Result<ProjectEntry> {
        let previous=self.load(id)?;
        let mut original=File::open(self.path(id)).map_err(|e|e.to_string())?;
        let name=Self::header(&mut original)?;
        let bytes=project::encode(state,0)?;
        self.backup(&previous)?;
        let temp=self.directory.join("update.tmp");
        let result=(||{
            let mut file=File::create(&temp).map_err(|e|e.to_string())?;
            file.write_all(MAGIC).and_then(|_|file.write_all(&(name.len() as u32).to_le_bytes()))
                .and_then(|_|file.write_all(name.as_bytes())).and_then(|_|file.write_all(&bytes))
                .and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
            drop(file);
            fs::rename(&temp,self.path(id)).map_err(|e|e.to_string())?;
            File::open(&self.directory).and_then(|f|f.sync_all()).map_err(|e|e.to_string())?;
            Ok(ProjectEntry{id,name})
        })();
        if result.is_err(){let _=fs::remove_file(temp);}result
    }
    pub fn save_copy(&mut self,name:Option<&str>,state:&WorkspaceState)->Result<ProjectEntry> {
        self.save_named(name,"PROJECT",state)
    }
    pub fn backup(&mut self,state:&WorkspaceState)->Result<ProjectEntry> {self.save_named(None,"BACKUP",state)}
    fn save_named(&mut self,name:Option<&str>,prefix:&str,state:&WorkspaceState)->Result<ProjectEntry> {
        let ids=self.ids()?;if ids.len()>=MAX_PROJECTS {return Err("Project catalog full; switch cancelled to protect current work".into());}
        let id=ProjectId(ids.first().map_or(Some(1),|id|id.0.checked_add(1)).ok_or("Project ID exhausted")?);
        let name=name.map(str::to_owned).unwrap_or_else(||format!("{prefix} {:04}",id.0));Self::validate_name(&name)?;
        let bytes=project::encode(state,0)?;
        let temp=self.directory.join("pending.tmp");
        // Only this catalog's single worker writes this private temporary file.
        let result=(|| {
            let mut file=OpenOptions::new().write(true).create(true).truncate(true).open(&temp).map_err(|e|e.to_string())?;
            file.write_all(MAGIC).and_then(|_|file.write_all(&(name.len() as u32).to_le_bytes()))
                .and_then(|_|file.write_all(name.as_bytes())).and_then(|_|file.write_all(&bytes))
                .and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
            drop(file);
            // Atomic no-clobber publication on Android's private filesystem.
            fs::hard_link(&temp,self.path(id)).map_err(|e|e.to_string())?;
            fs::remove_file(&temp).map_err(|e|e.to_string())?;
            File::open(&self.directory).and_then(|f|f.sync_all()).map_err(|e|e.to_string())?;
            Ok(ProjectEntry{id,name})
        })();
        if result.is_err() {let _=fs::remove_file(temp);} result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scene,Camera,Vec3,Mesh,viewport_ui::ToolMode};
    use std::sync::atomic::{AtomicU64,Ordering};
    static NEXT:AtomicU64=AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {fn new()->Self {let p=std::env::temp_dir().join(format!("rb-catalog-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));fs::create_dir(&p).unwrap();Self(p)}}
    impl Drop for Temp {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn state()->WorkspaceState {let mut scene=Scene::new();scene.add_object("Cube",Mesh::cube(1.0).unwrap()).unwrap();WorkspaceState{scene,camera:Camera::new(Vec3::ZERO,5.0,1.0).unwrap(),tool:ToolMode::Orbit,edit_mode:false,outliner_open:true,history:Default::default()}}
    #[test]fn copies_are_distinct_and_reopen_after_restart(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let s=state();let a=c.save_copy(Some("Motor Çalışması"),&s).unwrap();let b=c.save_copy(Some("Motor Çalışması"),&s).unwrap();assert_ne!(a.id,b.id);drop(c);let c=ProjectCatalog::open(d.0.clone()).unwrap();assert_eq!(c.list().unwrap(),vec![b,a.clone()]);assert_eq!(c.load(a.id).unwrap().scene,s.scene);}
    #[test]fn names_never_become_paths(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let entry=c.save_copy(Some("../outside"),&state()).unwrap();assert_eq!(c.list().unwrap()[0].name,"../outside");assert!(c.path(entry.id).parent()==Some(d.0.as_path()));assert_eq!(fs::read_dir(&d.0).unwrap().count(),1);}
    #[test]fn empty_and_control_names_rejected(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();for name in ["","   ","bad\nname"]{assert!(c.save_copy(Some(name),&state()).is_err());}assert!(c.list().unwrap().is_empty());}
    #[test]fn damaged_file_can_be_listed_but_not_opened(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let e=c.save_copy(None,&state()).unwrap();fs::write(c.path(e.id),b"broken").unwrap();assert!(c.list().unwrap()[0].name.starts_with("UNREADABLE"));assert!(c.load(e.id).is_err());}
    #[test]fn failed_save_preserves_existing_file(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let s=state();let e=c.save_copy(None,&s).unwrap();fs::create_dir(d.0.join("pending.tmp")).unwrap();assert!(c.save_copy(None,&s).is_err());assert_eq!(c.load(e.id).unwrap().scene,s.scene);}
    #[test]fn old_temporary_file_is_ignored(){let d=Temp::new();fs::write(d.0.join("pending.tmp"),b"partial").unwrap();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();assert!(c.list().unwrap().is_empty());assert!(c.save_copy(None,&state()).is_ok());}
    #[test]fn saving_same_id_preserves_previous_version(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let old=state();let entry=c.save_copy(Some("Motor"),&old).unwrap();let mut edited=old.clone();edited.scene=Scene::new();assert_eq!(c.save_current(entry.id,&edited).unwrap(),entry);assert!(c.load(entry.id).unwrap().scene.is_empty());let backup=c.list().unwrap().into_iter().find(|e|e.id!=entry.id).unwrap();assert_eq!(c.load(backup.id).unwrap().scene,old.scene);}
    #[test]fn failed_replacement_keeps_original(){let d=Temp::new();let mut c=ProjectCatalog::open(d.0.clone()).unwrap();let old=state();let entry=c.save_copy(None,&old).unwrap();fs::create_dir(d.0.join("update.tmp")).unwrap();assert!(c.save_current(entry.id,&old).is_err());assert_eq!(c.load(entry.id).unwrap().scene,old.scene);}

}
