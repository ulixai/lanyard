use lanyard_core::{legacy::LegacyReader, Error, MemoryStore, Result, Vault};
use std::{collections::HashMap, fs, path::Path};
struct Reader { values: HashMap<String,String>, access_failure: Option<String> }
impl LegacyReader for Reader {
    fn read_legacy(&self,id:&str)->Result<String> {
        if self.access_failure.as_deref()==Some(id) {return Err(Error::Storage("Access denied".into()));}
        self.values.get(id).cloned().ok_or(Error::NotFound)
    }
}
fn fixture(dir:&Path, ids:&[String]) {
    fs::create_dir_all(dir).unwrap();fs::write(dir.join("config.json"),"{}").unwrap();
    let items:Vec<_>=ids.iter().map(|id|serde_json::json!({"id":id,"title":format!("Title {id}"),"category":"api_key","project_id":null})).collect();
    fs::write(dir.join("meta.json"),serde_json::json!({"items":items,"projects":[]}).to_string()).unwrap();
}
fn payload()->String {r#"{"token":"example secret"}"#.into()}
#[test]
fn strict_import_stops_but_opt_in_reports_missing_and_retries_them_later() {
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("vault.json");
    let mut vault=Vault::open(&path,Box::<MemoryStore>::default()).unwrap();vault.initialize("current pin").unwrap();
    let a=uuid::Uuid::new_v4().to_string();let b=uuid::Uuid::new_v4().to_string();let old=dir.path().join("old");fixture(&old,&[a.clone(),b.clone()]);
    let original=fs::read(old.join("meta.json")).unwrap();let before=fs::read(&path).unwrap();
    let mut reader=Reader{values:HashMap::from([(a.clone(),payload())]),access_failure:None};
    assert!(vault.import_legacy_report(&old,"","",&reader).is_err());assert_eq!(fs::read(&path).unwrap(),before);
    let report=vault.import_legacy_with_options(&old,"","",&reader,true).unwrap();
    assert_eq!((report.imported,report.skipped,report.missing.len()),(1,0,1));assert_eq!(report.missing[0].id,b);assert_eq!(report.missing[0].title,format!("Title {b}"));
    assert_eq!(vault.reveal(&a).unwrap()["token"],"example secret");assert!(vault.reveal(&b).is_err());
    reader.values.insert(b.clone(),payload());
    let report=vault.import_legacy_with_options(&old,"","",&reader,true).unwrap();
    assert_eq!((report.imported,report.skipped,report.missing.len()),(1,1,0));assert!(vault.reveal(&b).is_ok());
    assert_eq!(fs::read(old.join("meta.json")).unwrap(),original);vault.lock();vault.unlock("current pin").unwrap();
}
#[test]
fn opt_in_does_not_hide_access_errors_or_corrupt_payloads() {
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("vault.json");let mut vault=Vault::open(&path,Box::<MemoryStore>::default()).unwrap();vault.initialize("current pin").unwrap();
    let a=uuid::Uuid::new_v4().to_string();let b=uuid::Uuid::new_v4().to_string();let old=dir.path().join("old");fixture(&old,&[a.clone(),b.clone()]);let before=fs::read(&path).unwrap();
    let reader=Reader{values:HashMap::from([(a.clone(),payload())]),access_failure:Some(b.clone())};
    let error=vault.import_legacy_with_options(&old,"","",&reader,true).unwrap_err();assert!(error.to_string().contains("Access denied"));assert_eq!(fs::read(&path).unwrap(),before);
    let reader=Reader{values:HashMap::from([(a.clone(),payload()),(b,"corrupt".into())]),access_failure:None};
    assert!(vault.import_legacy_with_options(&old,"","",&reader,true).is_err());assert_eq!(fs::read(&path).unwrap(),before);assert!(vault.reveal(&a).is_err());assert!(!vault.locked());
}
#[test]
fn all_missing_reports_each_item_without_changing_current_vault() {
    let dir=tempfile::tempdir().unwrap();let path=dir.path().join("vault.json");let mut vault=Vault::open(&path,Box::<MemoryStore>::default()).unwrap();vault.initialize("current pin").unwrap();
    let ids:Vec<_>=(0..2).map(|_|uuid::Uuid::new_v4().to_string()).collect();let old=dir.path().join("old");fixture(&old,&ids);let before=fs::read(&path).unwrap();
    let reader=Reader{values:HashMap::new(),access_failure:None};let report=vault.import_legacy_with_options(&old,"","",&reader,true).unwrap();
    assert_eq!((report.imported,report.skipped,report.missing.len()),(0,0,2));assert_eq!(fs::read(&path).unwrap(),before);assert!(!vault.locked());
}
