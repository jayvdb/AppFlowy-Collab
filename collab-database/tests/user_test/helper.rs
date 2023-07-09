use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use collab::core::collab::{CollabRawData, MutexCollab};
use collab::preclude::CollabBuilder;
use collab_database::database::{gen_database_id, gen_field_id, gen_row_id};
use collab_database::error::DatabaseError;
use collab_database::fields::Field;
use collab_database::rows::CellsBuilder;
use collab_database::rows::CreateRowParams;
use collab_database::user::{
  make_workspace_database_id, CollabFuture, DatabaseCollabService, RowRelationChange,
  RowRelationUpdateReceiver, WorkspaceDatabase,
};
use collab_database::views::{CreateDatabaseParams, DatabaseLayout};
use collab_persistence::kv::rocks_kv::RocksCollabDB;
use collab_plugins::disk::rocksdb::{CollabPersistenceConfig, RocksdbDiskPlugin};
use parking_lot::Mutex;
use tokio::sync::mpsc::{channel, Receiver};

use rand::Rng;
use tempfile::TempDir;

use crate::helper::{make_rocks_db, TestTextCell};

pub struct WorkspaceDatabaseTest {
  #[allow(dead_code)]
  uid: i64,
  inner: WorkspaceDatabase,
  pub db: Arc<RocksCollabDB>,
}

impl Deref for WorkspaceDatabaseTest {
  type Target = WorkspaceDatabase;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

pub fn random_uid() -> i64 {
  let mut rng = rand::thread_rng();
  rng.gen::<i64>()
}

pub struct TestUserDatabaseCollabBuilderImpl();

impl DatabaseCollabService for TestUserDatabaseCollabBuilderImpl {
  fn get_collab_updates(
    &self,
    _object_id: &str,
  ) -> CollabFuture<Result<Vec<Vec<u8>>, DatabaseError>> {
    Box::pin(async move { Ok(vec![]) })
  }

  fn build_collab_with_config(
    &self,
    uid: i64,
    object_id: &str,
    _object_name: &str,
    collab_db: Arc<RocksCollabDB>,
    collab_raw_data: CollabRawData,
    config: &CollabPersistenceConfig,
  ) -> Arc<MutexCollab> {
    let collab = CollabBuilder::new(uid, object_id)
      .with_raw_data(collab_raw_data)
      .with_plugin(RocksdbDiskPlugin::new_with_config(
        uid,
        collab_db,
        config.clone(),
      ))
      .build()
      .unwrap();
    collab.lock().initialize();
    Arc::new(collab)
  }
}

pub fn workspace_database_test(uid: i64) -> WorkspaceDatabaseTest {
  let db = make_rocks_db();
  user_database_test_with_db(uid, db)
}

pub fn workspace_database_test_with_config(
  uid: i64,
  config: CollabPersistenceConfig,
) -> WorkspaceDatabaseTest {
  let collab_db = make_rocks_db();
  let builder = TestUserDatabaseCollabBuilderImpl();
  let collab = builder.build_collab_with_config(
    uid,
    &make_workspace_database_id(uid),
    "databases",
    collab_db.clone(),
    CollabRawData::default(),
    &config,
  );
  let inner = WorkspaceDatabase::open(uid, collab, collab_db.clone(), config, builder);
  WorkspaceDatabaseTest {
    uid,
    inner,
    db: collab_db,
  }
}

pub fn workspace_database_with_db(
  uid: i64,
  collab_db: Arc<RocksCollabDB>,
  config: Option<CollabPersistenceConfig>,
) -> WorkspaceDatabase {
  let config = config.unwrap_or_else(|| CollabPersistenceConfig::new().snapshot_per_update(5));
  let builder = TestUserDatabaseCollabBuilderImpl();
  let collab = builder.build_collab_with_config(
    uid,
    &make_workspace_database_id(uid),
    "databases",
    collab_db.clone(),
    CollabRawData::default(),
    &config,
  );
  WorkspaceDatabase::open(uid, collab, collab_db, config, builder)
}

pub fn user_database_test_with_db(uid: i64, db: Arc<RocksCollabDB>) -> WorkspaceDatabaseTest {
  let inner = workspace_database_with_db(uid, db.clone(), None);
  WorkspaceDatabaseTest { uid, inner, db }
}

pub fn user_database_test_with_default_data(uid: i64) -> WorkspaceDatabaseTest {
  let tempdir = TempDir::new().unwrap();
  let path = tempdir.into_path();
  let db = Arc::new(RocksCollabDB::open(path).unwrap());
  let w_database = user_database_test_with_db(uid, db);

  w_database
    .create_database(create_database_params("d1"))
    .unwrap();

  w_database
}

fn create_database_params(database_id: &str) -> CreateDatabaseParams {
  let row_1 = CreateRowParams {
    id: 1.into(),
    cells: CellsBuilder::new()
      .insert_cell("f1", TestTextCell::from("1f1cell"))
      .insert_cell("f2", TestTextCell::from("1f2cell"))
      .insert_cell("f3", TestTextCell::from("1f3cell"))
      .build(),
    height: 0,
    visibility: true,
    prev_row_id: None,
    timestamp: 0,
  };
  let row_2 = CreateRowParams {
    id: 2.into(),
    cells: CellsBuilder::new()
      .insert_cell("f1", TestTextCell::from("2f1cell"))
      .insert_cell("f2", TestTextCell::from("2f2cell"))
      .build(),
    height: 0,
    visibility: true,
    prev_row_id: None,
    timestamp: 0,
  };
  let row_3 = CreateRowParams {
    id: 3.into(),
    cells: CellsBuilder::new()
      .insert_cell("f1", TestTextCell::from("3f1cell"))
      .insert_cell("f3", TestTextCell::from("3f3cell"))
      .build(),
    height: 0,
    visibility: true,
    prev_row_id: None,
    timestamp: 0,
  };
  let field_1 = Field::new("f1".to_string(), "text field".to_string(), 0, true);
  let field_2 = Field::new("f2".to_string(), "single select field".to_string(), 2, true);
  let field_3 = Field::new("f3".to_string(), "checkbox field".to_string(), 1, true);

  CreateDatabaseParams {
    database_id: database_id.to_string(),
    view_id: "v1".to_string(),
    name: "my first database".to_string(),
    layout: Default::default(),
    layout_settings: Default::default(),
    filters: vec![],
    groups: vec![],
    sorts: vec![],
    created_rows: vec![row_1, row_2, row_3],
    fields: vec![field_1, field_2, field_3],
  }
}

pub fn poll_row_relation_rx(mut rx: RowRelationUpdateReceiver) -> Receiver<RowRelationChange> {
  let (tx, ret) = channel(1);
  tokio::spawn(async move {
    let cloned_tx = tx.clone();
    while let Ok(change) = rx.recv().await {
      cloned_tx.send(change).await.unwrap();
    }
  });
  ret
}

pub async fn test_timeout<F: Future>(f: F) -> F::Output {
  tokio::time::timeout(Duration::from_secs(2), f)
    .await
    .unwrap()
}

pub fn make_default_grid(view_id: &str, name: &str) -> CreateDatabaseParams {
  let text_field = Field {
    id: gen_field_id(),
    name: "Name".to_string(),
    field_type: 0,
    visibility: false,
    width: 0,
    type_options: Default::default(),
    is_primary: true,
  };

  let single_select_field = Field {
    id: gen_field_id(),
    name: "Status".to_string(),
    field_type: 3,
    visibility: false,
    width: 0,
    type_options: Default::default(),
    is_primary: false,
  };

  let checkbox_field = Field {
    id: gen_field_id(),
    name: "Done".to_string(),
    field_type: 4,
    visibility: false,
    width: 0,
    type_options: Default::default(),
    is_primary: false,
  };

  CreateDatabaseParams {
    database_id: gen_database_id(),
    view_id: view_id.to_string(),
    name: name.to_string(),
    layout: DatabaseLayout::Grid,
    layout_settings: Default::default(),
    filters: vec![],
    groups: vec![],
    sorts: vec![],
    created_rows: vec![
      CreateRowParams::new(gen_row_id()),
      CreateRowParams::new(gen_row_id()),
      CreateRowParams::new(gen_row_id()),
    ],
    fields: vec![text_field, single_select_field, checkbox_field],
  }
}

#[derive(Clone)]
pub struct MutexUserDatabase(Arc<Mutex<WorkspaceDatabase>>);

impl MutexUserDatabase {
  pub fn new(inner: WorkspaceDatabase) -> Self {
    Self(Arc::new(Mutex::new(inner)))
  }
}

impl Deref for MutexUserDatabase {
  type Target = Arc<Mutex<WorkspaceDatabase>>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

unsafe impl Sync for MutexUserDatabase {}

unsafe impl Send for MutexUserDatabase {}
