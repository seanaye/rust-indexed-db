//! Database-related code

use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{DomException, IdbTransactionMode};

pub(crate) use idb_version_change_event::IdbVersionChangeCallback;
pub use idb_version_change_event::IdbVersionChangeEvent;

use crate::dom_string_iterator::DomStringIterator;
use crate::idb_object_store::{IdbObjectStore, IdbObjectStoreParameters};
use crate::idb_transaction::IdbTransaction;
use crate::internal_utils::arrayify_slice;
use crate::request::{OpenDbRequest, VoidOpenDbRequest};

mod idb_version_change_event;

/// Wrapper for an IndexedDB database
#[derive(Debug)]
pub struct IdbDatabase {
    inner: web_sys::IdbDatabase,
    on_version_change: Option<IdbVersionChangeCallback>,
}

type OpenDbResult = Result<OpenDbRequest, DomException>;

impl IdbDatabase {
    #[inline]
    pub(crate) fn new(inner: web_sys::IdbDatabase) -> Self {
        Self {
            inner,
            on_version_change: None,
        }
    }

    /// Open the database with the given name
    pub fn open(name: &str) -> OpenDbResult {
        Ok(OpenDbRequest::new(factory().open(name)?))
    }

    /// Open the database with the given name and u32 version
    pub fn open_u32(name: &str, version: u32) -> OpenDbResult {
        Ok(OpenDbRequest::new(factory().open_with_u32(name, version)?))
    }

    /// Open the database with the given name and f64 version
    pub fn open_f64(name: &str, version: f64) -> OpenDbResult {
        Ok(OpenDbRequest::new(factory().open_with_f64(name, version)?))
    }

    #[inline]
    fn inner(&self) -> &web_sys::IdbDatabase {
        &self.inner
    }

    /// List the names of the object stores within this database
    #[inline]
    pub fn object_store_names(&self) -> impl Iterator<Item = String> + 'static {
        DomStringIterator::from(self.inner().object_store_names())
    }

    /// Get the database name
    #[inline]
    pub fn name(&self) -> String {
        self.inner().name()
    }

    /// Get the database version
    #[inline]
    pub fn version(&self) -> f64 {
        self.inner().version()
    }

    /// Close the database connection
    #[inline]
    pub fn close(&self) {
        self.inner().close();
    }

    /// Delete the object store with the given name
    #[inline]
    pub fn delete_object_store(&self, name: &str) -> Result<(), DomException> {
        Ok(self.inner.delete_object_store(name)?)
    }

    /// Close and delete the database
    pub fn delete(self) -> Result<VoidOpenDbRequest, DomException> {
        let name = self.name();
        self.close();
        Self::delete_by_name(&name)
    }

    /// Delete the database with the given name
    pub fn delete_by_name(name: &str) -> Result<VoidOpenDbRequest, DomException> {
        Ok(VoidOpenDbRequest::new(factory().delete_database(name)?))
    }

    /// Set the callback to execute when the versionchange event is fired
    pub fn set_on_version_change<F>(&mut self, callback: Option<F>)
    where
        F: Fn(&IdbVersionChangeEvent) -> Result<(), JsValue> + 'static,
    {
        self.on_version_change = match callback {
            Some(callback) => {
                let cb = IdbVersionChangeEvent::wrap_callback(callback);
                self.inner
                    .set_onversionchange(Some(cb.as_ref().unchecked_ref()));
                Some(cb)
            }
            None => {
                self.inner.set_onversionchange(None);
                None
            }
        };
    }

    /// Start a transaction on the given object store
    pub fn transaction_on_one(&self, name: &str) -> Result<IdbTransaction, DomException> {
        let inner = self.inner().transaction_with_str(name)?;
        Ok(IdbTransaction::new(inner, self))
    }

    /// Start a transaction on the given object stores
    #[inline]
    pub fn transaction_on_multi(&self, names: &[&str]) -> Result<IdbTransaction, DomException> {
        self.transaction_on_multi_with_array(&arrayify_slice(names))
    }

    /// Start a transaction on the given JS array of object store names
    pub fn transaction_on_multi_with_array<V: JsCast>(
        &self,
        names: &V,
    ) -> Result<IdbTransaction, DomException> {
        let res = self
            .inner()
            .transaction_with_str_sequence(names.unchecked_ref())?;
        Ok(IdbTransaction::new(res, self))
    }

    /// Start a transaction on the given object store with the given mode
    pub fn transaction_on_one_with_mode(
        &self,
        name: &str,
        mode: IdbTransactionMode,
    ) -> Result<IdbTransaction, DomException> {
        let res = self.inner().transaction_with_str_and_mode(name, mode)?;
        Ok(IdbTransaction::new(res, self))
    }

    /// Start a transaction on the given object stores with the given mode
    #[inline]
    pub fn transaction_on_multi_with_mode(
        &self,
        names: &[&str],
        mode: IdbTransactionMode,
    ) -> Result<IdbTransaction, DomException> {
        self.transaction_on_multi_with_mode_and_array(&arrayify_slice(names), mode)
    }

    /// Start a transaction on the given JS array of object store names with the given mode
    pub fn transaction_on_multi_with_mode_and_array<V: JsCast>(
        &self,
        names: &V,
        mode: IdbTransactionMode,
    ) -> Result<IdbTransaction, DomException> {
        let res = self
            .inner()
            .transaction_with_str_sequence_and_mode(names.unchecked_ref(), mode)?;
        Ok(IdbTransaction::new(res, self))
    }

    /// Create an object store with the given name
    pub fn create_object_store(&self, name: &str) -> Result<IdbObjectStore, DomException> {
        let inner = self.inner().create_object_store(name)?;
        Ok(IdbObjectStore::from_db(inner, self))
    }

    /// Create an object store with the given name & optional parameters
    pub fn create_object_store_with_params(
        &self,
        name: &str,
        params: &IdbObjectStoreParameters,
    ) -> Result<IdbObjectStore, DomException> {
        let inner = self
            .inner()
            .create_object_store_with_optional_parameters(name, params.as_js_value())?;
        Ok(IdbObjectStore::from_db(inner, self))
    }
}

impl Drop for IdbDatabase {
    fn drop(&mut self) {
        if let Some(_) = self.on_version_change {
            self.inner.set_onversionchange(None);
        }
    }
}

impl_display_for_named!(IdbDatabase);

fn factory() -> web_sys::IdbFactory {
    web_sys::window().unwrap().indexed_db().unwrap().unwrap()
}

#[cfg(test)]
pub mod test {
    use crate::request::IdbOpenDbRequestLike;
    use core::future::Future;

    use super::*;

    fn db_name() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    async fn open_db(req: OpenDbRequest) -> IdbDatabase {
        req.into_future().await.expect("Future failed")
    }

    fn open_db_req(req: Result<OpenDbRequest, DomException>) -> impl Future<Output = IdbDatabase> {
        open_db(req.expect("Base open failed"))
    }

    async fn open_db_and_store() -> (IdbDatabase, &'static str) {
        let mut req = IdbDatabase::open(&db_name()).expect("Base open");

        fn on_upgrade_needed(evt: &IdbVersionChangeEvent) -> Result<(), JsValue> {
            evt.db().create_object_store("teststore")?;
            Ok(())
        }
        req.set_on_upgrade_needed(Some(on_upgrade_needed));
        let db = open_db(req).await;

        (db, "teststore")
    }

    pub mod object_store_names {
        test_mod_init!();

        test_case!(async empty_iter => {
            let db = open_db_req(IdbDatabase::open(&db_name())).await;
            let stores: Vec<String> = db.object_store_names().collect();
            assert_eq!(stores, Vec::<String>::new());
        });

        test_case!(async iter_with_two => {
            let mut req = IdbDatabase::open(&db_name()).expect("Base open");
            fn on_upgrade_needed(evt: &IdbVersionChangeEvent) -> Result<(), JsValue> {
                evt.db().create_object_store("store1")?;
                evt.db().create_object_store("store2")?;
                Ok(())
            }
            req.set_on_upgrade_needed(Some(on_upgrade_needed));
            let db = open_db(req).await;
            let stores: Vec<String> = db.object_store_names().collect();

            assert_eq!(stores, vec![String::from("store1"), String::from("store2")]);
        });
    }

    pub mod open {
        test_mod_init!();

        fn test_version(db: &IdbDatabase, version_expected: f64, name_expected: String) {
            assert_eq!(db.name(), name_expected, "name");
            assert_eq!(db.version(), version_expected, "version");
        }

        test_case!(async should_open_without_version => {
            let name = db_name();
            test_version(&open_db_req(IdbDatabase::open(&name)).await, 1.0, name);
        });

        test_case!(async should_open_with_u32 => {
            let name = db_name();
            test_version(&open_db_req(IdbDatabase::open_u32(&name, 101)).await, 101.0, name);
        });

        test_case!(async should_open_with_f64 => {
            let name = db_name();
            test_version(&open_db_req(IdbDatabase::open_f64(&name, 42.0)).await, 42.0, name);
        });
    }
}
