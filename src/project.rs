//! Versioned, bounded project codec and two-slot private recovery store.
//! No GPU resources or undo history are persisted. FNV detects accidental
//! corruption; it is not authentication. All decoded geometry is validated.
use std::{fs::{self, File}, io::{Read, Write}, path::{Path, PathBuf}};
use crate::math::Quaternion;
use crate::{Camera, Edge, Face, Mesh, Scene, Transform, Vec3,
    viewport_ui::ToolMode, workspace::WorkspaceState};

const MAGIC: &[u8; 8] = b"RBLEND01";
const VERSION: u32 = 1;
const HEADER: usize = 36;
pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_OBJECTS: usize = 10_000;
const MAX_NAME: usize = 4096;
const MAX_CORNERS: usize = 4096;

type Result<T> = std::result::Result<T, String>;
fn hash(header: &[u8], bytes: &[u8]) -> u64 {
    header.iter().chain(bytes.iter()).fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100000001b3))
}
fn u32w(out: &mut Vec<u8>, n: usize) -> Result<()> {
    out.extend_from_slice(&u32::try_from(n).map_err(|_| "Count overflow")?.to_le_bytes()); Ok(())
}
fn u64w(out: &mut Vec<u8>, n: u64) { out.extend_from_slice(&n.to_le_bytes()); }
fn f32w(out: &mut Vec<u8>, n: f32) { out.extend_from_slice(&n.to_le_bytes()); }
fn vecw(out: &mut Vec<u8>, v: Vec3) { for n in [v.x,v.y,v.z] { f32w(out,n); } }

pub fn encode(state: &WorkspaceState, generation: u64) -> Result<Vec<u8>> {
    // Preflight the complete serialized size before allocating the payload.
    let mut size = HEADER + 8 + 4 + 4 + state.scene.selection_count() * 8 + 8 + 7 * 4 + 3;
    if state.scene.object_count() > MAX_OBJECTS { return Err("Too many objects".into()); }
    for o in state.scene.objects() {
        let m = o.mesh();
        if o.name().len() > MAX_NAME || m.faces().iter().any(|f| f.vertices().len() > MAX_CORNERS) {
            return Err("Project element exceeds format limits".into());
        }
        size = size.checked_add(8 + 4 + o.name().len() + 40 + 12
            + m.vertex_count() * 12 + m.edge_count() * 8
            + m.faces().iter().map(|f| 4 + f.vertices().len() * 4).sum::<usize>())
            .ok_or("Project size overflow")?;
        if size > MAX_FILE_BYTES { return Err("Project exceeds 8 MiB recovery limit".into()); }
    }
    let mut p = Vec::with_capacity(size);
    u64w(&mut p, state.scene.next_object_id());
    u32w(&mut p, state.scene.object_count())?;
    for o in state.scene.objects() {
        u64w(&mut p,o.id().get()); u32w(&mut p,o.name().len())?;
        p.extend_from_slice(o.name().as_bytes());
        let t=o.transform(); vecw(&mut p,t.position());
        let q=t.rotation(); for n in [q.x,q.y,q.z,q.w] { f32w(&mut p,n); }
        vecw(&mut p,t.scale());
        let m=o.mesh(); u32w(&mut p,m.vertex_count())?;
        for &v in m.vertices() { vecw(&mut p,v); }
        u32w(&mut p,m.edge_count())?;
        for e in m.edges() { for n in e.vertices() { u32w(&mut p,n)?; } }
        u32w(&mut p,m.face_count())?;
        for f in m.faces() { u32w(&mut p,f.vertices().len())?; for &n in f.vertices() { u32w(&mut p,n)?; } }
    }
    u32w(&mut p,state.scene.selection_count())?;
    for id in state.scene.selected_ids() { u64w(&mut p,id.get()); }
    u64w(&mut p,state.scene.active_object_id().map_or(0,|id|id.get()));
    vecw(&mut p,state.camera.target());
    let (yaw,pitch)=state.camera.orbit_angles();
    for n in [state.camera.distance(),state.camera.aspect(),yaw,pitch] { f32w(&mut p,n); }
    p.push(match state.tool { ToolMode::Orbit=>0,ToolMode::Move=>1,ToolMode::Rotate=>2,ToolMode::Scale=>3 });
    p.push(u8::from(state.edit_mode)); p.push(u8::from(state.outliner_open));
    let mut out=Vec::with_capacity(HEADER+p.len()); out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes()); u64w(&mut out,generation);
    u64w(&mut out,p.len() as u64); let checksum=hash(&out,&p); u64w(&mut out,checksum); out.extend(p);
    if out.len()>MAX_FILE_BYTES { return Err("Project too large".into()); }
    Ok(out)
}

struct Reader<'a> { bytes: &'a [u8], pos: usize }
impl<'a> Reader<'a> {
    fn take(&mut self,n:usize)->Result<&'a [u8]> {
        let end=self.pos.checked_add(n).ok_or("Length overflow")?;
        let b=self.bytes.get(self.pos..end).ok_or("Truncated project")?; self.pos=end; Ok(b)
    }
    fn u32(&mut self)->Result<u32> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn u64(&mut self)->Result<u64> { Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap())) }
    fn float(&mut self)->Result<f32> {
        let v=f32::from_le_bytes(self.take(4)?.try_into().unwrap());
        if !v.is_finite() { return Err("Non-finite number".into()); } Ok(v)
    }
    fn vec(&mut self)->Result<Vec3> { Ok(Vec3::new(self.float()?,self.float()?,self.float()?)) }
    fn count(&mut self,max:usize,min_bytes:usize)->Result<usize> {
        let n=self.u32()? as usize;
        if n>max || n>self.bytes.len().saturating_sub(self.pos)/min_bytes { return Err("Invalid count".into()); } Ok(n)
    }
    fn boolean(&mut self)->Result<bool> { match self.take(1)?[0] {0=>Ok(false),1=>Ok(true),_=>Err("Invalid boolean".into())} }
}

pub fn decode(bytes: &[u8])->Result<(u64,WorkspaceState)> {
    if bytes.len()>MAX_FILE_BYTES { return Err("Project too large".into()); }
    let mut r=Reader{bytes,pos:0};
    if r.take(8)?!=MAGIC { return Err("Not a RUSTBLEND project".into()); }
    if r.u32()?!=VERSION { return Err("Unsupported project version".into()); }
    let generation=r.u64()?; let length=r.u64()?; let checksum=r.u64()?;
    if length!=(bytes.len()-HEADER) as u64 || hash(&bytes[..28],&bytes[HEADER..])!=checksum { return Err("Project checksum or length mismatch".into()); }
    let next=r.u64()?;
    let count=r.count(MAX_OBJECTS,64)?; let mut objects=Vec::with_capacity(count);
    for _ in 0..count {
        let id=r.u64()?; let n=r.count(MAX_NAME,1)?;
        let name=std::str::from_utf8(r.take(n)?).map_err(|_|"Invalid UTF-8 name")?.to_owned();
        let position=r.vec()?;
        let rotation=Quaternion::new(r.float()?,r.float()?,r.float()?,r.float()?);
        if (rotation.length_squared()-1.0).abs()>0.001 { return Err("Invalid rotation".into()); }
        let transform=Transform::new(position,rotation,r.vec()?).map_err(|_|"Invalid transform")?;
        let n=r.count(MAX_FILE_BYTES/12,12)?; let mut vertices=Vec::with_capacity(n);
        for _ in 0..n { vertices.push(r.vec()?); }
        let n=r.count(MAX_FILE_BYTES/8,8)?; let mut edges=Vec::with_capacity(n);
        for _ in 0..n { edges.push(Edge::new(r.u32()? as usize,r.u32()? as usize)); }
        let n=r.count(MAX_FILE_BYTES/4,4)?; let mut faces=Vec::with_capacity(n);
        for _ in 0..n { let n=r.count(MAX_CORNERS,4)?; let mut corners=Vec::with_capacity(n); for _ in 0..n { corners.push(r.u32()? as usize); } faces.push(Face::new(corners)); }
        let mesh=Mesh::new(vertices,edges,faces).map_err(|e|format!("Invalid mesh: {e:?}"))?;
        objects.push((id,name,mesh,transform));
    }
    let n=r.count(MAX_OBJECTS,8)?; let mut selected=Vec::with_capacity(n);
    for _ in 0..n { selected.push(r.u64()?); }
    let active=r.u64()?;
    let scene=Scene::restore(objects,next,selected,(active!=0).then_some(active)).map_err(|_|"Invalid scene references")?;
    let target=r.vec()?; let distance=r.float()?; let aspect=r.float()?;
    if !(0.05..=1_000_000.0).contains(&distance) { return Err("Invalid camera distance".into()); }
    let mut camera=Camera::new(target,distance,aspect).map_err(|_|"Invalid camera")?;
    let yaw=r.float()?; let pitch=r.float()?;
    if !(0.0..core::f32::consts::TAU).contains(&yaw) || pitch.abs()>core::f32::consts::PI*0.5-0.001 { return Err("Invalid camera angles".into()); }
    camera.orbit(yaw,pitch).map_err(|_|"Invalid camera orbit")?;
    let tool=match r.take(1)?[0] {0=>ToolMode::Orbit,1=>ToolMode::Move,2=>ToolMode::Rotate,3=>ToolMode::Scale,_=>return Err("Invalid tool".into())};
    let edit_mode=r.boolean()?; let outliner_open=r.boolean()?;
    if r.pos!=bytes.len() { return Err("Unexpected project data".into()); }
    Ok((generation,WorkspaceState{scene,camera,tool,edit_mode,outliner_open,history:Default::default()}))
}

fn read_file(path:&Path)->Result<Option<(u64,WorkspaceState)>> {
    let file=match File::open(path) { Ok(f)=>f,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(None),Err(e)=>return Err(e.to_string()) };
    let mut bytes=Vec::new(); file.take(MAX_FILE_BYTES as u64+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    decode(&bytes).map(Some)
}

/// One writer per application. Generation is part of the validated file header.
#[derive(Debug)]
pub struct RecoveryStore { directory:PathBuf, generation:u64 }
impl RecoveryStore {
    pub fn open(directory:PathBuf)->Result<(Self,Option<WorkspaceState>)> {
        fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
        let mut best=None; let mut errors=Vec::new();
        for slot in 0..2 {
            match read_file(&directory.join(format!("recovery-{slot}.rblend"))) {
                Ok(Some((g,s))) if best.as_ref().is_none_or(|(old,_)|g>*old)=>best=Some((g,s)),
                Ok(_)=>{}, Err(e)=>errors.push(e),
            }
        }
        // Never downgrade or overwrite a project created by a newer app.
        if errors.iter().any(|e|e=="Unsupported project version") { return Err("Unsupported project version; recovery left untouched".into()); }
        if best.is_none() && !errors.is_empty() { return Err(format!("No valid recovery: {}",errors.join("; "))); }
        if !errors.is_empty() { eprintln!("RUSTBLEND: recovered from alternate slot: {}",errors.join("; ")); }
        let (generation,state)=best.map_or((0,None),|(g,s)|(g,Some(s)));
        Ok((Self{directory,generation},state))
    }
    pub fn save(&mut self,state:&WorkspaceState)->Result<()> {
        let generation=self.generation.checked_add(1).ok_or("Recovery generation exhausted")?;
        let bytes=encode(state,generation)?;
        let temp=self.directory.join("recovery.tmp");
        let target=self.directory.join(format!("recovery-{}.rblend",generation%2));
        let mut file=File::create(&temp).map_err(|e|e.to_string())?;
        file.write_all(&bytes).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?; drop(file);
        fs::rename(&temp,&target).map_err(|e|e.to_string())?;
        File::open(&self.directory).and_then(|f|f.sync_all()).map_err(|e|e.to_string())?;
        self.generation=generation;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64,Ordering};
    static SERIAL:AtomicU64=AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new()->Self {
            let p=std::env::temp_dir().join(format!("rustblend-project-{}-{}",std::process::id(),SERIAL.fetch_add(1,Ordering::Relaxed)));
            fs::create_dir(&p).unwrap(); Self(p)
        }
    }
    impl Drop for Temp { fn drop(&mut self) { let _=fs::remove_dir_all(&self.0); } }
    fn sample()->WorkspaceState {
        let mut scene=Scene::new();
        let deleted=scene.add_object("Deleted",Mesh::cube(1.0).unwrap()).unwrap();
        let a=scene.add_object("Çalışma 🧊",Mesh::cube(2.0).unwrap()).unwrap();
        let b=scene.add_object("Second",Mesh::cube(3.0).unwrap()).unwrap();
        scene.remove(deleted); scene.select(b).unwrap(); scene.add_to_selection(a).unwrap();
        scene.translate_active(Vec3::new(3.0,2.0,-7.0)).unwrap();
        let mut camera=Camera::new(Vec3::new(1.0,2.0,3.0),7.0,1.5).unwrap();
        camera.orbit(0.7,-0.4).unwrap();
        WorkspaceState{scene,camera,tool:ToolMode::Rotate,edit_mode:true,outliner_open:false,history:Default::default()}
    }
    fn rehash(bytes:&mut [u8]) {
        let checksum=hash(&bytes[..28],&bytes[HEADER..]); bytes[28..36].copy_from_slice(&checksum.to_le_bytes());
    }
    #[test]
    fn round_trip_preserves_ids_names_mesh_transforms_selection_and_camera() {
        let s=sample(); let bytes=encode(&s,17).unwrap(); let (g,mut loaded)=decode(&bytes).unwrap();
        assert_eq!(g,17); assert_eq!(loaded.scene,s.scene); assert_eq!(loaded.camera,s.camera);
        assert_eq!(loaded.tool,s.tool); assert!(loaded.edit_mode); assert!(!loaded.outliner_open);
        assert_eq!(loaded.scene.add_object("Next",Mesh::cube(1.0).unwrap()).unwrap().get(),4);
    }
    #[test]
    fn empty_scene_round_trips_without_default_cube() {
        let mut s=sample(); s.scene=Scene::new();
        assert!(decode(&encode(&s,1).unwrap()).unwrap().1.scene.is_empty());
    }
    #[test]
    fn every_truncated_prefix_is_rejected_without_panic() {
        let bytes=encode(&sample(),1).unwrap();
        for n in 0..bytes.len() { assert!(decode(&bytes[..n]).is_err(),"prefix {n}"); }
    }
    #[test]
    fn corruption_of_generation_or_payload_is_rejected() {
        let bytes=encode(&sample(),1).unwrap();
        for index in [12,20,28,HEADER,bytes.len()-1] {
            let mut corrupt=bytes.clone(); corrupt[index]^=1; assert!(decode(&corrupt).is_err());
        }
    }
    #[test]
    fn wrong_version_trailing_data_and_oversized_input_are_rejected() {
        let mut bytes=encode(&sample(),1).unwrap(); bytes[8]=2;
        assert_eq!(decode(&bytes).unwrap_err(),"Unsupported project version");
        let mut bytes=encode(&sample(),1).unwrap(); bytes.push(0); assert!(decode(&bytes).is_err());
        assert!(decode(&vec![0;MAX_FILE_BYTES+1]).is_err());
    }
    #[test]
    fn valid_checksum_cannot_hide_invalid_counts_ids_or_flags() {
        let bytes=encode(&sample(),1).unwrap();
        let mut bad=bytes.clone(); bad[HEADER+8..HEADER+12].copy_from_slice(&u32::MAX.to_le_bytes()); rehash(&mut bad); assert!(decode(&bad).is_err());
        let mut bad=bytes.clone(); bad[HEADER..HEADER+8].copy_from_slice(&1u64.to_le_bytes()); rehash(&mut bad); assert!(decode(&bad).is_err());
        let mut bad=bytes.clone(); let n=bad.len(); bad[n-1]=2; rehash(&mut bad); assert!(decode(&bad).is_err());
    }
    #[test]
    fn nonfinite_transform_and_invalid_mesh_index_are_rejected() {
        let s=sample(); let bytes=encode(&s,1).unwrap();
        let transform=HEADER+12+8+4+s.scene.objects()[0].name().len();
        let mut bad=bytes.clone(); bad[transform..transform+4].copy_from_slice(&f32::NAN.to_le_bytes()); rehash(&mut bad); assert!(decode(&bad).is_err());
        let edge=transform+40+4+8*12+4;
        let mut bad=bytes.clone(); bad[edge..edge+4].copy_from_slice(&u32::MAX.to_le_bytes()); rehash(&mut bad); assert!(decode(&bad).is_err());
    }
    #[test]
    fn scene_restore_rejects_duplicate_and_dangling_references() {
        let record=||(1,"Cube".to_owned(),Mesh::cube(1.0).unwrap(),Transform::default());
        assert!(Scene::restore(vec![record(),record()],2,vec![],None).is_err());
        assert!(Scene::restore(vec![record()],2,vec![3],Some(3)).is_err());
        assert!(Scene::restore(vec![record()],2,vec![1,1],Some(1)).is_err());
        assert!(Scene::restore(vec![record()],2,vec![],Some(1)).is_err());
    }
    #[test]
    fn bounded_random_inputs_do_not_panic() {
        let mut seed=42u64;
        for len in 0..512 {
            let bytes:Vec<u8>=(0..len).map(|_|{seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);(seed>>32)as u8}).collect();
            assert!(decode(&bytes).is_err());
        }
    }
    #[test]
    fn newest_saved_generation_is_reopened() {
        let dir=Temp::new(); let (mut store,state)=RecoveryStore::open(dir.0.clone()).unwrap(); assert!(state.is_none());
        let mut s=sample(); store.save(&s).unwrap();
        s.scene.translate_active(Vec3::new(2.0,0.0,0.0)).unwrap(); store.save(&s).unwrap();
        let (store,state)=RecoveryStore::open(dir.0.clone()).unwrap(); assert_eq!(store.generation,2); assert_eq!(state.unwrap().scene,s.scene);
    }
    #[test]
    fn corrupt_latest_slot_falls_back_and_interrupted_temp_is_ignored() {
        let dir=Temp::new(); let (mut store,_)=RecoveryStore::open(dir.0.clone()).unwrap();
        let s=sample(); store.save(&s).unwrap(); store.save(&s).unwrap();
        fs::write(dir.0.join("recovery-0.rblend"),b"broken").unwrap();
        fs::write(dir.0.join("recovery.tmp"),b"unfinished").unwrap();
        let (mut recovered,state)=RecoveryStore::open(dir.0.clone()).unwrap(); assert_eq!(recovered.generation,1); assert_eq!(state.unwrap().scene,s.scene);
        recovered.save(&s).unwrap(); assert_eq!(RecoveryStore::open(dir.0.clone()).unwrap().0.generation,2);
    }
    #[test]
    fn failed_write_keeps_previous_project() {
        let dir=Temp::new(); let (mut store,_)=RecoveryStore::open(dir.0.clone()).unwrap();
        let s=sample(); store.save(&s).unwrap();
        fs::create_dir(dir.0.join("recovery.tmp")).unwrap();
        assert!(store.save(&s).is_err()); assert_eq!(store.generation,1);
        assert_eq!(RecoveryStore::open(dir.0.clone()).unwrap().1.unwrap().scene,s.scene);
    }
    #[test]
    fn both_bad_or_future_version_blocks_recovery_without_overwrite() {
        let dir=Temp::new(); let bad=dir.0.join("recovery-0.rblend");
        fs::write(&bad,b"broken").unwrap(); assert!(RecoveryStore::open(dir.0.clone()).is_err()); assert_eq!(fs::read(&bad).unwrap(),b"broken");
        let good=encode(&sample(),1).unwrap(); fs::write(dir.0.join("recovery-1.rblend"),&good).unwrap();
        let mut future=good; future[8]=2; fs::write(&bad,&future).unwrap();
        assert!(RecoveryStore::open(dir.0.clone()).is_err()); assert_eq!(fs::read(&bad).unwrap(),future);
    }
    #[test]
    fn oversized_name_is_rejected_before_save() {
        let mut s=sample(); s.scene.add_object("x".repeat(MAX_NAME+1),Mesh::cube(1.0).unwrap()).unwrap();
        assert!(encode(&s,1).is_err());
    }
}
