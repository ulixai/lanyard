use lanyard_core::{legacy::LegacyReader, Category, Error, Fields, ItemInput, Result, SecretStore, Vault};
use std::{collections::HashMap, fs, path::Path, sync::{Arc, Mutex}};
#[derive(Default)]
struct StoreState { values: HashMap<String, String>, remaining_writes: Option<usize> }
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<StoreState>>);
impl SecretStore for Shared {
    fn read(&self, id: &str) -> Result<String> { self.0.lock().unwrap().values.get(id).cloned().ok_or(Error::NotFound) }
    fn write(&mut self, id: &str, value: &str) -> Result<()> {
        let mut s = self.0.lock().unwrap();
        if let Some(n) = s.remaining_writes.as_mut() { if *n == 0 { return Err(Error::Storage("Simulated write failure".into())); } *n -= 1; }
        s.values.insert(id.into(),value.into()); Ok(())
    }
    fn delete(&mut self, id: &str) -> Result<()> { self.0.lock().unwrap().values.remove(id); Ok(()) }
}
struct Reader(HashMap<String,String>);
impl LegacyReader for Reader {
    fn read_legacy(&self,id:&str)->Result<String> { self.0.get(id).cloned().ok_or_else(||Error::Storage(format!("Missing legacy credential {id}"))) }
}
fn fixture(path:&Path,ids:&[String],project:Option<&str>) {
    fs::create_dir_all(path).unwrap(); fs::write(path.join("config.json"),r#"{"close_to_tray":true}"#).unwrap();
    let projects:Vec<_>=project.into_iter().map(|id|serde_json::json!({"id":id,"title":"Legacy project","description":"Original"})).collect();
    let items:Vec<_>=ids.iter().map(|id|serde_json::json!({"id":id,"title":"Legacy","category":"api_key","project_id":project,"allowed_apps":["Old app"]})).collect();
    fs::write(path.join("meta.json"),serde_json::json!({"items":items,"projects":projects}).to_string()).unwrap();
}
fn existing(v:&mut Vault)->String {
    v.initialize("current pin").unwrap(); v.set_close_to_tray(false).unwrap();
    let project=v.save_project(None,"Current project","Keep me").unwrap();
    v.save_item(ItemInput{id:None,title:"Current credential".into(),category:Category::ApiKey,project_id:Some(project),fields:Fields::from([("token".into(),"current secret".into())])}).unwrap()
}
fn payload()->String { r#"{"token":"legacy secret"}"#.into() }
#[test]
fn retry_preserves_pin_secrets_projects_grants_and_is_idempotent() {
    let temp=tempfile::tempdir().unwrap(); let path=temp.path().join("vault.json"); let store=Shared::default();
    let mut v=Vault::open(&path,Box::new(store.clone())).unwrap(); let current=existing(&mut v);
    use base64::Engine;
    let auth=lanyard_core::ClientAuth{client_id:uuid::Uuid::new_v4().to_string(),app_name:"Current app".into(),token:base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([42;32])};
    v.approve(&auth,&current,true).unwrap(); let before=v.snapshot();
    let id=uuid::Uuid::new_v4().to_string(); let project=uuid::Uuid::new_v4().to_string(); let old=temp.path().join("old");
    fixture(&old,&[current.clone(),id.clone()],Some(&project));let original=fs::read(old.join("meta.json")).unwrap();
    let r=v.import_legacy_report(&old,"","ignored new pin",&Reader(HashMap::from([(id.clone(),payload())]))).unwrap();
    assert_eq!((r.imported,r.skipped,r.projects_added),(1,1,1));
    assert_eq!(v.reveal(&current).unwrap()["token"],"current secret");assert_eq!(v.reveal(&id).unwrap()["token"],"legacy secret");
    let after=v.snapshot();assert!(!after.close_to_tray);assert_eq!(after.projects[0].title,"Current project");
    assert_eq!(serde_json::to_value(&before.clients).unwrap(),serde_json::to_value(&after.clients).unwrap());
    assert_eq!(serde_json::to_value(&before.grants).unwrap(),serde_json::to_value(&after.grants).unwrap());
    let committed=fs::read(&path).unwrap();
    let r=v.import_legacy_report(&old,"","",&Reader(HashMap::new())).unwrap();assert_eq!((r.imported,r.skipped,r.projects_added),(0,2,0));
    assert_eq!(fs::read(&path).unwrap(),committed);assert_eq!(fs::read(old.join("meta.json")).unwrap(),original);
    drop(v);let mut reopened=Vault::open(&path,Box::new(store)).unwrap();reopened.unlock("current pin").unwrap();assert!(reopened.reveal(&id).is_ok());
}
#[test]
fn missing_secret_does_not_partially_import_and_retry_succeeds() {
    let temp=tempfile::tempdir().unwrap();let path=temp.path().join("vault.json");let store=Shared::default();let mut v=Vault::open(&path,Box::new(store.clone())).unwrap();let current=existing(&mut v);
    let a=uuid::Uuid::new_v4().to_string();let b=uuid::Uuid::new_v4().to_string();let old=temp.path().join("old");fixture(&old,&[a.clone(),b.clone()],None);
    let bytes=fs::read(&path).unwrap();let secrets=store.0.lock().unwrap().values.clone();
    let err=v.import_legacy_report(&old,"","",&Reader(HashMap::from([(a.clone(),payload())]))).unwrap_err();assert!(err.to_string().contains(&b));
    assert_eq!(fs::read(&path).unwrap(),bytes);assert_eq!(store.0.lock().unwrap().values,secrets);assert!(v.reveal(&current).is_ok());
    let r=v.import_legacy_report(&old,"","",&Reader(HashMap::from([(a,payload()),(b,payload())]))).unwrap();assert_eq!(r.imported,2);
}
#[test]
fn write_failure_cleans_only_new_secrets_and_keeps_vault_unlocked() {
    let temp=tempfile::tempdir().unwrap();let path=temp.path().join("vault.json");let store=Shared::default();let mut v=Vault::open(&path,Box::new(store.clone())).unwrap();let current=existing(&mut v);
    let ids:Vec<_>=(0..2).map(|_|uuid::Uuid::new_v4().to_string()).collect();let old=temp.path().join("old");fixture(&old,&ids,None);
    let bytes=fs::read(&path).unwrap();let secrets=store.0.lock().unwrap().values.clone();store.0.lock().unwrap().remaining_writes=Some(1);
    assert!(v.import_legacy_report(&old,"","",&Reader(ids.into_iter().map(|id|(id,payload())).collect())).is_err());
    assert_eq!(fs::read(&path).unwrap(),bytes);assert_eq!(store.0.lock().unwrap().values,secrets);assert_eq!(v.reveal(&current).unwrap()["token"],"current secret");
}
#[test]
fn metadata_commit_failure_cleans_staged_secrets_and_keeps_existing_key() {
    let temp=tempfile::tempdir().unwrap();let path=temp.path().join("vault.json");let store=Shared::default();let mut v=Vault::open(&path,Box::new(store.clone())).unwrap();let current=existing(&mut v);
    let id=uuid::Uuid::new_v4().to_string();let old=temp.path().join("old");fixture(&old,&[id.clone()],None);let secrets=store.0.lock().unwrap().values.clone();
    let saved=temp.path().join("saved.json");fs::rename(&path,&saved).unwrap();fs::create_dir(&path).unwrap();
    assert!(v.import_legacy_report(&old,"","",&Reader(HashMap::from([(id,payload())]))).is_err());
    assert_eq!(store.0.lock().unwrap().values,secrets);assert_eq!(v.reveal(&current).unwrap()["token"],"current secret");
    fs::remove_dir(&path).unwrap();fs::rename(saved,path).unwrap();
}
#[test]
fn locked_vault_rejects_before_reading_legacy_files() {
    let temp=tempfile::tempdir().unwrap();let mut v=Vault::open(temp.path().join("vault.json"),Box::new(Shared::default())).unwrap();existing(&mut v);v.lock();
    assert!(matches!(v.import_legacy_report(&temp.path().join("absent"),"","",&Reader(HashMap::new())),Err(Error::Locked)));
}
