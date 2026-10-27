//! Connection accounting is independent of history deletion and diagnostic logging.
//! No keys, prompts, transcript, endpoint or response body enter this database.
use notype_core::{observations::{ApiCallDescriptor, ApiCallObserver, ApiOutcome, ApiReceipt, ApiSummary, summarize_api_calls}, TextFailure};
use rusqlite::{params, Connection, OptionalExtension};
use std::{collections::HashSet, path::Path, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};

pub struct ActivityStore { database: Mutex<Connection>, tests: Mutex<HashSet<String>> }
impl ActivityStore {
    pub fn open(path: &Path) -> Result<Arc<Self>, String> {
        Self::from_connection(Connection::open(path).map_err(|_| "activity-storage-failed")?)
    }
    pub fn memory() -> Result<Arc<Self>, String> {
        Self::from_connection(Connection::open_in_memory().map_err(|_| "activity-storage-failed")?)
    }
    fn from_connection(connection: Connection) -> Result<Arc<Self>, String> {
        connection.execute_batch("CREATE TABLE IF NOT EXISTS connection_calls(id TEXT PRIMARY KEY, connection_id TEXT NOT NULL, descriptor TEXT NOT NULL, receipt TEXT, sent INTEGER NOT NULL DEFAULT 0); CREATE INDEX IF NOT EXISTS connection_calls_owner ON connection_calls(connection_id); CREATE TABLE IF NOT EXISTS connection_tests(connection_id TEXT PRIMARY KEY, revision INTEGER NOT NULL, success INTEGER NOT NULL, error TEXT, tested_at INTEGER NOT NULL);").map_err(|_| "activity-storage-failed")?;
        Ok(Arc::new(Self { database: Mutex::new(connection), tests: Mutex::new(HashSet::new()) }))
    }
    pub fn summary(&self, id: &str) -> Result<ApiSummary, String> {
        let connection=self.database.lock().map_err(|_| "activity-storage-failed")?;
        let mut statement=connection.prepare("SELECT descriptor,receipt FROM connection_calls WHERE connection_id=?1 AND sent=1 ORDER BY rowid").map_err(|_| "activity-storage-failed")?;
        let rows=statement.query_map([id],|row| Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?))).map_err(|_| "activity-storage-failed")?;
        let mut calls=Vec::new();
        for row in rows {
            let (descriptor,receipt)=row.map_err(|_| "activity-storage-failed")?;
            calls.push((serde_json::from_str(&descriptor).map_err(|_| "activity-storage-failed")?,receipt.map(|r| serde_json::from_str(&r)).transpose().map_err(|_| "activity-storage-failed")?.unwrap_or_default()));
        }
        Ok(summarize_api_calls(calls))
    }
    /// Reset only this connection's counters. Late receipts use UPDATE and cannot resurrect deleted rows.
    pub fn clear(&self, id: &str) -> Result<(), String> {
        self.database.lock().map_err(|_| "activity-storage-failed")?.execute("DELETE FROM connection_calls WHERE connection_id=?1",[id]).map_err(|_| "activity-storage-failed")?;
        Ok(())
    }
    pub fn observer(self: &Arc<Self>, id: String, parent: Arc<dyn ApiCallObserver>) -> Arc<dyn ApiCallObserver> {
        Arc::new(MeteredObserver { store:self.clone(), connection_id:id, parent })
    }
    pub fn admit_test(self: &Arc<Self>, id: &str) -> Result<TestLease, String> {
        if !self.tests.lock().map_err(|_| "activity-storage-failed")?.insert(id.into()) { return Err("connection-test-busy".into()); }
        Ok(TestLease { store:self.clone(), id:id.into() })
    }
    pub fn test_result(&self, id: &str, revision: u64, success: bool, error: Option<&str>) -> Result<(), String> {
        let at=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(i64::MAX as u128) as i64;
        self.database.lock().map_err(|_| "activity-storage-failed")?.execute("INSERT INTO connection_tests(connection_id,revision,success,error,tested_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(connection_id) DO UPDATE SET revision=excluded.revision,success=excluded.success,error=excluded.error,tested_at=excluded.tested_at",params![id,revision,success,error,at]).map_err(|_| "activity-storage-failed")?;
        Ok(())
    }
    pub fn last_test(&self, id: &str, revision: u64) -> Result<Option<(bool,Option<String>,u64)>,String> {
        self.database.lock().map_err(|_| "activity-storage-failed")?.query_row("SELECT success,error,tested_at FROM connection_tests WHERE connection_id=?1 AND revision=?2",params![id,revision],|row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional().map_err(|_| "activity-storage-failed".into())
    }
}
pub struct TestLease { store:Arc<ActivityStore>, id:String }
impl Drop for TestLease { fn drop(&mut self) { if let Ok(mut tests)=self.store.tests.lock(){ tests.remove(&self.id); } } }
struct MeteredObserver { store:Arc<ActivityStore>, connection_id:String, parent:Arc<dyn ApiCallObserver> }
impl ApiCallObserver for MeteredObserver {
    fn processing_revision(&self)->Option<u64>{ self.parent.processing_revision() }
    fn begin(&self, descriptor:ApiCallDescriptor)->Result<String,TextFailure>{
        let json=serde_json::to_string(&descriptor).map_err(|_|TextFailure::HistoryStorage)?;
        let id=self.parent.begin(descriptor)?;
        self.store.database.lock().map_err(|_|TextFailure::HistoryStorage)?.execute("INSERT INTO connection_calls(id,connection_id,descriptor) VALUES(?1,?2,?3)",params![id,self.connection_id,json]).map_err(|_|TextFailure::HistoryStorage)?;
        Ok(id)
    }
    fn mark_sent(&self,id:&str)->Result<(),TextFailure>{
        self.parent.mark_sent(id)?;
        self.store.database.lock().map_err(|_|TextFailure::HistoryStorage)?.execute("UPDATE connection_calls SET sent=1 WHERE id=?1",[id]).map_err(|_|TextFailure::HistoryStorage)?;
        Ok(())
    }
    fn response(&self,id:&str,receipt:ApiReceipt){
        let receipt=receipt.sanitized(); self.parent.response(id,receipt.clone());
        let saved=(||{let json=serde_json::to_string(&receipt).map_err(|_|())?; self.store.database.lock().map_err(|_|())?.execute("UPDATE connection_calls SET receipt=?2 WHERE id=?1",params![id,json]).map_err(|_|())?;Ok::<_,()>(())})();
        if saved.is_err(){ self.parent.unavailable(); }
    }
    fn finish(&self,id:&str,outcome:ApiOutcome,elapsed_ms:u64){self.parent.finish(id,outcome,elapsed_ms);}
    fn unavailable(&self){self.parent.unavailable();}
}
/// Explicit connection probes have no speech-history session, but the same metering wrapper.
pub struct ProbeObserver;
impl ApiCallObserver for ProbeObserver {
    fn begin(&self,_:ApiCallDescriptor)->Result<String,TextFailure>{
        static NEXT:std::sync::atomic::AtomicU64=std::sync::atomic::AtomicU64::new(0);
        Ok(format!("probe-{}-{}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos(),NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)))
    }
    fn mark_sent(&self,_:&str)->Result<(),TextFailure>{Ok(())}
    fn response(&self,_:&str,_:ApiReceipt){}
    fn finish(&self,_:&str,_:ApiOutcome,_:u64){}
    fn unavailable(&self){}
}
